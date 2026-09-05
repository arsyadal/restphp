<?php
$code = isset($_GET['code']) ? (int)$_GET['code'] : 200;
http_response_code($code);

if (isset($_GET['header_key']) && isset($_GET['header_val'])) {
    header($_GET['header_key'] . ": " . $_GET['header_val']);
}

if (isset($_GET['content_type'])) {
    header("Content-Type: " . $_GET['content_type']);
} else {
    header("Content-Type: application/json");
}

echo json_encode([
    "status_code_set" => $code,
    "query" => $_GET,
    "message" => "response_ok"
]);
