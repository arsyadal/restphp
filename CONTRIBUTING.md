# Contributing to RestPHP

Thank you for your interest in contributing to RestPHP! Whether you are reporting a bug, improving documentation, or submitting a pull request, your help is welcome.

## Ways to Contribute

- **Report bugs**: Open an issue using the [Bug Report](https://github.com/arsyadal/restphp/issues/new?template=bug_report.yml) template with your OS, PHP version, RestPHP version, and minimal reproduction steps.
- **Request features**: Open an issue using the [Feature Request](https://github.com/arsyadal/restphp/issues/new?template=feature_request.yml) template describing the use case and expected behavior.
- **Submit pull requests**: Fix bugs, add tests, improve benchmarks, or refine documentation.

For significant architectural changes (such as SAPI lifecycle, worker pool behavior, or CLI flags), please open an issue first to discuss the design before writing large patches.

---

## Development Setup

### Prerequisites

- **Rust**: Stable toolchain (`rustfmt` and `clippy` components installed)
- **PHP (NTS)**: PHP `8.2`, `8.3`, or `8.4` with embed library and development headers (`libphp-embed`, `php-dev`)
- **Build tools**: C compiler (`clang` or `gcc`), `pkg-config`
- **Python 3**: Required for running Python regression and integration test suites

#### Ubuntu / Debian

```bash
sudo apt-get update
sudo apt-get install -y build-essential libphp-embed php-dev pkg-config clang
```

#### macOS (Homebrew)

Install PHP with embed support or use a local NTS PHP build, ensuring `php-config` is on your `PATH`.

#### Windows x64

Use Rust's MSVC toolchain and Visual Studio 2022 C++ Build Tools. Run `.\scripts\setup-windows-php.ps1` from PowerShell to install matching official NTS PHP runtime and development archives, then build in that same terminal. See [Windows native](docs/guide/windows.md) for DLL requirements and the Unix-only Artisan supervisor limitation.

### Build

```bash
git clone https://github.com/arsyadal/restphp.git
cd restphp
cargo build
```

---

## Running Checks & Tests

All pull requests are verified by CI (`.github/workflows/ci.yml`). Please run these checks locally before submitting:

```bash
# 1. Format code
cargo fmt --all

# 2. Run Clippy (warnings are treated as errors)
cargo clippy -- -D warnings

# 3. Run Rust tests (must be serialized because the NTS Zend VM is process-global)
cargo test -- --test-threads=1

# 4. Run Python regression tests
python3 -m unittest discover -s tests -p 'test_*.py'
```

---

## Pull Request Guidelines

1. **Fork and branch**: Create a feature or fix branch from `main`.
2. **Keep changes focused**: One logical change per PR. Avoid unrelated formatting or refactoring.
3. **Add tests**: Include regression tests for bug fixes or new behavior where applicable.
4. **Update docs**: If you change CLI flags, behavior, or architecture, update `README.md` or `docs/` accordingly.
5. **Pass CI checks**: Ensure `cargo fmt --all -- --check`, `cargo clippy -- -D warnings`, and `cargo test -- --test-threads=1` pass.

---

## License

By contributing to RestPHP, you agree that your contributions will be dual-licensed under the [MIT License](LICENSE-MIT) or [Apache License, Version 2.0](LICENSE-APACHE), at your option.
