//! Custom SAPI Subsystem for RestPHP.

pub mod callbacks;
pub mod context;

pub use callbacks::*;
pub use context::*;

use crate::ffi;
use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::path::{Path, PathBuf};

/// Target for script execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionTarget {
    /// Inline PHP source code evaluated in memory
    Inline(String),
    /// Alias for Inline
    Code(String),
    /// PHP script file on disk
    File(PathBuf),
}

/// High-level response returned by `PhpEngine`.
#[derive(Debug, Clone)]
pub struct PhpResponse {
    pub status: u16,
    pub content_type: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub success: bool,
}

/// Safe wrapper managing the Zend Engine SAPI lifecycle.
///
/// Non-`Send` / Non-`Sync` to enforce execution strictly on the dedicated OS worker thread.
pub struct PhpEngine {
    _marker: std::marker::PhantomData<*mut ()>,
}

impl PhpEngine {
    /// Boots the SAPI and initializes the Zend Engine VM.
    ///
    /// Must be called once on the dedicated worker thread.
    pub fn init() -> Result<Self, String> {
        unsafe {
            ffi::restphp_sapi_init();
        }
        tracing::info!("✅ [PhpEngine] RestPHP SAPI initialized successfully.");
        Ok(Self {
            _marker: std::marker::PhantomData,
        })
    }

    /// Executes a complete request lifecycle cycle:
    /// 1. Prepares heap-allocated CStrings for request info.
    /// 2. Sets `SG(server_context)` and `SG(request_info)`.
    /// 3. Executes `php_request_startup()`.
    /// 4. Evaluates string or executes file within bailout protection shield.
    /// 5. Executes `php_request_shutdown(NULL)`.
    /// 6. Clears `SG(server_context)`.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_request(
        &self,
        ctx: &mut WorkerRequestContext,
        target: &ExecutionTarget,
        method: &str,
        uri: &str,
        query: &str,
        content_type: Option<&str>,
        path_translated: Option<&Path>,
    ) -> Result<PhpResponse, String> {
        // Prepare writable CStrings (Zend may mutate these buffers in-place)
        let c_method = CString::new(method).map_err(|e| e.to_string())?;
        let c_uri = CString::new(uri).map_err(|e| e.to_string())?;
        let c_query = CString::new(query).map_err(|e| e.to_string())?;
        let c_content_type = match content_type {
            Some(ct) => Some(CString::new(ct).map_err(|e| e.to_string())?),
            None => None,
        };
        let mut c_path = match path_translated {
            Some(p) => {
                Some(CString::new(p.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?)
            }
            None => match target {
                ExecutionTarget::File(p) => {
                    Some(CString::new(p.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?)
                }
                _ => None,
            },
        };

        let content_len = ctx.post_body.len() as i64;

        unsafe {
            // Associate context with SAPI globals
            ffi::restphp_set_request_info(
                ctx as *mut WorkerRequestContext as *mut c_void,
                c_method.as_ptr(),
                c_uri.as_ptr() as *mut c_char,
                c_query.as_ptr() as *mut c_char,
                c_content_type
                    .as_ref()
                    .map_or(std::ptr::null(), |s| s.as_ptr()),
                content_len,
                c_path
                    .as_mut()
                    .map_or(std::ptr::null_mut(), |s| s.as_ptr() as *mut c_char),
            );

            // Set cookie data if provided
            if let Some(ref cookie) = ctx.raw_cookie {
                ffi::restphp_set_cookie_data(cookie.as_ptr() as *mut c_char);
            }

            // Startup request with bailout protection
            let startup_res = ffi::restphp_request_startup_safe();
            if startup_res != 0 {
                ffi::sapi_globals.server_context = std::ptr::null_mut();
                return Err("Failed to startup PHP request lifecycle".to_string());
            }

            // Execute target with bailout protection
            let exit_status = match target {
                ExecutionTarget::Inline(ref code) | ExecutionTarget::Code(ref code) => {
                    let c_code = CString::new(code.as_str()).map_err(|e| e.to_string())?;
                    let c_desc = CString::new("restphp_eval").unwrap();
                    ffi::restphp_eval_string_safe(c_code.as_ptr(), c_desc.as_ptr())
                }
                ExecutionTarget::File(ref path) => {
                    let c_filepath = CString::new(path.to_string_lossy().as_bytes())
                        .map_err(|e| e.to_string())?;
                    ffi::restphp_execute_script_safe(c_filepath.as_ptr())
                }
            };

            // Safe shutdown runs Zend GC and dismantles per-request heap
            ffi::restphp_request_shutdown_safe();

            // Clear context and cookie data
            ffi::restphp_set_cookie_data(std::ptr::null_mut());
            ffi::sapi_globals.server_context = std::ptr::null_mut();

            let ct = ctx
                .content_type()
                .unwrap_or("text/html; charset=UTF-8")
                .to_string();

            Ok(PhpResponse {
                status: if ctx.status_code == 0 {
                    200
                } else {
                    ctx.status_code
                },
                content_type: ct,
                headers: ctx.response_headers.clone(),
                body: ctx.output_buffer.clone(),
                success: exit_status == 0,
            })
        }
    }

    /// Convenience wrapper evaluating inline PHP code.
    pub fn execute_string(
        &self,
        code: &str,
        method: &str,
        uri: &str,
        query: &str,
        body: &[u8],
    ) -> PhpResponse {
        let mut ctx = WorkerRequestContext::new(bytes::Bytes::copy_from_slice(body), Vec::new());
        let target = ExecutionTarget::Inline(code.to_string());
        self.execute_request(&mut ctx, &target, method, uri, query, None, None)
            .unwrap_or_else(|err| PhpResponse {
                status: 500,
                content_type: "text/plain".to_string(),
                headers: Vec::new(),
                body: err.into_bytes(),
                success: false,
            })
    }

    /// Convenience wrapper executing a PHP script file.
    pub fn execute_file(
        &self,
        filepath: &str,
        method: &str,
        uri: &str,
        query: &str,
        body: &[u8],
    ) -> PhpResponse {
        let mut ctx = WorkerRequestContext::new(bytes::Bytes::copy_from_slice(body), Vec::new());
        let path = PathBuf::from(filepath);
        let target = ExecutionTarget::File(path.clone());
        self.execute_request(&mut ctx, &target, method, uri, query, None, Some(&path))
            .unwrap_or_else(|err| PhpResponse {
                status: 500,
                content_type: "text/plain".to_string(),
                headers: Vec::new(),
                body: err.into_bytes(),
                success: false,
            })
    }
}

impl Drop for PhpEngine {
    fn drop(&mut self) {
        unsafe {
            ffi::restphp_sapi_teardown();
        }
    }
}
