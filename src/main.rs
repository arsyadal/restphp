#[cfg(feature = "hot-reload")]
mod reload;

use clap::{Parser, Subcommand};
use restphp::{
    server::{ServerConfig, ServerControl, DEFAULT_MAX_BODY_BYTES, DEFAULT_MAX_QUEUE},
    tls::TlsOptions,
    ExecutionTarget, WorkerHandle,
};
use std::{future::Future, path::Path, sync::Arc, time::Duration};

#[derive(Parser)]
#[command(name = "restphp")]
#[command(about = "The Blazing-Fast, Persistent Application Server & Runtime for PHP")]
#[command(version)]
struct Cli {
    /// Optional PHP script to execute or serve (e.g. `restphp index.php`)
    #[arg(value_name = "FILE")]
    file: Option<String>,
    /// Port to listen on (e.g. `restphp -p 8080`)
    #[arg(short, long)]
    port: Option<u16>,
    /// Host IP address to bind to
    #[arg(long)]
    host: Option<String>,
    /// Number of PHP workers. The current NTS runtime supports only one.
    #[arg(short, long)]
    workers: Option<usize>,
    /// Evaluate PHP code directly from memory (e.g. `restphp -e 'echo 123;'`)
    #[arg(short = 'e', long = "eval")]
    eval: Option<String>,
    /// Restart the server process when PHP sources or application configuration change.
    #[arg(long)]
    watch: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[expect(
    clippy::large_enum_variant,
    reason = "Parse startup options inline without a heap allocation"
)]
#[derive(Subcommand)]
enum Commands {
    /// Start the RestPHP HTTP server
    Serve {
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
        #[arg(short, long, default_value = "public/index.php")]
        entrypoint: String,
        /// Boot a PHP callback worker once instead of executing a file per request.
        #[arg(long, value_name = "PHP")]
        persistent_worker: Option<String>,
        #[command(flatten)]
        tls: TlsOptions,
        /// Must be 1 for the current NTS PHP runtime.
        #[arg(short, long, default_value_t = 1)]
        workers: usize,
        /// Requests before graceful process drain; 0 disables recycling.
        #[arg(short = 'm', long, default_value_t = 0)]
        max_requests: u64,
        /// Maximum accepted request body bytes.
        #[arg(long, default_value_t = DEFAULT_MAX_BODY_BYTES)]
        max_body_bytes: usize,
        /// Maximum jobs waiting for or executing on the PHP worker.
        #[arg(long, default_value_t = DEFAULT_MAX_QUEUE)]
        max_queue: usize,
        /// Maximum seconds to drain HTTP requests and stop the PHP worker during shutdown.
        #[arg(long, default_value_t = 30)]
        shutdown_timeout_secs: u64,
        /// Restart the server process when PHP sources or application configuration change.
        #[arg(long)]
        watch: bool,
    },
    /// Evaluate inline PHP code directly from memory
    Eval { code: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::try_init().ok();
    let cli = Cli::parse();

    if let Some(code) = cli.eval.or_else(|| match &cli.command {
        Some(Commands::Eval { code }) => Some(code.clone()),
        _ => None,
    }) {
        let mut worker =
            WorkerHandle::new_pool(1, 0).map_err(|error| format!("Worker init failed: {error}"))?;
        let response = worker
            .dispatch(
                ExecutionTarget::Inline(code),
                "CLI".into(),
                "/cli".into(),
                "".into(),
                Vec::new(),
            )
            .await
            .map_err(|error| format!("Execution failed: {error}"))?;
        print!("{}", String::from_utf8_lossy(&response.body));
        worker.shutdown();
        return Ok(());
    }

    let options = match cli.command {
        Some(Commands::Serve {
            host,
            port,
            entrypoint,
            persistent_worker,
            tls,
            workers,
            max_requests,
            max_body_bytes,
            max_queue,
            shutdown_timeout_secs,
            watch,
        }) => ServeOptions {
            host,
            port,
            entrypoint,
            persistent_worker,
            tls,
            workers,
            max_requests,
            max_body_bytes,
            max_queue,
            shutdown_timeout_secs,
            watch,
        },
        _ => {
            let entrypoint = if let Some(file) = cli.file {
                file
            } else if Path::new("artisan").exists() {
                println!("✨ Detected Laravel project (artisan found)");
                "public/index.php".to_string()
            } else if Path::new("public/index.php").exists() {
                "public/index.php".to_string()
            } else {
                "index.php".to_string()
            };
            ServeOptions {
                host: cli.host.unwrap_or_else(|| "0.0.0.0".to_string()),
                port: cli.port.unwrap_or(8080),
                entrypoint,
                persistent_worker: None,
                tls: TlsOptions::default(),
                workers: cli.workers.unwrap_or(1),
                max_requests: 0,
                max_body_bytes: DEFAULT_MAX_BODY_BYTES,
                max_queue: DEFAULT_MAX_QUEUE,
                shutdown_timeout_secs: 30,
                watch: cli.watch,
            }
        }
    };

    if options.shutdown_timeout_secs == 0 {
        return Err("--shutdown-timeout-secs must be greater than zero".into());
    }
    if options.watch {
        #[cfg(feature = "hot-reload")]
        return reload::run(Duration::from_secs(options.shutdown_timeout_secs)).await;
        #[cfg(not(feature = "hot-reload"))]
        return Err("--watch requires building RestPHP with the hot-reload feature".into());
    }

    let effective_entrypoint = options
        .persistent_worker
        .as_ref()
        .unwrap_or(&options.entrypoint);
    let entrypoint = std::fs::canonicalize(effective_entrypoint).map_err(|error| {
        format!(
            "Entrypoint '{}' does not exist or cannot be resolved: {error}",
            effective_entrypoint
        )
    })?;
    if !entrypoint.is_file() {
        return Err(format!(
            "Entrypoint '{}' is not a regular file",
            entrypoint.display()
        )
        .into());
    }
    let entrypoint = entrypoint.to_string_lossy().into_owned();

    let config = ServerConfig {
        max_body_bytes: options.max_body_bytes,
        max_queue: options.max_queue,
    };
    config
        .validate()
        .map_err(|error| format!("Invalid server configuration: {error}"))?;
    if options.workers != 1 {
        return Err("the NTS PHP runtime requires --workers 1".into());
    }
    let tls = options.tls.prepare().await?;

    // Zend snapshots process signal handlers during initialization.
    let termination_signal = shutdown_signal();
    let worker = if options.persistent_worker.is_some() {
        WorkerHandle::new_persistent(
            std::path::PathBuf::from(&entrypoint),
            options.max_requests,
            options.max_queue,
        )
    } else {
        WorkerHandle::new_pool_with_queue(options.workers, options.max_requests, options.max_queue)
    }
    .map_err(|error| format!("Worker init failed: {error}"))?;
    let mut drain_notifier = worker.drain_notifier();
    let worker = Arc::new(tokio::sync::RwLock::new(worker));
    let control = ServerControl::new();
    let shutdown_control = control.clone();
    let (shutdown_started_tx, shutdown_started_rx) = tokio::sync::oneshot::channel();
    let shutdown = async move {
        tokio::select! {
            _ = termination_signal => tracing::info!("Shutdown signal received; draining RestPHP"),
            result = drain_notifier.changed() => {
                if result.is_ok() && *drain_notifier.borrow() {
                    tracing::info!("Request recycle limit reached; draining for supervisor restart");
                }
            }
        }
        shutdown_control.close_admission();
        let _ = shutdown_started_tx.send(tokio::time::Instant::now());
    };

    let server = restphp::server::run_server_with_config_and_shutdown(
        (options.host.parse::<std::net::IpAddr>()?, options.port).into(),
        &entrypoint,
        Arc::clone(&worker),
        config,
        control,
        tls,
        shutdown,
    );
    let completion = async {
        let result = server.await;
        // Joining Zend may block even after its HTTP client has disconnected.
        tokio::task::spawn_blocking(move || worker.blocking_write().shutdown()).await?;
        result
    };
    tokio::pin!(completion);

    let shutdown_started = tokio::select! {
        result = &mut completion => return result,
        started = shutdown_started_rx => match started {
            Ok(started) => started,
            // Startup errors can drop the shutdown future before it is polled.
            Err(_) => return completion.await,
        },
    };
    let deadline = shutdown_started + Duration::from_secs(options.shutdown_timeout_secs);

    match tokio::time::timeout_at(deadline, &mut completion).await {
        Ok(result) => result,
        Err(_) => {
            // Neither Zend execution nor an in-progress worker join can be
            // cancelled safely. Exit without waiting for blocking-task teardown.
            tracing::error!(
                "Graceful shutdown deadline exceeded; terminating for supervisor restart"
            );
            std::process::exit(1);
        }
    }
}

struct ServeOptions {
    host: String,
    port: u16,
    entrypoint: String,
    persistent_worker: Option<String>,
    tls: TlsOptions,
    workers: usize,
    max_requests: u64,
    max_body_bytes: usize,
    max_queue: usize,
    shutdown_timeout_secs: u64,
    watch: bool,
}

fn shutdown_signal() -> impl Future<Output = ()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        let mut interrupt = signal(SignalKind::interrupt()).expect("install SIGINT handler");
        async move {
            tokio::select! {
                _ = interrupt.recv() => {},
                _ = terminate.recv() => {},
            }
        }
    }
    #[cfg(windows)]
    {
        use tokio::signal::windows::{ctrl_break, ctrl_c};

        let mut interrupt = ctrl_c().expect("install Ctrl-C handler");
        let mut terminate = ctrl_break().expect("install Ctrl-Break handler");
        async move {
            tokio::select! {
                _ = interrupt.recv() => {},
                _ = terminate.recv() => {},
            }
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        async {
            tokio::signal::ctrl_c()
                .await
                .expect("install Ctrl-C handler");
        }
    }
}
