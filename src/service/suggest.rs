use std::collections::HashMap;

use crate::database::DatabaseHandle;

use super::{Response, json_error, json_list};

/// Handle the `/suggest` endpoint by returning locality suggestions for the `wp` query param.
pub(crate) fn handle_suggest(database: &DatabaseHandle, query: &str) -> Response {
    let mut query_text = None;

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        if key == "wp" {
            query_text = Some(value.to_string());
        }
    }

    let Some(query_text) = query_text else {
        return Response::new(400, json_error("missing wp"));
    };

    let suggestions = suggest_localities(database, &query_text);
    let body = json_list(&suggestions);
    Response::new(200, body)
}

/// Suggest localities using a light-weight fuzzy ranking:
/// - Fast substring match gets a top score boost.
/// - Otherwise combine subsequence coverage with bigram similarity.
/// - Ignore candidates that end up with a score below the threshold value.
/// - Return at most 10 highest scored matches.
const DEFAULT_SUGGEST_THRESHOLD: f32 = 0.7;

fn suggest_localities(database: &DatabaseHandle, query: &str) -> Vec<String> {
    let threshold = suggest_threshold();
    let normalized = normalize_query(query);
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut scored = Vec::new();
    for locality in database.localities() {
        let candidate = normalize_query(locality);
        let score = fuzzy_score(&normalized, &candidate);
        if score >= threshold {
            scored.push((score, locality));
        }
    }

    scored.sort_by(|(a_score, a_name), (b_score, b_name)| {
        b_score
            .partial_cmp(a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a_name.cmp(b_name))
    });

    scored
        .into_iter()
        .take(10)
        .map(|(_, locality)| locality.to_string())
        .collect()
}

/// Normalize user input and candidates for case-insensitive matching.
fn normalize_query(value: &str) -> String {
    value.trim().to_lowercase()
}

/// Read the minimum fuzzy-match score from the environment.
fn suggest_threshold() -> f32 {
    std::env::var("BAG_ADDRESS_LOOKUP_SUGGEST_THRESHOLD")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(DEFAULT_SUGGEST_THRESHOLD)
}

/// Compute a fuzzy score between the search `needle` and a candidate `haystack`.
///
/// Algorithm details:
/// - Substring boost: if `haystack` contains `needle`, return `1.0 + len(needle)/len(haystack)`.
///   This prioritizes contiguous matches while keeping longer exacts slightly below shorter perfects.
/// - Otherwise compute:
///   - `subsequence_ratio`: fraction of `needle` characters found in order within `haystack`.
///   - `dice_coefficient`: bigram overlap similarity for approximate string shape matching.
/// - Final score: `0.6 * subsequence_ratio + 0.4 * dice_coefficient`.
///   Subsequence helps partial-word matching; dice helps tolerate small typos.
fn fuzzy_score(needle: &str, haystack: &str) -> f32 {
    if needle.is_empty() || haystack.is_empty() {
        return 0.0;
    }

    if haystack.contains(needle) {
        let ratio = needle.chars().count() as f32 / haystack.chars().count() as f32;
        return 1.0 + ratio.min(1.0);
    }

    let subsequence = subsequence_ratio(needle, haystack);
    let dice = dice_coefficient(needle, haystack);
    (subsequence * 0.6) + (dice * 0.4)
}

/// Ratio of `needle` characters appearing in order inside `haystack`.
fn subsequence_ratio(needle: &str, haystack: &str) -> f32 {
    let mut matched = 0usize;
    let mut needle_chars = needle.chars();
    let mut current = needle_chars.next();

    if current.is_none() {
        return 0.0;
    }

    for ch in haystack.chars() {
        if let Some(target) = current {
            if ch == target {
                matched += 1;
                current = needle_chars.next();
            }
        } else {
            break;
        }
    }

    matched as f32 / needle.chars().count() as f32
}

/// Dice coefficient using character bigrams.
///
/// This measures similarity based on overlapping adjacent character pairs.
/// For each string, we count how many bigrams appear (including duplicates),
/// then compute `2 * overlap / (total_a + total_b)`. The result ranges
/// from 0.0 (no shared bigrams) to 1.0 (identical bigram multiset).
/// It is tolerant of small typos because nearby characters still form
/// similar bigrams even when a single character differs.
fn dice_coefficient(a: &str, b: &str) -> f32 {
    let mut b_counts: HashMap<(char, char), usize> = HashMap::new();
    let mut total_b = 0usize;
    let mut b_chars = b.chars();
    let mut prev_b = match b_chars.next() {
        Some(ch) => ch,
        None => return 0.0,
    };
    for ch in b_chars {
        total_b += 1;
        *b_counts.entry((prev_b, ch)).or_insert(0usize) += 1;
        prev_b = ch;
    }
    if total_b == 0 {
        return 0.0;
    }

    let mut intersection = 0usize;
    let mut total_a = 0usize;
    let mut a_chars = a.chars();
    let mut prev_a = match a_chars.next() {
        Some(ch) => ch,
        None => return 0.0,
    };
    for ch in a_chars {
        total_a += 1;
        if let Some(count) = b_counts.get_mut(&(prev_a, ch))
            && *count > 0
        {
            *count -= 1;
            intersection += 1;
        }
        prev_a = ch;
    }
    if total_a == 0 {
        return 0.0;
    }

    let total = total_a + total_b;
    (2 * intersection) as f32 / total as f32
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::{send_request, test_database};
    use super::{dice_coefficient, fuzzy_score, normalize_query, subsequence_ratio};
    use std::sync::Arc;

    #[test]
    fn fuzzy_score_prefers_substring_match() {
        let needle = normalize_query("dam");
        let exact = normalize_query("amsterdam");
        let fuzzy = normalize_query("dandandimam");
        let exact_score = fuzzy_score(&needle, &exact);
        let fuzzy_score_value = fuzzy_score(&needle, &fuzzy);

        dbg!(&exact_score);
        dbg!(&fuzzy_score_value);

        assert!(exact_score > 1.0);
        assert!(exact_score > fuzzy_score_value);
    }

    #[test]
    fn subsequence_ratio_respects_order() {
        let needle = normalize_query("ams");
        let in_order = normalize_query("amsterdam");
        let out_of_order = normalize_query("smaarten");

        assert!(subsequence_ratio(&needle, &in_order) > 0.9);
        assert!(subsequence_ratio(&needle, &out_of_order) < 0.9);
    }

    #[test]
    fn dice_coefficient_is_symmetric() {
        let a = normalize_query("utrecht");
        let b = normalize_query("utrech");

        let left = dice_coefficient(&a, &b);
        let right = dice_coefficient(&b, &a);

        assert!((left - right).abs() < f32::EPSILON);
        assert!(left > 0.5);
    }

    #[tokio::test]
    async fn suggest_success() {
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /suggest?wp=Amster HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("[\"Amsterdam\""));
    }

    #[tokio::test]
    async fn suggest_missing_query() {
        let db = Arc::new(test_database());
        let response = send_request("GET /suggest HTTP/1.1\r\nHost: localhost\r\n\r\n", db).await;

        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
        assert!(response.contains("{\"error\":\"missing wp\"}"));
    }
}
