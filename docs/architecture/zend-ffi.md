# Zend Engine C-FFI Embedding

RestPHP embeds the Zend VM directly using raw C Application Binary Interface (`extern "C"`) declarations.

---

## Zero-Cost C-ABI Linkage

Unlike Go (which incurs ~60ns per C function call via `cgo`), Rust compiles to native machine code that shares the standard POSIX C ABI.

In `src/ffi/mod.rs`:
```rust
extern "C" {
    pub fn php_request_startup() -> libc::c_int;
    pub fn php_request_shutdown(dummy: *mut libc::c_void);
    pub fn zend_eval_string(
        str: *const libc::c_char,
        retval_ptr: *mut types::zval,
        string_name: *const libc::c_char,
    ) -> libc::c_int;
    pub fn zend_gc_collect_cycles() -> libc::c_int;
}
```

Calls to these functions execute with **zero additional nanoseconds of latency** — identical to calling C functions from C.

---

## Build System Linkage (`build.rs`)

During `cargo build`, `build.rs` runs `php-config`:
- `--includes`: Identifies header search paths (`/usr/include/php/20240924/`).
- `--ldflags` & `--libs`: Links against `libphp.so` embedded library.
- Compiles `c/sapi.c` via the `cc` crate into a static C object linked into the final binary.
