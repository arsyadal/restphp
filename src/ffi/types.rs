//! C struct definitions and binary layout matches for Zend Engine and PHP SAPI.
//!
//! Verified against PHP 8.4.24 (NTS, 64-bit Linux).

use std::os::raw::{c_char, c_int, c_void};

/// Node in a Zend doubly linked list (`zend_llist_element`).
///
/// Offset 0: `next` (*mut ZendLlistElement)
/// Offset 8: `prev` (*mut ZendLlistElement)
/// Offset 16: Payload `data` begins (variable length based on list configuration).
#[repr(C)]
pub struct ZendLlistElement {
    pub next: *mut ZendLlistElement,
    pub prev: *mut ZendLlistElement,
}

impl ZendLlistElement {
    /// Returns a typed const pointer to the payload data stored at offset 16.
    ///
    /// # Safety
    /// The caller must ensure that this element contains valid payload data of type `T`.
    #[inline]
    pub unsafe fn data_ptr<T>(&self) -> *const T {
        (self as *const Self as *const u8).add(16) as *const T
    }

    /// Returns a typed mutable pointer to the payload data stored at offset 16.
    ///
    /// # Safety
    /// The caller must ensure that this element contains valid payload data of type `T`.
    #[inline]
    pub unsafe fn data_mut<T>(&mut self) -> *mut T {
        (self as *mut Self as *mut u8).add(16) as *mut T
    }
}

/// Zend doubly linked list (`zend_llist`, 56 bytes).
#[repr(C)]
pub struct ZendLlist {
    pub head: *mut ZendLlistElement,
    pub tail: *mut ZendLlistElement,
    pub count: usize,
    pub size: usize,
    pub dtor: Option<unsafe extern "C" fn(*mut c_void)>,
    pub persistent: u8,
    pub _pad: [u8; 7],
    pub traverse_ptr: *mut ZendLlistElement,
}

/// Single HTTP header representation (`sapi_header_struct`, 16 bytes).
#[repr(C)]
pub struct SapiHeader {
    pub header: *mut c_char,
    pub header_len: usize,
}

/// Collected SAPI headers structure (`sapi_headers_struct`, 80 bytes).
#[repr(C)]
pub struct SapiHeaders {
    /// Linked list of `SapiHeader` structures
    pub headers: ZendLlist,
    /// HTTP response code set by `http_response_code()` (0 if unset)
    pub http_response_code: c_int,
    /// Flag indicating whether default Content-Type should be sent
    pub send_default_content_type: u8,
    /// Alignment padding to align pointers to 8 bytes (offsets 61..64)
    pub _padding: [u8; 3],
    /// Explicit MIME type string pointer if set
    pub mimetype: *mut c_char,
    /// Explicit HTTP status line string pointer if set
    pub http_status_line: *mut c_char,
}

/// Opaque representation of PHP's `zval` (16 bytes on 64-bit).
///
/// Used when registering server variables into `$_SERVER`.
#[repr(C)]
pub struct zval {
    pub _opaque: [u8; 16],
}

/// Layout match for `sapi_globals_struct` (648 bytes on 64-bit PHP 8.4 NTS).
///
/// `server_context` is guaranteed to reside at offset 0.
#[repr(C)]
pub struct SapiGlobals {
    /// Opaque pointer to user context (points to `WorkerRequestContext`)
    pub server_context: *mut c_void,
    /// Remaining fields of `sapi_globals_struct` (request_info, headers, etc.)
    pub _rest: [u8; 640],
}

// Zend Result Codes
pub const SUCCESS: c_int = 0;
pub const FAILURE: c_int = -1;

// SAPI Header Return Codes
pub const SAPI_HEADER_SENT_SUCCESSFULLY: c_int = 1;
pub const SAPI_HEADER_DO_SEND: c_int = 2;
pub const SAPI_HEADER_SEND_FAILED: c_int = 3;
