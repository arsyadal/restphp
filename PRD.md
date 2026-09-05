# Product Requirements Document (PRD) — RestPHP

## 1. Project Overview
- **Product Name**: RestPHP
- **Tagline**: The Blazing-Fast, Persistent Application Server & Runtime for PHP.
- **Mission**: Outperform FrankenPHP (Go) and RoadRunner (Go) in throughput, p99 latency consistency, and memory efficiency, becoming the premier PHP application server.

---

## 2. Problem Statement
- Traditional PHP (Nginx + PHP-FPM) suffers from the shared-nothing penalty: every request incurs cold framework booting (~30–50ms overhead for Laravel/Symfony).
- Existing persistent runtimes like FrankenPHP (Go) and RoadRunner (Go) face Go-runtime bottlenecks:
  - `cgo` stack-switch overhead (~60ns per FFI call).
  - Double Garbage Collection (Go GC + PHP Zend GC) leading to p99 tail latency jitter.
  - Intermediate memory buffer duplication between Go slices and C memory.

---

## 3. The RestPHP Solution
1. **Zero-Cost C-FFI**: Direct `extern "C"` bindings to Zend Engine C core with zero wrapper overhead.
2. **Deterministic Memory (Zero Host GC)**: Rust manages server-level memory with compile-time RAII, eliminating stop-the-world pauses.
3. **Zero-Copy Network Pipeline**: Socket buffers flow straight into Zend Engine input streams.
4. **Trojan Horse Adoption**: Drop-in Laravel Octane driver (`OCTANE_SERVER=restphp`) and PSR-7/PSR-15 worker compliance.

---

## 4. Key Functional Requirements
- **FR-1**: Embed Zend Engine C core via Rust FFI with `php-config` linking.
- **FR-2**: Custom SAPI (`sapi_module_struct`) overriding `ub_write`, `read_post`, and `send_headers`.
- **FR-3**: Asynchronous HTTP/1.1 & HTTP/2 server (Tokio/Hyper/Axum).
- **FR-4**: Persistent worker execution loop (`php_request_startup()` -> run -> `php_request_shutdown()`).
- **FR-5**: Automatic worker recycling based on memory threshold or request count to isolate PHP memory leaks.

---

## 5. Non-Functional Requirements
- **NFR-1 (Latency)**: p99 latency < 2ms on micro-benchmarks.
- **NFR-2 (Memory)**: Base server idle memory < 15MB.
- **NFR-3 (Compatibility)**: Support PHP 8.2+ (ZTS and NTS).
