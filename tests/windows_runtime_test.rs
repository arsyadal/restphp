#![cfg(windows)]

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Once,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::System::{
    Console::{AllocConsole, GenerateConsoleCtrlEvent, GetConsoleCP, CTRL_BREAK_EVENT},
    Threading::CREATE_NEW_PROCESS_GROUP,
};

struct Server {
    child: Child,
    directory: PathBuf,
    port: u16,
}

impl Server {
    fn start(script: &str, watch: bool) -> Self {
        static CONSOLE: Once = Once::new();
        CONSOLE.call_once(|| unsafe {
            if GetConsoleCP() == 0 {
                assert_ne!(AllocConsole(), 0, "test needs a console for CTRL_BREAK");
            }
        });
        let socket = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();
        drop(socket);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "restphp Windows café-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("index.php"), script).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_restphp"));
        command
            .current_dir(&directory)
            .args([
                "serve",
                "--entrypoint",
                "index.php",
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--shutdown-timeout-secs",
                "5",
            ])
            .creation_flags(CREATE_NEW_PROCESS_GROUP)
            .stdout(Stdio::null());
        if watch {
            command.args(["--watch", "--persistent-worker", "index.php"]);
        }
        let child = command.spawn().unwrap();
        let mut server = Self {
            child,
            directory,
            port,
        };
        wait_until(|| {
            assert!(
                server.child.try_wait().unwrap().is_none(),
                "Windows server exited before readiness"
            );
            TcpStream::connect(("127.0.0.1", port)).is_ok()
        });
        server
    }

    fn stop(&mut self) {
        assert_ne!(
            unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, self.child.id()) },
            0
        );
    }

    fn wait_exit(&mut self) {
        wait_until(|| match self.child.try_wait().unwrap() {
            Some(status) => {
                assert!(status.success(), "{status}");
                true
            }
            None => false,
        });
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn wait_until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !ready() {
        assert!(
            Instant::now() < deadline,
            "Windows lifecycle operation timed out"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn request(port: u16) -> std::io::Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

#[test]
fn ctrl_break_drains_request_and_runs_php_shutdown_in_unicode_path() {
    let mut server = Server::start(
        r#"<?php
register_shutdown_function(function () { file_put_contents(__DIR__.'/stopped', 'yes'); });
file_put_contents(__DIR__.'/entered', 'yes');
while (!file_exists(__DIR__.'/release')) { clearstatcache(); usleep(10000); }
echo 'drained';
"#,
        false,
    );
    let port = server.port;
    let response = std::thread::spawn(move || request(port).unwrap());
    wait_until(|| server.directory.join("entered").exists());
    server.stop();
    std::fs::write(server.directory.join("release"), "yes").unwrap();
    let response = response.join().unwrap();
    assert!(
        response.starts_with("HTTP/1.1 200") && response.ends_with("drained"),
        "{response}"
    );
    server.wait_exit();
    assert_eq!(
        std::fs::read_to_string(server.directory.join("stopped")).unwrap(),
        "yes"
    );
}

#[cfg(feature = "hot-reload")]
#[test]
fn watch_drains_old_worker_before_serving_changed_php() {
    let source = |body: &str| {
        format!("<?php register_shutdown_function(function () {{ file_put_contents(__DIR__.'/stopped', 'done\\n', FILE_APPEND); }}); while (restphp_handle_request(function () {{ echo '{body}'; }})) {{}}")
    };
    let mut server = Server::start(&source("first"), true);
    assert!(request(server.port).unwrap().ends_with("first"));
    std::fs::write(server.directory.join("index.php"), source("second")).unwrap();
    wait_until(|| request(server.port).is_ok_and(|response| response.ends_with("second")));
    let shutdowns = std::fs::read_to_string(server.directory.join("stopped")).unwrap();
    assert_eq!(
        shutdowns.matches("done").count(),
        1,
        "old VM must run PHP shutdown before restart"
    );
    server.stop();
    server.wait_exit();
    let shutdowns = std::fs::read_to_string(server.directory.join("stopped")).unwrap();
    assert_eq!(
        shutdowns.matches("done").count(),
        2,
        "watcher must drain its final child"
    );
    assert!(
        TcpStream::connect(("127.0.0.1", server.port)).is_err(),
        "watcher left its child running"
    );
}
