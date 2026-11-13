"""Benchmark parser/report/process-boundary regressions; not benchmark measurements."""

import copy
import http.server
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import unittest

BENCHMARKS = Path(__file__).resolve().parents[1] / "benchmarks"
sys.path.insert(0, str(BENCHMARKS))
from wrk import InvalidResult, latency_ms, parse_wrk

spec = importlib.util.spec_from_file_location("benchmark_runner", BENCHMARKS / "run.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)

# Deliberately small unit-test output, never stored or presented as a measurement.
OUTPUT = """Running 1s test @ http://127.0.0.1:1/
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency  999.00ms  1.00ms  1.00s  90.00%
  1000 requests in 1.00s, 1MB read
Requests/sec: 1000.00
  50% 1ms
  95% 2ms
  99% 3ms
Benchmark non-2xx responses: 0
"""


class ParserTests(unittest.TestCase):
    def test_numeric_percentiles_and_counts(self):
        parsed = parse_wrk(OUTPUT, require_status_counter=True)
        self.assertEqual(parsed["latency_ms"], {"p50": 1.0, "p95": 2.0, "p99": 3.0})
        self.assertEqual(parsed["requests"], 1000)
        self.assertEqual(parsed["requests_per_second"], 1000)
        self.assertEqual(parsed["errors"]["total"], 0)

    def test_missing_99_never_uses_latency_mean(self):
        output = OUTPUT.replace("  99% 3ms\n", "")
        with self.assertRaisesRegex(InvalidResult, "p99 unavailable"):
            parse_wrk(output)
        self.assertIsNone(parse_wrk(output, require_percentiles=False)["latency_ms"]["p99"])

    def test_missing_95_is_not_interpolated(self):
        with self.assertRaisesRegex(InvalidResult, "p95 unavailable"):
            parse_wrk(OUTPUT.replace("  95% 2ms\n", ""))

    def test_units(self):
        for value, expected in (("2ns", 0.000002), ("2us", 0.002), ("2µs", 0.002),
                                ("2ms", 2), ("2s", 2000), ("2m", 120000)):
            with self.subTest(value=value):
                self.assertAlmostEqual(latency_ms(value), expected)
        parsed = parse_wrk(OUTPUT.replace("1ms", "1000us").replace("2ms", "0.002s").replace("3ms", "3000000ns"))
        self.assertEqual(parsed["latency_ms"]["p99"], 3)
        for value in ("nanms", "infms", "-1ms", "1", "1secs", "1e3ms"):
            with self.subTest(value=value), self.assertRaises(InvalidResult):
                latency_ms(value)

    def test_socket_and_http_errors_not_double_counted(self):
        output = OUTPUT.replace("responses: 0", "responses: 7") + (
            "  Socket errors: connect 1, read 2, write 3, timeout 4\n"
            "  Non-2xx or 3xx responses: 5\n")
        parsed = parse_wrk(output)
        self.assertEqual(parsed["errors"], {"connect": 1, "read": 2, "write": 3, "timeout": 4,
                                          "http_non_2xx_or_3xx": 5, "http_non_2xx": 7, "total": 17})
        # A redirect-only run is caught by the Lua counter even without wrk errors.
        self.assertEqual(parse_wrk(OUTPUT.replace("responses: 0", "responses: 1"))["errors"]["total"], 1)

    def test_invalid_boundary_outputs(self):
        cases = ("", "not wrk output", OUTPUT + "Requests/sec: 1000\n", OUTPUT + "  99% 4ms\n",
                 OUTPUT.replace("3ms", "nanms"), OUTPUT.replace("3ms", "0.1ms"),
                 OUTPUT.replace("1000 requests", "0 requests"), OUTPUT.replace("1.00s,", "0.00s,"),
                 OUTPUT.replace("Requests/sec: 1000.00", "Requests/sec: 2.00"),
                 OUTPUT + "Socket errors: connect invalid\n",
                 OUTPUT + "Non-2xx or 3xx responses: invalid\n",
                 OUTPUT.replace("responses: 0", "responses: invalid"),
                 OUTPUT.replace("responses: 0", "responses: 1001"))
        for output in cases:
            with self.subTest(output=output), self.assertRaises(InvalidResult):
                parse_wrk(output)

    def test_status_counter_required_for_controlled_runs(self):
        with self.assertRaises(InvalidResult):
            parse_wrk(OUTPUT.replace("Benchmark non-2xx responses: 0\n", ""), require_status_counter=True)

    def test_unchanged_historical_raw_files_have_no_p95(self):
        for path in (BENCHMARKS / "results").glob("*_wrk.txt"):
            with self.subTest(path=path):
                parsed = parse_wrk(path.read_text(), require_percentiles=False)
                self.assertIsNone(parsed["latency_ms"]["p95"])
                self.assertIsNotNone(parsed["latency_ms"]["p99"])
                with self.assertRaises(InvalidResult):
                    parse_wrk(path.read_text())


class ComparatorTests(unittest.TestCase):
    def test_matching_version_payload_architecture_and_ini_required(self):
        probe = {"php_version": "8.2.30", "architecture": "aarch64", "integer_bytes": 8,
                 "ini": dict(runner.INI), "payload_sha256": "hash", "processes": 1, "php_workers": 1}
        runner.verify_pair(probe, copy.deepcopy(probe))
        for key, value in (("php_version", "8.3.30"), ("architecture", "x86_64"), ("integer_bytes", 4),
                           ("payload_sha256", "different"), ("processes", 2), ("php_workers", 2),
                           ("ini", dict(runner.INI, memory_limit="256M"))):
            other = copy.deepcopy(probe)
            other[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                runner.verify_pair(probe, other)

    def test_owned_process_terminated_on_error(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(RuntimeError, "forced failure"):
                with runner.owned([sys.executable, "-c", "import time; time.sleep(60)"], Path(directory) / "child.log") as child:
                    self.assertIsNone(child.poll())
                    raise RuntimeError("forced failure")
            self.assertIsNotNone(child.poll())
            self.assertNotIn(child, runner.CHILDREN)

    def test_cleanup_reaches_descendant_after_leader_exit(self):
        with tempfile.TemporaryDirectory() as directory:
            pid_file = Path(directory) / "descendant.pid"
            code = ("import subprocess,sys; p = subprocess.Popen([sys.executable, '-c', "
                    "'import time; time.sleep(60)']); "
                    f"open({str(pid_file)!r}, 'w').write(str(p.pid))")
            with runner.owned([sys.executable, "-c", code], Path(directory) / "parent.log") as leader:
                leader.wait(timeout=5)
                descendant = int(pid_file.read_text())
            deadline = time.monotonic() + 5
            while True:
                rows = subprocess.check_output(["ps", "-axo", "pid=,stat="], text=True).splitlines()
                states = [row.split()[1] for row in rows if row.split() and int(row.split()[0]) == descendant]
                if not states or all(state.startswith("Z") for state in states):
                    break
                if time.monotonic() > deadline:
                    self.fail(f"owned process-group descendant {descendant} survived cleanup: {states}")
                time.sleep(0.05)

    def test_readiness_rejects_unrelated_service(self):
        class Unrelated(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                body = json.dumps({"token": "nonce", "pid": -1}).encode()
                self.send_response(200)
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *args):
                pass

        with http.server.HTTPServer(("127.0.0.1", 0), Unrelated) as server:
            thread = threading.Thread(target=server.serve_forever)
            thread.start()
            try:
                with tempfile.TemporaryDirectory() as directory:
                    with runner.owned([sys.executable, "-c", "import time; time.sleep(60)"], Path(directory) / "child.log") as child:
                        with self.assertRaisesRegex(RuntimeError, "different process/service"):
                            runner.ready(child, f"http://127.0.0.1:{server.server_port}/", "nonce")
            finally:
                server.shutdown()
                thread.join()

    def test_readiness_fails_on_child_exit(self):
        with tempfile.TemporaryDirectory() as directory:
            with runner.owned([sys.executable, "-c", "raise SystemExit(7)"], Path(directory) / "child.log") as child:
                child.wait(timeout=5)
                with self.assertRaisesRegex(RuntimeError, "exited during readiness"):
                    runner.ready(child, "http://127.0.0.1:1/", "nonce")


@unittest.skipUnless(shutil.which("php"), "PHP CLI required for real report boundary")
class ReportTests(unittest.TestCase):
    def report(self, data):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "results.json"
            path.write_text(json.dumps(data))
            return subprocess.run([shutil.which("php"), str(BENCHMARKS / "report.php"), str(path)],
                                  capture_output=True, text=True)

    def test_legacy_schema_rejected(self):
        result = self.report({"hello": {"req_sec": "100", "latency": "999ms"}})
        self.assertEqual(result.returncode, 1)
        self.assertIn("legacy latency strings are not p99", result.stderr)

    def test_null_historical_latencies_stay_unavailable(self):
        result = self.report(json.loads((BENCHMARKS / "results" / "results.json").read_text()))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("historical-unverified", result.stdout)
        self.assertIn("N/A", result.stdout)
        for obsolete in ("2.370", "5.520", "11.020"):
            self.assertNotIn(obsolete, result.stdout)

    def test_new_numeric_measurements_and_failed_status(self):
        data = {"schema_version": 2, "status": "complete", "provenance": "measured", "comparative": True,
                "runs": [{"runtime": "restphp", "scenario": "unit-test", "repetition": 1,
                          "measurement": parse_wrk(OUTPUT)}]}
        result = self.report(data)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("3.000", result.stdout)
        data["status"] = "failed"
        self.assertEqual(self.report(data).returncode, 1)
        data["runs"][0]["measurement"]["latency_ms"]["p99"] = "999ms"
        self.assertEqual(self.report(data).returncode, 1)


if __name__ == "__main__":
    unittest.main()
