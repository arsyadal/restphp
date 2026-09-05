<?php
$results_file = __DIR__ . '/results/results.json';
if (!file_exists($results_file)) {
    die("Results file not found.\n");
}

$results = json_decode(file_get_contents($results_file), true);

echo "\nRESTPHP BENCHMARK REPORT\n";
echo str_pad("Test Case", 20) . str_pad("Req/sec", 15) . str_pad("Latency", 15) . "\n";
echo str_repeat("-", 50) . "\n";
foreach ($results as $test => $data) {
    echo str_pad($test, 20) . str_pad($data['req_sec'], 15) . str_pad($data['latency'], 15) . "\n";
}
echo "\n";
