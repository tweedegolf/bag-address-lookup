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
/// - Ignore candidates that end up with a zero score.
/// - Return at most 10 highest scored matches.
const SUGGEST_THRESHOLD: f32 = 0.7;

fn suggest_localities(database: &DatabaseHandle, query: &str) -> Vec<String> {
    let normalized = normalize_query(query);
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut scored = Vec::new();
    for locality in database.localities() {
        let candidate = normalize_query(locality);
        let score = fuzzy_score(&normalized, &candidate);
        if score >= SUGGEST_THRESHOLD {
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
fn dice_coefficient(a: &str, b: &str) -> f32 {
    let a_bigrams = bigrams(a);
    let b_bigrams = bigrams(b);
    if a_bigrams.is_empty() || b_bigrams.is_empty() {
        return 0.0;
    }

    let mut intersection = 0usize;
    let mut b_counts: std::collections::HashMap<(char, char), usize> =
        std::collections::HashMap::new();
    for bg in b_bigrams.iter().copied() {
        *b_counts.entry(bg).or_insert(0usize) += 1;
    }

    for bg in a_bigrams.iter() {
        if let Some(count) = b_counts.get_mut(bg)
            && *count > 0
        {
            *count -= 1;
            intersection += 1;
        }
    }

    let total = a_bigrams.len() + b_bigrams.len();
    (2 * intersection) as f32 / total as f32
}

/// Build adjacent character bigrams for dice similarity.
fn bigrams(value: &str) -> Vec<(char, char)> {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() < 2 {
        return Vec::new();
    }
    let mut grams = Vec::with_capacity(chars.len() - 1);
    for window in chars.windows(2) {
        grams.push((window[0], window[1]));
    }
    grams
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::{send_request, test_database};
    use std::sync::Arc;

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
