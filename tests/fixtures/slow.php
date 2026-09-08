<?php

// Keep one worker busy long enough for the test to deterministically fill its queue.
usleep(1500000);
if (isset($_GET['marker'])) {
    file_put_contents($_GET['marker'], 'PHP_EXECUTED');
}
header('Content-Type: text/plain');
echo 'slow-complete';
