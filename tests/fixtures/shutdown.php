<?php
header('Content-Type: text/plain');

if (isset($_GET['started'])) {
    file_put_contents($_GET['started'], 'started');
}

if (isset($_GET['release'])) {
    set_time_limit(0);
    while (!file_exists($_GET['release'])) {
        clearstatcache(true, $_GET['release']);
        usleep(10000);
    }
}

echo 'shutdown-complete';
