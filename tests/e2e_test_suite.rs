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

        let child = Command::new(&final_bin)
            .args([
                "serve",
                "--port",
                &port.to_string(),
                "--entrypoint",
                entrypoint,
            ])
            .spawn()?;

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
            if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", self.port)) {
                let _ = stream
                    .write_all(b"HEAD / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
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

    let mut req = format!(
        "{} {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n",
        method, path, port
    );
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
    assert_eq!(json["server"]["REQUEST_URI"], "/api/data");
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
