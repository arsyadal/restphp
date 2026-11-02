#![cfg(all(unix, feature = "hot-reload"))]

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct WatchedServer {
    child: Child,
    root: std::path::PathBuf,
}

impl Drop for WatchedServer {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_some() {
            let _ = std::fs::remove_dir_all(&self.root);
            return;
        }
        let _ = Command::new("/bin/kill")
            .args(["-TERM", &self.child.id().to_string()])
            .status();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if self.child.try_wait().ok().flatten().is_some() {
                break;
            }
            if Instant::now() > deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn response(port: u16, path: &str) -> Option<String> {
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().ok()?,
        Duration::from_millis(100),
    )
    .ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
}

fn wait_response(port: u16, version: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(body) = response(port, "/") {
            if body.starts_with(version) {
                return body;
            }
        }
        assert!(
            Instant::now() < deadline,
            "server did not serve version {version}"
        );
        std::thread::sleep(Duration::from_millis(30));
    }
}

#[test]
fn watch_drains_inflight_request_then_replaces_process_and_stops_child() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let root = std::env::temp_dir().join(format!("restphp-watch-{}-{port}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let script = root.join("index.php");
    std::fs::write(&script, "<?php if (isset($_GET['slow'])) { file_put_contents(__DIR__.'/started', '1'); usleep(900000); } echo 'old:'.getmypid();").unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_restphp"))
        .current_dir(&root)
        .args([
            "serve",
            "--entrypoint",
            "index.php",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--watch",
            "--shutdown-timeout-secs",
            "3",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut server = WatchedServer { child, root };
    let old = wait_response(port, "old:");
    let slow = std::thread::spawn(move || response(port, "/?slow=1"));
    let deadline = Instant::now() + Duration::from_secs(3);
    while !server.root.join("started").exists() {
        assert!(Instant::now() < deadline, "slow request never started");
        std::thread::sleep(Duration::from_millis(10));
    }
    std::fs::write(script, "<?php echo 'new:'.getmypid();").unwrap();
    assert_eq!(
        slow.join().unwrap().unwrap(),
        old,
        "reload must drain accepted work"
    );
    let new = wait_response(port, "new:");
    assert_ne!(
        old.strip_prefix("old:"),
        new.strip_prefix("new:"),
        "reload must use a new Zend process"
    );
    assert!(Command::new("/bin/kill")
        .args(["-TERM", &server.child.id().to_string()])
        .status()
        .unwrap()
        .success());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = server.child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < deadline, "watch supervisor did not stop");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        TcpStream::connect(("127.0.0.1", port)).is_err(),
        "watch child listener must close on supervisor shutdown"
    );
}
