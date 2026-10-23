<?php
header("Content-Type: application/json");

$mode = $_GET['mode'] ?? 'ok';

if ($mode === 'notice') {
    $val = $non_existent_var;
    echo json_encode(["status" => "notice_triggered", "result" => "still_alive"]);
} elseif ($mode === 'warning') {
    $f = fopen("/non_existent_file_path_restphp_test", "r");
    echo json_encode(["status" => "warning_triggered", "result" => "still_alive"]);
} elseif ($mode === 'user_error') {
    trigger_error("Fatal test error", E_USER_ERROR);
    echo json_encode(["status" => "should_not_reach"]);
} elseif ($mode === 'hidden_exception') {
    ini_set('display_errors', '0');
    throw new RuntimeException("Hidden bootstrap failure");
} else {
    echo json_encode(["status" => "ok", "mode" => $mode]);
}
