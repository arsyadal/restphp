//! C-FFI bindings linking to `libphp.so` and the custom `c/sapi.c` shim.

pub mod types;
pub use types::*;

use std::os::raw::{c_char, c_int, c_void};

extern "C" {
    // =========================================================================
    // Custom SAPI Shim Lifecycle Functions (implemented in c/sapi.c)
    // =========================================================================

    /// Initializes RestPHP SAPI module and boots PHP module subsystem.
    pub fn restphp_sapi_init();

    /// Shuts down PHP module subsystem and SAPI module.
    pub fn restphp_sapi_teardown();

    /// Configures `SG(server_context)` and `SG(request_info)` for an incoming request.
    ///
    /// # Safety
    /// - `server_context` must point to a valid `WorkerRequestContext`.
    /// - Pointer strings (`uri`, `query_string`, etc.) must remain valid and writable
    ///   until `php_request_shutdown` finishes.
    pub fn restphp_set_request_info(
        server_context: *mut c_void,
        method: *const c_char,
        uri: *mut c_char,
        query_string: *mut c_char,
        content_type: *const c_char,
        content_length: i64,
        path_translated: *mut c_char,
    );

    /// Configures `SG(request_info).cookie_data`.
    pub fn restphp_set_cookie_data(cookie_data: *mut c_char);

    /// Safely prepares per-request memory manager and activates SAPI with bailout protection.
    pub fn restphp_request_startup_safe() -> c_int;

    /// Safely terminates per-request memory manager with bailout protection.
    pub fn restphp_request_shutdown_safe() -> c_int;

    /// Safely evaluates an in-memory PHP string wrapped with `zend_first_try` / `zend_catch`.
    ///
    /// Returns 0 on success, or PHP `exit_status` on bailout/error.
    pub fn restphp_eval_string_safe(code: *const c_char, desc: *const c_char) -> c_int;

    /// Safely executes a `.php` script file wrapped with `zend_first_try` / `zend_catch`.
    ///
    /// Returns 0 on success, or PHP `exit_status` on bailout/error.
    pub fn restphp_execute_script_safe(filepath: *const c_char) -> c_int;

    /// Returns current `SG(server_context)` pointer.
    pub fn restphp_get_server_context() -> *mut c_void;

    /// Helper for Rust to register $_SERVER variables safely into track_vars_array.
    pub fn restphp_register_variable(
        var: *const c_char,
        val: *const c_char,
        val_len: usize,
        track_vars_array: *mut c_void,
    );

    // =========================================================================
    // Zend Engine & PHP Core Symbols (exported from libphp.so)
    // =========================================================================

    /// Direct request startup symbol from `libphp.so`.
    pub fn php_request_startup() -> c_int;

    /// Direct request shutdown symbol from `libphp.so`.
    pub fn php_request_shutdown(dummy: *mut c_void);

    /// Forces Zend garbage collector to collect cyclic garbage.
    pub fn zend_gc_collect_cycles() -> c_int;

    /// Binary-safe registration of a variable into a PHP array (e.g. `$_SERVER`).
    pub fn php_register_variable_safe(
        var: *const c_char,
        val: *const c_char,
        val_len: usize,
        track_vars_array: *mut zval,
    );

    /// Null-terminated variable registration into a PHP array.
    pub fn php_register_variable(
        var: *const c_char,
        val: *const c_char,
        track_vars_array: *mut zval,
    );

    // =========================================================================
    // Zend Engine SAPI Globals (exported from libphp.so)
    // =========================================================================

    /// Global SAPI state in Non-Thread-Safe (NTS) mode.
    pub static mut sapi_globals: SapiGlobals;
}
