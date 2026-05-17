use serde::Serialize;

use crate::database::DatabaseHandle;

use super::Response;

/// One entry in the `/municipalities` JSON array.
#[derive(Serialize)]
struct MunicipalityEntry<'a> {
    gm: &'a str,
    gm_code: u16,
    pv: &'a str,
    unique: bool,
    had_suffix: bool,
}

/// Handle the `/municipalities` endpoint by returning all municipalities with their province.
pub(crate) fn handle_municipalities(database: &DatabaseHandle) -> Response {
    let entries: Vec<MunicipalityEntry> = database
        .municipality_details()
        .into_iter()
        .map(|d| MunicipalityEntry {
            gm: d.name,
            gm_code: d.code,
            pv: d.province,
            unique: d.unique,
            had_suffix: d.had_suffix,
        })
        .collect();
    let body = serde_json::to_string(&entries).expect("serialize municipalities");
    Response::new(200, body)
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::{send_request, test_database};
    use std::sync::Arc;

    #[tokio::test]
    async fn municipalities_returns_list() {
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /municipalities HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"gm\":"));
        assert!(response.contains("\"pv\":"));
        assert!(response.contains("\"gm_code\":"));
        assert!(response.contains("\"unique\":"));
        assert!(response.contains("\"had_suffix\":"));
    }
}
