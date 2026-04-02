#!/usr/bin/env bash
#
# Code Buddy - One Command Installer
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash -s nvidia YOUR_API_KEY
#
# With MLX support (macOS Apple Silicon):
#   curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash -s mlx
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
YELLOW='\033[1;33m'
NC='\033[0m'

echo ""
echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║                    Code Buddy Installer                        ║${NC}"
echo -e "${CYAN}║                      Version 2.1.89                           ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Darwin*)
            # Check if Apple Silicon
            if [[ "$(uname -m)" == "arm64" ]]; then
                echo "apple_silicon"
            else
                echo "macos"
            fi
            ;;
        Linux*)     echo "linux";;
        *)          echo "unknown";;
    esac
}

# Check for dependencies
check_deps() {
    local missing=""

    if ! command -v git &> /dev/null; then
        echo -e "${RED}[ERROR] git is required but not found.${NC}"
        echo ""
        echo "Install git:"
        echo "  Ubuntu/Debian: sudo apt install git"
        echo "  macOS:         brew install git"
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

# Install MLX dependencies (Apple Silicon)
install_mlx_deps() {
    local os=$(detect_os)

    if [[ "$os" != "apple_silicon" ]]; then
        return 0
    fi

    echo -e "${CYAN}[INFO] Apple Silicon Mac detected!${NC}"

    # Check for Python
    if ! command -v python3 &> /dev/null; then
        echo -e "${YELLOW}[WARN] Python3 not found. Installing...${NC}"
        if command -v brew &> /dev/null; then
            brew install python3
        else
            echo -e "${YELLOW}[WARN] Please install Python3 manually from python.org${NC}"
        fi
    fi

    # Check for mlx-lm
    if python3 -c "import mlx_lm" 2>/dev/null; then
        echo -e "${GREEN}[INFO] mlx-lm is already installed!${NC}"
    else
        echo -e "${CYAN}[INFO] Installing mlx-lm for MLX inference...${NC}"
        pip3 install mlx-lm --quiet || pip install mlx-lm --quiet || true
        if python3 -c "import mlx_lm" 2>/dev/null; then
            echo -e "${GREEN}[INFO] mlx-lm installed successfully!${NC}"
        else
            echo -e "${YELLOW}[WARN] Could not install mlx-lm. Run: pip3 install mlx-lm${NC}"
        fi
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
    local config_file

    # Determine config path based on OS
    case "$(detect_os)" in
        apple_silicon|macos)
            config_file="$HOME/Library/Application Support/code-buddy/config.json"
            ;;
        *)
            config_file="$HOME/.config/code-buddy/config.json"
            ;;
    esac

    mkdir -p "$(dirname "$config_file")"

    # MLX-specific configuration
    if [[ "$PROVIDER" == "mlx" ]]; then
        cat > "$config_file" << EOF
{
  "api_key": null,
  "llm_provider": "mlx",
  "model": "mlx-community/llama-3.2-3b-instruct-4bit",
  "base_url": null,
  "permission_mode": null,
  "additional_dirs": [],
  "mcp_servers": {},
  "agents": {},
  "project_choices": {},
  "session_history": [],
  "auto_compact": true,
  "compact_threshold": 85,
  "compact_messages": 20
}
EOF
        echo -e "${GREEN}[INFO] MLX configuration saved to $config_file${NC}"
        return
    fi

    # Standard configuration
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
  "session_history": [],
  "auto_compact": true,
  "compact_threshold": 85,
  "compact_messages": 20
}
EOF

    echo -e "${GREEN}[INFO] Configuration saved to $config_file${NC}"
}

# Print MLX info
print_mlx_info() {
    local os=$(detect_os)

    if [[ "$os" != "apple_silicon" ]]; then
        return
    fi

    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}                    MLX Setup Complete!                        ${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo "Your Apple Silicon Mac is configured for local LLM inference!"
    echo ""
    echo "Available MLX models:"
    echo "  • Llama 3.2 3B (~2GB) - Balanced"
    echo "  • Llama 3.2 1B (~700MB) - Fast"
    echo "  • Qwen 2.5 1.5B (~1GB) - Efficient"
    echo "  • Gemma 2B (~1.8GB) - Google's model"
    echo "  • Mistral 7B (~4GB) - High quality"
    echo ""
    echo "To download and use MLX models:"
    echo "  code-buddy --mlx"
    echo "  code-buddy --mlx-list-models"
    echo ""
    echo "Quick start with Llama 3.2 3B:"
    echo "  code-buddy --mlx-download mlx-community/llama-3.2-3b-instruct-4bit"
    echo "  code-buddy -p \"Hello, world!\""
    echo ""
}

# Print completion info
print_completion() {
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
    echo "Providers:"
    echo "  code-buddy setup            Guided setup"
    echo "  code-buddy -p \"Hello\"     Use default (NVIDIA)"
    echo ""

    if [[ "$PROVIDER" == "mlx" ]]; then
        print_mlx_info
    fi

    echo "Need help? https://github.com/simpletoolsindia/code-buddy"
    echo ""
}

# Show help
show_help() {
    echo "Usage: install-simple.sh [provider] [api-key] [options]"
    echo ""
    echo "Arguments:"
    echo "  provider     LLM provider (nvidia, openrouter, ollama, mlx, anthropic, openai, etc.)"
    echo "  api-key      API key for the provider (optional)"
    echo ""
    echo "Options:"
    echo "  --no-deps    Skip dependency installation (Rust must be installed)"
    echo ""
    echo "Examples:"
    echo "  # Interactive setup"
    echo "  curl -fsSL ... | bash"
    echo ""
    echo "  # NVIDIA NIM (FREE tier)"
    echo "  curl -fsSL ... | bash -s nvidia YOUR_API_KEY"
    echo ""
    echo "  # MLX (Apple Silicon - FREE, local)"
    echo "  curl -fsSL ... | bash -s mlx"
    echo ""
    echo "  # OpenRouter (free models)"
    echo "  curl -fsSL ... | bash -s openrouter YOUR_API_KEY"
    echo ""
    echo "  # Skip Rust installation"
    echo "  curl -fsSL ... | bash -s nvidia KEY --no-deps"
    echo ""
}

# Main
main() {
    # Show help if requested
    if [[ "$PROVIDER" == "--help" ]] || [[ "$PROVIDER" == "-h" ]]; then
        show_help
        exit 0
    fi

    check_deps

    # Install MLX dependencies if using MLX
    if [[ "$PROVIDER" == "mlx" ]]; then
        install_mlx_deps
    fi

    install

    if [ -n "$PROVIDER" ]; then
        configure
    fi

    print_completion
}

main
