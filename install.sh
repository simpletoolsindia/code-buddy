#!/usr/bin/env bash
# install.sh — Code Buddy installer
#
# Usage:
#   curl -fsSL https://example.com/install.sh | bash
#   bash install.sh
#   bash install.sh --prefix /usr/local/bin
#   bash install.sh --check      # verify existing install only
#
# This script is idempotent: re-running it will upgrade an existing install.

set -euo pipefail

# ── Constants ──────────────────────────────────────────────────────────────────

APP_NAME="code-buddy"
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_INSTALL_DIR="${HOME}/.local/bin"

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

# ── CLI flags ─────────────────────────────────────────────────────────────────

INSTALL_DIR=""
CHECK_ONLY=false

for arg in "$@"; do
    case "$arg" in
        --check)        CHECK_ONLY=true ;;
        --prefix=*)     INSTALL_DIR="${arg#--prefix=}" ;;
        --prefix)       shift; INSTALL_DIR="$1" ;;
        -h|--help)
            cat <<EOF
Usage: $0 [OPTIONS]

Options:
  --prefix DIR    Install binary to DIR (default: ~/.local/bin)
  --check         Verify existing installation without building
  -h, --help      Show this help message
EOF
            exit 0
            ;;
        *) warn "Unknown argument: $arg" ;;
    esac
done

INSTALL_DIR="${INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"

# ── Platform detection ────────────────────────────────────────────────────────

detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  PLATFORM="linux" ;;
        Darwin) PLATFORM="macos" ;;
        *)      fail "Unsupported operating system: $os (only Linux and macOS are supported)" ;;
    esac

    case "$arch" in
        x86_64)           ARCH="x86_64" ;;
        aarch64|arm64)    ARCH="aarch64" ;;
        *)                warn "Unusual architecture $arch — attempting build anyway" ; ARCH="$arch" ;;
    esac

    info "Platform: ${PLATFORM} / ${ARCH}"
}

# ── Rust toolchain check ──────────────────────────────────────────────────────

ensure_rust() {
    if command -v cargo &>/dev/null; then
        local rust_ver
        rust_ver="$(rustc --version 2>/dev/null || echo 'unknown')"
        success "Rust toolchain found: $rust_ver"
        return 0
    fi

    warn "Rust toolchain not found. Installing via rustup..."

    if ! command -v curl &>/dev/null && ! command -v wget &>/dev/null; then
        fail "Neither curl nor wget found. Please install one of them and retry."
    fi

    local rustup_init
    rustup_init="$(mktemp /tmp/rustup-init.XXXXXX)"
    trap 'rm -f "$rustup_init"' EXIT

    if command -v curl &>/dev/null; then
        curl -fsSL "https://sh.rustup.rs" -o "$rustup_init"
    else
        wget -qO "$rustup_init" "https://sh.rustup.rs"
    fi

    chmod +x "$rustup_init"

    # Install rustup non-interactively with the stable toolchain and no PATH modifications
    "$rustup_init" -y --no-modify-path --profile minimal --default-toolchain stable 2>&1 \
        | grep -v "^info:" || true

    # Source the cargo env so the rest of the script can use it
    # shellcheck source=/dev/null
    if [ -f "${HOME}/.cargo/env" ]; then
        source "${HOME}/.cargo/env"
    fi

    if ! command -v cargo &>/dev/null; then
        # Cargo might not be in PATH yet; add it explicitly
        export PATH="${HOME}/.cargo/bin:${PATH}"
    fi

    if command -v cargo &>/dev/null; then
        success "Rust installed: $(rustc --version)"
    else
        fail "rustup installation appeared to succeed but cargo is still not found."
    fi
}

# ── Build ─────────────────────────────────────────────────────────────────────

build_binary() {
    info "Building ${APP_NAME} (release)..."

    if [ ! -f "${REPO_DIR}/Cargo.toml" ]; then
        fail "Cargo.toml not found in ${REPO_DIR}. Run this script from the repository root."
    fi

    # Verify this looks like the right workspace
    if ! grep -q "code-buddy" "${REPO_DIR}/Cargo.toml" 2>/dev/null; then
        fail "The Cargo.toml at ${REPO_DIR} does not appear to be the code-buddy workspace."
    fi

    (
        cd "$REPO_DIR"
        cargo build --release --bin code-buddy 2>&1
    ) || fail "Build failed. Check the output above for errors."

    success "Build complete."
}

# ── Install ───────────────────────────────────────────────────────────────────

install_binary() {
    local src="${REPO_DIR}/target/release/${APP_NAME}"

    if [ ! -f "$src" ]; then
        fail "Expected binary at ${src} but it does not exist. Build step may have failed."
    fi

    mkdir -p "$INSTALL_DIR"

    # Check if an older install exists and show the version change
    local dest="${INSTALL_DIR}/${APP_NAME}"
    if [ -f "$dest" ]; then
        local old_ver
        old_ver="$("$dest" --version 2>/dev/null || echo 'unknown')"
        info "Upgrading existing install (was: ${old_ver})"
    fi

    cp "$src" "$dest"
    chmod +x "$dest"
    success "Installed to ${dest}"
}

# ── PATH guidance ─────────────────────────────────────────────────────────────

check_path() {
    local dest="${INSTALL_DIR}/${APP_NAME}"

    if command -v "$APP_NAME" &>/dev/null; then
        local found_at
        found_at="$(command -v "$APP_NAME")"
        success "${APP_NAME} is in PATH at: ${found_at}"
        return 0
    fi

    warn "${INSTALL_DIR} is not in your PATH."
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
        echo "    ${YELLOW}fish_add_path ${INSTALL_DIR}${RESET}"
    else
        echo "    ${YELLOW}echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ${shell_profile}${RESET}"
        echo "    ${YELLOW}source ${shell_profile}${RESET}"
    fi

    echo ""
    echo "  Or run directly:"
    echo "    ${YELLOW}${dest} --help${RESET}"
    echo ""
}

# ── Verification ──────────────────────────────────────────────────────────────

verify_install() {
    info "Verifying installation..."

    local bin="${INSTALL_DIR}/${APP_NAME}"

    if [ ! -f "$bin" ]; then
        # Also check PATH
        if ! command -v "$APP_NAME" &>/dev/null; then
            error "${APP_NAME} binary not found at ${bin} or in PATH."
            return 1
        fi
        bin="$(command -v "$APP_NAME")"
    fi

    # Check --help works
    if "$bin" --help &>/dev/null; then
        success "  --help:    OK"
    else
        error "  --help failed (exit code $?)"
        return 1
    fi

    # Check --version
    local ver
    ver="$("$bin" --version 2>/dev/null || echo '')"
    if [ -n "$ver" ]; then
        success "  version:   ${ver}"
    fi

    # Check config subcommand
    if "$bin" config show &>/dev/null; then
        success "  config:    OK"
    else
        warn "  config show returned non-zero (config may not exist yet — that's OK)"
    fi

    # Show binary location and config path
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
    build_binary
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
