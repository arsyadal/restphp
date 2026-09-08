//! Persistent Zend worker pool.
//!
//! PHP's NTS build has process-global engine state, so this runtime deliberately
//! hosts exactly one Zend VM per RestPHP process. Horizontal scaling belongs to a
//! process supervisor, not multiple threads in this process.

pub use crate::sapi::ExecutionTarget;
use crate::sapi::{PhpEngine, PhpResponse, WorkerRequestContext};
use bytes::Bytes;
use crossbeam_channel::{bounded, Sender, TrySendError};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::{oneshot, watch};

/// A single unit of work dispatched to the PHP worker thread.
pub struct WorkerJob {
    pub target: ExecutionTarget,
    pub method: String,
    pub uri: String,
    pub query: String,
    pub body: Vec<u8>,
    pub headers: Vec<(String, String)>,
    pub cookie: Option<String>,
    pub content_type: Option<String>,
    pub respond_to: oneshot::Sender<PhpResponse>,
}

enum WorkerCommand {
    Job(Box<WorkerJob>),
    Shutdown,
}

/// A non-blocking admission failure. HTTP callers can map these variants to
/// stable overload/unavailable responses without parsing error strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchError {
    /// The bounded queue has no free slot; callers should return 503.
    QueueFull,
    /// Request recycling or graceful shutdown has closed admission.
    Draining,
    /// The PHP worker did not initialize or is no longer available.
    Unhealthy,
    /// The PHP worker completed the job but the awaiting caller went away.
    ResponseDropped,
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueFull => f.write_str("PHP worker queue is full"),
            Self::Draining => f.write_str("PHP worker is draining for process recycle"),
            Self::Unhealthy => f.write_str("PHP worker is unavailable"),
            Self::ResponseDropped => f.write_str("PHP worker dropped the response channel"),
        }
    }
}

impl std::error::Error for DispatchError {}

#[derive(Debug)]
struct AdmissionState {
    accepting: bool,
    accepted_requests: u64,
    shutdown_enqueued: bool,
}

struct PoolInner {
    sender: Sender<WorkerCommand>,
    admission: Mutex<AdmissionState>,
    join_handle: Mutex<Option<thread::JoinHandle<()>>>,
    max_requests: u64,
    draining: Arc<AtomicBool>,
    drain_notify: watch::Sender<bool>,
}

/// Global request counter shared across all RestPHP worker instances.
static TOTAL_REQUESTS: AtomicU64 = AtomicU64::new(0);

/// Handle to the PHP worker. Clones share admission state but never own the
/// join handle; the original owner shuts down and joins the thread.
pub struct WorkerHandle {
    inner: Arc<PoolInner>,
    owner: bool,
}

impl Clone for WorkerHandle {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            owner: false,
        }
    }
}

impl WorkerHandle {
    /// Starts the one supported NTS PHP worker with the legacy queue default.
    pub fn new_pool(num_workers: usize, max_requests: u64) -> Result<Self, String> {
        Self::new_pool_with_queue(num_workers, max_requests, 256)
    }

    /// Starts the one supported NTS PHP worker with an explicit bounded queue.
    ///
    /// This method waits for PHP module startup before returning. A caller can
    /// therefore bind its TCP listener only after the worker is ready.
    pub fn new_pool_with_queue(
        num_workers: usize,
        max_requests: u64,
        max_queue: usize,
    ) -> Result<Self, String> {
        if num_workers != 1 {
            return Err(
                "RestPHP currently supports exactly one PHP worker per process because the embedded PHP build is NTS; use a process supervisor or container replicas for horizontal scaling."
                    .to_string(),
            );
        }
        if max_queue == 0 {
            return Err("max_queue must be greater than zero".to_string());
        }

        let (sender, receiver) = bounded::<WorkerCommand>(max_queue);
        let (ready_tx, ready_rx) = bounded::<Result<(), String>>(1);
        let (drain_notify, _) = watch::channel(false);
        let worker_drain_notify = drain_notify.clone();
        let draining = Arc::new(AtomicBool::new(false));
        let worker_draining = Arc::clone(&draining);

        let join_handle = thread::Builder::new()
            .name("restphp-worker-0".to_string())
            .spawn(move || match init_engine(0) {
                Ok(engine) => {
                    let _ = ready_tx.send(Ok(()));
                    worker_loop(
                        0,
                        receiver,
                        engine,
                        max_requests,
                        worker_draining,
                        worker_drain_notify,
                    );
                }
                Err(err) => {
                    let _ = ready_tx.send(Err(err));
                }
            })
            .map_err(|e| format!("Failed to spawn PHP worker: {e}"))?;

        match ready_rx.recv() {
            Ok(Ok(())) => {
                println!(
                    "🐘 [RestPHP] PHP worker is ready (max_requests: {}, max_queue: {})",
                    if max_requests == 0 {
                        "unlimited".to_string()
                    } else {
                        max_requests.to_string()
                    },
                    max_queue
                );
                Ok(Self {
                    inner: Arc::new(PoolInner {
                        sender,
                        admission: Mutex::new(AdmissionState {
                            accepting: true,
                            accepted_requests: 0,
                            shutdown_enqueued: false,
                        }),
                        join_handle: Mutex::new(Some(join_handle)),
                        max_requests,
                        draining,
                        drain_notify,
                    }),
                    owner: true,
                })
            }
            Ok(Err(err)) => {
                let _ = join_handle.join();
                Err(format!("Failed to initialize PHP worker: {err}"))
            }
            Err(_) => {
                let _ = join_handle.join();
                Err("PHP worker exited before reporting readiness".to_string())
            }
        }
    }

    /// Convenience: starts one worker with no request-limit recycle.
    pub fn new() -> Result<Self, String> {
        Self::new_pool(1, 0)
    }

    /// Returns a watch receiver that becomes `true` after the final accepted
    /// request for `max_requests` has settled. The server should then stop
    /// accepting connections, call [`Self::shutdown`], and exit so its process
    /// supervisor can replace it.
    pub fn drain_notifier(&self) -> watch::Receiver<bool> {
        self.inner.drain_notify.subscribe()
    }

    /// Whether admission is permanently closed for recycle or shutdown.
    pub fn is_draining(&self) -> bool {
        self.inner.draining.load(Ordering::Acquire)
    }

    /// Returns the configured process recycle limit (zero is unlimited).
    pub fn max_requests(&self) -> u64 {
        self.inner.max_requests
    }

    /// Returns the total number of PHP requests actually executed.
    pub fn total_requests() -> u64 {
        TOTAL_REQUESTS.load(Ordering::Relaxed)
    }

    /// NTS mode supports one PHP worker per process.
    pub fn worker_count(&self) -> usize {
        1
    }

    /// Dispatches a job without ever blocking a Tokio runtime thread on a full
    /// crossbeam queue.
    pub async fn dispatch(
        &self,
        target: ExecutionTarget,
        method: String,
        uri: String,
        query: String,
        body: Vec<u8>,
    ) -> Result<PhpResponse, DispatchError> {
        self.dispatch_with_headers(target, method, uri, query, body, Vec::new(), None, None)
            .await
    }

    /// Dispatches a job with headers, cookie, and content type.
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
    ) -> Result<PhpResponse, DispatchError> {
        let (respond_to, response) = oneshot::channel();
        let job = WorkerJob {
            target,
            method,
            uri,
            query,
            body,
            headers,
            cookie,
            content_type,
            respond_to,
        };

        self.admit(job)?;
        response.await.map_err(|_| DispatchError::ResponseDropped)
    }

    fn admit(&self, job: WorkerJob) -> Result<(), DispatchError> {
        // The mutex is held only across an in-memory `try_send`; it makes the
        // max-request admission limit exact without blocking on worker progress.
        let mut admission = self
            .inner
            .admission
            .lock()
            .map_err(|_| DispatchError::Unhealthy)?;
        if !admission.accepting {
            return Err(if self.inner.draining.load(Ordering::Acquire) {
                DispatchError::Draining
            } else {
                DispatchError::Unhealthy
            });
        }

        match self
            .inner
            .sender
            .try_send(WorkerCommand::Job(Box::new(job)))
        {
            Ok(()) => {
                admission.accepted_requests += 1;
                if self.inner.max_requests > 0
                    && admission.accepted_requests >= self.inner.max_requests
                {
                    // Do not admit request N+1. The worker publishes the drain
                    // event only after request N (and all earlier queued work)
                    // has settled.
                    admission.accepting = false;
                    self.inner.draining.store(true, Ordering::Release);
                }
                Ok(())
            }
            Err(TrySendError::Full(_)) => Err(DispatchError::QueueFull),
            Err(TrySendError::Disconnected(_)) => {
                admission.accepting = false;
                Err(DispatchError::Unhealthy)
            }
        }
    }

    /// Stops admission, drains queued jobs in FIFO order, and joins the PHP
    /// thread. It is idempotent and safe to call while non-owner clones exist.
    pub fn shutdown(&mut self) {
        let enqueue_shutdown = {
            let mut admission = match self.inner.admission.lock() {
                Ok(admission) => admission,
                Err(_) => return,
            };
            admission.accepting = false;
            self.inner.draining.store(true, Ordering::Release);
            if admission.shutdown_enqueued {
                false
            } else {
                admission.shutdown_enqueued = true;
                true
            }
        };

        if enqueue_shutdown {
            // This blocking send happens only during owner shutdown. It queues
            // the sentinel after all accepted jobs, which gives those jobs a
            // deterministic drain before the Zend VM is torn down.
            let _ = self.inner.sender.send(WorkerCommand::Shutdown);
        }

        if let Ok(mut join_handle) = self.inner.join_handle.lock() {
            if let Some(handle) = join_handle.take() {
                let _ = handle.join();
            }
        }
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        if self.owner {
            self.shutdown();
        }
    }
}

/// Main loop for the one PHP worker thread.
fn worker_loop(
    id: usize,
    receiver: crossbeam_channel::Receiver<WorkerCommand>,
    engine: PhpEngine,
    max_requests: u64,
    draining: Arc<AtomicBool>,
    drain_notify: watch::Sender<bool>,
) {
    let mut settled_requests = 0_u64;

    while let Ok(command) = receiver.recv() {
        let job = match command {
            WorkerCommand::Job(job) => *job,
            WorkerCommand::Shutdown => break,
        };

        // A client that disconnected while the job was queued must not trigger
        // PHP user code after the response receiver has been dropped.
        if job.respond_to.is_closed() {
            settled_requests += 1;
            notify_process_drain(settled_requests, max_requests, &draining, &drain_notify);
            continue;
        }

        TOTAL_REQUESTS.fetch_add(1, Ordering::Relaxed);
        let server_vars = build_server_vars(
            &job.method,
            &job.uri,
            &job.query,
            &job.headers,
            &job.content_type,
            job.body.len(),
        );
        let mut ctx = WorkerRequestContext::new(Bytes::copy_from_slice(&job.body), server_vars);
        if let Some(ref cookie) = job.cookie {
            ctx = ctx.with_cookie(cookie);
        }

        let response = execute_job(&engine, id, &mut ctx, &job);
        let _ = job.respond_to.send(response);
        settled_requests += 1;
        notify_process_drain(settled_requests, max_requests, &draining, &drain_notify);
    }

    tracing::info!("🛑 [Worker-{id}] Shutting down after {settled_requests} settled jobs.");
    // PhpEngine::drop() tears down the PHP module after every accepted job has drained.
}

fn notify_process_drain(
    settled_requests: u64,
    max_requests: u64,
    draining: &AtomicBool,
    drain_notify: &watch::Sender<bool>,
) {
    if max_requests > 0 && settled_requests >= max_requests && draining.load(Ordering::Acquire) {
        let _ = drain_notify.send(true);
    }
}

fn execute_job(
    engine: &PhpEngine,
    id: usize,
    ctx: &mut WorkerRequestContext,
    job: &WorkerJob,
) -> PhpResponse {
    match job.target {
        ExecutionTarget::File(ref path) => {
            let target = ExecutionTarget::File(path.clone());
            engine
                .execute_request(
                    ctx,
                    &target,
                    &job.method,
                    &job.uri,
                    &job.query,
                    job.content_type.as_deref(),
                    Some(path.as_path()),
                )
                .unwrap_or_else(|err| worker_error_response(id, err))
        }
        ExecutionTarget::Inline(ref code) | ExecutionTarget::Code(ref code) => {
            let target = ExecutionTarget::Inline(code.clone());
            engine
                .execute_request(
                    ctx,
                    &target,
                    &job.method,
                    &job.uri,
                    &job.query,
                    job.content_type.as_deref(),
                    None,
                )
                .unwrap_or_else(|err| worker_error_response(id, err))
        }
    }
}

fn worker_error_response(id: usize, err: String) -> PhpResponse {
    PhpResponse {
        status: 500,
        content_type: "text/plain".to_string(),
        headers: Vec::new(),
        body: format!("Worker-{id} error: {err}").into_bytes(),
        success: false,
    }
}

fn init_engine(id: usize) -> Result<PhpEngine, String> {
    tracing::info!("🐘 [Worker-{id}] Initializing dedicated PHP VM...");
    let engine = PhpEngine::init()?;
    tracing::info!("✅ [Worker-{id}] PHP VM initialized and ready.");
    Ok(engine)
}

/// Builds `$_SERVER` CGI variables from HTTP request metadata.
fn build_server_vars(
    method: &str,
    uri: &str,
    query: &str,
    headers: &[(String, String)],
    content_type: &Option<String>,
    content_length: usize,
) -> Vec<(String, String)> {
    let mut vars = Vec::with_capacity(headers.len() + 16);
    let request_uri = if query.is_empty() || uri.contains('?') {
        uri.to_string()
    } else {
        format!("{uri}?{query}")
    };

    vars.push(("REQUEST_METHOD".to_string(), method.to_string()));
    vars.push(("REQUEST_URI".to_string(), request_uri));
    vars.push(("QUERY_STRING".to_string(), query.to_string()));
    vars.push(("SERVER_SOFTWARE".to_string(), "RestPHP/0.1.0".to_string()));
    vars.push(("SERVER_PROTOCOL".to_string(), "HTTP/1.1".to_string()));
    vars.push(("GATEWAY_INTERFACE".to_string(), "CGI/1.1".to_string()));
    vars.push(("SERVER_NAME".to_string(), "restphp".to_string()));
    vars.push(("SERVER_PORT".to_string(), "80".to_string()));
    vars.push(("CONTENT_LENGTH".to_string(), content_length.to_string()));

    if let Some(ct) = content_type {
        vars.push(("CONTENT_TYPE".to_string(), ct.clone()));
    }

    for (name, value) in headers {
        let upper = name.to_uppercase().replace('-', "_");
        match upper.as_str() {
            "CONTENT_TYPE" | "CONTENT_LENGTH" => {}
            "HOST" => vars.push(("HTTP_HOST".to_string(), value.clone())),
            _ => vars.push((format!("HTTP_{upper}"), value.clone())),
        }
    }

    vars
}
