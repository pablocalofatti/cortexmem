#!/bin/sh
# cortexmem installer
# Usage: curl -fsSL https://raw.githubusercontent.com/pablocalofatti/cortexmem/main/scripts/install.sh | sh
#
# Environment variables:
#   CORTEXMEM_INSTALL_DIR  — override install directory (default: ~/.cortexmem/bin)

set -e

REPO="pablocalofatti/cortexmem"
BINARY_NAME="cortexmem"
DEFAULT_INSTALL_DIR="${HOME}/.cortexmem/bin"
INSTALL_DIR="${CORTEXMEM_INSTALL_DIR:-${DEFAULT_INSTALL_DIR}}"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

has_command() {
    command -v "$1" >/dev/null 2>&1
}

# Colors (only when stdout is a terminal)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    RESET='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    BOLD=''
    RESET=''
fi

info() {
    printf "${BLUE}info${RESET}: %s\n" "$1"
}

warn() {
    printf "${YELLOW}warn${RESET}: %s\n" "$1" >&2
}

error() {
    printf "${RED}error${RESET}: %s\n" "$1" >&2
    exit 1
}

success() {
    printf "${GREEN}success${RESET}: %s\n" "$1"
}

# ---------------------------------------------------------------------------
# Platform detection
# ---------------------------------------------------------------------------

detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "${OS}" in
        Darwin)  OS="darwin" ;;
        Linux)   OS="linux" ;;
        *)       error "Unsupported operating system: ${OS}. cortexmem supports macOS and Linux." ;;
    esac

    case "${ARCH}" in
        x86_64 | amd64)    ARCH="x64" ;;
        aarch64 | arm64)   ARCH="arm64" ;;
        *)                 error "Unsupported architecture: ${ARCH}. cortexmem supports x64 and arm64." ;;
    esac

    PLATFORM="${OS}-${ARCH}"
    info "Detected platform: ${BOLD}${PLATFORM}${RESET}"
}

# ---------------------------------------------------------------------------
# HTTP client abstraction
# ---------------------------------------------------------------------------

fetch() {
    URL="$1"
    OUTPUT="$2"  # empty means stdout

    if has_command curl; then
        if [ -n "${OUTPUT}" ]; then
            curl -fsSL -o "${OUTPUT}" "${URL}"
        else
            curl -fsSL "${URL}"
        fi
    elif has_command wget; then
        if [ -n "${OUTPUT}" ]; then
            wget -qO "${OUTPUT}" "${URL}"
        else
            wget -qO - "${URL}"
        fi
    else
        error "Neither curl nor wget found. Please install one and try again."
    fi
}

# ---------------------------------------------------------------------------
# Fetch latest release tag
# ---------------------------------------------------------------------------

get_latest_version() {
    info "Fetching latest release..."
    RELEASE_JSON="$(fetch "https://api.github.com/repos/${REPO}/releases/latest" "")"

    # Parse tag_name without jq — works with grep + sed
    VERSION="$(printf '%s' "${RELEASE_JSON}" | grep '"tag_name"' | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')"

    if [ -z "${VERSION}" ]; then
        error "Could not determine latest release version. Check https://github.com/${REPO}/releases"
    fi

    info "Latest version: ${BOLD}${VERSION}${RESET}"
}

# ---------------------------------------------------------------------------
# Check existing installation
# ---------------------------------------------------------------------------

check_existing() {
    TARGET="${INSTALL_DIR}/${BINARY_NAME}"
    if [ -f "${TARGET}" ]; then
        CURRENT="$("${TARGET}" --version 2>/dev/null || echo "unknown")"
        warn "cortexmem is already installed at ${TARGET} (${CURRENT})"
        info "Upgrading to ${VERSION}..."
    fi
}

# ---------------------------------------------------------------------------
# Download and install
# ---------------------------------------------------------------------------

download_and_install() {
    TARBALL="${BINARY_NAME}-${PLATFORM}.tar.gz"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${TARBALL}"

    TMPDIR_INSTALL="$(mktemp -d)"
    trap 'rm -rf "${TMPDIR_INSTALL}"' EXIT

    info "Downloading ${BOLD}${TARBALL}${RESET}..."
    fetch "${DOWNLOAD_URL}" "${TMPDIR_INSTALL}/${TARBALL}"

    info "Extracting..."
    tar -xzf "${TMPDIR_INSTALL}/${TARBALL}" -C "${TMPDIR_INSTALL}"

    # The tarball should contain the binary (possibly at top level or in a subdir)
    EXTRACTED_BIN=""
    if [ -f "${TMPDIR_INSTALL}/${BINARY_NAME}" ]; then
        EXTRACTED_BIN="${TMPDIR_INSTALL}/${BINARY_NAME}"
    else
        # Search one level deep
        EXTRACTED_BIN="$(find "${TMPDIR_INSTALL}" -maxdepth 2 -name "${BINARY_NAME}" -type f | head -1)"
    fi

    if [ -z "${EXTRACTED_BIN}" ]; then
        error "Could not find ${BINARY_NAME} binary in the downloaded archive."
    fi

    mkdir -p "${INSTALL_DIR}"
    mv "${EXTRACTED_BIN}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    success "Installed ${BINARY_NAME} to ${INSTALL_DIR}/${BINARY_NAME}"
}

# ---------------------------------------------------------------------------
# PATH setup
# ---------------------------------------------------------------------------

add_to_path() {
    # Skip if already in PATH
    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*) return ;;
    esac

    SHELL_NAME="$(basename "${SHELL:-/bin/sh}")"
    EXPORT_LINE="export PATH=\"${INSTALL_DIR}:\$PATH\""
    ADDED=false

    add_line_if_missing() {
        RC_FILE="$1"
        if [ -f "${RC_FILE}" ]; then
            if ! grep -qF "${INSTALL_DIR}" "${RC_FILE}" 2>/dev/null; then
                printf '\n# cortexmem\n%s\n' "${EXPORT_LINE}" >> "${RC_FILE}"
                info "Added cortexmem to PATH in ${RC_FILE}"
                ADDED=true
            fi
        fi
    }

    # Only modify the rc file for the current shell
    case "${SHELL_NAME}" in
        zsh)  add_line_if_missing "${HOME}/.zshrc" ;;
        *)    add_line_if_missing "${HOME}/.bashrc" ;;
    esac

    # If the detected rc file didn't exist, create it
    if [ "${ADDED}" = false ]; then
        case "${SHELL_NAME}" in
            zsh)  RC_FILE="${HOME}/.zshrc" ;;
            *)    RC_FILE="${HOME}/.bashrc" ;;
        esac
        printf '\n# cortexmem\n%s\n' "${EXPORT_LINE}" >> "${RC_FILE}"
        info "Added cortexmem to PATH in ${RC_FILE}"
    fi

    # Make it available in this session
    export PATH="${INSTALL_DIR}:${PATH}"
}

# ---------------------------------------------------------------------------
# Verify
# ---------------------------------------------------------------------------

verify() {
    INSTALLED_VERSION="$("${INSTALL_DIR}/${BINARY_NAME}" --version 2>/dev/null || true)"
    if [ -z "${INSTALLED_VERSION}" ]; then
        warn "Could not verify installation. You may need to restart your shell."
    else
        success "${INSTALLED_VERSION} installed successfully!"
    fi
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

main() {
    printf "\n${BOLD}cortexmem installer${RESET}\n\n"

    detect_platform
    get_latest_version
    check_existing
    download_and_install
    add_to_path
    verify

    printf "\n${GREEN}${BOLD}Installation complete!${RESET}\n\n"
    printf "  Run ${BOLD}cortexmem setup${RESET} to configure your agent.\n\n"
    printf "  If the command is not found, restart your shell or run:\n"
    printf "    export PATH=\"${INSTALL_DIR}:\$PATH\"\n\n"
}

main
