// tests/e2e_test_suite.rs
// Comprehensive Rust Integration Test Suite for RestPHP

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

static NEXT_PORT: AtomicU16 = AtomicU16::new(9100);

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

#[test]
fn test_harness_sanity() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn test_tier1_ac1_startup() {
    let server = TestServer::start("public/index.php").expect("Server should start");
    let resp =
        send_http_request(server.port, "GET", "/", &[], None).expect("HTTP request should succeed");
    assert_eq!(resp.status_code, 200);
    let json = resp.json().expect("Response should be valid JSON");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["engine"], "RestPHP");
}
