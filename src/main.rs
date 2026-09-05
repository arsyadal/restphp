mod ffi;

use std::ffi::CString;
use std::ptr;

fn main() {
    println!("🦀 [RestPHP] Initializing Zend Engine C-Core directly from Rust...");
    
    unsafe {
        let rc = ffi::php_embed_init(0, ptr::null_mut());
        if rc != 0 {
            eprintln!("❌ [RestPHP] Failed to initialize embedded Zend VM");
            std::process::exit(1);
        }

        let php_code = CString::new(
            "echo '>>> [PHP ' . PHP_VERSION . '] Executing in memory via Zero-Cost Rust FFI!' . PHP_EOL;\n"
        ).unwrap();
        let eval_name = CString::new("restphp_eval").unwrap();

        ffi::zend_eval_string(php_code.as_ptr(), ptr::null_mut(), eval_name.as_ptr());

        ffi::php_embed_shutdown();
    }

    println!("✅ [RestPHP] Zend Engine VM shut down cleanly. Proof of concept successful!");
}
