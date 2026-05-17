use serde::Serialize;

use crate::database::DatabaseHandle;

use super::Response;

/// One entry in the `/localities` JSON array.
#[derive(Serialize)]
struct LocalityEntry<'a> {
    wp: &'a str,
    wp_code: u16,
    gm: &'a str,
    gm_code: u16,
    pv: &'a str,
    unique: bool,
    had_suffix: bool,
}

/// Handle the `/localities` endpoint by returning all localities with their municipality.
pub(crate) fn handle_localities(database: &DatabaseHandle) -> Response {
    let entries: Vec<LocalityEntry> = database
        .locality_details()
        .into_iter()
        .map(|d| LocalityEntry {
            wp: d.name,
            wp_code: d.code,
            gm: d.municipality,
            gm_code: d.municipality_code,
            pv: d.province,
            unique: d.unique,
            had_suffix: d.had_suffix,
        })
        .collect();
    let body = serde_json::to_string(&entries).expect("serialize localities");
    Response::new(200, body)
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::{send_request, test_database};
    use std::sync::Arc;

    #[tokio::test]
    async fn localities_returns_list() {
        let db = Arc::new(test_database());
        let response =
            send_request("GET /localities HTTP/1.1\r\nHost: localhost\r\n\r\n", db).await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"wp\":"));
        assert!(response.contains("\"wp_code\":"));
        assert!(response.contains("\"gm\":"));
        assert!(response.contains("\"gm_code\":"));
        assert!(response.contains("\"pv\":"));
        assert!(response.contains("\"unique\":"));
        assert!(response.contains("\"had_suffix\":"));
    }
}
