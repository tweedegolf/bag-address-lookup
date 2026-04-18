//! Fuzzy suggest search over localities and municipalities.
//!
//! The scoring lives in core so it can be reused outside of the web service
//! (for example from the CLI or library consumers).

use std::collections::HashMap;

use crate::DatabaseHandle;

/// Default score threshold below which candidates are discarded.
pub const DEFAULT_SUGGEST_THRESHOLD: f32 = 0.7;

/// Default maximum number of suggestions returned.
pub const DEFAULT_SUGGEST_LIMIT: usize = 10;

/// A single scored suggestion result.
pub enum SuggestEntry {
    Locality {
        wp: String,
        wp_code: u16,
        gm: String,
        gm_code: u16,
        pv: String,
        unique: bool,
        had_suffix: bool,
    },
    Municipality {
        gm: String,
        gm_code: u16,
        pv: String,
        unique: bool,
        had_suffix: bool,
    },
}

impl SuggestEntry {
    pub fn name(&self) -> &str {
        match self {
            SuggestEntry::Locality { wp, .. } => wp,
            SuggestEntry::Municipality { gm, .. } => gm,
        }
    }
}

/// Suggest localities and municipalities matching `query`.
///
/// Candidates scoring below `threshold` are discarded. At most `limit`
/// highest-scoring results are returned, mixed across localities and
/// municipalities.
///
/// Prefer calling [`DatabaseHandle::suggest`] — this free function backs it.
pub(crate) fn suggest(
    database: &DatabaseHandle,
    query: &str,
    threshold: f32,
    limit: usize,
) -> Vec<SuggestEntry> {
    let normalized = normalize_query(query);
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(f32, SuggestEntry)> = Vec::new();

    for (wp, wp_code, gm, gm_code, pv, unique, had_suffix) in database.locality_details() {
        let score = fuzzy_score(&normalized, &normalize_query(wp));
        if score >= threshold {
            scored.push((
                score,
                SuggestEntry::Locality {
                    wp: wp.to_string(),
                    wp_code,
                    gm: gm.to_string(),
                    gm_code,
                    pv: pv.to_string(),
                    unique,
                    had_suffix,
                },
            ));
        }
    }

    for (gm, gm_code, pv, unique, had_suffix) in database.municipality_details() {
        let score = fuzzy_score(&normalized, &normalize_query(gm));
        if score >= threshold {
            scored.push((
                score,
                SuggestEntry::Municipality {
                    gm: gm.to_string(),
                    gm_code,
                    pv: pv.to_string(),
                    unique,
                    had_suffix,
                },
            ));
        }
    }

    scored.sort_by(|(a_score, a_entry), (b_score, b_entry)| {
        b_score
            .partial_cmp(a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a_entry.name().cmp(b_entry.name()))
    });

    scored
        .into_iter()
        .take(limit)
        .map(|(_, entry)| entry)
        .collect()
}

/// Normalize user input and candidates for case-insensitive matching.
pub(crate) fn normalize_query(value: &str) -> String {
    value.trim().to_lowercase()
}

/// Compute a fuzzy score between the search `needle` and a candidate `haystack`.
///
/// Algorithm details:
/// - Substring boost: if `haystack` contains `needle`, return `1.0 + len(needle)/len(haystack)`,
///   with an extra `+0.5` when the match is anchored at the start of `haystack`.
///   This prioritizes contiguous matches while keeping longer exacts slightly below shorter perfects.
/// - Otherwise compute:
///   - `subsequence_ratio`: fraction of `needle` characters found in order within `haystack`.
///   - `dice_coefficient`: bigram overlap similarity for approximate string shape matching.
/// - Final score: `0.6 * subsequence_ratio + 0.4 * dice_coefficient`, plus a prefix bonus
///   of up to `+0.2` proportional to the length of the common prefix between `needle` and `haystack`.
///   Subsequence helps partial-word matching; dice helps tolerate small typos.
pub(crate) fn fuzzy_score(needle: &str, haystack: &str) -> f32 {
    if needle.is_empty() || haystack.is_empty() {
        return 0.0;
    }

    if let Some(pos) = haystack.find(needle) {
        let ratio = needle.chars().count() as f32 / haystack.chars().count() as f32;
        let start_boost = if pos == 0 { 0.5 } else { 0.0 };
        return 1.0 + ratio.min(1.0) + start_boost;
    }

    let subsequence = subsequence_ratio(needle, haystack);
    let dice = dice_coefficient(needle, haystack);
    (subsequence * 0.6) + (dice * 0.4) + prefix_bonus(needle, haystack)
}

/// Bonus up to 0.2 scaling with the fraction of `needle` that matches `haystack` from the start.
fn prefix_bonus(needle: &str, haystack: &str) -> f32 {
    let matched = needle
        .chars()
        .zip(haystack.chars())
        .take_while(|(n, h)| n == h)
        .count();
    if matched == 0 {
        return 0.0;
    }
    let needle_len = needle.chars().count();
    (matched as f32 / needle_len as f32) * 0.2
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
    use super::{dice_coefficient, fuzzy_score, normalize_query, subsequence_ratio};

    #[test]
    fn fuzzy_score_prefers_substring_match() {
        let needle = normalize_query("dam");
        let exact = normalize_query("amsterdam");
        let fuzzy = normalize_query("dandandimam");
        let exact_score = fuzzy_score(&needle, &exact);
        let fuzzy_score_value = fuzzy_score(&needle, &fuzzy);

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
}
