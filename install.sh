#!/usr/bin/env bash
set -euo pipefail

# ctx installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/konacodes/ctx/main/install.sh | bash

REPO="konacodes/ctx"
BINARY_NAME="ctx"
INSTALL_DIR="${CTX_INSTALL_DIR:-$HOME/.local/bin}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() {
    echo -e "${BLUE}[info]${NC} $1"
}

success() {
    echo -e "${GREEN}[success]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[warn]${NC} $1"
}

error() {
    echo -e "${RED}[error]${NC} $1"
    exit 1
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "darwin" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *) error "Unsupported operating system: $(uname -s)" ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64) echo "x86_64" ;;
        arm64|aarch64) echo "aarch64" ;;
        armv7l) echo "armv7" ;;
        *) error "Unsupported architecture: $(uname -m)" ;;
    esac
}

# Check if a command exists
has_command() {
    command -v "$1" &> /dev/null
}

# Get the latest release tag from GitHub (with timeout)
get_latest_release() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    local result=""

    if has_command curl; then
        result=$(curl -sfL --max-time 10 "$url" 2>/dev/null | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || echo "")
    elif has_command wget; then
        result=$(wget -qO- --timeout=10 "$url" 2>/dev/null | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || echo "")
    fi

    echo "$result"
}

# Download file with timeout
download() {
    local url="$1"
    local output="$2"

    if has_command curl; then
        curl -fsSL --max-time 60 "$url" -o "$output"
    elif has_command wget; then
        wget -q --timeout=60 "$url" -O "$output"
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Build from source using cargo
build_from_source() {
    info "Building from source..."

    if ! has_command cargo; then
        error "Cargo not found. Please install Rust: https://rustup.rs"
    fi

    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT

    info "Cloning repository..."
    if has_command git; then
        git clone --depth 1 "https://github.com/${REPO}.git" "$tmp_dir/ctx" || error "Failed to clone repository"
    else
        # Download as tarball if git is not available
        download "https://github.com/${REPO}/archive/refs/heads/main.tar.gz" "$tmp_dir/ctx.tar.gz"
        tar -xzf "$tmp_dir/ctx.tar.gz" -C "$tmp_dir"
        mv "$tmp_dir/ctx-main" "$tmp_dir/ctx"
    fi

    cd "$tmp_dir/ctx"

    info "Compiling (this may take a minute)..."
    cargo build --release || error "Build failed"

    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"

    # Copy binary
    cp "target/release/$BINARY_NAME" "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    success "Built and installed $BINARY_NAME to $INSTALL_DIR"
}

# Try to download prebuilt binary
try_download_binary() {
    local os="$1"
    local arch="$2"
    local version="$3"

    if [ -z "$version" ]; then
        return 1
    fi

    local target=""
    case "$os-$arch" in
        linux-x86_64)   target="x86_64-unknown-linux-gnu" ;;
        linux-aarch64)  target="aarch64-unknown-linux-gnu" ;;
        darwin-x86_64)  target="x86_64-apple-darwin" ;;
        darwin-aarch64) target="aarch64-apple-darwin" ;;
        windows-x86_64) target="x86_64-pc-windows-msvc" ;;
        *) return 1 ;;
    esac

    local ext=""
    [ "$os" = "windows" ] && ext=".exe"

    local artifact_name="${BINARY_NAME}-${target}${ext}"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${artifact_name}"

    info "Downloading $BINARY_NAME $version for $os/$arch..."

    local tmp_file
    tmp_file=$(mktemp)
    trap "rm -f $tmp_file" RETURN

    if curl -fsSL --max-time 120 "$download_url" -o "$tmp_file" 2>/dev/null; then
        mkdir -p "$INSTALL_DIR"
        mv "$tmp_file" "$INSTALL_DIR/$BINARY_NAME$ext"
        chmod +x "$INSTALL_DIR/$BINARY_NAME$ext"
        return 0
    else
        rm -f "$tmp_file"
        return 1
    fi
}

# Add to PATH instructions
show_path_instructions() {
    local shell_name
    shell_name=$(basename "$SHELL")

    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo ""
        warn "$INSTALL_DIR is not in your PATH"
        echo ""
        echo "Add it to your PATH by running:"
        echo ""

        case "$shell_name" in
            zsh)
                echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.zshrc"
                echo "  source ~/.zshrc"
                ;;
            bash)
                if [ -f "$HOME/.bashrc" ]; then
                    echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bashrc"
                    echo "  source ~/.bashrc"
                else
                    echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bash_profile"
                    echo "  source ~/.bash_profile"
                fi
                ;;
            fish)
                echo "  fish_add_path $INSTALL_DIR"
                ;;
            *)
                echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
                ;;
        esac
        echo ""
    fi
}

main() {
    echo ""
    echo "  ┌─────────────────────────────────────┐"
    echo "  │  ctx - Context tool for coding agents │"
    echo "  └─────────────────────────────────────┘"
    echo ""

    local os arch version
    os=$(detect_os)
    arch=$(detect_arch)

    info "Detected platform: $os/$arch"
    info "Install directory: $INSTALL_DIR"

    # Try to get the latest release
    info "Checking for latest release..."
    version=$(get_latest_release)

    # Try to download prebuilt binary first
    if [ -n "$version" ]; then
        info "Found release: $version"
        if try_download_binary "$os" "$arch" "$version"; then
            success "Installed $BINARY_NAME $version to $INSTALL_DIR"
        else
            warn "No prebuilt binary available for $os/$arch, building from source..."
            build_from_source
        fi
    else
        warn "No releases found, building from source..."
        build_from_source
    fi

    # Verify installation
    if [ -x "$INSTALL_DIR/$BINARY_NAME" ]; then
        echo ""
        success "Installation complete!"
        echo ""

        # Show version
        local ver_output
        ver_output=$("$INSTALL_DIR/$BINARY_NAME" --version 2>/dev/null || echo "unknown")
        info "Version: $ver_output"

        show_path_instructions

        echo "Get started:"
        echo "  ctx --help      # Show available commands"
        echo "  ctx init        # Initialize in your project"
        echo "  ctx status      # Show project overview"
        echo ""
    else
        error "Installation failed - binary not found at $INSTALL_DIR/$BINARY_NAME"
    fi
}

main "$@"
