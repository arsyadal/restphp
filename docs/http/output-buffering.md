# Output Buffering & Header Capture

RestPHP hooks directly into PHP's low-level Server API (SAPI) callbacks to capture outputs in memory with zero standard I/O overhead.

---

## 1. Zero-Cost Output Buffering (`ub_write`)

In standard CLI PHP, calling `echo`, `print`, or outputting inline HTML writes directly to the operating system's standard output (`stdout` file descriptor). This requires expensive OS system calls (`write()`).

RestPHP intercepts the SAPI `ub_write` callback:
```c
/* c/sapi.c */
static size_t restphp_sapi_ub_write(const char *str, size_t str_length) {
    return restphp_rs_ub_write(str, str_length);
}
```

In Rust (`src/sapi/callbacks.rs`), `restphp_rs_ub_write` appends the bytes directly to the current request's in-memory `Vec<u8>` response buffer:
- **No File Descriptor Overhead**: Zero disk or pipe operations.
- **Binary Safe**: Null bytes and arbitrary binary data (images, PDFs, ZIP archives) are preserved without truncation.

---

## 2. Dynamic Status Codes (`http_response_code`)

When a PHP script sets an HTTP status code:
```php
http_response_code(201); // Created
// or
http_response_code(404); // Not Found
```

RestPHP's SAPI `send_headers` callback intercepts the integer code and forwards it to the Axum HTTP engine, which responds to the client with the matching HTTP status line.

---

## 3. Response Headers (`header()`)

Headers set in PHP via `header()` are captured into a key-value header list:
```php
header("Content-Type: application/json");
header("X-Powered-By: RestPHP");
header("Cache-Control: no-cache");
```

RestPHP automatically translates these headers into Axum HTTP response headers, deduplicating default content types while preserving custom metadata.
