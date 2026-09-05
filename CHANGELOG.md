# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-09-05

### Added
- **Core Engine & Zero-Cost C-FFI (Milestone 1)**:
  - Raw C-ABI linkage to embedded Zend Engine (`libphp.so`) via `extern "C"`.
  - Compile-time build system in `build.rs` querying system `php-config`.
  - In-memory execution of PHP 8.2, 8.3, and 8.4 scripts.
- **Custom SAPI Subsystem (Milestone 2)**:
  - Custom `sapi_module_struct` implementation in `c/sapi.c`.
  - In-memory output buffering hook (`ub_write`) eliminating OS standard I/O overhead.
  - HTTP header and status code interceptor (`send_headers`, `http_response_code`).
  - Request body streaming (`read_post`, `php://input`) preserving binary data and multipart payloads.
  - Raw cookie parser hook (`read_cookies`) mapping into `$_COOKIE`.
- **Async Concurrency & Tokio HTTP Server (Milestone 3)**:
  - High-throughput asynchronous HTTP engine powered by Tokio and Axum.
  - Non-blocking connection pooling capable of sustaining tens of thousands of idle connections.
  - Strict CGI environment mapping into `$_SERVER` (`REQUEST_METHOD`, `REQUEST_URI`, `QUERY_STRING`, HTTP headers).
- **Persistent Worker Pool & State Reset (Milestone 4)**:
  - Dedicated OS worker threads hosting isolated Zend VM instances.
  - Request lifecycle management (`php_request_startup()` -> handle -> `php_request_shutdown()`).
  - Zend bailout recovery (`zend_first_try` / `zend_catch`) preventing unhandled `exit()` / fatal errors from crashing threads.
  - Worker recycling via `--max-requests` (default: 10,000) to prevent memory leaks in static properties.
  - 1st-class Laravel Octane adapter package (`octane/`) with persistent worker entrypoint (`octane/bin/restphp-worker.php`).
- **Bun-Style Zero-Config CLI**:
  - `restphp`: Instant startup with auto-detection of Laravel (`artisan`), `public/index.php`, or `index.php`.
  - `restphp <file>`: Direct script execution and serving (e.g. `restphp app.php -p 3000`).
  - `restphp -e 'code'`: Instant in-memory PHP code evaluation.
- **DevOps, Tooling & Benchmarking (Milestone 5)**:
  - Hot code reload via `notify` crate (`--watch`) with 500ms debounce and collision-free worker recycling.
  - Automated comparative benchmarking suite in `benchmarks/` (`run.sh`, `report.php`).
  - Official TechEmpower Framework Benchmarks (TFB) submission files in `frameworks/Rust/restphp/`.
  - 4-Tier 60-test automated E2E test suite (`tests/run_e2e_tests.py`) passing at 100%.
- **Official Documentation Website**:
  - High-performance VitePress documentation site deployed to GitHub Pages.
  - Interactive tabbed `HeroTerminal` and animated `BenchmarkChart` components.
  - 7-tier documentation hierarchy matching `bun.sh`.
