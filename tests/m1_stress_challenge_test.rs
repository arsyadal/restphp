//! Milestone 1 Empirical Challenger Stress Test Suite
//!
//! Adversarial stress testing for M1 Core C-FFI & SAPI subsystem:
//! 1. Extreme input lengths (large eval strings, 64KB to 1MB+ POST streaming)
//! 2. Complex Zend features (recursion, classes, traits, closures, circular references)
//! 3. Edge case bailout scripts (exit(255), die("aborted"), trigger_error(E_USER_ERROR), nested exceptions)
//! 4. Rapid multi-cycle execution (60+ consecutive cycles, memory stability, state isolation)

use restphp::sapi::context::WorkerRequestContext;
use restphp::sapi::{ExecutionTarget, PhpEngine, PhpResponse};
use std::sync::Mutex;

static ZEND_CHALLENGER_MUTEX: Mutex<()> = Mutex::new(());

fn execute_eval(
    engine: &PhpEngine,
    code: &str,
    method: &str,
    post_body: bytes::Bytes,
    server_vars: Vec<(String, String)>,
    cookie: Option<&str>,
    content_type: Option<&str>,
) -> (PhpResponse, WorkerRequestContext) {
    let mut ctx = WorkerRequestContext::new(post_body, server_vars);
    if let Some(c) = cookie {
        ctx = ctx.with_cookie(c);
    }
    let target = ExecutionTarget::Inline(code.to_string());
    let resp = engine
        .execute_request(&mut ctx, &target, method, "/stress", "", content_type, None)
        .expect("execute_request call failed");
    (resp, ctx)
}

#[test]
fn test_m1_stress_and_edge_cases() {
    let _guard = ZEND_CHALLENGER_MUTEX.lock().unwrap();

    let engine = PhpEngine::init().expect("Failed to initialize PhpEngine in challenger");

    println!("--- [TEST] Non-existent script file execution ---");
    {
        let resp = engine.execute_file(
            "/tmp/definitely_does_not_exist_restphp_12345.php",
            "GET",
            "/test",
            "",
            b"",
        );
        println!(
            "Non-existent file execution returned: status={}, success={}",
            resp.status, resp.success
        );
        assert!(!resp.success);
    }

    println!("--- [CHALLENGE 1A] Large String Eval (128KB string literal & 1MB generated) ---");
    {
        // 128KB string expression in eval
        let chunk_size = 131_072; // 128KB
        let test_str = "X".repeat(chunk_size);
        let code = format!(
            "$s = '{}'; echo strlen($s) . ':' . substr($s, 0, 5);",
            test_str
        );
        let (resp, _) = execute_eval(
            &engine,
            &code,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert_eq!(resp.status, 200);
        assert!(resp.success);
        assert_eq!(
            String::from_utf8_lossy(&resp.body),
            format!("{}:XXXXX", chunk_size)
        );

        // 1MB string manipulation inside Zend VM
        let code_1mb = "
            $big = str_repeat('0123456789ABCDEF', 65536); // 1,048,576 bytes
            $first = substr($big, 0, 8);
            $last = substr($big, -8);
            echo strlen($big) . ':' . $first . ':' . $last;
        ";
        let (resp_1mb, _) = execute_eval(
            &engine,
            code_1mb,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert_eq!(resp_1mb.status, 200);
        assert!(resp_1mb.success);
        assert_eq!(
            String::from_utf8_lossy(&resp_1mb.body),
            "1048576:01234567:89ABCDEF"
        );
    }

    println!("--- [CHALLENGE 1B] Streaming Large POST Body via php://input (64KB, 256KB, 1MB) ---");
    {
        for size in [65_536, 262_144, 1_048_576] {
            let pattern = b"0123456789ABCDEF";
            let reps = size / pattern.len();
            let mut payload = Vec::with_capacity(size);
            for _ in 0..reps {
                payload.extend_from_slice(pattern);
            }
            let body_bytes = bytes::Bytes::from(payload);

            let code = format!(
                "
                $raw = file_get_contents('php://input');
                $expected = str_repeat('0123456789ABCDEF', {});
                echo strlen($raw) . ':' . ($raw === $expected ? 'EXACT_MATCH' : 'MISMATCH');
            ",
                reps
            );

            let (resp, _) = execute_eval(
                &engine,
                &code,
                "POST",
                body_bytes,
                Vec::new(),
                None,
                Some("application/octet-stream"),
            );

            assert_eq!(resp.status, 200, "Failed at POST size {}", size);
            assert!(resp.success, "Failed at POST size {}", size);
            assert_eq!(
                String::from_utf8_lossy(&resp.body),
                format!("{}:EXACT_MATCH", size),
                "Hash or size mismatch for size {}",
                size
            );
        }
    }

    println!("--- [CHALLENGE 1C] Multi-chunk form urlencoded body (64KB+) ---");
    {
        let val1 = "A".repeat(32_768);
        let val2 = "B".repeat(32_768);
        let post_data = format!("k1={}&k2={}", val1, val2);
        let body_bytes = bytes::Bytes::from(post_data);

        let code = "
            echo strlen($_POST['k1'] ?? '') . ':' . strlen($_POST['k2'] ?? '');
        ";
        let (resp, _) = execute_eval(
            &engine,
            code,
            "POST",
            body_bytes,
            Vec::new(),
            None,
            Some("application/x-www-form-urlencoded"),
        );

        assert_eq!(resp.status, 200);
        assert!(resp.success);
        assert_eq!(String::from_utf8_lossy(&resp.body), "32768:32768");
    }

    println!("--- [CHALLENGE 1D] Empty POST Body (0 bytes) ---");
    {
        let code = "
            $raw = file_get_contents('php://input');
            echo 'LEN:' . strlen($raw) . ';COUNT:' . count($_POST);
        ";
        let (resp, _) = execute_eval(
            &engine,
            code,
            "POST",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            Some("application/x-www-form-urlencoded"),
        );
        assert_eq!(resp.status, 200);
        assert!(resp.success);
        assert_eq!(String::from_utf8_lossy(&resp.body), "LEN:0;COUNT:0");
    }

    // =========================================================================
    // CHALLENGE 2: Recursive/Nested Functions, Multiple Classes & Complex Zend Features
    // =========================================================================
    println!("--- [CHALLENGE 2A] Recursion Stress (depth 500) ---");
    {
        let code = "
            function recurse_depth($n) {
                if ($n <= 0) return 0;
                return 1 + recurse_depth($n - 1);
            }
            echo recurse_depth(500);
        ";
        let (resp, _) = execute_eval(
            &engine,
            code,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert_eq!(resp.status, 200);
        assert!(resp.success);
        assert_eq!(String::from_utf8_lossy(&resp.body), "500");
    }

    println!("--- [CHALLENGE 2B] Multiple Classes, Traits, Interfaces, Inheritance ---");
    {
        let code = "
            interface Operable {
                public function execute(): string;
            }
            trait LogHelper {
                public function logMsg(string $msg): string {
                    return '[LOG:' . $msg . ']';
                }
            }
            abstract class BaseOp implements Operable {
                use LogHelper;
                abstract public function getLabel(): string;
                public function execute(): string {
                    return $this->logMsg($this->getLabel());
                }
            }
            class ConcreteOp extends BaseOp {
                private int $id;
                public function __construct(int $id) { $this->id = $id; }
                public function getLabel(): string { return 'Op#' . $this->id; }
            }
            $op = new ConcreteOp(42);
            echo $op->execute();
        ";
        let (resp, _) = execute_eval(
            &engine,
            code,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert_eq!(resp.status, 200);
        assert!(resp.success);
        assert_eq!(String::from_utf8_lossy(&resp.body), "[LOG:Op#42]");
    }

    println!("--- [CHALLENGE 2C] Advanced Zend Features: Generators, Match, Closures ---");
    {
        let code = "
            $gen = function(int $max) {
                for ($i = 1; $i <= $max; $i++) {
                    yield $i => match ($i % 3) {
                        0 => 'fizz',
                        1 => 'one',
                        2 => 'two',
                    };
                }
            };
            $res = [];
            foreach ($gen(6) as $k => $v) {
                $res[] = $k . '=' . $v;
            }
            echo implode(';', $res);
        ";
        let (resp, _) = execute_eval(
            &engine,
            code,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert_eq!(resp.status, 200);
        assert!(resp.success);
        assert_eq!(
            String::from_utf8_lossy(&resp.body),
            "1=one;2=two;3=fizz;4=one;5=two;6=fizz"
        );
    }

    println!("--- [CHALLENGE 2D] Circular Object Graphs & Zend GC Stress ---");
    {
        let code = "
            class Node {
                public ?Node $child = null;
                public string $data;
                public function __construct(string $data) { $this->data = $data; }
            }
            for ($i = 0; $i < 200; $i++) {
                $a = new Node('A' . $i);
                $b = new Node('B' . $i);
                $a->child = $b;
                $b->child = $a; // cyclic reference
            }
            echo 'CIRCULAR_CYCLE_DONE';
        ";
        let (resp, _) = execute_eval(
            &engine,
            code,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert_eq!(resp.status, 200);
        assert!(resp.success);
        assert_eq!(String::from_utf8_lossy(&resp.body), "CIRCULAR_CYCLE_DONE");
    }

    // =========================================================================
    // CHALLENGE 3: Edge Case Bailout Scripts & Fatal Errors
    // =========================================================================
    println!("--- [CHALLENGE 3A] exit(255) bailout handling ---");
    {
        let code = "echo 'Pre-exit255;'; exit(255); echo 'Post-exit255;';";
        let (resp, _) = execute_eval(
            &engine,
            code,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert!(
            !resp.success,
            "exit(255) must mark response as not successful"
        );
        assert_eq!(String::from_utf8_lossy(&resp.body), "Pre-exit255;");
    }

    println!("--- [CHALLENGE 3B] exit(128) and exit(-1) bailout handling ---");
    {
        let code_128 = "echo 'Pre-exit128;'; exit(128);";
        let (resp_128, _) = execute_eval(
            &engine,
            code_128,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert!(!resp_128.success);
        assert_eq!(String::from_utf8_lossy(&resp_128.body), "Pre-exit128;");

        let code_neg1 = "echo 'Pre-exitNeg;'; exit(-1);";
        let (resp_neg1, _) = execute_eval(
            &engine,
            code_neg1,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert!(!resp_neg1.success);
        assert_eq!(String::from_utf8_lossy(&resp_neg1.body), "Pre-exitNeg;");
    }

    println!("--- [CHALLENGE 3C] die('aborted') handling ---");
    {
        let code = "echo 'Start; '; die('aborted'); echo 'Unreachable;';";
        let (resp, _) = execute_eval(
            &engine,
            code,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert_eq!(String::from_utf8_lossy(&resp.body), "Start; aborted");
    }

    println!("--- [CHALLENGE 3D] trigger_error(..., E_USER_ERROR) fatal bailout ---");
    {
        let code = "
            echo 'Before trigger_error;';
            trigger_error('Host fatal user error test', E_USER_ERROR);
            echo 'After trigger_error;';
        ";
        let (resp, _) = execute_eval(
            &engine,
            code,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert!(
            !resp.success,
            "trigger_error(E_USER_ERROR) must yield success=false"
        );
        let body_str = String::from_utf8_lossy(&resp.body);
        assert!(
            body_str.contains("Fatal error") && body_str.contains("Host fatal user error test"),
            "Expected Fatal error message in output, got: {}",
            body_str
        );
    }

    println!("--- [CHALLENGE 3E] Deeply nested chained exceptions ---");
    {
        let code = "
            $e1 = new InvalidArgumentException('Root cause 1');
            $e2 = new RuntimeException('Intermediate cause 2', 0, $e1);
            $e3 = new LogicException('Top level exception 3', 0, $e2);
            throw $e3;
        ";
        let (resp, _) = execute_eval(
            &engine,
            code,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert!(!resp.success);
        let body_str = String::from_utf8_lossy(&resp.body);
        assert!(
            body_str.contains("Top level exception 3"),
            "Missing top exception: {}",
            body_str
        );
        assert!(
            body_str.contains("Intermediate cause 2"),
            "Missing intermediate exception: {}",
            body_str
        );
        assert!(
            body_str.contains("Root cause 1"),
            "Missing root exception: {}",
            body_str
        );
    }

    println!("--- [CHALLENGE 3F] Consecutive Bailout Recovery Resilience ---");
    {
        // 1. exit(255)
        let (r1, _) = execute_eval(
            &engine,
            "exit(255);",
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert!(!r1.success);

        // 2. trigger_error(E_USER_ERROR)
        let (r2, _) = execute_eval(
            &engine,
            "trigger_error('fatal', E_USER_ERROR);",
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert!(!r2.success);

        // 3. Uncaught Exception
        let (r3, _) = execute_eval(
            &engine,
            "throw new Exception('boom');",
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert!(!r3.success);

        // 4. Clean execution immediately following bailouts
        let (r4, _) = execute_eval(
            &engine,
            "echo 'ALL_RECOVERED_CLEANLY';",
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        assert_eq!(r4.status, 200);
        assert!(r4.success);
        assert_eq!(
            String::from_utf8_lossy(resp_clean_body(&r4)),
            "ALL_RECOVERED_CLEANLY"
        );
    }

    // =========================================================================
    // CHALLENGE 4: Rapid Multi-Cycle Execution (60+ consecutive cycles)
    // =========================================================================
    println!("--- [CHALLENGE 4A] 60 Consecutive Cycles State & Error Isolation ---");
    {
        for i in 1..=60 {
            println!("Starting Cycle {}", i);
            let code = format!(
                "
                if ({} % 5 == 0) {{
                    http_response_code(418);
                    header('X-Custom-Status: teapot');
                }}
                if (isset($cycle_global)) {{
                    echo 'LEAK_VAR_DETECTED';
                }} else if (isset($_SERVER['PREV_VAR'])) {{
                    echo 'LEAK_SERVER_DETECTED';
                }} else {{
                    echo 'CYCLE_{}_CLEAN';
                }}
                $cycle_global = 'set_in_cycle_{}';
                $_SERVER['MY_VAR'] = 'val_{}';
            ",
                i, i, i, i
            );

            let server_vars = vec![("CYCLE_NUM".to_string(), i.to_string())];
            let cookie_str = format!("sess_{}=val_{}", i, i);

            println!("  Calling execute_eval for cycle {}", i);
            let (resp, ctx) = execute_eval(
                &engine,
                &code,
                "GET",
                bytes::Bytes::new(),
                server_vars,
                Some(&cookie_str),
                None,
            );
            println!(
                "  Cycle {} execute_eval returned success={}",
                i, resp.success
            );

            assert!(resp.success, "Cycle {} failed", i);
            let body_str = String::from_utf8_lossy(&resp.body);
            assert_eq!(
                body_str,
                format!("CYCLE_{}_CLEAN", i),
                "Isolation breach detected in cycle {}: {}",
                i,
                body_str
            );

            if i % 5 == 0 {
                assert_eq!(resp.status, 418);
                assert_eq!(ctx.get_header("X-Custom-Status"), Some("teapot"));
            } else {
                assert_eq!(resp.status, 200);
                assert!(ctx.get_header("X-Custom-Status").is_none());
            }
        }
    }

    println!("--- [CHALLENGE 4B] 100 Consecutive Heavy-Allocation Cycles (Memory Stability) ---");
    {
        for i in 1..=100 {
            let code = "
                $arr = range(1, 5000);
                $json = json_encode($arr);
                $decoded = json_decode($json, true);
                echo count($decoded) . ':' . $decoded[4999];
            ";
            let (resp, _) = execute_eval(
                &engine,
                code,
                "GET",
                bytes::Bytes::new(),
                Vec::new(),
                None,
                None,
            );
            assert!(resp.success, "Heavy cycle {} failed", i);
            assert_eq!(String::from_utf8_lossy(&resp.body), "5000:5000");
        }
    }

    // Teardown
    drop(engine);
    println!("=== ALL CHALLENGE TESTS PASSED SUCCESSFULLY ===");
}

fn resp_clean_body(resp: &PhpResponse) -> &[u8] {
    &resp.body
}

/// Adversarial Challenge Test: Proves that setting headers after output or with invalid
/// formatting crashes the server process with SIGSEGV (signal 11) because `sapi_module.sapi_error`
/// is NULL in `c/sapi.c`.
#[test]
fn test_reproduce_sapi_error_null_segfault() {
    // Child process mode: execute the crashing code
    if std::env::var("RESTPHP_CRASH_SUBPROCESS").is_ok() {
        let engine = PhpEngine::init().unwrap();
        // Calling header after output triggers sapi_header_op -> sapi_module.sapi_error(...)
        let code = "echo 'Output commenced;'; header('X-Crash: 1');";
        let _ = execute_eval(
            &engine,
            code,
            "GET",
            bytes::Bytes::new(),
            Vec::new(),
            None,
            None,
        );
        return;
    }

    // Parent test mode: spawn subprocess and assert that it was killed by SIGSEGV (signal 11)
    let exe = std::env::current_exe().expect("Failed to get current test exe path");
    let output = std::process::Command::new(exe)
        .arg("test_reproduce_sapi_error_null_segfault")
        .arg("--nocapture")
        .env("RESTPHP_CRASH_SUBPROCESS", "1")
        .output()
        .expect("Failed to spawn crash reproduction subprocess");

    use std::os::unix::process::ExitStatusExt;
    let signal = output.status.signal();
    println!(
        "Subprocess terminated: status={:?}, signal={:?}",
        output.status, signal
    );

    assert_eq!(
        signal,
        Some(11),
        "Empirically verified bug: subprocess must be killed with signal 11 (SIGSEGV) due to NULL sapi_error callback!"
    );
}
