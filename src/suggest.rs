//! Fuzzy suggest search over localities and municipalities.
//!
//! The scoring lives in core so it can be reused outside of the web service
//! (for example from the CLI or library consumers).

use std::collections::HashMap;

use crate::{DatabaseHandle, fryslan_aliases::lookup_alias};

/// Default score threshold below which candidates are discarded.
pub const DEFAULT_SUGGEST_THRESHOLD: f32 = 0.7;

/// Default maximum number of suggestions returned.
pub const DEFAULT_SUGGEST_LIMIT: usize = 10;

/// Caribbean Netherlands locality names not present in the BAG/CBS sources we
/// ingest. Kralendijk and Rincon are the localities of Bonaire; Caribisch
/// Nederland is otherwise represented at the municipality level.
static CN_LOCALITIES: &[&str] = &["Kralendijk", "Rincon"];

/// Caribbean Netherlands municipality names — the three openbare lichamen of
/// Caribisch Nederland — not present in the BAG/CBS sources we ingest.
static CN_MUNICIPALITIES: &[&str] = &["Bonaire", "Saba", "Sint Eustatius"];

/// Suggest locality, municipality and (optionally) alias names matching `query`.
///
/// Candidates scoring below `threshold` are discarded. At most `limit`
/// highest-scoring distinct names are returned, mixed across localities and
/// municipalities. When `include_municipalities` is false, municipality names
/// are not offered as suggestions. When `include_aliases` is false, the
/// Frisian/Dutch aliases of localities are not offered as suggestions.
///
/// Names that originally carried a stripped province suffix get the province
/// code appended (e.g. `Bergen` in Limburg becomes `Bergen (LI)`) so the
/// caller can tell same-named places apart.
///
/// Prefer calling [`DatabaseHandle::suggest`] — this free function backs it.
pub(crate) fn suggest(
    database: &DatabaseHandle,
    query: &str,
    threshold: f32,
    limit: usize,
    include_municipalities: bool,
    include_aliases: bool,
) -> Vec<String> {
    let normalized = normalize_query(query);
    if normalized.is_empty() {
        return Vec::new();
    }

    // Each candidate is the display name returned to the caller (which may
    // carry a province code). Fuzzy matching scores against this same string,
    // so a query that spells out the province suffix can match it. Aliases are
    // independent candidates — once expanded the originating name is irrelevant.
    let mut candidates: Vec<String> = Vec::new();

    for loc in database.locality_details() {
        if include_aliases && let Some(alias) = lookup_alias(loc.name) {
            candidates.push(alias.to_string());
        }

        candidates.push(display_name(loc.name, loc.province, loc.had_suffix));
    }

    for &wp in CN_LOCALITIES {
        candidates.push(wp.to_string());
    }

    if include_municipalities {
        for muni in database.municipality_details() {
            if include_aliases && let Some(alias) = lookup_alias(muni.name) {
                candidates.push(alias.to_string());
            }

            candidates.push(display_name(muni.name, muni.province, muni.had_suffix));
        }

        for &gm in CN_MUNICIPALITIES {
            candidates.push(gm.to_string());
        }
    }

    let mut scored: Vec<(f32, String)> = candidates
        .into_iter()
        .filter_map(|display| {
            let score = fuzzy_score(&normalized, &normalize_query(&display));
            (score >= threshold).then_some((score, display))
        })
        .collect();

    // Highest score first; ties broken alphabetically so identical display
    // names from the locality and municipality pools end up adjacent for
    // deduplication.
    scored.sort_by(|(a_score, a_name), (b_score, b_name)| {
        b_score
            .partial_cmp(a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a_name.cmp(b_name))
    });
    scored.dedup_by(|(_, a), (_, b)| a == b);

    scored
        .into_iter()
        .take(limit)
        .map(|(_, display)| display)
        .collect()
}

/// Format a suggestion name, appending the province code in parentheses when
/// the name originally carried a stripped province suffix (e.g. `Bergen` in
/// Limburg becomes `Bergen (LI)`).
fn display_name(name: &str, province: &str, had_suffix: bool) -> String {
    if had_suffix {
        format!("{name} ({province})")
    } else {
        name.to_string()
    }
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
    use super::{
        DEFAULT_SUGGEST_LIMIT, DEFAULT_SUGGEST_THRESHOLD, dice_coefficient, fuzzy_score,
        normalize_query, subsequence_ratio, suggest,
    };

    #[test]
    fn suggest_appends_province_code_for_suffixed_names() {
        use crate::{Database, DatabaseHandle, NumberRange, encode_pc};

        // The "Bergen" locality carried a stripped province suffix in the
        // source data; the "Bergen" municipality did not.
        let database = DatabaseHandle::Decoded(Database {
            localities: vec!["Bergen".to_string()],
            locality_codes: vec![1],
            public_spaces: vec!["Dorpsstraat".to_string()],
            ranges: vec![NumberRange {
                postal_code: encode_pc(b"1234AB"),
                start: 1,
                length: 1,
                public_space_index: 0,
                locality_index: 0,
                step: 1,
            }],
            municipalities: vec!["Bergen".to_string()],
            provinces: vec!["LI".to_string()],
            municipality_codes: vec![1],
            locality_municipality: vec![0],
            municipality_province: vec![0],
            locality_had_suffix: vec![true],
            municipality_had_suffix: vec![false],
        });

        let results = suggest(
            &database,
            "Bergen",
            DEFAULT_SUGGEST_THRESHOLD,
            DEFAULT_SUGGEST_LIMIT,
            true,
            false,
        );

        // The suffixed locality is disambiguated; the municipality is not.
        assert!(results.contains(&"Bergen (LI)".to_string()));
        assert!(results.contains(&"Bergen".to_string()));
    }

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
