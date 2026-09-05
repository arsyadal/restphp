# Benchmark Methodology & Reproducibility

RestPHP believes in transparent, reproducible benchmarks. All test procedures and scripts are openly available.

---

## 🛠️ Testing Environment & Hardware Specs

- **CPU**: AMD EPYC™ 64-Core Processor (2.45 GHz Base, 3.10 GHz Boost)
- **RAM**: 128 GB DDR4 ECC
- **OS**: Linux kernel 6.1 (Debian 13 x86_64)
- **Benchmarking Tool**: `wrk` v4.2.0 (12 threads, 1,000 persistent connections, 30s test duration)
- **PHP Version**: PHP 8.4.24 NTS

---

## 🔬 Benchmark Scenarios

### 1. High-Throughput JSON API Test
Tests raw routing, superglobal mapping, JSON serialization, and output buffering.

```bash
# Start RestPHP
restphp serve -p 8080 -e public/index.php

# Run wrk benchmark
wrk -t12 -c1000 -d30s --latency http://127.0.0.1:8080/
```

### 2. Laravel Octane Benchmark
Tests full framework persistent execution (Eloquent, Routing, Dependency Injection).

```bash
# Start RestPHP Octane
php artisan octane:restphp --port 8000

# Benchmark
wrk -t12 -c1000 -d30s --latency http://127.0.0.1:8000/api/users
```

---

## 📊 Measuring Latency & Memory

Memory usage is monitored using `ps` resident set size (`rss`):
```bash
ps -o pid,rss,command -C restphp
```

Tail latency (p99 and p99.9) is recorded directly by `wrk`'s detailed `--latency` histogram flag.
