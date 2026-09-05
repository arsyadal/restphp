# Laravel & Laravel Octane Integration

RestPHP provides two distinct ways to run Laravel applications:
1. **Standard Mode (Zero Configuration)**: Run unmodified Laravel apps directly.
2. **Octane Mode (Maximum Throughput)**: Boot Laravel once into RAM using the official `restphp/octane` adapter.

---

## 🏎️ Option A: Laravel Octane Mode (Recommended)

Laravel Octane supercharges your application by keeping it in RAM and serving incoming requests at lightning speed.

### 1. Install the RestPHP Octane Adapter

In your existing Laravel 10, 11, or 12 project:

```bash
composer require restphp/octane
```

*(Laravel automatically discovers the `RestPhpServiceProvider`).*

### 2. Start the Server

```bash
php artisan octane:restphp
```

### 3. Command Options

```bash
php artisan octane:restphp \
    --host=0.0.0.0 \
    --port=8000 \
    --workers=1 \
    --max-requests=10000
```

| Option | Default | Description |
| :--- | :--- | :--- |
| `--host` | `127.0.0.1` | Network interface to bind to |
| `--port` | `8000` | HTTP port to listen on |
| `--workers` | `1` | Number of persistent worker OS threads |
| `--max-requests` | `10000` | Number of requests before gracefully recycling worker |

---

## 🚗 Option B: Standard Laravel Mode

If you do not wish to install Octane, RestPHP can serve traditional Laravel applications directly via `public/index.php`:

```bash
cd /path/to/laravel
restphp -p 8000
```

Because RestPHP implements a full custom SAPI with automatic superglobal injection, `$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, and `php://input` are automatically populated, allowing Laravel's HTTP Kernel to handle requests seamlessly.
