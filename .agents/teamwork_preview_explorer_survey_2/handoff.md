# Codebase & Environment Survey Report: RestPHP

**Agent**: `teamwork_preview_explorer_survey_2`  
**Date**: 2026-09-05T05:33:30Z  
**Workspace**: `/home/cads/restphp`  
**Status**: Survey Completed

---

## 1. Observation

### 1.1 Toolchain & System Environment
Direct execution of system discovery commands yielded:
- **Rust Toolchain**: Located in `/home/cads/.cargo/bin/`
  - Command: `export PATH="$HOME/.cargo/bin:$PATH"; cargo --version && rustc --version`
  - Output:
    ```
    cargo 1.98.1 (797e8a9bc 2026-08-05)
    rustc 1.98.1 (48a229cea 2026-09-01)
    ```
  - Note: Non-interactive subshell PATH does not include `$HOME/.cargo/bin` by default, so commands must prepend `PATH="$HOME/.cargo/bin:$PATH"`.
- **C Compiler Toolchain**:
  - `/usr/bin/gcc` (GCC on Debian/Linux x86_64)
  - `/usr/bin/clang` (Clang available)
- **PHP CLI & Runtime**:
  - Command: `php -v`
  - Output:
    ```
    PHP 8.4.24 (cli) (built: Jul 31 2026 05:11:11) (NTS)
    Copyright (c) The PHP Group
    Built by Debian
    Zend Engine v4.4.24, Copyright (c) Zend Technologies
        with Zend OPcache v8.4.24, Copyright (c), by Zend Technologies
    ```
- **PHP Configuration (`php-config`)**:
  - Binary: `/usr/bin/php-config`
  - Version: `8.4.24`
  - Prefix: `/usr`
  - Includes: `-I/usr/include/php/20240924 -I/usr/include/php/20240924/main -I/usr/include/php/20240924/TSRM -I/usr/include/php/20240924/Zend -I/usr/include/php/20240924/ext -I/usr/include/php/20240924/ext/date/lib`
  - Ldflags: `-L/usr/lib/php/20240924`
  - Libs: `-lm -lxml2 -lssl -lcrypto -lpcre2-8 -lz -lsodium -largon2 -lrt -ldl`
  - Extension Dir: `/usr/lib/php/20240924`
  - Supported SAPIs: `embed cli`
  - Thread Safety: Non-Thread-Safe (NTS). Verified via `/usr/include/php/20240924/main/php_config.h`: `/* #undef ZTS */`.

### 1.2 Shared Libraries (`libphp`) & Dynamic Symbols
- **Shared Library Location**:
  - `/usr/lib/libphp8.4.so` (size: 6,000,376 bytes)
  - Symlinks: `/usr/lib/libphp.so -> libphp8.4.so` and `/usr/lib/libphp8.so -> /etc/alternatives/libphp8`
- **Exported Symbols** (verified via `nm -D /usr/lib/libphp.so`):
  - Embed API: `php_embed_init` (at `0x4313e0`), `php_embed_shutdown` (at `0x4314b0`), `php_embed_module` (at `0x5b8640`, initialized writable data section `D`)
  - Request Lifecycle: `php_request_startup` (at `0x2c0d10`), `php_request_shutdown` (at `0x2c0fa0`), `php_module_startup` (at `0x2c0b60`), `php_module_shutdown` (at `0x2c10b0`)
  - SAPI Core: `sapi_module` (at `0x5d0180`), `sapi_globals` (at `0x5d02a0`), `sapi_startup` (at `0x2cebe0`), `sapi_shutdown` (at `0x2cec50`), `sapi_activate` (at `0x2cf430`), `sapi_deactivate` (at `0x2cf7a0`), `sapi_header_op` (at `0x2cf7e0`), `sapi_send_headers` (at `0x2d0060`), `sapi_handle_post` (at `0x2ced50`)
  - Zend Evaluation: `zend_eval_string` (at `0x3733d0`), `zend_eval_stringl` (at `0x373120`), `zend_eval_string_ex` (at `0x373430`), `php_execute_script` (at `0x2c2a20`), `zend_execute_scripts` (at `0x430ce0`)
  - Zend GC: `zend_gc_collect_cycles` (at `0x3d4a50`), `zend_gc_get_status` (at `0x3d6570`)

### 1.3 PHP Header Verification & C ABI Memory Layout
Headers reside in `/usr/include/php/20240924/`:
- Key headers present:
  - `main/php.h`: Core macros and Zend VM types
  - `main/SAPI.h`: SAPI definitions (`sapi_module_struct`, `sapi_globals_struct`, `sapi_headers_struct`, `sapi_request_info`)
  - `main/php_variables.h`: `php_register_variable`, `php_register_variable_safe`, `php_register_variable_ex`
  - `sapi/embed/php_embed.h`: `php_embed_init`, `php_embed_shutdown`, `php_embed_module`
  - `Zend/zend.h` & `Zend/zend_compile.h`: Zend compiler and evaluator definitions
- **Verified Struct Sizes & Alignments** (compiled and checked via GCC with PHP headers):
  - `sizeof(sapi_module_struct)`: **280 bytes**
  - `sizeof(sapi_headers_struct)`: **80 bytes**
  - `sizeof(sapi_header_struct)`: **16 bytes**
  - `sizeof(sapi_globals_struct)`: **648 bytes**
  - `sizeof(zval)`: **16 bytes**
  - `sizeof(zend_string)`: **32 bytes**
- **Verified Member Offsets in `sapi_module_struct`** (x86_64, 280 bytes total):
  - `0`: `name` (`*mut c_char`)
  - `8`: `pretty_name` (`*mut c_char`)
  - `16`: `startup` (`Option<unsafe extern "C" fn(*mut sapi_module_struct) -> c_int>`)
  - `24`: `shutdown` (`Option<unsafe extern "C" fn(*mut sapi_module_struct) -> c_int>`)
  - `32`: `activate` (`Option<unsafe extern "C" fn() -> c_int>`)
  - `40`: `deactivate` (`Option<unsafe extern "C" fn() -> c_int>`)
  - `48`: `ub_write` (`Option<unsafe extern "C" fn(*const c_char, size_t) -> size_t>`)
  - `56`: `flush` (`Option<unsafe extern "C" fn(*mut c_void)>`)
  - `64`: `get_stat` (`Option<unsafe extern "C" fn() -> *mut c_void>`)
  - `72`: `getenv` (`Option<unsafe extern "C" fn(*const c_char, size_t) -> *mut c_char>`)
  - `80`: `sapi_error` (`Option<unsafe extern "C" fn(c_int, *const c_char, ...)>`)
  - `88`: `header_handler` (`Option<unsafe extern "C" fn(*mut sapi_header_struct, c_int, *mut sapi_headers_struct) -> c_int>`)
  - `96`: `send_headers` (`Option<unsafe extern "C" fn(*mut sapi_headers_struct) -> c_int>`)
  - `104`: `send_header` (`Option<unsafe extern "C" fn(*mut sapi_header_struct, *mut c_void)>`)
  - `112`: `read_post` (`Option<unsafe extern "C" fn(*mut c_char, size_t) -> size_t>`)
  - `120`: `read_cookies` (`Option<unsafe extern "C" fn() -> *mut c_char>`)
  - `128`: `register_server_variables` (`Option<unsafe extern "C" fn(*mut zval)>`)
  - `136`: `log_message` (`Option<unsafe extern "C" fn(*const c_char, c_int)>`)
  - `144`: `get_request_time` (`Option<unsafe extern "C" fn(*mut f64) -> c_int>`)
  - `152`: `terminate_process` (`Option<unsafe extern "C" fn()>`)
  - `160`: `php_ini_path_override` (`*mut c_char`)
  - `168`: `default_post_reader` (`Option<unsafe extern "C" fn()>`)
  - `176`: `treat_data` (`Option<unsafe extern "C" fn(c_int, *mut c_char, *mut zval)>`)
  - `184`: `executable_location` (`*mut c_char`)
  - `192`: `php_ini_ignore` (`c_int`)
  - `196`: `php_ini_ignore_cwd` (`c_int`)
  - `200`: `get_fd` (`Option<unsafe extern "C" fn(*mut c_int) -> c_int>`)
  - `208`: `force_http_10` (`Option<unsafe extern "C" fn() -> c_int>`)
  - `216`: `get_target_uid` (`Option<unsafe extern "C" fn(*mut uid_t) -> c_int>`)
  - `224`: `get_target_gid` (`Option<unsafe extern "C" fn(*mut gid_t) -> c_int>`)
  - `232`: `input_filter` (`Option<unsafe extern "C" fn(c_int, *const c_char, *mut *mut c_char, size_t, *mut size_t) -> c_uint>`)
  - `240`: `ini_defaults` (`Option<unsafe extern "C" fn(*mut c_void)>`)
  - `248`: `phpinfo_as_text` (`c_int`)
  - `256`: `ini_entries` (`*const c_char`)
  - `264`: `additional_functions` (`*const c_void`)
  - `272`: `input_filter_init` (`Option<unsafe extern "C" fn() -> c_uint>`)

### 1.4 Codebase Structure & Current Build Status
- **Files in Repository**:
  - `Cargo.toml`: 38 lines defining dependencies (`tokio`, `axum`, `hyper`, `hyper-util`, `bytes`, `crossbeam-channel`, `clap`, `tracing`, `tracing-subscriber`, `serde`, `serde_json`, `notify`).
  - `build.rs`: 5 lines emitting `cargo:rustc-link-lib=php` and `cargo:rerun-if-changed=build.rs`.
  - `src/ffi.rs`: 14 lines declaring `ZendResult`, `php_embed_init`, `php_embed_shutdown`, `zend_eval_string`.
  - `src/main.rs`: 28 lines implementing a basic CLI PoC that calls `php_embed_init`, executes an inline string via `zend_eval_string`, and calls `php_embed_shutdown`.
  - `examples/`: Empty directory.
  - `benchmarks/`: Empty directory.
- **Build Status**:
  - Command: `cargo check` -> Success (clean build, 0 warnings).
  - Command: `cargo build` -> Success (`target/debug/restphp` generated in 0.09s).
  - Command: `cargo test` -> Success (0 passed, 0 failed; no unit tests currently written).
  - Command: `cargo run` -> Success:
    ```
    🦀 [RestPHP] Initializing Zend Engine C-Core directly from Rust...
    >>> [PHP 8.4.24] Executing in memory via Zero-Cost Rust FFI!
    ✅ [RestPHP] Zend Engine VM shut down cleanly. Proof of concept successful!
    ```
  - Dynamic linking: `ldd target/debug/restphp` confirms dynamic link to `/lib/libphp.so`, `/lib/x86_64-linux-gnu/libc.so.6`, `/lib/x86_64-linux-gnu/libxml2.so.2`, `/lib/x86_64-linux-gnu/libssl.so.3`, etc.

---

## 2. Logic Chain

### 2.1 Evaluation of Existing State vs. Requirements R1–R4

```
Requirement R1: Zend Engine C-FFI Core Embedding
├── Observation: src/ffi.rs declares php_embed_init, php_embed_shutdown, zend_eval_string.
├── Observation: build.rs links against system libphp via cargo:rustc-link-lib=php.
├── Observation: cargo run successfully executes inline PHP string within embedded Zend VM.
└── Inference: R1 is PARTIALLY IMPLEMENTED (PoC level).
    Missing: Execution of external PHP script files (zend_execute_scripts or file wrapper),
    capturing return values into Rust structures, and complete Zend C FFI declarations.

Requirement R2: Custom SAPI Implementation (sapi_module_struct)
├── Observation: src/ contains only main.rs and ffi.rs.
├── Observation: SAPI module in use is the default php_embed_module from libphp.
├── Observation: Output of zend_eval_string dumps directly to OS stdout via php_embed_module.ub_write.
├── Observation: No custom sapi_module_struct or callbacks (ub_write, sapi_header_op, read_post) exist.
└── Inference: R2 is NOT IMPLEMENTED (Completely missing).

Requirement R3: High-Concurrency Async HTTP Server & Request Dispatch
├── Observation: Cargo.toml includes tokio, axum, hyper, hyper-util, crossbeam-channel, clap.
├── Observation: src/main.rs contains only a synchronous CLI main() with no CLI parsing or network listener.
├── Observation: Acceptance criterion requires `cargo run -- serve --port 8080`.
├── Observation: No request channel or mapping to PHP superglobals ($_SERVER, $_GET, $_POST, $_COOKIE, php://input).
└── Inference: R3 is NOT IMPLEMENTED (Completely missing).

Requirement R4: Persistent Worker Mode & State Reset
├── Observation: src/main.rs runs php_embed_init -> zend_eval_string -> php_embed_shutdown, shutting down the VM immediately.
├── Observation: No persistent request loop calling php_request_startup() -> handle -> php_request_shutdown().
├── Observation: No superglobal state reset or Zend GC invocation (zend_gc_collect_cycles()) between requests.
└── Inference: R4 is NOT IMPLEMENTED (Completely missing).
```

### 2.2 Concurrency & Architecture Logic Chain (NTS Threading Reality)

```
Premise 1: System PHP is PHP 8.4.24 NTS (Non-Thread-Safe), confirmed by php_config.h (/* #undef ZTS */).
Premise 2: In NTS mode, Zend VM internal state (sapi_globals, executor_globals, compiler_globals)
           uses process-global static variables without thread-local storage or TSRM isolation.
Premise 3: Multiple OS threads invoking Zend VM concurrently within the same process in NTS mode
           will corrupt global state and cause memory faults or race conditions.
Conclusion:
RestPHP's execution engine MUST decouple the multi-threaded Tokio async HTTP listener
from Zend Engine execution:
- The Tokio runtime handles concurrent HTTP connections (I/O, parsing, streaming).
- HTTP requests are sent over a lock-free queue (crossbeam_channel) to a dedicated Zend VM worker thread
  (or dedicated worker process pool).
- The Zend VM worker executes requests sequentially:
    sapi_activate() / php_request_startup()
    -> populate $_SERVER, $_GET, $_POST
    -> execute script
    -> php_request_shutdown()
    -> zend_gc_collect_cycles()
- Responses are returned to the async Tokio task via oneshot channels.
```

---

## 3. Caveats

1. **NTS vs ZTS Compatibility**:
   - The current host environment provides only NTS `libphp8.4.so`. Any architectural assumption of spawning multiple concurrent Zend threads in a single process without ZTS is invalid and will crash. Multi-worker concurrency under NTS requires either multi-process workers (e.g. child worker processes) or a single-thread Zend worker actor.
2. **`build.rs` Hardcoding**:
   - The current `build.rs` merely emits `cargo:rustc-link-lib=php`. While this succeeds on Debian systems where `/usr/lib/libphp.so` exists in the system linker path, a production build script should invoke `php-config --ldflags` and `php-config --libs` to support custom installation prefixes.
3. **Embed SAPI vs Full Custom SAPI**:
   - `php_embed_module` is in writable data (`0x5b8640 D php_embed_module`), meaning its function pointers (`ub_write`, etc.) can be overridden dynamically at runtime, OR RestPHP can construct its own standalone static `sapi_module_struct` of 280 bytes and register it with `sapi_startup()`. Both approaches are technically viable, but defining a dedicated `restphp_sapi` struct avoids unintended side-effects from the embed module.

---

## 4. Conclusion

1. **System Readiness**:
   - The development environment is 100% prepared. GCC, Clang, Rust 1.98.1, PHP 8.4.24 development headers (`/usr/include/php/20240924/`), and shared library (`/usr/lib/libphp.so`) are fully operational.
   - Dynamic linking, compilation, and basic Zend Engine VM initialization and execution have been verified.
2. **Implementation Status**:
   - **R1 (C-FFI Embedding)**: 25% complete (minimal PoC compiles and runs). Needs script file execution and structured output capture.
   - **R2 (Custom SAPI)**: 0% complete. Needs `sapi_module_struct` with `ub_write`, `sapi_header_op`, `read_post`.
   - **R3 (Async HTTP Server)**: 0% complete. Needs Clap CLI (`serve --port <p>`), Tokio/Axum/Hyper HTTP front-end, request dispatch channel, and superglobal injector.
   - **R4 (Persistent Worker)**: 0% complete. Needs `php_request_startup()` / `php_request_shutdown()` loop and Zend GC cycle collection.
3. **Recommended Implementation Path for Subsequent Agents**:
   - **Step 1 (FFI & Custom SAPI Subsystem)**: Complete Rust FFI bindings for `sapi_module_struct` (280 bytes), `sapi_globals_struct`, `php_request_startup`, `php_request_shutdown`, `zend_gc_collect_cycles`, and implement Rust callbacks for `ub_write` and `read_post`.
   - **Step 2 (Zend Worker Loop Actor)**: Build a dedicated worker thread hosting the Zend VM, receiving request payloads over `crossbeam_channel::Receiver`, executing the request lifecycle, and returning output via `tokio::sync::oneshot`.
   - **Step 3 (Tokio/Axum HTTP Server)**: Implement `clap` CLI command (`serve --port 8080`), bind Axum/Tokio HTTP router, dispatch HTTP requests to the Zend worker, and stream response status, headers, and body back to the client.
   - **Step 4 (Acceptance Criteria Verification)**: Implement test suite covering GET `/` inline script, POST with query params and body superglobal mapping, and high-concurrency request cycling.

---

## 5. Verification Method

To independently verify all findings in this survey:

1. **Verify Toolchain & PHP Environment**:
   ```bash
   export PATH="$HOME/.cargo/bin:$PATH"
   php-config --version
   php-config --includes
   ls -la /usr/lib/libphp.so
   ```
   *Expected*: Version `8.4.24`, headers in `/usr/include/php/20240924`, library symlink to `libphp8.4.so`.

2. **Verify Struct Size and Offsets**:
   ```bash
   gcc $(php-config --includes) -x c - -o /tmp/verify_sapi << 'EOF'
   #include <main/php.h>
   #include <main/SAPI.h>
   #include <stdio.h>
   int main() {
       printf("sapi_module_struct size=%zu\n", sizeof(sapi_module_struct));
       return 0;
   }
   EOF
   /tmp/verify_sapi && rm /tmp/verify_sapi
   ```
   *Expected*: `sapi_module_struct size=280`.

3. **Verify Rust Build & Existing PoC Execution**:
   ```bash
   export PATH="$HOME/.cargo/bin:$PATH"
   cd /home/cads/restphp
   cargo check
   cargo build
   cargo run
   ```
   *Expected*: Exits 0, prints `>>> [PHP 8.4.24] Executing in memory via Zero-Cost Rust FFI!`.
