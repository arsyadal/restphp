#!/usr/bin/env python3
"""Mixed HTTP workloads, resource budgets, and process-lifecycle checks for NTS workers."""

import argparse
import concurrent.futures
import datetime
import hashlib
import http.client
import json
import os
from pathlib import Path
import platform
import re
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.parse
import uuid

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "tests" / "fixtures" / "soak_worker.php"
OPERATIONS = ("plain", "json", "binary", "form", "upload", "session", "exception", "plain")


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def free_port():
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


def wait_until(predicate, timeout, message):
    deadline = time.monotonic() + timeout
    while not predicate():
        require(time.monotonic() < deadline, message)
        time.sleep(0.01)


def resource_sample(pid):
    if sys.platform.startswith("linux"):
        status = Path(f"/proc/{pid}/status").read_text()
        rss = re.search(r"^VmRSS:\s+(\d+) kB$", status, re.MULTILINE)
        require(rss is not None, "Cannot measure worker VmRSS")
        descriptors = len(list(Path(f"/proc/{pid}/fd").iterdir()))
        return int(rss.group(1)), descriptors
    if sys.platform == "darwin":
        memory = subprocess.run(["ps", "-p", str(pid), "-o", "rss="], check=True,
                                capture_output=True, text=True).stdout.strip()
        require(memory.isdigit(), "Cannot measure worker RSS")
        handles = subprocess.run(["lsof", "-a", "-p", str(pid), "-Ff"],
                                 capture_output=True, text=True)
        require(handles.returncode == 0, "Cannot measure worker file descriptors with lsof")
        descriptors = sum(bool(re.fullmatch(r"f\d+", line)) for line in handles.stdout.splitlines())
        require(descriptors > 0, "Worker descriptor inventory is unavailable")
        return int(memory), descriptors
    raise RuntimeError("Resource measurement supports Linux /proc and macOS ps/lsof")


def check_budgets(samples, rss_growth_kib, fd_growth, peak_rss_kib):
    require(bool(samples), "A worker has no resource samples")
    baseline = samples[0]
    peak_rss = max(sample["rss_kib"] for sample in samples)
    peak_fd = max(sample["fd_count"] for sample in samples)
    require(peak_rss <= peak_rss_kib, f"Worker RSS exceeds budget: {peak_rss} KiB")
    require(peak_rss - baseline["rss_kib"] <= rss_growth_kib,
            f"Worker RSS grew by {peak_rss - baseline['rss_kib']} KiB after warmup")
    require(peak_fd - baseline["fd_count"] <= fd_growth,
            f"Worker descriptor count grew by {peak_fd - baseline['fd_count']}")
    return {"baseline_rss_kib": baseline["rss_kib"], "peak_rss_kib": peak_rss,
            "last_rss_kib": samples[-1]["rss_kib"],
            "rss_growth_kib": peak_rss - baseline["rss_kib"],
            "baseline_fd_count": baseline["fd_count"], "peak_fd_count": peak_fd,
            "last_fd_count": samples[-1]["fd_count"]}


class Worker:
    def __init__(self, binary, state, port, max_requests=0):
        self.state = state
        self.port = port
        self.log = tempfile.TemporaryFile(mode="w+")
        self.boot = None
        self.lock = threading.Lock()
        self.counts = {}
        self.samples = []
        self.peak_sequence = 0
        self.process = subprocess.Popen(
            [str(binary), "serve", "--host", "127.0.0.1", "--port", str(port),
             "--persistent-worker", str(FIXTURE), "--max-requests", str(max_requests),
             "--max-queue", "32", "--max-body-bytes", "1048576", "--shutdown-timeout-secs", "5"],
            cwd=state, env=dict(os.environ, RESTPHP_SOAK_STATE=str(state), PHPRC=str(state / "php.ini")),
            stdout=self.log, stderr=self.log)
        try:
            def listening():
                require(self.process.poll() is None, "Worker exited before listener readiness")
                try:
                    with socket.create_connection(("127.0.0.1", port), timeout=0.1):
                        return True
                except OSError:
                    return False
            wait_until(listening, 10, "Worker listener readiness timed out")
        except BaseException as error:
            self.log.seek(0)
            details = self.log.read()[-8192:].replace(str(state), "$SOAK_STATE")
            self.close()
            raise RuntimeError("Worker startup failed: " + details) from error

    def request(self, path, method="GET", body=None, headers=None, expected=200, identifier=None):
        identifier = identifier or uuid.uuid4().hex
        request_headers = {"X-Soak-Id": identifier, **(headers or {})}
        connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=8)
        try:
            connection.request(method, path, body=body, headers=request_headers)
            response = connection.getresponse()
            content = response.read()
            response_headers = {key.lower(): value for key, value in response.getheaders()}
            require(response.status == expected,
                    f"{method} {path} returned {response.status}, expected {expected}")
            decoded = json.loads(content)
        finally:
            connection.close()
        require(decoded["id"] == identifier, "Response belongs to a different request")
        require(decoded["pid"] == self.process.pid, "Response came from an unrelated process")
        require(decoded["method"] == method, "Request method leaked between callbacks")
        require(decoded["path"] == path.split("?", 1)[0], "Request URI leaked between callbacks")
        require(decoded["leaked"] is False, "A previous request mutated current superglobals")
        require(decoded["environment"] == decoded["server_environment"] == "boot",
                "Bootstrap environment baseline was not restored")
        require(decoded["probe_header"] == request_headers.get("X-Soak-Probe"),
                "A previous request header leaked")
        if path != "/json":
            require("x-soak-only" not in response_headers, "Response header leaked from JSON request")
        require(isinstance(decoded["sequence"], int) and decoded["sequence"] > 0,
                "Invalid persistent callback sequence")
        with self.lock:
            if self.boot is None:
                self.boot = decoded["boot"]
            require(decoded["boot"] == self.boot, "Application unexpectedly booted again in one process")
            self.peak_sequence = max(self.peak_sequence, decoded["sequence"])
            self.counts[path.split("?", 1)[0]] = self.counts.get(path.split("?", 1)[0], 0) + 1
        return decoded, response_headers

    def sample(self, origin):
        require(self.process.poll() is None, "Worker stopped unexpectedly during soak")
        rss, descriptors = resource_sample(self.process.pid)
        self.samples.append({"elapsed_s": round(time.monotonic() - origin, 3),
                             "rss_kib": rss, "fd_count": descriptors})

    def no_upload_residue(self):
        require(not any((self.state / "uploads").iterdir()), "Multipart temporary files leaked")

    def listener_closed(self):
        with socket.socket() as listener:
            require(listener.connect_ex(("127.0.0.1", self.port)) != 0,
                    "Stopped worker left an active listener")

    def active_disconnect(self):
        identifier = uuid.uuid4().hex
        stream = socket.create_connection(("127.0.0.1", self.port), timeout=2)
        try:
            stream.sendall((f"GET /disconnect?id={identifier} HTTP/1.1\r\nHost: localhost\r\n"
                            "Connection: close\r\n\r\n").encode())
            wait_until(lambda: (self.state / ("started-" + identifier)).exists(), 3,
                       "Disconnect workload did not enter PHP")
        finally:
            stream.close()
        wait_until(lambda: (self.state / ("finished-" + identifier)).exists(), 3,
                   "PHP did not settle work after client disconnect")
        for prefix in ("started-", "finished-"):
            (self.state / (prefix + identifier)).unlink()
        self.request("/plain")

    def drain_active(self):
        identifier = uuid.uuid4().hex
        with concurrent.futures.ThreadPoolExecutor(max_workers=1) as executor:
            active = executor.submit(self.request, "/slow?id=" + identifier, identifier=identifier)
            wait_until(lambda: (self.state / ("started-" + identifier)).exists(), 3,
                       "Shutdown workload did not enter PHP")
            self.process.send_signal(signal.SIGTERM)
            response, _ = active.result(timeout=8)
            require(response["finished"] is True, "Shutdown lost the active PHP response")
        require(self.process.wait(timeout=8) == 0, "Active drain did not exit successfully")
        self.listener_closed()
        self.no_upload_residue()
        for prefix in ("started-", "finished-"):
            (self.state / (prefix + identifier)).unlink()

    def fatal(self):
        connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=8)
        try:
            connection.request("GET", "/fatal")
            response = connection.getresponse()
            require(response.status == 500 and response.read() == b"Internal Server Error",
                    "Fatal callback exposed partial output or diagnostics")
        finally:
            connection.close()
        self.process.wait(timeout=8)
        self.listener_closed()
        self.no_upload_residue()

    def close(self):
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=8)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
        self.log.close()


def workload(worker, operation):
    token = uuid.uuid4().hex
    if operation == "plain":
        result, _ = worker.request("/plain")
        require(result["cookie"] == [] and result["query"] == [], "Plain request inherited input")
    elif operation == "json":
        body = json.dumps({"token": token}, separators=(",", ":")).encode()
        result, headers = worker.request("/json", "POST", body,
            {"Content-Type": "application/json", "X-Soak-Probe": token}, 201)
        require(result["json"] == {"token": token} and headers.get("x-soak-only") == "json",
                "JSON request or response metadata changed")
        require(result["body_sha256"] == hashlib.sha256(body).hexdigest(), "JSON input body changed")
    elif operation == "binary":
        body = bytes(range(256)) * 8 + token.encode()
        result, _ = worker.request("/binary", "PUT", body, {"Content-Type": "application/octet-stream"})
        require(result["body_bytes"] == len(body) and result["body_sha256"] == hashlib.sha256(body).hexdigest(),
                "Binary request body was truncated or leaked")
    elif operation == "form":
        body = urllib.parse.urlencode({"token": token, "value": "a+b & null\0"})
        result, _ = worker.request("/form", "POST", body,
                                   {"Content-Type": "application/x-www-form-urlencoded"})
        require(result["form"] == {"token": token, "value": "a+b & null\0"}
                and result["raw_body"] == body, "Form decoding or raw input changed")
    elif operation == "upload":
        payload = bytes(range(256)) * 4
        boundary = "restphp-" + token
        body = (f"--{boundary}\r\nContent-Disposition: form-data; name=\"token\"\r\n\r\n{token}\r\n"
                f"--{boundary}\r\nContent-Disposition: form-data; name=\"payload\"; filename=\"sample.bin\"\r\n"
                "Content-Type: application/octet-stream\r\n\r\n").encode() + payload + f"\r\n--{boundary}--\r\n".encode()
        result, _ = worker.request("/upload", "POST", body,
                                   {"Content-Type": "multipart/form-data; boundary=" + boundary})
        require(result["upload"] == {"name": "sample.bin", "bytes": len(payload),
                "sha256": hashlib.sha256(payload).hexdigest(), "is_uploaded_file": True},
                "Multipart upload metadata or content changed")
        require(result["form"] == {"token": token}, "Multipart field leaked or disappeared")
    elif operation == "session":
        first, _ = worker.request("/session")
        identifier = first["session_id"]
        require(re.fullmatch(r"[A-Za-z0-9,-]+", identifier) is not None, "Unsafe session identifier")
        second, _ = worker.request("/session", headers={"Cookie": "PHPSESSID=" + identifier})
        fresh, _ = worker.request("/session")
        require(first["session_counter"] == fresh["session_counter"] == 1,
                "Independent sessions inherited another client's state")
        require(second["session_id"] == identifier and second["session_counter"] == 2,
                "Explicit session continuity failed")
        require(fresh["session_id"] != identifier, "An absent cookie reused another session ID")
        for session_id in (identifier, fresh["session_id"]):
            require(re.fullmatch(r"[A-Za-z0-9,-]+", session_id) is not None, "Unsafe session identifier")
            (worker.state / "sessions" / ("sess_" + session_id)).unlink()
    elif operation == "exception":
        result, _ = worker.request("/exception", expected=500)
        require(result.get("caught") == "LogicException", "Recoverable exception escaped its handler")
    else:
        raise RuntimeError("Unknown soak operation " + operation)


def runtime_metadata(binary):
    result = subprocess.run([str(binary), "-e",
        "echo json_encode(['php'=>PHP_VERSION,'zts'=>(bool)PHP_ZTS,'extensions'=>get_loaded_extensions()]);"],
        check=True, capture_output=True, text=True, timeout=10)
    records = [line for line in result.stdout.splitlines() if line.startswith('{"php":')]
    require(len(records) == 1, "Cannot identify embedded PHP runtime")
    metadata = json.loads(records[0])
    require(metadata["zts"] is False, "This soak suite targets the one-worker NTS runtime")
    return metadata


def run(args, report):
    binary = args.binary.resolve(strict=True)
    require(binary.is_file() and os.access(binary, os.X_OK), "Runtime binary is not executable")
    report["binary_sha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
    report["runtime"] = runtime_metadata(binary)
    port = free_port()
    with tempfile.TemporaryDirectory(prefix="restphp-soak-") as directory:
        state = Path(directory)
        for name in ("sessions", "uploads"):
            (state / name).mkdir()
        (state / "php.ini").write_text(
            'memory_limit=128M\ndisplay_errors=0\nlog_errors=1\n'
            + 'session.use_cookies=1\nsession.use_strict_mode=1\nsession.gc_probability=0\n'
            + 'upload_tmp_dir="' + str(state / "uploads") + '"\n')
        worker = None
        workers = []
        origin = time.monotonic()
        try:
            worker = Worker(binary, state, port, max_requests=3)
            workload(worker, "upload")
            workload(worker, "json")
            workload(worker, "exception")
            require(worker.process.wait(timeout=8) == 0, "Native max-requests recycle did not exit cleanly")
            worker.listener_closed()
            worker.no_upload_residue()
            report["native_request_limit_recycle"] = True
            worker.close()
            worker = None
            deadline = time.monotonic() + args.duration_secs
            next_sample = 0.0
            next_recycle = time.monotonic() + args.recycle_interval_secs if args.recycle_interval_secs else float("inf")
            operation_index = 0
            batches = 0
            with concurrent.futures.ThreadPoolExecutor(max_workers=args.concurrency) as executor:
                while time.monotonic() < deadline:
                    if worker is None:
                        worker = Worker(binary, state, port)
                        for _ in range(args.warmup_requests):
                            worker.request("/plain")
                        worker.sample(origin)
                        workers.append(worker)
                        next_sample = time.monotonic() + args.sample_interval_secs
                    futures = [executor.submit(workload, worker, OPERATIONS[(operation_index + index) % len(OPERATIONS)])
                               for index in range(args.concurrency * 2)]
                    for future in futures:
                        future.result(timeout=20)
                    operation_index += len(futures)
                    batches += 1
                    report["operations"] += len(futures)
                    worker.no_upload_residue()
                    if batches % args.disconnect_every_batches == 0:
                        worker.active_disconnect()
                        report["active_client_disconnects"] += 1
                    now = time.monotonic()
                    if now >= next_sample:
                        worker.sample(origin)
                        check_budgets(worker.samples, args.max_rss_growth_mib * 1024,
                                      args.max_fd_growth, args.max_peak_rss_mib * 1024)
                        next_sample = now + args.sample_interval_secs
                    if now >= next_recycle:
                        worker.sample(origin)
                        worker.drain_active()
                        worker.close()
                        worker = None
                        report["graceful_process_replacements"] += 1
                        next_recycle = time.monotonic() + args.recycle_interval_secs
                    time.sleep(args.batch_delay_secs)
            if worker is not None:
                worker.sample(origin)
                worker.drain_active()
                worker.close()
                worker = None
            require(report["active_client_disconnects"] > 0, "No active-client disconnect was exercised")
            for previous in workers:
                summary = check_budgets(previous.samples, args.max_rss_growth_mib * 1024,
                                        args.max_fd_growth, args.max_peak_rss_mib * 1024)
                report["workers"].append({"pid": previous.process.pid, "boot": previous.boot,
                    "validated_http_responses": sum(previous.counts.values()), "workloads": previous.counts,
                    "peak_callback_sequence": previous.peak_sequence, "resources": summary,
                    "samples": previous.samples})
            require(len({previous.boot for previous in workers}) == len(workers),
                    "Process replacement reused a bootstrap identity")
            worker = Worker(binary, state, port)
            worker.fatal()
            report["fatal_worker_stopped_without_output_leak"] = True
            worker.close()
            worker = None
            report["active_shutdown_response_completed"] = True
            report["owned_listeners_closed"] = True
            report["upload_temporary_files_remaining"] = len(list((state / "uploads").iterdir()))
            report["elapsed_s"] = round(time.monotonic() - origin, 3)
            report["status"] = "passed"
        except BaseException:
            if worker is not None:
                worker.log.flush()
                worker.log.seek(0)
                report["last_worker_log"] = worker.log.read()[-16384:].replace(str(state), "$SOAK_STATE")
            raise
        finally:
            if worker is not None:
                worker.close()
            for previous in workers:
                if not previous.log.closed:
                    previous.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target" / "release" / "restphp")
    parser.add_argument("--duration-secs", type=float, default=1800)
    parser.add_argument("--concurrency", type=int, default=4)
    parser.add_argument("--sample-interval-secs", type=float, default=5)
    parser.add_argument("--recycle-interval-secs", type=float, default=600)
    parser.add_argument("--warmup-requests", type=int, default=100)
    parser.add_argument("--disconnect-every-batches", type=int, default=25)
    parser.add_argument("--batch-delay-secs", type=float, default=0.05)
    parser.add_argument("--max-rss-growth-mib", type=int, default=32)
    parser.add_argument("--max-peak-rss-mib", type=int, default=256)
    parser.add_argument("--max-fd-growth", type=int, default=16)
    parser.add_argument("--output", type=Path, default=ROOT / "scratch" / "soak-report.json")
    args = parser.parse_args()
    for name in ("duration_secs", "concurrency", "sample_interval_secs", "warmup_requests",
                 "disconnect_every_batches", "max_rss_growth_mib", "max_peak_rss_mib", "max_fd_growth"):
        if getattr(args, name) <= 0:
            parser.error(name + " must be positive")
    if args.recycle_interval_secs < 0 or args.batch_delay_secs < 0:
        parser.error("recycle interval and batch delay cannot be negative")
    report = {"schema_version": 1, "mode": "soak" if args.duration_secs >= 1800 else "smoke",
        "started_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "host": {"os": platform.system(), "architecture": platform.machine()},
        "configuration": {key: value for key, value in vars(args).items() if key not in ("binary", "output")},
        "operations": 0, "active_client_disconnects": 0, "graceful_process_replacements": 0,
        "workers": [], "status": "failed"}
    try:
        run(args, report)
    except (Exception, KeyboardInterrupt) as error:
        report["error"] = str(error) or type(error).__name__
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"status": report["status"], "mode": report["mode"],
        "operations": report["operations"], "workers": len(report["workers"]),
        "report": str(args.output), "error": report.get("error")}))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
