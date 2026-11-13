<?php

$stateDirectory = getenv('RESTPHP_SOAK_STATE');
if (!$stateDirectory || !is_dir($stateDirectory)) {
    throw new RuntimeException('RESTPHP_SOAK_STATE must name the isolated soak directory.');
}
if (!extension_loaded('session') || !extension_loaded('filter')) {
    throw new RuntimeException('The soak workload requires session and filter extensions.');
}

$_ENV['RESTPHP_SOAK_BASELINE'] = $_SERVER['RESTPHP_SOAK_BASELINE'] = 'boot';
$boot = bin2hex(random_bytes(16));
$sequence = 0;
ignore_user_abort(true);

while (restphp_handle_request(static function () use ($stateDirectory, $boot, &$sequence): void {
    $sequence++;
    $path = parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH);
    $id = $_SERVER['HTTP_X_SOAK_ID'] ?? ($_GET['id'] ?? '');
    $leaked = isset($_SERVER['SOAK_MUTATION']) || isset($_GET['SOAK_MUTATION'])
        || isset($_POST['SOAK_MUTATION']) || isset($_COOKIE['SOAK_MUTATION'])
        || isset($_REQUEST['SOAK_MUTATION']) || isset($_FILES['SOAK_MUTATION']);
    $response = [
        'id' => $id,
        'boot' => $boot,
        'pid' => getmypid(),
        'sequence' => $sequence,
        'method' => $_SERVER['REQUEST_METHOD'],
        'path' => $path,
        'leaked' => $leaked,
        'environment' => $_ENV['RESTPHP_SOAK_BASELINE'] ?? null,
        'server_environment' => $_SERVER['RESTPHP_SOAK_BASELINE'] ?? null,
        'query' => $_GET,
        'cookie' => $_COOKIE,
        'probe_header' => $_SERVER['HTTP_X_SOAK_PROBE'] ?? null,
    ];
    header('Content-Type: application/json');

    if ($path === '/fatal') {
        echo 'private-partial-soak-output';
        throw new RuntimeException('private-soak-fatal');
    }

    try {
        switch ($path) {
            case '/plain':
                break;
            case '/json':
                $body = file_get_contents('php://input');
                $response['json'] = json_decode($body, true, 512, JSON_THROW_ON_ERROR);
                $response['body_sha256'] = hash('sha256', $body);
                header('X-Soak-Only: json');
                http_response_code(201);
                break;
            case '/binary':
                $body = file_get_contents('php://input');
                $response['body_sha256'] = hash('sha256', $body);
                $response['body_bytes'] = strlen($body);
                break;
            case '/form':
                $response['form'] = $_POST;
                $response['raw_body'] = file_get_contents('php://input');
                break;
            case '/upload':
                $file = $_FILES['payload'] ?? null;
                if (!$file || $file['error'] !== UPLOAD_ERR_OK) {
                    throw new RuntimeException('Multipart upload failed.');
                }
                $response['upload'] = [
                    'name' => $file['name'],
                    'bytes' => $file['size'],
                    'sha256' => hash_file('sha256', $file['tmp_name']),
                    'is_uploaded_file' => is_uploaded_file($file['tmp_name']),
                ];
                $response['form'] = $_POST;
                break;
            case '/session':
                session_save_path($stateDirectory.'/sessions');
                session_cache_limiter('');
                if (!session_start()) {
                    throw new RuntimeException('Session startup failed.');
                }
                $_SESSION['counter'] = ($_SESSION['counter'] ?? 0) + 1;
                $response['session_id'] = session_id();
                $response['session_counter'] = $_SESSION['counter'];
                break;
            case '/exception':
                throw new LogicException('recoverable-soak-exception');
            case '/slow':
            case '/disconnect':
                if (!preg_match('/^[a-f0-9]{32}$/', $id)) {
                    throw new RuntimeException('Invalid slow-request marker.');
                }
                file_put_contents($stateDirectory.'/started-'.$id, 'started');
                usleep(200000);
                file_put_contents($stateDirectory.'/finished-'.$id, 'finished');
                $response['finished'] = true;
                break;
            default:
                http_response_code(404);
                $response['error'] = 'unknown workload';
        }
    } catch (LogicException $error) {
        http_response_code(500);
        $response['caught'] = get_class($error);
    }

    echo json_encode($response, JSON_THROW_ON_ERROR);
    $_SERVER['SOAK_MUTATION'] = $_GET['SOAK_MUTATION'] = $_POST['SOAK_MUTATION'] = true;
    $_COOKIE['SOAK_MUTATION'] = $_REQUEST['SOAK_MUTATION'] = $_FILES['SOAK_MUTATION'] = true;
    $_ENV['RESTPHP_SOAK_BASELINE'] = $_SERVER['RESTPHP_SOAK_BASELINE'] = 'request mutation';
})) {
}
