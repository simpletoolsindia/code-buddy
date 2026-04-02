#!/usr/bin/env bash
#
# Code Buddy Installer
# One-command installation for Linux and macOS
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.sh | bash -s -- --provider nvidia --api-key YOUR_KEY
#

set -e

VERSION="2.1.88"
REPO="simpletoolsindia/code-buddy"
INSTALL_DIR="${HOME}/.local/bin"
BINARY_NAME="code-buddy"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)     echo "linux";;
        Darwin*)    echo "macos";;
        CYGWIN*|MINGW*|MSYS*) echo "windows";;
        *)          echo "unknown";;
    esac
}

# Detect package manager
detect_pkg_manager() {
    if command -v apt-get &> /dev/null; then
        echo "apt"
    elif command -v brew &> /dev/null; then
        echo "brew"
    elif command -v dnf &> /dev/null; then
        echo "dnf"
    elif command -v pacman &> /dev/null; then
        echo "pacman"
    elif command -v apk &> /dev/null; then
        echo "apk"
    else
        echo "none"
    fi
}

# Install Rust if not present
install_rust() {
    log_info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "${HOME}/.cargo/env"

    # Add to PATH for this session
    export PATH="${HOME}/.cargo/bin:${PATH}"
}

# Build from source
build_from_source() {
    log_info "Building Code Buddy from source..."

    # Clone or update repo
    if [ -d "/tmp/code-buddy" ]; then
        cd /tmp/code-buddy
        git pull
    else
        git clone https://github.com/${REPO}.git /tmp/code-buddy
        cd /tmp/code-buddy
    fi

    # Build release
    cargo build --release

    # Install binary
    mkdir -p "${INSTALL_DIR}"
    cp "target/release/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
}

# Download pre-built binary (when releases are available)
download_binary() {
    local os="$1"
    local arch="$2"
    local download_url="https://github.com/${REPO}/releases/download/v${VERSION}/${BINARY_NAME}-${os}-${arch}"

    log_info "Downloading Code Buddy..."

    mkdir -p "${INSTALL_DIR}"
    curl -fsSL "${download_url}" -o "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
}

# Configure Code Buddy
configure() {
    local provider="$1"
    local api_key="$2"
    local model="$3"

    log_info "Configuring Code Buddy..."

    # Create config directory
    local config_dir
    case "$(detect_os)" in
        linux)  config_dir="${HOME}/.config/code-buddy";;
        macos)  config_dir="${HOME}/Library/Application Support/code-buddy";;
        *)      config_dir="${HOME}/.config/code-buddy";;
    esac

    mkdir -p "${config_dir}"

    # Create config file
    cat > "${config_dir}/config.json" << EOF
{
  "api_key": "${api_key:-null}",
  "llm_provider": "${provider:-ollama}",
  "model": ${model:+"\"${model}\""} || null,
  "base_url": null,
  "permission_mode": null,
  "additional_dirs": [],
  "mcp_servers": {},
  "agents": {},
  "project_choices": {},
  "session_history": []
}
EOF

    log_info "Configuration saved to ${config_dir}/config.json"
}

# Add to shell profile
add_to_shell_profile() {
    local profile="$1"

    # Check if already added
    if grep -q "code-buddy" "${profile}" 2>/dev/null; then
        return
    fi

    echo "" >> "${profile}"
    echo "# Code Buddy" >> "${profile}"
    echo 'export PATH="${HOME}/.local/bin:${PATH}"' >> "${profile}"

    log_info "Added to ${profile}"
}

# Print setup instructions
print_instructions() {
    echo ""
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║                  Installation Complete!                     ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo ""
    echo "Code Buddy has been installed to: ${INSTALL_DIR}/${BINARY_NAME}"
    echo ""
    echo "Quick Start:"
    echo "  1. Make sure ${INSTALL_DIR} is in your PATH"
    echo "  2. Run: code-buddy setup"
    echo "  3. Or: code-buddy -p \"Hello, world!\""
    echo ""
    echo "Need help? Visit: https://github.com/${REPO}"
    echo ""
}

# Parse arguments
parse_args() {
    PROVIDER=""
    API_KEY=""
    MODEL=""

    while [[ $# -gt 0 ]]; do
        case $1 in
            --provider)
                PROVIDER="$2"
                shift 2
                ;;
            --api-key)
                API_KEY="$2"
                shift 2
                ;;
            --model)
                MODEL="$2"
                shift 2
                ;;
            --help)
                echo "Usage: install.sh [options]"
                echo ""
                echo "Options:"
                echo "  --provider <name>    Set LLM provider (ollama, nvidia, openrouter, etc.)"
                echo "  --api-key <key>      Set API key"
                echo "  --model <name>      Set model name"
                echo "  --help              Show this help"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
}

# Main installation
main() {
    echo ""
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║                    Code Buddy Installer                        ║"
    echo "║                    Version ${VERSION}                             ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo ""

    # Parse arguments
    parse_args "$@"

    # Detect OS
    OS=$(detect_os)
    PKG_MANAGER=$(detect_pkg_manager)

    log_info "Detected OS: ${OS}"
    log_info "Package manager: ${PKG_MANAGER}"

    # Create install directory
    mkdir -p "${INSTALL_DIR}"

    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        log_warn "Rust not found. Installing Rust..."
        install_rust
    else
        log_info "Rust is already installed"
    fi

    # Build from source
    build_from_source

    # Configure if options provided
    if [ -n "${PROVIDER}" ] || [ -n "${API_KEY}" ]; then
        configure "${PROVIDER}" "${API_KEY}" "${MODEL}"
    fi

    # Add to shell profile
    case "${OS}" in
        linux)
            add_to_shell_profile "${HOME}/.bashrc"
            add_to_shell_profile "${HOME}/.zshrc"
            ;;
        macos)
            add_to_shell_profile "${HOME}/.zshrc"
            ;;
    esac

    # Print instructions
    print_instructions

    # Verify installation
    if "${INSTALL_DIR}/${BINARY_NAME}" --version &> /dev/null; then
        log_info "Installation verified successfully!"
    else
        log_error "Installation may have failed. Please add ${INSTALL_DIR} to your PATH."
    fi
}

main "$@"
