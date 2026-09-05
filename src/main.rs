use clap::{Parser, Subcommand};
use restphp::worker::{ExecutionTarget, WorkerHandle};

#[derive(Parser)]
#[command(name = "restphp")]
#[command(about = "The Blazing-Fast, Persistent Application Server & Runtime for PHP", long_about = None)]
#[command(version = "0.1.0")]
struct Cli {
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

    match cli.command {
        Some(Commands::Serve {
            host,
            port,
            entrypoint,
        }) => {
            println!("🚀 [RestPHP] Initializing async runtime & Zend Worker Pool...");
            let worker = WorkerHandle::new().map_err(|e| format!("Worker init failed: {}", e))?;

            // Verify entrypoint exists
            if !std::path::Path::new(&entrypoint).exists() {
                // Auto-create a sample index.php if none exists
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

            restphp::server::run_http_server(&host, port, &entrypoint, worker).await?;
        }
        Some(Commands::Eval { code }) => {
            let worker = WorkerHandle::new().map_err(|e| format!("Worker init failed: {}", e))?;
            let resp = worker
                .dispatch(
                    ExecutionTarget::Code(code),
                    "CLI".into(),
                    "/cli".into(),
                    "".into(),
                    vec![],
                )
                .await
                .map_err(|e| format!("Execution failed: {}", e))?;

            let body_str = String::from_utf8_lossy(&resp.body);
            print!("{}", body_str);
        }
        None => {
            // Default action if no subcommand is given: print banner and serve
            println!("🦀🐘 RestPHP v0.1.0 — Persistent Application Server for PHP");
            println!("Run `restphp --help` for available options.\n");

            let worker = WorkerHandle::new().map_err(|e| format!("Worker init failed: {}", e))?;
            let entrypoint = "public/index.php".to_string();

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
            }

            restphp::server::run_http_server("0.0.0.0", 8080, &entrypoint, worker).await?;
        }
    }

    Ok(())
}
