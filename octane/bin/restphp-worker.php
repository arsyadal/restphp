<?php

use Laravel\Octane\ApplicationFactory;
use Laravel\Octane\RequestContext;
use Laravel\Octane\Worker;
use RestPhp\Octane\RestPhpClient;

if (PHP_SAPI !== 'restphp' || !function_exists('restphp_handle_request')) {
    throw new RuntimeException('Launch this script with restphp serve --persistent-worker.');
}

$basePath = getenv('APP_BASE_PATH') ?: getcwd();
require $basePath.'/vendor/autoload.php';
require_once __DIR__.'/../src/RestPhpClient.php';

$_ENV['OCTANE_SERVER'] = $_SERVER['OCTANE_SERVER'] = 'restphp';
ignore_user_abort(true);

$worker = new Worker(new ApplicationFactory($basePath), new RestPhpClient());
$worker->boot();

try {
    $handler = static function () use ($worker): void {
        $client = $worker->application()->make(\Laravel\Octane\Contracts\Client::class);
        [$request, $context] = $client->marshalRequest(new RequestContext());
        $worker->handle($request, $context);
    };

    while (restphp_handle_request($handler)) {
        // Admission, request limits, and shutdown are owned by the native worker.
    }
} finally {
    $worker->terminate();
}
