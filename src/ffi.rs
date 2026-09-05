use std::os::raw::{c_char, c_int};

pub type ZendResult = c_int;

extern "C" {
    pub fn php_embed_init(argc: c_int, argv: *mut *mut c_char) -> c_int;
    pub fn php_embed_shutdown();
    pub fn zend_eval_string(
        str: *const c_char,
        retval_ptr: *mut std::ffi::c_void,
        string_name: *const c_char,
    ) -> ZendResult;
}
