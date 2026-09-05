<?php
header("Content-Type: application/json");
$input = file_get_contents("php://input");

echo json_encode([
    "received_len" => strlen($input),
    "received_md5" => md5($input),
    "first_16_hex" => bin2hex(substr($input, 0, 16)),
    "last_16_hex" => bin2hex(substr($input, -16)),
]);
