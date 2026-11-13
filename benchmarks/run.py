#!/usr/bin/env python3
"""Sequential, fail-closed classic-request comparison. See --help for controls."""

import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import platform
import resource
import secrets
import shutil
import signal
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request

from wrk import InvalidResult, parse_wrk

ROOT = Path(__file__).resolve().parent
SCENARIOS = {"hello_plaintext": "hello.php", "json_api": "json_api.php", "heavy_compute": "heavy_compute.php"}
INI = {"memory_limit": "128M", "max_execution_time": "0", "display_errors": "0", "log_errors": "1",
       "precision": "14", "serialize_precision": "-1", "date.timezone": "UTC",
       "opcache.enable": "0", "opcache.enable_cli": "0", "opcache.jit": "0"}
CHILDREN = set()
HTTP = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def tooling():
    names = ("wrk", "frankenphp", "docker", "podman", "php")
    return {name: shutil.which(name) for name in names}


def runtime_env(name):
    env = {key: value for key, value in os.environ.items() if key != "LD_LIBRARY_PATH"}
    if name == "restphp" and os.environ.get("RESTPHP_LD_LIBRARY_PATH"):
        env["LD_LIBRARY_PATH"] = os.environ["RESTPHP_LD_LIBRARY_PATH"]
    return env


def version(name, command):
    result = subprocess.run(command, env=runtime_env(name), stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=15)
    if name != "wrk" and result.returncode != 0:
        raise RuntimeError(f"version probe failed ({result.returncode}): {result.stdout.strip()}")
    # wrk --version prints its version banner and usage while exiting 1.
    if name == "wrk" and "wrk " not in result.stdout.lower():
        raise RuntimeError(f"unexpected wrk version output: {result.stdout.strip()}")
    if not result.stdout.strip():
        raise RuntimeError(f"no version output from {command}")
    return {"command": command, "exit_code": result.returncode, "output": result.stdout.strip()}


def limits(args):
    result = {"open_files": args.open_files, "core_bytes": 0, "address_space_bytes": args.memory_bytes,
              "cpu_affinity": args.cpus, "cpu_time_seconds": "inherited", "host_cpu_count": os.cpu_count()}
    result["inherited"] = {name: list(resource.getrlimit(getattr(resource, name)))
                           for name in ("RLIMIT_AS", "RLIMIT_CPU", "RLIMIT_NOFILE", "RLIMIT_CORE")}
    result["cgroup"] = {}
    for name in ("cpu.max", "cpuset.cpus.effective", "memory.max", "pids.max"):
        path = Path("/sys/fs/cgroup") / name
        if path.exists():
            result["cgroup"][name] = path.read_text().strip()
    if args.memory_bytes and sys.platform != "linux":
        raise RuntimeError("--memory-bytes needs Linux RLIMIT_AS; use the resource-limited container on macOS")
    if args.cpus and not hasattr(os, "sched_setaffinity"):
        raise RuntimeError("--cpus needs Linux sched_setaffinity; use the container on macOS")
    bounded_cgroup = (result["cgroup"].get("cpu.max", "max").split()[0] != "max"
                      and result["cgroup"].get("memory.max", "max") != "max")
    if args.runtime == "compare" and not (bounded_cgroup or (args.cpus and args.memory_bytes)):
        raise RuntimeError("controlled comparison requires finite CPU/memory limits: run the Docker image with "
                           "--cpus 2 --memory 2g, or supply Linux --cpus and --memory-bytes; no uncapped comparison")
    result["scope"] = "same inherited cgroup for sequential runs (and load generator); per-server rlimits/affinity below"
    return result


def child_setup(args):
    resource.setrlimit(resource.RLIMIT_NOFILE, (args.open_files, args.open_files))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    if args.memory_bytes:
        resource.setrlimit(resource.RLIMIT_AS, (args.memory_bytes, args.memory_bytes))
    if args.cpus:
        os.sched_setaffinity(0, set(args.cpus))


def stop(process):
    # Kill the entire owned group even if its leader already exited.
    with contextlib.suppress(ProcessLookupError):
        os.killpg(process.pid, signal.SIGTERM)
    try:
        process.wait(timeout=8)
    except subprocess.TimeoutExpired:
        with contextlib.suppress(ProcessLookupError):
            os.killpg(process.pid, signal.SIGKILL)
        process.wait(timeout=5)
    finally:
        CHILDREN.discard(process)


@contextlib.contextmanager
def owned(command, log, env=None, args=None, cwd=None):
    with open(log, "wb") as stream:
        process = subprocess.Popen(command, cwd=cwd, env=env, stdout=stream, stderr=subprocess.STDOUT,
                                   start_new_session=True, preexec_fn=(lambda: child_setup(args)) if args else None)
        CHILDREN.add(process)
        try:
            yield process
        finally:
            stop(process)


def interrupted(signum, frame):
    raise KeyboardInterrupt(f"received signal {signum}")


def snapshot(pid):
    output = subprocess.check_output(["ps", "-axo", "pid=,pgid=,rss="], text=True)
    rows = [tuple(map(int, row.split())) for row in output.splitlines() if row.strip()]
    group = [row for row in rows if row[1] == pid]
    if len(group) != 1 or group[0][0] != pid:
        raise RuntimeError(f"expected one server process in owned group {pid}, got {group}")
    return group[0][2] * 1024


def get(url):
    with HTTP.open(url, timeout=2) as response:
        return response.status, response.read()


def ready(process, url, token):
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"server exited during readiness: {process.returncode}")
        try:
            status, body = get(url + "?__benchmark_probe=" + token)
            data = json.loads(body)
            if status != 200 or data.get("token") != token or data.get("pid") != process.pid:
                raise RuntimeError("readiness response belongs to a different process/service")
            snapshot(process.pid)
            return data
        except (OSError, urllib.error.URLError, json.JSONDecodeError):
            time.sleep(0.05)
    raise RuntimeError("server readiness timed out")


def verify_pair(left, right):
    for key in ("php_version", "architecture", "integer_bytes", "ini", "payload_sha256", "processes", "php_workers"):
        if left[key] != right[key]:
            raise RuntimeError(f"comparison mismatch for {key}: {left[key]!r} != {right[key]!r}")
    # ZTS/NTS and SAPI are intentionally different; both still have one PHP VM.
    for probe in (left, right):
        for key, expected in INI.items():
            actual = probe["ini"].get(key)
            # A PHP build without opcache reports false rather than an INI value.
            if key.startswith("opcache.") and actual is False and expected == "0":
                continue
            if actual != expected:
                raise RuntimeError(f"effective PHP INI mismatch: {key}={actual!r}, expected {expected!r}")


def comparable(probe):
    result = dict(probe)
    result["ini"] = dict(probe["ini"])
    for key in INI:
        if key.startswith("opcache.") and result["ini"].get(key) is False:
            result["ini"][key] = "0"
    return result


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def run_load(args, server, url, seconds, log):
    command = [args.wrk, f"-t{args.threads}", f"-c{args.connections}", f"-d{seconds}s", "--timeout", "5s",
               "-s", str(ROOT / "percentiles.lua"), url]
    rss = []
    with owned(command, log) as load:
        deadline = time.monotonic() + seconds + 20
        while load.poll() is None:
            if server.poll() is not None:
                raise RuntimeError(f"server exited under load: {server.returncode}")
            if time.monotonic() > deadline:
                raise RuntimeError("load generator timed out")
            rss.append(snapshot(server.pid))
            time.sleep(0.1)
        if load.returncode:
            raise RuntimeError(f"load generator exited {load.returncode}: {log}")
        if server.poll() is not None:
            raise RuntimeError(f"server exited after load: {server.returncode}")
    result = parse_wrk(log.read_text(), require_status_counter=True)
    result.update({"command": command, "raw_log": str(log), "rss_bytes": {
        "peak_sampled": max(rss) if rss else snapshot(server.pid), "samples": len(rss), "sample_interval_seconds": 0.1,
        "scope": "entire single server process, excluding load generator; sampled, not kernel high-water mark"}})
    return result


def prepare_script(directory, script, token):
    target = directory / "index.php"
    keys = json.dumps(list(INI))
    # JSON list syntax is also valid PHP array syntax for these simple keys.
    target.write_text("<?php\nif (($_GET['__benchmark_probe'] ?? '') === " + json.dumps(token) + ") {\n"
        + "$ini = []; foreach (" + keys + " as $key) { $ini[$key] = ini_get($key); }\n"
        + "header('Content-Type: application/json'); echo json_encode(["
        + "'token'=>" + json.dumps(token) + ", 'pid'=>getmypid(), 'php_version'=>PHP_VERSION,"
        + "'architecture'=>php_uname('m'), 'integer_bytes'=>PHP_INT_SIZE, 'zts'=>PHP_ZTS, 'sapi'=>PHP_SAPI,"
        + "'ini'=>$ini, 'all_ini'=>ini_get_all(null, false), 'ini_file'=>php_ini_loaded_file(), 'ini_scanned'=>php_ini_scanned_files(),"
        + "'extensions'=>get_loaded_extensions()]); return; }\nrequire " + json.dumps(str(script)) + ";\n")
    return target


def server_command(name, args, directory, entry, port):
    if name == "restphp":
        return [args.restphp, "serve", "--host", "127.0.0.1", "--port", str(port), "--workers", "1",
                "--max-requests", "0", "--entrypoint", str(entry)]
    config = directory / "Caddyfile"
    config.write_text("{\n admin off\n auto_https off\n persist_config off\n frankenphp {\n num_threads 1\n max_threads 1\n }\n}\n"
                      + f"http://127.0.0.1:{port} {{\n root * {json.dumps(str(directory))}\n php_server {{\n file_server off\n }}\n}}\n")
    return [args.frankenphp, "run", "--config", str(config), "--adapter", "caddyfile"]


@contextlib.contextmanager
def measurement(name, scenario, repetition, args, output):
    label = f"{scenario}-{repetition}-{name}"
    directory = output / label
    directory.mkdir()
    token = secrets.token_hex(24)
    script = ROOT / "scripts" / SCENARIOS[scenario]
    entry = prepare_script(directory, script, token)
    (directory / "php.ini").write_text("\n".join(f"{key}={value}" for key, value in INI.items()) + "\n")
    env = runtime_env(name)
    env.update(PHPRC=str(directory / "php.ini"), PHP_INI_SCAN_DIR=str(directory / "no-ini-scan"),
               GOMAXPROCS=str(args.go_max_procs))
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    url = f"http://127.0.0.1:{port}/index.php"
    command = server_command(name, args, directory, entry, port)
    with owned(command, directory / "server.log", env=env, args=args) as process:
        probe = ready(process, url, token)
        status, payload = get(url)
        if status != 200:
            raise RuntimeError(f"scenario returned status {status}")
        probe.update({"payload_sha256": hashlib.sha256(payload).hexdigest(), "processes": 1, "php_workers": 1})
        write_json(directory / "probe.json", probe)
        yield {"runtime": name, "scenario": scenario, "repetition": repetition, "probe": probe,
               "server_command": command, "server_environment": {key: env[key] for key in ("PHPRC", "PHP_INI_SCAN_DIR", "GOMAXPROCS")},
               "script_sha256": digest(script), "server_log": str(directory / "server.log")}, process, url, directory


def main():
    parser = argparse.ArgumentParser(description=__doc__, epilog="Default compares BOTH runtimes; --runtime restphp explicitly requests a non-comparative run. Comparison requires finite Docker CPU/memory limits or Linux --cpus/--memory-bytes. Results never claim dedicated-host isolation.")
    parser.add_argument("--restphp", default="target/release/restphp")
    parser.add_argument("--frankenphp", default="frankenphp")
    parser.add_argument("--wrk", default="wrk")
    parser.add_argument("--runtime", choices=("compare", "restphp", "frankenphp"), default="compare")
    parser.add_argument("--scenario", choices=tuple(SCENARIOS), action="append")
    parser.add_argument("--duration", type=int, default=10)
    parser.add_argument("--warmup", type=int, default=2)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--threads", type=int, default=2)
    parser.add_argument("--connections", type=int, default=20)
    parser.add_argument("--open-files", type=int, default=1024)
    parser.add_argument("--memory-bytes", type=int)
    parser.add_argument("--cpus", type=lambda value: [int(cpu) for cpu in value.split(",")])
    parser.add_argument("--go-max-procs", type=int, default=2)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    for key in ("duration", "warmup", "repetitions", "threads", "connections", "open_files", "go_max_procs"):
        if getattr(args, key) <= 0:
            parser.error(f"--{key.replace('_', '-')} must be positive")
    if args.connections < args.threads:
        parser.error("connections must be at least threads")
    if args.memory_bytes is not None and args.memory_bytes <= 0:
        parser.error("--memory-bytes must be positive")
    if args.cpus is not None and (not args.cpus or min(args.cpus) < 0):
        parser.error("--cpus must be a comma-separated list of nonnegative CPU ids")
    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, interrupted)
    output = (args.output or ROOT / "results" / (time.strftime("%Y%m%dT%H%M%S") + "-" + secrets.token_hex(3))).resolve()
    output.mkdir(parents=True, exist_ok=False)
    result = {"schema_version": 2, "provenance": "measured", "status": "running", "comparative": args.runtime == "compare",
              "mode": "classic per-request execution, one server process and one PHP VM per runtime",
              "environment": {"platform": platform.platform(), "architecture": platform.machine(), "python": sys.version,
                              "cpu_count": os.cpu_count(), "load_average_start": os.getloadavg(), "available_tooling": tooling()},
              "settings": vars(args) | {"output": str(output)}, "runtimes": {}, "runs": []}
    result["settings"]["output"] = str(output)
    try:
        names = ("restphp", "frankenphp") if args.runtime == "compare" else (args.runtime,)
        active = ["wrk", *names]
        print("Available tooling:", json.dumps(result["environment"]["available_tooling"]))
        print("Active tooling:", ", ".join(active), "; other installed tools are NOT active comparison participants")
        result["environment"]["resource_limits"] = limits(args)
        for name in active:
            resolved = shutil.which(getattr(args, name))
            result["runtimes"][name] = {"binary": getattr(args, name), "sha256": digest(getattr(args, name)),
                                       "version": version(name, [getattr(args, name), "--version" if name != "frankenphp" else "version"])}
        for scenario in args.scenario or SCENARIOS:
            # Establish exact compatibility BEFORE any warmup or measurements.
            probes = {}
            for name in names:
                with measurement(name, scenario, "preflight", args, output) as prepared:
                    record, process, url, directory = prepared
                    probes[name] = comparable(record["probe"])
            baseline = probes[names[0]]
            for probe in probes.values():
                verify_pair(baseline, probe)
            result.setdefault("preflight", {})[scenario] = probes
            for repetition in range(1, args.repetitions + 1):
                # Alternate order to reduce monotonic temperature/time bias.
                order = names if repetition % 2 else tuple(reversed(names))
                for name in order:
                    with measurement(name, scenario, repetition, args, output) as prepared:
                        record, process, url, directory = prepared
                        verify_pair(baseline, comparable(record["probe"]))
                        warmup = run_load(args, process, url, args.warmup, directory / "warmup.log")
                        record["warmup"] = warmup
                        if warmup["errors"]["total"]:
                            record["status"] = "invalid-warmup"
                            result["runs"].append(record)
                            raise InvalidResult(f"warmup had errors: {directory}")
                        record["measurement"] = run_load(args, process, url, args.duration, directory / "wrk.log")
                        record["status"] = "valid" if not record["measurement"]["errors"]["total"] else "invalid-errors"
                        result["runs"].append(record)
                        write_json(output / "results.json", result)
                        if record["status"] != "valid":
                            raise InvalidResult(f"measurement had errors: {directory}")
                        print(name, scenario, repetition, json.dumps(record["measurement"]["latency_ms"]), "ms")
        result["status"] = "complete"
        return 0
    except (Exception, KeyboardInterrupt) as error:
        result["status"] = "failed"
        result["failure"] = str(error)
        print(f"Benchmark refused/failed: {error}", file=sys.stderr)
        return 1
    finally:
        for process in list(CHILDREN):
            stop(process)
        result["environment"]["load_average_end"] = os.getloadavg()
        write_json(output / "results.json", result)
        print(f"Evidence: {output / 'results.json'}")


if __name__ == "__main__":
    sys.exit(main())
