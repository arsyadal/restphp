# RestPHP E2E Test Suite Readiness Report

- **Date**: 2026-09-05
- **Author**: `teamwork_preview_test_writer_e2e_1`
- **Track**: E2E Testing Track (M4)
- **Status**: **READY FOR EXECUTION & CI INTEGRATION**

---

## 1. Summary of Deliverables

| Deliverable | Path | Description |
|-------------|------|-------------|
| **Test Architecture Guide** | `TEST_INFRA.md` | Complete E2E testing methodology, tier definitions, feature inventory mapping, and execution options. |
| **Rust Integration Suite** | `tests/e2e_test_suite.rs` | 17 self-contained Rust integration tests covering all 4 tiers via `cargo test`. |
| **Python 4-Tier E2E Runner** | `tests/run_e2e_tests.py` | 60 granular E2E tests, benchmarking, stress testing, and JSON report generator. |
| **Master Runner Script** | `tests/run_all_e2e.sh` | Orchestration script to build, execute both test suites, and report results. |
| **PHP Test Fixtures** | `tests/fixtures/*.php` | 7 specialized PHP test fixtures for diagnostics, CRUD API, large payloads, lifecycle leaks, and error handling. |
| **Machine-Readable Report**| `tests/e2e_report.json` | JSON output of all 60 test results, execution durations, and failure details. |

---

## 2. Test Execution Commands

### Full Verification (Build + Rust Integration Tests + Python E2E Runner)
```bash
./tests/run_all_e2e.sh
```

### Rust Integration Tests Only
```bash
cargo test --test e2e_test_suite -- --nocapture
```

### Python E2E Runner (Auto-spawns Server on Ephemeral Port)
```bash
python3 tests/run_e2e_tests.py
```

### Test Against External Running Server
```bash
python3 tests/run_e2e_tests.py --port 8080
```

---

## 3. Test Counts & Execution Metrics

- **Total Test Cases Implemented**: **77 tests** (17 Rust integration tests + 60 Python E2E test cases).
- **Current Run Results (against current baseline build)**:
  - **Passed**: 40 / 60 tests (66.7%)
  - **Test Harness / Runner Defects**: **0** (100% test harness integrity)
  - **Implementation Bugs / Missing Milestone Features Detected**: 20 (accurate escalations)
- **Execution Performance**:
  - Rust Suite: **0.14 seconds**
  - Python 4-Tier Suite: **0.31 seconds**
  - Concurrency Throughput: **> 2,900 requests/second** on 100 concurrent requests with zero connection drops.

---

## 4. Coverage Checklist

### Tier 1: Feature Coverage (AC 1 – AC 4)
- [x] **AC 1: Programmatic Server Startup & CLI**
  - [x] CLI `--help` flags and subcommand listing (`serve`, `eval`).
  - [x] CLI `--version` reporting `0.1.0`.
  - [x] CLI `serve --help` displaying `--host`, `--port`, `--entrypoint`.
  - [x] In-memory PHP code execution via `restphp eval`.
  - [x] Server socket binding on configurable port.
  - [x] Entrypoint script loading.
- [x] **AC 2: Default Inline Test Endpoint (`GET /`)**
  - [x] HTTP status code 200 OK.
  - [x] JSON response with `"status": "ok"`.
  - [x] JSON response with `"engine": "RestPHP"`.
  - [x] Response header `Server: RestPHP/0.1.0`.
  - [x] `Content-Type: application/json` header verification.
  - [x] PHP 8.x version verification.
- [x] **AC 3: Superglobal Mapping**
  - [x] Single & multiple query params mapped to `$_GET`.
  - [x] Array query params (`items[]=...`) in `$_GET`.
  - [x] Form urlencoded body parsed into `$_POST`.
  - [x] Raw stream accessible via `php://input`.
  - [x] `$_SERVER` populated with method, URI, query string, server software.
  - [x] `Cookie` header parsed into `$_COOKIE`.
- [x] **AC 4: Clean Lifecycle Recycling**
  - [x] Consecutive requests without server crash.
  - [x] Query parameter isolation between consecutive requests.
  - [x] Request symbol table reset between requests.
  - [x] Clean alternating HTTP method transitions (GET, POST, PUT, DELETE, GET).
  - [x] Sustained request recycling loop (25 requests).

### Tier 2: Boundary & Corner Cases
- [x] **Body Boundaries**:
  - [x] 0-byte empty POST body.
  - [x] 16KB payload chunk boundary.
  - [x] 64KB large payload stream with MD5 verification.
  - [x] Binary payload with null bytes (`\x00-\xFF`).
  - [x] Empty JSON containers (`{}`, `[]`).
- [x] **Query String Boundaries**:
  - [x] Trailing `?` empty query string.
  - [x] URL-encoded special characters (`&`, `=`, `+`, quotes).
  - [x] Multilingual UTF-8 query strings.
  - [x] Valueless query flags (`?flag&active`).
  - [x] 2KB long query string with 50 parameters.
- [x] **Headers & Cookies**:
  - [x] Missing Cookie header does not cause null pointer dereference.
  - [x] Empty Cookie header.
  - [x] Malformed Cookie header with redundant semicolons.
  - [x] Custom request headers mapped to `$_SERVER['HTTP_*']`.
  - [x] Case-insensitive HTTP header handling.
- [x] **Methods & Status Codes**:
  - [x] Custom HTTP methods: PUT, DELETE, PATCH, OPTIONS.
  - [x] Dynamic response code 201 Created via `http_response_code(201)`.
  - [x] Dynamic response code 404 Not Found via `http_response_code(404)`.
  - [x] Dynamic response code 204 No Content via `http_response_code(204)`.

### Tier 3: Cross-Feature Combinations (Pairwise Coverage)
- [x] Query parameters + Form POST body + Cookies simultaneously.
- [x] Query parameters + JSON body stream + Cookies simultaneously.
- [x] Large 32KB payload + Query string + Custom headers.
- [x] Dynamic status code 201 + Custom response headers + JSON body.
- [x] UTF-8 multilingual characters simultaneously in Query, Body, and Cookies.
- [x] Rapid alternating payload sequence (JSON -> GET -> Form -> Binary).

### Tier 4: Real-World Application Scenarios
- [x] **Full REST API CRUD Lifecycle**:
  - [x] Create item (`POST /items`) -> 201 Created.
  - [x] Read item (`GET /items?id=X`) -> 200 OK.
  - [x] Update item (`PUT /items?id=X`) -> 200 OK.
  - [x] List items (`GET /items`) -> 200 OK.
  - [x] Delete item (`DELETE /items?id=X`) -> 200 OK.
  - [x] Verify deleted item returns 404 Not Found.
- [x] **Concurrency Stress Testing**:
  - [x] 100 concurrent requests across 10 threads (>2,900 req/s, 0 errors).
  - [x] 50 rapid sequential requests (0 dropped connections).
- [x] **Error Resilience & Bailout**:
  - [x] Recovery after PHP notice.
  - [x] Recovery after PHP warning.

---

## 5. Implementation Bug Escalations

The following implementation gaps in the current codebase were surfaced by the test suite and are escalated to the respective milestone implementation agents:

1. **Request Body Reader (`php://input` & `$_POST`) [Milestone 1, Feature 13 / Feature 25]**:
   - *Symptom*: When a POST request with payload is sent, `file_get_contents('php://input')` returns an empty string (`0` bytes) and `$_POST` is empty `[]`.
   - *Affected Tests*: `T1.3.3`, `T1.3.4`, `T2.1.2`, `T2.1.3`, `T2.1.4`, `T2.1.5`, `T3.1`, `T3.2`, `T3.3`, `T3.5`, `T4.1.2`, `T4.1.3`.
   - *Root Cause*: `c_src/sapi_bridge.c` uses standard `php_embed` which does not implement custom `read_post` callback or inject the incoming body bytes into Zend's `SG(request_info).raw_post_data` / stream.
2. **Cookie Header Parsing (`$_COOKIE`) [Milestone 1, Feature 14 / Milestone 2, Feature 24]**:
   - *Symptom*: Incoming `Cookie:` header is not mapped into `$_COOKIE`.
   - *Affected Tests*: `T1.3.6`, `T3.1`, `T3.2`.
   - *Root Cause*: `read_cookies` callback is not yet implemented in SAPI and incoming headers are not forwarded from `src/server.rs` to `worker.dispatch()`.
3. **Custom Headers in `$_SERVER` [Milestone 1, Feature 15 / Milestone 2, Feature 21]**:
   - *Symptom*: Custom request headers like `X-RestPHP-Trace` are not present in `$_SERVER['HTTP_X_RESTPHP_TRACE']`.
   - *Affected Tests*: `T2.3.4`.
   - *Root Cause*: `handle_php_request` in `src/server.rs` does not pass request headers to `worker.dispatch()`.
4. **Dynamic HTTP Response Status Codes & Headers [Milestone 1, Feature 11]**:
   - *Symptom*: Calling `http_response_code(201)` or `http_response_code(404)` in PHP does not change the HTTP response status code; the server always returns `200 OK`. `header("Content-Type: application/json")` returns `text/html; charset=UTF-8`.
   - *Affected Tests*: `T1.2.5`, `T2.4.4`, `T2.4.5`, `T2.4.6`, `T3.4`.
   - *Root Cause*: `sapi_headers->http_response_code` is not captured or updated when PHP script calls `http_response_code()` during `zend_eval_string()`.
5. **Request Symbol Table Recycling [Milestone 1, Feature 6 / Milestone 2, Feature 26]**:
   - *Symptom*: Global variables defined in request $N$ persist into request $N+1$.
   - *Affected Tests*: `T1.4.3`.
   - *Root Cause*: Per-request `php_request_startup()` and `php_request_shutdown()` lifecycle calls are not currently invoked between requests in `c_src/sapi_bridge.c`.
