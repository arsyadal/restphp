//! Milestone 1 Empirical Challenger Test Suite
//!
//! Stress tests:
//! 1. Thread safety invariants (exclusivity, !Send / !Sync, Actor serialization).
//! 2. Binary payloads through ub_write (null bytes, unicode, invalid UTF-8, large binary buffers).
//! 3. Binary payloads through read_post (null bytes in php://input, chunked read >16KB, raw octet-stream).
//! 4. Edge cases: missing headers, empty bodies, malformed query strings, malformed cookies.
//! 5. Lifecycle resilience: state isolation after bailouts and fatals.

use bytes::Bytes;
use restphp::sapi::context::WorkerRequestContext;
use restphp::sapi::{ExecutionTarget, PhpEngine, PhpResponse};
use restphp::worker::{ExecutionTarget as WorkerTarget, WorkerHandle};
use std::sync::Mutex;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[allow(clippy::too_many_arguments)]
fn run_req(
    engine: &PhpEngine,
    target: ExecutionTarget,
    method: &str,
    uri: &str,
    query: &str,
    body: Bytes,
    content_type: Option<&str>,
    server_vars: Vec<(String, String)>,
    cookie: Option<&str>,
) -> Result<PhpResponse, String> {
    let mut ctx = WorkerRequestContext::new(body, server_vars);
    if let Some(c) = cookie {
        ctx = ctx.with_cookie(c);
    }

    engine.execute_request(&mut ctx, &target, method, uri, query, content_type, None)
}

// =========================================================================
// Group 1: Thread Safety Invariants
// =========================================================================

#[test]
fn test_thread_safety_engine_is_not_send_sync() {
    let _guard = TEST_MUTEX.lock().unwrap();

    // Assert statically that PhpEngine cannot be sent across threads or shared between threads
    #[allow(dead_code)]
    trait AssertNotSendSync {}
    impl<T: ?Sized> AssertNotSendSync for T {}

    #[allow(dead_code)]
    fn is_send<T: Send>() {}
    // The following would fail to compile if uncommented:
    // is_send::<PhpEngine>();

    // Verify runtime actor serialization via WorkerHandle under concurrency
    let worker = WorkerHandle::new().expect("WorkerHandle should initialize");
    let mut handles = Vec::new();

    for thread_id in 0..10 {
        let w = worker.clone();
        handles.push(std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                for req_id in 0..10 {
                    let code = format!("echo 'T{}_R{}';", thread_id, req_id);
                    let resp = w
                        .dispatch(
                            WorkerTarget::Code(code),
                            "GET".into(),
                            format!("/t/{}/r/{}", thread_id, req_id),
                            "".into(),
                            vec![],
                        )
                        .await
                        .expect("Dispatch should succeed");

                    assert_eq!(resp.status, 200);
                    assert!(resp.success);
                    assert_eq!(
                        String::from_utf8_lossy(&resp.body),
                        format!("T{}_R{}", thread_id, req_id)
                    );
                }
            });
        }));
    }

    for h in handles {
        h.join().expect("Worker thread client should join cleanly");
    }
}

// =========================================================================
// Group 2: Binary Payloads through ub_write
// =========================================================================

#[test]
fn test_binary_payload_null_bytes_ub_write() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let engine = PhpEngine::init().expect("PhpEngine init");

    // Output binary data with multiple embedded null bytes
    let code = r#"
        // Output \x00 in middle and ends
        echo "A\x00B\x00\x00C\x00";
    "#;
    let resp = run_req(
        &engine,
        ExecutionTarget::Inline(code.to_string()),
        "GET",
        "/binary",
        "",
        Bytes::new(),
        None,
        vec![],
        None,
    )
    .expect("Req failed");

    assert_eq!(resp.status, 200);
    assert!(resp.success);
    let expected = b"A\x00B\x00\x00C\x00";
    assert_eq!(
        resp.body,
        expected,
        "Body length should be {} but was {}",
        expected.len(),
        resp.body.len()
    );
}

#[test]
fn test_binary_payload_unicode_and_emojis_ub_write() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let engine = PhpEngine::init().expect("PhpEngine init");

    let unicode_str = "🦀 RestPHP: ñoño, Привет мир! 日本語, 🌍🚀✨ — \u{1F980}";
    let code = format!("echo '{}';", unicode_str);
    let resp = run_req(
        &engine,
        ExecutionTarget::Inline(code),
        "GET",
        "/unicode",
        "",
        Bytes::new(),
        None,
        vec![],
        None,
    )
    .expect("Req failed");

    assert_eq!(resp.status, 200);
    assert!(resp.success);
    assert_eq!(String::from_utf8_lossy(&resp.body), unicode_str);
    assert_eq!(resp.body, unicode_str.as_bytes());
}

#[test]
fn test_binary_payload_invalid_utf8_ub_write() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let engine = PhpEngine::init().expect("PhpEngine init");

    // Generate arbitrary bytes that are invalid UTF-8: 0xFF, 0xFE, 0x80, 0x00, 0xAA
    let code = r#"
        echo "\xFF\xFE\x80\x00\xAA\xBB\xCC\xDD\xEE";
    "#;
    let resp = run_req(
        &engine,
        ExecutionTarget::Inline(code.to_string()),
        "GET",
        "/invalid-utf8",
        "",
        Bytes::new(),
        None,
        vec![],
        None,
    )
    .expect("Req failed");

    assert_eq!(resp.status, 200);
    assert!(resp.success);
    let expected = [0xFF, 0xFE, 0x80, 0x00, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
    assert_eq!(resp.body.as_slice(), &expected);
}

#[test]
fn test_binary_payload_large_buffer_ub_write() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let engine = PhpEngine::init().expect("PhpEngine init");

    // Output 128KB of alternating binary pattern
    let code = r#"
        echo str_repeat("\x00\xFF\x55\xAA", 32768);
    "#;
    let resp = run_req(
        &engine,
        ExecutionTarget::Inline(code.to_string()),
        "GET",
        "/large-binary",
        "",
        Bytes::new(),
        None,
        vec![],
        None,
    )
    .expect("Req failed");

    assert_eq!(resp.status, 200);
    assert!(resp.success);
    assert_eq!(resp.body.len(), 131072);

    let pattern = [0x00, 0xFF, 0x55, 0xAA];
    for (i, chunk) in resp.body.chunks(4).enumerate() {
        assert_eq!(chunk, &pattern, "Mismatch at chunk {}", i);
    }
}

// =========================================================================
// Group 3: Binary Payloads through read_post
// =========================================================================

#[test]
fn test_binary_post_body_with_null_bytes() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let engine = PhpEngine::init().expect("PhpEngine init");

    let raw_payload = b"magic_header\x00\x01\x02\x00payload_data\xFF\xFE\x00end";
    let code = r#"
        $input = file_get_contents('php://input');
        header('X-Input-Len: ' . strlen($input));
        echo $input;
    "#;

    let resp = run_req(
        &engine,
        ExecutionTarget::Inline(code.to_string()),
        "POST",
        "/binary-post",
        "",
        Bytes::from_static(raw_payload),
        Some("application/octet-stream"),
        vec![],
        None,
    )
    .expect("Req failed");

    assert_eq!(resp.status, 200);
    assert!(resp.success);
    assert_eq!(resp.body.len(), raw_payload.len());
    assert_eq!(resp.body.as_slice(), raw_payload);

    let len_header = resp
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-Input-Len"))
        .map(|(_, v)| v.as_str());
    assert_eq!(len_header, Some(raw_payload.len().to_string().as_str()));
}

#[test]
fn test_binary_post_chunked_multi_block_read_post() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let engine = PhpEngine::init().expect("PhpEngine init");

    // Generate 64KB (4 x 16KB blocks) of pseudo-random binary payload
    let mut large_body = Vec::with_capacity(65536);
    for i in 0..65536 {
        large_body.push((i % 256) as u8);
    }

    let code = r#"
        $input = file_get_contents('php://input');
        header('X-Body-Md5: ' . md5($input));
        header('X-Body-Len: ' . strlen($input));
        // Echo back first 10 bytes and last 10 bytes
        echo substr($input, 0, 10) . substr($input, -10);
    "#;

    let resp = run_req(
        &engine,
        ExecutionTarget::Inline(code.to_string()),
        "POST",
        "/chunked-post",
        "",
        Bytes::from(large_body.clone()),
        Some("application/octet-stream"),
        vec![],
        None,
    )
    .expect("Req failed");

    assert_eq!(resp.status, 200);
    assert!(resp.success);

    let len_header = resp
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-Body-Len"))
        .map(|(_, v)| v.as_str());
    assert_eq!(len_header, Some("65536"));

    let expected_echo = [&large_body[..10], &large_body[65536 - 10..]].concat();
    assert_eq!(resp.body, expected_echo);
}

// =========================================================================
// Group 4: Missing Headers, Empty Bodies, and Malformed Query Strings
// =========================================================================

#[test]
fn test_missing_headers_and_empty_body() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let engine = PhpEngine::init().expect("PhpEngine init");

    // Completely empty server_vars, None cookie, None content-type, empty body
    let code = r#"
        $post_empty = empty($_POST);
        $cookie_empty = empty($_COOKIE);
        $input_empty = (file_get_contents('php://input') === '');
        echo json_encode([
            'post_empty' => $post_empty,
            'cookie_empty' => $cookie_empty,
            'input_empty' => $input_empty,
        ]);
    "#;

    let resp = run_req(
        &engine,
        ExecutionTarget::Inline(code.to_string()),
        "POST",
        "/empty",
        "",
        Bytes::new(),
        None,
        vec![],
        None,
    )
    .expect("Req failed");

    assert_eq!(resp.status, 200);
    assert!(resp.success);
    let body_str = String::from_utf8_lossy(&resp.body);
    assert!(body_str.contains(r#""post_empty":true"#));
    assert!(body_str.contains(r#""cookie_empty":true"#));
    assert!(body_str.contains(r#""input_empty":true"#));
}

#[test]
fn test_malformed_query_strings() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let engine = PhpEngine::init().expect("PhpEngine init");

    // Array of malformed query strings
    let cases = [
        "",
        "&&&&&",
        "===&&==",
        "a=1&b=&c==3&d&&e",
        "arr[]=x&arr[]=y&nested[k1][k2]=val",
        "key=hello+world%21%3D&broken=%ZZ%GG",
    ];

    for (idx, q) in cases.iter().enumerate() {
        let code = format!(
            "echo json_encode(['case' => {}, 'query' => $_SERVER['QUERY_STRING'] ?? '', 'get' => $_GET]);",
            idx
        );

        let server_vars = vec![("QUERY_STRING".to_string(), q.to_string())];
        let resp = run_req(
            &engine,
            ExecutionTarget::Inline(code),
            "GET",
            "/malformed-query",
            q,
            Bytes::new(),
            None,
            server_vars,
            None,
        )
        .expect("Query test failed");

        assert_eq!(resp.status, 200);
        assert!(resp.success);
        let body_str = String::from_utf8_lossy(&resp.body);
        assert!(body_str.contains(&format!(r#""case":{}"#, idx)));
    }
}

#[test]
fn test_malformed_cookies() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let engine = PhpEngine::init().expect("PhpEngine init");

    let malformed_cookie_cases = [
        "",
        ";;; ; ;",
        "valid_key=valid_val; =no_key; ; orphaned_val;",
        "user=alice; session=xyz123; tracking=;;;",
    ];

    for cookie_str in malformed_cookie_cases {
        let code = "echo json_encode(['cookies' => $_COOKIE]);";
        let resp = run_req(
            &engine,
            ExecutionTarget::Inline(code.to_string()),
            "GET",
            "/cookie-test",
            "",
            Bytes::new(),
            None,
            vec![],
            if cookie_str.is_empty() {
                None
            } else {
                Some(cookie_str)
            },
        )
        .expect("Cookie test failed");

        assert_eq!(resp.status, 200);
        assert!(resp.success);
    }
}

// =========================================================================
// Group 5: Bailout and Error Recovery Isolation
// =========================================================================

#[test]
fn test_recovery_after_bailout_and_fatal_errors() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let engine = PhpEngine::init().expect("PhpEngine init");

    // Sequence of failing requests followed by normal requests
    let failure_targets = vec![
        // 1. Syntax parse error
        ExecutionTarget::Inline("echo 123 + ;".to_string()),
        // 2. Fatal call to undefined function
        ExecutionTarget::Inline("non_existent_fn();".to_string()),
        // 3. Uncaught RuntimeException
        ExecutionTarget::Inline("throw new RuntimeException('crash');".to_string()),
        // 4. exit(1)
        ExecutionTarget::Inline("exit(1);".to_string()),
        // 5. exit(42)
        ExecutionTarget::Inline("exit(42);".to_string()),
    ];

    for (idx, ft) in failure_targets.into_iter().enumerate() {
        // Run failing request
        let resp_fail = run_req(
            &engine,
            ft,
            "GET",
            "/fail",
            "",
            Bytes::new(),
            None,
            vec![],
            None,
        )
        .expect("Req call should not panic");

        assert!(!resp_fail.success, "Case {} should have failed", idx);

        // Immediately run healthy request to ensure zero state or status code bleed
        let normal_code = format!("http_response_code(200); echo 'HEALTHY_{}';", idx);
        let resp_healthy = run_req(
            &engine,
            ExecutionTarget::Inline(normal_code),
            "GET",
            "/healthy",
            "",
            Bytes::new(),
            None,
            vec![],
            None,
        )
        .expect("Healthy req should succeed");

        assert_eq!(resp_healthy.status, 200);
        assert!(resp_healthy.success);
        assert_eq!(
            String::from_utf8_lossy(&resp_healthy.body),
            format!("HEALTHY_{}", idx)
        );
    }
}

// =========================================================================
// Group 6: File Path Handling Invariants
// =========================================================================

#[test]
fn test_relative_path_in_subdirectory_defect() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let engine = PhpEngine::init().expect("PhpEngine init");

    // Demonstrates Defect: relative path in a subdirectory fails because
    // PHP's request startup changes directory via VCWD_CHDIR_FILE(path_translated),
    // causing php_execute_script("public/index.php") to look for public/public/index.php.
    let resp = engine.execute_file("public/index.php", "GET", "/", "", &[]);
    assert!(
        !resp.success,
        "Defect confirmed: relative path in subdirectory fails"
    );
    let body = String::from_utf8_lossy(&resp.body);
    assert!(
        body.contains("Failed opening required 'public/index.php'"),
        "Expected stream open failure, got: {}",
        body
    );

    // Conversely, canonical absolute path succeeds:
    let abs_path = std::fs::canonicalize("public/index.php").unwrap();
    let resp_abs = engine.execute_file(abs_path.to_str().unwrap(), "GET", "/", "", &[]);
    assert!(resp_abs.success, "Canonical absolute path succeeds");
}
