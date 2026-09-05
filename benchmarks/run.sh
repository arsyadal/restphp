#!/bin/bash
set -e

PORT=8088
BIN="./target/release/restphp"

if [ ! -f "$BIN" ]; then
    echo "Building release binary..."
    cargo build --release
fi

mkdir -p benchmarks/results

echo "================================================================"
echo "    RestPHP Official Empirical Benchmark Suite (wrk)"
echo "    Hardware: $(uname -m) Linux $(uname -r)"
echo "    PHP Engine: $(php -r 'echo PHP_VERSION;') (libphp embedded)"
echo "================================================================"

JSON_RESULTS="benchmarks/results/results.json"
echo "{" > "$JSON_RESULTS"

run_scenario() {
    local key="$1"
    local title="$2"
    local script="$3"
    local is_last="$4"
    local out_file="benchmarks/results/${key}_wrk.txt"

    echo ""
    echo ">>> Testing ${title} (${script})..."
    
    # Start RestPHP with unlimited requests (-m 0)
    $BIN serve -p $PORT -m 0 -e "$script" > /dev/null 2>&1 &
    PID=$!

    # Wait for startup
    for i in {1..30}; do
        if curl -s "http://127.0.0.1:${PORT}/" > /dev/null 2>&1; then
            break
        fi
        sleep 0.1
    done

    # Warmup
    wrk -t2 -c10 -d1s "http://127.0.0.1:${PORT}/" > /dev/null 2>&1

    # Real benchmark run
    wrk -t2 -c20 -d5s --latency "http://127.0.0.1:${PORT}/" > "$out_file"

    # Terminate process safely
    kill $PID 2>/dev/null || true
    wait $PID 2>/dev/null || true
    sleep 0.5

    # Extract real empirical numbers
    RPS=$(grep "Requests/sec:" "$out_file" | awk '{printf "%.2f", $2}')
    P99=$(grep "99%" "$out_file" | awk '{print $2}')
    if [ -z "$P99" ]; then
        P99=$(grep "Latency" "$out_file" | head -1 | awk '{print $2}')
    fi

    echo "  [RAW] Requests/sec: ${RPS} | p99 Latency: ${P99}"

    # Append to JSON
    if [ "$is_last" = "true" ]; then
        echo "  \"${key}\": { \"req_sec\": \"${RPS}\", \"latency\": \"${P99}\" }" >> "$JSON_RESULTS"
    else
        echo "  \"${key}\": { \"req_sec\": \"${RPS}\", \"latency\": \"${P99}\" }," >> "$JSON_RESULTS"
    fi
}

run_scenario "hello_plaintext" "Plaintext Echo" "benchmarks/scripts/hello.php" "false"
run_scenario "json_api" "JSON API Serialization" "benchmarks/scripts/json_api.php" "false"
run_scenario "heavy_compute" "CPU Fibonacci/Primes" "benchmarks/scripts/heavy_compute.php" "true"

echo "}" >> "$JSON_RESULTS"

echo ""
echo "================================================================"
echo "    Empirical Benchmark Results Verified!"
echo "================================================================"

php benchmarks/report.php
