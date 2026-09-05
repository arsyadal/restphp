# RestPHP Task Roadmap & Backlog

## Milestone 1: Zend Engine C FFI Embedding (PoC)
- [ ] 1.1 Toolchain Setup: Verify `clang`, `rustc`, `cargo`, and `php-dev` (`php-config`).
- [ ] 1.2 Build System: Create `build.rs` to parse `php-config --includes`, `--ldflags`, and `--libs`.
- [ ] 1.3 Minimal FFI Bindings: Generate or declare raw C bindings for `php_embed_init`, `zend_eval_string`, and `php_embed_shutdown`.
- [ ] 1.4 CLI PoC: Execute PHP code string from Rust binary and capture output.

## Milestone 2: Custom SAPI Implementation
- [ ] 2.1 Implement `sapi_module_struct` in Rust.
- [ ] 2.2 Implement `ub_write` callback to capture output buffer without stdout dumping.
- [ ] 2.3 Implement `send_headers` callback to capture HTTP status codes and headers.
- [ ] 2.4 Implement `read_post` callback to read request body.

## Milestone 3: Async HTTP Server & Concurrency Engine
- [ ] 3.1 Setup Tokio + Hyper / Axum HTTP listener.
- [ ] 3.2 Build thread-pool worker queue (`crossbeam-channel`).
- [ ] 3.3 Map HTTP headers, query params, cookies, and body to PHP superglobals (`$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`).
- [ ] 3.4 Wire async HTTP handler with Zend worker thread execution.

## Milestone 4: Persistent Worker Loop & Framework Integration
- [ ] 4.1 Implement request lifecycle loop: `php_request_startup()` -> run script -> `php_request_shutdown()`.
- [ ] 4.2 State reset verification: ensure variables and superglobals do not leak across requests.
- [ ] 4.3 PSR-7 / PSR-15 bridge worker script.
- [ ] 4.4 Laravel Octane driver adapter specification and package.

## Milestone 5: DX, Tooling & Benchmarking
- [ ] 5.1 Hot code reload via `notify` crate.
- [ ] 5.2 Automatic TLS termination via `rustls`.
- [ ] 5.3 Micro-benchmark suite comparing against FrankenPHP, Swoole, and Nginx+FPM.
