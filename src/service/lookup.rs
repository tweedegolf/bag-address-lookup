use crate::database::DatabaseHandle;

use super::{Response, json_error, json_ok};

/// Handle the `/lookup` endpoint using `pc` (postal code) and `n` (house number).
pub(crate) fn handle_lookup(database: &DatabaseHandle, query: &str) -> Response {
    let mut postal_code = None;
    let mut house_number = None;

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        match key {
            "pc" => postal_code = Some(value.to_string()),
            "n" => house_number = value.parse::<u32>().ok(),
            _ => {}
        }
    }

    let Some(postal_code) = postal_code else {
        return Response::new(400, json_error("missing postal_code"));
    };

    let Some(house_number) = house_number else {
        return Response::new(400, json_error("missing house_number"));
    };

    if !is_valid_postal_code(&postal_code) {
        return Response::new(400, json_error("invalid postal_code"));
    }

    match database.lookup(&postal_code, house_number) {
        Some((public_space, locality)) => {
            let body = json_ok(public_space, locality);
            Response::new(200, body)
        }
        None => Response::new(404, json_error("address not found")),
    }
}

/// Validate Dutch postal code format: 4 digits + 2 uppercase letters.
fn is_valid_postal_code(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 6 {
        return false;
    }
    if !bytes[..4].iter().all(|b| b.is_ascii_digit()) {
        return false;
    }
    bytes[4].is_ascii_uppercase() && bytes[5].is_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::{send_request, test_database};
    use std::sync::Arc;

    #[tokio::test]
    async fn lookup_success() {
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /lookup?pc=1234AB&n=11 HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("{\"pr\":\"Stationsstraat\",\"wp\":\"Amsterdam\"}"));
    }

    #[tokio::test]
    async fn lookup_missing_postal_code() {
        let db = Arc::new(test_database());
        let response =
            send_request("GET /lookup?n=11 HTTP/1.1\r\nHost: localhost\r\n\r\n", db).await;

        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
        assert!(response.contains("{\"error\":\"missing postal_code\"}"));
    }

    #[tokio::test]
    async fn lookup_missing_house_number() {
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /lookup?pc=1234AB HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
        assert!(response.contains("{\"error\":\"missing house_number\"}"));
    }

    #[tokio::test]
    async fn lookup_invalid_postal_code() {
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /lookup?pc=1234ab&n=11 HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
        assert!(response.contains("{\"error\":\"invalid postal_code\"}"));
    }

    #[tokio::test]
    async fn lookup_not_found() {
        let db = Arc::new(test_database());
        let response = send_request(
            "GET /lookup?pc=9999ZZ&n=1 HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(response.contains("{\"error\":\"address not found\"}"));
    }

    #[tokio::test]
    async fn method_not_allowed() {
        let db = Arc::new(test_database());
        let response = send_request(
            "POST /lookup?pc=1234AB&n=11 HTTP/1.1\r\nHost: localhost\r\n\r\n",
            db,
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 405 Method Not Allowed"));
        assert!(response.contains("{\"error\":\"method not allowed\"}"));
    }

    #[tokio::test]
    async fn large_request_with_valid_query() {
        let db = Arc::new(test_database());
        let mut request =
            String::from("GET /lookup?pc=1234AB&n=11 HTTP/1.1\r\nHost: localhost\r\n");
        request.push_str(&("X-Long: ".to_string() + &"a".repeat(4242) + "\r\n\r\n"));

        let response = send_request(&request, db).await;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("{\"pr\":\"Stationsstraat\",\"wp\":\"Amsterdam\"}"));
    }
}
