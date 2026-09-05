# RestPHP E2E Test Infrastructure & Architecture Guide

## Overview

RestPHP is a persistent, ultra-high-performance application server and runtime for PHP embedding the Zend Engine via zero-cost C FFI. The End-to-End (E2E) testing framework provides opaque-box verification of the full server lifecycle, asynchronous HTTP request dispatch (Tokio/Axum), persistent Zend worker actor execution, superglobals mapping, request recycling, and boundary stress resilience.

The test infrastructure is dual-layered:
1. **Rust Integration Test Suite** (`tests/e2e_test_suite.rs`): Executes via `cargo test --test e2e_test_suite` for standard Rust development, CI builds, and zero-dependency integration.
2. **Comprehensive 4-Tier Python E2E Runner** (`tests/run_e2e_tests.py`): Standalone, highly configurable test runner capable of executing 60+ discrete test cases, measuring p95/p99 latency, stress testing concurrency (multi-threaded workers), and outputting structured JSON test manifests (`tests/e2e_report.json`).
3. **Automated Master Runner** (`tests/run_all_e2e.sh`): Single entrypoint script executing both suites sequentially.

---

## Test Methodology (4 Tiers)

The test suite is structured into 4 hierarchical testing tiers:

| Tier | Focus | Description | Minimum Count |
|------|-------|-------------|---------------|
| **Tier 1** | **Feature Coverage** | Primary behavior & happy paths for AC 1–4 from `ORIGINAL_REQUEST.md` and `PROJECT.md` | >= 5 per feature |
| **Tier 2** | **Boundary & Corner Cases** | Edge values, 0-byte/64KB payloads, special characters, unicode, custom HTTP methods | >= 5 per category |
| **Tier 3** | **Cross-Feature Combinations** | Pairwise interaction of query params, request bodies, cookies, and response headers | Pairwise matrix |
| **Tier 4** | **Real-World Scenarios** | REST API CRUD simulation, 100-request high concurrency stress, error recovery | Production loads |

---

## Feature Inventory Mapping

| Project Feature | Milestone | Description | Covered By Tests |
|-----------------|-----------|-------------|------------------|
| F1 - System Libphp Linker | M1 | Link system `libphp.so` | T1.1.1–T1.1.4, Rust harness |
| F7 - In-Memory String Evaluation | M1 | Safe string evaluation | T1.1.4 (`restphp eval`) |
| F8 - PHP Script File Execution | M1 | Execute `.php` script files | T1.1.6, T4.1.1–T4.1.6 |
| F10 - Zero-Stdout Output Capturing | M1 | Stream output chunks to buffer | T1.2.1–T1.2.3, T1.3.1 |
| F11 - Response Headers & Status | M1 | `send_headers`, status codes | T2.4.4–T2.4.6, T3.4 |
| F13 - Request Body Reader | M1 | `read_post` streaming for `php://input` | T1.3.4, T2.1.1–T2.1.5, T3.3 |
| F14 - Cookie String Reader | M1 | `read_cookies` for `$_COOKIE` | T1.3.6, T2.3.1–T2.3.3, T3.1 |
| F15 - Server Variables Registration | M1 | Populate `$_SERVER` | T1.3.5, T2.3.4 |
| F18 - Bailout & Exit Protection | M1 | Safe bailout & notice recovery | T4.3.1–T4.3.2 |
| F20 - Dedicated Worker Thread | M2 | OS thread hosting Zend VM | T1.4.1, T4.2.1 |
| F21 - `$_SERVER` Full CGI Mapping | M2 | Method, URI, query in `$_SERVER` | T1.3.5, T1.4.4 |
| F22 - `$_GET` Query Parameter Parsing | M2 | Query string parsing | T1.3.1–T1.3.2, T2.2.1–T2.2.5 |
| F23 - `$_POST` Form Data Parsing | M2 | Form urlencoded data into `$_POST`| T1.3.3, T3.1 |
| F24 - `$_COOKIE` Header Parsing | M2 | Cookie header parsing | T1.3.6, T3.1, T3.2 |
| F25 - `php://input` Raw Stream | M2 | Raw stream for JSON/XML | T1.3.4, T2.1.2–T2.1.5 |
| F26 - Clean Lifecycle Reset & GC | M2 | Consecutive request isolation | T1.4.1–T1.4.3 |
| F29 - Async HTTP Server | M3 | Tokio/Axum HTTP listener | T1.1.5, T4.2.1 |
| F30 - CLI `serve` Subcommand | M3 | Clap CLI `serve` | T1.1.1, T1.1.3 |
| F31 - `--port` & `--host` Options | M3 | Bind address and port | T1.1.5 |
| F32 - `--entrypoint` Option | M3 | Script entrypoint path | T1.1.6 |
| F33 - Default Inline Test Endpoint | M3 | GET `/` returns JSON | T1.2.1–T1.2.6 |
| F34 - E2E Test Suite | M4 | 4-Tier verification suite | Complete Suite |

---

## Test Directory Layout

```
tests/
├── e2e_test_suite.rs          # Rust integration test suite (cargo test)
├── run_e2e_tests.py           # 4-Tier Python E2E runner (60 test cases)
├── run_all_e2e.sh             # Master execution script
├── e2e_report.json            # Machine-readable test execution report
└── fixtures/                  # Specialized PHP scripts for test scenarios
    ├── info.php               # Superglobals and input diagnostics
    ├── status_and_headers.php # Dynamic HTTP response code & header generator
    ├── crud.php               # REST API CRUD engine (GET, POST, PUT, DELETE)
    ├── large_body.php         # Large body & MD5 checksum verification
    ├── lifecycle.php          # State leak & symbol table pollution detector
    ├── error.php              # PHP notices, warnings, and bailout trigger
    └── utf8_special.php       # UTF-8 and special character encoding test
```

---

## How to Run the Tests

### 1. Master Runner (Build + Rust Suite + Python Suite)
```bash
./tests/run_all_e2e.sh
```

### 2. Rust Integration Tests Only
```bash
cargo test --test e2e_test_suite -- --nocapture
```

### 3. Standalone Python Runner (Auto-spawns Server on Free Port)
```bash
python3 tests/run_e2e_tests.py
```

### 4. Run Against an External Running Server
```bash
# In terminal 1:
cargo run -- serve --port 8080

# In terminal 2:
python3 tests/run_e2e_tests.py --port 8080
```

### 5. Filter by Specific Tier
```bash
python3 tests/run_e2e_tests.py --tier 1
python3 tests/run_e2e_tests.py --tier 4
```

---

## Test Inventory & Tier Breakdown

### Tier 1: Feature Coverage (23 Tests)
- **AC 1: Server Startup & CLI Options**
  - `T1.1.1`: CLI `--help` displays `serve` and `eval` subcommands.
  - `T1.1.2`: CLI `--version` returns `0.1.0`.
  - `T1.1.3`: CLI `serve --help` displays `--port`, `--host`, `--entrypoint`.
  - `T1.1.4`: CLI `eval` executes PHP code in-memory.
  - `T1.1.5`: Server binds and listens on configurable port.
  - `T1.1.6`: Server loads custom script entrypoint.
- **AC 2: Default Inline Test Endpoint (`GET /`)**
  - `T1.2.1`: HTTP status code is 200 OK.
  - `T1.2.2`: Response body contains `"status": "ok"`.
  - `T1.2.3`: Response body contains `"engine": "RestPHP"`.
  - `T1.2.4`: Header includes `Server: RestPHP/0.1.0`.
  - `T1.2.5`: Header includes `Content-Type: application/json`.
  - `T1.2.6`: Response includes valid PHP 8.x version info.
- **AC 3: Superglobal Mapping**
  - `T1.3.1`: Single & multiple query params in `$_GET`.
  - `T1.3.2`: Array query params (`items[]=apple&items[]=banana`) in `$_GET`.
  - `T1.3.3`: Form URL-encoded data in `$_POST`.
  - `T1.3.4`: Raw JSON body in `php://input`.
  - `T1.3.5`: `$_SERVER` contains `REQUEST_METHOD`, `REQUEST_URI`, `QUERY_STRING`.
  - `T1.3.6`: `Cookie` header parsed into `$_COOKIE`.
- **AC 4: Clean Lifecycle Recycling**
  - `T1.4.1`: 10 consecutive requests execute cleanly without crashing.
  - `T1.4.2`: Consecutive requests maintain query string isolation.
  - `T1.4.3`: PHP request teardown resets global symbol table.
  - `T1.4.4`: Alternating sequential HTTP methods (GET, POST, PUT, DELETE, GET).
  - `T1.4.5`: Sustained 25-request recycling loop with 100% success.

### Tier 2: Boundary & Corner Cases (21 Tests)
- **Body Boundaries**:
  - `T2.1.1`: 0-byte empty POST body.
  - `T2.1.2`: 16KB payload chunk boundary.
  - `T2.1.3`: 64KB large payload streaming with exact MD5 match.
  - `T2.1.4`: Binary payload containing null bytes (`\x00-\xFF`).
  - `T2.1.5`: Empty JSON structures (`{}`, `[]`).
- **Query String Boundaries**:
  - `T2.2.1`: Trailing `?` with empty query string.
  - `T2.2.2`: URL-encoded characters (`&`, `=`, `+`, quotes).
  - `T2.2.3`: UTF-8 multilingual characters in query string.
  - `T2.2.4`: Valueless query flags (`?flag&active`).
  - `T2.2.5`: 2KB query string with 50 parameters.
- **Headers & Cookies Boundaries**:
  - `T2.3.1`: Missing Cookie header (avoids null pointer dereference).
  - `T2.3.2`: Empty Cookie header value.
  - `T2.3.3`: Malformed Cookie header with redundant semicolons.
  - `T2.3.4`: Custom HTTP headers mapped to `$_SERVER['HTTP_*']`.
  - `T2.3.5`: Mixed-case HTTP headers.
- **Methods & Status Codes**:
  - `T2.4.1`: HTTP PUT method.
  - `T2.4.2`: HTTP DELETE method.
  - `T2.4.3`: HTTP PATCH method.
  - `T2.4.4`: Dynamic status code 201 Created via `http_response_code(201)`.
  - `T2.4.5`: Dynamic status code 404 Not Found via `http_response_code(404)`.
  - `T2.4.6`: Dynamic status code 204 No Content via `http_response_code(204)`.

### Tier 3: Cross-Feature Combinations (6 Tests)
- `T3.1`: Query params + Form POST body + Cookies simultaneously.
- `T3.2`: Query params + JSON body stream + Cookies simultaneously.
- `T3.3`: Large 32KB payload + Query params + Custom headers.
- `T3.4`: Dynamic HTTP 201 status + Custom response header combined.
- `T3.5`: Multilingual UTF-8 data preserved across Query, Body, and Cookies.
- `T3.6`: Rapid alternating payload sequence (JSON -> GET -> Form -> Binary).

### Tier 4: Real-World Scenarios (10 Tests)
- **REST API CRUD Lifecycle**:
  - `T4.1.1`: Create item (`POST /items`) -> returns 201 Created with generated ID.
  - `T4.1.2`: Read item (`GET /items?id=X`) -> returns 200 OK with stored item.
  - `T4.1.3`: Update item (`PUT /items?id=X`) -> returns 200 OK with updated entity.
  - `T4.1.4`: List items (`GET /items`) -> returns 200 OK with active items.
  - `T4.1.5`: Delete item (`DELETE /items?id=X`) -> returns 200 OK.
  - `T4.1.6`: Verify deleted item returns 404 Not Found.
- **High-Throughput Concurrency Stress**:
  - `T4.2.1`: 100 concurrent requests across 10 threads (>2,500 req/s, 0 errors).
  - `T4.2.2`: 50 rapid sequential requests (0 dropped connections).
- **Error Resilience & Recovery**:
  - `T4.3.1`: Server recovers immediately after PHP notice and serves subsequent requests.
  - `T4.3.2`: Server recovers immediately after PHP warning and serves subsequent requests.

---

## Authoritative Expected Output Sources

All expected outputs in this test suite are derived authoritatively from:
1. `ORIGINAL_REQUEST.md`: Core requirements R1-R4 and Acceptance Criteria AC 1-4.
2. `PROJECT.md`: Architectural specifications, feature inventory definitions, and interface contracts.
3. RFC 7230 / RFC 7231 (HTTP/1.1 Specification): Standard HTTP status codes, headers, and framing.
4. PHP 8.4 Language Specification: Superglobals (`$_GET`, `$_POST`, `$_COOKIE`, `$_SERVER`, `php://input`), `http_response_code()`, `header()`, and request shutdown lifecycle.
