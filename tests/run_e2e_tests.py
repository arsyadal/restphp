#!/usr/bin/env python3
"""
RestPHP Comprehensive E2E Test Suite (Tiers 1 - 4)
Executable against `cargo run -- serve --port <port>` or standalone.

Requirements from ORIGINAL_REQUEST.md and PROJECT.md:
- Tier 1: Feature Coverage (AC 1-4, >=5 tests per feature)
- Tier 2: Boundary & Corner Cases (>=5 tests per category)
- Tier 3: Cross-Feature Combinations (Pairwise coverage)
- Tier 4: Real-World Scenarios (REST API CRUD & Concurrency Stress)
"""

import argparse
import concurrent.futures
import hashlib
import json
import os
import socket
import subprocess
import sys
import time
import urllib.parse
import urllib.request
import http.client

# ANSI colors for terminal output
GREEN = "\033[92m"
RED = "\033[91m"
YELLOW = "\033[93m"
CYAN = "\033[96m"
BOLD = "\033[1m"
RESET = "\033[0m"

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))

def get_free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("", 0))
        return s.getsockname()[1]

class ServerProcess:
    def __init__(self, entrypoint="public/index.php", port=None, host="127.0.0.1"):
        self.entrypoint = entrypoint
        self.host = host
        self.port = port or get_free_port()
        self.proc = None

    def start(self):
        bin_path = os.path.join(PROJECT_ROOT, "target", "debug", "restphp")
        if not os.path.exists(bin_path):
            subprocess.run(["cargo", "build"], cwd=PROJECT_ROOT, check=True)

        cmd = [
            bin_path,
            "serve",
            "--host", self.host,
            "--port", str(self.port),
            "--entrypoint", self.entrypoint
        ]
        self.proc = subprocess.Popen(
            cmd,
            cwd=PROJECT_ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE
        )
        # Wait for TCP connect
        start_time = time.time()
        while time.time() - start_time < 5.0:
            try:
                with socket.create_connection((self.host, self.port), timeout=0.2):
                    return self
            except (socket.error, ConnectionRefusedError):
                time.sleep(0.05)
        raise RuntimeError(f"Server on port {self.port} failed to start within 5s")

    def stop(self):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=2.0)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait()
            self.proc = None

    def __enter__(self):
        return self.start()

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.stop()

def raw_http_request(host, port, method, path, headers=None, body=None, timeout=5.0):
    """Low-level HTTP request helper returning (status, headers_dict, body_bytes)."""
    headers = headers or {}
    conn = http.client.HTTPConnection(host, port, timeout=timeout)
    body_bytes = body if isinstance(body, bytes) else (body.encode("utf-8") if body else None)
    
    conn.request(method, path, body=body_bytes, headers=headers)
    response = conn.getresponse()
    resp_body = response.read()
    resp_headers = {k.lower(): v for k, v in response.getheaders()}
    status = response.status
    conn.close()
    return status, resp_headers, resp_body

def safe_get(container, key, default=None):
    if isinstance(container, dict):
        return container.get(key, default)
    return default

class TestResult:

    def __init__(self, test_id, tier, name, passed, message="", duration=0.0, is_bug=False):
        self.test_id = test_id
        self.tier = tier
        self.name = name
        self.passed = passed
        self.message = message
        self.duration = duration
        self.is_bug = is_bug  # Flag if failure is an implementation bug

    def to_dict(self):
        return {
            "test_id": self.test_id,
            "tier": self.tier,
            "name": self.name,
            "passed": self.passed,
            "is_bug": self.is_bug,
            "message": self.message,
            "duration_ms": round(self.duration * 1000, 2)
        }

class E2ETestRunner:
    def __init__(self, host="127.0.0.1", port=None):
        self.host = host
        self.port = port
        self.results = []
        self.managed_server = None

    def record(self, test_id, tier, name, passed, message="", duration=0.0, is_bug=False):
        res = TestResult(test_id, tier, name, passed, message, duration, is_bug)
        self.results.append(res)
        status_str = f"{GREEN}[PASS]{RESET}" if passed else (f"{YELLOW}[BUG/ESCALATE]{RESET}" if is_bug else f"{RED}[FAIL]{RESET}")
        print(f"  {status_str} {test_id} - {name} ({res.duration*1000:.1f}ms)")
        if not passed and message:
            print(f"         {RED}Details: {message}{RESET}")

    def run_all(self, selected_tier=None):
        print(f"\n{BOLD}{CYAN}======================================================================{RESET}")
        print(f"{BOLD}{CYAN}       RestPHP Comprehensive 4-Tier E2E Test Suite Runner             {RESET}")
        print(f"{BOLD}{CYAN}======================================================================{RESET}\n")

        tiers_to_run = [1, 2, 3, 4] if selected_tier is None else [selected_tier]

        for t in tiers_to_run:
            start_t = time.time()
            print(f"{BOLD}>>> Executing Tier {t} Tests...{RESET}")
            if t == 1:
                self.run_tier_1()
            elif t == 2:
                self.run_tier_2()
            elif t == 3:
                self.run_tier_3()
            elif t == 4:
                self.run_tier_4()
            print(f"{CYAN}--- Tier {t} finished in {time.time() - start_t:.2f}s ---\n{RESET}")

        self.print_summary()

    # =========================================================================
    # TIER 1: FEATURE COVERAGE (>=5 tests per feature)
    # =========================================================================
    def run_tier_1(self):
        bin_path = os.path.join(PROJECT_ROOT, "target", "debug", "restphp")

        # --- Feature 1: AC 1 - Programmatic server startup & CLI options ---
        # Test 1.1.1: CLI Help flag
        t0 = time.time()
        try:
            res = subprocess.run([bin_path, "--help"], capture_output=True, text=True, timeout=3)
            passed = res.returncode == 0 and "serve" in res.stdout and "eval" in res.stdout
            self.record("T1.1.1", 1, "CLI --help displays serve and eval commands", passed, res.stderr, time.time() - t0)
        except Exception as e:
            self.record("T1.1.1", 1, "CLI --help displays serve and eval commands", False, str(e), time.time() - t0)

        # Test 1.1.2: CLI Version flag
        t0 = time.time()
        try:
            res = subprocess.run([bin_path, "--version"], capture_output=True, text=True, timeout=3)
            passed = res.returncode == 0 and "0.1.0" in res.stdout
            self.record("T1.1.2", 1, "CLI --version returns version 0.1.0", passed, res.stderr, time.time() - t0)
        except Exception as e:
            self.record("T1.1.2", 1, "CLI --version returns version 0.1.0", False, str(e), time.time() - t0)

        # Test 1.1.3: CLI Serve Subcommand Help
        t0 = time.time()
        try:
            res = subprocess.run([bin_path, "serve", "--help"], capture_output=True, text=True, timeout=3)
            passed = res.returncode == 0 and "--port" in res.stdout and "--entrypoint" in res.stdout
            self.record("T1.1.3", 1, "CLI serve --help lists --port, --host, --entrypoint", passed, res.stderr, time.time() - t0)
        except Exception as e:
            self.record("T1.1.3", 1, "CLI serve --help lists --port, --host, --entrypoint", False, str(e), time.time() - t0)

        # Test 1.1.4: CLI Eval Subcommand
        t0 = time.time()
        try:
            res = subprocess.run([bin_path, "eval", "echo 'RestPHP_CLI_Eval_Success';"], capture_output=True, text=True, timeout=3)
            passed = res.returncode == 0 and "RestPHP_CLI_Eval_Success" in res.stdout
            self.record("T1.1.4", 1, "CLI eval executes inline PHP code string directly", passed, res.stderr or res.stdout, time.time() - t0)
        except Exception as e:
            self.record("T1.1.4", 1, "CLI eval executes inline PHP code string directly", False, str(e), time.time() - t0)

        # Test 1.1.5: Programmatic Server Startup on custom port
        t0 = time.time()
        custom_port = get_free_port()
        try:
            with ServerProcess(entrypoint="public/index.php", port=custom_port) as s:
                st, hd, bd = raw_http_request(s.host, s.port, "GET", "/")
                passed = (st == 200)
                self.record("T1.1.5", 1, f"Programmatic startup binds and listens on custom port {custom_port}", passed, f"Status: {st}", time.time() - t0)
        except Exception as e:
            self.record("T1.1.5", 1, f"Programmatic startup binds and listens on custom port {custom_port}", False, str(e), time.time() - t0)

        # Test 1.1.6: Server Startup with custom entrypoint
        t0 = time.time()
        try:
            with ServerProcess(entrypoint="tests/fixtures/info.php") as s:
                st, hd, bd = raw_http_request(s.host, s.port, "GET", "/")
                data = json.loads(bd.decode("utf-8", errors="ignore"))
                passed = (st == 200 and data.get("engine") == "RestPHP")
                self.record("T1.1.6", 1, "Startup with custom entrypoint script loads successfully", passed, f"Status: {st}", time.time() - t0)
        except Exception as e:
            self.record("T1.1.6", 1, "Startup with custom entrypoint script loads successfully", False, str(e), time.time() - t0)

        # --- Feature 2: AC 2 - Default inline test endpoint (GET /) ---
        with ServerProcess(entrypoint="public/index.php") as s:
            # Test 1.2.1: Status code 200 OK
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/")
            self.record("T1.2.1", 1, "Default endpoint GET / returns HTTP 200 OK", st == 200, f"Got status {st}", time.time() - t0)

            # Test 1.2.2: JSON body contains status: ok
            t0 = time.time()
            try:
                data = json.loads(bd.decode("utf-8"))
                passed = data.get("status") == "ok"
                self.record("T1.2.2", 1, "Default endpoint body contains 'status': 'ok'", passed, f"Body: {data}", time.time() - t0)
            except Exception as e:
                self.record("T1.2.2", 1, "Default endpoint body contains 'status': 'ok'", False, str(e), time.time() - t0)

            # Test 1.2.3: JSON body contains engine: RestPHP
            t0 = time.time()
            try:
                data = json.loads(bd.decode("utf-8"))
                passed = data.get("engine") == "RestPHP"
                self.record("T1.2.3", 1, "Default endpoint body contains 'engine': 'RestPHP'", passed, f"Body: {data}", time.time() - t0)
            except Exception as e:
                self.record("T1.2.3", 1, "Default endpoint body contains 'engine': 'RestPHP'", False, str(e), time.time() - t0)

            # Test 1.2.4: Server header contains RestPHP
            t0 = time.time()
            srv_header = hd.get("server", "")
            passed = "RestPHP" in srv_header
            self.record("T1.2.4", 1, "Response headers include 'Server: RestPHP/...'", passed, f"Header: {srv_header}", time.time() - t0)

            # Test 1.2.5: Content-Type header is application/json
            t0 = time.time()
            ct_header = hd.get("content-type", "")
            passed = "application/json" in ct_header
            # Note: If server returns text/html instead, this is an implementation bug to escalate
            self.record("T1.2.5", 1, "Response Content-Type is application/json", passed, f"Got: '{ct_header}', expected: 'application/json'", time.time() - t0, is_bug=not passed)

            # Test 1.2.6: PHP version is reported
            t0 = time.time()
            try:
                data = json.loads(bd.decode("utf-8"))
                passed = "php_version" in data and str(data["php_version"]).startswith("8.")
                self.record("T1.2.6", 1, "Response contains valid PHP 8.x version info", passed, f"Version: {data.get('php_version')}", time.time() - t0)
            except Exception as e:
                self.record("T1.2.6", 1, "Response contains valid PHP 8.x version info", False, str(e), time.time() - t0)

        # --- Feature 3: AC 3 - Superglobal Mapping ($_GET, $_POST, php://input, $_COOKIE, $_SERVER) ---
        with ServerProcess(entrypoint="tests/fixtures/info.php") as s:
            # Test 1.3.1: $_GET query parameters
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/test?user=alice&role=developer")
            try:
                data = json.loads(bd.decode("utf-8"))
                get_params = data.get("get", {})
                passed = (get_params.get("user") == "alice" and get_params.get("role") == "developer")
                self.record("T1.3.1", 1, "Query parameters correctly parsed into $_GET", passed, f"Got $_GET: {get_params}", time.time() - t0)
            except Exception as e:
                self.record("T1.3.1", 1, "Query parameters correctly parsed into $_GET", False, str(e), time.time() - t0)

            # Test 1.3.2: $_GET array parameters (items[]=1&items[]=2)
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/test?items[]=apple&items[]=banana")
            try:
                data = json.loads(bd.decode("utf-8"))
                items = data.get("get", {}).get("items")
                passed = (items == ["apple", "banana"] or items == {"0": "apple", "1": "banana"})
                self.record("T1.3.2", 1, "Query array parameters parsed into $_GET array", passed, f"Got: {items}", time.time() - t0)
            except Exception as e:
                self.record("T1.3.2", 1, "Query array parameters parsed into $_GET array", False, str(e), time.time() - t0)

            # Test 1.3.3: $_POST form-urlencoded
            t0 = time.time()
            body_form = "name=John+Doe&email=john%40example.com&age=30"
            st, hd, bd = raw_http_request(s.host, s.port, "POST", "/form",
                                         headers={"Content-Type": "application/x-www-form-urlencoded"},
                                         body=body_form)
            try:
                data = json.loads(bd.decode("utf-8"))
                post_data = safe_get(data, "post", {})
                passed = (safe_get(post_data, "name") == "John Doe" and safe_get(post_data, "email") == "john@example.com" and safe_get(post_data, "age") == "30")
                self.record("T1.3.3", 1, "Form data parsed into $_POST for urlencoded body", passed, f"Got $_POST: {post_data}", time.time() - t0, is_bug=not passed)
            except Exception as e:
                self.record("T1.3.3", 1, "Form data parsed into $_POST for urlencoded body", False, str(e), time.time() - t0, is_bug=True)

            # Test 1.3.4: php://input raw JSON stream
            t0 = time.time()
            json_payload = json.dumps({"action": "create", "id": 101, "tags": ["rust", "php"]})
            st, hd, bd = raw_http_request(s.host, s.port, "POST", "/api",
                                         headers={"Content-Type": "application/json"},
                                         body=json_payload)
            try:
                data = json.loads(bd.decode("utf-8"))
                raw_input = data.get("raw_input", "")
                passed = (raw_input == json_payload)
                self.record("T1.3.4", 1, "Raw JSON body readable via php://input", passed, f"Got raw_input: {raw_input}", time.time() - t0, is_bug=not passed)
            except Exception as e:
                self.record("T1.3.4", 1, "Raw JSON body readable via php://input", False, str(e), time.time() - t0, is_bug=True)

            # Test 1.3.5: $_SERVER Request Method and URI
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "POST", "/v1/resource?id=99")
            try:
                data = json.loads(bd.decode("utf-8"))
                srv = safe_get(data, "server", {})
                passed = (safe_get(srv, "REQUEST_METHOD") == "POST" and safe_get(srv, "REQUEST_URI") == "/v1/resource" and safe_get(srv, "QUERY_STRING") == "id=99")
                self.record("T1.3.5", 1, "$_SERVER contains accurate REQUEST_METHOD, REQUEST_URI, QUERY_STRING", passed, f"Got $_SERVER: {srv}", time.time() - t0)
            except Exception as e:
                self.record("T1.3.5", 1, "$_SERVER contains accurate REQUEST_METHOD, REQUEST_URI, QUERY_STRING", False, str(e), time.time() - t0)

            # Test 1.3.6: $_COOKIE mapping from Cookie header
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/auth",
                                         headers={"Cookie": "session_id=sess_abcdef123; user_pref=dark_mode"})
            try:
                data = json.loads(bd.decode("utf-8"))
                cookies = safe_get(data, "cookie", {})
                passed = (safe_get(cookies, "session_id") == "sess_abcdef123" and safe_get(cookies, "user_pref") == "dark_mode")
                # Cookie mapping may fail in current build if headers aren't dispatched; flag as bug
                self.record("T1.3.6", 1, "Cookie header mapped directly into $_COOKIE", passed, f"Got $_COOKIE: {cookies}", time.time() - t0, is_bug=not passed)
            except Exception as e:
                self.record("T1.3.6", 1, "Cookie header mapped directly into $_COOKIE", False, str(e), time.time() - t0, is_bug=True)

        # --- Feature 4: AC 4 - Clean lifecycle recycling & consecutive requests ---
        with ServerProcess(entrypoint="tests/fixtures/lifecycle.php") as s:
            # Test 1.4.1: Consecutive requests succeed without crash (10 requests)
            t0 = time.time()
            consecutive_ok = True
            err_msg = ""
            for i in range(10):
                st, hd, bd = raw_http_request(s.host, s.port, "GET", f"/lifecycle?req_id={i}")
                if st != 200:
                    consecutive_ok = False
                    err_msg = f"Request {i} failed with status {st}"
                    break
            self.record("T1.4.1", 1, "10 consecutive requests execute cleanly without crashing", consecutive_ok, err_msg, time.time() - t0)

            # Test 1.4.2: Query string isolation between consecutive requests
            t0 = time.time()
            st1, _, bd1 = raw_http_request(s.host, s.port, "GET", "/lifecycle?token=secret123&action=delete")
            st2, _, bd2 = raw_http_request(s.host, s.port, "GET", "/lifecycle?only_new_param=1")
            try:
                data2 = json.loads(bd2.decode("utf-8"))
                query2 = safe_get(data2, "current_query", {})
                passed = ("token" not in query2 and "action" not in query2 and safe_get(query2, "only_new_param") == "1")
                self.record("T1.4.2", 1, "Consecutive requests maintain strict query parameter isolation", passed, f"Req 2 query: {query2}", time.time() - t0)
            except Exception as e:
                self.record("T1.4.2", 1, "Consecutive requests maintain strict query parameter isolation", False, str(e), time.time() - t0)

            # Test 1.4.3: Global scope leak prevention across requests
            t0 = time.time()
            st1, _, bd1 = raw_http_request(s.host, s.port, "GET", "/lifecycle?req_id=pollute_global")
            st2, _, bd2 = raw_http_request(s.host, s.port, "GET", "/lifecycle?req_id=check_leak")
            try:
                data2 = json.loads(bd2.decode("utf-8"))
                had_leak = data2.get("had_previous_leak", False)
                passed = not had_leak
                self.record("T1.4.3", 1, "PHP request teardown resets global symbol table between requests", passed, f"Leak detected: {data2.get('previous_value')}", time.time() - t0, is_bug=not passed)
            except Exception as e:
                self.record("T1.4.3", 1, "PHP request teardown resets global symbol table between requests", False, str(e), time.time() - t0)

            # Test 1.4.4: Sequential different HTTP methods (GET, POST, PUT, DELETE)
            t0 = time.time()
            methods = ["GET", "POST", "PUT", "DELETE", "GET"]
            methods_ok = True
            m_err = ""
            for m in methods:
                st, _, bd = raw_http_request(s.host, s.port, m, f"/lifecycle?req_id={m}")
                if st != 200:
                    methods_ok = False
                    m_err = f"Method {m} failed with status {st}"
                    break
            self.record("T1.4.4", 1, "Sequential alternating HTTP methods transition cleanly", methods_ok, m_err, time.time() - t0)

            # Test 1.4.5: Sustained request recycling (25 requests)
            t0 = time.time()
            sustained_ok = True
            for i in range(25):
                st, _, _ = raw_http_request(s.host, s.port, "GET", f"/lifecycle?req_id={i}")
                if st != 200:
                    sustained_ok = False
                    break
            self.record("T1.4.5", 1, "Sustained request recycling loop completes 25 requests with 100% 200 OK", sustained_ok, "", time.time() - t0)

    # =========================================================================
    # TIER 2: BOUNDARY & CORNER CASES (>=5 tests per feature)
    # =========================================================================
    def run_tier_2(self):
        # --- Category 2.1: Body Boundaries & Extreme Sizes ---
        with ServerProcess(entrypoint="tests/fixtures/large_body.php") as s:
            # Test 2.1.1: Empty POST body (0 bytes)
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "POST", "/large", headers={"Content-Type": "application/octet-stream"}, body=b"")
            try:
                data = json.loads(bd.decode("utf-8"))
                passed = (st == 200 and data.get("received_len") == 0)
                self.record("T2.1.1", 2, "Empty POST body (0 bytes) handled cleanly", passed, f"Data: {data}", time.time() - t0)
            except Exception as e:
                self.record("T2.1.1", 2, "Empty POST body (0 bytes) handled cleanly", False, str(e), time.time() - t0)

            # Test 2.1.2: 16KB Payload (chunk boundary)
            t0 = time.time()
            payload_16k = b"A" * (16 * 1024)
            expected_md5 = hashlib.md5(payload_16k).hexdigest()
            st, hd, bd = raw_http_request(s.host, s.port, "POST", "/large", headers={"Content-Type": "application/octet-stream"}, body=payload_16k)
            try:
                data = json.loads(bd.decode("utf-8"))
                passed = (st == 200 and data.get("received_len") == len(payload_16k) and data.get("received_md5") == expected_md5)
                self.record("T2.1.2", 2, "16KB payload streamed without truncation", passed, f"Expected len {len(payload_16k)}, got {data.get('received_len')}", time.time() - t0, is_bug=not passed)
            except Exception as e:
                self.record("T2.1.2", 2, "16KB payload streamed without truncation", False, str(e), time.time() - t0, is_bug=True)

            # Test 2.1.3: 64KB Payload (large body)
            t0 = time.time()
            payload_64k = b"B" * (64 * 1024)
            expected_md5_64k = hashlib.md5(payload_64k).hexdigest()
            st, hd, bd = raw_http_request(s.host, s.port, "POST", "/large", headers={"Content-Type": "application/octet-stream"}, body=payload_64k)
            try:
                data = json.loads(bd.decode("utf-8"))
                passed = (st == 200 and data.get("received_len") == len(payload_64k) and data.get("received_md5") == expected_md5_64k)
                self.record("T2.1.3", 2, "64KB large payload streamed with exact MD5 match", passed, f"Got: {data}", time.time() - t0, is_bug=not passed)
            except Exception as e:
                self.record("T2.1.3", 2, "64KB large payload streamed with exact MD5 match", False, str(e), time.time() - t0, is_bug=True)

            # Test 2.1.4: Binary Payload with null bytes
            t0 = time.time()
            binary_data = bytes([i % 256 for i in range(2048)])
            expected_bin_md5 = hashlib.md5(binary_data).hexdigest()
            st, hd, bd = raw_http_request(s.host, s.port, "POST", "/large", headers={"Content-Type": "application/octet-stream"}, body=binary_data)
            try:
                data = json.loads(bd.decode("utf-8"))
                passed = (st == 200 and data.get("received_len") == len(binary_data) and data.get("received_md5") == expected_bin_md5)
                self.record("T2.1.4", 2, "Binary payload containing null bytes preserved without truncation", passed, f"Got: {data}", time.time() - t0, is_bug=not passed)
            except Exception as e:
                self.record("T2.1.4", 2, "Binary payload containing null bytes preserved without truncation", False, str(e), time.time() - t0, is_bug=True)

            # Test 2.1.5: Empty JSON structures ({}, [])
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "POST", "/large", headers={"Content-Type": "application/json"}, body=b"{}")
            try:
                data = json.loads(bd.decode("utf-8"))
                passed = (st == 200 and data.get("received_len") == 2)
                self.record("T2.1.5", 2, "Empty JSON object '{}' body received with exact length", passed, f"Got: {data}", time.time() - t0, is_bug=not passed)
            except Exception as e:
                self.record("T2.1.5", 2, "Empty JSON object '{}' body received with exact length", False, str(e), time.time() - t0, is_bug=True)

        # --- Category 2.2: Query String Boundaries & Escaping ---
        with ServerProcess(entrypoint="tests/fixtures/info.php") as s:
            # Test 2.2.1: Empty query string (GET /?)
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/test?")
            try:
                data = json.loads(bd.decode("utf-8"))
                passed = (st == 200 and data.get("query_string") == "" and len(data.get("get", {})) == 0)
                self.record("T2.2.1", 2, "Trailing '?' with empty query string produces empty $_GET", passed, f"Got: {data.get('get')}", time.time() - t0)
            except Exception as e:
                self.record("T2.2.1", 2, "Trailing '?' with empty query string produces empty $_GET", False, str(e), time.time() - t0)

            # Test 2.2.2: Special characters & URL encoding in query string
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/test?text=Hello+World%21&math=1%2B1%3D2&quote=%22escaped%22")
            try:
                data = json.loads(bd.decode("utf-8"))
                g = data.get("get", {})
                passed = (g.get("text") == "Hello World!" and g.get("math") == "1+1=2" and g.get("quote") == '"escaped"')
                self.record("T2.2.2", 2, "URL-encoded special characters (&, =, +, quotes) correctly decoded", passed, f"Got: {g}", time.time() - t0)
            except Exception as e:
                self.record("T2.2.2", 2, "URL-encoded special characters (&, =, +, quotes) correctly decoded", False, str(e), time.time() - t0)

            # Test 2.2.3: Multilingual Unicode in query string (München, 日本語)
            t0 = time.time()
            encoded_query = "city=" + urllib.parse.quote("München") + "&lang=" + urllib.parse.quote("日本語")
            st, hd, bd = raw_http_request(s.host, s.port, "GET", f"/test?{encoded_query}")
            try:
                data = json.loads(bd.decode("utf-8"))
                g = data.get("get", {})
                passed = (g.get("city") == "München" and g.get("lang") == "日本語")
                self.record("T2.2.3", 2, "UTF-8 multilingual unicode in query string properly preserved", passed, f"Got: {g}", time.time() - t0)
            except Exception as e:
                self.record("T2.2.3", 2, "UTF-8 multilingual unicode in query string properly preserved", False, str(e), time.time() - t0)

            # Test 2.2.4: Query param with no value (?flag&active)
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/test?flag&active")
            try:
                data = json.loads(bd.decode("utf-8"))
                g = data.get("get", {})
                passed = ("flag" in g and "active" in g)
                self.record("T2.2.4", 2, "Valueless query flags (?flag&active) recognized in $_GET", passed, f"Got: {g}", time.time() - t0)
            except Exception as e:
                self.record("T2.2.4", 2, "Valueless query flags (?flag&active) recognized in $_GET", False, str(e), time.time() - t0)

            # Test 2.2.5: 2KB long query string
            t0 = time.time()
            long_query = "&".join([f"k{i}=val_{'x'*20}" for i in range(50)])
            st, hd, bd = raw_http_request(s.host, s.port, "GET", f"/test?{long_query}")
            try:
                data = json.loads(bd.decode("utf-8"))
                g = data.get("get", {})
                passed = (len(g) == 50 and g.get("k0") == f"val_{'x'*20}")
                self.record("T2.2.5", 2, "2KB long query string with 50 parameters parsed without buffer overrun", passed, f"Keys parsed: {len(g)}", time.time() - t0)
            except Exception as e:
                self.record("T2.2.5", 2, "2KB long query string with 50 parameters parsed without buffer overrun", False, str(e), time.time() - t0)

        # --- Category 2.3: Headers & Cookie Boundaries ---
        with ServerProcess(entrypoint="tests/fixtures/info.php") as s:
            # Test 2.3.1: Missing Cookie header (no null pointer SIGSEGV)
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/no-cookies")
            try:
                data = json.loads(bd.decode("utf-8"))
                passed = (st == 200 and data.get("cookie") == [])
                self.record("T2.3.1", 2, "Missing Cookie header does not cause null dereference or crash", passed, f"Got: {data.get('cookie')}", time.time() - t0)
            except Exception as e:
                self.record("T2.3.1", 2, "Missing Cookie header does not cause null dereference or crash", False, str(e), time.time() - t0)

            # Test 2.3.2: Empty Cookie header value (Cookie: )
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/empty-cookie", headers={"Cookie": ""})
            passed = (st == 200)
            self.record("T2.3.2", 2, "Empty Cookie header handled cleanly", passed, f"Status: {st}", time.time() - t0)

            # Test 2.3.3: Malformed Cookie header with semicolons
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/malformed-cookie", headers={"Cookie": ";;; a=1; ; b=2;; ;="})
            passed = (st == 200)
            self.record("T2.3.3", 2, "Malformed Cookie header with redundant semicolons handled without crash", passed, f"Status: {st}", time.time() - t0)

            # Test 2.3.4: Custom HTTP Headers in $_SERVER
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/custom-headers", headers={"X-RestPHP-Trace": "trace_9999"})
            try:
                data = json.loads(bd.decode("utf-8"))
                srv = data.get("server", {})
                passed = (srv.get("HTTP_X_RESTPHP_TRACE") == "trace_9999")
                # Custom header mapping may fail in current build; flag as bug
                self.record("T2.3.4", 2, "Custom request header mapped to $_SERVER['HTTP_X_RESTPHP_TRACE']", passed, f"Got server vars: {srv}", time.time() - t0, is_bug=not passed)
            except Exception as e:
                self.record("T2.3.4", 2, "Custom request header mapped to $_SERVER['HTTP_X_RESTPHP_TRACE']", False, str(e), time.time() - t0, is_bug=True)

            # Test 2.3.5: Mixed-case headers
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/case-header", headers={"x-cUsToM-hEaDeR": "mixed_case_val"})
            passed = (st == 200)
            self.record("T2.3.5", 2, "Mixed-case HTTP headers handled without parser rejection", passed, f"Status: {st}", time.time() - t0)

        # --- Category 2.4: HTTP Methods & Status Codes ---
        with ServerProcess(entrypoint="tests/fixtures/status_and_headers.php") as s:
            # Test 2.4.1: HTTP Method PUT
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "PUT", "/status?code=200")
            passed = (st == 200)
            self.record("T2.4.1", 2, "HTTP PUT method accepted and processed", passed, f"Status: {st}", time.time() - t0)

            # Test 2.4.2: HTTP Method DELETE
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "DELETE", "/status?code=200")
            passed = (st == 200)
            self.record("T2.4.2", 2, "HTTP DELETE method accepted and processed", passed, f"Status: {st}", time.time() - t0)

            # Test 2.4.3: HTTP Method PATCH
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "PATCH", "/status?code=200")
            passed = (st == 200)
            self.record("T2.4.3", 2, "HTTP PATCH method accepted and processed", passed, f"Status: {st}", time.time() - t0)

            # Test 2.4.4: Dynamic Status Code 201 Created
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/status?code=201")
            passed = (st == 201)
            # If server does not propagate http_response_code(), this is an implementation bug
            self.record("T2.4.4", 2, "PHP http_response_code(201) reflected in HTTP response status", passed, f"Got status: {st}, expected 201", time.time() - t0, is_bug=not passed)

            # Test 2.4.5: Dynamic Status Code 404 Not Found
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/status?code=404")
            passed = (st == 404)
            self.record("T2.4.5", 2, "PHP http_response_code(404) reflected in HTTP response status", passed, f"Got status: {st}, expected 404", time.time() - t0, is_bug=not passed)

            # Test 2.4.6: Dynamic Status Code 204 No Content
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/status?code=204")
            passed = (st == 204)
            self.record("T2.4.6", 2, "PHP http_response_code(204) reflected in HTTP response status", passed, f"Got status: {st}, expected 204", time.time() - t0, is_bug=not passed)

    # =========================================================================
    # TIER 3: CROSS-FEATURE COMBINATIONS (Pairwise Coverage)
    # =========================================================================
    def run_tier_3(self):
        with ServerProcess(entrypoint="tests/fixtures/info.php") as s:
            # Test 3.1: Query params + Form POST body + Cookies simultaneously
            t0 = time.time()
            form_body = "form_key=hello_post&amount=99"
            st, hd, bd = raw_http_request(
                s.host, s.port, "POST", "/combo?filter=active&sort=desc",
                headers={
                    "Content-Type": "application/x-www-form-urlencoded",
                    "Cookie": "uid=user_77; authtoken=xyz987"
                },
                body=form_body
            )
            try:
                data = json.loads(bd.decode("utf-8"))
                g = safe_get(data, "get", {})
                p = safe_get(data, "post", {})
                c = safe_get(data, "cookie", {})
                q_ok = (safe_get(g, "filter") == "active" and safe_get(g, "sort") == "desc")
                p_ok = (safe_get(p, "form_key") == "hello_post" and safe_get(p, "amount") == "99")
                c_ok = (safe_get(c, "uid") == "user_77")
                passed = (st == 200 and q_ok and p_ok and c_ok)
                self.record("T3.1", 3, "Query params + Form POST + Cookie header simultaneously populated", passed, f"GET:{g}, POST:{p}, COOKIE:{c}", time.time() - t0, is_bug=not passed)
            except Exception as e:
                self.record("T3.1", 3, "Query params + Form POST + Cookie header simultaneously populated", False, str(e), time.time() - t0, is_bug=True)

            # Test 3.2: Query params + JSON body + Cookies simultaneously
            t0 = time.time()
            json_body = json.dumps({"nested": {"id": 42}, "flag": True})
            st, hd, bd = raw_http_request(
                s.host, s.port, "POST", "/combo-json?action=sync&v=2",
                headers={
                    "Content-Type": "application/json",
                    "Cookie": "session=sess_combo"
                },
                body=json_body
            )
            try:
                data = json.loads(bd.decode("utf-8"))
                g = safe_get(data, "get", {})
                raw_in = data.get("raw_input", "")
                c = safe_get(data, "cookie", {})
                q_ok = (safe_get(g, "action") == "sync" and safe_get(g, "v") == "2")
                in_ok = (raw_in == json_body)
                c_ok = (safe_get(c, "session") == "sess_combo")
                passed = (st == 200 and q_ok and in_ok and c_ok)
                self.record("T3.2", 3, "Query params + JSON body stream + Cookie simultaneously accessible", passed, f"GET:{g}, Input matches:{in_ok}, COOKIE:{c}", time.time() - t0, is_bug=not passed)
            except Exception as e:
                self.record("T3.2", 3, "Query params + JSON body stream + Cookie simultaneously accessible", False, str(e), time.time() - t0, is_bug=True)

            # Test 3.3: Large 32KB payload + Query string + Custom headers
            t0 = time.time()
            large_32k = b"X" * (32 * 1024)
            large_md5 = hashlib.md5(large_32k).hexdigest()
            st, hd, bd = raw_http_request(
                s.host, s.port, "POST", "/large-combo?checksum=md5&type=raw",
                headers={
                    "Content-Type": "application/octet-stream",
                    "X-Batch-ID": "batch-888"
                },
                body=large_32k
            )
            try:
                data = json.loads(bd.decode("utf-8"))
                g = safe_get(data, "get", {})
                raw_len = data.get("raw_input_len", 0)
                raw_md5 = data.get("raw_input_md5", "")
                len_ok = (raw_len == len(large_32k))
                md5_ok = (raw_md5 == large_md5)
                q_ok = (safe_get(g, "checksum") == "md5")
                passed = (st == 200 and len_ok and md5_ok and q_ok)
                self.record("T3.3", 3, "32KB payload + Query params + Custom headers integrated smoothly", passed, f"len_ok:{len_ok}, md5_ok:{md5_ok}, q_ok:{q_ok}", time.time() - t0, is_bug=not passed)
            except Exception as e:
                self.record("T3.3", 3, "32KB payload + Query params + Custom headers integrated smoothly", False, str(e), time.time() - t0, is_bug=True)

        # Test 3.4: Dynamic Status Code 201 + Custom response header + JSON body
        with ServerProcess(entrypoint="tests/fixtures/status_and_headers.php") as s:
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/status?code=201&header_key=X-Custom-Resource&header_val=Created-12345")
            has_custom_hdr = "x-custom-resource" in hd
            passed = (st == 201 and has_custom_hdr)
            self.record("T3.4", 3, "Dynamic HTTP 201 status + Custom response header combined", passed, f"Status: {st}, Headers: {hd}", time.time() - t0, is_bug=not passed)

        # Test 3.5: Multilingual UTF-8 characters across query, POST body, and Cookies
        with ServerProcess(entrypoint="tests/fixtures/utf8_special.php") as s:
            t0 = time.time()
            utf8_form = "message=" + urllib.parse.quote("Bonjour le monde, café & thé")
            st, hd, bd = raw_http_request(
                s.host, s.port, "POST", "/utf8?tag=" + urllib.parse.quote("こんにちは"),
                headers={
                    "Content-Type": "application/x-www-form-urlencoded",
                    "Cookie": "locale=fr_FR; greeting=" + urllib.parse.quote("здравствуйте")
                },
                body=utf8_form
            )
            try:
                data = json.loads(bd.decode("utf-8"))
                q_tag = safe_get(safe_get(data, "query", {}), "tag")
                p_msg = safe_get(safe_get(data, "post", {}), "message")
                passed = (st == 200 and q_tag == "こんにちは" and p_msg == "Bonjour le monde, café & thé")
                self.record("T3.5", 3, "UTF-8 multilingual data preserved across Query and POST body", passed, f"Got: {data}", time.time() - t0, is_bug=not passed)
            except Exception as e:
                self.record("T3.5", 3, "UTF-8 multilingual data preserved across Query and POST body", False, str(e), time.time() - t0, is_bug=True)


        # Test 3.6: Rapid alternating payload types (JSON -> empty GET -> Form -> Binary)
        with ServerProcess(entrypoint="tests/fixtures/info.php") as s:
            t0 = time.time()
            cycle_ok = True
            # Step 1: JSON
            st1, _, bd1 = raw_http_request(s.host, s.port, "POST", "/step1", headers={"Content-Type": "application/json"}, body=b'{"step":1}')
            # Step 2: Empty GET
            st2, _, bd2 = raw_http_request(s.host, s.port, "GET", "/step2")
            # Step 3: Form
            st3, _, bd3 = raw_http_request(s.host, s.port, "POST", "/step3", headers={"Content-Type": "application/x-www-form-urlencoded"}, body=b"step=3")
            # Step 4: Binary
            st4, _, bd4 = raw_http_request(s.host, s.port, "POST", "/step4", headers={"Content-Type": "application/octet-stream"}, body=b"\x00\x01\x02\x03")
            passed = (st1 == 200 and st2 == 200 and st3 == 200 and st4 == 200)
            self.record("T3.6", 3, "Rapid alternating payload sequence (JSON -> GET -> Form -> Binary) succeeds", passed, f"Statuses: {[st1, st2, st3, st4]}", time.time() - t0)

    # =========================================================================
    # TIER 4: REAL-WORLD APPLICATION SCENARIOS
    # =========================================================================
    def run_tier_4(self):
        # --- Category 4.1: Real-World REST API CRUD Lifecycle ---
        with ServerProcess(entrypoint="tests/fixtures/crud.php") as s:
            created_id = None

            # Test 4.1.1: CREATE item (POST /items) -> 201 Created
            t0 = time.time()
            new_item = {"name": "ThinkPad P1", "price": 2500, "category": "workstation"}
            st, hd, bd = raw_http_request(s.host, s.port, "POST", "/items",
                                         headers={"Content-Type": "application/json"},
                                         body=json.dumps(new_item))
            try:
                data = json.loads(bd.decode("utf-8"))
                created_id = str(data.get("item", {}).get("id"))
                passed = (data.get("status") == "created" and created_id is not None)
                self.record("T4.1.1", 4, "CRUD POST /items creates resource and returns created entity", passed, f"Status: {st}, Data: {data}", time.time() - t0)
            except Exception as e:
                self.record("T4.1.1", 4, "CRUD POST /items creates resource and returns created entity", False, str(e), time.time() - t0)

            # Test 4.1.2: READ item (GET /items?id=<id>) -> 200 OK
            t0 = time.time()
            if created_id:
                st, hd, bd = raw_http_request(s.host, s.port, "GET", f"/items?id={created_id}")
                try:
                    data = json.loads(bd.decode("utf-8"))
                    item = data.get("item", {})
                    passed = (st == 200 and item.get("name") == "ThinkPad P1" and item.get("price") == 2500)
                    self.record("T4.1.2", 4, f"CRUD GET /items?id={created_id} fetches stored resource", passed, f"Got: {data}", time.time() - t0, is_bug=not passed)
                except Exception as e:
                    self.record("T4.1.2", 4, f"CRUD GET /items?id={created_id} fetches stored resource", False, str(e), time.time() - t0, is_bug=True)
            else:
                self.record("T4.1.2", 4, "CRUD GET item skipped due to creation failure", False, "No created_id", time.time() - t0)

            # Test 4.1.3: UPDATE item (PUT /items?id=<id>) -> 200 OK
            t0 = time.time()
            if created_id:
                update_payload = {"price": 2200}
                st, hd, bd = raw_http_request(s.host, s.port, "PUT", f"/items?id={created_id}",
                                             headers={"Content-Type": "application/json"},
                                             body=json.dumps(update_payload))
                try:
                    data = json.loads(bd.decode("utf-8"))
                    item = data.get("item", {})
                    passed = (item.get("price") == 2200 and item.get("name") == "ThinkPad P1")
                    self.record("T4.1.3", 4, f"CRUD PUT /items?id={created_id} updates resource price", passed, f"Got: {data}", time.time() - t0, is_bug=not passed)
                except Exception as e:
                    self.record("T4.1.3", 4, f"CRUD PUT /items?id={created_id} updates resource price", False, str(e), time.time() - t0, is_bug=True)

            else:
                self.record("T4.1.3", 4, "CRUD PUT item skipped", False, "No created_id", time.time() - t0)

            # Test 4.1.4: LIST items (GET /items) -> 200 OK
            t0 = time.time()
            st, hd, bd = raw_http_request(s.host, s.port, "GET", "/items")
            try:
                data = json.loads(bd.decode("utf-8"))
                items = data.get("items", [])
                passed = (st == 200 and len(items) >= 1)
                self.record("T4.1.4", 4, "CRUD GET /items returns array of active resources", passed, f"Items count: {len(items)}", time.time() - t0)
            except Exception as e:
                self.record("T4.1.4", 4, "CRUD GET /items returns array of active resources", False, str(e), time.time() - t0)

            # Test 4.1.5: DELETE item (DELETE /items?id=<id>) -> 200 OK
            t0 = time.time()
            if created_id:
                st, hd, bd = raw_http_request(s.host, s.port, "DELETE", f"/items?id={created_id}")
                try:
                    data = json.loads(bd.decode("utf-8"))
                    passed = (data.get("status") == "deleted")
                    self.record("T4.1.5", 4, f"CRUD DELETE /items?id={created_id} deletes resource", passed, f"Got: {data}", time.time() - t0)
                except Exception as e:
                    self.record("T4.1.5", 4, f"CRUD DELETE /items?id={created_id} deletes resource", False, str(e), time.time() - t0)
            else:
                self.record("T4.1.5", 4, "CRUD DELETE item skipped", False, "No created_id", time.time() - t0)

            # Test 4.1.6: VERIFY deleted item returns 404 (GET /items?id=<id>) -> 404
            t0 = time.time()
            if created_id:
                st, hd, bd = raw_http_request(s.host, s.port, "GET", f"/items?id={created_id}")
                try:
                    data = json.loads(bd.decode("utf-8"))
                    passed = (data.get("status") == "error" and data.get("message") == "Item not found")
                    self.record("T4.1.6", 4, f"CRUD GET deleted resource returns not found", passed, f"Got: {data}", time.time() - t0)
                except Exception as e:
                    self.record("T4.1.6", 4, f"CRUD GET deleted resource returns not found", False, str(e), time.time() - t0)
            else:
                self.record("T4.1.6", 4, "CRUD verify deleted skipped", False, "No created_id", time.time() - t0)

        # --- Category 4.2: High-Throughput Concurrency Stress Test ---
        with ServerProcess(entrypoint="public/index.php") as s:
            # Test 4.2.1: 100 concurrent requests across 10 workers
            t0 = time.time()
            num_requests = 100
            num_threads = 10

            def worker_task(req_index):
                st, _, bd = raw_http_request(s.host, s.port, "GET", f"/?req={req_index}")
                return st == 200

            successes = 0
            with concurrent.futures.ThreadPoolExecutor(max_workers=num_threads) as executor:
                futures = [executor.submit(worker_task, i) for i in range(num_requests)]
                for f in concurrent.futures.as_completed(futures):
                    try:
                        if f.result():
                            successes += 1
                    except Exception:
                        pass

            duration = time.time() - t0
            passed = (successes == num_requests)
            rps = round(num_requests / duration, 1) if duration > 0 else 0
            self.record("T4.2.1", 4, f"High-concurrency stress test: {num_requests} requests across {num_threads} threads (100% 200 OK, {rps} req/s)",
                        passed, f"Success rate: {successes}/{num_requests}", duration)

            # Test 4.2.2: 50 rapid sequential requests
            t0 = time.time()
            seq_count = 50
            seq_ok = True
            for i in range(seq_count):
                st, _, _ = raw_http_request(s.host, s.port, "GET", f"/?seq={i}")
                if st != 200:
                    seq_ok = False
                    break
            dur = time.time() - t0
            self.record("T4.2.2", 4, f"Rapid sequential test: {seq_count} requests with 0 dropped connections ({round(seq_count/dur, 1)} req/s)",
                        seq_ok, "", dur)

        # --- Category 4.3: Error Resilience & Recovery ---
        with ServerProcess(entrypoint="tests/fixtures/error.php") as s:
            # Test 4.3.1: Recovery after PHP notice
            t0 = time.time()
            st1, _, bd1 = raw_http_request(s.host, s.port, "GET", "/error?mode=notice")
            st2, _, bd2 = raw_http_request(s.host, s.port, "GET", "/error?mode=ok")
            passed = (st1 == 200 and st2 == 200 and b"ok" in bd2)
            self.record("T4.3.1", 4, "Server recovers immediately after PHP notice and processes subsequent requests", passed, f"st1={st1}, st2={st2}", time.time() - t0)

            # Test 4.3.2: Recovery after PHP warning
            t0 = time.time()
            st1, _, bd1 = raw_http_request(s.host, s.port, "GET", "/error?mode=warning")
            st2, _, bd2 = raw_http_request(s.host, s.port, "GET", "/error?mode=ok")
            passed = (st1 == 200 and st2 == 200 and b"ok" in bd2)
            self.record("T4.3.2", 4, "Server recovers immediately after PHP warning and processes subsequent requests", passed, f"st1={st1}, st2={st2}", time.time() - t0)

    def print_summary(self):
        total = len(self.results)
        passed = sum(1 for r in self.results if r.passed)
        failed = sum(1 for r in self.results if not r.passed and not r.is_bug)
        bugs = sum(1 for r in self.results if r.is_bug)

        print(f"\n{BOLD}{CYAN}======================================================================{RESET}")
        print(f"{BOLD}                        TEST EXECUTION SUMMARY                        {RESET}")
        print(f"{BOLD}{CYAN}======================================================================{RESET}")
        print(f"Total Tests Executed : {BOLD}{total}{RESET}")
        print(f"Passed               : {GREEN}{passed}{RESET} ({round(passed/total*100, 1) if total else 0}%)")
        print(f"Failed (Test Defect) : {RED}{failed}{RESET}")
        print(f"Implementation Bugs  : {YELLOW}{bugs}{RESET} (Escalations required)")
        print(f"{BOLD}{CYAN}======================================================================{RESET}\n")

        if bugs > 0:
            print(f"{BOLD}{YELLOW}Identified Implementation Bugs for Escalation:{RESET}")
            for r in self.results:
                if r.is_bug:
                    print(f"  - [{r.test_id}] {r.name}: {r.message}")
            print()

        # Write e2e_report.json
        report_path = os.path.join(PROJECT_ROOT, "tests", "e2e_report.json")
        try:
            with open(report_path, "w", encoding="utf-8") as f:
                json.dump({
                    "timestamp": time.time(),
                    "total": total,
                    "passed": passed,
                    "failed": failed,
                    "bugs": bugs,
                    "results": [r.to_dict() for r in self.results]
                }, f, indent=2)
            print(f"Detailed JSON test report written to: {report_path}\n")
        except Exception as e:
            print(f"Could not write report: {e}")

def main():
    parser = argparse.ArgumentParser(description="RestPHP Comprehensive E2E Test Suite Runner")
    parser.add_argument("--host", default="127.0.0.1", help="Target host")
    parser.add_argument("--port", type=int, default=None, help="Target port (if not specified, auto-spawns server)")
    parser.add_argument("--tier", type=int, choices=[1, 2, 3, 4], default=None, help="Run specific tier only")
    args = parser.parse_args()

    runner = E2ETestRunner(host=args.host, port=args.port)
    runner.run_all(selected_tier=args.tier)

if __name__ == "__main__":
    main()
