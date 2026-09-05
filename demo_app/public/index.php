<?php
// Simple Real-World Micro-Router & API Service
header("Content-Type: application/json");
header("X-Powered-By: RestPHP-Rust-Engine");

$uri = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH);
$method = $_SERVER['REQUEST_METHOD'] ?? 'GET';

// Route: GET /
if ($uri === '/' && $method === 'GET') {
    echo json_encode([
        'app' => 'RestPHP Production Demo',
        'engine' => 'Rust 2021 + Zend Engine C-ABI',
        'php_version' => PHP_VERSION,
        'uptime_status' => 'healthy',
        'routes' => [
            'GET /',
            'GET /api/users?role=developer',
            'POST /api/users',
            'GET /health',
        ]
    ], JSON_PRETTY_PRINT);
    exit;
}

// Route: GET /api/users
if ($uri === '/api/users' && $method === 'GET') {
    $role = $_GET['role'] ?? 'all';
    $users = [
        ['id' => 1, 'name' => 'Arsyad Alghital', 'role' => 'founder'],
        ['id' => 2, 'name' => 'Taylor Otwell', 'role' => 'developer'],
        ['id' => 3, 'name' => 'Linus Torvalds', 'role' => 'developer'],
    ];

    if ($role !== 'all') {
        $users = array_values(array_filter($users, fn($u) => $u['role'] === $role));
    }

    echo json_encode([
        'status' => 'success',
        'filter_role' => $role,
        'count' => count($users),
        'data' => $users,
    ], JSON_PRETTY_PRINT);
    exit;
}

// Route: POST /api/users
if ($uri === '/api/users' && $method === 'POST') {
    $raw = file_get_contents('php://input');
    $data = json_decode($raw, true) ?? [];
    
    http_response_code(201);
    echo json_encode([
        'status' => 'created',
        'message' => 'User entity successfully persisted in memory',
        'user' => [
            'id' => rand(100, 999),
            'name' => $data['name'] ?? 'Anonymous',
            'role' => $data['role'] ?? 'user',
            'created_at' => date('Y-m-d H:i:s'),
        ]
    ], JSON_PRETTY_PRINT);
    exit;
}

// Route: GET /health
if ($uri === '/health') {
    echo json_encode([
        'status' => 'ok',
        'memory_kb' => memory_get_usage(true) / 1024,
        'timestamp' => microtime(true),
    ]);
    exit;
}

// Fallback 404
http_response_code(404);
echo json_encode(['error' => 'Not Found', 'path' => $uri]);
