# Project Roadmap & Milestones

The development of **RestPHP** follows a strict multi-phase architecture blueprint designed for production resilience and world-record performance.

---

## Progress Overview

- [x] **Milestone 1: Zend Engine C-FFI Core Embedding**
  - Toolchain setup (`libphp-embed`, `clang`, `rustc`).
  - Raw C bindings for `php_embed_init`, `zend_eval_string`, and `php_embed_shutdown`.
  - In-memory execution verified.

- [x] **Milestone 2: Custom SAPI Implementation**
  - Dedicated `sapi_module_struct` implementation.
  - Intercepted output buffering (`ub_write`) to stream directly to Rust memory buffers.
  - Header capturing (`send_headers`) for HTTP status codes and custom headers.
  - Superglobal injection (`$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, `php://input`).

- [x] **Milestone 3: Async Tokio HTTP Server & REST Engine**
  - Tokio + Axum async HTTP listener.
  - Lock-free crossbeam worker dispatcher.
  - CLI commands (`serve`, `eval`).
  - Verified with live HTTP traffic.

- [x] **Milestone 4: Persistent Worker Loop & Laravel Octane Adapter**
  - Per-request lifecycle (`php_request_startup` -> handler -> `php_request_shutdown`).
  - State reset verified across 60/60 E2E tests (100% pass rate).
  - Released `restphp/octane` Composer package for 1st-class Laravel integration.

- [x] **Milestone 5: Benchmarking Suite & TechEmpower**
  - Automated comparative benchmarks vs FrankenPHP and Swoole.
  - Hot code reload via `notify` crate (`--watch`).
  - Official TechEmpower Framework Benchmarks submission.
  - Interactive BenchmarkChart and documentation deployed to GitHub Pages.
