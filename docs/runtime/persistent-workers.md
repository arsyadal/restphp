# Persistent Worker Architecture

RestPHP hosts one NTS Zend VM on one dedicated OS thread per process. The HTTP listener runs on Tokio; admitted jobs cross a bounded channel and return responses through oneshot channels. `--workers` must be `1`. Scale with multiple supervised processes rather than multiple NTS VMs in a process.

## Execution modes

| Mode | Startup | Per HTTP request | Teardown |
| :--- | :--- | :--- | :--- |
| Script, `--entrypoint` | Initialize PHP module once | PHP request startup, execute script, PHP request shutdown | Stop module after queue drains |
| Persistent, `--persistent-worker` | Initialize module, start PHP request, boot script once | Invoke `restphp_handle_request` callback with fresh HTTP context | Finish loop, shut down PHP request and module |

A persistent script must reach its first callback before the runtime reports readiness. For Laravel, use the [Octane adapter](/frameworks/laravel-octane), which supplies the framework request sandbox and lifecycle listeners. A custom callback retains its application objects by design; see [state isolation](/runtime/state-reset).

## Admission and shutdown

`--max-queue` bounds requests waiting for or executing on PHP. Full queues return HTTP 503 rather than blocking Tokio threads. `--max-body-bytes` rejects excess bodies with HTTP 413 before dispatch.

SIGTERM and Ctrl-C close admission, drain accepted requests, wake an idle persistent worker, and join the worker thread. An unrecoverable persistent callback failure stops that worker rather than continuing on potentially corrupted application state. Run RestPHP under a supervisor for replacement.

## Recycling

```bash
restphp serve --entrypoint public/index.php --max-requests 10000
restphp serve --persistent-worker worker.php --max-requests 10000
```

`--max-requests` limits accepted HTTP requests; `0` means unlimited. At the limit, the process closes admission, finishes admitted work, and exits. Recycling replaces the whole process: Zend module shutdown and startup in the same process are not used as a reload strategy.

`--shutdown-timeout-secs` bounds the drain and worker join, starting when shutdown begins. Completion exits `0`; deadline expiry exits `1`. Configure your supervisor to restart after either outcome. Development `--watch` handles source-triggered replacement and successful request-limit recycling itself.

## Mixed-traffic soak validation

```bash
python3 tests/run_soak.py --binary target/release/restphp \
  --duration-secs 1800 --recycle-interval-secs 600 --output scratch/soak-report.json
```

`tests/run_soak.py` exercises mixed plain, JSON, binary, form, multipart upload, session, recoverable exception, active client-disconnect, and process-replacement workloads against `--persistent-worker`. It checks peak RSS and open file descriptor growth across samples, verifies upload temp-file cleanup, confirms active requests complete during SIGTERM draining, and verifies that an unrecoverable callback failure stops the worker without exposing partial output.
