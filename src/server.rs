//! HTTP admission and response handling for RestPHP.
//!
//! This module deliberately owns the HTTP-facing safety boundary. PHP and the
//! worker pool only receive already-admitted request jobs and return a small,
//! validated response representation.

use crate::sapi::ExecutionTarget;
use crate::worker::{DispatchError, WorkerHandle};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use std::{
    future::Future,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Default maximum size accepted for an HTTP request body (16 MiB).
pub const DEFAULT_MAX_BODY_BYTES: usize = 16 * 1024 * 1024;
/// Default upper bound for admitted PHP jobs, including one being executed.
pub const DEFAULT_MAX_QUEUE: usize = 256;

/// HTTP safety limits. The CLI can construct this from its serve options while
/// embedders can use the same API without depending on clap.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub max_body_bytes: usize,
    pub max_queue: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
            max_queue: DEFAULT_MAX_QUEUE,
        }
    }
}

impl ServerConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_body_bytes == 0 {
            return Err("max_body_bytes must be greater than zero".to_string());
        }
        if self.max_queue == 0 {
            return Err("max_queue must be greater than zero".to_string());
        }
        Ok(())
    }
}

/// Shared control plane for request admission and graceful shutdown.
#[derive(Clone, Debug)]
pub struct ServerControl {
    admission_open: Arc<AtomicBool>,
}

impl Default for ServerControl {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerControl {
    pub fn new() -> Self {
        Self {
            admission_open: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Reject subsequent requests with a retryable 503 while admitted jobs drain.
    pub fn close_admission(&self) {
        self.admission_open.store(false, Ordering::Release);
    }

    pub fn open_admission(&self) {
        self.admission_open.store(true, Ordering::Release);
    }

    pub fn admission_is_open(&self) -> bool {
        self.admission_open.load(Ordering::Acquire)
    }
}

#[derive(Clone)]
pub struct ServerState {
    pub worker: Arc<tokio::sync::RwLock<WorkerHandle>>,
    pub default_script: String,
    control: ServerControl,
    queue: Arc<Semaphore>,
}

impl ServerState {
    pub fn new(
        worker: Arc<tokio::sync::RwLock<WorkerHandle>>,
        default_script: impl Into<String>,
        config: &ServerConfig,
        control: ServerControl,
    ) -> Self {
        Self {
            worker,
            default_script: default_script.into(),
            control,
            queue: Arc::new(Semaphore::new(config.max_queue)),
        }
    }

    pub fn control(&self) -> ServerControl {
        self.control.clone()
    }
}

/// Runs with production defaults and no externally-triggered shutdown.
/// Applications that own signal handling should use
/// [`run_http_server_with_config_and_shutdown`].
pub async fn run_http_server(
    host: &str,
    port: u16,
    script_path: &str,
    worker: Arc<tokio::sync::RwLock<WorkerHandle>>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_http_server_with_config_and_shutdown(
        host,
        port,
        script_path,
        worker,
        ServerConfig::default(),
        ServerControl::new(),
        std::future::pending(),
    )
    .await
}

/// Runs with explicit limits and a shutdown future. The caller should close
/// admission before resolving `shutdown`, then drain and join the worker pool.
pub async fn run_http_server_with_config_and_shutdown<F>(
    host: &str,
    port: u16,
    script_path: &str,
    worker: Arc<tokio::sync::RwLock<WorkerHandle>>,
    config: ServerConfig,
    control: ServerControl,
    shutdown: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Future<Output = ()> + Send + 'static,
{
    config
        .validate()
        .map_err(|err| format!("Invalid HTTP server configuration: {err}"))?;

    let state = Arc::new(ServerState::new(worker, script_path, &config, control));
    let app = Router::new()
        .fallback(any(handle_php_request))
        // Reject oversized known-length and streamed bodies before `Bytes`
        // materializes them. Axum maps this rejection to HTTP 413.
        .layer(DefaultBodyLimit::max(config.max_body_bytes))
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("🦀 [RestPHP] Listening on http://{}", addr);
    println!(
        "🐘 [RestPHP] Serving persistent PHP entrypoint: {}",
        script_path
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

async fn handle_php_request(
    State(state): State<Arc<ServerState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !state.control.admission_is_open() {
        return service_unavailable();
    }

    // Holding this permit through dispatch bounds admitted jobs (including a
    // currently executing job), so the HTTP handler never waits to admit work.
    let permit = match state.queue.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => return service_unavailable(),
    };

    // Do not let a request racing with shutdown enter the worker queue.
    if !state.control.admission_is_open() {
        return service_unavailable();
    }

    dispatch_php_request(state, method, uri, headers, body, permit).await
}

async fn dispatch_php_request(
    state: Arc<ServerState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
    _permit: OwnedSemaphorePermit,
) -> Response {
    let method_str = method.as_str().to_string();
    // CGI REQUEST_URI retains the raw path and query; PHP receives QUERY_STRING
    // separately for $_GET population.
    let request_uri = uri
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string());
    let query_str = uri.query().unwrap_or_default().to_string();
    let body_vec = body.to_vec();

    let cookie = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let mut header_list = Vec::with_capacity(headers.len());
    for (name, value) in &headers {
        if let Ok(value) = value.to_str() {
            header_list.push((name.as_str().to_string(), value.to_string()));
        }
    }

    let target = ExecutionTarget::File(std::path::PathBuf::from(&state.default_script));
    let php_response = {
        let worker_guard = state.worker.read().await;
        worker_guard
            .dispatch_with_headers(
                target,
                method_str,
                request_uri,
                query_str,
                body_vec,
                header_list,
                cookie,
                content_type,
            )
            .await
    };

    match php_response {
        Ok(response) if response.success => build_php_response(
            response.status,
            &response.content_type,
            &response.headers,
            response.body,
        ),
        Ok(response) => {
            tracing::error!(
                status = response.status,
                "PHP request failed; returning an opaque internal-server-error response"
            );
            internal_server_error()
        }
        Err(DispatchError::QueueFull | DispatchError::Draining) => service_unavailable(),
        Err(error) => {
            // PHP paths and engine internals stay in logs, not network output.
            tracing::error!(error = %error, "PHP worker request failed");
            internal_server_error()
        }
    }
}

fn build_php_response(
    raw_status: u16,
    content_type: &str,
    php_headers: &[(String, String)],
    body: Vec<u8>,
) -> Response {
    let status = match valid_status(raw_status) {
        Some(status) => status,
        None => {
            tracing::error!(status = raw_status, "PHP returned an invalid HTTP status");
            return internal_server_error();
        }
    };

    let mut response = Response::new(axum::body::Body::from(body));
    *response.status_mut() = status;
    let response_headers = response.headers_mut();
    let mut has_content_type = false;

    for (name, value) in php_headers {
        let header_name = match HeaderName::from_bytes(name.as_bytes()) {
            Ok(header_name) if !is_hop_by_hop_or_server_header(&header_name) => header_name,
            Ok(_) => continue,
            Err(error) => {
                tracing::warn!(header = %name, error = %error, "Dropping invalid PHP response header name");
                continue;
            }
        };
        let header_value = match HeaderValue::from_str(value) {
            Ok(header_value) => header_value,
            Err(error) => {
                tracing::warn!(header = %name, error = %error, "Dropping invalid PHP response header value");
                continue;
            }
        };
        if header_name == header::CONTENT_TYPE {
            has_content_type = true;
        }
        // Append keeps repeatable Set-Cookie fields distinct.
        response_headers.append(header_name, header_value);
    }

    if !has_content_type {
        match HeaderValue::from_str(content_type) {
            Ok(value) => {
                response_headers.insert(header::CONTENT_TYPE, value);
            }
            Err(error) => {
                tracing::warn!(error = %error, "PHP returned an invalid content type; using safe default");
                response_headers.insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("text/plain; charset=utf-8"),
                );
            }
        }
    }
    response_headers.insert(header::SERVER, HeaderValue::from_static("RestPHP/0.1.0"));
    response
}

fn valid_status(status: u16) -> Option<StatusCode> {
    (100..=599)
        .contains(&status)
        .then(|| StatusCode::from_u16(status).ok())
        .flatten()
}

fn is_hop_by_hop_or_server_header(name: &HeaderName) -> bool {
    name == header::CONNECTION
        || name.as_str() == "keep-alive"
        || name == header::TE
        || name == header::TRAILER
        || name == header::TRANSFER_ENCODING
        || name == header::UPGRADE
        || name == header::CONTENT_LENGTH
        || name == header::SERVER
        || name.as_str().starts_with("proxy-")
}

fn service_unavailable() -> Response {
    let mut response = (
        StatusCode::SERVICE_UNAVAILABLE,
        "Service temporarily unavailable; retry shortly.",
    )
        .into_response();
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
    response
}

fn internal_server_error() -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_hop_by_hop_headers_and_preserves_set_cookie() {
        let response = build_php_response(
            201,
            "application/json",
            &[
                ("Connection".into(), "close".into()),
                ("Content-Length".into(), "999".into()),
                ("Proxy-Authenticate".into(), "nope".into()),
                ("Server".into(), "PHP".into()),
                ("Set-Cookie".into(), "one=1".into()),
                ("Set-Cookie".into(), "two=2".into()),
                ("X-Visible".into(), "yes".into()),
            ],
            b"body".to_vec(),
        );

        assert_eq!(response.status(), StatusCode::CREATED);
        assert!(response.headers().get(header::CONNECTION).is_none());
        assert!(response.headers().get(header::CONTENT_LENGTH).is_none());
        assert_eq!(
            response.headers().get(header::SERVER).unwrap(),
            "RestPHP/0.1.0"
        );
        assert_eq!(
            response
                .headers()
                .get_all(header::SET_COOKIE)
                .iter()
                .count(),
            2
        );
        assert_eq!(response.headers().get("x-visible").unwrap(), "yes");
    }

    #[test]
    fn invalid_status_is_a_generic_internal_error() {
        let response = build_php_response(99, "text/plain", &[], Vec::new());
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn control_closes_admission() {
        let control = ServerControl::new();
        assert!(control.admission_is_open());
        control.close_admission();
        assert!(!control.admission_is_open());
    }
}
