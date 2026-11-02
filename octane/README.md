# RestPHP Adapter for Laravel Octane

This package boots Laravel once inside RestPHP's native callback loop. Requests run through Laravel Octane's `Worker`, application sandbox, and lifecycle listeners; the adapter does not reuse a raw Laravel HTTP kernel as a persistent singleton.

## Installation

From your Laravel application, register this directory as a Composer path repository:

```bash
composer config repositories.restphp path /absolute/path/to/restphp/octane
composer require restphp/octane:@dev
```

Laravel discovers `RestPhpServiceProvider` automatically. Install the RestPHP binary and a matching NTS PHP embed runtime. Artisan process supervision requires the `pcntl` and `posix` extensions in the PHP CLI.

## Operation

```bash
php artisan octane:restphp start --binary=/absolute/path/to/restphp \
  --host=127.0.0.1 --port=8000 --max-requests=10000

php artisan octane:restphp status
php artisan octane:restphp reload
php artisan octane:restphp stop
```

Start runs in the foreground as a supervisor. It holds a locked state file, replaces a child after successful request-limit recycling or explicit reload, and drains its child on stop. An unlocked state file is stale even if its recorded PID has been reused. Use a process manager such as systemd to manage the supervisor itself.

| Option | Default | Meaning |
| :--- | :--- | :--- |
| `--binary` | Find `restphp` in PATH | Native runtime executable |
| `--host` | `127.0.0.1` | Listener IP address |
| `--port` | `8000` | Listener port |
| `--max-requests` | `10000` | Accepted requests before process replacement; `0` is unlimited |
| `--max-queue` | `256` | Maximum admitted PHP requests, including the executing request |
| `--max-body-bytes` | `16777216` | Maximum HTTP request body size |
| `--shutdown-timeout-secs` | `30` | Drain deadline |
| `--watch` | Off | Debounced development source reload with fresh processes |

The command also forwards `--tls-cert`, `--tls-key`, `--acme-domain` (repeatable), `--acme-contact` (repeatable), `--acme-cache`, and `--acme-staging`. PEM certificates and ACME options are mutually exclusive. ACME needs public DNS, inbound TCP 443 reaching the TLS listener unchanged, and a durable private cache (Unix mode `0700`). Staging certificates are untrusted.

The current NTS runtime supports **one worker per process**. There is no multi-threaded `--workers` option in this adapter; scale with separate supervised processes.

## Native invocation

```bash
cd /path/to/laravel
restphp serve --persistent-worker vendor/restphp/octane/bin/restphp-worker.php \
  --host 127.0.0.1 --port 8000 --max-requests 10000
```

Set `APP_BASE_PATH` when the application is not the working directory. A directly invoked native process exits on recycling; an external supervisor must replace it. The worker script must run under RestPHP, not plain `php`.

## Isolation and failure behavior

Native callbacks receive fresh HTTP metadata, superglobals, input, response headers, status, and output. Octane performs framework sandbox and scoped-service cleanup. Arbitrary globals, static properties, and retained singleton references remain application-owned state: follow Octane's compatibility rules and avoid retaining request objects across requests.

Unrecoverable callback failures stop the worker for replacement. Ordinary exceptions rendered by Laravel's HTTP kernel can return HTTP 500 without ending the worker. A finite request limit bounds the lifetime of persistent application state.

Verified locally with Laravel **12.70.2**, Octane **2.22.0**, and PHP **8.2.30 NTS**: stable boot token/PID across requests, scoped-service reset, body/header/cookie isolation, kernel-exception recovery, recycling, status, reload, and stop. Other framework and extension combinations need their own deployment validation.

## License

Dual licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), at your option. Copyright 2026 Arsyad Alghital.
