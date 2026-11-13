#!/usr/bin/env python3
"""Real TLS-ALPN-01 issuance, cache reuse and renewal with official Pebble v2.8.0.

Prerequisites (Go >= 1.24):
  go install github.com/letsencrypt/pebble/v2/cmd/pebble@v2.8.0
  go install github.com/letsencrypt/pebble/v2/cmd/pebble-challtestsrv@v2.8.0
  python3 tests/run_acme_e2e.py --binary target/debug/restphp

No root, hosts-file changes, Docker, system trust changes or validation bypass.
The optional public mode requires externally owned DNS pointing to this machine,
reachable inbound TCP/443, and an explicit Let's Encrypt staging root PEM. Public
mode tests issuance/cache reuse only: a normal staging lifetime cannot renew in
this harness's finite local-CA deadline.
"""

import argparse
import contextlib
import hashlib
import http.client
import json
import os
from pathlib import Path
import re
import signal
import socket
import ssl
import subprocess
import sys
import tempfile
import time
import urllib.request


ROOT = Path(__file__).resolve().parents[1]
PEBBLE_VERSION = "v2.8.0"
HOSTNAME = "restphp.acme.test"


def require(condition, message):
    if not condition:
        raise AssertionError(message)


class Process:
    def __init__(self, command, log, env=None):
        self.path = log
        self.output = log.open("wb")
        if env is None:
            env = dict(os.environ, RUST_LOG="info")
        self.child = subprocess.Popen(
            [str(arg) for arg in command], cwd=ROOT, stdout=self.output,
            stderr=subprocess.STDOUT, env=env, start_new_session=True,
        )

    def text(self):
        return self.path.read_text(errors="replace")

    def alive(self):
        require(self.child.poll() is None,
                f"process exited ({self.child.returncode}): {self.path}\n{self.text()}")

    def stop(self, successful=False):
        if self.child.poll() is None:
            os.killpg(self.child.pid, signal.SIGTERM)
            try:
                self.child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(self.child.pid, signal.SIGKILL)
                self.child.wait(timeout=5)
                if successful:
                    raise AssertionError(f"process did not drain: {self.path}")
        self.output.close()
        if successful:
            require(self.child.returncode == 0,
                    f"unclean server shutdown: {self.path}\n{self.text()}")


@contextlib.contextmanager
def running(command, log, env=None):
    process = Process(command, log, env)
    try:
        yield process
    finally:
        process.stop()


def port(dns=False):
    with socket.socket() as tcp:
        tcp.bind(("127.0.0.1", 0))
        number = tcp.getsockname()[1]
        if dns:
            with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as udp:
                udp.bind(("127.0.0.1", number))
        return number


def wait_for(description, action, processes, timeout=30):
    deadline = time.monotonic() + timeout
    error = None
    while time.monotonic() < deadline:
        for process in processes:
            process.alive()
        try:
            value = action()
            if value:
                return value
        except (OSError, http.client.HTTPException, ValueError) as caught:
            error = caught
        time.sleep(0.1)
    raise AssertionError(f"timed out waiting for {description}; last error: {error}")


def api(url, trust=None, data=None):
    context = ssl.create_default_context(cafile=str(trust)) if trust else None
    request = urllib.request.Request(
        url, data=json.dumps(data).encode() if data is not None else None,
        headers={"Content-Type": "application/json"},
    )
    opener = urllib.request.build_opener(
        urllib.request.ProxyHandler({}), urllib.request.HTTPSHandler(context=context),
    )
    with opener.open(request, timeout=3) as response:
        return response.read()


def probe(hostname, port_number, trust):
    context = ssl.create_default_context(cafile=str(trust))
    context.set_alpn_protocols(["http/1.1"])
    with socket.create_connection(("127.0.0.1", port_number), timeout=3) as tcp:
        with context.wrap_socket(tcp, server_hostname=hostname) as secure:
            certificate = secure.getpeercert()
            fingerprint = hashlib.sha256(secure.getpeercert(binary_form=True)).hexdigest()
            secure.sendall((
                f"GET /?acme=verified HTTP/1.1\r\nHost: {hostname}:{port_number}\r\n"
                "Connection: close\r\n\r\n"
            ).encode())
            response = http.client.HTTPResponse(secure)
            response.begin()
            body = json.loads(response.read())
            require(response.status == 200 and body["status"] == "ok", "HTTPS PHP response failed")
            require(body["server"]["HTTPS"] == "on", "PHP did not see HTTPS")
            require(body["get"]["acme"] == "verified", "HTTPS request did not reach PHP")
            return {
                "sha256": fingerprint, "serial": certificate["serialNumber"],
                "not_before": ssl.cert_time_to_seconds(certificate["notBefore"]),
                "not_after": ssl.cert_time_to_seconds(certificate["notAfter"]),
            }


def restphp_command(args, port_number, cache, hostname=HOSTNAME, directory=None, ca=None):
    command = [args.binary, "serve", "--host", "0.0.0.0" if args.public_domain else "127.0.0.1",
               "--port", str(port_number), "--entrypoint", "tests/fixtures/info.php",
               "--acme-domain", hostname, "--acme-contact", args.contact,
               "--acme-cache", str(cache)]
    if directory:
        command += ["--acme-directory", directory]
    else:
        command += ["--acme-staging"]
    if ca:
        command += ["--acme-ca-cert", str(ca)]
    return command


def cached(cache, prefix):
    paths = list(cache.glob(prefix + "*"))
    require(len(paths) == 1, f"expected one {prefix} in {cache}, found {len(paths)}")
    return paths[0].read_bytes()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def accounts(pebble):
    return len(re.findall(r"There are now \d+ accounts in memory", pebble.text()))


def assert_no_certificate(cache):
    require(not list(cache.glob("cached_cert_*")), "failed validation unexpectedly cached a certificate")


def negative_cases(args, work, port_number, directory, api_ca, dns_api, pebble, dns):
    result = {}
    empty_ca = work / "empty-ca.pem"
    empty_ca.write_text("")
    for flags, expected in [
        (["--acme-staging"], "--acme-directory cannot be combined"),
        (["--acme-directory", "http://localhost/dir"], "must be an absolute HTTPS URL"),
        (["--acme-ca-cert", str(empty_ca)], "must contain at least one CA certificate"),
    ]:
        command = restphp_command(args, port_number, work / "invalid-options", directory=directory)
        if "http://localhost/dir" in flags:
            command = command[:-2]
        output = subprocess.run(command + flags, cwd=ROOT, capture_output=True, timeout=10)
        text = (output.stdout + output.stderr).decode(errors="replace")
        require(output.returncode != 0 and expected in text, f"invalid flags accepted: {text}")
        require("Listening on" not in text, "invalid flags reached listener startup")
    result["conflicting_and_insecure_flags_rejected"] = True

    cache = work / "untrusted-cache"
    with running(restphp_command(args, port_number, cache, directory=directory),
                 work / "untrusted.log") as server:
        wait_for("ACME CA trust failure", lambda: "UnknownIssuer" in server.text(),
                 [server, pebble, dns])
        assert_no_certificate(cache)
        server.stop(successful=True)
    result["untrusted_acme_api_rejected"] = True

    api(dns_api + "/set-default-ipv4", data={"ip": "127.0.0.2"})
    cache = work / "wrong-route-cache"
    try:
        with running(restphp_command(args, port_number, cache, directory=directory, ca=api_ca),
                     work / "wrong-route.log") as server:
            wait_for(
                "failed real TLS-ALPN-01 route",
                lambda: "set INVALID by completed challenge" in pebble.text()
                and "ACME certificate provisioning or renewal failed" in server.text(),
                [server, pebble, dns],
            )
            assert_no_certificate(cache)
            server.stop(successful=True)
    finally:
        api(dns_api + "/set-default-ipv4", data={"ip": "127.0.0.1"})
    result["wrong_challenge_route_rejected"] = True
    return result


def lifecycle(args, work, port_number, hostname, trust, directory=None, api_ca=None,
              pebble=None, dns=None, dns_api=None):
    cache = work / "cache"
    peers = [peer for peer in [pebble, dns] if peer]
    command = restphp_command(args, port_number, cache, hostname, directory, api_ca)
    with running(command, work / "issuance.log") as server:
        initial = wait_for("trusted hostname-matching issuance",
                           lambda: probe(hostname, port_number, trust), [server] + peers,
                           timeout=args.issuance_timeout)
        wait_for("certificate and account cache writes",
                 lambda: "CertCacheStore" in server.text() and list(cache.glob("cached_account_*")),
                 [server] + peers)
        account = cached(cache, "cached_account_")
        certificate_cache = cached(cache, "cached_cert_")
        if pebble:
            require("set VALID by completed challenge" in pebble.text(), "CA did not validate challenge")
            history = json.loads(api(dns_api + "/dns-request-history", data={"host": hostname}))
            require(history, "CA did not query the local DNS server")
            require(100 <= initial["not_after"] - initial["not_before"] <= 120,
                    "Pebble did not issue the configured short-lived certificate")
            try:
                probe(hostname, port_number, api_ca)
            except ssl.SSLCertVerificationError:
                pass
            else:
                raise AssertionError("issued HTTPS certificate was accepted with unrelated CA trust")
        server.stop(successful=True)

    before_accounts = accounts(pebble) if pebble else None
    before_issued = pebble.text().count("Issued certificate serial") if pebble else None
    with running(command, work / "restart-renewal.log") as server:
        restarted = wait_for("cached certificate after restart",
                             lambda: probe(hostname, port_number, trust), [server] + peers, timeout=10)
        require(restarted == initial, "restart did not reuse the same valid certificate")
        require(cached(cache, "cached_account_") == account, "restart changed ACME account key")
        require(cached(cache, "cached_cert_") == certificate_cache, "restart rewrote certificate cache")
        require("DeployedCachedCert" in server.text(), "server did not report cached certificate deployment")
        if pebble:
            require(accounts(pebble) == before_accounts, "restart registered a new account")
            require(pebble.text().count("Issued certificate serial") == before_issued,
                    "restart issued a new certificate instead of reusing cache")
        result = {"initial": initial, "restart": restarted, "account_key_sha256": digest(account)}
        if not pebble:
            server.stop(successful=True)
            result["renewal"] = "not exercised: public staging uses normal certificate lifetimes"
            return result

        expected_renewal = initial["not_after"] - (initial["not_after"] - initial["not_before"]) / 3
        deadline = time.monotonic() + args.renewal_timeout
        samples = 0
        renewed = None
        observed_at = None
        stable_until = None
        while time.monotonic() < deadline:
            server.alive()
            for peer in peers:
                peer.alive()
            current = probe(hostname, port_number, trust)
            samples += 1
            if current["sha256"] != initial["sha256"]:
                if renewed is None:
                    renewed, observed_at = current, time.time()
                    stable_until = time.monotonic() + 2
                require(current == renewed, "unexpected additional renewal during observation")
            if stable_until and time.monotonic() >= stable_until:
                break
            time.sleep(0.1)
        require(renewed is not None, "no real certificate renewal before deadline")
        require(renewed["serial"] != initial["serial"], "renewal retained the old serial")
        require(renewed["not_after"] > initial["not_after"], "renewal did not extend validity")
        require(observed_at >= expected_renewal - 2, "renewal occurred before the real certificate schedule")
        require(observed_at < initial["not_after"], "renewal did not complete before expiry")
        wait_for("renewed certificate cache", lambda: cached(cache, "cached_cert_") != certificate_cache,
                 [server] + peers)
        require(cached(cache, "cached_account_") == account, "renewal changed ACME account key")
        require(accounts(pebble) == before_accounts, "renewal registered a different ACME account")
        require(pebble.text().count("Issued certificate serial") == before_issued + 1,
                "expected exactly one renewal, not a renewal storm")
        require(pebble.text().count("set VALID by completed challenge") >= 2,
                "renewal did not perform a fresh real challenge")
        require("DeployedNewCert" in server.text(), "renewed certificate was not deployed")
        server.stop(successful=True)
        result.update({"renewed": renewed, "renewal_observed_at": observed_at,
                       "scheduled_renewal_at": expected_renewal,
                       "continuous_verified_https_samples": samples,
                       "sample_interval_seconds": 0.1, "failed_samples": 0})
        return result


def generate_acme_api_tls(work):
    ca_key, api_ca = work / "acme-api-ca.key", work / "acme-api-ca.pem"
    leaf_key, leaf_csr, leaf_cert = work / "acme-api.key", work / "acme-api.csr", work / "acme-api.pem"
    ext = work / "acme-api.ext"
    ext.write_text(
        "basicConstraints=critical,CA:FALSE\n"
        "keyUsage=critical,digitalSignature,keyEncipherment\n"
        "extendedKeyUsage=serverAuth\n"
        "subjectKeyIdentifier=hash\n"
        "authorityKeyIdentifier=keyid,issuer\n"
        "subjectAltName=DNS:localhost,IP:127.0.0.1\n"
    )
    subprocess.run(
        ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "2",
         "-subj", "/CN=RestPHP Test ACME API Root",
         "-addext", "basicConstraints=critical,CA:TRUE",
         "-addext", "keyUsage=critical,keyCertSign,cRLSign",
         "-addext", "subjectKeyIdentifier=hash",
         "-keyout", str(ca_key), "-out", str(api_ca)],
        check=True, capture_output=True, text=True,
    )
    subprocess.run(
        ["openssl", "req", "-newkey", "rsa:2048", "-nodes", "-subj", "/CN=localhost",
         "-keyout", str(leaf_key), "-out", str(leaf_csr)],
        check=True, capture_output=True, text=True,
    )
    subprocess.run(
        ["openssl", "x509", "-req", "-in", str(leaf_csr), "-CA", str(api_ca),
         "-CAkey", str(ca_key), "-CAcreateserial", "-days", "2",
         "-extfile", str(ext), "-out", str(leaf_cert)],
        check=True, capture_output=True, text=True,
    )
    return api_ca, leaf_cert, leaf_key


def local_run(args, work):
    for binary, package in [
        (args.pebble, "github.com/letsencrypt/pebble/v2/cmd/pebble"),
        (args.challtestsrv, "github.com/letsencrypt/pebble/v2/cmd/pebble-challtestsrv"),
    ]:
        metadata = subprocess.check_output(["go", "version", "-m", str(binary)], text=True)
        require(f"path\t{package}" in metadata and
                f"mod\tgithub.com/letsencrypt/pebble/v2\t{PEBBLE_VERSION}\t" in metadata,
                f"expected official go install {package}@{PEBBLE_VERSION}: {metadata}")
    api_ca, api_cert, api_key = generate_acme_api_tls(work)
    dns_port, dns_management = port(dns=True), port()
    acme_port, management, tls_port = port(), port(), port()
    dns_api = f"http://127.0.0.1:{dns_management}"
    directory = f"https://localhost:{acme_port}/dir"
    config = json.loads((ROOT / "tests/acme/pebble-config.json").read_text())
    config["pebble"].update({
        "listenAddress": f"127.0.0.1:{acme_port}",
        "managementListenAddress": f"127.0.0.1:{management}", "tlsPort": tls_port,
        "certificate": str(api_cert),
        "privateKey": str(api_key),
    })
    config_file = work / "pebble-config.json"
    config_file.write_text(json.dumps(config, indent=2))
    dns_command = [args.challtestsrv, "-dns01", f"127.0.0.1:{dns_port}",
                   "-management", f"127.0.0.1:{dns_management}", "-defaultIPv4", "127.0.0.1",
                   "-defaultIPv6", "", "-http01", "", "-https01", "", "-tlsalpn01", "", "-doh", ""]
    env = {key: value for key, value in os.environ.items() if not key.startswith("PEBBLE_")}
    env.update({"PEBBLE_VA_NOSLEEP": "1", "PEBBLE_WFE_NONCEREJECT": "0", "PEBBLE_AUTHZREUSE": "0"})
    with running(dns_command, work / "dns.log") as dns:
        wait_for("DNS management", lambda: api(dns_api + "/set-default-ipv4", data={"ip": "127.0.0.1"}) == b"", [dns])
        with running([args.pebble, "-config", config_file, "-strict=false", "-dnsserver", f"127.0.0.1:{dns_port}"],
                     work / "pebble.log", env) as pebble:
            wait_for("trusted Pebble directory", lambda: api(directory, api_ca), [pebble, dns])
            issued_ca = work / "issued-root.pem"
            issued_ca.write_bytes(api(f"https://localhost:{management}/roots/0", api_ca))
            failures = negative_cases(args, work, tls_port, directory, api_ca, dns_api, pebble, dns)
            result = lifecycle(args, work, tls_port, HOSTNAME, issued_ca, directory, api_ca, pebble, dns, dns_api)
            result["negative_cases"] = failures
            require("Skipping real validation" not in pebble.text() and "VA always returns valid" not in pebble.text(),
                    "Pebble bypassed real validation")
            result["ca"] = f"official Pebble {PEBBLE_VERSION}"
            result["hostname"] = HOSTNAME
            return result


def interrupted(number, _frame):
    raise SystemExit(128 + number)


def main():
    signal.signal(signal.SIGTERM, interrupted)
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/restphp")
    parser.add_argument("--pebble", type=Path, default=Path.home() / "go/bin/pebble")
    parser.add_argument("--challtestsrv", type=Path, default=Path.home() / "go/bin/pebble-challtestsrv")
    parser.add_argument("--output", type=Path, help="Retain logs, private cache, CA roots and result JSON in this new directory")
    parser.add_argument("--issuance-timeout", type=float, default=45)
    parser.add_argument("--renewal-timeout", type=float, default=115)
    parser.add_argument("--public-domain", help="Optional externally owned DNS name; binds TCP/443")
    parser.add_argument("--public-root-ca", type=Path, help="Explicit trusted Let's Encrypt staging issuance root PEM")
    parser.add_argument("--contact", default="admin@example.test")
    args = parser.parse_args()
    args.binary = args.binary.resolve()
    if args.public_domain:
        if not args.public_root_ca or args.contact == "admin@example.test":
            parser.error("--public-domain requires --public-root-ca and a real --contact email")
    elif args.public_root_ca:
        parser.error("--public-root-ca requires --public-domain")
    with contextlib.ExitStack() as stack:
        if args.output:
            work = args.output.resolve()
            work.mkdir(mode=0o700, parents=True, exist_ok=False)
        else:
            work = Path(stack.enter_context(tempfile.TemporaryDirectory(prefix="restphp-acme-")))
        try:
            if args.public_domain:
                result = lifecycle(args, work, 443, args.public_domain, args.public_root_ca)
                result["ca"] = "Let's Encrypt staging"
            else:
                result = local_run(args, work)
            result["passed"] = True
            (work / "result.json").write_text(json.dumps(result, indent=2) + "\n")
            print(json.dumps(result, indent=2))
        except Exception:
            for log in sorted(work.glob("*.log")):
                print(f"\n--- {log.name} ---\n{log.read_text(errors='replace')}", file=sys.stderr)
            raise


if __name__ == "__main__":
    main()
