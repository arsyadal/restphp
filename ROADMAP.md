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
- [x] 4.2 State reset verification: ensure variables and superglobals do not leak across requests (verified across 60 E2E tests).
- [x] 4.3 SAPI / HTTP superglobal bridge worker script (`octane/bin/restphp-worker.php`).
- [x] 4.4 Laravel Octane driver adapter specification and package (`octane/`).

## Milestone 5: DX, Tooling & Benchmarking
- [ ] 5.1 Hot code reload via `notify` crate.
- [ ] 5.2 Automatic TLS termination via `rustls`.
- [ ] 5.3 Micro-benchmark suite comparing against FrankenPHP, Swoole, and Nginx+FPM.
