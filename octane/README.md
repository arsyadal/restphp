# RestPHP Adapter for Laravel Octane

The ultra-high-performance Laravel Octane driver for RestPHP powered by Rust.

---

## 🚀 Overview

`restphp/octane` connects your Laravel application directly to the **RestPHP** application server, unlocking:
- **Zero Host GC Latency**: Eliminates Stop-The-World latency spikes present in Go-based servers (FrankenPHP/RoadRunner).
- **Persistent Memory Mode**: Boots your Laravel application once in RAM; handles thousands of consecutive requests with zero file reloading overhead.
- **Pure C-FFI Embedding**: Direct zero-cost Zend VM embedding without cgo stack switching.

---

## 📦 Installation

In your Laravel application:

```bash
composer require restphp/octane
```

*(Note: RestPHP service provider is automatically discovered by Laravel package discovery).*

Ensure the `restphp` binary is installed on your system or available in your `PATH`:

```bash
# Check restphp binary
restphp --version
```

---

## ⚡ Usage

Start your high-performance server using Artisan:

```bash
php artisan octane:restphp
```

### Options

| Option | Default | Description |
| :--- | :--- | :--- |
| `--host` | `127.0.0.1` | The IP address to bind to |
| `--port` | `8000` | The port to listen on |
| `--workers` | `1` | Number of persistent Zend worker threads |
| `--max-requests` | `10000` | Number of requests to process before recycling worker |

Example with custom port:

```bash
php artisan octane:restphp --host=0.0.0.0 --port=8080
```

---

## 🧪 License

MIT License. Authored by [Arsyad Alghital](https://github.com/arsyadal).
