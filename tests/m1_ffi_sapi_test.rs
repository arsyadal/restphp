//! Milestone 1 Core C-FFI & Custom SAPI Subsystem Integration Test Suite
//!
//! Tests in-memory string eval, script file execution, bailout/exit recovery,
//! syntax and runtime error protection, and multi-cycle request lifecycle isolation.

use restphp::sapi::context::WorkerRequestContext;
use restphp::sapi::{ExecutionTarget, PhpEngine};
use std::sync::Mutex;

static ZEND_TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Helper that executes a closure within an active request lifecycle.
fn run_in_request_context(
    engine: &PhpEngine,
    target: ExecutionTarget,
    post_body: bytes::Bytes,
    server_vars: Vec<(String, String)>,
    cookie: Option<&str>,
) -> (restphp::sapi::PhpResponse, WorkerRequestContext) {
    run_in_request_context_full(engine, target, "GET", post_body, server_vars, cookie, None)
}

fn run_in_request_context_full(
    engine: &PhpEngine,
    target: ExecutionTarget,
    method: &str,
    post_body: bytes::Bytes,
    server_vars: Vec<(String, String)>,
    cookie: Option<&str>,
    content_type: Option<&str>,
) -> (restphp::sapi::PhpResponse, WorkerRequestContext) {
    let mut ctx = WorkerRequestContext::new(post_body, server_vars);
    if let Some(c) = cookie {
        ctx = ctx.with_cookie(c);
    }

    let resp = engine
        .execute_request(&mut ctx, &target, method, "/test", "", content_type, None)
        .expect("Execution request failed");

    (resp, ctx)
}

#[test]
fn test_m1_core_ffi_sapi_comprehensive_suite() {
    let _guard = ZEND_TEST_MUTEX.lock().unwrap();

    // 1. Initialize Engine & SAPI
    let engine = PhpEngine::init().expect("Failed to initialize PhpEngine");

    // =========================================================================
    // Test 1: In-memory string evaluation capturing output in memory
    // =========================================================================
    {
        let target = ExecutionTarget::Inline("echo 'Hello RestPHP from FFI!';".to_string());
        let (resp, ctx) =
            run_in_request_context(&engine, target, bytes::Bytes::new(), Vec::new(), None);
        assert_eq!(resp.status, 200);
        assert!(resp.success);
        assert_eq!(
            String::from_utf8_lossy(&resp.body),
            "Hello RestPHP from FFI!"
        );
        assert_eq!(ctx.output_buffer, resp.body);
    }

    // =========================================================================
    // Test 2: PHP script file execution capturing output in memory
    // =========================================================================
    {
        let script_path = std::env::temp_dir().join("restphp_test_m1_script.php");
        std::fs::write(
            &script_path,
            "<?php echo json_encode(['status'=>'ok','engine'=>'RestPHP']); ?>",
        )
        .expect("Failed to write test script");

        let target = ExecutionTarget::File(script_path.clone());
        let (resp, _) =
            run_in_request_context(&engine, target, bytes::Bytes::new(), Vec::new(), None);
        let _ = std::fs::remove_file(&script_path);

        assert_eq!(resp.status, 200);
        assert!(resp.success);
        assert_eq!(
            String::from_utf8_lossy(&resp.body),
            "{\"status\":\"ok\",\"engine\":\"RestPHP\"}"
        );
    }

    // =========================================================================
    // Test 3: Handling exit(0) without crashing host process
    // =========================================================================
    {
        let target = ExecutionTarget::Inline(
            "echo 'Before exit(0); '; exit(0); echo 'After exit(0); ';".to_string(),
        );
        let (resp, _) =
            run_in_request_context(&engine, target, bytes::Bytes::new(), Vec::new(), None);
        assert_eq!(resp.status, 200);
        assert!(resp.success);
        assert_eq!(String::from_utf8_lossy(&resp.body), "Before exit(0); ");
    }

    // =========================================================================
    // Test 4: Handling exit(1) and exit(42) without crashing host process
    // =========================================================================
    {
        let target = ExecutionTarget::Inline("echo 'Pre-exit(1); '; exit(1);".to_string());
        let (resp, _) =
            run_in_request_context(&engine, target, bytes::Bytes::new(), Vec::new(), None);
        assert!(
            !resp.success,
            "exit(1) should report unsuccessful execution"
        );
        assert_eq!(String::from_utf8_lossy(&resp.body), "Pre-exit(1); ");

        let target_42 = ExecutionTarget::Inline("exit(42);".to_string());
        let (resp_42, _) =
            run_in_request_context(&engine, target_42, bytes::Bytes::new(), Vec::new(), None);
        assert!(
            !resp_42.success,
            "exit(42) should report unsuccessful execution"
        );
    }

    // =========================================================================
    // Test 5: Handling die("message") without crashing host process
    // =========================================================================
    {
        let target = ExecutionTarget::Inline("die('Server terminating gracefully');".to_string());
        let (resp, _) =
            run_in_request_context(&engine, target, bytes::Bytes::new(), Vec::new(), None);
        assert_eq!(resp.status, 200);
        assert_eq!(
            String::from_utf8_lossy(&resp.body),
            "Server terminating gracefully"
        );
    }

    // =========================================================================
    // Test 6: Parse errors and syntax errors without segfaulting
    // =========================================================================
    {
        let target = ExecutionTarget::Inline("echo 123 + ;".to_string());
        let (resp, _) =
            run_in_request_context(&engine, target, bytes::Bytes::new(), Vec::new(), None);
        assert!(
            !resp.success,
            "Syntax error must report unsuccessful execution"
        );
        let body_str = String::from_utf8_lossy(&resp.body);
        assert!(
            body_str.contains("Parse error") || body_str.contains("syntax error"),
            "Expected Parse error in output buffer, got: {}",
            body_str
        );
    }

    // =========================================================================
    // Test 7: Division by zero error handling
    // =========================================================================
    {
        let target = ExecutionTarget::Inline("echo 10 / 0;".to_string());
        let (resp, _) =
            run_in_request_context(&engine, target, bytes::Bytes::new(), Vec::new(), None);
        assert!(!resp.success);
        let body_str = String::from_utf8_lossy(&resp.body);
        assert!(
            body_str.contains("DivisionByZeroError"),
            "Expected DivisionByZeroError in output buffer, got: {}",
            body_str
        );
    }

    // =========================================================================
    // Test 8: Uncaught RuntimeException error handling
    // =========================================================================
    {
        let target = ExecutionTarget::Inline(
            "throw new RuntimeException('Intentional uncaught exception');".to_string(),
        );
        let (resp, _) =
            run_in_request_context(&engine, target, bytes::Bytes::new(), Vec::new(), None);
        assert!(!resp.success);
        let body_str = String::from_utf8_lossy(&resp.body);
        assert!(
            body_str.contains("RuntimeException")
                && body_str.contains("Intentional uncaught exception"),
            "Expected RuntimeException stack trace in output buffer, got: {}",
            body_str
        );
    }

    // =========================================================================
    // Test 9: Call to undefined function without segfaulting
    // =========================================================================
    {
        let target = ExecutionTarget::Inline("undefined_restphp_test_func();".to_string());
        let (resp, _) =
            run_in_request_context(&engine, target, bytes::Bytes::new(), Vec::new(), None);
        assert!(!resp.success);
        let body_str = String::from_utf8_lossy(&resp.body);
        assert!(
            body_str.contains("Call to undefined function undefined_restphp_test_func()"),
            "Expected undefined function error, got: {}",
            body_str
        );
    }

    // =========================================================================
    // Test 10: Multi-cycle request lifecycle test (10 iterations)
    // =========================================================================
    {
        for i in 1..=10 {
            let code = format!(
                "if (isset($persistent_marker)) {{ echo 'LEAK_DETECTED'; }} \
                 else {{ echo 'CYCLE_{}_OK'; }} \
                 $persistent_marker = 'cycle_{}_val'; \
                 function reusable_fn_{}() {{ return {}; }} \
                 class ReusableClass_{} {{ public static function val() {{ return {}; }} }}",
                i, i, i, i, i, i
            );
            let target = ExecutionTarget::Inline(code);
            let (resp, _) =
                run_in_request_context(&engine, target, bytes::Bytes::new(), Vec::new(), None);
            assert!(resp.success, "Cycle {} failed", i);
            assert_eq!(
                String::from_utf8_lossy(&resp.body),
                format!("CYCLE_{}_OK", i),
                "State leak detected in cycle {}",
                i
            );
        }
    }

    // =========================================================================
    // Test 11: Response headers and HTTP status code capture
    // =========================================================================
    {
        let target = ExecutionTarget::Inline(
            "http_response_code(201); header('X-RestPHP-Custom: Active'); echo 'created';"
                .to_string(),
        );
        let (resp, ctx) =
            run_in_request_context(&engine, target, bytes::Bytes::new(), Vec::new(), None);
        assert_eq!(resp.status, 201);
        assert_eq!(ctx.status_code, 201);
        assert!(
            resp.headers
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("X-RestPHP-Custom") && v == "Active"),
            "Custom header missing: {:?}",
            resp.headers
        );
    }

    // =========================================================================
    // Test 12: Server variables ($_SERVER) registration
    // =========================================================================
    {
        let server_vars = vec![
            ("REQUEST_METHOD".to_string(), "GET".to_string()),
            ("HTTP_HOST".to_string(), "localhost:8080".to_string()),
            ("MY_VAR".to_string(), "m1_verified".to_string()),
        ];
        let target = ExecutionTarget::Inline(
            "echo json_encode(['m' => $_SERVER['REQUEST_METHOD'] ?? '', 'h' => $_SERVER['HTTP_HOST'] ?? '', 'v' => $_SERVER['MY_VAR'] ?? '']);".to_string(),
        );
        let (resp, _) =
            run_in_request_context(&engine, target, bytes::Bytes::new(), server_vars, None);
        assert_eq!(resp.status, 200);
        let body_str = String::from_utf8_lossy(&resp.body);
        assert_eq!(
            body_str,
            "{\"m\":\"GET\",\"h\":\"localhost:8080\",\"v\":\"m1_verified\"}"
        );
    }

    // =========================================================================
    // Test 13: Request body streaming (read_post) populating $_POST & php://input
    // =========================================================================
    {
        let body_bytes = bytes::Bytes::from_static(b"user=alice&action=login&score=100");
        let target = ExecutionTarget::Inline(
            "echo json_encode(['user' => $_POST['user'] ?? '', 'score' => $_POST['score'] ?? '', 'raw' => file_get_contents('php://input')]);".to_string(),
        );
        let (resp, _) = run_in_request_context_full(
            &engine,
            target,
            "POST",
            body_bytes,
            Vec::new(),
            None,
            Some("application/x-www-form-urlencoded"),
        );
        assert_eq!(resp.status, 200);
        assert!(resp.success);
        let body_str = String::from_utf8_lossy(&resp.body);
        assert_eq!(
            body_str,
            "{\"user\":\"alice\",\"score\":\"100\",\"raw\":\"user=alice&action=login&score=100\"}"
        );
    }

    // =========================================================================
    // Test 14: Cookie string parsing (read_cookies) populating $_COOKIE
    // =========================================================================
    {
        let target = ExecutionTarget::Inline(
            "echo json_encode(['sid' => $_COOKIE['session_id'] ?? '', 'theme' => $_COOKIE['theme'] ?? '']);".to_string(),
        );
        let (resp, _) = run_in_request_context_full(
            &engine,
            target,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            Some("session_id=s3cr3t; theme=dark"),
            None,
        );
        assert_eq!(resp.status, 200);
        assert!(resp.success);
        let body_str = String::from_utf8_lossy(&resp.body);
        assert_eq!(body_str, "{\"sid\":\"s3cr3t\",\"theme\":\"dark\"}");
    }

    // Drop engine cleanly triggers restphp_sapi_teardown()
    drop(engine);
}
