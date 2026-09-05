# BRIEFING — 2026-09-05T05:35:50Z

## Mission
Investigate Zend Engine C FFI embedding, sapi_module_struct callbacks, persistent worker lifecycle, superglobals injection, and integration with Tokio/Hyper async runtime.

## 🔒 My Identity
- Archetype: explorer
- Roles: investigation, synthesis
- Working directory: /home/cads/restphp/.agents/teamwork_preview_explorer_survey_3
- Original parent: 68c0faad-eea6-4f55-90d3-5c0618ffa842
- Milestone: survey

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Produce structured report at /home/cads/restphp/.agents/teamwork_preview_explorer_survey_3/handoff.md
- Report back via send_message to parent (68c0faad-eea6-4f55-90d3-5c0618ffa842)

## Current Parent
- Conversation ID: 68c0faad-eea6-4f55-90d3-5c0618ffa842
- Updated: 2026-09-05T05:35:50Z

## Investigation State
- **Explored paths**: DISPATCH.md, ORIGINAL_REQUEST.md, Cargo.toml, SPEC.md, PRD.md, README.md, /usr/include/php/20240924 (SAPI.h, zend.h, php_main.h, php_variables.h, zend_stream.h), /usr/lib/libphp.so, disassembly of SAPI callbacks.
- **Key findings**:
  1. System PHP is 8.4.24 NTS (Thread Safety disabled) with Zend Engine 4.4.24.
  2. Zend VM is single-threaded in NTS mode; multi-threaded access causes fatal race conditions.
  3. Tokio async HTTP engine must be decoupled from Zend VM via a dedicated worker thread using MPSC/Oneshot channels.
  4. SAPI pitfalls: `read_cookies` is called unconditionally by `sapi_activate` without a NULL check (causes crash if NULL); `send_headers` must return `SAPI_HEADER_SENT_SUCCESSFULLY` (1) to prevent crashing on NULL `send_header`.
  5. Bailout trap: `exit()`/`die()` invokes `longjmp`, which will corrupt Rust stack unwinding unless wrapped with `zend_first_try` / `zend_catch` in a C shim (`c/sapi.c`).
  6. Superglobals: automatic population of `$_GET`, `$_POST`, `$_COOKIE`, and streaming `php://input` verified with zero memory leaks across consecutive requests.
- **Unexplored areas**: None for survey scope.

## Key Decisions Made
- Recommend C shim bridge (`c/sapi.c`) compiled via `cc` in `build.rs` to safely handle macros (`STANDARD_SAPI_MODULE_PROPERTIES`) and `zend_first_try` bailouts.
- Recommend Tokio Actor model: Tokio handles network I/O, dispatches tasks to dedicated Zend worker thread via Crossbeam channel.

## Artifact Index
- /home/cads/restphp/.agents/teamwork_preview_explorer_survey_3/BRIEFING.md — Persistent working memory
- /home/cads/restphp/.agents/teamwork_preview_explorer_survey_3/progress.md — Liveness heartbeat
- /home/cads/restphp/.agents/teamwork_preview_explorer_survey_3/handoff.md — Comprehensive technical report
