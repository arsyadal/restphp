# Superglobal Mapping

RestPHP maps all standard HTTP protocol structures directly into native PHP superglobals with zero data corruption.

---

## 1. `$_SERVER`

RestPHP populates CGI and HTTP headers into `$_SERVER` using PHP's internal `php_register_variable_safe()` API:

| HTTP Request Property | `$_SERVER` Key | Example Value |
| :--- | :--- | :--- |
| HTTP Method | `REQUEST_METHOD` | `"POST"` |
| Request Path | `REQUEST_URI` | `"/api/users"` |
| Query String | `QUERY_STRING` | `"role=admin&active=1"` |
| HTTP Protocol | `SERVER_PROTOCOL` | `"HTTP/1.1"` |
| Content-Type | `CONTENT_TYPE` | `"application/json"` |
| Content-Length | `CONTENT_LENGTH` | `"1024"` |
| Custom Header `X-Custom-Token` | `HTTP_X_CUSTOM_TOKEN` | `"secret123"` |

Headers are automatically capitalized and prefixed with `HTTP_` according to CGI RFC 3875 standards.

---

## 2. `$_GET`

Query parameters are forwarded directly to the SAPI request context:
- URL-encoded strings (e.g. `%20`, `%26`) are automatically decoded by Zend's standard query parser.
- Query arrays (e.g. `?filter[status]=active&filter[type]=user`) are parsed into native multidimensional PHP arrays.

---

## 3. `$_POST` and `php://input`

RestPHP implements the SAPI `read_post` callback:
- **`application/x-www-form-urlencoded`**: The body is parsed automatically into `$_POST`.
- **`application/json` & Binary Payloads**: Raw request payloads are streamable via `file_get_contents('php://input')` or `fopen('php://input', 'rb')` with zero truncation.

---

## 4. `$_COOKIE`

The incoming HTTP `Cookie` header is supplied to Zend via the SAPI `read_cookies` callback:
- Multiple cookies separated by semicolons are parsed directly into `$_COOKIE`.
- Missing or malformed cookie headers are safely ignored with zero null pointer dereferences.
