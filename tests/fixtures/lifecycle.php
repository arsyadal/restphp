<?php
header("Content-Type: application/json");

// Check if request-level isolation is preserved
global $test_leaked_variable;

$had_previous_leak = isset($test_leaked_variable);
$previous_value = $test_leaked_variable ?? null;

// Intentionally pollute global scope
$req_id = $_GET['req_id'] ?? 'none';
$test_leaked_variable = "polluted_by_" . $req_id;

echo json_encode([
    "req_id" => $req_id,
    "had_previous_leak" => $had_previous_leak,
    "previous_value" => $previous_value,
    "current_query" => $_GET,
]);
