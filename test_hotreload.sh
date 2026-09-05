#!/bin/bash
set -e
# Start restphp in the background
cargo build
./target/debug/restphp serve --port 8080 -e public/index.php --watch &
SERVER_PID=$!
sleep 2

# run wrk
wrk -t4 -c50 -d10s http://127.0.0.1:8080/ > wrk_out.txt &
WRK_PID=$!

# spam file changes
for i in {1..50}; do
    echo "<?php echo 'Hello $i';" > public/index.php
    sleep 0.1
done

wait $WRK_PID
kill $SERVER_PID
cat wrk_out.txt
