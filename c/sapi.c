#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
#include "sapi.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>

#include <main/php.h>
#include <main/SAPI.h>
#include <main/php_main.h>
#include <main/php_variables.h>
#include <main/php_content_types.h>
#include <main/rfc1867.h>
#include <Zend/zend.h>
#include <Zend/zend_signal.h>
#include <Zend/zend_stream.h>
#include <Zend/zend_gc.h>
#include <Zend/zend_execute.h>
#include <Zend/zend_exceptions.h>

/*
 * External Rust Callbacks (Exported from src/sapi/callbacks.rs)
 */
extern size_t restphp_rs_ub_write(const char *str, size_t str_length);
extern void   restphp_rs_flush(void *server_context);
extern int    restphp_rs_send_headers(sapi_headers_struct *sapi_headers);
extern size_t restphp_rs_read_post(char *buffer, size_t count_bytes);
extern char  *restphp_rs_read_cookies(void);
extern void   restphp_rs_register_server_variables(zval *track_vars_array);
extern void   restphp_rs_log_message(const char *message, int syslog_type_int);

/*
 * SAPI Internal Callbacks & Wrappers
 */

static int restphp_sapi_startup(sapi_module_struct *sapi_module) {
    (void)sapi_module;
    return SUCCESS;
}

/*
 * send_headers wrapper:
 * Invokes Rust callback to capture response status code and headers,
 * and GUARANTEES returning SAPI_HEADER_SENT_SUCCESSFULLY (1).
 * If 2 or anything else is returned, Zend falls back to calling send_header,
 * which is NULL and causes SIGSEGV.
 */
static int restphp_sapi_send_headers(sapi_headers_struct *sapi_headers) {
    if (sapi_headers) {
        restphp_rs_send_headers(sapi_headers);
    }
    return SAPI_HEADER_SENT_SUCCESSFULLY;
}

/*
 * read_cookies wrapper:
 * MUST NEVER be NULL in sapi_module_struct because sapi_activate() calls
 * sapi_module->read_cookies() unconditionally via `call *0x78(%rbp)`.
 */
static char *restphp_sapi_read_cookies(void) {
    char *cookies = restphp_rs_read_cookies();
    if (cookies) {
        return cookies;
    }
    return SG(request_info).cookie_data;
}

/*
 * Supported POST content-types registration table
 */
static sapi_post_entry restphp_post_entries[] = {
    { DEFAULT_POST_CONTENT_TYPE, sizeof(DEFAULT_POST_CONTENT_TYPE) - 1, sapi_read_standard_form_data, php_std_post_handler },
    { MULTIPART_CONTENT_TYPE, sizeof(MULTIPART_CONTENT_TYPE) - 1, NULL, rfc1867_post_handler },
    { NULL, 0, NULL, NULL }
};

/*
 * Dedicated SAPI Module Struct (280 bytes on x86_64)
 */
static sapi_module_struct restphp_sapi_module = {
    "restphp",                             /* name */
    "RestPHP Server SAPI",                 /* pretty_name */
    restphp_sapi_startup,                  /* startup */
    php_module_shutdown_wrapper,           /* shutdown */
    NULL,                                  /* activate */
    NULL,                                  /* deactivate */
    restphp_rs_ub_write,                   /* ub_write */
    restphp_rs_flush,                      /* flush */
    NULL,                                  /* get_stat */
    NULL,                                  /* getenv */
    NULL,                                  /* sapi_error: NULL routes through php_error_cb to ub_write */
    NULL,                                  /* header_handler */
    restphp_sapi_send_headers,             /* send_headers */
    NULL,                                  /* send_header (intentionally NULL) */
    restphp_rs_read_post,                  /* read_post */
    restphp_sapi_read_cookies,             /* read_cookies (NEVER NULL) */
    restphp_rs_register_server_variables,  /* register_server_variables */
    restphp_rs_log_message,                /* log_message */
    NULL,                                  /* get_request_time */
    NULL,                                  /* terminate_process */
    NULL,                                  /* php_ini_path_override */
    php_default_post_reader,               /* default_post_reader */
    php_default_treat_data,                /* treat_data */
    NULL,                                  /* executable_location */
    1,                                     /* php_ini_ignore */
    1,                                     /* php_ini_ignore_cwd */
    NULL,                                  /* get_fd */
    NULL,                                  /* force_http_10 */
    NULL,                                  /* get_target_uid */
    NULL,                                  /* get_target_gid */
    NULL,                                  /* input_filter */
    NULL,                                  /* ini_defaults */
    1,                                     /* phpinfo_as_text */
    NULL,                                  /* ini_entries */
    NULL,                                  /* additional_functions */
    NULL                                   /* input_filter_init */
};

/*
 * Subsystem Lifecycle Implementation
 */

void restphp_sapi_init(void) {
    signal(SIGPIPE, SIG_IGN);
    zend_signal_startup();
    restphp_sapi_module.php_ini_ignore = 1;
    restphp_sapi_module.php_ini_ignore_cwd = 1;
    sapi_startup(&restphp_sapi_module);
    sapi_register_post_entries(restphp_post_entries);
    php_module_startup(&restphp_sapi_module, NULL);
}

void restphp_sapi_teardown(void) {
    php_module_shutdown();
    sapi_shutdown();
}

/*
 * Request Setup & Metadata
 */

void restphp_set_request_info(
    void *server_context,
    const char *method,
    char *uri,
    char *query_string,
    const char *content_type,
    int64_t content_length,
    char *path_translated
) {
    /*
     * CRITICAL: SG(server_context) MUST be non-NULL before php_request_startup().
     * In sapi_activate(), PHP checks: `if (SG(server_context) == NULL) skip_post_reading;`
     */
    SG(server_context) = server_context;
    SG(request_info).request_method = method;
    SG(request_info).request_uri = uri;
    SG(request_info).query_string = query_string;
    SG(request_info).content_type = content_type;
    SG(request_info).content_length = content_length;
    SG(request_info).path_translated = path_translated;
}

void restphp_set_cookie_data(char *cookie_data) {
    SG(request_info).cookie_data = cookie_data;
}

void *restphp_get_server_context(void) {
    return SG(server_context);
}

/*
 * Safe Request Lifecycle (Bailout Protected)
 */

int restphp_request_startup_safe(void) {
    volatile int status = 0;
    EG(exit_status) = 0;
    SG(sapi_headers).http_response_code = 0;
    zend_first_try {
        if (php_request_startup() == FAILURE) {
            status = -1;
        }
    } zend_catch {
        status = EG(exit_status) ? EG(exit_status) : -1;
    } zend_end_try();
    return status;
}

int restphp_request_shutdown_safe(void) {
    volatile int status = 0;
    zend_first_try {
        zend_gc_collect_cycles();
        php_request_shutdown(NULL);
    } zend_catch {
        status = EG(exit_status) ? EG(exit_status) : -1;
    } zend_end_try();
    SG(sapi_headers).http_response_code = 0;
    return status;
}

/*
 * Safe Evaluation Functions
 */

int restphp_eval_string_safe(const char *code, const char *desc) {
    volatile int status = 0;
    EG(exit_status) = 0; // Prevent exit status bleed from prior requests
    zend_first_try {
        status = zend_eval_string((char *)code, NULL, (char *)desc);
        if (EG(exception)) {
            if (zend_is_unwind_exit(EG(exception))) {
                zend_clear_exception();
                status = EG(exit_status);
            } else {
                zend_exception_error(EG(exception), E_ERROR);
                status = (EG(exit_status) != 0) ? EG(exit_status) : 255;
            }
        } else if (EG(exit_status) != 0) {
            status = EG(exit_status);
        } else if (status != SUCCESS) {
            status = 255;
        }
    } zend_catch {
        status = (EG(exit_status) != 0) ? EG(exit_status) : 255;
    } zend_end_try();
    return status;
}

int restphp_execute_script_safe(const char *filepath) {
    volatile int status = 0;
    EG(exit_status) = 0; // Prevent exit status bleed from prior requests
    zend_file_handle file_handle;
    zend_stream_init_filename(&file_handle, filepath);
    zend_first_try {
        bool ok = php_execute_script(&file_handle);
        if (EG(exception)) {
            if (zend_is_unwind_exit(EG(exception))) {
                zend_clear_exception();
                status = EG(exit_status);
            } else {
                zend_exception_error(EG(exception), E_ERROR);
                status = (EG(exit_status) != 0) ? EG(exit_status) : 255;
            }
        } else if (EG(exit_status) != 0) {
            status = EG(exit_status);
        } else if (!ok) {
            status = 255;
        }
    } zend_catch {
        status = (EG(exit_status) != 0) ? EG(exit_status) : 255;
    } zend_end_try();
    zend_destroy_file_handle(&file_handle);
    return status;
}

/*
 * Helper for Rust to register $_SERVER variables
 */
void restphp_register_variable(const char *var, const char *val, size_t val_len, void *track_vars_array) {
    if (var && val && track_vars_array) {
        php_register_variable_safe(var, val, val_len, (zval *)track_vars_array);
    }
}
