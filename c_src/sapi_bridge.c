#include "sapi_bridge.h"
#include <sapi/embed/php_embed.h>
#include <Zend/zend_execute.h>
#include <main/php_main.h>
#include <main/SAPI.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Dynamic response buffer for the current execution
typedef struct {
    char *data;
    size_t len;
    size_t cap;
    int status_code;
    char *content_type;
} RequestBuffer;

static RequestBuffer g_current_buf = {0};

static size_t restphp_ub_write(const char *str, size_t str_length) {
    if (!str || str_length == 0) return 0;

    if (g_current_buf.len + str_length + 1 > g_current_buf.cap) {
        size_t new_cap = (g_current_buf.cap == 0) ? 4096 : g_current_buf.cap * 2;
        while (new_cap < g_current_buf.len + str_length + 1) {
            new_cap *= 2;
        }
        char *new_data = (char *)realloc(g_current_buf.data, new_cap);
        if (!new_data) return 0;
        g_current_buf.data = new_data;
        g_current_buf.cap = new_cap;
    }

    memcpy(g_current_buf.data + g_current_buf.len, str, str_length);
    g_current_buf.len += str_length;
    g_current_buf.data[g_current_buf.len] = '\0';

    return str_length;
}

static int restphp_send_headers(sapi_headers_struct *sapi_headers) {
    if (sapi_headers) {
        if (sapi_headers->http_response_code > 0) {
            g_current_buf.status_code = sapi_headers->http_response_code;
        }
        if (sapi_headers->mimetype) {
            free(g_current_buf.content_type);
            g_current_buf.content_type = strdup(sapi_headers->mimetype);
        }
    }
    return SAPI_HEADER_SENT_SUCCESSFULLY;
}

int restphp_engine_init(void) {
    php_embed_module.ub_write = restphp_ub_write;
    php_embed_module.send_headers = restphp_send_headers;
    php_embed_module.phpinfo_as_text = 1;

    char *argv[] = {"restphp", NULL};
    if (php_embed_init(1, argv) != 0) {
        return -1;
    }
    return 0;
}

void restphp_engine_shutdown(void) {
    php_embed_shutdown();
    if (g_current_buf.data) {
        free(g_current_buf.data);
        g_current_buf.data = NULL;
    }
    if (g_current_buf.content_type) {
        free(g_current_buf.content_type);
        g_current_buf.content_type = NULL;
    }
}

static void reset_request_buffer(void) {
    if (g_current_buf.data) {
        g_current_buf.data[0] = '\0';
    }
    g_current_buf.len = 0;
    g_current_buf.status_code = 200;
    if (g_current_buf.content_type) {
        free(g_current_buf.content_type);
        g_current_buf.content_type = NULL;
    }
}

static void setup_php_superglobals(const char *method, const char *uri, const char *query, const char *body, size_t body_len) {
    char init_script[2048];
    const char *safe_method = method ? method : "GET";
    const char *safe_uri = uri ? uri : "/";
    const char *safe_query = query ? query : "";

    snprintf(init_script, sizeof(init_script),
        "$_SERVER['REQUEST_METHOD'] = '%s';\n"
        "$_SERVER['REQUEST_URI'] = '%s';\n"
        "$_SERVER['QUERY_STRING'] = '%s';\n"
        "$_SERVER['SERVER_SOFTWARE'] = 'RestPHP/0.1.0';\n"
        "$_SERVER['GATEWAY_INTERFACE'] = 'CGI/1.1';\n"
        "parse_str($_SERVER['QUERY_STRING'], $_GET);\n",
        safe_method, safe_uri, safe_query
    );

    zend_eval_string(init_script, NULL, "restphp_superglobals");

    if (body && body_len > 0) {
        // If body exists and method is POST, parse it
        if (strcmp(safe_method, "POST") == 0) {
            // Check if JSON
            if (body[0] == '{' || body[0] == '[') {
                // Keep for raw stream / custom handler
            } else {
                // Form URL-encoded
                char parse_post_script[1024];
                snprintf(parse_post_script, sizeof(parse_post_script),
                    "parse_str(file_get_contents('php://input'), $_POST);\n"
                );
                zend_eval_string(parse_post_script, NULL, "restphp_post_parser");
            }
        }
    }
}

RestPhpResponse restphp_execute_string(const char *php_code, const char *method, const char *uri, const char *query, const char *body, size_t body_len) {
    RestPhpResponse resp = {0};
    reset_request_buffer();
    setup_php_superglobals(method, uri, query, body, body_len);

    zend_result result = zend_eval_string(php_code, NULL, "restphp_eval");

    resp.success = (result == SUCCESS);
    resp.status_code = g_current_buf.status_code;
    resp.output_len = g_current_buf.len;

    if (g_current_buf.len > 0) {
        resp.output_bytes = (char *)malloc(g_current_buf.len + 1);
        memcpy(resp.output_bytes, g_current_buf.data, g_current_buf.len);
        resp.output_bytes[g_current_buf.len] = '\0';
    } else {
        resp.output_bytes = strdup("");
    }

    if (g_current_buf.content_type) {
        resp.content_type = strdup(g_current_buf.content_type);
    } else {
        resp.content_type = strdup("text/html; charset=UTF-8");
    }

    return resp;
}

RestPhpResponse restphp_execute_file(const char *filepath, const char *method, const char *uri, const char *query, const char *body, size_t body_len) {
    RestPhpResponse resp = {0};
    reset_request_buffer();
    setup_php_superglobals(method, uri, query, body, body_len);

    char eval_include[1024];
    snprintf(eval_include, sizeof(eval_include), "require '%s';", filepath);

    zend_result result = zend_eval_string(eval_include, NULL, filepath);

    resp.success = (result == SUCCESS);
    resp.status_code = g_current_buf.status_code;
    resp.output_len = g_current_buf.len;

    if (g_current_buf.len > 0) {
        resp.output_bytes = (char *)malloc(g_current_buf.len + 1);
        memcpy(resp.output_bytes, g_current_buf.data, g_current_buf.len);
        resp.output_bytes[g_current_buf.len] = '\0';
    } else {
        resp.output_bytes = strdup("");
    }

    if (g_current_buf.content_type) {
        resp.content_type = strdup(g_current_buf.content_type);
    } else {
        resp.content_type = strdup("text/html; charset=UTF-8");
    }

    return resp;
}

void restphp_free_response(RestPhpResponse *resp) {
    if (resp) {
        if (resp->output_bytes) free(resp->output_bytes);
        if (resp->content_type) free(resp->content_type);
        if (resp->error_message) free(resp->error_message);
        resp->output_bytes = NULL;
        resp->content_type = NULL;
        resp->error_message = NULL;
    }
}
