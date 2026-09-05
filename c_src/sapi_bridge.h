#ifndef RESTPHP_SAPI_BRIDGE_H
#define RESTPHP_SAPI_BRIDGE_H

#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    char *output_bytes;
    size_t output_len;
    int status_code;
    char *content_type;
    bool success;
    char *error_message;
} RestPhpResponse;

int restphp_engine_init(void);
void restphp_engine_shutdown(void);

RestPhpResponse restphp_execute_string(const char *php_code, const char *request_method, const char *request_uri, const char *query_string, const char *request_body, size_t body_len);
RestPhpResponse restphp_execute_file(const char *filepath, const char *request_method, const char *request_uri, const char *query_string, const char *request_body, size_t body_len);

void restphp_free_response(RestPhpResponse *resp);

#ifdef __cplusplus
}
#endif

#endif // RESTPHP_SAPI_BRIDGE_H
