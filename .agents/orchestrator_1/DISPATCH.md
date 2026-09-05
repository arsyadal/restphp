## 2026-09-05T05:28:37Z

You are the Project Orchestrator for the RestPHP project.
Your working directory is /home/cads/restphp/.agents/orchestrator_1/.
The project workspace is /home/cads/restphp/.
The original user request is recorded in /home/cads/restphp/.agents/ORIGINAL_REQUEST.md.

Project Goal:
Build RestPHP: a persistent, ultra-high-performance PHP application server and runtime written in Rust that embeds Zend Engine via zero-cost C FFI, featuring a custom SAPI, async HTTP engine (Tokio/Hyper), and persistent worker execution designed to beat FrankenPHP in throughput, RAM footprint, and p99 latency.

Requirements:
- R1. Zend Engine C-FFI Core Embedding: embed Zend Engine C core via zero-cost Rust FFI (`extern "C"`), link against system libphp / php-config, initialize Zend VM in-memory, evaluate PHP code strings and script files, capture execution results.
- R2. Custom SAPI Implementation (`sapi_module_struct`): dedicated PHP SAPI in Rust hooking into `ub_write`, `sapi_header_op`, `read_post`, streaming output directly into Rust network buffers without dumping to OS stdout.
- R3. High-Concurrency Async HTTP Server & Request Dispatch: Tokio / Hyper / Axum server receiving requests, dispatching to persistent worker, mapping method, URI, query parameters, headers, cookies, body to `$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, `php://input`.
- R4. Persistent Worker Mode & State Reset: boot script once in memory, execute via `php_request_startup()` -> handler -> `php_request_shutdown()`, clean state reset and Zend GC between requests.

Acceptance Criteria:
1. `cargo run -- serve --port 8080` binds and listens on port 8080.
2. GET `http://localhost:8080/` returns `{"status":"ok","engine":"RestPHP"}` (200 OK, `Content-Type: application/json`).
3. POST request with query params and JSON/form body populates `$_GET`, `$_POST`, and `php://input`.
4. Clean lifecycle recycling: consecutive requests run without crashing or leaking state.
