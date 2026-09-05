# Task Assignment: Milestone 1 - C SAPI Shim & Build Script Design

You are `teamwork_preview_explorer_m1_1`.
Working Directory: `/home/cads/restphp/.agents/teamwork_preview_explorer_m1_1`
Project Root: `/home/cads/restphp`
Original Request: `/home/cads/restphp/.agents/ORIGINAL_REQUEST.md`
Project Plan: `/home/cads/restphp/PROJECT.md`

## Milestone 1 Scope
Features 1-18: Core C-FFI & Custom SAPI Subsystem:
- `c/sapi.c`: Implement C SAPI shim with `restphp_sapi_module` (`sapi_module_struct`), `zend_first_try` / `zend_catch` bailout protection, `restphp_sapi_init`, `restphp_sapi_teardown`, `restphp_eval_string_safe`, `restphp_execute_script_safe`, `restphp_set_request_info`.
- `build.rs`: Use `cc` crate to compile `c/sapi.c` with PHP include paths from `php-config --includes`, link against `libphp.so` via `cargo:rustc-link-lib=php` and `cargo:rustc-link-search`.
- `Cargo.toml`: Add `cc = "1.0"` under `[build-dependencies]`.

## Objectives
1. Read `/home/cads/restphp/.agents/ORIGINAL_REQUEST.md` and `/home/cads/restphp/PROJECT.md`.
2. Analyze the survey reports at `/home/cads/restphp/.agents/teamwork_preview_explorer_survey_3/handoff.md` and `/home/cads/restphp/.agents/teamwork_preview_spec_miner_survey_1/handoff.md`.
3. Provide the complete, exact, compile-ready code and architecture for `c/sapi.c`, `build.rs`, and `Cargo.toml`.
4. Ensure critical pitfalls are addressed: `read_cookies` must never be NULL, `send_headers` must return `1`, and `zend_first_try` must prevent `longjmp` from escaping into Rust.
5. Write your recommendations and implementation blueprint to `/home/cads/restphp/.agents/teamwork_preview_explorer_m1_1/handoff.md`.

## 2026-09-05T05:36:53Z
You are teamwork_preview_explorer_m1_1.
Your working directory is /home/cads/restphp/.agents/teamwork_preview_explorer_m1_1.
Your task assignment is in /home/cads/restphp/.agents/teamwork_preview_explorer_m1_1/DISPATCH.md.
Also read /home/cads/restphp/.agents/ORIGINAL_REQUEST.md and /home/cads/restphp/PROJECT.md.
Analyze c/sapi.c shim design, build.rs, and Cargo.toml.
Write your recommendations to /home/cads/restphp/.agents/teamwork_preview_explorer_m1_1/handoff.md and report back via send_message.

