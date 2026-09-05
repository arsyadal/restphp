# RestPHP — AI Agent Handover & Session State

> **For the incoming AI Agent / Engineer (Claude Code, Cursor, Windsurf, new Gemini session, or CLI)**:  
> Read this file first. It contains the complete state of the project, architecture decisions, and exact instructions to resume development without losing context.

---

## 1. Project Metadata
- **Project**: RestPHP (The Blazing-Fast Persistent Application Server & Runtime for PHP in Rust)
- **Repository**: https://github.com/arsyadal/restphp
- **Author**: `Arsyad Alghital <116419335+arsyadal@users.noreply.github.com>`
- **Working Directory**: `/home/cads/restphp`
- **Current Target**: Outperform FrankenPHP (Go) and RoadRunner (Go) in throughput, p99 latency consistency, and memory efficiency.

---

## 2. Completed Milestones (Verified Working)
- [x] **Milestone 1: Zero-Cost C-FFI Zend Engine Core Embedding**
  - Links system `libphp` via `php-config` in `build.rs`.
  - In-memory execution of PHP 8.4 via Zend C API (`php_embed_init`, `zend_eval_string`, `php_embed_shutdown`).
- [x] **Milestone 2: Custom SAPI Implementation**
  - Implemented in `c_src/sapi_bridge.c` and `c_src/sapi_bridge.h`.
  - Overrides `php_embed_module.ub_write` to stream PHP `echo` directly to Rust memory buffers without stdout dumping.
  - Overrides `send_headers` to capture HTTP status codes and Content-Type.
  - Maps `$_SERVER['REQUEST_METHOD']`, `$_SERVER['REQUEST_URI']`, `$_SERVER['QUERY_STRING']`, and `$_GET`.
- [x] **Milestone 3: Async Tokio/Axum HTTP Server & Persistent Worker Pool**
  - `src/server.rs`: Axum async HTTP server listening on `--port` (default 8080).
  - `src/worker.rs`: Dedicated OS worker thread running isolated Zend VM with lock-free `crossbeam-channel` and `tokio::sync::oneshot`.
  - `src/main.rs`: CLI commands (`restphp serve --port 8080`, `restphp eval 'code'`).
- [x] **Milestone 4: Persistent Worker Loop & Laravel Octane Adapter**
  - Dedicated OS worker threads with bounded work-stealing queue (`crossbeam-channel`).
  - Strict per-request lifecycle: `php_request_startup()` -> handle -> `php_request_shutdown()`.
  - Bailout protection via `zend_first_try` / `zend_catch` preventing fatal errors from crashing threads.
  - SAPI bridge mapping HTTP headers to `$_SERVER`, cookies to `$_COOKIE`, and body to `$_POST` / `php://input`.
  - Laravel Octane official adapter package in `octane/` (`restphp/octane`) with worker script `octane/bin/restphp-worker.php`.
  - 4-Tier 60-test E2E test runner (`tests/run_e2e_tests.py`) passing at 100%.
- [x] **Milestone 5: DX, Tooling, Hot Reload & Benchmarks**
  - Bun-style zero-config CLI: `restphp` (auto-detects Laravel/entrypoint), `restphp app.php -p 3000`, `restphp -e 'code'`.
  - Hot code reload via `notify` crate (`--watch`) with 500ms debounce and collision-free worker recycling.
  - Automated benchmark suite in `benchmarks/` (`run.sh`, `report.php`).
  - Official TechEmpower Framework Benchmarks (TFB) configuration in `frameworks/Rust/restphp/`.
  - High-performance VitePress documentation website deployed to GitHub Pages (https://arsyadal.github.io/restphp/) with interactive `HeroTerminal` and animated `BenchmarkChart`.

---

## 3. Architecture & File Mapping
```
/home/cads/restphp/
├── Cargo.toml                  # Tokio, Axum, Crossbeam, Clap, Notify, Serde
├── build.rs                    # Compiles c/sapi.c with php-config --includes & links libphp
├── c/
│   └── sapi.c                 # Custom SAPI module (ub_write, send_headers, read_post, cookies)
├── src/
│   ├── lib.rs                 # Library crate root (re-exports worker, server, sapi)
│   ├── ffi/                   # Raw C-ABI extern "C" bindings to libphp.so & custom SAPI
│   ├── sapi/                  # Safe Rust wrappers (PhpEngine, callbacks, context)
│   ├── worker.rs              # Persistent worker pool, work-stealing, recycling, shutdown
│   ├── server.rs              # Tokio + Axum async HTTP server & header mapping
│   └── main.rs                # CLI entrypoint (Clap) with Bun-style commands & --watch
├── octane/                    # Official Laravel Octane adapter (restphp/octane)
│   ├── composer.json          # Laravel package auto-discovery
│   ├── bin/restphp-worker.php # Persistent Octane worker entrypoint
│   └── src/                   # ServiceProvider, commands, process inspector
├── benchmarks/                # Automated wrk benchmark runner & reports
├── frameworks/Rust/restphp/   # Official TechEmpower Framework Benchmark configuration
├── docs/                      # Official VitePress documentation site
│   ├── .vitepress/theme/      # Custom theme with HeroTerminal.vue & BenchmarkChart.vue
│   └── public/icons/          # Sharp SVG vector icons (zap, shield, cpu, etc.)
├── tests/
│   ├── run_e2e_tests.py       # Comprehensive 4-Tier 60-test E2E test runner
│   └── e2e_test_suite.rs      # Native Rust integration test suite (32 tests)
├── AGENTS.md & GEMINI.md      # Auto-loaded AI agent memory rules
├── CHANGELOG.md               # Versioned changelog (v0.1.0)
└── ROADMAP.md & PRD.md & SPEC.md
```

---

## 4. Verification Commands (Run to confirm healthy state)
```bash
# 1. Format & Lint (Must pass cleanly)
source ~/.cargo/env
cargo fmt --all
cargo clippy -- -D warnings

# 2. Run Test Suites
cargo test -- --test-threads=1
python3 tests/run_e2e_tests.py

# 3. Test Documentation Build
cd docs && NODE_ENV=production npm run docs:build

# 4. Run Server (Bun-Style Zero-Config)
cargo run --
```

---

## 5. Future Horizons (Post v0.1.0)
1. **Automatic TLS / HTTPS**: Native TLS termination via `rustls` / ACME Let's Encrypt.
2. **IO_Uring Engine**: Optional Linux `io_uring` driver for zero-syscall network socket polling.
3. **TechEmpower Round Submission**: Submit RestPHP results to the official TechEmpower Framework Benchmarks repository.
