// tests/e2e_test_suite.rs
// RestPHP Comprehensive Rust E2E Integration Test Suite (Tiers 1 - 4)

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

static NEXT_PORT: AtomicU16 = AtomicU16::new(9200);

pub fn get_ephemeral_port() -> u16 {
    NEXT_PORT.fetch_add(1, Ordering::SeqCst)
}

#[derive(Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }
}

pub struct TestServer {
    child: Child,
    pub port: u16,
    pub entrypoint: String,
}

impl TestServer {
    pub fn start(entrypoint: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::start_with_max_requests(entrypoint, 0)
    }

    pub fn start_with_max_requests(
        entrypoint: &str,
        max_requests: u64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::start_with_args(
            entrypoint,
            vec!["--max-requests".to_string(), max_requests.to_string()],
        )
    }

    pub fn start_with_args(
        entrypoint: &str,
        extra_args: Vec<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let port = get_ephemeral_port();
        let bin_path = std::env::current_exe()?
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("restphp"))
            .unwrap_or_else(|| std::path::PathBuf::from("target/debug/restphp"));

        let final_bin = if bin_path.exists() {
            bin_path
        } else {
            std::path::PathBuf::from("target/debug/restphp")
        };

        let mut command = Command::new(&final_bin);
        command
            .arg("serve")
            .arg("--port")
            .arg(port.to_string())
            .arg("--entrypoint")
            .arg(entrypoint)
            .args(extra_args);
        let child = command.spawn()?;

        let mut server = TestServer {
            child,
            port,
            entrypoint: entrypoint.to_string(),
        };

        server.wait_for_ready(Duration::from_secs(5))?;
        Ok(server)
    }

    pub fn wait_for_ready(&mut self, timeout: Duration) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            // A readiness probe is deliberately connection-only: sending an HTTP
            // request would consume a max-requests budget before a test begins.
            if let Ok(stream) = TcpStream::connect(("127.0.0.1", self.port)) {
                drop(stream);
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err(format!(
            "Server failed to bind to port {} within {:?}",
            self.port, timeout
        )
        .into())
    }

    pub fn wait_for_exit_success(
        &mut self,
        timeout: Duration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Some(status) = self.child.try_wait()? {
                if status.success() {
                    return Ok(());
                }
                return Err(format!("Server exited unsuccessfully: {status}").into());
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        Err(format!("Server did not exit within {:?}", timeout).into())
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn send_http_request(
    port: u16,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> std::io::Result<HttpResponse> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let mut req = format!("{} {} HTTP/1.1\r\n", method, path);
    let has_host = headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host"));
    if !has_host {
        req.push_str(&format!("Host: 127.0.0.1:{}\r\n", port));
    }
    let mut has_content_length = false;
    let body_bytes = body.unwrap_or(&[]);

    for (k, v) in headers {
        if k.eq_ignore_ascii_case("content-length") {
            has_content_length = true;
        }
        req.push_str(&format!("{}: {}\r\n", k, v));
    }

    if !has_content_length && !body_bytes.is_empty() {
        req.push_str(&format!("Content-Length: {}\r\n", body_bytes.len()));
    }
    req.push_str("Connection: close\r\n\r\n");

    stream.write_all(req.as_bytes())?;
    if !body_bytes.is_empty() {
        stream.write_all(body_bytes)?;
    }
    stream.flush()?;

    let mut raw_resp = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => raw_resp.extend_from_slice(&buf[..n]),
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                break
            }
            Err(e) => return Err(e),
        }
    }

    parse_http_response(&raw_resp)
}

fn parse_http_response(raw: &[u8]) -> std::io::Result<HttpResponse> {
    let header_end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "No HTTP header delimiter found in response (len={}): {:?}",
                    raw.len(),
                    String::from_utf8_lossy(raw)
                ),
            )
        })?;

    let header_bytes = &raw[..header_end];
    let body = raw[header_end + 4..].to_vec();
    let header_str = String::from_utf8_lossy(header_bytes);

    let mut lines = header_str.lines();
    let status_line = lines.next().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "Empty HTTP response")
    })?;

    let parts: Vec<&str> = status_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid status line",
        ));
    }
    let status_code: u16 = parts[1].parse().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid status code: {}", e),
        )
    })?;

    let mut headers = Vec::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }

    Ok(HttpResponse {
        status_code,
        headers,
        body,
    })
}

// =========================================================================
// TIER 1: FEATURE COVERAGE
// =========================================================================

#[test]
fn test_tier1_cli_help() {
    let output = Command::new("target/debug/restphp")
        .arg("--help")
        .output()
        .expect("Failed to execute restphp --help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("serve"));
    assert!(stdout.contains("eval"));
}

#[test]
fn test_tier1_cli_version() {
    let output = Command::new("target/debug/restphp")
        .arg("--version")
        .output()
        .expect("Failed to execute restphp --version");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_tier1_cli_eval() {
    let output = Command::new("target/debug/restphp")
        .args(["eval", "echo 'Rust_E2E_Eval_OK';"])
        .output()
        .expect("Failed to execute restphp eval");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Rust_E2E_Eval_OK"));
}

#[test]
fn test_tier1_cli_rejects_multiple_nts_workers_before_listening() {
    let port = get_ephemeral_port().to_string();
    let output = Command::new("target/debug/restphp")
        .args([
            "serve",
            "--port",
            &port,
            "--entrypoint",
            "tests/fixtures/info.php",
            "--workers",
            "2",
        ])
        .output()
        .expect("Failed to execute restphp serve");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("worker"));
    assert!(
        stderr.contains("supervisor"),
        "error should direct horizontal scaling to a process supervisor: {stderr}"
    );
}

#[test]
fn test_tier1_missing_entrypoint_fails_before_binding_or_creating_a_file() {
    let missing_entrypoint = std::env::temp_dir().join(format!(
        "restphp-missing-entrypoint-{}-{}.php",
        std::process::id(),
        get_ephemeral_port()
    ));
    assert!(!missing_entrypoint.exists());
    let port = get_ephemeral_port().to_string();

    let output = Command::new("target/debug/restphp")
        .args([
            "serve",
            "--port",
            &port,
            "--entrypoint",
            missing_entrypoint
                .to_str()
                .expect("temporary path must be UTF-8"),
        ])
        .output()
        .expect("Failed to execute restphp serve");
    assert!(!output.status.success());
    assert!(
        !missing_entrypoint.exists(),
        "RestPHP must not create an example entrypoint in response to a typo"
    );
}

#[test]
fn test_tier1_default_endpoint_get_root() {
    let server = TestServer::start("public/index.php").expect("Server should start");
    let resp =
        send_http_request(server.port, "GET", "/", &[], None).expect("Request should succeed");
    assert_eq!(resp.status_code, 200);
    assert_eq!(resp.header("server"), Some("RestPHP/0.1.0"));
    let json = resp.json().expect("Body should be JSON");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["engine"], "RestPHP");
    assert!(json["php_version"].as_str().unwrap().starts_with("8."));
}

#[test]
fn test_tier1_superglobals_query_params() {
    let server = TestServer::start("tests/fixtures/info.php").expect("Server should start");
    let resp = send_http_request(
        server.port,
        "GET",
        "/test?user=charlie&role=tester",
        &[],
        None,
    )
    .expect("Request should succeed");
    assert_eq!(resp.status_code, 200);
    let json = resp.json().expect("Body should be JSON");
    assert_eq!(json["get"]["user"], "charlie");
    assert_eq!(json["get"]["role"], "tester");
}

#[test]
fn test_tier1_superglobals_query_array() {
    let server = TestServer::start("tests/fixtures/info.php").expect("Server should start");
    let resp = send_http_request(server.port, "GET", "/test?tag[]=rust&tag[]=php", &[], None)
        .expect("Request should succeed");
    assert_eq!(resp.status_code, 200);
    let json = resp.json().expect("Body should be JSON");
    let tags = &json["get"]["tag"];
    assert!(tags.is_array() || tags.is_object());
}

#[test]
fn test_tier1_superglobals_server_vars() {
    let server = TestServer::start("tests/fixtures/info.php").expect("Server should start");
    let resp = send_http_request(server.port, "POST", "/api/data?debug=1", &[], None)
        .expect("Request should succeed");
    assert_eq!(resp.status_code, 200);
    let json = resp.json().expect("Body should be JSON");
    assert_eq!(json["server"]["REQUEST_METHOD"], "POST");
    assert_eq!(json["server"]["REQUEST_URI"], "/api/data?debug=1");
    assert_eq!(json["server"]["QUERY_STRING"], "debug=1");
    assert_eq!(json["server"]["SERVER_SOFTWARE"], "RestPHP/0.1.0");
}

#[test]
fn test_tier1_lifecycle_consecutive_requests() {
    let server = TestServer::start("tests/fixtures/lifecycle.php").expect("Server should start");
    for i in 0..10 {
        let path = format!("/lifecycle?req_id={}", i);
        let resp = send_http_request(server.port, "GET", &path, &[], None)
            .expect("Sequential request should succeed");
        assert_eq!(resp.status_code, 200);
    }
}

#[test]
fn test_tier1_max_requests_drains_then_exits_for_supervisor_recycle() {
    for budget in [1_u64, 2, 3] {
        let mut server =
            TestServer::start_with_max_requests("tests/fixtures/lifecycle.php", budget)
                .expect("Server should start");

        for request_id in 0..budget {
            let path = format!("/lifecycle?req_id={request_id}");
            let response = send_http_request(server.port, "GET", &path, &[], None)
                .expect("Accepted request should complete before the process drains");
            assert_eq!(response.status_code, 200);
        }

        server
            .wait_for_exit_success(Duration::from_secs(5))
            .expect("Request budget should cause a clean supervisor-restart exit");
    }
}

#[test]
fn test_tier1_readiness_connection_does_not_consume_max_requests_budget() {
    let mut server = TestServer::start_with_max_requests("tests/fixtures/lifecycle.php", 1)
        .expect("Server should start");

    let response = send_http_request(server.port, "GET", "/lifecycle?req_id=real", &[], None)
        .expect("The first HTTP request, not the readiness TCP connection, must be accepted");
    assert_eq!(response.status_code, 200);
    server
        .wait_for_exit_success(Duration::from_secs(5))
        .expect("The one real request should exhaust the budget exactly once");
}

#[test]
fn test_tier1_lifecycle_query_isolation() {
    let server = TestServer::start("tests/fixtures/lifecycle.php").expect("Server should start");
    let _ = send_http_request(server.port, "GET", "/lifecycle?key=first_leak", &[], None).unwrap();
    let resp2 =
        send_http_request(server.port, "GET", "/lifecycle?new_key=second", &[], None).unwrap();
    assert_eq!(resp2.status_code, 200);
    let json2 = resp2.json().unwrap();
    assert!(json2["current_query"].get("key").is_none());
    assert_eq!(json2["current_query"]["new_key"], "second");
}

#[test]
fn test_tier1_lifecycle_alternating_methods() {
    let server = TestServer::start("tests/fixtures/info.php").expect("Server should start");
    for method in ["GET", "POST", "PUT", "DELETE", "GET"] {
        let resp = send_http_request(server.port, method, "/path", &[], None).unwrap();
        assert_eq!(resp.status_code, 200);
        let json = resp.json().unwrap();
        assert_eq!(json["server"]["REQUEST_METHOD"], method);
    }
}

// =========================================================================
// TIER 2: BOUNDARY & CORNER CASES
// =========================================================================

#[test]
fn test_tier2_boundary_empty_query() {
    let server = TestServer::start("tests/fixtures/info.php").expect("Server should start");
    let resp = send_http_request(server.port, "GET", "/path?", &[], None).unwrap();
    assert_eq!(resp.status_code, 200);
    let json = resp.json().unwrap();
    assert_eq!(json["query_string"], "");
}

#[test]
fn test_tier2_boundary_special_chars_query() {
    let server = TestServer::start("tests/fixtures/info.php").expect("Server should start");
    let resp = send_http_request(
        server.port,
        "GET",
        "/path?msg=Hello+World%21&math=1%2B1%3D2",
        &[],
        None,
    )
    .unwrap();
    assert_eq!(resp.status_code, 200);
    let json = resp.json().unwrap();
    assert_eq!(json["get"]["msg"], "Hello World!");
    assert_eq!(json["get"]["math"], "1+1=2");
}

#[test]
fn test_tier2_boundary_missing_cookie_no_crash() {
    let server = TestServer::start("tests/fixtures/info.php").expect("Server should start");
    let resp = send_http_request(server.port, "GET", "/nocookie", &[], None).unwrap();
    assert_eq!(resp.status_code, 200);
}

#[test]
fn test_tier2_boundary_custom_methods() {
    let server =
        TestServer::start("tests/fixtures/status_and_headers.php").expect("Server should start");
    for method in ["PATCH", "OPTIONS"] {
        let resp = send_http_request(server.port, method, "/status", &[], None).unwrap();
        assert_eq!(resp.status_code, 200);
    }
}

#[test]
fn test_tier2_body_limit_rejects_before_php_execution() {
    let server = TestServer::start_with_args(
        "tests/fixtures/info.php",
        vec![
            "--max-body-bytes".to_string(),
            "16".to_string(),
            "--max-requests".to_string(),
            "0".to_string(),
        ],
    )
    .expect("Server should start with a small test body limit");

    let response = send_http_request(
        server.port,
        "POST",
        "/too-large",
        &[("Content-Type", "application/octet-stream")],
        Some(b"0123456789abcdefx"),
    )
    .expect("Oversized request should receive an HTTP response");
    assert_eq!(response.status_code, 413);
}

#[test]
fn test_tier2_cgi_request_uri_keeps_query_and_host_does_not_become_server_name() {
    let server = TestServer::start("tests/fixtures/info.php").expect("Server should start");
    let response = send_http_request(
        server.port,
        "GET",
        "/cgi/path?item=one",
        &[("Host", "attacker.example:4242")],
        None,
    )
    .expect("Request should succeed");
    assert_eq!(response.status_code, 200);
    let json = response.json().expect("Body should be JSON");
    assert_eq!(json["server"]["REQUEST_URI"], "/cgi/path?item=one");
    assert_eq!(json["server"]["HTTP_HOST"], "attacker.example:4242");
    assert_ne!(json["server"]["SERVER_NAME"], "attacker.example:4242");
}

// =========================================================================
// TIER 3: CROSS-FEATURE COMBINATIONS
// =========================================================================

#[test]
fn test_tier3_combo_rapid_alternating_payloads() {
    let server = TestServer::start("tests/fixtures/info.php").expect("Server should start");
    let r1 = send_http_request(
        server.port,
        "POST",
        "/step1",
        &[("Content-Type", "application/json")],
        Some(b"{\"a\":1}"),
    )
    .unwrap();
    let r2 = send_http_request(server.port, "GET", "/step2", &[], None).unwrap();
    let r3 = send_http_request(
        server.port,
        "POST",
        "/step3",
        &[("Content-Type", "application/x-www-form-urlencoded")],
        Some(b"key=val"),
    )
    .unwrap();
    assert_eq!(r1.status_code, 200);
    assert_eq!(r2.status_code, 200);
    assert_eq!(r3.status_code, 200);
}

#[test]
fn test_tier3_php_response_filters_hop_by_hop_headers_but_keeps_cookies() {
    let server =
        TestServer::start("tests/fixtures/response_safety.php").expect("Server should start");
    let response = send_http_request(server.port, "GET", "/headers", &[], None)
        .expect("Request should succeed");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.text(), "safe");
    assert_eq!(response.header("x-allowed"), Some("retained"));
    // Hyper owns HTTP connection management and may itself emit
    // `Connection: close` for this client's close-delimited request. The PHP
    // supplied `keep-alive` value must still never escape the safety boundary.
    assert_ne!(response.header("connection"), Some("keep-alive"));
    // The HTTP framework calculates the real body length itself; PHP's forged
    // value must not be forwarded.
    assert_ne!(response.header("content-length"), Some("1"));
    for forbidden in [
        "keep-alive",
        "proxy-test",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
    ] {
        assert!(
            response.header(forbidden).is_none(),
            "PHP must not forward hop-by-hop header {forbidden}"
        );
    }
    assert_eq!(response.header("server"), Some("RestPHP/0.1.0"));
    let cookies: Vec<_> = response
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, value)| value.as_str())
        .collect();
    assert_eq!(cookies, vec!["first=1", "second=2"]);
}

#[test]
fn test_tier3_invalid_php_status_becomes_generic_internal_server_error() {
    let server =
        TestServer::start("tests/fixtures/status_and_headers.php").expect("Server should start");
    let response = send_http_request(server.port, "GET", "/status?code=999", &[], None)
        .expect("Request should receive an HTTP response");
    assert_eq!(response.status_code, 500);
    assert!(
        !response.text().contains("Worker-"),
        "PHP/runtime implementation details must not be exposed to clients"
    );
}

// =========================================================================
// TIER 4: REAL-WORLD SCENARIOS
// =========================================================================

#[test]
fn test_tier4_concurrency_stress_100_requests() {
    let server = Arc::new(TestServer::start("public/index.php").expect("Server should start"));
    let mut handles = Vec::new();

    for thread_idx in 0..10 {
        let srv = Arc::clone(&server);
        handles.push(std::thread::spawn(move || {
            for req_idx in 0..10 {
                let path = format!("/?thread={}&req={}", thread_idx, req_idx);
                let resp = send_http_request(srv.port, "GET", &path, &[], None)
                    .expect("Concurrent request should succeed");
                assert_eq!(resp.status_code, 200);
            }
        }));
    }

    for handle in handles {
        handle.join().expect("Thread should finish cleanly");
    }
}

#[test]
fn test_tier4_process_recycle_handles_100_requests_and_binary_output() {
    let mut server = TestServer::start_with_max_requests("tests/fixtures/response_safety.php", 100)
        .expect("Server should start");

    let binary = send_http_request(server.port, "GET", "/binary?mode=binary", &[], None)
        .expect("Binary response should succeed");
    assert_eq!(binary.status_code, 200);
    assert_eq!(binary.body, b"binary\0payload");

    for request_id in 1..100 {
        let response = send_http_request(
            server.port,
            "GET",
            &format!("/stable?request_id={request_id}"),
            &[],
            None,
        )
        .expect("Every admitted request must complete before recycle");
        assert_eq!(response.status_code, 200);
        assert_eq!(response.text(), "safe");
    }

    server
        .wait_for_exit_success(Duration::from_secs(10))
        .expect("The 100-request process budget should exit cleanly for its supervisor");
}

#[test]
fn test_tier4_queue_saturation_returns_fast_retryable_overload() {
    let server = TestServer::start_with_args(
        "tests/fixtures/slow.php",
        vec![
            "--max-queue".to_string(),
            "2".to_string(),
            "--max-requests".to_string(),
            "0".to_string(),
        ],
    )
    .expect("Server should start with a two-job queue");
    let port = server.port;

    let active = std::thread::spawn(move || send_http_request(port, "GET", "/active", &[], None));
    std::thread::sleep(Duration::from_millis(100));
    let queued = std::thread::spawn(move || send_http_request(port, "GET", "/queued", &[], None));
    std::thread::sleep(Duration::from_millis(100));

    let started = Instant::now();
    let overloaded = send_http_request(server.port, "GET", "/overloaded", &[], None)
        .expect("Overloaded request should receive an HTTP response");
    assert!(
        started.elapsed() < Duration::from_millis(750),
        "Overload handling must not wait for PHP execution"
    );
    assert_eq!(overloaded.status_code, 503);
    assert_eq!(overloaded.header("retry-after"), Some("1"));

    assert_eq!(
        active
            .join()
            .expect("active client should join")
            .unwrap()
            .status_code,
        200
    );
    assert_eq!(
        queued
            .join()
            .expect("queued client should join")
            .unwrap()
            .status_code,
        200
    );
}

#[test]
fn test_tier4_disconnected_queued_client_does_not_execute_php() {
    let server = TestServer::start_with_args(
        "tests/fixtures/slow.php",
        vec![
            "--max-queue".to_string(),
            "2".to_string(),
            "--max-requests".to_string(),
            "0".to_string(),
        ],
    )
    .expect("Server should start");
    let marker = std::env::temp_dir().join(format!(
        "restphp-disconnected-client-{}-{}",
        std::process::id(),
        server.port
    ));
    assert!(!marker.exists());

    let port = server.port;
    let active = std::thread::spawn(move || send_http_request(port, "GET", "/active", &[], None));
    std::thread::sleep(Duration::from_millis(100));

    let marker_query = marker.to_string_lossy().replace('/', "%2F");
    let mut cancelled =
        TcpStream::connect(("127.0.0.1", server.port)).expect("Cancelled client should connect");
    cancelled
        .write_all(
            format!(
                "GET /cancelled?marker={marker_query} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        )
        .expect("Cancelled client should send its request");
    drop(cancelled);

    assert_eq!(
        active
            .join()
            .expect("active client should join")
            .unwrap()
            .status_code,
        200
    );
    std::thread::sleep(Duration::from_millis(250));
    let executed = marker.exists();
    let _ = std::fs::remove_file(&marker);
    assert!(
        !executed,
        "a queued request whose client disconnected must not execute PHP"
    );
}

#[cfg(unix)]
#[test]
fn test_tier4_sigterm_closes_admission_drains_and_exits_successfully() {
    let mut server = TestServer::start_with_args(
        "tests/fixtures/slow.php",
        vec![
            "--max-queue".to_string(),
            "2".to_string(),
            "--max-requests".to_string(),
            "0".to_string(),
        ],
    )
    .expect("Server should start");
    let port = server.port;
    let active = std::thread::spawn(move || send_http_request(port, "GET", "/active", &[], None));
    std::thread::sleep(Duration::from_millis(100));
    let queued = std::thread::spawn(move || send_http_request(port, "GET", "/queued", &[], None));
    std::thread::sleep(Duration::from_millis(100));

    let pid = server.child.id().to_string();
    let signal_status = Command::new("/bin/kill")
        .args(["-TERM", &pid])
        .status()
        .expect("SIGTERM command should execute");
    assert!(signal_status.success());

    assert_eq!(
        active
            .join()
            .expect("active client should join")
            .unwrap()
            .status_code,
        200
    );
    assert_eq!(
        queued
            .join()
            .expect("queued client should join")
            .unwrap()
            .status_code,
        200
    );
    server
        .wait_for_exit_success(Duration::from_secs(10))
        .expect("SIGTERM should produce a clean drained shutdown");
}

#[test]
fn test_tier4_error_resilience() {
    let server = TestServer::start("tests/fixtures/error.php").expect("Server should start");
    // Trigger PHP notice
    let r1 = send_http_request(server.port, "GET", "/error?mode=notice", &[], None).unwrap();
    assert_eq!(r1.status_code, 200);
    // Ensure subsequent normal request succeeds
    let r2 = send_http_request(server.port, "GET", "/error?mode=ok", &[], None).unwrap();
    assert_eq!(r2.status_code, 200);
    let json2 = r2.json().unwrap();
    assert_eq!(json2["status"], "ok");
}

#[test]
fn test_tier4_php_fatal_does_not_poison_next_request() {
    let server = TestServer::start("tests/fixtures/error.php").expect("Server should start");
    let fatal = send_http_request(server.port, "GET", "/error?mode=user_error", &[], None)
        .expect("Fatal PHP request should be converted to an HTTP response");
    assert_eq!(fatal.status_code, 500);

    let recovery = send_http_request(server.port, "GET", "/error?mode=ok", &[], None)
        .expect("Worker must survive a PHP fatal");
    assert_eq!(recovery.status_code, 200);
    assert_eq!(recovery.json().unwrap()["status"], "ok");
}
