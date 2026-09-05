//! Per-request execution context passed across FFI boundaries.

use bytes::Bytes;
use std::ffi::CString;

/// Execution context for an active PHP request.
///
/// Pinned to the worker thread's stack or owned allocation during request execution,
/// and associated with `sapi_globals.server_context` via `restphp_set_request_info`.
#[derive(Debug)]
pub struct WorkerRequestContext {
    /// Inbound HTTP request body (streamed via `read_post` callback into `php://input` or `$_POST`)
    pub post_body: Bytes,
    /// Current read offset within `post_body`
    pub post_offset: usize,
    /// HTTP status code (defaults to 200, updated by `http_response_code()` in `send_headers`)
    pub status_code: u16,
    /// Captured response headers from PHP `header()` calls
    pub response_headers: Vec<(String, String)>,
    /// Accumulated unbuffered script output captured via `ub_write`
    pub output_buffer: Vec<u8>,
    /// CGI environment variables and `HTTP_*` request headers for `$_SERVER`
    pub server_vars: Vec<(String, String)>,
    /// Raw HTTP `Cookie` header string for Zend's automatic cookie parser (`$_COOKIE`)
    pub raw_cookie: Option<CString>,
}

impl WorkerRequestContext {
    /// Creates a new request context with the provided body and server variables.
    pub fn new(post_body: Bytes, server_vars: Vec<(String, String)>) -> Self {
        Self {
            post_body,
            post_offset: 0,
            status_code: 200,
            response_headers: Vec::new(),
            output_buffer: Vec::with_capacity(4096),
            server_vars,
            raw_cookie: None,
        }
    }

    /// Configures the raw cookie string for `$_COOKIE` population.
    pub fn with_cookie(mut self, cookie_str: &str) -> Self {
        if !cookie_str.is_empty() {
            self.raw_cookie = CString::new(cookie_str).ok();
        }
        self
    }

    /// Looks up a captured response header by case-insensitive key name.
    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.response_headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    /// Returns the captured Content-Type header if present.
    pub fn content_type(&self) -> Option<&str> {
        self.get_header("Content-Type")
    }
}
