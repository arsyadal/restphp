#![cfg(unix)]

use std::{
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn start(extra: &[String]) -> (Server, u16) {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let child = Command::new(env!("CARGO_BIN_EXE_restphp"))
        .args([
            "serve",
            "--host",
            "127.0.0.1",
            "--entrypoint",
            "tests/fixtures/info.php",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .args(extra)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut server = Server(child);
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        assert!(
            server.0.try_wait().unwrap().is_none(),
            "server exited during startup"
        );
        assert!(Instant::now() < deadline, "server never bound listener");
        std::thread::sleep(Duration::from_millis(20));
    }
    (server, port)
}

fn wait_success(server: &mut Server) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = server.0.try_wait().unwrap() {
            assert!(status.success(), "request-limit drain failed: {status}");
            return;
        }
        assert!(
            Instant::now() < deadline,
            "server did not drain after request limit"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn tls_metadata_comes_from_listener_not_forwarded_headers() {
    let directory = std::env::temp_dir().join(format!("restphp-tls-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let certificate = directory.join("cert.pem");
    let key = directory.join("key.pem");
    let output = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-days",
            "1",
            "-subj",
            "/CN=localhost",
            "-addext",
            "subjectAltName=DNS:localhost",
            "-out",
        ])
        .arg(&certificate)
        .arg("-keyout")
        .arg(&key)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let (mut secure, secure_port) = start(&[
        "--tls-cert".into(),
        certificate.to_string_lossy().into_owned(),
        "--tls-key".into(),
        key.to_string_lossy().into_owned(),
    ]);
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--max-time",
            "5",
            "--noproxy",
            "*",
            "--cacert",
        ])
        .arg(&certificate)
        .args([
            "--resolve",
            &format!("localhost:{secure_port}:127.0.0.1"),
            "-H",
            "X-Forwarded-Proto: http",
            "-H",
            "X-Forwarded-Port: 80",
            &format!("https://localhost:{secure_port}/?secure=yes"),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let secure_response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(secure_response["server"]["HTTPS"], "on");
    assert_eq!(secure_response["server"]["REQUEST_SCHEME"], "https");
    assert_eq!(
        secure_response["server"]["SERVER_PORT"],
        secure_port.to_string()
    );
    assert_eq!(secure_response["get"]["secure"], "yes");
    wait_success(&mut secure);

    let (mut plain, plain_port) = start(&[]);
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--max-time",
            "5",
            "--noproxy",
            "*",
            "-H",
            "X-Forwarded-Proto: https",
            "-H",
            "X-Forwarded-Port: 443",
            &format!("http://127.0.0.1:{plain_port}/"),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plain_response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(plain_response["server"]["HTTPS"], "off");
    assert_eq!(plain_response["server"]["REQUEST_SCHEME"], "http");
    assert_eq!(
        plain_response["server"]["SERVER_PORT"],
        plain_port.to_string()
    );
    wait_success(&mut plain);
    std::fs::remove_dir_all(directory).unwrap();
}
