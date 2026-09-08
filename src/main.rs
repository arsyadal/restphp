use clap::{Parser, Subcommand};
use restphp::{
    server::{ServerConfig, ServerControl, DEFAULT_MAX_BODY_BYTES, DEFAULT_MAX_QUEUE},
    ExecutionTarget, WorkerHandle,
};
use std::{path::Path, sync::Arc, time::Duration};

#[derive(Parser)]
#[command(name = "restphp")]
#[command(about = "The Blazing-Fast, Persistent Application Server & Runtime for PHP")]
#[command(version = "0.1.0")]
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
    /// Watch mode is temporarily unavailable while process-safe reload is redesigned.
    #[arg(long)]
    watch: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

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
        /// Maximum seconds to wait for accepted requests during shutdown.
        #[arg(long, default_value_t = 30)]
        shutdown_timeout_secs: u64,
        /// Watch mode is temporarily unavailable while process-safe reload is redesigned.
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
                workers: cli.workers.unwrap_or(1),
                max_requests: 0,
                max_body_bytes: DEFAULT_MAX_BODY_BYTES,
                max_queue: DEFAULT_MAX_QUEUE,
                shutdown_timeout_secs: 30,
                watch: cli.watch,
            }
        }
    };

    if options.watch {
        return Err(
            "--watch is temporarily unavailable while process-safe reload is redesigned".into(),
        );
    }
    if options.shutdown_timeout_secs == 0 {
        return Err("--shutdown-timeout-secs must be greater than zero".into());
    }

    let entrypoint = std::fs::canonicalize(&options.entrypoint).map_err(|error| {
        format!(
            "Entrypoint '{}' does not exist or cannot be resolved: {error}",
            options.entrypoint
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

    let worker =
        WorkerHandle::new_pool_with_queue(options.workers, options.max_requests, options.max_queue)
            .map_err(|error| format!("Worker init failed: {error}"))?;
    let mut drain_notifier = worker.drain_notifier();
    let worker = Arc::new(tokio::sync::RwLock::new(worker));
    let control = ServerControl::new();
    let shutdown_control = control.clone();
    let shutdown = async move {
        tokio::select! {
            _ = shutdown_signal() => tracing::info!("Shutdown signal received; draining RestPHP"),
            result = drain_notifier.changed() => {
                if result.is_ok() && *drain_notifier.borrow() {
                    tracing::info!("Request recycle limit reached; draining for supervisor restart");
                }
            }
        }
        shutdown_control.close_admission();
    };

    let server = restphp::server::run_http_server_with_config_and_shutdown(
        &options.host,
        options.port,
        &entrypoint,
        Arc::clone(&worker),
        config,
        control,
        shutdown,
    );
    let outcome =
        tokio::time::timeout(Duration::from_secs(options.shutdown_timeout_secs), server).await;

    match outcome {
        Ok(result) => {
            worker.write().await.shutdown();
            result?;
            Ok(())
        }
        Err(_) => {
            // PHP execution cannot be safely cancelled from another thread. Do
            // not call `shutdown()` here: joining an unbounded user script
            // would defeat the deadline. A supervisor replaces this process.
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
    workers: usize,
    max_requests: u64,
    max_body_bytes: usize,
    max_queue: usize,
    shutdown_timeout_secs: u64,
    watch: bool,
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl-C handler");
    }
}
