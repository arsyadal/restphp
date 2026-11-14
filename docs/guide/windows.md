# Windows native

RestPHP's Windows build targets x64 MSVC and PHP 8.2, 8.3, or 8.4 Non Thread Safe (NTS). It embeds `php8.dll` directly; WSL and Docker are not required. Windows ARM64 and 32-bit PHP builds are not supported.

## Download a prebuilt executable

Download `restphp-windows-x86_64.zip` from [RestPHP v0.3.0](https://github.com/arsyadal/restphp/releases/tag/v0.3.0). Install the [x64 Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist) if it is not already installed, then extract the whole ZIP into a directory you control.

Open PowerShell in that directory:

```powershell
.\restphp.exe --version
.\restphp.exe -e "echo PHP_OS_FAMILY, ':', PHP_VERSION;"
.\restphp.exe serve --entrypoint C:\apps\my-project\public\index.php --host 127.0.0.1 --port 8080
```

Replace the entrypoint with your application's PHP file. Keep the bundled DLLs beside `restphp.exe`. This route needs neither Rust nor the PHP development pack.

## Build from source

Install these prerequisites:

- [Rust](https://rustup.rs/) with the `x86_64-pc-windows-msvc` toolchain.
- [Visual Studio 2022 Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with **Desktop development with C++** and the Windows SDK.
- The [x64 Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist).
- Git and PowerShell.

Open PowerShell in the repository:

```powershell
git clone https://github.com/arsyadal/restphp.git
cd restphp
.\scripts\setup-windows-php.ps1 -PhpVersion 8.4
cargo build --release
if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
.\target\release\restphp.exe --version
.\target\release\restphp.exe -e "echo PHP_OS_FAMILY, ':', PHP_VERSION;"
.\target\release\restphp.exe serve --entrypoint public/index.php --host 127.0.0.1 --port 8080
```

In a second terminal, run `curl.exe http://127.0.0.1:8080/`. Stop the server with Ctrl-C.

The setup script downloads the official matching runtime and development pack and checks their SHA-256 values against the PHP release manifest. It installs under `%LOCALAPPDATA%\RestPHP\php-8.4` by default and refuses to overwrite an existing directory. Use `-InstallDir` to choose another empty location. It changes environment variables only for the current PowerShell session, not your machine's permanent configuration.

### Existing PHP installation

Use matching **NTS x64** runtime and development archives from [PHP for Windows](https://windows.php.net/download/). Do not mix patch versions, compiler builds, architectures, or TS/NTS packages.

```powershell
$env:PHP_DEV_DIR = 'C:\php-devel\php-8.4-devel'
# PHP_DEV_DIR must contain include\main\php.h and lib\php8.lib.
$env:PATH = "C:\php;$env:PATH"
$env:PHPRC = 'C:\php\php.ini'
cargo build --release
```

`PHP_DEV_DIR` is a build dependency. At runtime, keep the matching PHP runtime directory on `PATH`, or put `restphp.exe` beside `php8.dll` and the other runtime DLLs. An `.exe` alone is not a standalone PHP distribution.

To reuse the setup-script installation in a new terminal:

```powershell
$root = Join-Path $env:LOCALAPPDATA 'RestPHP\php-8.4'
$env:PHP_DEV_DIR = Get-ChildItem "$root\devel_pack" -Directory | Select-Object -ExpandProperty FullName -First 1
$env:PATH = "$root\zip;$env:PATH"
$env:PHPRC = "$root\zip\php.ini-development"
```

## Configuration and Laravel

RestPHP loads `php.ini` and `PHP_INI_SCAN_DIR`. Enable the extensions your application needs in the matching Windows PHP installation; for example, set `extension_dir` to its `ext` directory before enabling shared extensions. The downloaded development INI is a starting point, not a production configuration.

A conventional Laravel application can run with:

```powershell
.\target\release\restphp.exe serve --entrypoint C:\apps\my-laravel\public\index.php
```

Native persistent callback workers also use `--persistent-worker`. The `php artisan octane:restphp` **supervisor** still requires Unix `pcntl` and `posix` extensions; use WSL2/Linux for those Artisan start/status/reload/stop commands. This limitation does not apply to the native RestPHP CLI or its callback worker.

## Shutdown and reload

Ctrl-C and Ctrl-Break initiate graceful draining. `--watch` starts its child in a separate Windows process group and sends Ctrl-Break before replacing it. `--shutdown-timeout-secs` bounds teardown. Keep the watcher attached to a console; forcibly ending a process with Task Manager or `Stop-Process` cannot drain active requests.

## Windows verification and binaries

`.github/workflows/windows.yml` builds and tests the native executable on Windows Server 2022 with PHP 8.2, 8.3, and 8.4 NTS. The tests cover CLI evaluation, HTTP requests, headers, bodies, state reset, persistent callbacks, recycling, Unicode paths, Ctrl-Break shutdown, and watch reload.

Successful Windows workflow runs upload a `restphp-windows-x86_64` artifact containing a ZIP with the release executable and its matching PHP runtime. Tag-triggered runs also attach that ZIP to the GitHub release. Older releases made before this workflow do not include it. Extract the whole ZIP, not just `restphp.exe`.
