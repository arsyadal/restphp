//! Persistent Zend worker pool.
//!
//! PHP's NTS build has process-global engine state, so this runtime deliberately
//! hosts exactly one Zend VM per RestPHP process. Horizontal scaling belongs to a
//! process supervisor, not multiple threads in this process.

pub use crate::sapi::ExecutionTarget;
use crate::sapi::{PhpEngine, PhpResponse, WorkerRequestContext};
use bytes::Bytes;
use crossbeam_channel::{bounded, Sender, TrySendError};
use std::ffi::CString;
use std::fmt;
use std::os::raw::{c_int, c_void};
use std::path::PathBuf;
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
    pub is_https: bool,
    pub server_port: u16,
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
        Self::start(num_workers, max_requests, max_queue, None)
    }

    /// Boots a script once and waits until it enters `restphp_handle_request`.
    /// Recycling ends this VM; a supervisor must start a replacement process.
    pub fn new_persistent(
        script: PathBuf,
        max_requests: u64,
        max_queue: usize,
    ) -> Result<Self, String> {
        let script = script
            .canonicalize()
            .map_err(|e| format!("Invalid worker script: {e}"))?;
        Self::start(1, max_requests, max_queue, Some(script))
    }

    fn start(
        num_workers: usize,
        max_requests: u64,
        max_queue: usize,
        persistent_script: Option<PathBuf>,
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
                    if let Some(script) = persistent_script {
                        persistent_worker_loop(
                            receiver,
                            engine,
                            script,
                            ready_tx,
                            max_requests,
                            worker_draining,
                            worker_drain_notify,
                        );
                    } else {
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
        self.dispatch_with_transport(
            target,
            method,
            uri,
            query,
            body,
            headers,
            cookie,
            content_type,
            false,
            80,
        )
        .await
    }

    /// Dispatches with trusted listener metadata, never client forwarding headers.
    #[allow(clippy::too_many_arguments)]
    pub async fn dispatch_with_transport(
        &self,
        target: ExecutionTarget,
        method: String,
        uri: String,
        query: String,
        body: Vec<u8>,
        headers: Vec<(String, String)>,
        cookie: Option<String>,
        content_type: Option<String>,
        is_https: bool,
        server_port: u16,
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
            is_https,
            server_port,
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
        let server_vars = build_server_vars(&job);
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
    draining.store(true, Ordering::Release);
    let _ = drain_notify.send(true);
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
        diagnostics: Vec::new(),
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
fn build_server_vars(job: &WorkerJob) -> Vec<(String, String)> {
    let WorkerJob {
        method,
        uri,
        query,
        headers,
        content_type,
        body,
        is_https,
        server_port,
        ..
    } = job;
    let content_length = body.len();
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
    vars.push(("SERVER_PORT".to_string(), server_port.to_string()));
    vars.push((
        "HTTPS".to_string(),
        if *is_https { "on" } else { "off" }.to_string(),
    ));
    vars.push((
        "REQUEST_SCHEME".to_string(),
        if *is_https { "https" } else { "http" }.to_string(),
    ));
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

struct ActivePersistentRequest {
    job: WorkerJob,
    context: WorkerRequestContext,
    method: CString,
    uri: CString,
    query: CString,
    content_type: Option<CString>,
    script: CString,
}

impl ActivePersistentRequest {
    fn new(
        mut job: WorkerJob,
        script: &std::path::Path,
    ) -> Result<Box<Self>, (oneshot::Sender<PhpResponse>, String)> {
        let metadata = (|| -> Result<_, std::ffi::NulError> {
            Ok((
                CString::new(job.method.as_str())?,
                CString::new(job.uri.as_str())?,
                CString::new(job.query.as_str())?,
                job.content_type.as_deref().map(CString::new).transpose()?,
                CString::new(script.to_string_lossy().as_bytes())?,
            ))
        })();
        let (method, uri, query, content_type, script) = match metadata {
            Ok(metadata) => metadata,
            Err(error) => return Err((job.respond_to, error.to_string())),
        };
        let mut variables = build_server_vars(&job);
        variables.push((
            "SCRIPT_FILENAME".to_string(),
            script.to_string_lossy().into_owned(),
        ));
        let mut context =
            WorkerRequestContext::new(Bytes::from(std::mem::take(&mut job.body)), variables);
        if let Some(cookie) = job.cookie.as_deref() {
            context = context.with_cookie(cookie);
        }
        Ok(Box::new(Self {
            job,
            context,
            method,
            uri,
            query,
            content_type,
            script,
        }))
    }

    unsafe fn install(&mut self) {
        crate::ffi::restphp_set_request_info(
            (&mut self.context as *mut WorkerRequestContext).cast(),
            self.method.as_ptr(),
            self.uri.as_ptr().cast_mut(),
            self.query.as_ptr().cast_mut(),
            self.content_type
                .as_ref()
                .map_or(std::ptr::null(), |value| value.as_ptr()),
            self.context.post_body.len() as i64,
            self.script.as_ptr().cast_mut(),
        );
        crate::ffi::restphp_set_cookie_data(
            self.context
                .raw_cookie
                .as_ref()
                .map_or(std::ptr::null_mut(), |value| value.as_ptr().cast_mut()),
        );
    }
}

struct PersistentBridge {
    receiver: crossbeam_channel::Receiver<WorkerCommand>,
    ready: Option<Sender<Result<(), String>>>,
    active: Option<Box<ActivePersistentRequest>>,
    script: PathBuf,
    settled: u64,
    max_requests: u64,
    draining: Arc<AtomicBool>,
    drain_notify: watch::Sender<bool>,
    failed: bool,
}

impl PersistentBridge {
    fn next(&mut self) -> bool {
        if self.failed || (self.max_requests > 0 && self.settled >= self.max_requests) {
            return false;
        }
        if let Some(ready) = self.ready.take() {
            let _ = ready.send(Ok(()));
        }
        loop {
            let job = match self.receiver.recv() {
                Ok(WorkerCommand::Job(job)) => *job,
                Ok(WorkerCommand::Shutdown) | Err(_) => return false,
            };
            if job.respond_to.is_closed() {
                self.settled += 1;
                notify_process_drain(
                    self.settled,
                    self.max_requests,
                    &self.draining,
                    &self.drain_notify,
                );
                if self.max_requests > 0 && self.settled >= self.max_requests {
                    return false;
                }
                continue;
            }
            match ActivePersistentRequest::new(job, &self.script) {
                Ok(mut active) => {
                    TOTAL_REQUESTS.fetch_add(1, Ordering::Relaxed);
                    unsafe {
                        active.install();
                    }
                    self.active = Some(active);
                    return true;
                }
                Err((respond_to, error)) => {
                    let _ = respond_to.send(worker_error_response(0, error));
                    self.settled += 1;
                    notify_process_drain(
                        self.settled,
                        self.max_requests,
                        &self.draining,
                        &self.drain_notify,
                    );
                    if self.max_requests > 0 && self.settled >= self.max_requests {
                        return false;
                    }
                }
            }
        }
    }

    fn finish(&mut self, success: bool) {
        if let Some(active) = self.active.take() {
            let ActivePersistentRequest {
                job, mut context, ..
            } = *active;
            let response = if success {
                PhpResponse {
                    status: context.status_code,
                    content_type: context
                        .content_type()
                        .unwrap_or("text/html; charset=UTF-8")
                        .to_string(),
                    headers: std::mem::take(&mut context.response_headers),
                    body: std::mem::take(&mut context.output_buffer),
                    diagnostics: std::mem::take(&mut context.diagnostics),
                    success: true,
                }
            } else {
                let mut response = worker_error_response(
                    0,
                    "Persistent PHP callback failed; worker is stopping".to_string(),
                );
                response.diagnostics = std::mem::take(&mut context.diagnostics);
                response
            };
            let _ = job.respond_to.send(response);
            self.settled += 1;
            notify_process_drain(
                self.settled,
                self.max_requests,
                &self.draining,
                &self.drain_notify,
            );
        }
        self.failed |= !success;
    }
}

/// # Safety
/// C calls this only with the live worker-thread bridge, without entering Zend.
#[no_mangle]
pub unsafe extern "C" fn restphp_rs_worker_next(bridge: *mut c_void) -> c_int {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        i32::from((&mut *bridge.cast::<PersistentBridge>()).next())
    }))
    .unwrap_or(0)
}

/// # Safety
/// C has detached all request pointers before completing the live bridge's job.
#[no_mangle]
pub unsafe extern "C" fn restphp_rs_worker_finish(bridge: *mut c_void, success: c_int) {
    let bridge = &mut *bridge.cast::<PersistentBridge>();
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| bridge.finish(success != 0)))
        .is_err()
    {
        bridge.failed = true;
    }
}

fn persistent_worker_loop(
    receiver: crossbeam_channel::Receiver<WorkerCommand>,
    engine: PhpEngine,
    script: PathBuf,
    ready: Sender<Result<(), String>>,
    max_requests: u64,
    draining: Arc<AtomicBool>,
    drain_notify: watch::Sender<bool>,
) {
    let mut bridge = PersistentBridge {
        receiver,
        ready: Some(ready),
        active: None,
        script: script.clone(),
        settled: 0,
        max_requests,
        draining,
        drain_notify,
        failed: false,
    };
    let mut bootstrap = WorkerRequestContext::new(Bytes::new(), Vec::new());
    unsafe {
        crate::ffi::restphp_set_worker_bridge((&mut bridge as *mut PersistentBridge).cast());
    }
    let result = engine.execute_request(
        &mut bootstrap,
        &ExecutionTarget::File(script.clone()),
        "GET",
        "/",
        "",
        None,
        Some(&script),
    );
    unsafe {
        crate::ffi::restphp_set_worker_bridge(std::ptr::null_mut());
        crate::ffi::restphp_clear_request_info();
    }
    if let Some(ready) = bridge.ready.take() {
        let _ = ready.send(Err(format!(
            "Persistent bootstrap exited before entering restphp_handle_request: {}",
            match result {
                Ok(response) => String::from_utf8_lossy(&response.diagnostics).into_owned(),
                Err(error) => error,
            },
        )));
    }
    bridge.finish(false);
    bridge.draining.store(true, Ordering::Release);
    let _ = bridge.drain_notify.send(true);
    // Settle accepted jobs even when fatal PHP errors terminate the callback loop.
    for command in bridge.receiver.try_iter() {
        if let WorkerCommand::Job(job) = command {
            let _ = job
                .respond_to
                .send(worker_error_response(0, "PHP worker stopped".to_string()));
        }
    }
}
