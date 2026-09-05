use crate::ffi;
use std::ffi::{CStr, CString};
use std::ptr;

#[derive(Debug, Clone)]
pub struct PhpResponse {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    pub success: bool,
}

pub struct PhpEngine;

impl PhpEngine {
    pub fn init() -> Result<Self, String> {
        unsafe {
            let res = ffi::restphp_engine_init();
            if res != 0 {
                return Err("Failed to initialize Zend Engine SAPI".to_string());
            }
        }
        Ok(PhpEngine)
    }

    pub fn execute_string(
        &self,
        code: &str,
        method: &str,
        uri: &str,
        query: &str,
        body: &[u8],
    ) -> PhpResponse {
        let c_code = CString::new(code).unwrap_or_default();
        let c_method = CString::new(method).unwrap_or_default();
        let c_uri = CString::new(uri).unwrap_or_default();
        let c_query = CString::new(query).unwrap_or_default();
        let c_body = if body.is_empty() {
            ptr::null()
        } else {
            body.as_ptr() as *const std::os::raw::c_char
        };

        unsafe {
            let mut raw_resp = ffi::restphp_execute_string(
                c_code.as_ptr(),
                c_method.as_ptr(),
                c_uri.as_ptr(),
                c_query.as_ptr(),
                c_body,
                body.len(),
            );

            let status = if raw_resp.status_code > 0 {
                raw_resp.status_code as u16
            } else {
                200
            };

            let content_type = if !raw_resp.content_type.is_null() {
                CStr::from_ptr(raw_resp.content_type)
                    .to_string_lossy()
                    .into_owned()
            } else {
                "text/html; charset=UTF-8".to_string()
            };

            let body_bytes = if !raw_resp.output_bytes.is_null() && raw_resp.output_len > 0 {
                let slice = std::slice::from_raw_parts(
                    raw_resp.output_bytes as *const u8,
                    raw_resp.output_len,
                );
                slice.to_vec()
            } else {
                Vec::new()
            };

            let success = raw_resp.success;
            ffi::restphp_free_response(&mut raw_resp);

            PhpResponse {
                status,
                content_type,
                body: body_bytes,
                success,
            }
        }
    }

    pub fn execute_file(
        &self,
        filepath: &str,
        method: &str,
        uri: &str,
        query: &str,
        body: &[u8],
    ) -> PhpResponse {
        let c_path = CString::new(filepath).unwrap_or_default();
        let c_method = CString::new(method).unwrap_or_default();
        let c_uri = CString::new(uri).unwrap_or_default();
        let c_query = CString::new(query).unwrap_or_default();
        let c_body = if body.is_empty() {
            ptr::null()
        } else {
            body.as_ptr() as *const std::os::raw::c_char
        };

        unsafe {
            let mut raw_resp = ffi::restphp_execute_file(
                c_path.as_ptr(),
                c_method.as_ptr(),
                c_uri.as_ptr(),
                c_query.as_ptr(),
                c_body,
                body.len(),
            );

            let status = if raw_resp.status_code > 0 {
                raw_resp.status_code as u16
            } else {
                200
            };

            let content_type = if !raw_resp.content_type.is_null() {
                CStr::from_ptr(raw_resp.content_type)
                    .to_string_lossy()
                    .into_owned()
            } else {
                "text/html; charset=UTF-8".to_string()
            };

            let body_bytes = if !raw_resp.output_bytes.is_null() && raw_resp.output_len > 0 {
                let slice = std::slice::from_raw_parts(
                    raw_resp.output_bytes as *const u8,
                    raw_resp.output_len,
                );
                slice.to_vec()
            } else {
                Vec::new()
            };

            let success = raw_resp.success;
            ffi::restphp_free_response(&mut raw_resp);

            PhpResponse {
                status,
                content_type,
                body: body_bytes,
                success,
            }
        }
    }
}

impl Drop for PhpEngine {
    fn drop(&mut self) {
        unsafe {
            ffi::restphp_engine_shutdown();
        }
    }
}
