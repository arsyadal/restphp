# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Multi-agent autonomous teamwork prompt draft and execution specification.
- `PRD.md` defining core mission, problem statement, and NFR targets.
- `SPEC.md` covering zero-cost C-FFI, custom SAPI callbacks, and concurrency model.
- `ROADMAP.md` task backlog across 5 execution milestones.
- Cargo project configuration with Tokio, Axum, Hyper, Crossbeam, and release profiles.

## [0.1.0-alpha.1] - 2026-09-05
### Added
- Initial project scaffolding and architecture design.
- Direct Zend Engine C-FFI embedding proof-of-concept setup.
- Custom SAPI hook declarations (`ub_write`, `read_post`, `send_headers`).
