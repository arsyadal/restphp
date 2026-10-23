# Traditional PHP & Microframeworks

RestPHP is fully compatible with vanilla PHP applications and microframeworks (Slim, Flight, Leaf, Kirby, WordPress, etc.).

---

## 🚀 Running Any PHP Script Directly

Just point RestPHP at your entrypoint script:

```bash
# Serve single-file API
restphp api.php

# Serve custom directory entrypoint
restphp -p 8080 -e src/index.php
```

---

## Example: Slim Framework 4

RestPHP serves Slim applications without any code modifications:

```php
<?php
// public/index.php
use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Slim\Factory\AppFactory;

require __DIR__ . '/../vendor/autoload.php';

$app = AppFactory::create();

$app->get('/', function (Request $request, Response $response) {
    $payload = json_encode(['message' => 'Hello from Slim 4 on RestPHP!']);
    $response->getBody()->write($payload);
    return $response->withHeader('Content-Type', 'application/json');
});

$app->run();
```

Start the application:

```bash
restphp -p 8080
```

Because RestPHP maps all CGI environment variables into `$_SERVER`, Slim's request factory extracts the URI, method, headers, and body seamlessly.

## Laravel Without Octane

Run `restphp --host 127.0.0.1` from the Laravel project root. RestPHP detects
`artisan` and serves `public/index.php` through Laravel's normal HTTP kernel.
Octane is not required.

RestPHP loads the embedded PHP installation's `php.ini` and scanned `.ini`
files so configured extensions are available. It honors `PHPRC` and
`PHP_INI_SCAN_DIR`, but does not search the working directory for `php.ini`.
The CLI and embedded PHP installations can use different configuration paths.
Inspect the embedded runtime directly when diagnosing missing extensions:

```bash
restphp -e 'var_dump(php_ini_loaded_file(), php_ini_scanned_files(), get_loaded_extensions());'
```

### Diagnosing Failed Requests

Failed PHP execution returns HTTP 500 with the generic `Internal Server Error`
body. Server logs distinguish `php_status` (PHP's captured response status,
which may still be its default 200) from `http_status` (the actual client status).
They include up to 8 KiB of SAPI diagnostics and, separately, the first 8 KiB of
failed script output. `output_truncated=true` indicates a shortened output
excerpt.

PHP error logging is enabled by default, independently of `display_errors`.
Explicit PHP configuration can override `log_errors` or redirect `error_log`;
check that destination and Laravel's `storage/logs` when appropriate. Error
messages and stack traces stay out of RestPHP's generic HTTP error response.
Failure logs can contain application output, filesystem paths, or secrets;
restrict access to them.
