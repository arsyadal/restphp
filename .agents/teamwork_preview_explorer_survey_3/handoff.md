# Zend Engine C FFI Embedding, Custom SAPI & Concurrency Architecture Survey Report

**Agent**: `teamwork_preview_explorer_survey_3`  
**Date**: 2026-09-05  
**Workspace Root**: `/home/cads/restphp`  
**Target File**: `/home/cads/restphp/.agents/teamwork_preview_explorer_survey_3/handoff.md`  

---

## 1. Observation

### 1.1 System PHP Installation & Configuration
Direct inspection of the system environment yielded:
- **PHP Version**: `PHP 8.4.24 (cli) (built: Jul 31 2026 05:11:11) (NTS)`
- **Zend Engine**: `v4.4.24`
- **OPcache**: `Zend OPcache v8.4.24`
- **Thread Safety**: `Thread Safety => disabled` (`NTS` mode)
- **Shared Library**: `/usr/lib/libphp.so` and `/usr/lib/libphp8.4.so` (installed via `libphp8.4-embed`)
- **Headers Path**: `/usr/include/php/20240924`
- **Include Flags (`php-config --includes`)**:
  `-I/usr/include/php/20240924 -I/usr/include/php/20240924/main -I/usr/include/php/20240924/TSRM -I/usr/include/php/20240924/Zend -I/usr/include/php/20240924/ext -I/usr/include/php/20240924/ext/date/lib`
- **Linker Flags (`php-config --ldflags --libs`)**:
  `-L/usr/lib/php/20240924 -lm -lxml2 -lssl -lcrypto -lpcre2-8 -lz -lsodium -largon2 -lrt -ldl`
- **Rust Toolchain**: `rustc 1.98.1` and `cargo 1.98.1` located at `/home/cads/.cargo/bin`

### 1.2 Exported Symbols in `libphp.so`
Using `nm -D /usr/lib/libphp.so`, the following core symbols were confirmed present and dynamically linkable:
- SAPI Lifecycle: `sapi_startup` (0x2cebe0), `sapi_shutdown` (0x2cec50), `sapi_activate` (0x2cf430), `sapi_deactivate` (0x2cf7a0)
- Module Lifecycle: `php_module_startup` (0x2c1430), `php_module_shutdown` (0x2c2440), `php_module_shutdown_wrapper` (0x2c2510)
- Request Lifecycle: `php_request_startup` (0x2c0d10), `php_request_shutdown` (0x2c0fa0)
- Execution: `php_execute_script` (0x2c2a20), `zend_eval_string` (0x3733d0), `zend_eval_stringl` (0x373120), `zend_stream_init_filename` (0x422490), `zend_destroy_file_handle` (0x422460)
- Superglobals: `php_register_variable` (0x2cac40), `php_register_variable_safe` (0x2cab50), `php_default_treat_data` (0x2cb080), `php_default_post_reader` (0x2c6e50)
- Globals: `sapi_globals` (0x5d02a0), `sapi_module` (0x5d0180)

### 1.3 Exact Header Definitions & Offsets
Inspection of `/usr/include/php/20240924/main/SAPI.h` revealed:
- `struct _sapi_module_struct` (lines 237–290):
  ```c
  struct _sapi_module_struct {
      char *name;                                                 /* 0x00 */
      char *pretty_name;                                          /* 0x08 */
      int (*startup)(struct _sapi_module_struct *sapi_module);    /* 0x10 */
      int (*shutdown)(struct _sapi_module_struct *sapi_module);   /* 0x18 */
      int (*activate)(void);                                      /* 0x20 */
      int (*deactivate)(void);                                    /* 0x28 */
      size_t (*ub_write)(const char *str, size_t str_length);     /* 0x30 */
      void (*flush)(void *server_context);                        /* 0x38 */
      zend_stat_t *(*get_stat)(void);                             /* 0x40 */
      char *(*getenv)(const char *name, size_t name_len);         /* 0x48 */
      void (*sapi_error)(int type, const char *error_msg, ...);   /* 0x50 */
      int (*header_handler)(sapi_header_struct *, sapi_header_op_enum, sapi_headers_struct *); /* 0x58 */
      int (*send_headers)(sapi_headers_struct *sapi_headers);     /* 0x60 */
      void (*send_header)(sapi_header_struct *, void *server_context); /* 0x68 */
      size_t (*read_post)(char *buffer, size_t count_bytes);      /* 0x70 */
      char *(*read_cookies)(void);                                /* 0x78 */
      void (*register_server_variables)(zval *track_vars_array);  /* 0x80 */
      void (*log_message)(const char *message, int syslog_type_int); /* 0x88 */
      zend_result (*get_request_time)(double *request_time);      /* 0x90 */
      void (*terminate_process)(void);                            /* 0x98 */
      char *php_ini_path_override;                                /* 0xa0 */
      void (*default_post_reader)(void);                          /* 0xa8 */
      void (*treat_data)(int arg, char *str, zval *destArray);    /* 0xb0 */
      char *executable_location;                                  /* 0xb8 */
      int php_ini_ignore;                                         /* 0xc0 */
      int php_ini_ignore_cwd;                                     /* 0xc4 */
      int (*get_fd)(int *fd);                                     /* 0xc8 */
      int (*force_http_10)(void);                                 /* 0xd0 */
      int (*get_target_uid)(uid_t *);                             /* 0xd8 */
      int (*get_target_gid)(gid_t *);                             /* 0xe0 */
      unsigned int (*input_filter)(...);                          /* 0xe8 */
      void (*ini_defaults)(HashTable *);                          /* 0xf0 */
      int phpinfo_as_text;                                        /* 0xf8 */
      const char *ini_entries;                                    /* 0x100 */
      const zend_function_entry *additional_functions;            /* 0x108 */
      unsigned int (*input_filter_init)(void);                    /* 0x110 */
  };
  ```
- `sapi_globals_struct` (lines 132–151):
  - Offset 0: `void *server_context` — Opaque context pointer passed to callbacks and accessible via `SG(server_context)`.
  - Offset 8: `sapi_request_info request_info` — Contains `request_method`, `query_string`, `cookie_data`, `content_length`, `path_translated`, `request_uri`, `content_type`, etc.
  - Offset 160: `sapi_headers_struct sapi_headers` — Contains `zend_llist headers`, `http_response_code`, `mimetype`, `http_status_line`.

### 1.4 Critical SAPI Pitfalls Identified During Disassembly & Prototyping
1. **Unchecked `read_cookies` Call in `sapi_activate`**:
   Disassembly of `sapi_activate` (`libphp.so` at offset `0x2cf549`):
   ```assembly
   2cf542: mov 0x2e7677(%rip), %rbp # sapi_module
   2cf549: call *0x78(%rbp)         # call sapi_module->read_cookies() WITHOUT checking for NULL!
   2cf54c: mov %rax, 0x18(%rbx)     # store to SG(request_info).cookie_data
   ```
   **Observation**: If `sapi_module.read_cookies` is `NULL`, PHP issues `call *0x0`, crashing with a `SIGSEGV` at RIP `0x0`. Therefore, `sapi_module.read_cookies` **MUST** be implemented (even if it simply returns `NULL`).
2. **Fallback in `sapi_send_headers`**:
   Disassembly of `sapi_send_headers` (`libphp.so` at offset `0x2d0100`-`0x2d01ac`):
   ```assembly
   2d0100: mov 0x60(%rbp), %rax # sapi_module->send_headers
   2d010b: test %rax, %rax
   2d010e: je 2d0180            # if NULL, jump to fallback loop
   2d0117: call *%rax           # call send_headers()
   2d0119: cmp $0x2, %eax       # SAPI_HEADER_DO_SEND == 2
   2d011c: je 2d0180            # if returns 2, jump to fallback loop
   ...
   2d01ac: call *0x68(%rbp)     # fallback loop calls sapi_module->send_header()!
   ```
   **Observation**: If `sapi_module.send_headers` is `NULL` or returns `SAPI_HEADER_DO_SEND` (2), PHP attempts to call `sapi_module.send_header` (`0x68(%rbp)`). If `send_header` is also `NULL`, it crashes with `SIGSEGV` at `0x0`. Therefore, RestPHP must implement `send_headers` and return `SAPI_HEADER_SENT_SUCCESSFULLY` (1).
3. **Bailout / `longjmp` Trap on `exit()` or Fatal Errors**:
   In `Zend/zend.h` lines 270–286, Zend's bailout mechanism uses `setjmp` / `longjmp`:
   ```c
   #define zend_first_try EG(bailout)=NULL; zend_try
   #define zend_try { JMP_BUF *__orig_bailout = EG(bailout); JMP_BUF __bailout; EG(bailout) = &__bailout; if (SETJMP(__bailout)==0) {
   #define zend_catch } else { EG(bailout) = __orig_bailout;
   #define zend_end_try() } EG(bailout) = __orig_bailout; }
   ```
   **Observation**: When a script calls `exit()`, `die()`, or triggers an unhandled fatal error, `_zend_bailout()` performs a `longjmp`. Calling Zend evaluation directly from Rust without a C `zend_first_try` wrapper bypasses Rust stack unwinding (RAII `Drop` implementations), causing memory leaks, mutex lockups, and undefined behavior. A C shim with `zend_first_try` / `zend_catch` is mandatory.

### 1.5 Verification of In-Memory State Isolation Across Requests
A standalone prototype compiled against `/usr/lib/libphp.so` executed 3 consecutive requests (GET with query strings, POST with form-urlencoded, and POST with raw JSON payload):
- Request 1: `GET /api?foo=bar` -> `$_GET['foo'] == 'bar'`
- Request 2: `POST /api` (body `user=restphp_dev&score=99`) -> `$_POST['user'] == 'restphp_dev'`, `$_POST['score'] == '99'`
- Request 3: `POST /api` (JSON body `{"server":"RestPHP"}`) -> `file_get_contents('php://input')` returned raw JSON string.
- In each request, superglobals (`$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`) and Zend MM heap were completely freed and reinitialized during `php_request_shutdown(NULL)` -> `php_request_startup()`. Zero cross-request contamination was observed.

---

## 2. Logic Chain

1. **System PHP is NTS (Non-Thread-Safe)** (from Observation 1.1).
   - In NTS mode, Zend engine globals (`executor_globals`, `compiler_globals`, `sapi_globals`, `core_globals`) are non-thread-local C global variables.
   - Any concurrent execution across multiple OS threads within the same process will corrupt Zend VM state and crash.
   - Therefore, multi-threaded Tokio worker threads cannot invoke Zend Engine APIs concurrently within the same process.
2. **Tokio Runtime is Multi-Threaded & Asynchronous** (from Cargo.toml and system requirements).
   - Network I/O (listening on sockets, parsing HTTP requests, handling TLS) benefits immensely from multi-core async event loops.
   - The boundary between Tokio and the synchronous Zend Engine must be strictly isolated via an Actor / Worker pattern.
   - A dedicated OS worker thread (`std::thread::Builder::new().name("zend-worker")`) hosts the Zend VM, processing requests sequentially from a lock-free Crossbeam channel (`crossbeam_channel::Receiver`).
   - For multi-core scaling, the server can spawn multiple isolated worker processes (Prefork model, like PHP-FPM / RoadRunner), each process hosting an independent Zend VM.
3. **Zend Bailouts (`longjmp`) Invalidate Rust Invariants** (from Observation 1.4).
   - Calling `php_execute_script` or `zend_eval_string` directly from Rust without a C `setjmp` wrapper risks an uncontrolled `longjmp` out of the Rust call stack.
   - A C shim (`c/sapi.c`) wrapping execution with `zend_first_try` / `zend_catch` captures `EG(exit_status)` and returns a structured C integer code back to Rust.
4. **Custom SAPI Implementation Details** (from Observations 1.3, 1.4, 1.5):
   - By leveraging `cc = "1.0"` in `build.rs`, we compile a small C bridge defining `restphp_sapi_module` using the official `STANDARD_SAPI_MODULE_PROPERTIES` macro. This automatically initializes default parsers (`php_default_treat_data`, `php_default_post_reader`).
   - `sapi_module.ub_write`: Extracts `WorkerRequestContext` from `SG(server_context)` and writes bytes directly into a pre-allocated Rust buffer or streams to Tokio.
   - `sapi_module.send_headers`: Traverses the `zend_llist` of `sapi_header_struct`, reads `http_response_code`, and returns `SAPI_HEADER_SENT_SUCCESSFULLY` (1).
   - `sapi_module.read_post`: Reads from the request body buffer provided by Tokio and streams bytes into PHP's input stream.
   - `sapi_module.read_cookies`: Returns `NULL` (or cookie string) to prevent the NULL-dereference bug in `sapi_activate`.
   - `sapi_module.register_server_variables`: Populates `$_SERVER` via `php_register_variable_safe`.

---

## 3. Caveats

1. **NTS In-Process Concurrency Limit**:
   - Because the system PHP is compiled with `Thread Safety => disabled` (NTS), you cannot run multiple threads against Zend Engine inside a single process without causing race conditions on `executor_globals`.
   - In single-process mode, one dedicated worker thread processes requests sequentially.
   - For horizontal scaling across multiple CPU cores, RestPHP should implement a multi-process architecture (master async process + worker processes) or require a ZTS-enabled PHP build (`--enable-zts`).
2. **String Buffer Lifetime for `SG(request_info)`**:
   - Fields such as `request_uri`, `query_string`, and `path_translated` must be allocated by the embedder and remain valid throughout `php_request_startup()` until `php_request_shutdown()`. They must be deallocated *after* `php_request_shutdown()`.
3. **Framework Class Re-Declaration**:
   - If user PHP scripts define global functions or classes in bare scripts evaluated repeatedly without `include_once`/`require_once` guards, running multiple `zend_eval_string` calls within the same request startup block will fail with "Cannot redeclare class/function".
   - Using `php_request_startup()` -> run script -> `php_request_shutdown()` resets all class and function tables between requests, completely avoiding this issue.

---

## 4. Conclusion & Technical Recommendations

### 4.1 Recommended C Shim (`c/sapi.c`)
Create a robust C shim in `c/sapi.c` compiled via `cc` in `build.rs`:

```c
#define _GNU_SOURCE
#include <main/php.h>
#include <main/SAPI.h>
#include <main/php_main.h>
#include <main/php_variables.h>
#include <Zend/zend.h>
#include <Zend/zend_stream.h>

/* Callbacks exported from Rust */
extern size_t restphp_rs_ub_write(const char *str, size_t str_length);
extern void restphp_rs_flush(void *server_context);
extern int restphp_rs_send_headers(sapi_headers_struct *sapi_headers);
extern size_t restphp_rs_read_post(char *buffer, size_t count_bytes);
extern char *restphp_rs_read_cookies(void);
extern void restphp_rs_register_server_variables(zval *track_vars_array);
extern void restphp_rs_log_message(const char *message, int syslog_type_int);

static int restphp_sapi_startup(sapi_module_struct *sapi_module) {
    return php_module_startup(sapi_module, NULL);
}

static int restphp_sapi_shutdown(sapi_module_struct *sapi_module) {
    return php_module_shutdown_wrapper(sapi_module);
}

static sapi_module_struct restphp_sapi_module = {
    "restphp",
    "RestPHP Server SAPI",
    restphp_sapi_startup,
    restphp_sapi_shutdown,
    NULL, /* activate */
    NULL, /* deactivate */
    restphp_rs_ub_write,
    restphp_rs_flush,
    NULL, /* get_stat */
    NULL, /* getenv */
    php_error,
    NULL, /* header_handler */
    restphp_rs_send_headers,
    NULL, /* send_header */
    restphp_rs_read_post,
    restphp_rs_read_cookies,
    restphp_rs_register_server_variables,
    restphp_rs_log_message,
    NULL, /* get_request_time */
    NULL, /* terminate_process */
    STANDARD_SAPI_MODULE_PROPERTIES
};

void restphp_sapi_init(void) {
    sapi_startup(&restphp_sapi_module);
    restphp_sapi_module.startup(&restphp_sapi_module);
}

void restphp_sapi_teardown(void) {
    php_module_shutdown();
    sapi_shutdown();
}

int restphp_eval_string_safe(const char *code, const char *desc) {
    int status = 0;
    zend_first_try {
        status = zend_eval_string((char *)code, NULL, (char *)desc);
    } zend_catch {
        status = EG(exit_status);
    } zend_end_try();
    return status;
}

int restphp_execute_script_safe(const char *filepath) {
    zend_file_handle file_handle;
    zend_stream_init_filename(&file_handle, filepath);
    int status = 0;
    zend_first_try {
        status = php_execute_script(&file_handle) ? 0 : 1;
    } zend_catch {
        status = EG(exit_status);
    } zend_end_try();
    zend_destroy_file_handle(&file_handle);
    return status;
}

void restphp_set_request_info(
    void *server_context,
    const char *method,
    char *uri,
    char *query_string,
    const char *content_type,
    int64_t content_length,
    char *path_translated
) {
    SG(server_context) = server_context;
    SG(request_info).request_method = method;
    SG(request_info).request_uri = uri;
    SG(request_info).query_string = query_string;
    SG(request_info).content_type = content_type;
    SG(request_info).content_length = content_length;
    SG(request_info).path_translated = path_translated;
}
```

### 4.2 Tokio/Hyper Async Integration Architecture
```
┌─────────────────────────────────────────────────────────────┐
│                 Tokio / Hyper / Axum Server                 │
│  - Multi-threaded async network I/O                         │
│  - TCP listener, TLS, keep-alive, protocol parsing          │
└──────────────┬──────────────────────────────▲───────────────┘
               │                              │
     ZendWorkerTask (MPSC)          ZendWorkerResponse (Oneshot)
               │                              │
               ▼                              │
┌─────────────────────────────────────────────┴───────────────┐
│               Dedicated Zend Worker Thread                  │
│  - Hosts Zend VM & Custom SAPI (single-threaded NTS)        │
│                                                             │
│  Request Loop:                                              │
│    1. restphp_set_request_info(ctx, method, uri, query...)  │
│    2. php_request_startup()                                 │
│    3. restphp_execute_script_safe() / eval_safe()           │
│    4. php_request_shutdown(NULL)                            │
│    5. Clean SAPI CStrings & send ZendWorkerResponse         │
└─────────────────────────────────────────────────────────────┘
```

### 4.3 Data Structures for Bridge Context
In Rust (`src/sapi/context.rs`):
```rust
pub struct WorkerRequestContext {
    pub post_body: Bytes,
    pub post_offset: usize,
    pub status_code: u16,
    pub response_headers: Vec<(String, String)>,
    pub output_buffer: Vec<u8>,
    pub server_vars: Vec<(String, String)>,
}
```

Rust SAPI callback implementations (`src/sapi/callbacks.rs`):
```rust
#[no_mangle]
pub unsafe extern "C" fn restphp_rs_ub_write(str: *const c_char, len: usize) -> usize {
    let ctx_ptr = sapi_globals.server_context as *mut WorkerRequestContext;
    if ctx_ptr.is_null() || str.is_null() || len == 0 {
        return 0;
    }
    let slice = std::slice::from_raw_parts(str as *const u8, len);
    (*ctx_ptr).output_buffer.extend_from_slice(slice);
    len
}

#[no_mangle]
pub unsafe extern "C" fn restphp_rs_send_headers(sapi_headers: *mut sapi_headers_struct) -> c_int {
    let ctx_ptr = sapi_globals.server_context as *mut WorkerRequestContext;
    if !ctx_ptr.is_null() && !sapi_headers.is_null() {
        let code = (*sapi_headers).http_response_code;
        (*ctx_ptr).status_code = if code == 0 { 200 } else { code as u16 };
        // Extract headers from zend_llist
        let mut curr = (*sapi_headers).headers.head;
        while !curr.is_null() {
            let h = (*curr).data.as_ptr() as *const sapi_header_struct;
            if !h.is_null() && !(*h).header.is_null() {
                let s = CStr::from_ptr((*h).header).to_string_lossy();
                if let Some((k, v)) = s.split_once(':') {
                    (*ctx_ptr).response_headers.push((k.trim().to_string(), v.trim().to_string()));
                }
            }
            curr = (*curr).next;
        }
    }
    1 // SAPI_HEADER_SENT_SUCCESSFULLY
}

#[no_mangle]
pub unsafe extern "C" fn restphp_rs_read_post(buffer: *mut c_char, count: usize) -> usize {
    let ctx_ptr = sapi_globals.server_context as *mut WorkerRequestContext;
    if ctx_ptr.is_null() || buffer.is_null() || count == 0 {
        return 0;
    }
    let ctx = &mut *ctx_ptr;
    let rem = ctx.post_body.len().saturating_sub(ctx.post_offset);
    if rem == 0 { return 0; }
    let n = rem.min(count);
    std::ptr::copy_nonoverlapping(ctx.post_body.as_ptr().add(ctx.post_offset), buffer as *mut u8, n);
    ctx.post_offset += n;
    n
}

#[no_mangle]
pub unsafe extern "C" fn restphp_rs_read_cookies() -> *mut c_char {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn restphp_rs_register_server_variables(track_vars: *mut zval) {
    let ctx_ptr = sapi_globals.server_context as *mut WorkerRequestContext;
    if ctx_ptr.is_null() { return; }
    let ctx = &*ctx_ptr;
    for (k, v) in &ctx.server_vars {
        let ck = CString::new(k.as_str()).unwrap();
        let cv = CString::new(v.as_str()).unwrap();
        php_register_variable_safe(ck.as_ptr(), cv.as_ptr(), v.len(), track_vars);
    }
}
```

---

## 5. Verification Method

### 5.1 Independent Verification Commands
To independently verify this survey's findings:
1. **Verify PHP Installation and Symbols**:
   ```bash
   php -v
   php-config --version --includes --ldflags --libs
   nm -D /usr/lib/libphp.so | grep -E "sapi_startup|php_request_startup|zend_eval_string"
   ```
2. **Verify C SAPI Compilation & Linking**:
   Run the compiled prototype at `/home/cads/.gemini/antigravity-cli/brain/a4558ccb-428e-4c68-946b-f7225a5997fb/scratch/test_all_safe`:
   ```bash
   /home/cads/.gemini/antigravity-cli/brain/a4558ccb-428e-4c68-946b-f7225a5997fb/scratch/test_all_safe
   ```
   *Expected Output*: Displays clean execution for GET query strings, POST form-urlencoded, POST JSON, and clean shutdown without memory leaks or crashes.
3. **Verify Bailout Catching**:
   ```bash
   /home/cads/.gemini/antigravity-cli/brain/a4558ccb-428e-4c68-946b-f7225a5997fb/scratch/test_bailout
   ```
   *Expected Output*: "Recovered from exit() cleanly!".
4. **Verify Rust Build Toolchain**:
   ```bash
   export PATH="$HOME/.cargo/bin:$PATH"
   rustc -V
   cargo -V
   ```

### 5.2 Invalidation Conditions
This investigation's conclusions would be invalidated if:
- PHP is upgraded to a ZTS-enabled build (which would allow multiple in-process threads to host Zend VMs concurrently).
- PHP 8.5+ modifies `struct _sapi_module_struct` or `sapi_activate` internal calling conventions.
