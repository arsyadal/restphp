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

## 🚀 Quickstart (Bun-Style Simplicity)

### 1. Zero-Config Instant Startup

```bash
# Just run restphp — auto-detects Laravel, public/index.php, or index.php!
restphp

# Run any script directly on a custom port
restphp app.php -p 3000

# Evaluate inline PHP directly from terminal
restphp -e 'echo "Hello from RestPHP!\n";'
```

### 2. Laravel Octane Integration

Install the official RestPHP adapter:

```bash
composer require restphp/octane
```

Run persistent Laravel server:

```bash
php artisan octane:restphp --port 8000
```

---

## 🎯 Next Steps

- Explore [CLI Commands](/guide/cli-commands) for serve and eval options.
- Supercharge your Laravel app with the [Laravel Octane Adapter](/frameworks/laravel-octane).
- Understand how RestPHP eliminates latency spikes in [Architecture Overview](/architecture/overview).
