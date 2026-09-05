<?php
header("Content-Type: application/json");
echo json_encode([
    "status" => "ok",
    "engine" => "RestPHP",
    "version" => "0.1.0",
    "php_version" => PHP_VERSION,
    "method" => $_SERVER["REQUEST_METHOD"] ?? "GET",
    "uri" => $_SERVER["REQUEST_URI"] ?? "/",
    "query" => $_GET,
    "time" => microtime(true),
], JSON_PRETTY_PRINT);
