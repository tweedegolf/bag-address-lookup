use crate::database::DatabaseHandle;

use super::Response;

/// Handle the `/localities` endpoint by returning all localities with their municipality.
pub(crate) fn handle_localities(database: &DatabaseHandle) -> Response {
    let details = database.locality_details();
    let mut body = String::from("[");
    for (i, (wp, gm, gm_code)) in details.iter().enumerate() {
        if i > 0 {
            body.push(',');
        }
        body.push_str(&format!(
            "{{\"wp\":{},\"gm\":{},\"gm_code\":{}}}",
            serde_json::to_string(wp).expect("serialize wp"),
            serde_json::to_string(gm).expect("serialize gm"),
            gm_code,
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
    async fn localities_returns_list() {
        let db = Arc::new(test_database());
        let response =
            send_request("GET /localities HTTP/1.1\r\nHost: localhost\r\n\r\n", db).await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"wp\":"));
        assert!(response.contains("\"gm\":"));
        assert!(response.contains("\"gm_code\":"));
    }
}
