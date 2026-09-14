# Launches the locally built (custom) wezterm from this checkout without touching
# the wezterm that is installed under C:\Program Files\WezTerm. Both read the same
# config file, so the only difference is the binary.
#
# Usage examples:
#
# run the custom build (uses the same ~/.wezterm.lua as the installed wezterm):
# .\run-custom-wezterm.ps1
#
# build release first, then run:
# .\run-custom-wezterm.ps1 -Build
#
# run a checkout in another directory (positional or named):
# .\run-custom-wezterm.ps1 C:\src\wezterm
# .\run-custom-wezterm.ps1 -Path C:\src\wezterm
#
# open the new window in a specific directory:
# .\run-custom-wezterm.ps1 -Cwd C:\Users\jonas\Code2
#
# try a throwaway config instead of the real one:
# .\run-custom-wezterm.ps1 -Config C:\temp\smear-test.lua
#
# print the equivalent commands instead of running anything:
# .\run-custom-wezterm.ps1 -Build -ShowCmd

param(
    [Parameter(Position = 0)]
    [Alias('Path')]
    [string]$RepoPath = $PSScriptRoot,

    # Run `cargo build --release` before launching
    [switch]$Build,

    # Config file to pass through to wezterm; default is wezterm's own lookup,
    # which finds the same ~/.wezterm.lua the installed wezterm uses
    [string]$Config,

    # Directory the new window should start in
    [string]$Cwd,

    # Print the commands this run would execute instead of executing them
    [switch]$ShowCmd
)

function Write-Ok      ([string]$m) { Write-Host $m -ForegroundColor Green }
function Write-Err     ([string]$m) { Write-Host $m -ForegroundColor Red }
function Write-Warn    ([string]$m) { Write-Host $m -ForegroundColor DarkYellow }
function Write-Info    ([string]$m) { Write-Host $m -ForegroundColor Cyan }
function Write-InfoAlt ([string]$m) { Write-Host $m -ForegroundColor Magenta }

# Strawberry Perl is required to build the vendored openssl on Windows, and it
# must come ahead of the perl that ships with Git
$StrawberryBinDirs = @(
    'C:\Strawberry\c\bin',
    'C:\Strawberry\perl\site\bin',
    'C:\Strawberry\perl\bin'
)

$RepoPath = [System.IO.Path]::GetFullPath($RepoPath)
$ExePath = Join-Path $RepoPath 'target\release\wezterm-gui.exe'

$weztermArgs = @('start')
if ($Cwd) { $weztermArgs += @('--cwd', $Cwd) }

# wezterm wants --config-file before the subcommand
$leadingArgs = @()
if ($Config) { $leadingArgs += @('--config-file', $Config) }
$weztermArgs = $leadingArgs + $weztermArgs

if ($ShowCmd) {
    $origin = '# equivalent of: .\run-custom-wezterm.ps1'
    if ($Build) { $origin += ' -Build' }
    if ($Config) { $origin += " -Config $Config" }
    if ($Cwd) { $origin += " -Cwd $Cwd" }
    Write-InfoAlt $origin
    if ($Build) {
        Write-Host ('$env:PATH = "{0};" + $env:PATH' -f ($StrawberryBinDirs -join ';'))
        Write-Host ('cargo build --release --manifest-path "{0}\Cargo.toml" -p wezterm-gui -p wezterm' -f $RepoPath)
    }
    $quoted = $weztermArgs | ForEach-Object { '"{0}"' -f $_ }
    Write-Host ('& "{0}" {1}' -f $ExePath, ($quoted -join ' '))
    return
}

if ($Build) {
    Write-Info "Building release binaries in $RepoPath ..."
    foreach ($dir in $StrawberryBinDirs) {
        if (-not (Test-Path $dir)) {
            Write-Warn "Strawberry Perl dir not found: $dir"
            Write-Warn 'Install it with: winget install StrawberryPerl.StrawberryPerl'
        }
    }
    $env:PATH = ($StrawberryBinDirs -join ';') + ';' + $env:PATH
    cargo build --release --manifest-path (Join-Path $RepoPath 'Cargo.toml') -p wezterm-gui -p wezterm
    if ($LASTEXITCODE -ne 0) {
        Write-Err "cargo build failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }
    Write-Ok 'Build finished.'
}

if (-not (Test-Path $ExePath)) {
    Write-Err "Not built yet: $ExePath"
    Write-Warn 'Run this script with -Build, or run cargo build --release yourself.'
    exit 1
}

$installed = 'C:\Program Files\WezTerm\wezterm-gui.exe'
if (Test-Path $installed) {
    Write-Info "Installed wezterm is left alone: $installed"
}
Write-Info "Launching custom build: $ExePath"
if ($Config) {
    Write-InfoAlt "Config file: $Config"
} else {
    Write-InfoAlt 'Config file: wezterm default lookup (same file as the installed wezterm)'
}

& $ExePath @weztermArgs
if ($LASTEXITCODE -ne 0) {
    Write-Err "wezterm-gui exited with code $LASTEXITCODE"
    exit $LASTEXITCODE
}
Write-Ok 'Done.'
