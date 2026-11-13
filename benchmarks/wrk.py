"""Parse observed wrk output; absent percentiles are never estimated."""

import math
import re


class InvalidResult(ValueError):
    pass


NUMBER = r"(?:\d+(?:\.\d*)?|\.\d+)"
LATENCY = re.compile(rf"({NUMBER})(ns|us|µs|ms|s|m)")


def latency_ms(value):
    match = LATENCY.fullmatch(value)
    if not match:
        raise InvalidResult(f"invalid latency: {value!r}")
    number = float(match[1])
    scale = {"ns": 0.000001, "us": 0.001, "µs": 0.001, "ms": 1, "s": 1000, "m": 60000}
    result = number * scale[match[2]]
    if not math.isfinite(result):
        raise InvalidResult("non-finite latency")
    return result


def one(pattern, text, label, optional=False):
    matches = re.findall(pattern, text, flags=re.MULTILINE)
    if len(matches) != 1:
        if optional and not matches:
            return None
        raise InvalidResult(f"expected one {label}, found {len(matches)}")
    return matches[0]


def parse_wrk(text, require_percentiles=True, require_status_counter=False):
    summary = one(rf"^\s*(\d+) requests in ({NUMBER})(ms|s|m), .+ read\s*$", text, "request summary")
    requests = int(summary[0])
    duration = latency_ms(summary[1] + summary[2]) / 1000
    rate = float(one(rf"^Requests/sec:\s*({NUMBER})\s*$", text, "throughput"))
    if requests <= 0 or duration <= 0 or rate <= 0 or not math.isfinite(rate):
        raise InvalidResult("request count, duration and throughput must be positive and finite")
    # wrk rounds elapsed time and throughput independently.
    if abs(rate - requests / duration) > max(rate * 0.025, 1):
        raise InvalidResult("throughput disagrees with request count/duration")
    percentiles = {}
    for percentile in (50, 95, 99):
        value = one(rf"^\s*{percentile}%\s+(\S+)\s*$", text, f"p{percentile}", optional=True)
        if value is None and require_percentiles:
            raise InvalidResult(f"p{percentile} unavailable; rerun with benchmarks/percentiles.lua")
        if value is None and re.search(rf"^\s*{percentile}%(?:\s|$)", text, flags=re.MULTILINE):
            raise InvalidResult(f"malformed p{percentile}")
        percentiles[f"p{percentile}"] = latency_ms(value) if value is not None else None
    observed = [value for value in percentiles.values() if value is not None]
    if observed != sorted(observed):
        raise InvalidResult("percentiles are not monotonic")
    socket = one(r"^\s*Socket errors: connect (\d+), read (\d+), write (\d+), timeout (\d+)\s*$", text, "socket errors", optional=True)
    errors = dict(zip(("connect", "read", "write", "timeout"), map(int, socket or (0, 0, 0, 0))))
    status = one(r"^\s*Non-2xx or 3xx responses:\s*(\d+)\s*$", text, "HTTP status errors", optional=True)
    non_2xx = one(r"^Benchmark non-2xx responses:\s*(\d+)\s*$", text, "non-2xx counter", optional=not require_status_counter)
    errors["http_non_2xx_or_3xx"] = int(status or 0)
    errors["http_non_2xx"] = int(non_2xx) if non_2xx is not None else None
    if errors["http_non_2xx"] is not None and errors["http_non_2xx"] < errors["http_non_2xx_or_3xx"]:
        raise InvalidResult("HTTP error counters disagree")
    if max(errors["http_non_2xx_or_3xx"], errors["http_non_2xx"] or 0) > requests:
        raise InvalidResult("HTTP errors exceed completed requests")
    # Reject malformed counter lines instead of interpreting them as zero.
    for label, parsed in (("Socket errors:", socket), ("Non-2xx or 3xx responses:", status),
                          ("Benchmark non-2xx responses:", non_2xx)):
        if label in text and parsed is None:
            raise InvalidResult(f"malformed {label}")
    errors["total"] = sum(errors[key] for key in ("connect", "read", "write", "timeout")) + max(errors["http_non_2xx_or_3xx"], errors["http_non_2xx"] or 0)
    return {"requests": requests, "duration_seconds": duration, "requests_per_second": rate,
            "latency_ms": percentiles, "errors": errors}
