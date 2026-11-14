use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Server {
    child: Child,
    directory: PathBuf,
    port: u16,
}

impl Server {
    fn start(script: &str, max_requests: u64) -> Self {
        let socket = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();
        drop(socket);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("restphp-bridge-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("worker.php"), script).unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_restphp"))
            .current_dir(&directory)
            .args([
                "serve",
                "--persistent-worker",
                "worker.php",
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--max-requests",
                &max_requests.to_string(),
                "--shutdown-timeout-secs",
                "3",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut server = Self {
            child,
            directory,
            port,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                break;
            }
            assert!(
                server.child.try_wait().unwrap().is_none(),
                "persistent worker failed to boot"
            );
            assert!(
                Instant::now() < deadline,
                "persistent bootstrap never became ready"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        server
    }

    fn wait_exit(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.child.try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "persistent worker did not stop");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn request(port: u16, method: &str, path: &str, headers: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    write!(stream, "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n{body}", body.len()).unwrap();
    let mut result = String::new();
    stream.read_to_string(&mut result).unwrap();
    result
}

#[test]
fn boot_once_isolates_http_state_and_recycles_after_last_response() {
    let mut server = Server::start(
        r#"<?php
file_put_contents(__DIR__.'/boot-count', 'boot\n', FILE_APPEND);
echo 'bootstrap-output-must-not-leak';
$counter = 0;
$_ENV['BRIDGE_ENV'] = $_SERVER['BRIDGE_ENV'] = 'boot';
$held = null;
while (restphp_handle_request(function () use (&$counter, &$held) {
    $counter++;
    $seen = isset($_SERVER['LEAK']) || isset($_GET['leak']) || isset($_POST['leak']) || isset($_COOKIE['leak']) || isset($_REQUEST['leak']) || isset($_FILES['leak']);
    $payload = ['counter' => $counter, 'method' => $_SERVER['REQUEST_METHOD'], 'query' => $_GET, 'post' => $_POST, 'cookie' => $_COOKIE, 'input' => file_get_contents('php://input'), 'seen' => $seen, 'header' => $_SERVER['HTTP_X_BRIDGE'] ?? null];
    $payload['environment'] = $_ENV['BRIDGE_ENV'] ?? null;
    $payload['server_environment'] = $_SERVER['BRIDGE_ENV'] ?? null;
    $payload['held'] = is_resource($held) ? stream_get_contents($held) : null;
    $payload['filtered'] = function_exists('filter_input') ? filter_input(INPUT_GET, 'a') : null;
    if (function_exists('session_start')) {
        session_save_path(__DIR__);
        session_cache_limiter('');
        session_start();
        $payload['session_id'] = session_id();
        $payload['session_seen'] = isset($_SESSION['secret']);
        $_SESSION['secret'] = true;
    }
    if ($counter === 1) { http_response_code(201); header('X-First: yes'); }
    header('Content-Type: application/json');
    ob_start(); echo json_encode($payload);
    $held = fopen('php://input', 'rb');
    $_SERVER['LEAK'] = $_GET['leak'] = $_POST['leak'] = $_COOKIE['leak'] = $_REQUEST['leak'] = $_FILES['leak'] = true;
    $_ENV['BRIDGE_ENV'] = $_SERVER['BRIDGE_ENV'] = 'request mutation';
})) {}
echo 'shutdown-output-must-not-leak';
"#,
        2,
    );
    let first = request(
        server.port,
        "POST",
        "/one?a=1",
        "Content-Type: application/x-www-form-urlencoded\r\nCookie: first=yes\r\nX-Bridge: one\r\n",
        "value=one",
    );
    assert!(first.starts_with("HTTP/1.1 201"), "{first}");
    assert!(first.to_ascii_lowercase().contains("x-first: yes"));
    let first: serde_json::Value =
        serde_json::from_str(first.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(first["counter"], 1);
    assert_eq!(first["input"], "value=one");
    assert_eq!(first["post"]["value"], "one");
    assert_eq!(first["cookie"]["first"], "yes");
    assert_eq!(first["seen"], false);
    let second = request(server.port, "GET", "/two?b=2", "", "");
    assert!(second.starts_with("HTTP/1.1 200"), "{second}");
    assert!(!second.to_ascii_lowercase().contains("x-first:"));
    let second: serde_json::Value =
        serde_json::from_str(second.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(second["counter"], 2);
    assert_eq!(second["method"], "GET");
    assert_eq!(second["input"], "");
    assert_eq!(second["held"], "value=one");
    assert_eq!(second["environment"], "boot");
    assert_eq!(second["server_environment"], "boot");
    assert!(second["filtered"].is_null());
    if first.get("session_id").is_some() {
        assert_ne!(first["session_id"], second["session_id"]);
        assert_eq!(first["session_seen"], false);
        assert_eq!(second["session_seen"], false);
    }
    assert_eq!(second["query"]["b"], "2");
    assert_eq!(second["post"], serde_json::json!([]));
    assert_eq!(second["cookie"], serde_json::json!([]));
    assert_eq!(second["seen"], false);
    assert!(second["header"].is_null());
    server.wait_exit();
    assert_eq!(
        std::fs::read_to_string(server.directory.join("boot-count")).unwrap(),
        "boot\\n"
    );
}

#[test]
fn persistent_worker_resets_execution_timeout_for_each_request_and_idle_wait() {
    let mut server = Server::start(
        "<?php set_time_limit(1); while (restphp_handle_request(function () { usleep(600000); echo 'ok'; })) {}",
        3,
    );
    for _ in 0..3 {
        let response = request(server.port, "GET", "/", "", "");
        assert!(
            response.starts_with("HTTP/1.1 200") && response.ends_with("ok"),
            "{response}"
        );
    }
    server.wait_exit();
}

#[cfg(unix)]
#[test]
fn shutdown_wakes_idle_native_callback_and_runs_outer_finally() {
    let mut server = Server::start("<?php try { while (restphp_handle_request(function () { echo 'ok'; })) {} } finally { file_put_contents(__DIR__.'/terminated', 'yes'); }", 0);
    assert!(request(server.port, "GET", "/", "", "").ends_with("ok"));
    assert!(Command::new("/bin/kill")
        .args(["-TERM", &server.child.id().to_string()])
        .status()
        .unwrap()
        .success());
    server.wait_exit();
    assert_eq!(
        std::fs::read_to_string(server.directory.join("terminated")).unwrap(),
        "yes"
    );
}

#[test]
fn callback_exit_and_uncaught_exception_stop_unhealthy_process() {
    for failure in ["exit(0);", "throw new RuntimeException('private failure');"] {
        let mut server = Server::start(&format!("<?php while (restphp_handle_request(function () {{ echo 'partial-private-output'; {failure} }})) {{}}"), 0);
        let response = request(server.port, "GET", "/", "", "");
        assert!(response.starts_with("HTTP/1.1 500"), "{response}");
        assert!(!response.contains("partial-private-output"));
        assert!(!response.contains("private failure"));
        server.wait_exit();
    }
}

#[test]
fn bootstrap_return_without_callback_fails_before_listener_bind() {
    let socket = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    drop(socket);
    let script = std::env::temp_dir().join(format!(
        "restphp-no-callback-{}-{port}.php",
        std::process::id()
    ));
    std::fs::write(&script, "<?php echo 'not a worker';").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_restphp"))
        .args([
            "serve",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--persistent-worker",
            script.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let _ = std::fs::remove_file(script);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("before entering restphp_handle_request")
    );
    assert!(TcpStream::connect(("127.0.0.1", port)).is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn fatal_callback_settles_every_already_queued_response_channel() {
    let script =
        std::env::temp_dir().join(format!("restphp-queued-fatal-{}.php", std::process::id()));
    std::fs::write(
        &script,
        "<?php while (restphp_handle_request(function () { usleep(200000); exit(0); })) {}",
    )
    .unwrap();
    let mut worker = restphp::WorkerHandle::new_persistent(script.clone(), 0, 4).unwrap();
    match restphp::WorkerHandle::new() {
        Ok(_) => panic!("a process must not host two simultaneous NTS VMs"),
        Err(error) => assert!(error.contains("Only one NTS PHP VM")),
    }
    let target = restphp::ExecutionTarget::File(script.clone());
    let (first, second, third) = tokio::join!(
        worker.dispatch(
            target.clone(),
            "GET".into(),
            "/first".into(),
            "".into(),
            vec![]
        ),
        worker.dispatch(
            target.clone(),
            "GET".into(),
            "/second".into(),
            "".into(),
            vec![]
        ),
        worker.dispatch(target, "GET".into(), "/third".into(), "".into(), vec![]),
    );
    for response in [first, second, third] {
        let response = response.expect("accepted requests must settle even after worker failure");
        assert_eq!(response.status, 500);
        assert!(!response.success);
    }
    worker.shutdown();
    std::fs::remove_file(script).unwrap();
}
