<?php
$sum = 0;
for ($i = 0; $i < 10000; $i++) {
    $sum += sqrt($i);
}
echo "Result: " . $sum;
