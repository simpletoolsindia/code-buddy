#!/usr/bin/env bash
#
# Code Buddy - One Command Installer
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash -s nvidia YOUR_API_KEY
#
# With Rust already installed:
#   curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash -s --no-deps
#

set -e

PROVIDER="${1:-}"
API_KEY="${2:-}"
NO_DEPS="${3:-}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

echo ""
echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║                    Code Buddy Installer                        ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check for dependencies
check_deps() {
    local missing=""

    if ! command -v git &> /dev/null; then
        missing="$missing git"
    fi

    # Check for git
    if ! command -v git &> /dev/null; then
        echo -e "${RED}[ERROR] git is required but not found.${NC}"
        echo ""
        echo "Install git:"
        echo "  Ubuntu/Debian: sudo apt install git"
        echo "  macOS:         brew install git"
        echo "  Windows:       Download from https://git-scm.com"
        echo ""
        exit 1
    fi

    # Install Rust if not present
    if ! command -v cargo &> /dev/null; then
        echo -e "${CYAN}[INFO] Rust not found. Installing Rust...${NC}"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "${HOME}/.cargo/env"
        echo -e "${GREEN}[INFO] Rust installed successfully!${NC}"
    fi
}

# Install Code Buddy
install() {
    local install_cmd="cargo install --git https://github.com/simpletoolsindia/code-buddy.git --force code-buddy"

    if [ -n "$NO_DEPS" ]; then
        install_cmd="$install_cmd --no-deps"
    fi

    echo -e "${GREEN}[INFO] Installing Code Buddy...${NC}"
    eval $install_cmd
}

# Configure if args provided
configure() {
    if [ -z "$PROVIDER" ] && [ -z "$API_KEY" ]; then
        return
    fi

    echo -e "${GREEN}[INFO] Configuring Code Buddy...${NC}"

    local config_file="$HOME/.config/code-buddy/config.json"
    mkdir -p "$(dirname "$config_file")"

    # Create config
    cat > "$config_file" << EOF
{
  "api_key": $([ -n "$API_KEY" ] && echo "\"$API_KEY\"" || echo "null"),
  "llm_provider": "$PROVIDER",
  "model": null,
  "base_url": null,
  "permission_mode": null,
  "additional_dirs": [],
  "mcp_servers": {},
  "agents": {},
  "project_choices": {},
  "session_history": []
}
EOF

    echo -e "${GREEN}[INFO] Configuration saved to $config_file${NC}"
}

# Main
main() {
    check_deps
    install

    if [ -n "$PROVIDER" ]; then
        configure
    fi

    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                  Installation Complete!                       ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "Quick Start:"
    echo "  code-buddy --help           Show help"
    echo "  code-buddy setup            Interactive setup"
    echo "  code-buddy -p \"Hello!\"    Run a prompt"
    echo ""
    echo "Need help? https://github.com/simpletoolsindia/code-buddy"
    echo ""
}

main
