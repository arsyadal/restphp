#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
#include "sapi.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <stdarg.h>

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
#include <main/php_output.h>
#include <main/php_streams.h>
#include <ext/standard/php_fopen_wrappers.h>

extern int restphp_rs_worker_next(void *bridge);
extern void restphp_rs_worker_finish(void *bridge, int success);
static void *restphp_worker_bridge = NULL;
static bool restphp_worker_stopped = false;
static bool restphp_worker_in_callback = false;
static bool restphp_variable_bailout = false;
static bool restphp_request_modules_live = true;
static zval restphp_boot_env;
static zval restphp_boot_server;

static void restphp_sapi_register_variables(zval *variables);
PHP_FUNCTION(restphp_handle_request);
PHP_FUNCTION(restphp_stop_worker);

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_restphp_handle_request, 0, 1, _IS_BOOL, 0)
    ZEND_ARG_TYPE_INFO(0, callback, IS_CALLABLE, 0)
ZEND_END_ARG_INFO()

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_restphp_stop_worker, 0, 0, IS_VOID, 0)
ZEND_END_ARG_INFO()

static const zend_function_entry restphp_functions[] = {
    PHP_FE(restphp_handle_request, arginfo_restphp_handle_request)
    PHP_FE(restphp_stop_worker, arginfo_restphp_stop_worker)
    PHP_FE_END
};

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
 * PHP invokes sapi_error directly for SAPI-level failures. Leaving this callback
 * NULL makes error paths depend on internal fallbacks and has caused crashes
 * after output/header interactions. Format once and route it through Rust
 * tracing just like log_message.
 */
static void restphp_sapi_error(int type, const char *error_msg, ...) {
    char message[2048];
    va_list args;

    if (!error_msg) {
        restphp_rs_log_message("PHP SAPI error with no message", type);
        return;
    }

    va_start(args, error_msg);
    (void)vsnprintf(message, sizeof(message), error_msg, args);
    va_end(args);
    restphp_rs_log_message(message, type);
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

static void restphp_ini_defaults(HashTable *configuration_hash) {
    zval value;
    ZVAL_NEW_STR(&value, zend_string_init(ZEND_STRL("1"), 1));
    zend_hash_str_update(configuration_hash, ZEND_STRL("log_errors"), &value);
}

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
    restphp_sapi_error,                    /* sapi_error */
    NULL,                                  /* header_handler */
    restphp_sapi_send_headers,             /* send_headers */
    NULL,                                  /* send_header (intentionally NULL) */
    restphp_rs_read_post,                  /* read_post */
    restphp_sapi_read_cookies,             /* read_cookies (NEVER NULL) */
    restphp_sapi_register_variables,       /* register_server_variables */
    restphp_rs_log_message,                /* log_message */
    NULL,                                  /* get_request_time */
    NULL,                                  /* terminate_process */
    NULL,                                  /* php_ini_path_override */
    php_default_post_reader,               /* default_post_reader */
    php_default_treat_data,                /* treat_data */
    NULL,                                  /* executable_location */
    0,                                     /* php_ini_ignore */
    1,                                     /* php_ini_ignore_cwd */
    NULL,                                  /* get_fd */
    NULL,                                  /* force_http_10 */
    NULL,                                  /* get_target_uid */
    NULL,                                  /* get_target_gid */
    NULL,                                  /* input_filter */
    restphp_ini_defaults,                  /* ini_defaults */
    1,                                     /* phpinfo_as_text */
    NULL,                                  /* ini_entries */
    restphp_functions,                     /* additional_functions */
    NULL                                   /* input_filter_init */
};

/*
 * Subsystem Lifecycle Implementation
 */

int restphp_sapi_init(void) {
    volatile int status = FAILURE;
    volatile bool sapi_initialized = false;
    zend_first_try {
        signal(SIGPIPE, SIG_IGN);
        zend_signal_startup();
        restphp_sapi_module.php_ini_ignore = 0;
        restphp_sapi_module.php_ini_ignore_cwd = 1;
        sapi_startup(&restphp_sapi_module);
        sapi_initialized = true;
        sapi_register_post_entries(restphp_post_entries);
        status = php_module_startup(&restphp_sapi_module, NULL);
    } zend_catch {
        status = FAILURE;
    } zend_end_try();
    if (status != SUCCESS && sapi_initialized) {
        zend_first_try {
            sapi_shutdown();
        } zend_catch {
            restphp_rs_log_message("PHP SAPI cleanup after startup failure bailed out", 3);
        } zend_end_try();
    }
    return status;
}

void restphp_sapi_teardown(void) {
    zend_first_try {
        php_module_shutdown();
    } zend_catch {
        restphp_rs_log_message("PHP module shutdown bailed out", 3);
    } zend_end_try();
    zend_first_try {
        sapi_shutdown();
    } zend_catch {
        restphp_rs_log_message("PHP SAPI shutdown bailed out", 3);
    } zend_end_try();
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

void restphp_clear_request_info(void) {
    /*
     * All request_info string pointers and server_context point to Rust stack
     * storage. Clear every field only after request shutdown (or a failed
     * startup) so a later request cannot dereference stale CStrings/context.
     */
    SG(server_context) = NULL;
    memset(&SG(request_info), 0, sizeof(SG(request_info)));
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
        /* PHP catches shutdown-handler bailouts internally; retain their exit status. */
        status = EG(exit_status);
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
    volatile bool initialized = false;
    zend_first_try {
        zend_stream_init_filename(&file_handle, filepath);
        initialized = true;
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
    if (initialized) {
        zend_first_try {
            zend_destroy_file_handle(&file_handle);
        } zend_catch {
            status = 255;
        } zend_end_try();
    }
    return status;
}

/*
 * Helper for Rust to register $_SERVER variables
 */
void restphp_register_variable(const char *var, const char *val, size_t val_len, void *track_vars_array) {
    /* Catch here, not above the calling Rust frame. Re-raise after it returns. */
    if (restphp_variable_bailout) {
        return;
    }
    zend_try {
        php_register_variable_safe(var, val, val_len, (zval *)track_vars_array);
    } zend_catch {
        restphp_variable_bailout = true;
    } zend_end_try();
}

static void restphp_sapi_register_variables(zval *variables) {
    restphp_variable_bailout = false;
    restphp_rs_register_server_variables(variables);
    if (restphp_variable_bailout) {
        zend_bailout();
    }
}

void restphp_set_worker_bridge(void *bridge) {
    restphp_worker_bridge = bridge;
    restphp_worker_stopped = false;
    restphp_worker_in_callback = false;
    restphp_request_modules_live = true;
    /* Snapshot storage belongs to the one outer Zend request's allocator. */
    ZVAL_UNDEF(&restphp_boot_env);
    ZVAL_UNDEF(&restphp_boot_server);
}


/* php://input wrappers borrow the request-body stream without owning it. Do
 * not invalidate a stream retained by userland; release ordinary bodies now. */
static void restphp_release_body(php_stream *body) {
    typedef struct {
        php_stream *body;
        zend_off_t position;
    } input_stream_state;
    zend_resource *resource, *body_resource = NULL;
    ZEND_HASH_FOREACH_PTR(&EG(regular_list), resource) {
        if (resource->type != php_file_le_stream()) {
            continue;
        }
        php_stream *stream = resource->ptr;
        if (stream == body) {
            body_resource = resource;
        } else if (stream->wrapper == &php_stream_php_wrapper
                && strcmp(stream->ops->label, "Input") == 0
                && ((input_stream_state *)stream->abstract)->body == body) {
            return;
        }
    } ZEND_HASH_FOREACH_END();
    if (body_resource) {
        zend_list_delete(body_resource);
    }
}

static void restphp_request_modules(bool startup) {
    if (startup == restphp_request_modules_live) {
        return;
    }
    const char *names[] = {"session", "filter"};
    for (size_t i = 0; i < sizeof(names) / sizeof(names[0]); i++) {
        zend_module_entry *module = zend_hash_str_find_ptr(&module_registry, names[i], strlen(names[i]));
        if (!module) {
            continue;
        }
        if (startup && module->request_startup_func) {
            if (module->request_startup_func(module->type, module->module_number) != SUCCESS) {
                zend_bailout();
            }
        } else if (!startup && module->request_shutdown_func) {
            module->request_shutdown_func(module->type, module->module_number);
        }
    }
    restphp_request_modules_live = startup;
}
/* Release only HTTP state. The Zend request and booted application stay alive. */
static void restphp_reset_http_state(void) {
    restphp_request_modules(false);
    php_output_discard_all();
    php_output_deactivate();
    if (SG(request_info).request_body) {
        restphp_release_body(SG(request_info).request_body);
        SG(request_info).request_body = NULL;
    }
    sapi_deactivate();
    if (!Z_ISUNDEF(SG(callback_func))) {
        zval_ptr_dtor(&SG(callback_func));
        ZVAL_UNDEF(&SG(callback_func));
    }
    restphp_clear_request_info();
    sapi_activate();
    SG(sapi_headers).http_response_code = 200;
    php_output_activate();
}


static void restphp_reset_superglobals(void) {
    static const char *names[] = {
        "_POST", "_GET", "_COOKIE", "_SERVER", "_ENV", "_FILES", "_REQUEST", "_SESSION"
    };
    for (size_t i = 0; i < sizeof(names) / sizeof(names[0]); i++) {
        zend_hash_str_del(&EG(symbol_table), names[i], strlen(names[i]));
    }
    for (int i = 0; i < 6; i++) {
        zval_ptr_dtor(&PG(http_globals)[i]);
        ZVAL_UNDEF(&PG(http_globals)[i]);
    }
    php_hash_environment();
    zend_is_auto_global_str(ZEND_STRL("_SERVER"));
    zend_is_auto_global_str(ZEND_STRL("_ENV"));
    zend_is_auto_global_str(ZEND_STRL("_REQUEST"));
    /* Dotenv writes superglobals rather than process getenv(). Preserve the
     * boot environment, but let every current CGI value override its baseline. */
    zval_ptr_dtor(&PG(http_globals)[TRACK_VARS_ENV]);
    ZVAL_COPY(&PG(http_globals)[TRACK_VARS_ENV], &restphp_boot_env);
    Z_ADDREF(PG(http_globals)[TRACK_VARS_ENV]);
    zend_hash_str_update(&EG(symbol_table), ZEND_STRL("_ENV"), &PG(http_globals)[TRACK_VARS_ENV]);
    SEPARATE_ARRAY(&PG(http_globals)[TRACK_VARS_SERVER]);
    zend_string *key;
    zval *value;
    ZEND_HASH_FOREACH_STR_KEY_VAL(Z_ARRVAL(restphp_boot_server), key, value) {
        if (key) {
            zval *added = zend_hash_add(Z_ARRVAL(PG(http_globals)[TRACK_VARS_SERVER]), key, value);
            if (added) {
                Z_TRY_ADDREF_P(added);
            }
        }
    } ZEND_HASH_FOREACH_END();
    Z_ADDREF(PG(http_globals)[TRACK_VARS_SERVER]);
    zend_hash_str_update(&EG(symbol_table), ZEND_STRL("_SERVER"), &PG(http_globals)[TRACK_VARS_SERVER]);
}

PHP_FUNCTION(restphp_handle_request) {
    zend_fcall_info fci;
    zend_fcall_info_cache fcc;
    ZEND_PARSE_PARAMETERS_START(1, 1)
        Z_PARAM_FUNC(fci, fcc)
    ZEND_PARSE_PARAMETERS_END();

    if (!restphp_worker_bridge || restphp_worker_in_callback) {
        zend_throw_error(NULL, "restphp_handle_request requires persistent worker mode and cannot be nested");
        RETURN_THROWS();
    }
    if (restphp_worker_stopped) {
        RETURN_FALSE;
    }

    volatile bool success = true;
    zval retval;
    ZVAL_UNDEF(&retval);
    /* Every Zend operation, including setup and destructors, stays inside C. */
    zend_try {
        if (Z_ISUNDEF(restphp_boot_env)) {
            zend_is_auto_global_str(ZEND_STRL("_ENV"));
            zend_is_auto_global_str(ZEND_STRL("_SERVER"));
            zval *environment = zend_hash_str_find_ind(&EG(symbol_table), ZEND_STRL("_ENV"));
            zval *server = zend_hash_str_find_ind(&EG(symbol_table), ZEND_STRL("_SERVER"));
            if (!environment || !server) {
                zend_error(E_ERROR, "Persistent bootstrap must keep environment superglobals");
            }
            ZVAL_DEREF(environment);
            ZVAL_DEREF(server);
            if (Z_TYPE_P(environment) != IS_ARRAY || Z_TYPE_P(server) != IS_ARRAY) {
                zend_error(E_ERROR, "Persistent bootstrap environment superglobals must be arrays");
            }
            ZVAL_COPY(&restphp_boot_env, environment);
            ZVAL_COPY(&restphp_boot_server, server);
        }
        restphp_reset_http_state();
    } zend_catch {
        success = false;
    } zend_end_try();
    if (!success) {
        restphp_worker_stopped = true;
        zend_bailout();
    }

    /* This callback only waits and installs non-owning pointers; no Zend calls. */
    if (!restphp_rs_worker_next(restphp_worker_bridge)) {
        restphp_worker_stopped = true;
        RETURN_FALSE;
    }

    restphp_worker_in_callback = true;
    zend_try {
        /* next installed new request_info after the empty SAPI activation. */
        zend_llist_destroy(&SG(sapi_headers).headers);
        sapi_activate();
        SG(sapi_headers).http_response_code = 200;
        EG(exit_status) = 0;
        restphp_reset_superglobals();
        restphp_request_modules(true);
        fci.retval = &retval;
        fci.param_count = 0;
        fci.params = NULL;
        if (zend_call_function(&fci, &fcc) != SUCCESS || EG(exception)) {
            success = false;
        }
        if (success) {
            restphp_request_modules(false);
            php_output_end_all();
            sapi_send_headers();
            zend_gc_collect_cycles();
        }
        if (!Z_ISUNDEF(retval)) {
            zval_ptr_dtor(&retval);
        }
    } zend_catch {
        success = false;
    } zend_end_try();
    if (EG(exception)) {
        success = false;
        zend_try {
            zend_exception_error(EG(exception), E_WARNING);
            zend_clear_exception();
        } zend_catch {
            success = false;
        } zend_end_try();
    }
    if (EG(exit_status)) {
        success = false;
    }
    if (!success) {
        /* A bailed-out VM cannot resume the PHP while loop. The outer C script
         * shield catches this; Rust retains the active job through RSHUTDOWN. */
        restphp_worker_in_callback = false;
        restphp_worker_stopped = true;
        zend_bailout();
    }
    zend_try {
        restphp_reset_http_state();
    } zend_catch {
        success = false;
        /* Never leave Rust-owned job pointers attached after completion. */
        restphp_clear_request_info();
    } zend_end_try();
    restphp_worker_in_callback = false;
    if (!success) {
        restphp_worker_stopped = true;
        zend_bailout();
    }
    restphp_rs_worker_finish(restphp_worker_bridge, 1);
    RETURN_TRUE;
}

PHP_FUNCTION(restphp_stop_worker) {
    ZEND_PARSE_PARAMETERS_NONE();
    (void)return_value;
    restphp_worker_stopped = true;
}
