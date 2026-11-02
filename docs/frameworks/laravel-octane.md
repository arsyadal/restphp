# Laravel Octane

RestPHP offers two Laravel execution modes. Standard mode executes `public/index.php` with PHP request startup and shutdown for each HTTP request. The `restphp/octane` adapter boots Laravel once and delegates request handling and sandbox cleanup to Laravel Octane's `Worker` lifecycle.

## Install the adapter

From your Laravel application, make the adapter directory available as a Composer path repository:

```bash
composer config repositories.restphp path /absolute/path/to/restphp/octane
composer require restphp/octane:@dev
```

Composer discovers the package service provider. Laravel Octane is a package dependency; retain the standard Octane configuration and listeners. Use an application and extensions compatible with the installed Octane and Laravel versions.

## Start and manage the server

```bash
php artisan octane:restphp start --binary /absolute/path/to/restphp \
  --host 127.0.0.1 --port 8000 --max-requests 10000

php artisan octane:restphp status
php artisan octane:restphp reload
php artisan octane:restphp stop
```

The start command supervises the native child and keeps a locked state file. Request-limit recycling and reload replace the child process; they do not shut down and reinitialize Zend in place. Stop the supervisor, not an unrelated process discovered through a port probe. Use `--watch` for development source reloads.

The native equivalent is:

```bash
cd /path/to/laravel
restphp serve --persistent-worker vendor/restphp/octane/bin/restphp-worker.php \
  --host 127.0.0.1 --port 8000 --max-requests 10000
```

When invoking the native binary directly, an external process supervisor must restart it after recycling. Native `serve` also accepts the [TLS and admission options](/guide/cli-commands).

## Request lifecycle and limits

The PHP bootstrap loads the application once, boots Octane's worker, and enters the native `restphp_handle_request` loop. Each accepted HTTP request receives a new transport context, request body, superglobals, response headers, status, and output buffer. Octane clones the application sandbox and invokes its configured request and termination listeners.

Application globals, static properties, and references retained by your own code are not automatically made request-local. Follow Octane's application compatibility rules, avoid retaining request objects in long-lived singletons, and use a finite request limit to bound process lifetime.

An unrecoverable native callback failure causes the persistent worker to stop for process replacement rather than continue on possibly corrupted state. Standard mode retains its independent per-request PHP teardown and bailout recovery. NTS PHP permits **one worker per process**; scale with multiple supervised processes.

## Standard Laravel mode

```bash
cd /path/to/laravel
restphp serve --entrypoint public/index.php --port 8000
```

This mode does not require the adapter and does not preserve the Laravel application between requests.
