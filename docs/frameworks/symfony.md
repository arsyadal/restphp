# Symfony Framework Integration

RestPHP runs Symfony applications with high throughput and predictable response latencies.

---

## 🏎️ Running Symfony Applications

To serve a standard Symfony 6.x or 7.x application:

```bash
cd /path/to/symfony-app
restphp -p 8000 -e public/index.php
```

---

## Performance Optimizations for Symfony

### 1. OPcache Preloading
In `php.ini`, enable OPcache preloading targeting Symfony's generated preload script:

```ini
opcache.preload=/path/to/symfony-app/var/cache/prod/App_KernelProdContainer.preload.php
opcache.preload_user=www-data
```

When RestPHP starts, the Zend VM executes the preloaded classes once into shared memory, making class lookups instantaneous across all requests.

### 2. Environment Variables
Ensure `APP_ENV=prod` is set in your environment:

```bash
APP_ENV=prod restphp -p 8000
```
