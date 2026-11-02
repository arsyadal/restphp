# Request Lifecycle and State Reset

The isolation boundary depends on execution mode. Keeping the Zend module alive does not, by itself, keep a Laravel application alive.

## Script mode

For each HTTP request, the worker installs request metadata, starts a PHP request, executes the entrypoint, shuts down the PHP request, and clears the context. PHP destroys that request's symbol table and heap. Superglobals and userland globals do not survive into the next script execution.

Script execution and teardown use C-side Zend bailout guards. A failed request is returned as a generic HTTP 500, with bounded diagnostics logged server-side. Normal per-request teardown allows subsequent script requests to execute after a PHP fatal error.

## Persistent callback mode

`--persistent-worker` starts one PHP request for the lifetime of the bootstrap script. The script boots its application and repeatedly calls `restphp_handle_request($handler)`. The native bridge waits for admitted HTTP jobs and invokes the callback on the same NTS worker thread.

The HTTP boundary is refreshed between callbacks: request metadata, superglobals, input body, cookies, headers, status, and captured output belong to the current job. Response failures must not expose partial output or diagnostics to clients. Zend operations that can bail out remain inside C guards, never unwinding through Rust frames.

Built-in session and filter request hooks are refreshed between callbacks so session IDs and `filter_input()` data do not carry into the next request. Register request-specific native session handlers inside the callback, not during bootstrap. Streams or other objects deliberately retained by userland are still long-lived application state; release references when they are no longer needed.

**This is not full userland memory isolation.** Arbitrary globals, static properties, singleton references, and framework objects remain alive unless the application or framework resets them. In the Laravel adapter, Octane's worker creates the application sandbox and runs its request/termination listeners. Follow Octane's rules for long-lived services; do not retain request-specific objects in persistent singletons.

A persistent worker cannot safely recover arbitrary application state after an unrecoverable callback bailout. It closes admission and stops for process replacement instead of handling another request on that state.

## Draining and replacement

SIGTERM, Ctrl-C, and request-limit recycling close admission, settle accepted jobs, and stop the worker. An idle callback wait is woken during shutdown. A finite shutdown deadline terminates a process whose PHP execution cannot finish safely.

Hot reload replaces the whole child process after draining. It does not attempt Zend module shutdown followed by startup in the same process. See [persistent workers](/runtime/persistent-workers) and [CLI options](/guide/cli-commands).
