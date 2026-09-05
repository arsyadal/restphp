# RestPHP CLI Commands

RestPHP is distributed as a single standalone executable with built-in commands. The CLI supports zero-config Bun-style execution.

---

## 1. Zero-Config Serve

Starts the high-performance asynchronous HTTP application server. RestPHP auto-detects common entrypoints like `public/index.php` or `index.php`.

```bash
restphp [ENTRYPOINT] [OPTIONS]
```

### Options

| Flag | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--host` | | `0.0.0.0` | IP address to bind to |
| `--port` | `-p` | `8080` | Port to listen on |
| `--workers` | `-w` | `1` | Number of persistent worker OS threads |
| `--max-requests`| `-m` | `10000` | Max requests before worker is recycled (0 for unlimited) |

### Examples

```bash
# Auto-detect entrypoint and start server
restphp

# Serve specific script on port 3000
restphp app.php -p 3000

# High-concurrency mode with 4 workers on port 80
restphp --host 0.0.0.0 -p 80 -w 4
```

---

## 2. Evaluate Code (`-e`)

Evaluates inline PHP code directly from memory inside a persistent Zend VM context.

```bash
restphp -e "<php_code>"
```

### Examples

```bash
# Print current PHP version
restphp -e 'echo PHP_VERSION . PHP_EOL;'

# Test JSON encoding
restphp -e 'echo json_encode(["status" => "ok", "engine" => "RestPHP"]);'
```

---

## 3. Global Flags

| Flag | Description |
| :--- | :--- |
| `--help` / `-h` | Display help information |
| `--version` / `-V` | Display RestPHP version |
