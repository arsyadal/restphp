# Task Assignment: Milestone 1 - Rust FFI & SAPI Bindings Architecture

You are `teamwork_preview_explorer_m1_2`.
Working Directory: `/home/cads/restphp/.agents/teamwork_preview_explorer_m1_2`
Project Root: `/home/cads/restphp`
Original Request: `/home/cads/restphp/.agents/ORIGINAL_REQUEST.md`
Project Plan: `/home/cads/restphp/PROJECT.md`

## Milestone 1 Scope
Features 1-18: Core C-FFI & Custom SAPI Subsystem:
- `src/ffi/mod.rs` & `src/ffi/types.rs`: Declare `extern "C"` functions from `c/sapi.c` and Zend Engine (e.g. `php_request_startup`, `php_request_shutdown`, `zend_gc_collect_cycles`, `restphp_sapi_init`, `restphp_sapi_teardown`, `restphp_set_request_info`, `restphp_eval_string_safe`, `restphp_execute_script_safe`, `php_register_variable_safe`).
- `src/sapi/context.rs`: Define `WorkerRequestContext` holding request metadata, post body buffer, post offset, status code, response headers, output buffer, server variables.
- `src/sapi/callbacks.rs`: Implement Rust callback functions (`restphp_rs_ub_write`, `restphp_rs_flush`, `restphp_rs_send_headers`, `restphp_rs_read_post`, `restphp_rs_read_cookies`, `restphp_rs_register_server_variables`, `restphp_rs_log_message`).
- Ensure memory safety across FFI: thread boundaries, pointer lifetimes, CString null-termination.

## Objectives
1. Read `/home/cads/restphp/.agents/ORIGINAL_REQUEST.md` and `/home/cads/restphp/PROJECT.md`.
2. Analyze survey reports at `/home/cads/restphp/.agents/teamwork_preview_explorer_survey_3/handoff.md` and `/home/cads/restphp/.agents/teamwork_preview_spec_miner_survey_1/handoff.md`.
3. Provide the complete code and structure for `src/ffi/mod.rs`, `src/ffi/types.rs`, `src/sapi/mod.rs`, `src/sapi/context.rs`, and `src/sapi/callbacks.rs`.
4. Detail how `WorkerRequestContext` is safely associated with `sapi_globals.server_context` during request startup and teardown.
5. Write your recommendations and implementation blueprint to `/home/cads/restphp/.agents/teamwork_preview_explorer_m1_2/handoff.md`.
