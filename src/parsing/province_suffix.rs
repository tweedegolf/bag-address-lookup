// Normalizes locality/municipality names by stripping a trailing parenthesized
// province-style abbreviation.
//
// Upstream sources bake inconsistent province hints into names — the BAG ships
// `Hengelo (Gld)`, CBS ships `Hengelo (O.)`. Stripping here lets downstream code
// re-add a canonical `(GE)` / `(OV)` only when there is an actual name collision.

/// Province abbreviations that upstream sources bake into names (matched
/// case-insensitively against the parenthesized suffix, with an optional
/// trailing dot). Anything not in this list is left alone so unrelated
/// parentheticals like `(oud)` survive.
const PROVINCE_SUFFIX_TOKENS: &[&str] = &[
    "DR", "FL", "FR", "FRL", "GE", "GLD", "GR", "L", "LB", "LI", "NB", "NH", "O", "OV", "UT", "ZE",
    "ZH",
];

pub fn strip_province_suffix(name: &str) -> &str {
    let Some(without_close) = name.strip_suffix(')') else {
        return name;
    };
    let Some(open) = without_close.rfind(" (") else {
        return name;
    };
    let inside = &without_close[open + 2..];
    let core = inside.strip_suffix('.').unwrap_or(inside);
    if PROVINCE_SUFFIX_TOKENS
        .iter()
        .any(|tok| tok.eq_ignore_ascii_case(core))
    {
        &without_close[..open]
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::strip_province_suffix;

    #[test]
    fn strips_known_province_style_suffixes() {
        assert_eq!(strip_province_suffix("Hengelo (Gld)"), "Hengelo");
        assert_eq!(strip_province_suffix("Hengelo (O.)"), "Hengelo");
        assert_eq!(strip_province_suffix("Rijswijk (GLD)"), "Rijswijk");
        assert_eq!(strip_province_suffix("Rijswijk (NB)"), "Rijswijk");
        assert_eq!(strip_province_suffix("Bergen (NH)"), "Bergen");
        assert_eq!(strip_province_suffix("Bergen (L)"), "Bergen");
    }

    #[test]
    fn leaves_other_parentheticals_intact() {
        assert_eq!(strip_province_suffix("Foo (oud)"), "Foo (oud)");
        assert_eq!(strip_province_suffix("Plain"), "Plain");
        assert_eq!(strip_province_suffix("Foo ()"), "Foo ()");
        assert_eq!(strip_province_suffix("Foo (12)"), "Foo (12)");
        assert_eq!(strip_province_suffix("Foo (ABCDE)"), "Foo (ABCDE)");
        assert_eq!(strip_province_suffix("Foo(NH)"), "Foo(NH)");
    }
}
