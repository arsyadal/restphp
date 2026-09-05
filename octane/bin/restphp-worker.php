<?php

/**
 * RestPHP Persistent Worker Entrypoint for Laravel Octane.
 *
 * This script is executed by RestPHP to boot the Laravel application once in RAM
 * and handle incoming HTTP requests persistently with zero framework reloading overhead.
 */

use Illuminate\Http\Request;
use Laravel\Octane\ApplicationFactory;
use Laravel\Octane\Octane;
use Symfony\Component\HttpFoundation\Response as SymfonyResponse;

$basePath = $argv[1] ?? getcwd();

// Load Composer Autoloader
if (file_exists($basePath . '/vendor/autoload.php')) {
    require_once $basePath . '/vendor/autoload.php';
} else {
    fwrite(STDERR, "RestPHP Worker: Autoloader not found in {$basePath}\n");
    exit(1);
}

// Check if Laravel Octane is installed
if (!class_exists(ApplicationFactory::class)) {
    fwrite(STDERR, "RestPHP Worker: Laravel Octane is not installed in {$basePath}\n");
    exit(1);
}

// Boot the Laravel Application once in memory
$appFactory = new ApplicationFactory($basePath);
$app = $appFactory->createApplication();

// Set environment flag indicating RestPHP is serving
$_ENV['OCTANE_SERVER'] = 'restphp';
$_SERVER['OCTANE_SERVER'] = 'restphp';

$maxRequests = (int) ($_SERVER['MAX_REQUESTS'] ?? 10000);
$requestCount = 0;

// Worker Request Handler Callback
$handler = static function () use ($app, &$requestCount) {
    $requestCount++;

    try {
        // 1. Capture incoming HTTP request populated by RestPHP SAPI
        $request = Request::capture();

        // 2. Clone fresh sandbox app instance or handle via Octane
        if (class_exists(Octane::class) && method_exists(Octane::class, 'handle')) {
            $response = $app->handle($request);
        } else {
            $response = $app->handle($request);
        }

        // 3. Send response headers and body (streamed via RestPHP SAPI ub_write & send_headers)
        if ($response instanceof SymfonyResponse) {
            $response->send();
        }

        // 4. Terminate request lifecycle and reset state
        if (method_exists($app, 'terminate')) {
            $app->terminate($request, $response);
        }
    } catch (Throwable $e) {
        http_response_code(500);
        header('Content-Type: application/json');
        echo json_encode([
            'message' => 'Internal Server Error (RestPHP Worker)',
            'exception' => get_class($e),
            'error' => $e->getMessage(),
        ]);
    } finally {
        // 5. Zend garbage collection cycle
        if (function_exists('gc_collect_cycles')) {
            gc_collect_cycles();
        }
    }
};

// Check if RestPHP native persistent loop function is available
if (function_exists('restphp_handle_request')) {
    do {
        $keepRunning = restphp_handle_request($handler);
        if (!$keepRunning || ($maxRequests > 0 && $requestCount >= $maxRequests)) {
            break;
        }
    } while (true);
} else {
    // Fallback single-request invocation (for standard SAPI execution)
    $handler();
}
