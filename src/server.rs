use crate::sapi::ExecutionTarget;
use crate::worker::WorkerHandle;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderValue, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
pub struct ServerState {
    pub worker: WorkerHandle,
    pub default_script: String,
}

pub async fn run_http_server(
    host: &str,
    port: u16,
    script_path: &str,
    worker: WorkerHandle,
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
    body: Bytes,
) -> Response {
    let method_str = method.as_str().to_string();
    let uri_str = uri.path().to_string();
    let query_str = uri.query().unwrap_or("").to_string();
    let body_vec = body.to_vec();

    let target = ExecutionTarget::File(std::path::PathBuf::from(state.default_script.clone()));

    match state
        .worker
        .dispatch(target, method_str, uri_str, query_str, body_vec)
        .await
    {
        Ok(php_resp) => {
            let status = StatusCode::from_u16(php_resp.status).unwrap_or(StatusCode::OK);
            let mut response = Response::builder().status(status);

            if let Ok(val) = HeaderValue::from_str(&php_resp.content_type) {
                response = response.header("Content-Type", val);
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
