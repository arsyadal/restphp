use std::os::raw::{c_char, c_int};

#[repr(C)]
pub struct RestPhpResponse {
    pub output_bytes: *mut c_char,
    pub output_len: usize,
    pub status_code: c_int,
    pub content_type: *mut c_char,
    pub success: bool,
    pub error_message: *mut c_char,
}

extern "C" {
    pub fn restphp_engine_init() -> c_int;
    pub fn restphp_engine_shutdown();

    pub fn restphp_execute_string(
        php_code: *const c_char,
        method: *const c_char,
        uri: *const c_char,
        query: *const c_char,
        body: *const c_char,
        body_len: usize,
    ) -> RestPhpResponse;

    pub fn restphp_execute_file(
        filepath: *const c_char,
        method: *const c_char,
        uri: *const c_char,
        query: *const c_char,
        body: *const c_char,
        body_len: usize,
    ) -> RestPhpResponse;

    pub fn restphp_free_response(resp: *mut RestPhpResponse);
}
