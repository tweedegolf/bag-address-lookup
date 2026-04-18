use serde_json::json;
use std::{
    error::Error,
    future::Future,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

/// Maximum time allowed for handling a single connection (read + process + write).
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

/// Upper bound on request header bytes consumed per connection.
///
/// Large enough for realistic browser requests (cookies, Accept-*, Sec-Fetch-*,
/// Referer) while bounding memory. Closing a TCP socket with unread bytes
/// pending in the receive queue makes Linux emit a RST instead of FIN, which
/// surfaces as `ERR_CONNECTION_RESET` in the browser — so we read through the
/// end-of-headers marker rather than stopping at a fixed byte count.
const MAX_REQUEST_BYTES: usize = 8192;

use crate::database::DatabaseHandle;

mod localities_list;
mod lookup;
mod municipalities;
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
                    match tokio::time::timeout(
                        CONNECTION_TIMEOUT,
                        handle_connection(&mut stream, db),
                    )
                    .await
                    {
                        Ok(Err(err)) => {
                            let _ = write_response(
                                &mut stream,
                                500,
                                &json_error(&err.to_string()),
                                None,
                            )
                            .await;
                        }
                        Err(_elapsed) => {
                            let _ = write_response(
                                &mut stream,
                                408,
                                &json_error("request timeout"),
                                None,
                            )
                            .await;
                        }
                        Ok(Ok(())) => {}
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
    let mut buffer = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];

    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if find_header_end(&buffer).is_some() {
            break;
        }
        if buffer.len() >= MAX_REQUEST_BYTES {
            break;
        }
    }

    let request = String::from_utf8_lossy(&buffer);

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
        write_response(
            stream,
            response.status_code,
            &response.body,
            Some(duration_ms),
        )
        .await?;
        return Ok(());
    }

    let (path, query) = target.split_once('?').unwrap_or((target, ""));

    if path == "/" {
        return write_html_response(stream, API_DOCS_HTML).await;
    }

    let response = match path {
        "/suggest" => suggest::handle_suggest(database.as_ref(), query),
        "/lookup" => lookup::handle_lookup(database.as_ref(), query),
        "/localities" => localities_list::handle_localities(database.as_ref()),
        "/municipalities" => municipalities::handle_municipalities(database.as_ref()),
        _ => Response::new(404, json_error("not found")),
    };

    let duration_ms = start.elapsed().as_millis();
    write_response(
        stream,
        response.status_code,
        &response.body,
        Some(duration_ms),
    )
    .await?;
    Ok(())
}

/// Write an HTML response and close the connection.
async fn write_html_response(
    stream: &mut tokio::net::TcpStream,
    body: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;
    stream.shutdown().await?;
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
        408 => "Request Timeout",
        _ => "Internal Server Error",
    };

    if !logging_disabled() {
        let preview = log_preview(body);
        if status_code == 200 {
            if let Some(duration_ms) = duration_ms {
                println!(
                    "[bag-address-lookup] successful lookup ({} ms): {}",
                    duration_ms, preview
                );
            } else {
                println!("[bag-address-lookup] successful lookup: {}", preview);
            }
        } else if let Some(duration_ms) = duration_ms {
            eprintln!(
                "[bag-address-lookup] error {} ({} ms): {}",
                status_code, duration_ms, preview
            );
        } else {
            eprintln!("[bag-address-lookup] error {}: {}", status_code, preview);
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

const API_DOCS_HTML: &str = include_str!("api_docs.html");

/// Maximum number of body characters to include in request logs.
const LOG_BODY_PREVIEW_CHARS: usize = 200;

/// Format a response body for logging: the first `LOG_BODY_PREVIEW_CHARS`
/// characters (not bytes, so multi-byte UTF-8 names aren't split) followed by
/// the full body length. Short bodies are returned as-is.
fn log_preview(body: &str) -> String {
    let len = body.len();
    let mut end = 0;
    let mut count = 0;
    for (i, _) in body.char_indices() {
        if count == LOG_BODY_PREVIEW_CHARS {
            end = i;
            break;
        }
        count += 1;
    }
    if count < LOG_BODY_PREVIEW_CHARS {
        return format!("{body} ({len} bytes)");
    }
    format!("{}… ({len} bytes)", &body[..end])
}

/// Return the offset just past the first `\r\n\r\n` header terminator, if any.
fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| i + 4)
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
        let locality_codes = vec![3594, 1245, 3451];
        let public_spaces = vec!["Stationsstraat".to_string()];
        let ranges = vec![NumberRange {
            postal_code: encode_pc(b"1234AB"),
            start: 10,
            length: 2,
            public_space_index: 0,
            locality_index: 0,
            step: 1,
        }];

        let municipalities = vec![
            "Amsterdam".to_string(),
            "Rotterdam".to_string(),
            "Utrecht".to_string(),
        ];
        let provinces = vec!["NH".to_string(), "UT".to_string(), "ZH".to_string()];
        // Amsterdam -> Amsterdam (code 363, Noord-Holland)
        // Rotterdam -> Rotterdam (code 599, Zuid-Holland)
        // Utrecht -> Utrecht (code 344, Utrecht)
        let municipality_codes = vec![363, 599, 344];
        let locality_municipality = vec![0, 1, 2]; // each locality maps to its municipality
        let municipality_province = vec![0, 2, 1]; // Amsterdam->NH, Rotterdam->ZH, Utrecht->Utrecht
        let locality_had_suffix = vec![false, false, false];
        let municipality_had_suffix = vec![false, false, false];

        DatabaseHandle::Decoded(Database {
            localities,
            locality_codes,
            public_spaces,
            ranges,
            municipalities,
            provinces,
            municipality_codes,
            locality_municipality,
            municipality_province,
            locality_had_suffix,
            municipality_had_suffix,
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
