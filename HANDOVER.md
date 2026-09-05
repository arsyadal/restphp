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
  - Tested live with `curl -i http://localhost:8089/` -> returned HTTP 200 OK and JSON response.
- [x] **DevOps & Quality Automation**
  - GitHub Actions CI in `.github/workflows/ci.yml`.
  - Dependabot & auto-merge bot in `.github/workflows/dependabot-auto-merge.yml`.
  - Automatic binary release builder in `.github/workflows/release.yml`.
  - Stale issue/PR bot in `.github/workflows/stale.yml`.
  - Context7 MCP installed and configured in `~/.gemini/config/mcp_config.json`.
  - E2E Test Suite in `tests/e2e_test_suite.rs` with 7 PHP fixtures.

---

## 3. Architecture & File Mapping
```
/home/cads/restphp/
├── Cargo.toml                  # Tokio, Axum, Crossbeam, Clap, Serde
├── build.rs                    # Compiles c_src/sapi_bridge.c with php-config --includes
├── c_src/
│   ├── sapi_bridge.h          # C SAPI bridge header (RestPhpResponse)
│   └── sapi_bridge.c          # Overrides ub_write, send_headers, superglobals
├── src/
│   ├── lib.rs                 # Root library crate
│   ├── ffi.rs                 # Raw extern "C" bindings to sapi_bridge
│   ├── sapi.rs                # Safe Rust wrappers (PhpEngine, PhpResponse)
│   ├── worker.rs              # Persistent worker thread pool & dispatcher
│   ├── server.rs              # Tokio + Axum async HTTP server
│   └── main.rs                # CLI entrypoint (clap)
├── tests/
│   ├── e2e_test_suite.rs      # Native Rust E2E test suite (cargo test)
│   └── fixtures/              # PHP test scripts (crud, lifecycle, error, etc.)
├── GEMINI.md & AGENTS.md      # Auto-loaded agent memory files
└── PRD.md & SPEC.md           # Product requirements & low-level tech specs
```

---

## 4. Verification Commands (Run to confirm healthy state)
```bash
# 1. Format & Lint (Must pass cleanly)
cargo fmt --all -- --check
cargo clippy -- -D warnings

# 2. Run Test Suite
cargo test

# 3. Test HTTP Server
cargo run -- serve --port 8080
# In another terminal:
curl -i http://127.0.0.1:8080/
```

---

## 5. Next Immediate Tasks (To Be Done Next)
1. **Milestone 4: Persistent Zend Worker Actor & State Reset**
   - Implement `php_request_startup()` -> run -> `php_request_shutdown()` per-request lifecycle to guarantee zero cross-request memory leaks.
   - Build the Laravel Octane persistent worker adapter (`octane/restphp-worker.php`).
   - Implement graceful worker recycling (auto-recycle worker after 10,000 requests or memory threshold).
2. **Milestone 5: Production Benchmarks vs FrankenPHP**
   - Setup `benchmarks/` with `oha` / `wrk` benchmarking scripts.
   - Benchmark RestPHP vs FrankenPHP on:
     - Plaintext / JSON Serialization throughput (RPS).
     - p99 tail latency.
     - Idle & peak memory consumption.
   - Update README.md with benchmark proof.
