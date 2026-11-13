# Getting Started with RestPHP

Welcome to **RestPHP**, the persistent application server and runtime for PHP powered by Rust.

---

## ⚡ System Requirements

RestPHP runs natively on Linux (x86_64, arm64) and macOS:
- **PHP**: 8.2, 8.3, or 8.4 (`libphp-embed` / `php-dev`)
- **Operating System**: Linux kernel 5.4+ or macOS 12+
- **Rust Toolchain**: current stable (only if compiling from source)

Use a linkable PHP **embed** library matching the headers and CPU architecture. Homebrew's Apache `httpd/modules/libphp.so` is a Mach-O bundle, not an embed dylib, and cannot be linked into RestPHP. A PHP CLI installation alone is not sufficient.

---

## 📦 Installation

### Option 1: One-Line Install Script (Recommended)

```bash
curl -fsSL https://restphp.dev/install.sh | bash
```

Verify your installation:

```bash
restphp --version
# Output: restphp 0.2.0
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

When multiple PHP versions are installed, set `PHP_CONFIG`, `PHP_EMBED_LIB_NAME`, and `PHP_EMBED_LIB_DIR` before building so the headers and `libphp` match:

```bash
PHP_CONFIG=/usr/bin/php-config8.3 \
PHP_EMBED_LIB_NAME=php8.3 \
PHP_EMBED_LIB_DIR=/usr/lib \
cargo build --release
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

### 2. Laravel applications

Serve a conventional Laravel application through `public/index.php`, or install the [Laravel Octane adapter](/frameworks/laravel-octane) for a boot-once application with Octane request sandboxes. Both modes use one NTS PHP worker per process; scale through supervised processes.

---

## 🎯 Next Steps

- Explore [CLI Commands](/guide/cli-commands) for serve and eval options.
- Review the [Laravel runtime status](/frameworks/laravel-octane).
- Understand how RestPHP eliminates latency spikes in [Architecture Overview](/architecture/overview).
