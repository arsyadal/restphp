# RestPHP CLI Commands

RestPHP is distributed as a single standalone executable with built-in commands.

---

## 1. `restphp serve`

Starts the high-performance asynchronous HTTP application server.

```bash
restphp serve [OPTIONS]
```

### Options

| Flag | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--host` | | `0.0.0.0` | IP address to bind to |
| `--port` | `-p` | `8080` | Port to listen on |
| `--entrypoint` | `-e` | `public/index.php` | Path to entrypoint PHP script file |
| `--workers` | `-w` | `1` | Number of persistent worker OS threads |
| `--max-requests`| `-m` | `10000` | Max requests before worker is recycled (0 for unlimited) |

### Examples

```bash
# Serve local Laravel application on port 8000
restphp serve -p 8000 -e /var/www/laravel/public/index.php

# High-concurrency mode with 4 workers on port 80
restphp serve --host 0.0.0.0 --port 80 --workers 4
```

---

## 2. `restphp eval`

Evaluates inline PHP code directly from memory inside a persistent Zend VM context.

```bash
restphp eval "<php_code>"
```

### Examples

```bash
# Print current PHP version
restphp eval 'echo PHP_VERSION . PHP_EOL;'

# Test JSON encoding
restphp eval 'echo json_encode(["status" => "ok", "engine" => "RestPHP"]);'
```

---

## 3. Global Flags

| Flag | Description |
| :--- | :--- |
| `--help` / `-h` | Display help information and subcommands |
| `--version` / `-V` | Display RestPHP version |
