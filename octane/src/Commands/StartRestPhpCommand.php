<?php

namespace RestPhp\Octane\Commands;

use Illuminate\Console\Command;
use RestPhp\Octane\RestPhpServerProcessInspector;
use Symfony\Component\Process\ExecutableFinder;
use Symfony\Component\Process\Process;

class StartRestPhpCommand extends Command
{
    protected $signature = 'octane:restphp
        {action=start : start, status, stop, or reload}
        {--host=127.0.0.1 : Listener address}
        {--port=8000 : Listener port}
        {--binary= : Path to the RestPHP executable}
        {--max-requests=10000 : Requests per process before a clean replacement (0 is unlimited)}
        {--max-queue=256 : Maximum admitted PHP requests, including the executing request}
        {--max-body-bytes=16777216 : Maximum request body size}
        {--shutdown-timeout-secs=30 : Graceful drain deadline}
        {--watch : Watch application files and restart the entire PHP process}
        {--tls-cert= : PEM certificate chain}
        {--tls-key= : PEM private key}
        {--acme-domain=* : DNS names for automatic TLS}
        {--acme-contact=* : ACME contact emails}
        {--acme-cache= : Private certificate and account cache directory}
        {--acme-directory= : Explicit HTTPS directory URL for a private or test ACME CA}
        {--acme-ca-cert= : PEM root CA trusted only for the custom ACME directory}
        {--acme-staging : Use the untrusted ACME staging issuer}';

    protected $description = 'Manage the boot-once RestPHP Octane process supervisor';

    public function handle(RestPhpServerProcessInspector $inspector): int
    {
        if (!extension_loaded('pcntl') || !extension_loaded('posix')) {
            $this->error('RestPHP Artisan supervision requires the pcntl and posix PHP extensions.');
            return self::FAILURE;
        }
        $action = $this->argument('action');
        if ($action === 'status') {
            $state = $inspector->state();
            $this->line($state ? json_encode($state, JSON_PRETTY_PRINT | JSON_THROW_ON_ERROR) : 'RestPHP supervisor is not running.');
            return $state ? self::SUCCESS : self::FAILURE;
        }
        if (in_array($action, ['stop', 'reload'], true)) {
            if (!$inspector->signal($action === 'stop' ? SIGTERM : SIGUSR1)) {
                $this->error('RestPHP supervisor is not running.');
                return self::FAILURE;
            }
            $this->info($action === 'stop' ? 'Graceful stop requested.' : 'Graceful process replacement requested.');
            return self::SUCCESS;
        }
        if ($action !== 'start') {
            $this->error('Action must be start, status, stop, or reload.');
            return self::FAILURE;
        }

        foreach (['port' => [1, 65535], 'max-requests' => [0, PHP_INT_MAX], 'max-queue' => [1, PHP_INT_MAX], 'max-body-bytes' => [1, PHP_INT_MAX], 'shutdown-timeout-secs' => [1, PHP_INT_MAX]] as $name => [$min, $max]) {
            if (filter_var($this->option($name), FILTER_VALIDATE_INT, ['options' => ['min_range' => $min, 'max_range' => $max]]) === false) {
                $this->error("Invalid --{$name} value.");
                return self::FAILURE;
            }
        }
        $binary = $this->option('binary') ?: (new ExecutableFinder())->find('restphp');
        if (!$binary || !is_executable($binary)) {
            $this->error('RestPHP executable not found. Use --binary or add it to PATH.');
            return self::FAILURE;
        }
        $lock = $inspector->acquire();
        $stopping = false;
        $reloading = false;
        pcntl_async_signals(true);
        $previous = [];
        foreach ([SIGINT, SIGTERM, SIGUSR1] as $signal) {
            $previous[$signal] = pcntl_signal_get_handler($signal);
            pcntl_signal($signal, function (int $signal) use (&$stopping, &$reloading): void {
                if ($signal === SIGUSR1) {
                    $reloading = true;
                } else {
                    $stopping = true;
                }
            });
        }
        $command = [$binary, 'serve', '--host', (string) $this->option('host'), '--port', (string) $this->option('port'), '--workers', '1', '--persistent-worker', realpath(__DIR__.'/../../bin/restphp-worker.php')];
        foreach (['max-requests', 'max-queue', 'max-body-bytes', 'shutdown-timeout-secs'] as $name) {
            $command[] = '--'.$name;
            $command[] = (string) $this->option($name);
        }
        if ($this->option('watch')) {
            $command[] = '--watch';
        }
        foreach (['tls-cert', 'tls-key', 'acme-cache', 'acme-directory', 'acme-ca-cert', 'acme-domain', 'acme-contact'] as $name) {
            foreach ((array) $this->option($name) as $value) {
                if ($value !== null && $value !== '') {
                    $command[] = '--'.$name;
                    $command[] = (string) $value;
                }
            }
        }
        if ($this->option('acme-staging')) {
            $command[] = '--acme-staging';
        }

        $process = null;
        try {
            do {
                $reloading = false;
                $process = new Process($command, base_path(), ['APP_BASE_PATH' => base_path(), 'OCTANE_SERVER' => 'restphp'], null, null);
                $process->start(fn ($type, $buffer) => $this->output->write($buffer));
                $inspector->writeState($lock, [
                    'supervisor_pid' => getmypid(),
                    'server_pid' => $process->getPid(),
                    'host' => $this->option('host'),
                    'port' => (int) $this->option('port'),
                    'started_at' => date(DATE_ATOM),
                ]);
                while ($process->isRunning() && !$stopping && !$reloading) {
                    usleep(100000);
                }
                if ($process->isRunning()) {
                    $process->stop((int) $this->option('shutdown-timeout-secs') + 2, SIGTERM);
                }
                $exitCode = $process->getExitCode() ?? self::FAILURE;
                // Successful max-request recycle and explicit reload start a new process.
            } while (!$stopping && ($reloading || $exitCode === 0));
            return $stopping ? self::SUCCESS : $exitCode;
        } finally {
            if ($process?->isRunning()) {
                $process->stop((int) $this->option('shutdown-timeout-secs') + 2, SIGTERM);
            }
            foreach ($previous as $signal => $handler) {
                pcntl_signal($signal, $handler);
            }
            ftruncate($lock, 0);
            flock($lock, LOCK_UN);
            fclose($lock);
        }
    }
}
