---
layout: home

hero:
  name: "RestPHP"
  text: "The Blazing-Fast PHP Runtime"
  tagline: "Persistent Application Server & Runtime powered by Rust. Zero Host GC. Zero CGO Overhead. Outperforming FrankenPHP, RoadRunner, and Swoole."
  actions:
    - theme: brand
      text: Get Started →
      link: /guide/getting-started
    - theme: alt
      text: View Benchmarks
      link: /benchmarks/comparison
    - theme: alt
      text: GitHub
      link: https://github.com/arsyadal/restphp

features:
  - icon:
      src: /icons/zap.svg
      alt: Zero-Cost C-FFI
    title: Zero-Cost C-FFI
    details: Directly embeds Zend Engine via raw C-ABI (extern "C"). Eliminates ~60ns cgo stack-switching overhead present in Go-based servers.
  - icon:
      src: /icons/shield.svg
      alt: Zero Host Garbage Collection
    title: Zero Host Garbage Collection
    details: Rust compile-time ownership (RAII) manages server memory. No Stop-The-World pauses; guarantees ultra-predictable p99 tail latency.
  - icon:
      src: /icons/cpu.svg
      alt: Persistent Worker Architecture
    title: Persistent Worker Architecture
    details: Dedicated OS threads host isolated Zend VM instances. Boots your app once into RAM; zero file-reloading overhead on incoming requests.
  - icon:
      src: /icons/rocket.svg
      alt: 1st-Class Laravel Octane Driver
    title: 1st-Class Laravel Octane Driver
    details: Official adapter package (restphp/octane). Supercharge existing Laravel applications up to 10x throughput with zero code changes.
  - icon:
      src: /icons/box.svg
      alt: Single Standalone Binary
    title: Single Standalone Binary
    details: Shipped as a single static executable (restphp). No Nginx, PHP-FPM, or Caddy configuration required.
  - icon:
      src: /icons/plug.svg
      alt: 100% PHP Extension Compatible
    title: 100% PHP Extension Compatible
    details: Works seamlessly with all native PHP extensions (PDO, MySQL, Redis, OPcache, cURL) without dangerous coroutine monkey-patching.
---

<HeroTerminal />

<BenchmarkChart />

---

## One Minute with RestPHP

Install, build, and serve any PHP application in seconds:

::: code-group

```bash [Single Command Startup]
# Just run restphp — auto-detects Laravel, public/index.php, or index.php!
restphp
```

```bash [Laravel Octane]
# Install official RestPHP adapter
composer require restphp/octane

# Start persistent Laravel server
php artisan octane:restphp --port 8000
```

```bash [CLI Evaluation]
# Execute PHP code directly in-memory from terminal
restphp -e 'echo "PHP Version: " . PHP_VERSION . "\n";'
```


:::

---

## Architecture Overview

```mermaid
graph TD
    Client[HTTP Clients / Browsers] -->|TCP / HTTP/1.1 & HTTP/2| Axum[Axum / Tokio Async HTTP Engine]
    
    subgraph RustHost ["RestPHP Rust Core"]
        Axum --> Router[Request Dispatcher]
        Router --> Channel["Lock-Free Crossbeam Channel"]
        Channel --> WorkerPool["Persistent Worker Pool"]
    end
    
    subgraph WorkerThread ["Dedicated OS Worker Thread"]
        WorkerPool --> SAPIBridge["RestPHP SAPI Bridge (c/sapi.c)"]
        SAPIBridge --> FFI["Zero-Cost C-ABI FFI"]
        FFI --> ZendVM["Embedded Zend VM (libphp.so)"]
        ZendVM --> Script["User Script / Laravel Kernel"]
        Script --> OutputBuffer["ub_write / send_headers Hook"]
        OutputBuffer --> Response["In-Memory Response Bytes"]
    end

    Response --> Oneshot["Tokio Oneshot Channel"]
    Oneshot --> Axum
    Axum --> Client
```

---

## Architectural Comparison

Why does RestPHP outperform Go and C++ alternatives?

```mermaid
graph LR
    subgraph FrankenPHP_Go ["FrankenPHP (Go)"]
        HTTP1[HTTP Request] --> Caddy[Caddy / Go]
        Caddy -- "cgo stack-switch (~60ns)" --> Zend1[Zend Engine]
        GC1["Go GC (STW Pause) + PHP GC"]
    end

    subgraph RestPHP_Rust ["RestPHP (Rust)"]
        HTTP2[HTTP Request] --> Tokio[Tokio Async / Axum]
        Tokio -- "Zero-Cost C ABI (0ns)" --> Zend2[Persistent Zend VM]
        GC2["Zero Host GC (Compile-time RAII)"]
    end

    style RestPHP_Rust fill:#1e293b,stroke:#f97316,stroke-width:3px
    style FrankenPHP_Go fill:#1e293b,stroke:#64748b,stroke-width:1px
```

---

<div class="cta-banner">

### Ready to build the future of PHP?

Join the revolution. Say goodbye to slow cold boots and unpredictable GC pauses.

[Get Started with RestPHP →](/guide/getting-started){.btn-primary}
[View Source on GitHub ★](https://github.com/arsyadal/restphp){.btn-secondary}

</div>

<style>
.benchmark-hero-container {
  margin: 3rem 0;
  padding: 1.5rem;
  background: var(--vp-c-bg-soft);
  border-radius: 12px;
  border: 1px solid var(--vp-c-divider);
}

.cta-banner {
  text-align: center;
  padding: 3rem 1rem;
  margin: 3rem 0;
  background: radial-gradient(circle at 50% 50%, rgba(249, 115, 22, 0.12) 0%, transparent 70%);
  border-radius: 16px;
  border: 1px solid rgba(249, 115, 22, 0.25);
}

.btn-primary {
  display: inline-block;
  padding: 0.75rem 1.75rem;
  margin: 0.5rem;
  border-radius: 8px;
  background: #f97316;
  color: white !important;
  font-weight: 600;
  text-decoration: none;
  transition: all 0.2s;
}

.btn-primary:hover {
  background: #ea580c;
  transform: translateY(-1px);
}

.btn-secondary {
  display: inline-block;
  padding: 0.75rem 1.75rem;
  margin: 0.5rem;
  border-radius: 8px;
  background: var(--vp-c-bg-soft);
  color: var(--vp-c-text-1) !important;
  border: 1px solid var(--vp-c-divider);
  font-weight: 600;
  text-decoration: none;
  transition: all 0.2s;
}

.btn-secondary:hover {
  border-color: #f97316;
  transform: translateY(-1px);
}
</style>
