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
