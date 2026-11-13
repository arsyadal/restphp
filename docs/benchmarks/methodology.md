# Benchmark Methodology & Reproducibility

RestPHP uses a fail-closed benchmark runner (`benchmarks/run.py`) and Lua histogram script (`benchmarks/percentiles.lua`). It never promotes mean latency to p99: missing or malformed percentile output fails the run, or remains `null` in historical unverified records.

## Controlled Container Comparison

Build the self-contained comparison image from the repository root. It extracts the comparator's exact `PHP_VERSION` from FrankenPHP, compiles a matching NTS `libphp` embed library from that PHP release, and builds RestPHP in release mode:

```bash
docker build -f benchmarks/Dockerfile -t restphp-comparison .
docker run --rm --init --network none --cpus 2 --memory 2g --pids-limit 128 \
  -v "$PWD/benchmarks/results:/app/benchmarks/results" restphp-comparison
```

Before running warmup or measurements, `benchmarks/run.py` probes both runtimes over HTTP and refuses to compare unless `php_version`, CPU architecture, integer width, PHP INI settings, response payload SHA-256, process count (`1`), and PHP worker count (`1`) match. Runtime-specific differences (`PHP_SAPI`, NTS vs ZTS, loaded extensions) are recorded in the result JSON rather than hidden.

## Local Container Measurement (PHP 8.2.35, 2 CPUs, 2 GiB RAM)

Measured sequentially in `restphp-comparison` on `Linux-6.12.76-linuxkit-aarch64` (`--cpus 2 --memory 2g --pids-limit 128`, `wrk` 4.1.0, 2 threads, 20 connections, 2s warmup, 3 × 10s repetitions, OPcache/JIT disabled, zero socket/HTTP errors). The load generator runs in the same cgroup, so treat these figures as a controlled container comparison, not a dedicated-host capacity ceiling:

| Scenario | Runtime | Median Req/sec | Median p50 (ms) | Median p95 (ms) | Median p99 (ms) | Peak Sampled RSS (MiB) | Errors |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `hello_plaintext` | RestPHP | 2,458.95 | 7.601 | 11.629 | 16.462 | 13.68 | 0 |
| `hello_plaintext` | FrankenPHP v1.13.4 | 981.66 | 19.188 | 46.620 | 88.311 | 77.91 | 0 |
| `json_api` | RestPHP | 2,269.51 | 7.936 | 14.010 | 39.398 | 13.69 | 0 |
| `json_api` | FrankenPHP v1.13.4 | 1,051.84 | 18.364 | 28.590 | 54.708 | 77.93 | 0 |
| `heavy_compute` | RestPHP | 696.57 | 24.412 | 78.733 | 123.170 | 13.52 | 0 |
| `heavy_compute` | FrankenPHP v1.13.4 | 497.69 | 31.958 | 113.087 | 182.632 | 78.19 | 0 |

## Rendering Reports

```bash
php benchmarks/report.php benchmarks/results/results.json
```

`benchmarks/results/results.json` retains pre-existing historical outputs as `historical-unverified` with `null` percentiles because their original `wrk` outputs did not include a full p50/p95/p99 histogram or environment provenance. New runs write schema-v2 evidence directories with raw `wrk` logs, server logs, preflight probes, and sampled RSS.
