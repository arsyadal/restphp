use crate::sapi::ExecutionTarget;
use crate::worker::WorkerHandle;
use axum::{
    body::Bytes,
    extract::State,
    http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
pub struct ServerState {
    pub worker: Arc<tokio::sync::RwLock<WorkerHandle>>,
    pub default_script: String,
}

pub async fn run_http_server(
    host: &str,
    port: u16,
    script_path: &str,
    worker: Arc<tokio::sync::RwLock<WorkerHandle>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(ServerState {
        worker,
        default_script: script_path.to_string(),
    });

    let app = Router::new()
        .fallback(any(handle_php_request))
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    println!("🦀 [RestPHP] Listening on http://{}", addr);
    println!(
        "🐘 [RestPHP] Serving persistent PHP entrypoint: {}",
        script_path
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_php_request(
    State(state): State<Arc<ServerState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let method_str = method.as_str().to_string();
    let uri_str = uri.path().to_string();
    let query_str = uri.query().unwrap_or("").to_string();
    let body_vec = body.to_vec();

    // Extract cookie
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok().map(|s| s.to_string()));

    // Extract content-type
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok().map(|s| s.to_string()));

    // Collect all headers as (String, String) pairs
    let mut header_list = Vec::with_capacity(headers.len());
    for (k, v) in &headers {
        if let Ok(v_str) = v.to_str() {
            header_list.push((k.as_str().to_string(), v_str.to_string()));
        }
    }

    let target = ExecutionTarget::File(std::path::PathBuf::from(state.default_script.clone()));

    let worker = state.worker.read().await.clone();

    match worker
        .dispatch_with_headers(
            target,
            method_str,
            uri_str,
            query_str,
            body_vec,
            header_list,
            cookie,
            content_type,
        )
        .await
    {
        Ok(php_resp) => {
            let status = StatusCode::from_u16(php_resp.status).unwrap_or(StatusCode::OK);
            let mut response = Response::builder().status(status);

            let mut has_content_type = false;
            // Forward custom response headers from PHP
            for (k, v) in &php_resp.headers {
                if let (Ok(hname), Ok(hval)) = (
                    HeaderName::from_bytes(k.as_bytes()),
                    HeaderValue::from_str(v),
                ) {
                    if hname == header::CONTENT_TYPE {
                        has_content_type = true;
                    }
                    response = response.header(hname, hval);
                }
            }

            if !has_content_type {
                if let Ok(val) = HeaderValue::from_str(&php_resp.content_type) {
                    response = response.header(header::CONTENT_TYPE, val);
                }
            }
            response = response.header("Server", "RestPHP/0.1.0");

            response
                .body(axum::body::Body::from(php_resp.body))
                .unwrap_or_else(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to construct body",
                    )
                        .into_response()
                })
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("RestPHP Worker Error: {}", err),
        )
            .into_response(),
    }
}
