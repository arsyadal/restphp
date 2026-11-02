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
  - Native boot-once callback bridge with fresh HTTP state and C-contained bailout recovery.
  - Real Octane worker sandboxes and lifecycle listeners, plus Artisan process supervision.
  - Bounded admission, shutdown deadlines, and process replacement on recycling.

- [x] **Milestone 5: Benchmarking Suite & TechEmpower**
  - Automated comparative benchmarks vs FrankenPHP and Swoole.
  - Process-safe source reload via `notify` (`--watch`).
  - rustls HTTPS with provided PEM certificates or automatic ACME provisioning and renewal.
  - TechEmpower Framework Benchmarks configuration files.
  - VitePress documentation and benchmark methodology.

## Verified scope

The current runtime passed 60 Rust tests on PHP 8.2.30 NTS / macOS arm64. A separate Laravel 12.70.2 / Octane 2.22.0 smoke verified stable boot/PID, request and scoped-service isolation, exception recovery, recycling, and Artisan status/reload/stop.

HTTPS was exercised with a trusted local certificate. Public ACME issuance needs an owned public DNS name and TCP 443 reaching the TLS listener; issuance against a public certificate authority was not exercised here. Benchmark tooling and TechEmpower configurations do not establish comparative performance or official acceptance. Validate other framework versions and production extension combinations for your deployment.
