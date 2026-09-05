<?php
header("Content-Type: application/json");
header("X-Powered-By: RestPHP-Rust-Engine");
echo json_encode([
    "status" => "ok",
    "engine" => "RestPHP",
    "version" => "0.1.0",
    "php_version" => PHP_VERSION,
    "method" => $_SERVER["REQUEST_METHOD"] ?? "GET",
    "uri" => $_SERVER["REQUEST_URI"] ?? "/",
    "query" => $_GET,
    "post" => $_POST,
    "cookie" => $_COOKIE,
    "custom_header" => $_SERVER["HTTP_X_RESTPHP_TEST"] ?? null,
    "raw_input" => file_get_contents("php://input"),
    "time" => microtime(true),
], JSON_PRETTY_PRINT);
