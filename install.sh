#!/usr/bin/env bash
# install.sh — Code Buddy installer
#
# Downloads a pre-built binary from GitHub Releases.
# Falls back to building from source with --source.
#
# Quick install (Linux / macOS):
#   curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.sh | bash
#
# Options:
#   --prefix DIR   Install root (binary → DIR/bin). Default: ~/.local
#   --source       Build from source with cargo (requires Rust)
#   --check        Verify existing install only
#   -h, --help     Show this help

set -euo pipefail

APP_NAME="code-buddy"
REPO="simpletoolsindia/code-buddy"
DEFAULT_PREFIX="${HOME}/.local"
GITHUB_API="https://api.github.com/repos/${REPO}/releases/latest"

# ── Colours ────────────────────────────────────────────────────────────────────
if [ -t 1 ] && command -v tput &>/dev/null && tput colors &>/dev/null && [ "$(tput colors)" -ge 8 ]; then
    GREEN=$(tput setaf 2) YELLOW=$(tput setaf 3) RED=$(tput setaf 1)
    CYAN=$(tput setaf 6) BOLD=$(tput bold) RESET=$(tput sgr0)
else
    GREEN="" YELLOW="" RED="" CYAN="" BOLD="" RESET=""
fi

info()    { echo "${BOLD}${CYAN}  ✻  ${RESET}${BOLD}$*${RESET}"; }
warn()    { echo "${BOLD}${YELLOW}  ⚠  ${RESET} $*" >&2; }
success() { echo "${BOLD}${GREEN}  ✔  ${RESET} $*"; }
fail()    { echo "${BOLD}${RED}  ✘  ${RESET} $*" >&2; exit 1; }

# ── Args ───────────────────────────────────────────────────────────────────────
INSTALL_PREFIX="${DEFAULT_PREFIX}"
CHECK_ONLY=false
BUILD_FROM_SOURCE=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --check)    CHECK_ONLY=true; shift ;;
        --source)   BUILD_FROM_SOURCE=true; shift ;;
        --prefix)   [[ $# -lt 2 ]] && fail "--prefix requires an argument"
                    INSTALL_PREFIX="$2"; shift 2 ;;
        --prefix=*) INSTALL_PREFIX="${1#--prefix=}"; shift ;;
        -h|--help)
            cat <<'EOF'
Usage: bash install.sh [OPTIONS]

Options:
  --prefix DIR   Install root (binary → DIR/bin). Default: ~/.local
  --source       Build from source with cargo (requires Rust toolchain)
  --check        Verify existing installation without making changes
  -h, --help     Show this help

One-line install:
  curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.sh | bash
EOF
            exit 0 ;;
        *) warn "Unknown argument: $1 (ignored)"; shift ;;
    esac
done

INSTALL_BIN_DIR="${INSTALL_PREFIX}/bin"
BINARY="${INSTALL_BIN_DIR}/${APP_NAME}"

# ── Check-only ────────────────────────────────────────────────────────────────
if "${CHECK_ONLY}"; then
    if command -v "${APP_NAME}" &>/dev/null; then
        success "${APP_NAME} found: $(command -v "${APP_NAME}")"
        "${APP_NAME}" --version || true
        exit 0
    else
        fail "${APP_NAME} not found in PATH."
    fi
fi

# ── Platform detection ─────────────────────────────────────────────────────────
detect_target() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"
    case "${os}" in
        Linux*)
            case "${arch}" in
                x86_64)          echo "x86_64-unknown-linux-musl" ;;
                aarch64|arm64)   echo "aarch64-unknown-linux-musl" ;;
                *) fail "Unsupported Linux arch '${arch}'. Use --source to build." ;;
            esac ;;
        Darwin*)
            case "${arch}" in
                x86_64) echo "x86_64-apple-darwin" ;;
                arm64)  echo "aarch64-apple-darwin" ;;
                *) fail "Unsupported macOS arch '${arch}'. Use --source to build." ;;
            esac ;;
        *) fail "Unsupported OS '${os}'. Use --source to build." ;;
    esac
}

# ── PATH hint ──────────────────────────────────────────────────────────────────
path_hint() {
    local dir="$1"
    if ! echo ":${PATH}:" | grep -q ":${dir}:"; then
        echo ""
        warn "${dir} is not in your PATH. Add it:"
        echo "  ${BOLD}echo 'export PATH="${dir}:\$PATH"' >> ~/.bashrc && source ~/.bashrc${RESET}"
        echo "  (or ~/.zshrc for zsh)"
        echo ""
    fi
}

# ── Source build ───────────────────────────────────────────────────────────────
if "${BUILD_FROM_SOURCE}"; then
    info "Building from source (requires Rust)…"
    if ! command -v cargo &>/dev/null; then
        info "Installing Rust via rustup…"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
            | sh -s -- -y --no-modify-path
        export PATH="${HOME}/.cargo/bin:${PATH}"
    fi
    if [[ -f "Cargo.toml" ]] && grep -q '"code-buddy"' Cargo.toml 2>/dev/null; then
        cargo install --path crates/cli --root "${INSTALL_PREFIX}" --locked
    else
        cargo install --git "https://github.com/${REPO}" \
            --bin code-buddy --root "${INSTALL_PREFIX}" --locked
    fi
    success "${APP_NAME} built and installed to ${BINARY}"
    path_hint "${INSTALL_BIN_DIR}"
    exit 0
fi

# ── Binary download ────────────────────────────────────────────────────────────
TARGET="$(detect_target)"
info "Platform: ${TARGET}"
info "Fetching latest release…"

fetch() {
    if command -v curl &>/dev/null; then
        curl -fsSL "$1"
    elif command -v wget &>/dev/null; then
        wget -qO- "$1"
    else
        fail "curl or wget is required."
    fi
}

LATEST_JSON="$(fetch "${GITHUB_API}" 2>/dev/null || true)"
TAG="$(echo "${LATEST_JSON}" | grep '"tag_name"' | head -1 \
    | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')"
[[ -z "${TAG}" ]] && fail "Could not determine latest release. Use --source to build from source."

info "Installing ${APP_NAME} ${TAG}…"

ARCHIVE="${APP_NAME}-${TAG}-${TARGET}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/${TAG}"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "${TMPDIR}"' EXIT

if command -v curl &>/dev/null; then
    curl -fsSL -o "${TMPDIR}/${ARCHIVE}" "${BASE_URL}/${ARCHIVE}" \
        || fail "Download failed. Try --source."
    curl -fsSL -o "${TMPDIR}/checksums.txt" "${BASE_URL}/checksums.txt" 2>/dev/null || true
else
    wget -qO "${TMPDIR}/${ARCHIVE}" "${BASE_URL}/${ARCHIVE}" \
        || fail "Download failed. Try --source."
    wget -qO "${TMPDIR}/checksums.txt" "${BASE_URL}/checksums.txt" 2>/dev/null || true
fi

# Verify checksum when available.
if [[ -s "${TMPDIR}/checksums.txt" ]]; then
    info "Verifying checksum…"
    (
        cd "${TMPDIR}"
        if command -v sha256sum &>/dev/null; then
            grep "${ARCHIVE}" checksums.txt | sha256sum --check --status \
                || fail "Checksum mismatch — download may be corrupt."
        elif command -v shasum &>/dev/null; then
            grep "${ARCHIVE}" checksums.txt | shasum -a 256 --check --status \
                || fail "Checksum mismatch — download may be corrupt."
        else
            warn "sha256sum/shasum not found — skipping checksum verification."
        fi
    )
    success "Checksum OK."
fi

mkdir -p "${INSTALL_BIN_DIR}"
tar -xzf "${TMPDIR}/${ARCHIVE}" -C "${TMPDIR}"
install -m 755 "${TMPDIR}/${APP_NAME}" "${BINARY}"
success "${APP_NAME} ${TAG} → ${BINARY}"

path_hint "${INSTALL_BIN_DIR}"
echo ""
info "Run ${BOLD}${APP_NAME}${RESET} to start! The setup wizard will guide you on first launch."
echo ""
