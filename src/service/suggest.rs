use crate::database::DatabaseHandle;
use crate::suggest::{DEFAULT_SUGGEST_LIMIT, DEFAULT_SUGGEST_THRESHOLD, SuggestEntry};

use super::{Response, json_error};

/// Handle the `/suggest` endpoint by returning locality and municipality
/// suggestions for the `wp` query param.
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

    Response::new(200, suggest_json(database, &query_text))
}

fn suggest_json(database: &DatabaseHandle, query: &str) -> String {
    let results = database.suggest(query, suggest_threshold(), DEFAULT_SUGGEST_LIMIT);

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
        assert!(response.contains("\"wp\":\"Amsterdam\""));
        assert!(response.contains("\"wp_code\":"));
        assert!(response.contains("\"gm\":\"Amsterdam\""));
        assert!(response.contains("\"gm_code\":"));
        assert!(response.contains("\"pv\":"));
    }

    #[tokio::test]
    async fn suggest_missing_query() {
        let db = Arc::new(test_database());
        let response = send_request("GET /suggest HTTP/1.1\r\nHost: localhost\r\n\r\n", db).await;

        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
        assert!(response.contains("{\"error\":\"missing wp\"}"));
    }
}
