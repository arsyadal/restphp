//! Persistent Zend Worker Pool — Dedicated OS threads hosting isolated PHP VM instances.
//!
//! Each worker thread owns a `PhpEngine` (non-Send, non-Sync) and processes requests
//! dispatched via lock-free crossbeam channels. Tokio oneshot channels return responses
//! to the async HTTP server.

pub use crate::sapi::ExecutionTarget;
use crate::sapi::{PhpEngine, PhpResponse, WorkerRequestContext};
use bytes::Bytes;
use crossbeam_channel::{bounded, Sender};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use tokio::sync::oneshot;

/// A single unit of work dispatched to a PHP worker thread.
pub struct WorkerJob {
    pub target: ExecutionTarget,
    pub method: String,
    pub uri: String,
    pub query: String,
    pub body: Vec<u8>,
    /// HTTP request headers as `(name, value)` pairs for `$_SERVER` population.
    pub headers: Vec<(String, String)>,
    /// Raw `Cookie` header value for `$_COOKIE` population.
    pub cookie: Option<String>,
    /// Content-Type of the request body (for `$_POST` parsing).
    pub content_type: Option<String>,
    /// Oneshot channel to send the response back to the async handler.
    pub respond_to: oneshot::Sender<PhpResponse>,
}

/// Global request counter shared across all worker threads.
static TOTAL_REQUESTS: AtomicU64 = AtomicU64::new(0);

/// Handle to the worker pool, cheaply cloneable.
#[derive(Clone)]
pub struct WorkerHandle {
    sender: Sender<WorkerJob>,
    worker_count: usize,
}

impl WorkerHandle {
    /// Spawns a pool of `num_workers` dedicated OS threads, each hosting an isolated Zend VM.
    ///
    /// All workers share a single bounded crossbeam channel as a work-stealing queue.
    /// The `max_requests` parameter controls worker recycling: after processing this many
    /// requests, the worker re-initializes its PHP VM to prevent memory fragmentation.
    /// Set to 0 to disable recycling.
    pub fn new_pool(num_workers: usize, max_requests: u64) -> Result<Self, String> {
        let num_workers = num_workers.max(1);
        let (sender, receiver) = bounded::<WorkerJob>(num_workers * 256);

        for i in 0..num_workers {
            let rx = receiver.clone();
            thread::Builder::new()
                .name(format!("restphp-worker-{}", i))
                .spawn(move || {
                    worker_loop(i, rx, max_requests);
                })
                .map_err(|e| format!("Failed to spawn worker-{}: {}", i, e))?;
        }

        println!(
            "🐘 [RestPHP] Spawned {} persistent Zend worker thread(s) (max_requests: {})",
            num_workers,
            if max_requests == 0 {
                "unlimited".to_string()
            } else {
                max_requests.to_string()
            }
        );

        Ok(WorkerHandle {
            sender,
            worker_count: num_workers,
        })
    }

    /// Convenience: spawns a single worker thread (legacy API).
    pub fn new() -> Result<Self, String> {
        Self::new_pool(1, 0)
    }

    /// Returns the total number of requests processed across all workers.
    pub fn total_requests() -> u64 {
        TOTAL_REQUESTS.load(Ordering::Relaxed)
    }

    /// Returns the number of worker threads in the pool.
    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Dispatches a job to any available worker in the pool (simple convenience version).
    pub async fn dispatch(
        &self,
        target: ExecutionTarget,
        method: String,
        uri: String,
        query: String,
        body: Vec<u8>,
    ) -> Result<PhpResponse, String> {
        self.dispatch_with_headers(target, method, uri, query, body, Vec::new(), None, None)
            .await
    }

    /// Dispatches a job to any available worker in the pool with full headers, cookie, and content-type.
    #[allow(clippy::too_many_arguments)]
    pub async fn dispatch_with_headers(
        &self,
        target: ExecutionTarget,
        method: String,
        uri: String,
        query: String,
        body: Vec<u8>,
        headers: Vec<(String, String)>,
        cookie: Option<String>,
        content_type: Option<String>,
    ) -> Result<PhpResponse, String> {
        let (tx, rx) = oneshot::channel();
        let job = WorkerJob {
            target,
            method,
            uri,
            query,
            body,
            headers,
            cookie,
            content_type,
            respond_to: tx,
        };

        self.sender
            .send(job)
            .map_err(|e| format!("Failed to dispatch job to worker: {}", e))?;

        rx.await
            .map_err(|_| "Worker dropped response channel".to_string())
    }
}

/// Main loop for a single worker thread.
fn worker_loop(id: usize, receiver: crossbeam_channel::Receiver<WorkerJob>, max_requests: u64) {
    tracing::info!("🐘 [Worker-{}] Initializing dedicated PHP VM...", id);

    let engine = match PhpEngine::init() {
        Ok(e) => e,
        Err(err) => {
            tracing::error!("❌ [Worker-{}] Failed to initialize PhpEngine: {}", id, err);
            return;
        }
    };

    tracing::info!(
        "✅ [Worker-{}] PHP VM initialized. Ready for persistent requests.",
        id
    );

    let mut local_count: u64 = 0;

    while let Ok(job) = receiver.recv() {
        local_count += 1;
        TOTAL_REQUESTS.fetch_add(1, Ordering::Relaxed);

        // Build server variables from HTTP headers
        let server_vars = build_server_vars(
            &job.method,
            &job.uri,
            &job.query,
            &job.headers,
            &job.content_type,
            job.body.len(),
        );

        // Create per-request context
        let mut ctx = WorkerRequestContext::new(Bytes::from(job.body), server_vars);

        // Set cookie if provided
        if let Some(ref cookie_str) = job.cookie {
            ctx = ctx.with_cookie(cookie_str);
        }

        // Execute the request
        let resp = match job.target {
            ExecutionTarget::File(ref path) => {
                let target = ExecutionTarget::File(path.clone());
                engine
                    .execute_request(
                        &mut ctx,
                        &target,
                        &job.method,
                        &job.uri,
                        &job.query,
                        job.content_type.as_deref(),
                        Some(path.as_path()),
                    )
                    .unwrap_or_else(|err| PhpResponse {
                        status: 500,
                        content_type: "text/plain".to_string(),
                        headers: Vec::new(),
                        body: format!("Worker-{} error: {}", id, err).into_bytes(),
                        success: false,
                    })
            }
            ExecutionTarget::Inline(ref code) | ExecutionTarget::Code(ref code) => {
                let target = ExecutionTarget::Inline(code.clone());
                engine
                    .execute_request(
                        &mut ctx,
                        &target,
                        &job.method,
                        &job.uri,
                        &job.query,
                        job.content_type.as_deref(),
                        None,
                    )
                    .unwrap_or_else(|err| PhpResponse {
                        status: 500,
                        content_type: "text/plain".to_string(),
                        headers: Vec::new(),
                        body: format!("Worker-{} error: {}", id, err).into_bytes(),
                        success: false,
                    })
            }
        };

        let _ = job.respond_to.send(resp);

        // Worker recycling: if max_requests is set and reached, break and let
        // the thread exit. A supervisor can respawn if needed.
        if max_requests > 0 && local_count >= max_requests {
            tracing::info!(
                "♻️ [Worker-{}] Reached max_requests ({}), recycling...",
                id,
                max_requests
            );
            break;
        }
    }

    tracing::info!(
        "🛑 [Worker-{}] Shutting down after {} requests.",
        id,
        local_count
    );
    // PhpEngine::drop() calls restphp_sapi_teardown()
}

/// Builds the `$_SERVER` superglobal variables from HTTP request metadata.
fn build_server_vars(
    method: &str,
    uri: &str,
    query: &str,
    headers: &[(String, String)],
    content_type: &Option<String>,
    content_length: usize,
) -> Vec<(String, String)> {
    let mut vars = Vec::with_capacity(headers.len() + 16);

    // Standard CGI variables
    vars.push(("REQUEST_METHOD".to_string(), method.to_string()));
    vars.push(("REQUEST_URI".to_string(), uri.to_string()));
    vars.push(("QUERY_STRING".to_string(), query.to_string()));
    vars.push(("SERVER_SOFTWARE".to_string(), "RestPHP/0.1.0".to_string()));
    vars.push(("SERVER_PROTOCOL".to_string(), "HTTP/1.1".to_string()));
    vars.push(("GATEWAY_INTERFACE".to_string(), "CGI/1.1".to_string()));
    vars.push(("CONTENT_LENGTH".to_string(), content_length.to_string()));

    if let Some(ref ct) = content_type {
        vars.push(("CONTENT_TYPE".to_string(), ct.clone()));
    }

    // Convert HTTP headers to CGI-style HTTP_* variables
    for (name, value) in headers {
        let upper = name.to_uppercase().replace('-', "_");
        // Special headers that don't get HTTP_ prefix
        match upper.as_str() {
            "CONTENT_TYPE" | "CONTENT_LENGTH" => {
                // Already handled above
            }
            "HOST" => {
                vars.push(("HTTP_HOST".to_string(), value.clone()));
                vars.push(("SERVER_NAME".to_string(), value.clone()));
            }
            _ => {
                vars.push((format!("HTTP_{}", upper), value.clone()));
            }
        }
    }

    vars
}
