//! URL query-string parsing.
//!
//! The query component of an HTTP request target is a URI query (RFC 3986
//! §3.4), not an `application/x-www-form-urlencoded` payload. The two differ in
//! a way that matters here: the form encoding decodes `+` to a space, while in
//! a URI query `+` is an ordinary literal character. Parsing the query as a
//! form payload — as `form_urlencoded` does — silently rewrites a queried `+`
//! into a space. We therefore percent-decode `%XX` escapes only and leave `+`
//! untouched.

use percent_encoding::percent_decode_str;

/// Parse a URI query string into `(key, value)` pairs.
///
/// Pairs are separated by `&`; the first `=` in a pair splits the key from the
/// value (so a value may itself contain `=`). Both sides are percent-decoded,
/// with invalid UTF-8 replaced lossily. `+` is preserved literally. A pair with
/// no `=` yields an empty value, and empty segments (e.g. from a trailing or
/// doubled `&`) are skipped.
pub(crate) fn parse_query(query: &str) -> impl Iterator<Item = (String, String)> + '_ {
    query
        .split('&')
        .filter(|segment| !segment.is_empty())
        .map(|segment| match segment.split_once('=') {
            Some((key, value)) => (decode(key), decode(value)),
            None => (decode(segment), String::new()),
        })
}

/// Percent-decode a single query component, replacing invalid UTF-8 lossily.
fn decode(value: &str) -> String {
    percent_decode_str(value).decode_utf8_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::parse_query;

    /// Collect the parser output into an owned vector for assertions.
    fn pairs(query: &str) -> Vec<(String, String)> {
        parse_query(query).collect()
    }

    #[test]
    fn parses_single_pair() {
        assert_eq!(pairs("wp=Amsterdam"), [("wp".into(), "Amsterdam".into())]);
    }

    #[test]
    fn parses_multiple_pairs() {
        assert_eq!(
            pairs("pc=1234AB&n=11"),
            [("pc".into(), "1234AB".into()), ("n".into(), "11".into()),]
        );
    }

    #[test]
    fn decodes_percent_encoded_space() {
        assert_eq!(
            pairs("wp=Bergen%20(LI)"),
            [("wp".into(), "Bergen (LI)".into())]
        );
    }

    #[test]
    fn decodes_percent_encoded_utf8() {
        // `%C3%BA` is `ú` in UTF-8 — part of the municipality Súdwest-Fryslân.
        assert_eq!(pairs("wp=S%C3%BAdwest"), [("wp".into(), "Súdwest".into())]);
    }

    #[test]
    fn keeps_plus_literal() {
        // A URI query `+` is a literal character, not a space. This is the
        // behaviour that distinguishes us from `form_urlencoded`.
        assert_eq!(pairs("wp=Den+Bosch"), [("wp".into(), "Den+Bosch".into())]);
    }

    #[test]
    fn decodes_percent_encoded_plus() {
        // An intentional literal `+` is still expressible as `%2B`.
        assert_eq!(pairs("wp=C%2B%2B"), [("wp".into(), "C++".into())]);
    }

    #[test]
    fn first_equals_splits_key_from_value() {
        // Only the first `=` is a separator; later ones belong to the value.
        assert_eq!(pairs("wp=a=b=c"), [("wp".into(), "a=b=c".into())]);
    }

    #[test]
    fn pair_without_equals_has_empty_value() {
        assert_eq!(pairs("wp"), [("wp".into(), String::new())]);
    }

    #[test]
    fn skips_empty_segments() {
        // Leading, doubled and trailing `&` produce no spurious empty pairs.
        assert_eq!(
            pairs("&wp=Amsterdam&&n=1&"),
            [("wp".into(), "Amsterdam".into()), ("n".into(), "1".into()),]
        );
    }

    #[test]
    fn empty_query_yields_nothing() {
        assert!(pairs("").is_empty());
    }

    #[test]
    fn invalid_percent_escape_is_kept_literally() {
        // A truncated or malformed `%` escape is left as-is rather than erroring.
        assert_eq!(pairs("wp=bad%2"), [("wp".into(), "bad%2".into())]);
        assert_eq!(pairs("wp=x%GZy"), [("wp".into(), "x%GZy".into())]);
    }
}
