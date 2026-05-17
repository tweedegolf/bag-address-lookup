use crate::{
    database::DatabaseHandle,
    suggest::{DEFAULT_SUGGEST_LIMIT, DEFAULT_SUGGEST_THRESHOLD},
};

use super::{Response, json_error, query::parse_query};

/// Handle the `/suggest` endpoint by returning a JSON list of locality and
/// municipality names matching the `wp` query param.
pub(crate) fn handle_suggest(database: &DatabaseHandle, query: &str) -> Response {
    let mut query_text = None;
    let mut include_municipalities = true;
    let mut include_aliases = false;

    for (key, value) in parse_query(query) {
        match key.as_str() {
            "wp" => query_text = Some(value),
            "municipalities" => include_municipalities = parse_bool(&value),
            "aliases" => include_aliases = parse_bool(&value),
            _ => {}
        }
    }

    let Some(query_text) = query_text else {
        return Response::new(400, json_error("missing wp"));
    };

    Response::new(
        200,
        suggest_json(
            database,
            &query_text,
            include_municipalities,
            include_aliases,
        ),
    )
}

/// Parse a boolean-ish query parameter. `false`, `0` and `no` (case-insensitive)
/// are false; anything else (including a malformed or empty value) is true.
fn parse_bool(value: &str) -> bool {
    !matches!(value.to_ascii_lowercase().as_str(), "false" | "0" | "no")
}

/// Build the JSON response body: a flat array of suggestion names.
fn suggest_json(
    database: &DatabaseHandle,
    query: &str,
    include_municipalities: bool,
    include_aliases: bool,
) -> String {
    let names = database.suggest(
        query,
        suggest_threshold(),
        DEFAULT_SUGGEST_LIMIT,
        include_municipalities,
        include_aliases,
    );

    serde_json::to_string(&names).expect("serialize suggestions")
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
        parse_bool,
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
        // The body is a flat JSON array of names.
        assert!(response.contains("[\"Amsterdam\"]"));
    }

    #[tokio::test]
    async fn suggest_includes_alias_when_requested() {
        // "Boalsert" is the Frisian alias for the official BAG name "Bolsward".
        // With aliases enabled it is offered as a suggestion in its own right.
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /suggest?wp=Boalsert&aliases=true HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"Boalsert\""));
    }

    #[tokio::test]
    async fn suggest_omits_aliases_by_default() {
        // Without the aliases param the Frisian alias is not a candidate, and
        // "Boalsert" is too dissimilar from "Bolsward" to match on its own.
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /suggest?wp=Boalsert HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(!response.contains("Boalsert"));
        assert!(!response.contains("Bolsward"));
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
        assert!(response.contains("\"Kralendijk\""));

        let response = send_request(
            "GET /suggest?wp=Saba HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db.clone(),
        )
        .await;
        assert!(response.contains("\"Saba\""));

        let response = send_request(
            "GET /suggest?wp=Eustatius HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;
        assert!(response.contains("\"Sint Eustatius\""));
    }

    #[tokio::test]
    async fn suggest_includes_municipalities_by_default() {
        // "Súdwest-Fryslân" is a municipality with no matching locality, so it
        // can only appear when municipality names are suggested.
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /suggest?wp=S%C3%BAdwest HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"Súdwest-Fryslân\""));
    }

    #[tokio::test]
    async fn suggest_excludes_municipalities_when_requested() {
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /suggest?wp=S%C3%BAdwest&municipalities=false HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(!response.contains("Súdwest-Fryslân"));
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
    async fn suggest_decodes_percent_encoded_space() {
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /suggest?wp=Amster%20 HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"Amsterdam\""));
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
