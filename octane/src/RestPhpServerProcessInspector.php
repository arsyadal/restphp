<?php

namespace RestPhp\Octane;

use Laravel\Octane\ProcessFactory;
use Symfony\Component\Process\Process;

class RestPhpServerProcessInspector
{
    /**
     * Determine if the RestPHP server process is currently running.
     */
    public function serverIsRunning(string $host, int $port): bool
    {
        $connection = @fsockopen($host, $port, $errno, $errstr, 0.5);

        if (is_resource($connection)) {
            fclose($connection);
            return true;
        }

        return false;
    }

    /**
     * Stop the given server process gracefully.
     */
    public function stopServer(Process $serverProcess): void
    {
        $serverProcess->signal(SIGTERM);

        $startTime = time();
        while ($serverProcess->isRunning() && time() - $startTime < 5) {
            usleep(100000);
        }

        if ($serverProcess->isRunning()) {
            $serverProcess->signal(SIGKILL);
        }
    }
}
