<?php
header("Content-Type: application/json; charset=utf-8");

echo json_encode([
    "query" => $_GET,
    "post" => $_POST,
    "cookie" => $_COOKIE,
    "raw_input" => file_get_contents("php://input"),
    "server_uri" => $_SERVER["REQUEST_URI"] ?? null,
], JSON_UNESCAPED_UNICODE | JSON_PRETTY_PRINT);
