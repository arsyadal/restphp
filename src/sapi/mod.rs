//! Custom SAPI Subsystem for RestPHP.

pub mod callbacks;
pub mod context;

pub use callbacks::*;
pub use context::*;

use crate::ffi;
use std::ffi::{CString, NulError};
use std::os::raw::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static ENGINE_ACTIVE: AtomicBool = AtomicBool::new(false);

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
    /// Bounded PHP diagnostics for server-side failure reporting, never HTTP output.
    pub diagnostics: Vec<u8>,
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
        if ENGINE_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("Only one NTS PHP VM may be active in a process".to_string());
        }
        if unsafe { ffi::restphp_sapi_init() } != 0 {
            ENGINE_ACTIVE.store(false, Ordering::Release);
            return Err("PHP module startup failed".to_string());
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
        // Prepare every C string before php_request_startup(). Zend can longjmp
        // during startup, so no Rust allocation/error path may happen after the
        // request info pointers are installed.
        let c_method = CString::new(method).map_err(|e| e.to_string())?;
        let c_uri = CString::new(uri).map_err(|e| e.to_string())?;
        let c_query = CString::new(query).map_err(|e| e.to_string())?;
        let c_content_type = match content_type {
            Some(ct) => Some(CString::new(ct).map_err(|e| e.to_string())?),
            None => None,
        };
        let mut c_path = match path_translated {
            Some(p) => Some(php_filename(p).map_err(|e| e.to_string())?),
            None => match target {
                ExecutionTarget::File(p) => Some(php_filename(p).map_err(|e| e.to_string())?),
                _ => None,
            },
        };
        let (c_code, c_desc, c_filepath) = match target {
            ExecutionTarget::Inline(code) | ExecutionTarget::Code(code) => (
                Some(CString::new(code.as_str()).map_err(|e| e.to_string())?),
                Some(CString::new("restphp_eval").expect("static string has no NUL")),
                None,
            ),
            ExecutionTarget::File(path) => (
                None,
                None,
                Some(php_filename(path).map_err(|e| e.to_string())?),
            ),
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
                ffi::restphp_set_cookie_data(std::ptr::null_mut());
                ffi::restphp_clear_request_info();
                return Err("Failed to startup PHP request lifecycle".to_string());
            }

            // Execute target with bailout protection
            let exit_status = match target {
                ExecutionTarget::Inline(_) | ExecutionTarget::Code(_) => {
                    ffi::restphp_eval_string_safe(
                        c_code.as_ref().expect("inline code was prepared").as_ptr(),
                        c_desc
                            .as_ref()
                            .expect("inline descriptor was prepared")
                            .as_ptr(),
                    )
                }
                ExecutionTarget::File(_) => ffi::restphp_execute_script_safe(
                    c_filepath
                        .as_ref()
                        .expect("file path was prepared")
                        .as_ptr(),
                ),
            };

            // Safe shutdown runs Zend GC and dismantles per-request heap
            let shutdown_status = ffi::restphp_request_shutdown_safe();

            // Clear context and cookie data
            ffi::restphp_set_cookie_data(std::ptr::null_mut());
            ffi::restphp_clear_request_info();

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
                diagnostics: std::mem::take(&mut ctx.diagnostics),
                success: exit_status == 0 && shutdown_status == 0,
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
                diagnostics: Vec::new(),
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
                diagnostics: Vec::new(),
                success: false,
            })
    }
}

impl Drop for PhpEngine {
    fn drop(&mut self) {
        unsafe {
            ffi::restphp_sapi_teardown();
        }
        ENGINE_ACTIVE.store(false, Ordering::Release);
    }
}

pub(crate) fn php_filename(path: &Path) -> Result<CString, NulError> {
    let filename = path.to_string_lossy();
    // Rust canonicalization adds Win32 verbatim prefixes that Zend's stream
    // resolver cannot open. Keep Rust's path intact and adapt only the PHP ABI.
    #[cfg(windows)]
    if let Some(std::path::Component::Prefix(prefix)) = path.components().next() {
        match prefix.kind() {
            std::path::Prefix::VerbatimDisk(_) => {
                return CString::new(&filename.as_bytes()[4..]);
            }
            std::path::Prefix::VerbatimUNC(_, _) => {
                return CString::new(format!(r"\\{}", &filename[8..]));
            }
            _ => {}
        }
    }
    CString::new(filename.as_bytes())
}
