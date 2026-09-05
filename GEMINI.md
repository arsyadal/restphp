# RestPHP Project Memory & Agent Context

This file is automatically loaded into the AI agent's memory on every session and turn.
It serves as the single source of truth for architecture, decisions, rules, and current progress.

---

## 1. Project Vision & Mission
- **Project Name**: RestPHP (`restphp`)
- **Tagline**: The Blazing-Fast, Persistent Application Server & Runtime for PHP powered by Rust.
- **Repository**: https://github.com/arsyadal/restphp
- **Primary Objective**: Outperform FrankenPHP (Go), RoadRunner (Go), and Swoole in throughput, p99 latency predictability, and memory footprint.
- **Author Identity**: All Git commits must be authored by:
  `Arsyad Alghital <116419335+arsyadal@users.noreply.github.com>`

---

## 2. Architectural Blueprint & Invariants
1. **Zero-Cost C-FFI**:
   - Embeds Zend Engine C API directly via raw C ABI (`extern "C"`).
   - No cgo stack-switching overhead (~60ns saved per call compared to FrankenPHP).
2. **Zero Host Garbage Collection**:
   - Rust manages server memory via compile-time RAII (zero host GC).
   - Eliminates Stop-The-World latency spikes; ensures rock-solid p99 tail latency.
3. **Custom SAPI Bridge (`c_src/sapi_bridge.c`)**:
   - Hooks `php_embed_module.ub_write` to stream PHP `echo` directly to Rust HTTP response buffers without OS stdout dumping.
   - Hooks `send_headers` to capture HTTP status codes (`http_response_code()`) and response headers.
4. **Async HTTP Engine (`src/server.rs`)**:
   - Powered by Tokio + Axum.
   - Maps HTTP URI, query string, method, headers, and body into PHP superglobals (`$_SERVER`, `$_GET`, `$_POST`, `php://input`).
5. **Persistent Worker Pool (`src/worker.rs`)**:
   - Dedicated OS threads hosting isolated Zend VM instances.
   - Requests dispatched via lock-free `crossbeam-channel` and `tokio::sync::oneshot`.

---

## 3. Progress Status & Milestone Tracking
- [x] **Milestone 1**: Zend Engine C-FFI Core Embedding (Verified in memory).
- [x] **Milestone 2**: Custom SAPI Implementation (`ub_write`, `send_headers`, superglobals).
- [x] **Milestone 3**: Async Tokio HTTP Server & REST routing (Verified with live `curl`).
- [x] **Milestone 4 (Core)**: Persistent Zend Worker Actor, State Reset & 60/60 E2E test pass.
- [ ] **Milestone 4 (Integration)**: Laravel Octane Adapter (`octane/`).
- [ ] **Milestone 5**: Micro-benchmarks vs FrankenPHP & TechEmpower submissions.

---

## 4. Coding & Maintenance Rules
- Code formatting: Always run `cargo fmt --all` before committing.
- Linter: Must pass `cargo clippy -- -D warnings`.
- Language stats: `.gitattributes` excludes Python test runners from GitHub language stats.
- MCP Documentation: Context7 is configured globally in `~/.gemini/config/mcp_config.json` and available via `context7` tool for real-time docs.
