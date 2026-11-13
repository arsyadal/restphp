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

The current runtime passed 62 Rust tests and 21 Python boundary tests on PHP 8.2.30 NTS / macOS arm64, with CI workflows covering PHP 8.2, 8.3, and 8.4 alongside Laravel 10, 11, and 12. Local authentic regressions passed against Laravel 10.50.4, 11.57.1, and 12.70.2 with Octane 2.22.0.

HTTPS and ACME were exercised both with provided PEM certificates and against official Pebble v2.8.0 for real TLS-ALPN-01 issuance, restart cache reuse, and scheduled short-lived certificate renewal (720 continuous verified HTTPS probes, 0 failed samples). Public Let's Encrypt issuance still needs an owned public DNS name and inbound TCP 443. A 1,800-second mixed-traffic persistent soak completed 188,144 operations with 0 KiB post-warmup RSS growth and 0 FD growth, and the controlled 2-CPU/2-GiB container benchmark compared RestPHP against FrankenPHP v1.13.4 on PHP 8.2.35 using observed `wrk` histograms.
