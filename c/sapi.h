#ifndef RESTPHP_SAPI_H
#define RESTPHP_SAPI_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Subsystem Lifecycle */
void restphp_sapi_init(void);
void restphp_sapi_teardown(void);

/* Request Metadata & Lifecycle */
void restphp_set_request_info(
    void *server_context,
    const char *method,
    char *uri,
    char *query_string,
    const char *content_type,
    int64_t content_length,
    char *path_translated
);

void restphp_set_cookie_data(char *cookie_data);

int restphp_request_startup_safe(void);
int restphp_request_shutdown_safe(void);

/* Safe Script & String Evaluation with Bailout Protection */
int restphp_eval_string_safe(const char *code, const char *desc);
int restphp_execute_script_safe(const char *filepath);

/* Helper for Server Variable Registration */
void restphp_register_variable(const char *var, const char *val, size_t val_len, void *track_vars_array);

/* SAPI context accessor */
void *restphp_get_server_context(void);

#ifdef __cplusplus
}
#endif

#endif /* RESTPHP_SAPI_H */
