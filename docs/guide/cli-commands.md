# RestPHP CLI Commands

## Serve a script

```bash
restphp
restphp app.php -p 3000
restphp serve --host 127.0.0.1 --port 8080 --entrypoint public/index.php
```

Without a subcommand, RestPHP detects `artisan`, `public/index.php`, or `index.php`. The convenience form accepts `--host`, `--port`, `--workers`, and `--watch`. Use the `serve` subcommand for the production and TLS options below.

| `serve` flag | Default | Description |
| :--- | :--- | :--- |
| `--host` | `0.0.0.0` | Listener IP address |
| `--port`, `-p` | `8080` | HTTP or HTTPS listener port |
| `--entrypoint`, `-e` | `public/index.php` | Script executed for each request |
| `--persistent-worker` | None | PHP script booted once; replaces `--entrypoint` |
| `--workers`, `-w` | `1` | Must be `1` for NTS PHP; scale using processes |
| `--max-requests`, `-m` | `0` | Accepted requests before process drain; `0` means unlimited |
| `--max-body-bytes` | `16777216` | Maximum request body size; excess receives HTTP 413 |
| `--max-queue` | `256` | Maximum admitted jobs including the executing request; overload receives HTTP 503 |
| `--shutdown-timeout-secs` | `30` | Drain deadline, starting when shutdown begins |
| `--watch` | Off | Drain and replace the server process on source changes |

## Persistent callbacks

```bash
restphp serve --persistent-worker worker.php --max-requests 10000
```

The worker script boots the application and calls `restphp_handle_request($handler)` repeatedly. Startup does not report ready until the script reaches its first callback. See [Laravel Octane](/frameworks/laravel-octane) for the framework adapter and [state reset](/runtime/state-reset) for the isolation boundary.

## Graceful shutdown and recycling

On SIGTERM or Ctrl-C, RestPHP closes admission, drains accepted work, and joins the PHP worker. `--max-requests` uses the same drain path; configure a process supervisor to start a replacement. Successful shutdown exits with status `0`.

The shutdown deadline includes HTTP draining and work whose client disconnected. If PHP does not finish before the deadline, RestPHP exits with status `1`. Native PHP execution cannot be cancelled safely from another thread; this is not a per-request execution timeout.

## Hot reload

```bash
restphp serve --entrypoint public/index.php --watch
```

With the default `hot-reload` Cargo feature, a supervisor watches the current working directory recursively. Changes to PHP files, `.env`, `composer.json`, or `composer.lock` are debounced for 300 ms. `.git`, `target`, `node_modules`, `vendor`, and `storage` are excluded.

Reload sends SIGTERM to the child, waits for its drain deadline, and launches a new process. Zend is never shut down and reinitialized in the same process. The watcher also replaces a successfully recycled child. A child startup failure stops the watcher with an error. Use an external supervisor rather than watch mode for production.

## HTTPS with PEM certificates

```bash
restphp serve --entrypoint public/index.php --port 8443 \
  --tls-cert /private/fullchain.pem --tls-key /private/privkey.pem
```

Both files are required. Keys are loaded before PHP starts. HTTPS populates `HTTPS=on`, `REQUEST_SCHEME=https`, and the actual listener `SERVER_PORT`; plain HTTP remains `off`/`http`. Forwarded headers do not override these transport facts. Restart after replacing manually supplied PEM files.

## Automatic HTTPS with ACME

```bash
restphp serve --entrypoint public/index.php --port 443 \
  --acme-domain app.example.com --acme-contact admin@example.com \
  --acme-cache /private/restphp-acme --acme-staging
```

Replace the example domain and email with your own. Repeat `--acme-domain` and `--acme-contact` for additional names or contacts. The cache stores account keys and certificates; keep it durable, private, and writable. Existing Unix cache directories must have mode `0700`.

ACME uses Let's Encrypt and TLS-ALPN-01. Public DNS must point to this server, and inbound public TCP **443** must reach its TLS listener unchanged. Outbound ACME connectivity is required. Provisioning and renewal run while the server runs. The first HTTPS request can fail until issuance completes. Staging certificates are untrusted; omit `--acme-staging` for production after validating DNS and challenge reachability.

ACME and manually supplied PEM options are mutually exclusive. RestPHP does not start an HTTP-to-HTTPS redirect listener.

For a private or test ACME authority, pass `--acme-directory <https-url>` instead of `--acme-staging`. When that authority is signed by a private root, pass `--acme-ca-cert <pem>` to trust it only for the ACME client connection.

## Evaluate PHP

```bash
restphp -e 'echo PHP_VERSION . PHP_EOL;'
restphp eval 'echo json_encode(["status" => "ok"]);'
```

`--help` lists flags for the current command; `--version` prints the runtime version.
