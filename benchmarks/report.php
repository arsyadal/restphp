<?php
$path = $argv[1] ?? __DIR__ . '/results/results.json';
try {
    $contents = @file_get_contents($path);
    if ($contents === false) {
        throw new RuntimeException("Cannot read results: $path");
    }
    $results = json_decode($contents, true, 512, JSON_THROW_ON_ERROR);
    if (($results['schema_version'] ?? null) !== 2 || !isset($results['runs'])) {
        throw new RuntimeException('Unsupported result schema; legacy latency strings are not p99 data.');
    }
    echo "RESTPHP BENCHMARK REPORT\n";
    echo 'Status: ' . ($results['status'] ?? 'unknown') . "\n";
    echo 'Provenance: ' . ($results['provenance'] ?? 'unknown') . "\n";
    echo 'Comparison: ' . (!empty($results['comparative']) ? 'RestPHP and FrankenPHP' : 'not comparative') . "\n";
    if (isset($results['note'])) {
        echo $results['note'] . "\n";
    }
    if (isset($results['failure'])) {
        echo 'Failure: ' . $results['failure'] . "\n";
    }
    printf("%-12s %-20s %3s %11s %10s %10s %10s %10s %8s %10s\n",
        'Runtime', 'Scenario', 'Run', 'Req/sec', 'p50 ms', 'p95 ms', 'p99 ms', 'Requests', 'Errors', 'RSS MiB');
    $number = static function ($value, int $decimals = 3): string {
        if ($value === null) {
            return 'N/A';
        }
        if (!is_int($value) && !is_float($value)) {
            throw new RuntimeException('Expected numeric schema-v2 measurement, not legacy string.');
        }
        return number_format($value, $decimals, '.', '');
    };
    foreach ($results['runs'] as $run) {
        $data = $run['measurement'] ?? [];
        $latency = $data['latency_ms'] ?? [];
        $rss = $data['rss_bytes']['peak_sampled'] ?? null;
        printf("%-12s %-20s %3s %11s %10s %10s %10s %10s %8s %10s\n",
            $run['runtime'], $run['scenario'], $run['repetition'] ?? '-',
            $number($data['requests_per_second'] ?? null, 2),
            $number($latency['p50'] ?? null), $number($latency['p95'] ?? null), $number($latency['p99'] ?? null),
            $number($data['requests'] ?? null, 0), $number($data['errors']['total'] ?? null, 0),
            $number($rss === null ? null : $rss / 1048576, 2));
    }
    echo "Percentiles are observed wrk histogram values, not mean latency estimates. RSS is sampled process RSS.\n";
    if (($results['status'] ?? null) === 'failed') {
        exit(1);
    }
} catch (Throwable $error) {
    fwrite(STDERR, $error->getMessage() . "\n");
    exit(1);
}
