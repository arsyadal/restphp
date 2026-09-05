use crate::sapi::{PhpEngine, PhpResponse};
use crossbeam_channel::{bounded, Sender};
use std::thread;
use tokio::sync::oneshot;

pub enum ExecutionTarget {
    File(String),
    Code(String),
}

pub struct WorkerJob {
    pub target: ExecutionTarget,
    pub method: String,
    pub uri: String,
    pub query: String,
    pub body: Vec<u8>,
    pub respond_to: oneshot::Sender<PhpResponse>,
}

#[derive(Clone)]
pub struct WorkerHandle {
    sender: Sender<WorkerJob>,
}

impl WorkerHandle {
    pub fn new() -> Result<Self, String> {
        let (sender, receiver) = bounded::<WorkerJob>(1024);

        thread::Builder::new()
            .name("restphp-worker-0".into())
            .spawn(move || {
                tracing::info!("🐘 [Zend Worker] Initializing dedicated PHP VM in thread...");
                let engine = match PhpEngine::init() {
                    Ok(e) => e,
                    Err(err) => {
                        tracing::error!("❌ Failed to initialize PhpEngine: {}", err);
                        return;
                    }
                };
                tracing::info!("✅ [Zend Worker] PHP VM initialized and ready for persistent requests.");

                while let Ok(job) = receiver.recv() {
                    let resp = match job.target {
                        ExecutionTarget::File(ref path) => engine.execute_file(
                            path,
                            &job.method,
                            &job.uri,
                            &job.query,
                            &job.body,
                        ),
                        ExecutionTarget::Code(ref code) => engine.execute_string(
                            code,
                            &job.method,
                            &job.uri,
                            &job.query,
                            &job.body,
                        ),
                    };

                    let _ = job.respond_to.send(resp);
                }

                tracing::info!("🛑 [Zend Worker] Shutting down PHP VM...");
            })
            .map_err(|e| e.to_string())?;

        Ok(WorkerHandle { sender })
    }

    pub async fn dispatch(
        &self,
        target: ExecutionTarget,
        method: String,
        uri: String,
        query: String,
        body: Vec<u8>,
    ) -> Result<PhpResponse, String> {
        let (tx, rx) = oneshot::channel();
        let job = WorkerJob {
            target,
            method,
            uri,
            query,
            body,
            respond_to: tx,
        };

        self.sender
            .send(job)
            .map_err(|e| format!("Failed to dispatch job to worker: {}", e))?;

        rx.await
            .map_err(|_| "Worker dropped response channel".to_string())
    }
}
