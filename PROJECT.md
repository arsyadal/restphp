# Project: RestPHP

Persistent, ultra-high-performance PHP application server and runtime written in Rust embedding Zend Engine via zero-cost C FFI.

## Architecture

RestPHP uses a decoupled Actor architecture separating multi-threaded asynchronous network I/O from the single-threaded Non-Thread-Safe (NTS) Zend Engine VM:

```
                  ┌─────────────────────────────────────────┐
                  │       Client HTTP / TCP Requests        │
                  └────────────────────┬────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                          Tokio / Axum HTTP Server                               │
│  - Multi-threaded async network I/O (epoll / kqueue)                           │
│  - TCP listener bound to configured port (default 8080)                         │
│  - Parses HTTP method, URI, query parameters, headers, cookies, body            │
│  - Routes requests and packages WorkerRequestContext                            │
│  - Generates tokio::sync::oneshot channel for response streaming                │
└──────────────────────────────────────┬──────────────────────────────────────────┘
                                       │
                      crossbeam_channel (ZendWorkerTask)
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         Dedicated Zend Worker Thread                            │
│  - Hosts in-memory Zend Engine VM instance (PHP 8.4 NTS)                        │
│  - Executes synchronous request lifecycle loop:                                 │
│      1. Sets SG(server_context) and SG(request_info) (method, URI, query, etc.) │
│      2. php_request_startup()                                                   │
│      3. Safe evaluation (zend_first_try / zend_catch bailout protection)        │
│         - Executes external .php script or inline PHP string                    │
│         - Output streamed via ub_write into buffer                              │
│         - Headers & status captured via send_headers                            │
│         - Body read via read_post for $_POST & php://input                      │
│         - Cookies read via read_cookies for $_COOKIE                            │
│         - Server variables registered via register_server_variables for $_SERVER│
│      4. php_request_shutdown(NULL) & Zend GC collection (zend_gc_collect_cycles)│
│      5. Dispatches ZendWorkerResponse back through oneshot channel              │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Feature Inventory

Every feature from the Survey phase appears here with its assigned milestone.

| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | System Libphp Linker | Build script and linker configuration for system `libphp.so` via `php-config` | M1 | Survey |
| 2 | Signal & Environment Init | Process signal initialization (`SIGPIPE` ignore, Zend signals) | M1 | Survey |
| 3 | Zend VM Module Startup | Initialize Zend VM and built-in modules via `sapi_startup` & `php_module_startup` | M1 | Survey |
| 4 | Zend VM Module Shutdown | Teardown Zend VM via `php_module_shutdown` & `sapi_shutdown` | M1 | Survey |
| 5 | Request Startup Lifecycle | Prepare per-request memory manager & symbol table via `php_request_startup` | M1 | Survey |
| 6 | Request Shutdown Lifecycle | Tear down request symbol table, close streams, run Zend GC via `php_request_shutdown` | M1 | Survey |
| 7 | In-Memory String Evaluation | Safe string evaluation via `zend_eval_stringl` with bailout protection | M1 | Survey |
| 8 | PHP Script File Execution | Execute `.php` script files via `php_execute_script` / `zend_file_handle` | M1 | Survey |
| 9 | Dedicated SAPI Registration | Define and register `restphp_sapi_module` (`sapi_module_struct`, 280 bytes) | M1 | Survey |
| 10 | Zero-Stdout Output Capturing | Direct `ub_write` callback streaming output chunks into Rust buffer | M1 | Survey |
| 11 | Response Headers & Status | `send_headers` callback extracting `http_response_code` and headers from `sapi_headers` | M1 | Survey |
| 12 | Header Handler Hook | `header_handler` callback handling dynamic `header()` and `header_remove()` | M1 | Survey |
| 13 | Request Body Reader | `read_post` callback streaming incoming HTTP body to `php://input` & `$_POST` | M1 | Survey |
| 14 | Cookie String Reader | `read_cookies` callback supplying cookie string to avoid null-pointer SIGSEGV | M1 | Survey |
| 15 | Server Variables Registration | `register_server_variables` callback populating `$_SERVER` via `php_register_variable` | M1 | Survey |
| 16 | Default Post Reader | SAPI integration with `php_default_post_reader` | M1 | Survey |
| 17 | Data Treatment Hook | SAPI integration with `php_default_treat_data` | M1 | Survey |
| 18 | Bailout & Exit Protection | C shim with `zend_first_try` / `zend_catch` preventing `longjmp` over Rust frames | M1 | Survey |
| 19 | Request Dispatch Queue | Crossbeam lock-free MPSC channel channeling tasks from Tokio to Zend Worker | M2 | Survey |
| 20 | Dedicated Worker Thread | Dedicated OS thread hosting Zend VM, processing requests sequentially | M2 | Survey |
| 21 | `$_SERVER` Full CGI Mapping | Full mapping of method, URI, query, server name, port, remote addr, headers | M2 | Survey |
| 22 | `$_GET` Query Parameter Parsing | Automatic parsing of query string into `$_GET` (supporting arrays) | M2 | Survey |
| 23 | `$_POST` Form Data Parsing | Automatic parsing of form urlencoded and multipart data into `$_POST` | M2 | Survey |
| 24 | `$_COOKIE` Header Parsing | Automatic parsing of `Cookie` header into `$_COOKIE` | M2 | Survey |
| 25 | `php://input` Raw Stream | Clean exposure of raw request body via `php://input` for JSON / XML payloads | M2 | Survey |
| 26 | Clean Lifecycle Reset & GC | Consecutive request isolation without variable leaks; `zend_gc_collect_cycles` | M2 | Survey |
| 27 | Auto-Recycling by Requests | Worker recycling after configurable maximum requests | M2 | Survey |
| 28 | Auto-Recycling by Memory | Worker recycling when memory threshold is exceeded | M2 | Survey |
| 29 | Async HTTP Server | Tokio + Axum / Hyper HTTP listener handling concurrent HTTP connections | M3 | Survey |
| 30 | CLI `serve` Subcommand | Clap CLI subcommand parsing `serve` command and flags | M3 | Survey |
| 31 | `--port` & `--host` Options | Command-line configuration of bind address and port (default 8080) | M3 | Survey |
| 32 | `--script` Option | Command-line configuration of PHP entrypoint script | M3 | Survey |
| 33 | Default Inline Test Endpoint | GET `/` returns `{"status":"ok","engine":"RestPHP"}` (200 OK, application/json) | M3 | Survey |
| 34 | E2E Comprehensive Test Suite | 4-tier test suite verifying features, boundaries, pairwise combos, and workloads | M4 | Survey |

## Milestones

| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M1 | Core C-FFI & Custom SAPI Subsystem | Features 1-18: `c/sapi.c` shim, `build.rs`, Rust FFI bindings, `sapi_module_struct`, `ub_write`, `send_headers`, `read_post`, `read_cookies`, `register_server_variables`, bailout protection, in-memory string and file evaluation | None | PLANNED |
| M2 | Persistent Zend Worker Actor & State Lifecycle | Features 19-28: Dedicated OS worker thread, `ZendWorkerTask` queue, `WorkerRequestContext`, `$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, `php://input` mapping, clean request recycling, Zend GC | M1 | PLANNED |
| M3 | CLI & High-Concurrency Async HTTP Server | Features 29-33: Clap CLI `serve --port 8080`, Tokio/Axum HTTP listener, request routing, async dispatch to worker, response streaming, default GET `/` JSON test endpoint | M2 | PLANNED |
| M4 | E2E Testing & Final Acceptance Verification | Feature 34: Requirement-driven E2E test suite (Tiers 1-4), 100% test pass verification, and Tier 5 adversarial hardening | M3 | PLANNED |

## Interface Contracts

### M1 (C-FFI & SAPI) ↔ M2 (Worker Actor)

- **Header / Functions in `c/sapi.c` & Rust FFI**:
  ```c
  void restphp_sapi_init(void);
  void restphp_sapi_teardown(void);
  void restphp_set_request_info(
      void *server_context,
      const char *method,
      char *uri,
      char *query_string,
      const char *content_type,
      int64_t content_length,
      char *path_translated
  );
  int restphp_request_startup_safe(void);
  int restphp_request_shutdown_safe(void);
  int restphp_eval_string_safe(const char *code, const char *desc);
  int restphp_execute_script_safe(const char *filepath);
  ```

- **Rust SAPI Context**:
  ```rust
  pub struct WorkerRequestContext {
      pub post_body: bytes::Bytes,
      pub post_offset: usize,
      pub status_code: u16,
      pub response_headers: Vec<(String, String)>,
      pub output_buffer: Vec<u8>,
      pub server_vars: Vec<(String, String)>,
  }
  ```

### M2 (Worker Actor) ↔ M3 (HTTP Server)

- **Worker Task**:
  ```rust
  pub enum WorkerScriptTarget {
      Inline(String),
      File(std::path::PathBuf),
  }

  pub struct ZendWorkerTask {
      pub method: String,
      pub uri: String,
      pub query_string: Option<String>,
      pub headers: Vec<(String, String)>,
      pub body: bytes::Bytes,
      pub remote_addr: Option<String>,
      pub script_target: WorkerScriptTarget,
      pub response_tx: tokio::sync::oneshot::Sender<ZendWorkerResponse>,
  }

  pub struct ZendWorkerResponse {
      pub status_code: u16,
      pub headers: Vec<(String, String)>,
      pub body: Vec<u8>,
  }
  ```

- **Worker Handle**:
  ```rust
  #[derive(Clone)]
  pub struct ZendWorkerHandle {
      sender: crossbeam_channel::Sender<ZendWorkerTask>,
  }
  ```

### M3 (HTTP Server) ↔ User / E2E Client

- **CLI invocation**: `cargo run -- serve --port <PORT> [--host <HOST>] [--script <PATH>]`
- **GET `http://localhost:<PORT>/`**: HTTP 200 OK, `Content-Type: application/json`, Body: `{"status":"ok","engine":"RestPHP"}`
- **POST `http://localhost:<PORT>/path?query`**: Form or JSON payload parsed and exposed to script.

## Code Layout

```
/home/cads/restphp/
├── Cargo.toml                  # Dependencies: tokio, axum, hyper, crossbeam-channel, clap, bytes, cc
├── build.rs                    # Links libphp, compiles c/sapi.c
├── c/
│   └── sapi.c                  # C SAPI shim, bailout catchers, Zend macros
├── src/
│   ├── lib.rs                  # Library entrypoint
│   ├── main.rs                 # Binary CLI entrypoint (clap parser)
│   ├── ffi/
│   │   ├── mod.rs              # C FFI bindings and prototypes
│   │   └── types.rs            # Struct declarations (sapi_module, etc.)
│   ├── sapi/
│   │   ├── mod.rs              # SAPI module interface
│   │   ├── callbacks.rs        # Rust callbacks (ub_write, send_headers, read_post, etc.)
│   │   └── context.rs          # WorkerRequestContext
│   ├── worker/
│   │   ├── mod.rs              # Dedicated OS worker thread
│   │   ├── lifecycle.rs        # php_request_startup / shutdown loop
│   │   └── superglobals.rs     # $_SERVER, $_GET, $_POST, $_COOKIE injector
│   └── server/
│       ├── mod.rs              # Async HTTP server
│       ├── routes.rs           # Request handling & dispatch
│       └── options.rs          # Server configuration
└── tests/
    ├── e2e_test_suite.rs       # Comprehensive E2E test suite
    └── fixtures/               # Test PHP scripts
```
