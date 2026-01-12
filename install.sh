#!/usr/bin/env bash
set -euo pipefail

# ctx installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/konacodes/ctx/main/install.sh | bash

REPO="konacodes/ctx"
REPO_URL="https://github.com/${REPO}.git"
BINARY_NAME="ctx"
INSTALL_DIR="${CTX_INSTALL_DIR:-$HOME/.local/bin}"
SKILLS_DIR="$HOME/.claude/skills"

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

# Check if a command exists
has_command() {
    command -v "$1" &> /dev/null
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
    echo "  ┌───────────────────────────────────────┐"
    echo "  │  ctx - Context tool for coding agents │"
    echo "  └───────────────────────────────────────┘"
    echo ""

    # Check for required tools
    if ! has_command cargo; then
        error "Cargo not found. Please install Rust first: https://rustup.rs"
    fi

    # Create temp directory for cloning
    local tmp_dir
    tmp_dir=$(mktemp -d)

    # Cleanup function
    cleanup() {
        info "Cleaning up temporary files..."
        rm -rf "$tmp_dir"
    }
    trap cleanup EXIT

    # Clone the repository
    info "Cloning repository..."
    if has_command git; then
        git clone --depth 1 "$REPO_URL" "$tmp_dir/ctx" || error "Failed to clone repository"
    else
        # Download as tarball if git is not available
        info "Git not found, downloading tarball..."
        download "https://github.com/${REPO}/archive/refs/heads/main.tar.gz" "$tmp_dir/ctx.tar.gz"
        tar -xzf "$tmp_dir/ctx.tar.gz" -C "$tmp_dir"
        mv "$tmp_dir/ctx-main" "$tmp_dir/ctx"
    fi

    # Build the project
    info "Building ctx (this may take a minute)..."
    cd "$tmp_dir/ctx"
    cargo build --release || error "Build failed"

    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"

    # Copy binary to install directory
    info "Installing binary to $INSTALL_DIR..."
    cp "target/release/$BINARY_NAME" "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
    success "Installed $BINARY_NAME to $INSTALL_DIR"

    # Install skills for Claude Code
    info "Installing skills for Claude Code..."
    mkdir -p "$SKILLS_DIR"
    if [ -d ".claude/skills/ctx" ]; then
        cp -r ".claude/skills/ctx" "$SKILLS_DIR/"
        success "Installed skills to $SKILLS_DIR/ctx"
    elif [ -d ".skills/ctx" ]; then
        cp -r ".skills/ctx" "$SKILLS_DIR/"
        success "Installed skills to $SKILLS_DIR/ctx"
    else
        warn "Skills directory not found in repository, skipping skills installation"
    fi

    # Return to original directory before cleanup
    cd - > /dev/null

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

        if [ -d "$SKILLS_DIR/ctx" ]; then
            info "Claude Code skills installed at: $SKILLS_DIR/ctx"
            echo ""
        fi
    else
        error "Installation failed - binary not found at $INSTALL_DIR/$BINARY_NAME"
    fi
}

main "$@"
