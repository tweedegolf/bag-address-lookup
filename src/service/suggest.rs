use crate::{
    database::DatabaseHandle,
    suggest::{DEFAULT_SUGGEST_LIMIT, DEFAULT_SUGGEST_THRESHOLD, SuggestEntry},
};

use super::{Response, json_error};

/// Handle the `/suggest` endpoint by returning locality and municipality
/// suggestions for the `wp` query param.
pub(crate) fn handle_suggest(database: &DatabaseHandle, query: &str) -> Response {
    let mut query_text = None;
    let mut include_municipalities = true;

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        match key {
            "wp" => query_text = Some(percent_decode(value)),
            "municipalities" => include_municipalities = parse_bool(value),
            _ => {}
        }
    }

    let Some(query_text) = query_text else {
        return Response::new(400, json_error("missing wp"));
    };

    Response::new(
        200,
        suggest_json(database, &query_text, include_municipalities),
    )
}

/// Parse a boolean-ish query parameter. `false`, `0` and `no` (case-insensitive)
/// are false; anything else (including a malformed value) keeps the default of
/// including municipalities.
fn parse_bool(value: &str) -> bool {
    !matches!(value.to_ascii_lowercase().as_str(), "false" | "0" | "no")
}

/// Decode a URL form-encoded query value: `+` becomes space, `%XX` becomes the
/// byte with hex value `XX`. Malformed `%` escapes are emitted literally and
/// decoding continues; if the decoded bytes are not valid UTF-8 the original
/// input is returned unchanged.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push((h * 16 + l) as u8);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| value.to_string())
}

fn suggest_json(database: &DatabaseHandle, query: &str, include_municipalities: bool) -> String {
    let results = database.suggest(
        query,
        suggest_threshold(),
        DEFAULT_SUGGEST_LIMIT,
        include_municipalities,
    );

    let mut body = String::from("[");
    for (i, entry) in results.iter().enumerate() {
        if i > 0 {
            body.push(',');
        }
        append_entry_json(entry, &mut body);
    }
    body.push(']');
    body
}

fn append_entry_json(entry: &SuggestEntry, body: &mut String) {
    match entry {
        SuggestEntry::Locality {
            wp,
            wp_code,
            gm,
            gm_code,
            pv,
            unique,
            had_suffix,
        } => {
            body.push_str(&format!(
                "{{\"wp\":{},\"wp_code\":{},\"gm\":{},\"gm_code\":{},\"pv\":{},\"unique\":{},\"had_suffix\":{}}}",
                serde_json::to_string(wp).expect("serialize wp"),
                wp_code,
                serde_json::to_string(gm).expect("serialize gm"),
                gm_code,
                serde_json::to_string(pv).expect("serialize pv"),
                unique,
                had_suffix,
            ));
        }
        SuggestEntry::Municipality {
            gm,
            gm_code,
            pv,
            unique,
            had_suffix,
        } => {
            body.push_str(&format!(
                "{{\"gm\":{},\"gm_code\":{},\"pv\":{},\"unique\":{},\"had_suffix\":{}}}",
                serde_json::to_string(gm).expect("serialize gm"),
                gm_code,
                serde_json::to_string(pv).expect("serialize pv"),
                unique,
                had_suffix,
            ));
        }
    }
}

/// Read the minimum fuzzy-match score from the environment.
fn suggest_threshold() -> f32 {
    std::env::var("BAG_ADDRESS_LOOKUP_SUGGEST_THRESHOLD")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(DEFAULT_SUGGEST_THRESHOLD)
}

#[cfg(test)]
mod tests {
    use super::{
        super::test_utils::{send_request, test_database},
        parse_bool, percent_decode,
    };
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
        assert!(response.contains("\"wp\":\"Amsterdam\""));
        assert!(response.contains("\"wp_code\":"));
        assert!(response.contains("\"gm\":\"Amsterdam\""));
        assert!(response.contains("\"gm_code\":"));
        assert!(response.contains("\"pv\":"));
    }

    #[tokio::test]
    async fn suggest_includes_caribbean_netherlands() {
        let db = Arc::new(test_database());

        let response = send_request(
            "GET /suggest?wp=Kralendijk HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db.clone(),
        )
        .await;
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"wp\":\"Kralendijk\""));
        assert!(response.contains("\"gm\":\"Bonaire\""));
        assert!(response.contains("\"pv\":\"BES\""));

        let response = send_request(
            "GET /suggest?wp=Saba HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db.clone(),
        )
        .await;
        assert!(response.contains("\"gm\":\"Saba\""));
        assert!(response.contains("\"pv\":\"BES\""));

        let response = send_request(
            "GET /suggest?wp=Eustatius HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;
        assert!(response.contains("\"gm\":\"Sint Eustatius\""));
    }

    #[tokio::test]
    async fn suggest_includes_municipalities_by_default() {
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /suggest?wp=Amster HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        // The standalone municipality entry serializes as an object starting
        // with `gm` (locality entries start with `wp`).
        assert!(response.contains("{\"gm\":\"Amsterdam\""));
    }

    #[tokio::test]
    async fn suggest_excludes_municipalities_when_requested() {
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /suggest?wp=Amster&municipalities=false HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        // The locality is still suggested...
        assert!(response.contains("\"wp\":\"Amsterdam\""));
        // ...but the standalone municipality entry is gone.
        assert!(!response.contains("{\"gm\":\"Amsterdam\""));
    }

    #[tokio::test]
    async fn suggest_excludes_caribbean_municipalities_when_requested() {
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /suggest?wp=Saba&municipalities=false HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(!response.contains("Saba"));
    }

    #[tokio::test]
    async fn suggest_missing_query() {
        let db = Arc::new(test_database());
        let response = send_request("GET /suggest HTTP/1.1\r\nHost: localhost\r\n\r\n", db).await;

        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
        assert!(response.contains("{\"error\":\"missing wp\"}"));
    }

    #[tokio::test]
    async fn suggest_decodes_plus_as_space() {
        // `Amster+` decodes to `Amster ` which normalize_query trims back to
        // `amster`, matching Amsterdam. Without decoding, the raw `Amster+`
        // would fail to match.
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /suggest?wp=Amster+ HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"wp\":\"Amsterdam\""));
    }

    #[tokio::test]
    async fn suggest_decodes_percent_encoded_space() {
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /suggest?wp=Amster%20 HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"wp\":\"Amsterdam\""));
    }

    #[test]
    fn percent_decode_plain_text_passthrough() {
        assert_eq!(percent_decode("Amsterdam"), "Amsterdam");
    }

    #[test]
    fn percent_decode_plus_becomes_space() {
        assert_eq!(percent_decode("Den+Haag"), "Den Haag");
    }

    #[test]
    fn percent_decode_percent_20_becomes_space() {
        assert_eq!(percent_decode("Den%20Haag"), "Den Haag");
    }

    #[test]
    fn percent_decode_multibyte_utf8() {
        // `%C3%A9` is UTF-8 for `é`.
        assert_eq!(percent_decode("caf%C3%A9"), "café");
    }

    #[test]
    fn percent_decode_malformed_escape_is_literal() {
        // Non-hex follow-up: emit `%` literally, continue decoding the rest.
        assert_eq!(percent_decode("a%ZZb+c"), "a%ZZb c");
    }

    #[test]
    fn percent_decode_trailing_percent_is_literal() {
        // No room for two hex digits: emit `%` literally.
        assert_eq!(percent_decode("ab%"), "ab%");
        assert_eq!(percent_decode("ab%4"), "ab%4");
    }

    #[test]
    fn percent_decode_invalid_utf8_falls_back_to_raw() {
        // `%FF` is not valid UTF-8 on its own; we keep the input verbatim.
        let input = "x%FFy";
        assert_eq!(percent_decode(input), input);
    }

    #[test]
    fn percent_decode_empty() {
        assert_eq!(percent_decode(""), "");
    }

    #[test]
    fn parse_bool_false_values() {
        assert!(!parse_bool("false"));
        assert!(!parse_bool("False"));
        assert!(!parse_bool("FALSE"));
        assert!(!parse_bool("0"));
        assert!(!parse_bool("no"));
    }

    #[test]
    fn parse_bool_other_values_are_true() {
        assert!(parse_bool("true"));
        assert!(parse_bool("1"));
        assert!(parse_bool(""));
        assert!(parse_bool("yes"));
        assert!(parse_bool("garbage"));
    }
}
