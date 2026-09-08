<?php

$mode = $_GET['mode'] ?? 'safe';

if ($mode === 'binary') {
    header('Content-Type: application/octet-stream');
    echo "binary\x00payload";
    return;
}

header('Connection: keep-alive');
header('Keep-Alive: timeout=5');
header('Proxy-Test: must-not-leak');
header('TE: trailers');
header('Trailer: X-Trailer');
header('Transfer-Encoding: chunked');
header('Upgrade: h2c');
header('Content-Length: 1');
header('Server: untrusted-php');
header('X-Allowed: retained');
header('Set-Cookie: first=1', false);
header('Set-Cookie: second=2', false);
echo 'safe';
