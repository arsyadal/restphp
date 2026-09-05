# RestPHP: Technical Specification

## 1. System Goals
- **Mission**: Zero-overhead Zend Engine persistent runtime in Rust outperforming FrankenPHP (Go) and RoadRunner (Go).
- **Core KPIs**:
  - p99 Latency: < 2ms for microservice endpoints, jitter variance < 15%.
  - Throughput: > 150,000 req/sec on basic JSON/plaintext benchmarks.
  - Idle Memory: < 15MB base footprint for the server process.

## 2. Low-Level Architecture & Components

### 2.1 FFI Bridge (`restphp-ffi`)
- Wraps Zend Engine C API (`php.h`, `sapi.h`, `zend.h`).
- Links against system `libphp` via `php-config --ldflags --libs`.
- Supports both ZTS (Zend Thread Safety) and NTS multi-process architectures.
- Zero-cost FFI: Raw `extern "C"` bindings without translation wrappers.

### 2.2 SAPI Subsystem (`restphp-sapi`)
- Custom `sapi_module_struct` implementation:
  - `name`: "restphp"
  - `pretty_name`: "RestPHP Server SAPI"
  - `startup`: Initializes Zend VM and registers server constants.
  - `shutdown`: Cleans up Zend VM.
  - `activate`: Per-request initialization (`php_request_startup`).
  - `deactivate`: Per-request teardown (`php_request_shutdown`) triggering Zend GC without terminating the OS thread/process.
  - `ub_write`: Direct streaming of output bytes to response writer channel.
  - `read_post`: Non-blocking stream reader from Rust input buffers to `php://input`.
  - `sapi_header_op`: Captures HTTP status code and response headers.

### 2.3 Concurrency & Networking (`restphp-core`)
- High-concurrency async runtime: Tokio / Hyper.
- Request queue: Crossbeam lock-free channel or MPSC worker dispatcher.
- Worker Pool: Dedicated OS threads each hosting a Zend VM instance.
- Graceful Recycling: Monitors request count and resident set size (RSS); recycles worker cleanly if thresholds are exceeded.

## 3. Boundary & Non-Goals for V1
- V1 does not rewrite the PHP language or Zend VM; it hosts Zend Engine natively.
- V1 focuses on Linux (x86_64 / aarch64) with standard POSIX sockets.
