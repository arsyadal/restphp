# Custom SAPI Bridge

RestPHP implements a dedicated Server Application Programming Interface (SAPI) defined in `c/sapi.c` and hooked into Rust callbacks in `src/sapi/callbacks.rs`.

---

## The `sapi_module_struct` Implementation

In `c/sapi.c`, the `restphp_sapi_module` structure overrides key Zend SAPI hooks:

```c
static sapi_module_struct restphp_sapi_module = {
    "restphp",                             /* name */
    "RestPHP High-Performance Server",     /* pretty name */
    restphp_sapi_startup,                  /* startup */
    restphp_sapi_shutdown,                 /* shutdown */
    NULL,                                  /* activate */
    restphp_sapi_deactivate,               /* deactivate */
    restphp_sapi_ub_write,                 /* ub_write */
    restphp_sapi_flush,                    /* flush */
    NULL,                                  /* get_stat */
    NULL,                                  /* getenv */
    php_error,                             /* sapi_error */
    NULL,                                  /* header_handler */
    restphp_sapi_send_headers,             /* send_headers */
    NULL,                                  /* send_header */
    restphp_sapi_read_post,                /* read_post */
    restphp_sapi_read_cookies,             /* read_cookies */
    restphp_rs_register_server_variables,  /* register_server_variables */
    restphp_rs_log_message,                /* log_message */
    // ...
};
```

---

## Safe Bailout Protection

If a PHP script triggers an unhandled exception or calls `exit()`, PHP triggers a `longjmp` bailout.

RestPHP wraps script execution inside Zend's exception handler:
```c
int restphp_execute_script_safe(const char *filepath) {
    int status = 0;
    zend_first_try {
        zend_file_handle file_handle;
        zend_stream_init_filename(&file_handle, filepath);
        status = zend_execute_scripts(ZEND_REQUIRE, NULL, 1, &file_handle);
        zend_destroy_file_handle(&file_handle);
    } zend_catch {
        status = EG(exit_status);
    } zend_end_try();
    return status;
}
```

This prevents unhandled script terminations from crashing the worker thread or corrupting server memory.
