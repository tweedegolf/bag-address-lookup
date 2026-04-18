use crate::database::DatabaseHandle;

use super::Response;

/// Handle the `/municipalities` endpoint by returning all municipalities with their province.
pub(crate) fn handle_municipalities(database: &DatabaseHandle) -> Response {
    let details = database.municipality_details();
    let mut body = String::from("[");
    for (i, (gm, gm_code, pv, unique, had_suffix)) in details.iter().enumerate() {
        if i > 0 {
            body.push(',');
        }
        body.push_str(&format!(
            "{{\"gm\":{},\"gm_code\":{},\"pv\":{},\"unique\":{},\"had_suffix\":{}}}",
            serde_json::to_string(gm).expect("serialize gm"),
            gm_code,
            serde_json::to_string(pv).expect("serialize pv"),
            unique,
            had_suffix,
        ));
    }
    body.push(']');
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
