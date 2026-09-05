use clap::{Parser, Subcommand};
use restphp::{ExecutionTarget, WorkerHandle};

#[derive(Parser)]
#[command(name = "restphp")]
#[command(about = "The Blazing-Fast, Persistent Application Server & Runtime for PHP", long_about = None)]
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

    /// Number of worker threads
    #[arg(short, long)]
    workers: Option<usize>,

    /// Evaluate PHP code directly from memory (e.g. `restphp -e 'echo 123;'`)
    #[arg(short = 'e', long = "eval")]
    eval: Option<String>,

    /// Watch for PHP file changes and hot-reload
    #[arg(long)]
    watch: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the high-performance RestPHP async HTTP server
    Serve {
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        #[arg(short, long, default_value_t = 8080)]
        port: u16,

        #[arg(short, long, default_value = "public/index.php")]
        entrypoint: String,

        /// Number of persistent Zend worker OS threads (default 1 for NTS PHP)
        #[arg(short, long, default_value_t = 1)]
        workers: usize,

        /// Maximum requests per worker before recycling (0 = unlimited)
        #[arg(short = 'm', long, default_value_t = 10000)]
        max_requests: u64,

        /// Watch for PHP file changes and hot-reload
        #[arg(long)]
        watch: bool,
    },
    /// Evaluate inline PHP code directly from memory
    Eval {
        /// PHP code string to execute
        code: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // 1. Direct inline eval via `restphp -e "..."` or `restphp eval "..."`
    if let Some(code) = cli.eval.or_else(|| {
        if let Some(Commands::Eval { code }) = &cli.command {
            Some(code.clone())
        } else {
            None
        }
    }) {
        let worker =
            WorkerHandle::new_pool(1, 10000).map_err(|e| format!("Worker init failed: {}", e))?;
        let resp = worker
            .dispatch(
                ExecutionTarget::Inline(code),
                "CLI".into(),
                "/cli".into(),
                "".into(),
                vec![],
            )
            .await
            .map_err(|e| format!("Execution failed: {}", e))?;

        let body_str = String::from_utf8_lossy(&resp.body);
        print!("{}", body_str);
        return Ok(());
    }

    // 2. Resolve host, port, workers, and entrypoint
    let (host, port, entrypoint, workers, max_requests, watch) = match cli.command {
        Some(Commands::Serve {
            host,
            port,
            entrypoint,
            workers,
            max_requests,
            watch,
        }) => (host, port, entrypoint, workers, max_requests, watch),
        _ => {
            let host = cli.host.unwrap_or_else(|| "0.0.0.0".to_string());
            let workers = cli.workers.unwrap_or(1);
            let max_requests = 10000;
            let watch = cli.watch;

            // Smart entrypoint detection (like Bun)
            let (entrypoint, default_port) = if let Some(ref file) = cli.file {
                (file.clone(), 8080)
            } else if std::path::Path::new("artisan").exists() {
                println!("✨ Detected Laravel project (artisan found)");
                if std::path::Path::new("octane/bin/restphp-worker.php").exists() {
                    ("octane/bin/restphp-worker.php".to_string(), 8000)
                } else {
                    ("public/index.php".to_string(), 8000)
                }
            } else if std::path::Path::new("public/index.php").exists() {
                ("public/index.php".to_string(), 8080)
            } else if std::path::Path::new("index.php").exists() {
                ("index.php".to_string(), 8080)
            } else {
                ("public/index.php".to_string(), 8080)
            };

            let port = cli.port.unwrap_or(default_port);
            (host, port, entrypoint, workers, max_requests, watch)
        }
    };

    let worker = WorkerHandle::new_pool(workers, max_requests)
        .map_err(|e| format!("Worker init failed: {}", e))?;
    let worker = std::sync::Arc::new(tokio::sync::RwLock::new(worker));

    if watch {
        #[cfg(feature = "hot-reload")]
        {
            let watcher_worker = worker.clone();
            tokio::task::spawn_blocking(move || {
                use notify::{RecursiveMode, Watcher};
                let (tx, rx) = std::sync::mpsc::channel();
                if let Ok(mut watcher) = notify::recommended_watcher(tx) {
                    let _ = watcher.watch(std::path::Path::new("."), RecursiveMode::Recursive);
                    for res in rx {
                        match res {
                            Ok(event) => {
                                if event
                                    .paths
                                    .iter()
                                    .any(|p| p.extension().is_some_and(|ext| ext == "php"))
                                {
                                    println!("🔄 [RestPHP] Detected PHP file change. Recycling workers...");
                                    if let Ok(new_worker) =
                                        restphp::WorkerHandle::new_pool(workers, max_requests)
                                    {
                                        tokio::runtime::Handle::current().block_on(async {
                                            *watcher_worker.write().await = new_worker;
                                        });
                                    }
                                }
                            }
                            Err(e) => println!("watch error: {:?}", e),
                        }
                    }
                }
            });
        }
        #[cfg(not(feature = "hot-reload"))]
        {
            println!("⚠️ [RestPHP] Watch mode requested but 'hot-reload' feature is not enabled.");
        }
    }

    // Auto-create sample file if entrypoint doesn't exist yet
    if !std::path::Path::new(&entrypoint).exists() {
        if let Some(parent) = std::path::Path::new(&entrypoint).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let sample_code = r#"<?php
header("Content-Type: application/json");
echo json_encode([
    "status" => "ok",
    "engine" => "RestPHP",
    "version" => "0.1.0",
    "php_version" => PHP_VERSION,
    "method" => $_SERVER["REQUEST_METHOD"] ?? "GET",
    "uri" => $_SERVER["REQUEST_URI"] ?? "/",
    "query" => $_GET,
    "time" => microtime(true),
], JSON_PRETTY_PRINT);
"#;
        let _ = std::fs::write(&entrypoint, sample_code);
        println!("✨ Created sample entrypoint file at: {}", entrypoint);
    }

    let entrypoint = std::fs::canonicalize(&entrypoint)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or(entrypoint);

    restphp::server::run_http_server(&host, port, &entrypoint, worker).await?;

    Ok(())
}
