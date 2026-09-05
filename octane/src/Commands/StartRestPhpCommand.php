<?php

namespace RestPhp\Octane\Commands;

use Illuminate\Console\Command;
use RestPhp\Octane\RestPhpServerProcessInspector;
use Symfony\Component\Process\Process;

class StartRestPhpCommand extends Command
{
    /**
     * The name and signature of the console command.
     *
     * @var string
     */
    protected $signature = 'octane:restphp
                            {--host=127.0.0.1 : The IP address the server should bind to}
                            {--port=8000 : The port the server should bind to}
                            {--workers=auto : The number of workers that should be started}
                            {--max-requests=10000 : The number of requests to process before reloading a worker}
                            {--watch : Automatically reload the server when files are modified}';

    /**
     * The console command description.
     *
     * @var string
     */
    protected $description = 'Start the RestPHP high-performance Octane application server';

    /**
     * Execute the console command.
     */
    public function handle(RestPhpServerProcessInspector $inspector): int
    {
        $host = (string) $this->option('host');
        $port = (int) $this->option('port');

        if ($inspector->serverIsRunning($host, $port)) {
            $this->error("RestPHP server is already running on http://{$host}:{$port}");
            return 1;
        }

        $this->info("🦀🐘 Starting RestPHP server on http://{$host}:{$port}...");

        $binary = $this->resolveRestPhpBinary();
        if (!$binary) {
            $this->error("RestPHP binary not found. Please ensure `restphp` is installed and available in PATH.");
            return 1;
        }

        $workers = $this->option('workers') === 'auto' ? 1 : (int) $this->option('workers');
        $maxRequests = (int) $this->option('max-requests');

        $workerPath = __DIR__ . '/../../bin/restphp-worker.php';
        if (!file_exists($workerPath)) {
            $workerPath = base_path('public/index.php');
        }

        $cmd = [
            $binary,
            'serve',
            '--host', $host,
            '--port', (string) $port,
            '--workers', (string) $workers,
            '--max-requests', (string) $maxRequests,
            '--entrypoint', $workerPath,
        ];

        $process = new Process($cmd, base_path(), [
            'MAX_REQUESTS' => (string) $maxRequests,
            'OCTANE_SERVER' => 'restphp',
        ], null, null);

        $process->start(function ($type, $buffer) {
            $this->output->write($buffer);
        });

        // Register termination handler
        if (function_exists('pcntl_async_signals')) {
            pcntl_async_signals(true);
            pcntl_signal(SIGINT, function () use ($process, $inspector) {
                $this->info("\nStopping RestPHP server...");
                $inspector->stopServer($process);
                exit(0);
            });
            pcntl_signal(SIGTERM, function () use ($process, $inspector) {
                $this->info("\nStopping RestPHP server...");
                $inspector->stopServer($process);
                exit(0);
            });
        }

        while ($process->isRunning()) {
            usleep(500000);
        }

        return $process->getExitCode() ?? 0;
    }

    /**
     * Resolve the RestPHP binary path.
     */
    protected function resolveRestPhpBinary(): ?string
    {
        // 1. Check vendor/bin
        $vendorBin = base_path('vendor/bin/restphp');
        if (file_exists($vendorBin) && is_executable($vendorBin)) {
            return $vendorBin;
        }

        // 2. Check local repo build
        $localBuild = __DIR__ . '/../../../target/release/restphp';
        if (file_exists($localBuild) && is_executable($localBuild)) {
            return $localBuild;
        }

        $localDebug = __DIR__ . '/../../../target/debug/restphp';
        if (file_exists($localDebug) && is_executable($localDebug)) {
            return $localDebug;
        }

        // 3. Search in system PATH
        $output = shell_exec('which restphp 2>/dev/null');
        if ($output) {
            $path = trim($output);
            if (is_executable($path)) {
                return $path;
            }
        }

        return null;
    }
}
