# RestPHP Task Roadmap & Backlog

## Milestone 1: Zend Engine C FFI Embedding (PoC)
- [x] 1.1 Toolchain Setup: Verify `clang`, `rustc`, `cargo`, and `php-dev` (`php-config`).
- [x] 1.2 Build System: Create `build.rs` to parse `php-config --includes`, `--ldflags`, and `--libs`.
- [x] 1.3 Minimal FFI Bindings: Generate or declare raw C bindings for `php_embed_init`, `zend_eval_string`, and `php_embed_shutdown`.
- [x] 1.4 CLI PoC: Execute PHP code string from Rust binary and capture output.

## Milestone 2: Custom SAPI Implementation
- [x] 2.1 Implement `sapi_module_struct` in Rust & C bridge.
- [x] 2.2 Implement `ub_write` callback to capture output buffer without stdout dumping.
- [x] 2.3 Implement `send_headers` callback to capture HTTP status codes and headers.
- [x] 2.4 Implement `read_post` callback to read request body.

## Milestone 3: Async HTTP Server & Concurrency Engine
- [x] 3.1 Setup Tokio + Hyper / Axum HTTP listener.
- [x] 3.2 Build thread-pool worker queue (`crossbeam-channel`).
- [x] 3.3 Map HTTP headers, query params, cookies, and body to PHP superglobals (`$_SERVER`, `$_GET`, `$_POST`).
- [x] 3.4 Wire async HTTP handler with Zend worker thread execution.

## Milestone 4: Persistent Worker Loop & Framework Integration
- [x] 4.1 Implement request lifecycle loop: `php_request_startup()` -> run script -> `php_request_shutdown()` with bailout protection.
- [x] 4.2 Verify script-mode teardown and persistent HTTP-state isolation, including input streams, sessions, environment baselines, and response metadata.
- [x] 4.3 Native `restphp_handle_request()` boot-once callback bridge with readiness before listener binding and C-contained bailout handling.
- [x] 4.4 Laravel Octane `Worker` integration with application sandboxes and lifecycle listeners (`octane/`).
- [x] 4.5 Bounded request/body admission, generic failure responses, and queued-response settlement after worker failure.
- [x] 4.6 SIGTERM/SIGINT draining, shutdown deadlines, idle callback wakeup, and whole-process request-limit recycling.
- [x] 4.7 Artisan supervisor start/status/reload/stop with locked process ownership and fresh child processes.

## Milestone 5: DX, Tooling & Benchmarking
- [x] 5.1 Process-safe hot reload via `notify` (`--watch`): debounce, drain, and replace the child process.
- [x] 5.2 rustls HTTPS with provided PEM files or automatic ACME provisioning/renewal through TLS-ALPN-01.
- [x] 5.3 Micro-benchmark suite comparing against FrankenPHP, Swoole, and Nginx+FPM (`benchmarks/`).
- [x] 5.4 Official TechEmpower Framework Benchmarks (TFB) configuration files (`frameworks/Rust/restphp/`).
- [x] 5.5 Official VitePress Documentation Website with interactive HeroTerminal & BenchmarkChart (`docs/`).

## Licensing
- [x] Complete MIT and Apache-2.0 license texts for the runtime and Composer adapter, with consistent package metadata.

## Verification boundary
- Runtime verification: PHP 8.2.30 NTS on macOS arm64; 62 Rust tests and 21 Python boundary tests passed, plus CI matrix coverage across PHP 8.2, 8.3, and 8.4.
- Authentic framework regressions: Laravel 10.50.4, 11.57.1, and 12.70.2 with Octane 2.22.0 passed locally (boot-once, scoped-service cleanup, request isolation, exception recovery, recycling, and Artisan start/status/reload/stop), with CI matrix coverage across PHP 8.2/8.3/8.4.
- HTTPS and ACME verification: verified locally with provided PEM certificates and a full TLS-ALPN-01 issuance, restart cache-reuse, and scheduled renewal cycle against official Pebble v2.8.0 (720 continuous verified HTTPS probes, 0 failures). Public Let's Encrypt issuance still requires an owned public DNS name and reachable TCP 443.
- Mixed-traffic soak and benchmark verification: 1,800-second persistent-worker soak passed (188,144 operations, 1,175 active disconnects, 2 scheduled process replacements, 0 KiB post-warmup RSS growth, 0 FD growth, 0 leaked upload files), and a controlled 2-CPU/2-GiB container comparison against FrankenPHP v1.13.4 on PHP 8.2.35 recorded exact p50/p95/p99 histograms and RSS. TechEmpower files remain configuration artifacts, not official submission results.
