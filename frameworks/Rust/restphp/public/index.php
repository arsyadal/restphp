<?php
$uri = $_SERVER['REQUEST_URI'] ?? '/';

if ($uri === '/json') {
    header('Content-Type: application/json');
    echo json_encode(["message" => "Hello, World!"]);
} elseif ($uri === '/plaintext') {
    header('Content-Type: text/plain');
    echo "Hello, World!";
} else {
    http_response_code(404);
    echo "Not Found";
}
