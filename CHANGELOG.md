# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Native Windows x64 MSVC build configuration using official NTS PHP development packs and `php8.lib`.
- PowerShell setup for matching PHP runtime/SDK archives, with release-manifest SHA-256 verification.
- Windows PHP 8.2/8.3/8.4 CI, persistent-worker and HTTP regressions, graceful Ctrl-Break/watch lifecycle checks, and packaged Windows release artifacts with matching PHP DLLs.
- Windows installation guide with explicit runtime requirements and the Unix-only Artisan supervisor boundary.

### Fixed
- Avoid Unix-only SIGPIPE setup on Windows and reject incompatible ZTS headers.
- Derive integration-test executable paths from Cargo so native `.exe` binaries are exercised.
- Normalize Rust's Windows verbatim disk/UNC filenames at the PHP ABI boundary so Zend can open canonicalized entrypoints and persistent scripts.

## [0.2.0] - 2026-11-13

### Added
- Native `restphp_handle_request()` callback mode with boot-once readiness, fresh HTTP state between requests, C-contained bailout handling, and graceful process recycling.
- Laravel Octane integration using the real worker sandbox and lifecycle listeners, with Artisan start/status/reload/stop supervision and locked process ownership.
- Opt-in rustls HTTPS with provided PEM keys or automatic ACME certificate provisioning and renewal through TLS-ALPN-01, plus transport-derived PHP HTTPS metadata.
- Process-safe `--watch` reload: debounce source/config changes, drain the old child, and start a new process without reinitializing Zend in place.
- Complete MIT and Apache-2.0 license texts for the runtime and Composer adapter; align documentation package license metadata.
- Compatibility CI matrix across PHP 8.2, 8.3, and 8.4 plus authentic Laravel 10, 11, and 12 Octane lifecycle regressions (`tests/run_laravel_integration.py`).
- Fail-closed `wrk` histogram parser (`benchmarks/wrk.py`), Lua percentile reporter (`benchmarks/percentiles.lua`), and controlled RestPHP vs FrankenPHP container comparison (`benchmarks/Dockerfile`, `benchmarks/run.py`).
- Scoped custom ACME directory and CA trust flags (`--acme-directory`, `--acme-ca-cert`) with end-to-end Pebble TLS-ALPN-01 issuance, cache-reuse, and renewal verification (`tests/run_acme_e2e.py`).
- Mixed-traffic persistent-worker soak harness and resource-budget verifier (`tests/run_soak.py`, `tests/test_soak_metrics.py`).


### Fixed
- Start the graceful shutdown deadline only when shutdown begins, covering HTTP draining and PHP worker teardown without limiting healthy server uptime.
- Register shutdown signal handlers before Zend initialization so SIGTERM and SIGINT still trigger graceful shutdown between PHP requests.
- Load PHP configuration and scanned extension settings instead of disabling them, allowing standard Laravel applications to boot with their configured extensions.
- Retain bounded per-request SAPI diagnostics and log failed output with distinct PHP and HTTP status fields, while preserving generic HTTP 500 responses.
- Preserve PHP shutdown-handler failure status so fatal errors during request teardown also produce a generic HTTP 500.
- Dependabot automation no longer requests an Actions-generated approval and skips auto-merge cleanly when the repository setting is disabled.
- Reset PHP `max_execution_time` per persistent callback and pause the timer while waiting for the next request so cumulative worker uptime does not trigger a 30-second fatal timeout.
- Never substitute mean `wrk` latency when `99%` histogram output is absent; reject or mark missing percentiles explicitly.

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
