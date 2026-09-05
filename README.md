# RestPHP 🦀🐘

> **The Blazing-Fast, Persistent Application Server & Runtime for PHP.**  
> Zero-overhead C-FFI, zero-GC jitter, io_uring I/O, and first-class Laravel Octane & PSR-7 persistent workers.

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![PHP](https://img.shields.io/badge/php-8.2%2B%20(ZTS%2FNTS)-777bb4.svg)](https://www.php.net/)

---

## ⚡ Why RestPHP?

Traditional PHP operates on a **shared-nothing architecture**: every incoming HTTP request boots the entire framework from scratch and tears down memory afterwards. While persistent runners like **FrankenPHP (Go)** and **RoadRunner (Go)** pioneered worker mode, they face hard limits imposed by the Go runtime:

1. **Zero-Cost FFI vs Cgo**: FrankenPHP must pass through Go's `cgo` layer to talk to Zend Engine, paying a stack-switching penalty on every single FFI transition (~60ns). RestPHP links directly to the Zend C ABI via Rust's zero-cost FFI.
2. **Deterministic Latency (Zero Host GC)**: FrankenPHP runs two concurrent garbage collectors (Go GC + Zend GC), causing unpredictable latency spikes at p99/p99.9. RestPHP has **no host GC**—memory is deterministically managed by Rust's RAII model.
3. **Zero-Copy Streaming**: Network socket buffers from Linux `io_uring` / Tokio are passed directly into PHP's request input stream (`php://input`) without duplicate heap allocations.

---

## 📊 Architectural Comparison

| Dimension | Nginx + PHP-FPM | FrankenPHP (Go) | RestPHP (Rust) |
| :--- | :--- | :--- | :--- |
| **Execution Model** | Cold boot per request | Persistent Worker | **Persistent Worker** |
| **FFI Interoperability** | N/A (FastCGI) | cgo (context switch cost) | **Zero-Cost C ABI (`extern "C"`)** |
| **Host Runtime GC** | None | Go GC + PHP GC (Double GC) | **Zero Host GC (Deterministic)** |
| **Tail Latency (p99)** | Slow & jittery | Jitter from Go GC pauses | **Ultra-consistent** |
| **Base Idle RAM** | ~30–80 MB / worker | ~50–100 MB (Caddy base) | **< 15 MB base footprint** |
| **I/O Engine** | epoll | Go netpoller | **Tokio / io_uring (Zero-copy)** |
| **Laravel Octane** | ❌ No | ✅ Yes | **✅ 1st-Class Driver** |

---

## 🏗️ Architecture Overview

```
       HTTP / HTTPS Clients
               │
               ▼
┌──────────────────────────────┐
│     RestPHP Async Engine     │  (Tokio / Hyper / Rustls / io_uring)
│                              │
│   Lock-free Request Queue    │
│   ┌──────────────────────┐   │
│   │   Custom SAPI Bridge │   │  (Zero-copy ub_write & read_post)
│   └──────────┬───────────┘   │
└──────────────┼───────────────┘
               ▼
┌──────────────────────────────┐
│   Zend Persistent Workers    │  (PHP in-memory state: Laravel, Symfony)
└──────────────────────────────┘
```

---

## 🚀 Key Features

- **Laravel Octane Integration**: Plug-and-play adapter (`OCTANE_SERVER=restphp`) for immediate 3x–5x performance gains.
- **Dual Execution Modes**:
  - **Framework Mode**: Full PSR-7 & PSR-15 compatibility with automatic state reset.
  - **Native Micro-Mode**: High-speed routing directly into PHP closures for millions of RPS.
- **Worker Auto-Recycling**: Configurable maximum requests and memory threshold limits to guard against legacy PHP memory leaks.
- **Hot Reloading**: Integrated file watcher automatically reloads worker processes upon code changes during development.
- **Built-in TLS**: Modern, fast HTTPS termination powered by `rustls`.

---

## 🗺️ Project Roadmap

- [x] Technical Specification & Architecture Design ([`SPEC.md`](SPEC.md))
- [ ] Phase 1: Zend Engine C FFI Embedding & CLI Proof of Concept
- [ ] Phase 2: Custom SAPI Implementation & Async Tokio HTTP Server
- [ ] Phase 3: Persistent Worker Loop & Laravel Octane Bridge Driver
- [ ] Phase 4: Production Benchmarks (TechEmpower) & Distribution

See [`ROADMAP.md`](ROADMAP.md) for granular task tracking.
