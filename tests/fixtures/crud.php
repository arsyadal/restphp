<?php
header("Content-Type: application/json");

$method = $_SERVER['REQUEST_METHOD'] ?? 'GET';
$db_file = sys_get_temp_dir() . '/restphp_crud_test_' . getmypid() . '.json';

if (!function_exists('restphp_crud_get_db')) {
    function restphp_crud_get_db($file) {
        if (file_exists($file)) {
            $content = file_get_contents($file);
            $data = json_decode($content, true);
            if (is_array($data)) {
                return $data;
            }
        }
        return [];
    }
}

if (!function_exists('restphp_crud_save_db')) {
    function restphp_crud_save_db($file, $data) {
        file_put_contents($file, json_encode($data, JSON_PRETTY_PRINT));
    }
}

$db = restphp_crud_get_db($db_file);
$id = $_GET['id'] ?? null;

if ($method === 'GET') {
    if ($id !== null) {
        if (isset($db[$id])) {
            http_response_code(200);
            echo json_encode(["status" => "ok", "item" => $db[$id]]);
        } else {
            http_response_code(404);
            echo json_encode(["status" => "error", "message" => "Item not found"]);
        }
    } else {
        http_response_code(200);
        echo json_encode(["status" => "ok", "items" => array_values($db)]);
    }
} elseif ($method === 'POST') {
    $raw = file_get_contents("php://input");
    $data = json_decode($raw, true);
    if (!is_array($data)) {
        $data = $_POST;
    }
    $new_id = (string)(count($db) + 1);
    $item = array_merge(["id" => $new_id], $data);
    $db[$new_id] = $item;
    restphp_crud_save_db($db_file, $db);
    http_response_code(201);
    echo json_encode(["status" => "created", "item" => $item]);
} elseif ($method === 'PUT') {
    if ($id !== null && isset($db[$id])) {
        $raw = file_get_contents("php://input");
        $data = json_decode($raw, true);
        if (!is_array($data)) {
            $data = $_POST;
        }
        $db[$id] = array_merge($db[$id], $data);
        restphp_crud_save_db($db_file, $db);
        http_response_code(200);
        echo json_encode(["status" => "updated", "item" => $db[$id]]);
    } else {
        http_response_code(404);
        echo json_encode(["status" => "error", "message" => "Item not found"]);
    }
} elseif ($method === 'DELETE') {
    if ($id !== null && isset($db[$id])) {
        $deleted = $db[$id];
        unset($db[$id]);
        restphp_crud_save_db($db_file, $db);
        http_response_code(200);
        echo json_encode(["status" => "deleted", "item" => $deleted]);
    } else {
        http_response_code(404);
        echo json_encode(["status" => "error", "message" => "Item not found"]);
    }
} else {
    http_response_code(405);
    echo json_encode(["status" => "error", "message" => "Method not allowed"]);
}
