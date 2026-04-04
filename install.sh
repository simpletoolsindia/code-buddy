#!/usr/bin/env bash
# install.sh — Code Buddy installer
#
# Usage:
#   bash install.sh
#   bash install.sh --prefix /usr/local
#   bash install.sh --check      # verify existing install only
#   bash install.sh --help

set -euo pipefail

# ── Constants ──────────────────────────────────────────────────────────────────

APP_NAME="code-buddy"
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_PREFIX="${HOME}/.local"

# ── Colour helpers ─────────────────────────────────────────────────────────────

if [ -t 1 ] && command -v tput &>/dev/null && tput colors &>/dev/null && [ "$(tput colors)" -ge 8 ]; then
    GREEN=$(tput setaf 2)
    YELLOW=$(tput setaf 3)
    RED=$(tput setaf 1)
    BOLD=$(tput bold)
    RESET=$(tput sgr0)
else
    GREEN="" YELLOW="" RED="" BOLD="" RESET=""
fi

info()    { echo "${BOLD}${GREEN}==> ${RESET}${BOLD}$*${RESET}"; }
warn()    { echo "${BOLD}${YELLOW}[warn]${RESET} $*" >&2; }
error()   { echo "${BOLD}${RED}[error]${RESET} $*" >&2; }
success() { echo "${BOLD}${GREEN}[ok]${RESET}   $*"; }
fail()    { error "$*"; exit 1; }

# ── Argument parsing ──────────────────────────────────────────────────────────

INSTALL_PREFIX=""
CHECK_ONLY=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --check)
            CHECK_ONLY=true
            shift
            ;;
        --prefix)
            if [[ $# -lt 2 ]]; then
                fail "--prefix requires an argument"
            fi
            INSTALL_PREFIX="$2"
            shift 2
            ;;
        --prefix=*)
            INSTALL_PREFIX="${1#--prefix=}"
            shift
            ;;
        -h|--help)
            cat <<EOF
Usage: $0 [OPTIONS]

Options:
  --prefix DIR    Install root (binary goes to DIR/bin). Default: ~/.local
  --check         Verify existing installation without building
  -h, --help      Show this help message

The installer uses \`cargo install --path crates/cli --root <prefix>\` so the
binary is placed at <prefix>/bin/code-buddy (default: ~/.local/bin/code-buddy).
EOF
            exit 0
            ;;
        *)
            warn "Unknown argument: $1 (ignored)"
            shift
            ;;
    esac
done

INSTALL_PREFIX="${INSTALL_PREFIX:-$DEFAULT_PREFIX}"
INSTALL_BIN_DIR="${INSTALL_PREFIX}/bin"

# ── Platform detection ────────────────────────────────────────────────────────

detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  PLATFORM="linux" ;;
        Darwin) PLATFORM="macos" ;;
        *) fail "Unsupported operating system: $os (only Linux and macOS are supported)" ;;
    esac

    case "$arch" in
        x86_64)        ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        *) warn "Unusual architecture: $arch — attempting build anyway"; ARCH="$arch" ;;
    esac

    info "Platform: ${PLATFORM} / ${ARCH}"
}

# ── Rust toolchain ────────────────────────────────────────────────────────────

ensure_rust() {
    if command -v cargo &>/dev/null; then
        success "Rust toolchain found: $(rustc --version 2>/dev/null || echo 'unknown')"
        return 0
    fi

    warn "Rust toolchain not found — installing via rustup..."

    if ! command -v curl &>/dev/null && ! command -v wget &>/dev/null; then
        fail "Neither curl nor wget found. Please install one of them and retry."
    fi

    local rustup_init
    rustup_init="$(mktemp /tmp/rustup-init.XXXXXX)"
    # Ensure temp file is removed on exit
    # shellcheck disable=SC2064
    trap "rm -f '$rustup_init'" EXIT

    if command -v curl &>/dev/null; then
        curl -fsSL "https://sh.rustup.rs" -o "$rustup_init"
    else
        wget -qO "$rustup_init" "https://sh.rustup.rs"
    fi

    chmod +x "$rustup_init"
    "$rustup_init" -y --no-modify-path --profile minimal --default-toolchain stable

    # Source cargo env so subsequent steps find cargo
    if [ -f "${HOME}/.cargo/env" ]; then
        # shellcheck source=/dev/null
        source "${HOME}/.cargo/env"
    fi

    # Fallback: add to PATH directly
    if ! command -v cargo &>/dev/null; then
        export PATH="${HOME}/.cargo/bin:${PATH}"
    fi

    if command -v cargo &>/dev/null; then
        success "Rust installed: $(rustc --version)"
    else
        fail "rustup install appeared to succeed but cargo is still not in PATH."
    fi
}

# ── Build & install ───────────────────────────────────────────────────────────

install_binary() {
    info "Installing ${APP_NAME} to ${INSTALL_BIN_DIR}..."

    if [ ! -f "${REPO_DIR}/Cargo.toml" ]; then
        fail "Cargo.toml not found in ${REPO_DIR}. Run this script from the repository root."
    fi

    # Verify the CLI crate exists — more reliable than grepping the workspace manifest.
    if [ ! -f "${REPO_DIR}/crates/cli/Cargo.toml" ]; then
        fail "crates/cli/Cargo.toml not found. Is ${REPO_DIR} the code-buddy repository root?"
    fi

    # Show existing version if upgrading
    local dest="${INSTALL_BIN_DIR}/${APP_NAME}"
    if [ -f "$dest" ]; then
        local old_ver
        old_ver="$("$dest" --version 2>/dev/null || echo 'unknown')"
        info "Upgrading existing install (was: ${old_ver})"
    fi

    # `cargo install --path . --root <prefix>` compiles a release binary and
    # places it at <prefix>/bin/code-buddy — exactly what the task requires.
    (
        cd "$REPO_DIR"
        cargo install \
            --path "crates/cli" \
            --root "$INSTALL_PREFIX" \
            --bin "$APP_NAME" \
            2>&1
    ) || fail "cargo install failed. Check the output above for errors."

    if [ ! -f "$dest" ]; then
        fail "Expected binary at ${dest} after install but it was not found."
    fi

    success "Installed to ${dest}"
}

# ── PATH guidance ─────────────────────────────────────────────────────────────

check_path() {
    if command -v "$APP_NAME" &>/dev/null; then
        success "${APP_NAME} is in PATH at: $(command -v "$APP_NAME")"
        return 0
    fi

    warn "${INSTALL_BIN_DIR} is not in your PATH."
    echo ""
    echo "  Add it to your shell profile:"
    echo ""

    local shell_profile
    case "${SHELL:-bash}" in
        */zsh)  shell_profile="~/.zshrc" ;;
        */fish) shell_profile="~/.config/fish/config.fish" ;;
        *)      shell_profile="~/.bashrc" ;;
    esac

    if [[ "${SHELL:-bash}" == */fish ]]; then
        echo "    ${YELLOW}fish_add_path ${INSTALL_BIN_DIR}${RESET}"
    else
        echo "    ${YELLOW}echo 'export PATH=\"${INSTALL_BIN_DIR}:\$PATH\"' >> ${shell_profile}${RESET}"
        echo "    ${YELLOW}source ${shell_profile}${RESET}"
    fi

    echo ""
    echo "  Or run directly:  ${YELLOW}${INSTALL_BIN_DIR}/${APP_NAME} --help${RESET}"
    echo ""
}

# ── Verification ──────────────────────────────────────────────────────────────

verify_install() {
    info "Verifying installation..."

    local bin="${INSTALL_BIN_DIR}/${APP_NAME}"

    if [ ! -f "$bin" ]; then
        if command -v "$APP_NAME" &>/dev/null; then
            bin="$(command -v "$APP_NAME")"
        else
            error "${APP_NAME} not found at ${bin} or in PATH."
            return 1
        fi
    fi

    if "$bin" --help &>/dev/null; then
        success "  --help:    OK"
    else
        error "  --help failed (exit code $?)"
        return 1
    fi

    local ver
    ver="$("$bin" --version 2>/dev/null || echo '')"
    if [ -n "$ver" ]; then
        success "  version:   ${ver}"
    fi

    if "$bin" config show &>/dev/null; then
        success "  config:    OK"
    else
        warn "  config show returned non-zero (config may not exist yet — OK on first install)"
    fi

    success "  binary:    ${bin}"

    echo ""
    echo "  ${BOLD}Quick start:${RESET}"
    echo "    ${APP_NAME} config set provider lm-studio"
    echo "    ${APP_NAME} config set model mistral-7b-instruct"
    echo "    ${APP_NAME} run"
    echo ""
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
    echo ""
    echo "  ${BOLD}Code Buddy Installer${RESET}"
    echo "  ─────────────────────────────────────────"
    echo ""

    detect_platform

    if $CHECK_ONLY; then
        verify_install
        exit $?
    fi

    ensure_rust
    install_binary
    check_path
    verify_install

    echo "${BOLD}${GREEN}Installation complete!${RESET}"
    echo ""
    echo "  Run ${BOLD}${APP_NAME} --help${RESET} to get started."
    echo "  Run ${BOLD}${APP_NAME} install${RESET} at any time to verify your setup."
    echo ""
}

main "$@"
