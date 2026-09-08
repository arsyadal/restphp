# Benchmarks & Architectural Comparison

This page describes architecture and a reproducible benchmark workflow. RestPHP has no independently validated comparative performance figures yet; do not use the table below for production capacity planning.

---

<BenchmarkChart />

## Detailed Metrics Breakdown

| Dimension / Feature | **Nginx + PHP-FPM** | **RoadRunner (Go)** | **FrankenPHP (Go)** | **Swoole (C++)** | 🦀 **RestPHP (Rust)** |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Language Runtime** | C | Go | Go (via Caddy) | C++ | **Rust 2021 (Zero-Cost)** |
| **Execution Model** | Cold boot per req | Persistent Worker | Persistent Worker | Coroutine Event Loop | **Persistent Worker (Actor)** |
| **PHP Binding Method** | FastCGI TCP/Unix socket | IPC Pipes / Protobuf | `cgo` (stack switch cost) | PHP C Extension | **Zero-Cost C-ABI (`extern "C"`)** |
| **Host Garbage Collection** | None | Go GC (Stop-the-World) | Go GC + PHP GC (**Double GC**) | Manual C++ | **Zero Host GC (Compile-time RAII)** |
| **Tail Latency (p99)** | Benchmark required | Benchmark required | Benchmark required | Benchmark required | **Pending reproducible benchmark** |
| **Base Memory Footprint** | Benchmark required | Benchmark required | Benchmark required | Benchmark required | **Pending reproducible benchmark** |
| **Throughput (RPS)** | Benchmark required | Benchmark required | Benchmark required | Benchmark required | **Pending reproducible benchmark** |
| **Async I/O Engine** | epoll | Go netpoller | Go netpoller | Custom epoll/kqueue | **Tokio / Axum (Zero-copy)** |
| **PHP Extension Compatibility** | Varies | Varies | Varies | Varies | **Compatibility matrix pending** |
| **Host Memory Safety** | C (leaks/overflows) | Safe (Go runtime) | Safe (Go runtime) | ⚠️ Segfault / Leak risks | **100% Memory Safe (Borrow Checker)** |
| **Single Binary CLI** | ❌ Needs Nginx + FPM | ✅ Single Binary (`rr`) | ✅ Single Binary | ❌ Needs `.so` extension | **✅ Single Static Binary (`restphp`)** |
| **Laravel Octane boot-once support** | ❌ No | ✅ Yes | ✅ Yes | ✅ Yes | **⚠️ Not yet supported** |

---

<ScenarioCards />

---

## 🔬 Architectural design goals

### 1. Eliminating the CGO Tax (~60ns per call)
FrankenPHP relies on Go's `cgo` layer to communicate with the Zend Engine. Go's runtime uses non-standard stack frames (goroutines), so every transition from Go to C requires:
- Saving CPU registers and switching to an OS thread stack.
- Allocating CGO execution contexts.
- Synchronizing with the Go runtime scheduler.

**RestPHP runs on Rust**, which compiles directly to raw machine code and adheres to the standard C Application Binary Interface (C-ABI). Rust calls Zend Engine C functions at **0 nanoseconds additional overhead** — exactly as fast as C calling C.

### 2. Zero Host Garbage Collection
In Go-based runtimes, as the server handles tens of thousands of requests per second, the Go runtime allocates temporary objects for HTTP handling, triggering periodic **concurrent mark-and-sweep GC cycles**. During these phases, thread scheduling is throttled, creating latency spikes in p99 and p99.9 percentiles.

**Rust has no garbage collector**. Memory is allocated and deallocated strictly via compile-time deterministic destructors (RAII). When an HTTP request completes, its network buffers and context structures are freed instantaneously.

### 3. In-Memory Direct Buffering (Zero IPC)
Unlike RoadRunner, which serializes PSR-7 HTTP requests and responses over standard input/output pipes using the Goridge binary protocol, **RestPHP passes request structures directly in shared memory** on the worker thread.

---

## 🧪 Reproducing the Benchmarks

All benchmarks can be reproduced using standard benchmarking tools (`wrk` or `k6`) against the identical JSON endpoint:

```bash
# Benchmark RestPHP
wrk -t12 -c400 -d30s http://127.0.0.1:8080/

# Benchmark FrankenPHP
wrk -t12 -c400 -d30s http://127.0.0.1:8081/
```
