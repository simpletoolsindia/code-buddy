# install.ps1 — Code Buddy Windows installer
#
# Installs the latest pre-built Windows binary from GitHub Releases to
# %LOCALAPPDATA%\code-buddy\bin\ and adds it to the user PATH.
#
# One-line install (PowerShell):
#   irm https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.ps1 | iex

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$AppName    = "code-buddy"
$Repo       = "simpletoolsindia/code-buddy"
$ApiUrl     = "https://api.github.com/repos/$Repo/releases/latest"
$Target     = "x86_64-pc-windows-msvc"
$InstallDir = Join-Path $env:LOCALAPPDATA "code-buddy\bin"

function Write-Info  { param([string]$msg) Write-Host "  [*] $msg" -ForegroundColor Cyan }
function Write-Ok    { param([string]$msg) Write-Host "  [+] $msg" -ForegroundColor Green }
function Write-Warn  { param([string]$msg) Write-Host "  [!] $msg" -ForegroundColor Yellow }
function Write-Fail  { param([string]$msg) Write-Error "  [x] $msg"; exit 1 }

Write-Host ""
Write-Host "  * Welcome to Code Buddy Installer" -ForegroundColor Magenta
Write-Host ""

# ── Fetch latest release ───────────────────────────────────────────────────────
Write-Info "Fetching latest release from GitHub…"
try {
    $release = Invoke-RestMethod -Uri $ApiUrl -UseBasicParsing
} catch {
    Write-Fail "Failed to fetch release info: $_"
}

$tag = $release.tag_name
if (-not $tag) { Write-Fail "Could not determine latest release tag." }
Write-Info "Latest release: $tag"

$archiveName = "$AppName-$tag-$Target.zip"
$downloadUrl = "https://github.com/$Repo/releases/download/$tag/$archiveName"
$checksumUrl = "https://github.com/$Repo/releases/download/$tag/checksums.txt"

# ── Download ───────────────────────────────────────────────────────────────────
$tmpDir = Join-Path $env:TEMP "code-buddy-install"
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null

$archivePath = Join-Path $tmpDir $archiveName
Write-Info "Downloading $archiveName…"
try {
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing
} catch {
    Write-Fail "Download failed: $_. Check your internet connection."
}

# ── Checksum ───────────────────────────────────────────────────────────────────
try {
    $checksumPath = Join-Path $tmpDir "checksums.txt"
    Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumPath -UseBasicParsing
    $expected = (Select-String -Path $checksumPath -Pattern $archiveName |
        Select-Object -First 1).Line.Split()[0]
    $actual = (Get-FileHash $archivePath -Algorithm SHA256).Hash.ToLower()
    if ($expected -ne $actual) {
        Write-Fail "Checksum mismatch! Expected $expected but got $actual."
    }
    Write-Ok "Checksum verified."
} catch {
    Write-Warn "Could not verify checksum (non-fatal): $_"
}

# ── Extract & install ──────────────────────────────────────────────────────────
Write-Info "Installing to $InstallDir…"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Expand-Archive -Path $archivePath -DestinationPath $tmpDir -Force

$exeSrc = Join-Path $tmpDir "$AppName.exe"
if (-not (Test-Path $exeSrc)) {
    # Some archives nest in a subdirectory.
    $exeSrc = Get-ChildItem -Path $tmpDir -Recurse -Filter "$AppName.exe" |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not $exeSrc) { Write-Fail "$AppName.exe not found in archive." }

Copy-Item $exeSrc (Join-Path $InstallDir "$AppName.exe") -Force
Write-Ok "$AppName $tag installed to $InstallDir"

# ── PATH ───────────────────────────────────────────────────────────────────────
$userPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$InstallDir*") {
    Write-Info "Adding $InstallDir to user PATH…"
    [System.Environment]::SetEnvironmentVariable(
        "PATH",
        "$InstallDir;$userPath",
        "User"
    )
    $env:PATH = "$InstallDir;$env:PATH"
    Write-Ok "PATH updated. Restart your shell for it to take effect."
} else {
    Write-Ok "$InstallDir already in PATH."
}

# ── Cleanup ────────────────────────────────────────────────────────────────────
Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "  ✔  Installation complete!" -ForegroundColor Green
Write-Host "     Run 'code-buddy' in a new terminal to get started."
Write-Host "     The setup wizard will run automatically on first launch."
Write-Host ""
