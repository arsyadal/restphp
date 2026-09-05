<?php
header("Content-Type: application/json");
$raw_input = file_get_contents("php://input");

echo json_encode([
    "status" => "ok",
    "engine" => "RestPHP",
    "method" => $_SERVER["REQUEST_METHOD"] ?? null,
    "uri" => $_SERVER["REQUEST_URI"] ?? null,
    "query_string" => $_SERVER["QUERY_STRING"] ?? null,
    "get" => $_GET ?? [],
    "post" => $_POST ?? [],
    "cookie" => $_COOKIE ?? [],
    "server" => $_SERVER ?? [],
    "raw_input" => $raw_input,
    "raw_input_len" => strlen($raw_input),
    "raw_input_md5" => md5($raw_input),
], JSON_PRETTY_PRINT);
