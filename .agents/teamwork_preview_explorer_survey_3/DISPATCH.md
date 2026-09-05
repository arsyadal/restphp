# Task Assignment: Zend Engine C FFI, Custom SAPI & Concurrency Model Survey

You are `teamwork_preview_explorer_survey_3`.
Working Directory: `/home/cads/restphp/.agents/teamwork_preview_explorer_survey_3`
Project Root: `/home/cads/restphp`
Original Request: `/home/cads/restphp/.agents/ORIGINAL_REQUEST.md`

## Mission
Investigate the technical details of Zend Engine C FFI embedding, `sapi_module_struct` implementation in Rust, persistent worker lifecycle, and integration with Tokio/Hyper async runtime.

## Scope & Sources
Read and investigate:
- `/home/cads/restphp/.agents/ORIGINAL_REQUEST.md`
- PHP headers on system (`php-config --includes`), specifically `main/SAPI.h`, `main/php.h`, `Zend/zend.h`, `Zend/zend_API.h`, `Zend/zend_execute.h`.
- Review existing FFI bindings in `src/ffi/` or wherever Zend FFI is defined in the repo.
- Examine how SAPI callbacks (`ub_write`, `sapi_header_op`, `read_post`, etc.) can stream data to Rust channels/buffers without thread safety issues.
- Analyze how Zend Engine interacts with threads: Zend VM is not thread-safe by default (requires TSRM / ZTS if multi-threaded, or single-threaded worker process / dedicated OS thread with an MPSC channel from Tokio).
- Determine the exact FFI lifecycle sequence: `sapi_startup`, `php_module_startup`, `php_request_startup`, script execution, `php_request_shutdown`, `php_module_shutdown`, `sapi_shutdown`.
- Detail the superglobals injection mechanism (`php_register_variable`, `track_vars`, `SG(request_info)`).

## Objectives
1. Document the exact FFI types, structs, and function signatures required.
2. Outline the safest and fastest architecture connecting Tokio async requests to the synchronous Zend worker.
3. Detail the lifecycle and state reset mechanism (avoiding memory leaks or state bleeding across requests).
4. Write your detailed technical recommendations to `/home/cads/restphp/.agents/teamwork_preview_explorer_survey_3/handoff.md` and send a message back with your summary.
