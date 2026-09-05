# Original User Request

## Initial Request — 2026-09-05T05:28:02Z

Build RestPHP: a persistent, ultra-high-performance PHP application server and runtime written in Rust that embeds Zend Engine via zero-cost C FFI, featuring a custom SAPI, async HTTP engine (Tokio/Hyper), and persistent worker execution designed to beat FrankenPHP in throughput, RAM footprint, and p99 latency.

Working directory: /home/cads/restphp
Integrity mode: development

## Requirements

### R1. Zend Engine C-FFI Core Embedding
The server must embed the Zend Engine C core directly via zero-cost Rust FFI (`extern "C"`), linking against the system `libphp` / `php-config`. It must be capable of initializing the Zend VM in-memory, evaluating PHP code strings and PHP script files, and capturing execution results.

### R2. Custom SAPI Implementation (`sapi_module_struct`)
Implement a dedicated PHP SAPI in Rust that hooks into PHP's output buffering (`ub_write`), header handling (`sapi_header_op`), and body streaming (`read_post`). PHP script output (e.g., `echo`, response headers, status codes) must stream directly into Rust network buffers without dumping to standard OS stdout.

### R3. High-Concurrency Async HTTP Server & Request Dispatch
The system must expose an asynchronous HTTP server (using Tokio / Hyper / Axum) that receives incoming HTTP requests and dispatches them to the Zend persistent worker. HTTP method, URI, query parameters, headers, cookies, and payload body must be mapped directly to PHP superglobals (`$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, `php://input`).

### R4. Persistent Worker Mode & State Reset
The worker loop must support persistent execution: booting the application script once in memory, executing incoming requests through `php_request_startup()` → handler → `php_request_shutdown()`, cleanly resetting superglobals and running Zend GC between requests to eliminate cross-request memory leaks.

## Acceptance Criteria

### Functionality & Verification
- [ ] Programmatic server startup: Running `cargo run -- serve --port 8080` successfully binds and listens on port 8080.
- [ ] Inline PHP test endpoint: An HTTP GET request to `http://localhost:8080/` executes a PHP script returning `{"status":"ok","engine":"RestPHP"}` with HTTP 200 OK and `Content-Type: application/json`.
- [ ] Superglobal mapping: An HTTP POST request with query params and JSON/form body correctly populates `$_GET`, `$_POST`, and `php://input` in the PHP worker.
- [ ] Clean lifecycle recycling: Consecutive HTTP requests run without crashing or leaking state between subsequent executions.
