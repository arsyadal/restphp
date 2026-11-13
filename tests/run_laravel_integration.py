#!/usr/bin/env python3
"""Install real Laravel/Octane packages and exercise native and Artisan lifecycles."""

import argparse
import base64
import contextlib
import hashlib
import http.client
import json
import os
from pathlib import Path
import secrets
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
SUPPORTED = {"8.2": (10, 11, 12), "8.3": (10, 11, 12), "8.4": (11, 12)}
VERSIONS = {
    10: {"skeleton": "10.3.3", "framework": "10.50.4"},
    11: {"skeleton": "11.6.1", "framework": "11.57.1"},
    12: {"skeleton": "12.12.2", "framework": "12.70.2"},
}
OCTANE_VERSION = "2.22.0"


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def wait_for(description, predicate, timeout=30):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = predicate()
        if value:
            return value
        time.sleep(0.05)
    raise TimeoutError(description)


def listening(port):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=0.2):
            return True
    except OSError:
        return False


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def run(command, cwd, env, timeout=600, check=True):
    result = subprocess.run(command, cwd=cwd, env=env, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
    if check and result.returncode:
        raise RuntimeError(f"Command failed ({result.returncode}): {command}\n{result.stdout}")
    return result


def php_details(php, env):
    code = 'echo json_encode(["version"=>PHP_VERSION,"binary"=>PHP_BINARY,"extensions"=>get_loaded_extensions(),"zts"=>PHP_ZTS]);'
    return json.loads(run([php, "-r", code], ROOT, env).stdout)


def install(app, major, php, composer, env):
    require(not app.exists() or not any(app.iterdir()), f"Installation directory must be empty: {app}")
    app.parent.mkdir(parents=True, exist_ok=True)
    run([php, composer, "create-project", "laravel/laravel", str(app), VERSIONS[major]["skeleton"],
         "--prefer-dist", "--no-dev", "--no-interaction", "--no-scripts", "--no-install"], ROOT, env)
    manifest = app / "composer.json"
    config = json.loads(manifest.read_text())
    config.setdefault("repositories", []).insert(0, {
        "type": "path", "url": str(ROOT / "octane"), "options": {"symlink": False},
    })
    config["require"]["restphp/octane"] = "@dev"
    config["require"]["laravel/framework"] = VERSIONS[major]["framework"]
    config["require"]["laravel/octane"] = OCTANE_VERSION
    # Historical supported majors still need security regressions in CI even
    # after upstream support ends; never apply this policy to user projects.
    config.setdefault("config", {}).setdefault("audit", {})["block-insecure"] = False
    manifest.write_text(json.dumps(config, indent=4) + "\n")
    shutil.copyfile(ROOT / "tests/laravel/AppServiceProvider.php", app / "app/Providers/AppServiceProvider.php")
    key = base64.b64encode(secrets.token_bytes(32)).decode()
    (app / ".env").write_text(
        f"APP_NAME=RestPHPRegression\nAPP_ENV=testing\nAPP_KEY=base64:{key}\n"
        "APP_DEBUG=false\nAPP_URL=http://localhost\nLOG_CHANNEL=stderr\n"
        "DB_CONNECTION=sqlite\nDB_DATABASE=:memory:\nCACHE_DRIVER=array\n"
        "CACHE_STORE=array\nSESSION_DRIVER=array\nQUEUE_CONNECTION=sync\n"
    )
    run([php, composer, "update", "--prefer-dist", "--no-dev", "--no-interaction",
         "--no-progress", "--no-scripts"], app, env)
    run([php, "artisan", "package:discover", "--ansi"], app, env)
    (app / ".restphp-regression.json").write_text(json.dumps({"laravel": major}))


class OwnedProcess:
    def __init__(self, command, app, env, log_name):
        self.log_path = app / "storage/logs" / log_name
        self.log = self.log_path.open("w+")
        self.process = subprocess.Popen(command, cwd=app, env=env, stdout=self.log,
                                        stderr=subprocess.STDOUT, start_new_session=True)

    def ready(self, port):
        def check():
            if self.process.poll() is not None:
                raise RuntimeError(f"Process exited before readiness: {self.log_path.read_text()}")
            return listening(port)
        wait_for("listener did not become ready", check)

    def exited(self):
        return self.process.poll() is not None

    def finish(self):
        wait_for("owned process did not stop", self.exited)
        require(self.process.returncode == 0,
                f"Unclean exit {self.process.returncode}: {self.log_path.read_text()}")

    def close(self):
        # A supervisor can exit with a child still alive. Its process group is
        # ours even after the leader exits, so always clean the entire group.
        try:
            os.killpg(self.process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass
        try:
            os.killpg(self.process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        self.process.wait(timeout=5)
        self.log.close()

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()


def request(port, path="/regression/probe", method="GET", body=None, headers=None):
    client = http.client.HTTPConnection("127.0.0.1", port, timeout=10)
    try:
        client.request(method, path, body=body, headers={"Accept": "application/json", **(headers or {})})
        response = client.getresponse()
        payload = response.read()
        return response.status, {key.lower(): value for key, value in response.getheaders()}, payload
    finally:
        client.close()


def probe(port, mutate=False):
    body = '{"value":"first"}' if mutate else None
    status, headers, payload = request(
        port, "/regression/probe?mutate=1" if mutate else "/regression/probe",
        "POST" if mutate else "GET", body,
        {"Content-Type": "application/json", "Cookie": "seed=first", "X-Regression": "first"} if mutate else {},
    )
    require(status == (201 if mutate else 200), f"Unexpected probe HTTP {status}: {payload!r}")
    data = json.loads(payload)
    require(data["method"] == ("POST" if mutate else "GET"), f"Wrong request method: {data}")
    require(data["body"] == (body or ""), f"Stale request body: {data}")
    for key in ("input", "cookie", "header"):
        require(data[key] == ("first" if mutate else None), f"Stale request {key}: {data}")
    require(data["scope_same"] is True and data["scope_uses"] == 2, f"Scoped resolution failed: {data}")
    if mutate:
        require(headers.get("x-regression-only") == "first", "Response header missing")
        require("regression=first" in headers.get("set-cookie", ""), "Response cookie missing")
    else:
        require("x-regression-only" not in headers and "set-cookie" not in headers,
                f"Response headers/cookies leaked: {headers}")
    return data


def same_worker(first, second):
    require((first["pid"], first["boot"]) == (second["pid"], second["boot"]), "Worker rebooted unexpectedly")
    require(first["scope"] != second["scope"], "Scoped service survived a request boundary")


def exercise_requests(port):
    first = probe(port, True)
    second = probe(port)
    same_worker(first, second)
    status, _, _ = request(port, "/regression/exception")
    require(status == 500, f"Laravel kernel did not render ordinary exception: HTTP {status}")
    recovered = probe(port)
    same_worker(first, recovered)
    require(second["scope"] != recovered["scope"], "Scope leaked after ordinary exception")
    return first


def boots(app):
    path = app / "storage/logs/regression-boots.jsonl"
    return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []


def no_listener(port):
    wait_for("stale listener after shutdown", lambda: not listening(port))
    with socket.socket() as sock:
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind(("127.0.0.1", port))


def process_gone(pid):
    try:
        os.kill(pid, 0)
        return False
    except ProcessLookupError:
        return True


def artisan(php, app, env, action, check=True):
    return run([php, "artisan", "octane:restphp", action, "--no-ansi"], app, env, timeout=30, check=check)


def state(app):
    try:
        return json.loads((app / "storage/logs/restphp-server-state.json").read_text())
    except (OSError, ValueError):
        return {}


def changed_worker(owner, app, port, previous):
    def changed():
        require(not owner.exited(), f"Supervisor exited: {owner.log_path.read_text()}")
        current = state(app)
        return current if current.get("server_pid") not in (None, previous) and listening(port) else None
    current = wait_for("supervisor did not replace worker", changed)
    wait_for("replaced native process remained alive", lambda: process_gone(previous))
    return current


def exercise(app, php, binary, env):
    log = app / "storage/logs/regression-boots.jsonl"
    log.unlink(missing_ok=True)
    port = free_port()
    with OwnedProcess([str(binary), "serve", "--host", "127.0.0.1", "--port", str(port),
                       "--workers", "1", "--max-requests", "4", "--shutdown-timeout-secs", "3",
                       "--persistent-worker", str(app / "vendor/restphp/octane/bin/restphp-worker.php")],
                      app, env, "regression-native.log") as native:
        native.ready(port)
        first = exercise_requests(port)
        require(first["pid"] == native.process.pid, "Response came from an unexpected native PID")
        native.finish()
        require(boots(app) == [{"pid": first["pid"], "boot": first["boot"]}], "Native application did not boot exactly once")
        no_listener(port)

    log.unlink(missing_ok=True)
    port = free_port()
    with OwnedProcess([php, "artisan", "octane:restphp", "start", "--host=127.0.0.1",
                       f"--port={port}", f"--binary={binary}", "--max-requests=4",
                       "--shutdown-timeout-secs=3", "--no-ansi"], app, env,
                      "regression-supervisor.log") as supervisor:
        supervisor.ready(port)
        initial_state = json.loads(artisan(php, app, env, "status").stdout)
        require(initial_state["supervisor_pid"] == supervisor.process.pid, "Wrong supervisor PID")
        supervised = exercise_requests(port)
        require(supervised["pid"] == initial_state["server_pid"], "Status reports wrong native worker")
        replacement_state = changed_worker(supervisor, app, port, supervised["pid"])
        recycled = probe(port)
        require(recycled["pid"] == replacement_state["server_pid"] and recycled["boot"] != supervised["boot"],
                "Request-limit recycle did not boot a new application")
        artisan(php, app, env, "reload")
        reloaded_state = changed_worker(supervisor, app, port, recycled["pid"])
        reloaded = probe(port)
        require(reloaded["pid"] == reloaded_state["server_pid"] and reloaded["boot"] != recycled["boot"],
                "Explicit reload did not replace the application")
        require(boots(app) == [{"pid": item["pid"], "boot": item["boot"]}
                               for item in (supervised, recycled, reloaded)],
                "Supervisor application boot count differs from process generations")
        artisan(php, app, env, "stop")
        supervisor.finish()
        wait_for("stopped native process remained alive", lambda: process_gone(reloaded["pid"]))
        no_listener(port)
        stopped = artisan(php, app, env, "status", check=False)
        require(stopped.returncode != 0, "Stopped supervisor still reports active")
        require(not state(app), "Supervisor left stale PID state")
    return {"native": first, "supervised": supervised, "recycled": recycled, "reloaded": reloaded,
            "assertions": ["boot-once", "same-pid", "fresh-scoped-services", "fresh-request-body",
                           "fresh-cookies", "fresh-request-headers", "fresh-status-and-response-headers",
                           "kernel-exception-recovery", "request-limit-recycle", "explicit-reload",
                           "explicit-stop", "no-stale-listener"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--laravel", type=int, choices=(10, 11, 12), required=True)
    parser.add_argument("--php", default=os.environ.get("PHP_BINARY", "php"))
    parser.add_argument("--composer", default=os.environ.get("COMPOSER", "composer"))
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/restphp")
    parser.add_argument("--app-dir", type=Path, help="Keep a fixture outside the repository for reuse or inspection")
    parser.add_argument("--skip-install", action="store_true", help="Reuse --app-dir installed by this runner")
    parser.add_argument("--install-only", action="store_true", help="Prepare dependencies without launching servers; requires --app-dir")
    parser.add_argument("--output", type=Path, help="Write versioned JSON results, including failures")
    args = parser.parse_args()
    env = os.environ.copy()
    # COMPOSER is our executable override, but Composer interprets that same
    # variable as an alternate composer.json filename.
    env.pop("COMPOSER", None)
    env["COMPOSER_NO_INTERACTION"] = "1"
    env["COMPOSER_NO_SECURITY_BLOCKING"] = "1"
    report = {"schema": 1, "requested_laravel": args.laravel, "passed": False}
    try:
        php = shutil.which(args.php) or args.php
        details = php_details(php, env)
        report["cli_php"] = details
        minor = ".".join(details["version"].split(".")[:2])
        require(args.laravel in SUPPORTED.get(minor, ()),
                f"Unsupported Laravel/PHP combination: Laravel {args.laravel}, PHP {minor}")
        required = {"ctype", "dom", "fileinfo", "filter", "hash", "mbstring", "openssl", "PDO",
                    "session", "tokenizer", "xml", "pcntl", "posix"}
        require(required <= set(details["extensions"]), f"Missing CLI extensions: {required - set(details['extensions'])}")
        require(not details["zts"], "Use a matching NTS PHP CLI")
        require(not args.install_only or args.app_dir is not None, "--install-only requires --app-dir")
        require(not args.skip_install or args.app_dir is not None, "--skip-install requires --app-dir")
        composer = shutil.which(args.composer) or args.composer
        with contextlib.ExitStack() as stack:
            app = args.app_dir.resolve() if args.app_dir else Path(stack.enter_context(tempfile.TemporaryDirectory(prefix="restphp-laravel-"))) / "app"
            if args.skip_install:
                marker = json.loads((app / ".restphp-regression.json").read_text())
                require(marker["laravel"] == args.laravel, "Reused fixture has wrong Laravel major")
                run([php, composer, "check-platform-reqs", "--no-dev"], app, env)
            else:
                install(app, args.laravel, php, composer, env)
            lock = (app / "composer.lock").read_bytes()
            packages = {item["name"]: item["version"] for item in json.loads(lock)["packages"]}
            report["dependencies"] = {name: packages[name] for name in ("laravel/framework", "laravel/octane", "restphp/octane")}
            require(packages["laravel/framework"].lstrip("v").split(".")[0] == str(args.laravel), "Wrong installed Laravel major")
            require(packages["laravel/octane"].lstrip("v").split(".")[0] == "2", "Wrong installed Octane major")
            report["composer_lock_sha256"] = hashlib.sha256(lock).hexdigest()
            report["fixture"] = str(app) if args.app_dir else "isolated temporary application"
            if not args.install_only:
                env["APP_BASE_PATH"] = str(app)
                env["OCTANE_SERVER"] = "restphp"
                report["scenarios"] = exercise(app, php, args.binary.resolve(), env)
                native = report["scenarios"]["native"]
                require(".".join(native["php"].split(".")[:2]) == minor, "Embedded PHP does not match the selected CLI minor")
                report["embed_php"] = native["php"]
            report["mode"] = "install-only" if args.install_only else "regression"
            report["passed"] = True
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        encoded = json.dumps(report, indent=2) + "\n"
        print(encoded, end="")
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(encoded)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
