//! Rust callback implementations registered with `restphp_sapi_module`.
//!
//! These functions are declared as `extern` in `c/sapi.c` and exported from Rust
//! via `#[no_mangle] pub unsafe extern "C"`.

use crate::ffi::{
    php_register_variable_safe, sapi_globals, zval, SapiHeader, SapiHeaders,
    SAPI_HEADER_SENT_SUCCESSFULLY,
};
use crate::sapi::context::WorkerRequestContext;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};

/// Intercepts script output (`echo`, `print`, `var_dump`, inline HTML).
///
/// Appends bytes directly to `WorkerRequestContext.output_buffer`.
///
/// # Safety
/// Called by Zend VM with a valid string pointer and length.
/// `sapi_globals.server_context` must point to a valid `WorkerRequestContext`.
#[no_mangle]
pub unsafe extern "C" fn restphp_rs_ub_write(str: *const c_char, str_length: usize) -> usize {
    let ctx_ptr = sapi_globals.server_context as *mut WorkerRequestContext;
    if ctx_ptr.is_null() || str.is_null() || str_length == 0 {
        return 0;
    }

    let slice = std::slice::from_raw_parts(str as *const u8, str_length);
    (*ctx_ptr).output_buffer.extend_from_slice(slice);
    str_length
}

/// SAPI flush callback.
///
/// # Safety
/// Called by Zend VM during output flush operations.
#[no_mangle]
pub unsafe extern "C" fn restphp_rs_flush(server_context: *mut c_void) {
    let ctx_ptr = if !server_context.is_null() {
        server_context as *mut WorkerRequestContext
    } else {
        sapi_globals.server_context as *mut WorkerRequestContext
    };

    if !ctx_ptr.is_null() {
        tracing::trace!("SAPI flush triggered for request");
    }
}

/// Captures HTTP response code and response headers from Zend Engine.
///
/// CRITICAL: Must always return `SAPI_HEADER_SENT_SUCCESSFULLY` (1) to prevent
/// Zend from attempting a fallback call to `sapi_module.send_header` (which is NULL).
///
/// # Safety
/// Called by Zend SAPI header sender. `sapi_headers` must point to a valid Zend SAPI headers structure.
#[no_mangle]
pub unsafe extern "C" fn restphp_rs_send_headers(sapi_headers: *mut SapiHeaders) -> c_int {
    let ctx_ptr = sapi_globals.server_context as *mut WorkerRequestContext;
    if !ctx_ptr.is_null() && !sapi_headers.is_null() {
        let ctx = &mut *ctx_ptr;

        // Capture status code (0 means PHP did not set a custom code, default 200)
        let code = (*sapi_headers).http_response_code;
        if code > 0 {
            ctx.status_code = code as u16;
        }

        // Traverse headers linked list (`zend_llist`)
        let mut curr = (*sapi_headers).headers.head;
        while !curr.is_null() {
            let header_ptr = (*curr).data_ptr::<SapiHeader>();
            if !header_ptr.is_null() && !(*header_ptr).header.is_null() {
                let header_str = CStr::from_ptr((*header_ptr).header).to_string_lossy();
                if let Some((k, v)) = header_str.split_once(':') {
                    ctx.response_headers
                        .push((k.trim().to_string(), v.trim().to_string()));
                }
            }
            curr = (*curr).next;
        }

        // If mimetype is set and Content-Type was not in headers, record it
        if !(*sapi_headers).mimetype.is_null() && ctx.content_type().is_none() {
            let mime = CStr::from_ptr((*sapi_headers).mimetype).to_string_lossy();
            ctx.response_headers
                .push(("Content-Type".to_string(), mime.into_owned()));
        }
    }

    SAPI_HEADER_SENT_SUCCESSFULLY
}

/// Streams request body bytes into Zend Engine for `$_POST` and `php://input`.
///
/// Called iteratively by `sapi_read_post_block` in 16KB blocks until 0 is returned.
///
/// # Safety
/// Called by Zend SAPI POST reader with a writable buffer of size `count_bytes`.
#[no_mangle]
pub unsafe extern "C" fn restphp_rs_read_post(buffer: *mut c_char, count_bytes: usize) -> usize {
    let ctx_ptr = sapi_globals.server_context as *mut WorkerRequestContext;
    if ctx_ptr.is_null() || buffer.is_null() || count_bytes == 0 {
        return 0;
    }

    let ctx = &mut *ctx_ptr;
    let remaining = ctx.post_body.len().saturating_sub(ctx.post_offset);
    if remaining == 0 {
        return 0;
    }

    let to_copy = remaining.min(count_bytes);
    std::ptr::copy_nonoverlapping(
        ctx.post_body.as_ptr().add(ctx.post_offset),
        buffer as *mut u8,
        to_copy,
    );
    ctx.post_offset += to_copy;
    to_copy
}

/// Supplies raw cookie string for Zend's automatic cookie parser (`$_COOKIE`).
///
/// CRITICAL: In PHP 8.4, `sapi_activate` invokes `read_cookies` unconditionally
/// without checking for NULL. This function must NEVER be a NULL function pointer.
/// Returning NULL from this function safely leaves `$_COOKIE` empty.
///
/// # Safety
/// Called by Zend `sapi_activate` during request activation.
#[no_mangle]
pub unsafe extern "C" fn restphp_rs_read_cookies() -> *mut c_char {
    let ctx_ptr = sapi_globals.server_context as *mut WorkerRequestContext;
    if ctx_ptr.is_null() {
        return std::ptr::null_mut();
    }

    match &(*ctx_ptr).raw_cookie {
        Some(cookie) => cookie.as_ptr() as *mut c_char,
        None => std::ptr::null_mut(),
    }
}

/// Populates `$_SERVER` superglobal array with CGI variables and HTTP headers.
///
/// # Safety
/// Called by Zend SAPI variable registration. `track_vars_array` must point to valid PHP array zval.
#[no_mangle]
pub unsafe extern "C" fn restphp_rs_register_server_variables(track_vars_array: *mut zval) {
    let ctx_ptr = sapi_globals.server_context as *mut WorkerRequestContext;
    if ctx_ptr.is_null() || track_vars_array.is_null() {
        return;
    }

    let ctx = &*ctx_ptr;
    for (k, v) in &ctx.server_vars {
        if let (Ok(ck), Ok(cv)) = (CString::new(k.as_str()), CString::new(v.as_str())) {
            php_register_variable_safe(ck.as_ptr(), cv.as_ptr(), v.len(), track_vars_array);
        }
    }
}

/// Intercepts PHP log messages and routes them through the Rust `tracing` subsystem.
///
/// # Safety
/// Called by Zend error logging routines with a null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn restphp_rs_log_message(message: *const c_char, syslog_type_int: c_int) {
    if message.is_null() {
        return;
    }

    let msg = CStr::from_ptr(message).to_string_lossy();
    match syslog_type_int {
        1 | 2 => tracing::error!(target: "php", "{}", msg),
        3 | 4 => tracing::warn!(target: "php", "{}", msg),
        5 | 6 => tracing::info!(target: "php", "{}", msg),
        _ => tracing::debug!(target: "php", "[type={}] {}", syslog_type_int, msg),
    }
}
