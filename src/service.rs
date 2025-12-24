use serde_json::json;
use std::{error::Error, future::Future, sync::Arc};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use crate::database::DatabaseHandle;

fn logging_disabled() -> bool {
    std::env::var("BAG_ADDRESS_LOOKUP_QUIET")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

pub async fn serve(addr: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind(addr).await?;

    serve_with_shutdown(listener, tokio::signal::ctrl_c()).await
}

pub async fn serve_with_shutdown<F>(
    listener: TcpListener,
    shutdown: F,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    F: Future<Output = Result<(), std::io::Error>> + Send + 'static,
{
    let database = Arc::new(DatabaseHandle::load()?);

    if database.is_empty() {
        return Err("Database is empty; rebuild the database file".into());
    }

    let mut shutdown = Box::pin(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            accept = listener.accept() => {
                let (stream, _) = accept?;
                let db = database.clone();
                tokio::spawn(async move {
                    let mut stream = stream;
                    if let Err(err) = handle_connection(&mut stream, db).await {
                        let _ = write_response(&mut stream, 500, &json_error(&err.to_string())).await;
                    }
                });
            }
        }
    }

    Ok(())
}

async fn handle_connection(
    stream: &mut tokio::net::TcpStream,
    database: Arc<DatabaseHandle<'static>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut buffer = [0u8; 255];
    let mut total_read = 0usize;

    while total_read < 255 {
        let read = stream.read(&mut buffer[total_read..]).await?;
        if read == 0 {
            break;
        }
        total_read += read;
        // detect end
        if buffer[..total_read].ends_with(b"\n") {
            break;
        }
    }

    let request = String::from_utf8_lossy(&buffer[..total_read]);

    let mut lines = request.lines();
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();

    if !logging_disabled() {
        println!(
            "[bag-address-lookup] received request: {} {}",
            method, target
        );
    }

    if method != "GET" {
        write_response(stream, 405, &json_error("method not allowed")).await?;
        return Ok(());
    }

    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != "/" && path != "/lookup" {
        write_response(stream, 404, &json_error("not found")).await?;
        return Ok(());
    }

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
        write_response(stream, 400, &json_error("missing postal_code")).await?;
        return Ok(());
    };

    let Some(house_number) = house_number else {
        write_response(stream, 400, &json_error("missing house_number")).await?;
        return Ok(());
    };

    if !is_valid_postal_code(&postal_code) {
        write_response(stream, 400, &json_error("invalid postal_code")).await?;
        return Ok(());
    }

    match database.lookup(&postal_code, house_number) {
        Some((public_space, locality)) => {
            let body = json_ok(public_space, locality);
            write_response(stream, 200, &body).await?;
        }
        None => {
            write_response(stream, 404, &json_error("address not found")).await?;
        }
    }

    Ok(())
}

async fn write_response(
    stream: &mut tokio::net::TcpStream,
    status_code: u16,
    body: &str,
) -> std::io::Result<()> {
    let status_text = match status_code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Internal Server Error",
    };

    if !logging_disabled() {
        if status_code == 200 {
            println!("[bag-address-lookup] successful lookup: {}", body);
        } else {
            eprintln!("[bag-address-lookup] error {}: {}", status_code, body);
        }
    }

    let header = format!(
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );

    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;
    stream.shutdown().await
}

fn json_ok(public_space: &str, locality: &str) -> String {
    serde_json::to_string(&json!({ "pr": public_space, "wp": locality }))
        .expect("serialize ok response")
}

fn json_error(message: &str) -> String {
    serde_json::to_string(&json!({ "error": message })).expect("serialize error response")
}

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
    use super::handle_connection;
    use crate::{Database, DatabaseHandle, NumberRange, encode_pc};
    use std::sync::Arc;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    fn test_database() -> DatabaseHandle<'static> {
        let localities = vec!["Amsterdam".to_string()];
        let public_spaces = vec!["Stationsstraat".to_string()];
        let ranges = vec![NumberRange {
            postal_code: encode_pc(b"1234AB"),
            start: 10,
            length: 2,
            public_space_index: 0,
            locality_index: 0,
        }];

        DatabaseHandle::Decoded(Database {
            localities,
            public_spaces,
            ranges,
        })
    }

    async fn send_request(request: &str, db: Arc<DatabaseHandle<'static>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let _ = handle_connection(&mut stream, db).await;
        });

        let mut client = TcpStream::connect(addr).await.unwrap();
        client.write_all(request.as_bytes()).await.unwrap();
        client.shutdown().await.unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();
        let _ = server.await;
        response
    }

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
