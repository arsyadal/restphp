<?php

namespace RestPhp\Octane;

use RuntimeException;

class RestPhpServerProcessInspector
{
    public function __construct(private string $statePath)
    {
    }

    public function acquire()
    {
        $directory = dirname($this->statePath);
        if (!is_dir($directory) && !mkdir($directory, 0755, true) && !is_dir($directory)) {
            throw new RuntimeException('Cannot create RestPHP state directory.');
        }
        $lock = fopen($this->statePath, 'c+');
        if (!$lock || !flock($lock, LOCK_EX | LOCK_NB)) {
            if (is_resource($lock)) {
                fclose($lock);
            }
            throw new RuntimeException('A RestPHP supervisor already owns the state file.');
        }
        return $lock;
    }

    public function writeState($lock, array $state): void
    {
        rewind($lock);
        ftruncate($lock, 0);
        if (fwrite($lock, json_encode($state, JSON_THROW_ON_ERROR)) === false || !fflush($lock)) {
            throw new RuntimeException('Cannot write RestPHP process state.');
        }
    }

    public function state(): ?array
    {
        $file = @fopen($this->statePath, 'r');
        if (!$file) {
            return null;
        }
        try {
            // An unlocked file is stale, even if its recorded PID was reused.
            if (flock($file, LOCK_EX | LOCK_NB)) {
                flock($file, LOCK_UN);
                return null;
            }
            $state = json_decode(stream_get_contents($file), true);
            return is_array($state) && isset($state['supervisor_pid'])
                && posix_kill((int) $state['supervisor_pid'], 0) ? $state : null;
        } finally {
            fclose($file);
        }
    }

    public function signal(int $signal): bool
    {
        $state = $this->state();
        return $state !== null && posix_kill((int) $state['supervisor_pid'], $signal);
    }
}
