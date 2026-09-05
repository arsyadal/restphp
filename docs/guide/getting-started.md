# Getting Started with RestPHP

Welcome to **RestPHP**, the persistent application server and runtime for PHP powered by Rust.

---

## ⚡ System Requirements

RestPHP runs natively on Linux (x86_64, arm64) and macOS:
- **PHP**: 8.2, 8.3, or 8.4 (`libphp-embed` / `php-dev`)
- **Operating System**: Linux kernel 5.4+ or macOS 12+
- **Rust Toolchain**: 1.75+ (only if compiling from source)

---

## 📦 Installation

### Option 1: One-Line Install Script (Recommended)

```bash
curl -fsSL https://restphp.dev/install.sh | bash
```

Verify your installation:

```bash
restphp --version
# Output: restphp 0.1.0
```

### Option 2: Build from Source with Cargo

Ensure you have `php-dev` and build essentials installed:

```bash
# Ubuntu / Debian
sudo apt-get install -y build-essential libphp-embed php-dev pkg-config clang

# Clone and compile
git clone https://github.com/arsyadal/restphp.git
cd restphp
cargo build --release

# Install binary to /usr/local/bin
sudo cp target/release/restphp /usr/local/bin/
```

---

## 🚀 Quickstart: Your First Application

### 1. Create a Project Directory

```bash
mkdir my-app && cd my-app
mkdir public
```

### 2. Create `public/index.php`

```php
<?php
header("Content-Type: application/json");

echo json_encode([
    "message" => "Hello from RestPHP!",
    "engine" => "RestPHP (Rust)",
    "php_version" => PHP_VERSION,
    "time" => microtime(true),
], JSON_PRETTY_PRINT);
```

### 3. Start the Server

```bash
restphp -p 8080
```

Open your browser or run:

```bash
curl -i http://localhost:8080/
```

You should see:

```http
HTTP/1.1 200 OK
content-type: application/json
server: RestPHP/0.1.0

{
    "message": "Hello from RestPHP!",
    "engine": "RestPHP (Rust)",
    "php_version": "8.4.24",
    "time": 1788588150.117025
}
```

---

## 🎯 Next Steps

- Explore [CLI Commands](/guide/cli-commands) for serve and eval options.
- Supercharge your Laravel app with the [Laravel Octane Adapter](/frameworks/laravel-octane).
- Understand how RestPHP eliminates latency spikes in [Architecture Overview](/architecture/overview).
