# Laravel integration

RestPHP supports serving a conventional Laravel application through `public/index.php`. Laravel Octane boot-once persistence is not implemented in the current runtime; the `restphp/octane` package is experimental and must not be used for production.

---

## Laravel Octane status

The current SAPI bridge does not provide the persistent request callback required to keep a Laravel application booted once. Do not configure `php artisan octane:restphp` as a production server until that bridge exists and is covered by compatibility tests.

## Standard Laravel mode

If you do not wish to install Octane, RestPHP can serve traditional Laravel applications directly via `public/index.php`:

```bash
cd /path/to/laravel
restphp -p 8000
```

Because RestPHP implements a full custom SAPI with automatic superglobal injection, `$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, and `php://input` are automatically populated, allowing Laravel's HTTP Kernel to handle requests seamlessly.
