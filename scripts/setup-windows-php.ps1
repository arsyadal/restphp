param(
    [ValidateSet("8.2", "8.3", "8.4")]
    [string]$PhpVersion = "8.4",
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "RestPHP\php-$PhpVersion")
)

$ErrorActionPreference = "Stop"
if (Test-Path $InstallDir) {
    throw "InstallDir already exists: $InstallDir. Choose a new directory."
}

$baseUrl = "https://windows.php.net/downloads/releases"
$releases = Invoke-RestMethod "$baseUrl/releases.json"
$compiler = if ($PhpVersion -eq "8.4") { "vs17" } else { "vs16" }
$release = $releases.$PhpVersion."nts-$compiler-x64"
if (-not $release.zip.path -or -not $release.devel_pack.path) {
    throw "No official PHP $PhpVersion NTS x64 runtime and development pack found."
}

New-Item -ItemType Directory -Path $InstallDir | Out-Null
$InstallDir = (Resolve-Path $InstallDir).Path
foreach ($name in @("zip", "devel_pack")) {
    $package = $release.$name
    $archive = Join-Path $InstallDir $package.path
    Invoke-WebRequest "$baseUrl/$($package.path)" -OutFile $archive
    if ((Get-FileHash $archive -Algorithm SHA256).Hash -ne $package.sha256) {
        throw "PHP archive checksum mismatch: $($package.path)"
    }
    $destination = Join-Path $InstallDir $name
    Expand-Archive $archive -DestinationPath $destination
    Remove-Item $archive
}

$runtime = Join-Path $InstallDir "zip"
$sdk = Get-ChildItem (Join-Path $InstallDir "devel_pack") -Directory |
    Where-Object { Test-Path (Join-Path $_.FullName "include\main\php.h") } |
    Select-Object -ExpandProperty FullName -First 1
if (-not $sdk -or -not (Test-Path "$sdk\lib\php8.lib")) {
    throw "The PHP development pack does not contain the NTS headers and php8.lib."
}

$env:PHP_DEV_DIR = $sdk
$env:PHPRC = Join-Path $runtime "php.ini-development"
$env:PATH = "$runtime;$env:PATH"
& "$runtime\php.exe" -r 'if (PHP_ZTS || PHP_INT_SIZE !== 8) { exit(1); } echo PHP_VERSION, PHP_EOL;'
if ($LASTEXITCODE -ne 0) {
    throw "PHP could not start. Install the x64 Visual C++ Redistributable for Visual Studio 2022."
}

if ($env:GITHUB_ENV) {
    "PHP_DEV_DIR=$sdk" | Out-File $env:GITHUB_ENV -Encoding utf8 -Append
    "PHPRC=$env:PHPRC" | Out-File $env:GITHUB_ENV -Encoding utf8 -Append
    "RESTPHP_PHP_RUNTIME=$runtime" | Out-File $env:GITHUB_ENV -Encoding utf8 -Append
    $runtime | Out-File $env:GITHUB_PATH -Encoding utf8 -Append
}
Write-Host "PHP runtime: $runtime"
Write-Host "PHP_DEV_DIR: $sdk"
Write-Host "This PowerShell session is ready for cargo build."
