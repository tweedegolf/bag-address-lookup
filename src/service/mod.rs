use serde_json::json;
use std::{error::Error, future::Future, sync::Arc, time::Instant};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use crate::database::DatabaseHandle;

mod lookup;
mod suggest;

/// Minimal response wrapper for handler results.
struct Response {
    status_code: u16,
    body: String,
}

impl Response {
    /// Construct a response with status code and serialized JSON body.
    fn new(status_code: u16, body: String) -> Self {
        Self { status_code, body }
    }
}

/// Enable/disable request logging via `BAG_ADDRESS_LOOKUP_QUIET`.
fn logging_disabled() -> bool {
    std::env::var("BAG_ADDRESS_LOOKUP_QUIET")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

/// Start a BAG lookup HTTP server on the given address.
pub async fn serve(addr: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind(addr).await?;

    serve_with_shutdown(listener, tokio::signal::ctrl_c()).await
}

/// Start the server with a shutdown future (e.g. Ctrl-C).
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

    if !logging_disabled() {
        println!("[bag-address-lookup] database initialized");
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
                        let _ = write_response(
                            &mut stream,
                            500,
                            &json_error(&err.to_string()),
                            None,
                        )
                        .await;
                    }
                });
            }
        }
    }

    Ok(())
}

/// Handle a single HTTP connection and route to the correct handler.
async fn handle_connection(
    stream: &mut tokio::net::TcpStream,
    database: Arc<DatabaseHandle>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let start = Instant::now();
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
        let response = Response::new(405, json_error("method not allowed"));
        let duration_ms = start.elapsed().as_millis();
        write_response(stream, response.status_code, &response.body, Some(duration_ms)).await?;
        return Ok(());
    }

    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let response = match path {
        "/suggest" => suggest::handle_suggest(database.as_ref(), query),
        "/" | "/lookup" => lookup::handle_lookup(database.as_ref(), query),
        _ => Response::new(404, json_error("not found")),
    };

    let duration_ms = start.elapsed().as_millis();
    write_response(stream, response.status_code, &response.body, Some(duration_ms)).await?;
    Ok(())
}

/// Write the HTTP response with JSON body and close the connection.
async fn write_response(
    stream: &mut tokio::net::TcpStream,
    status_code: u16,
    body: &str,
    duration_ms: Option<u128>,
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
            if let Some(duration_ms) = duration_ms {
                println!(
                    "[bag-address-lookup] successful lookup ({} ms): {}",
                    duration_ms, body
                );
            } else {
                println!("[bag-address-lookup] successful lookup: {}", body);
            }
        } else if let Some(duration_ms) = duration_ms {
            eprintln!(
                "[bag-address-lookup] error {} ({} ms): {}",
                status_code, duration_ms, body
            );
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

/// JSON for a successful lookup response.
pub(crate) fn json_ok(public_space: &str, locality: &str) -> String {
    serde_json::to_string(&json!({ "pr": public_space, "wp": locality }))
        .expect("serialize ok response")
}

/// JSON for an error response.
pub(crate) fn json_error(message: &str) -> String {
    serde_json::to_string(&json!({ "error": message })).expect("serialize error response")
}

/// JSON list response (used by suggestions).
pub(crate) fn json_list(values: &[String]) -> String {
    serde_json::to_string(values).expect("serialize list response")
}

#[cfg(test)]
pub(crate) mod test_utils {
    use super::handle_connection;
    use crate::{Database, DatabaseHandle, NumberRange, encode_pc};
    use std::sync::Arc;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    pub(crate) fn test_database() -> DatabaseHandle {
        let localities = vec![
            "Amsterdam".to_string(),
            "Rotterdam".to_string(),
            "Utrecht".to_string(),
        ];
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

    pub(crate) async fn send_request(request: &str, db: Arc<DatabaseHandle>) -> String {
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
}
