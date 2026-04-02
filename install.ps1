# Code Buddy Installer for Windows
# One-command installation for Windows
#
# Usage:
#   irm https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.ps1 | iex
#   irm https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.ps1 | iex -ProviderName nvidia -ApiKey YOUR_KEY
#
# Supported Providers:
#   nvidia, openrouter, ollama, anthropic, openai, groq, deepseek, mistral, etc.
#
# Note: MLX is not available on Windows (requires Apple Silicon)
#

param(
    [string]$Provider = "",
    [string]$ApiKey = "",
    [string]$Model = "",
    [switch]$Help
)

if ($Help) {
    Write-Host ""
    Write-Host "Code Buddy Installer for Windows"
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  irm .../install.ps1 | iex"
    Write-Host "  irm .../install.ps1 | iex -ProviderName nvidia -ApiKey YOUR_KEY"
    Write-Host ""
    Write-Host "Parameters:"
    Write-Host "  -ProviderName   LLM provider (nvidia, openrouter, ollama, anthropic, etc.)"
    Write-Host "  -ApiKey         API key for the provider"
    Write-Host "  -Model          Model name (optional)"
    Write-Host "  -Help           Show this help"
    Write-Host ""
    Write-Host "Examples:"
    Write-Host "  # Interactive setup"
    Write-Host "  irm .../install.ps1 | iex"
    Write-Host ""
    Write-Host "  # NVIDIA NIM (FREE tier)"
    Write-Host "  irm .../install.ps1 | iex -ProviderName nvidia -ApiKey YOUR_KEY"
    Write-Host ""
    Write-Host "  # OpenRouter (free models)"
    Write-Host "  irm .../install.ps1 | iex -ProviderName openrouter -ApiKey YOUR_KEY"
    Write-Host ""
    Write-Host "Note: MLX is not available on Windows (requires Apple Silicon)"
    Write-Host ""
    exit 0
}

$ErrorActionPreference = "Stop"
$Version = "2.1.89"
$Repo = "simpletoolsindia/code-buddy"
$InstallDir = "$env:LOCALAPPDATA\Programs\code-buddy"
$BinaryName = "code-buddy.exe"

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Write-Err {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

Write-Host ""
Write-Host "╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║                    Code Buddy Installer                        ║" -ForegroundColor Cyan
Write-Host "║                    Version $Version                             ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

Write-Info "Detected OS: Windows"

# Create install directory
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

# Check if Rust is installed
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Warn "Rust not found. Installing Rust..."
    $rustInstaller = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs" -OutFile $rustInstaller
    Start-Process -FilePath $rustInstaller -ArgumentList "-y" -Wait
    Remove-Item $rustInstaller -Force

    # Refresh environment
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
}

Write-Info "Rust is ready"

# Clone or update repo
$TempDir = "$env:TEMP\code-buddy"
if (Test-Path $TempDir) {
    Set-Location $TempDir
    git pull | Out-Null
} else {
    git clone "https://github.com/$Repo.git" $TempDir | Out-Null
    Set-Location $TempDir
}

Write-Info "Building Code Buddy from source..."
cargo build --release

# Copy binary
Copy-Item "target\release\$BinaryName" "$InstallDir\$BinaryName" -Force

# Add to PATH
$userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    [System.Environment]::SetEnvironmentVariable(
        "Path",
        "$userPath;$InstallDir",
        "User"
    )
    Write-Info "Added $InstallDir to your PATH"
}

# Configure if options provided
if ($Provider -or $ApiKey) {
    Write-Info "Configuring Code Buddy..."

    $configDir = "$env:APPDATA\code-buddy"
    New-Item -ItemType Directory -Force -Path $configDir | Out-Null

    $config = @{
        api_key = if ($ApiKey) { $ApiKey } else { $null }
        llm_provider = if ($Provider) { $Provider } else { "ollama" }
        model = if ($Model) { $Model } else { $null }
        base_url = $null
        permission_mode = $null
        additional_dirs = @()
        mcp_servers = @{}
        agents = @{}
        project_choices = @{}
        session_history = @()
        auto_compact = $true
        compact_threshold = 85
        compact_messages = 20
    } | ConvertTo-Json -Depth 10

    Set-Content -Path "$configDir\config.json" -Value $config
    Write-Info "Configuration saved to $configDir\config.json"
}

Write-Host ""
Write-Host "╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║                  Installation Complete!                       ║" -ForegroundColor Green
Write-Host "╚════════════════════════════════════════════════════════════════╝" -ForegroundColor Green
Write-Host ""
Write-Host "Code Buddy has been installed to: $InstallDir\$BinaryName"
Write-Host ""
Write-Host "Quick Start:"
Write-Host "  1. Restart your terminal (to refresh PATH)"
Write-Host "  2. Run: code-buddy setup"
Write-Host "  3. Or: code-buddy -p `"Hello, world!`""
Write-Host ""
Write-Host "Supported Providers:"
Write-Host "  nvidia, openrouter, ollama, anthropic, openai, groq, deepseek..."
Write-Host "  (MLX requires Apple Silicon - not available on Windows)"
Write-Host ""
Write-Host "Need help? Visit: https://github.com/$Repo"
Write-Host ""

# Verify installation
& "$InstallDir\$BinaryName" --version 2>$null | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Info "Installation verified successfully!"
} else {
    Write-Err "Installation may have failed. Please restart your terminal and try again."
}
