#!/bin/bash
# EasySSH Windows Release Workflow
# Automates the complete Windows release process
#
# Usage:
#   ./scripts/release-windows.sh [version]
#
# Steps:
#   1. Build release binary
#   2. Run tests
#   3. Create installers (MSI + NSIS)
#   4. Sign all binaries
#   5. Generate checksums
#   6. Create GitHub release draft

set -e

VERSION="${1:-0.3.0}"
RELEASE_DIR="releases/v${VERSION}"
INSTALLER_DIR="installer"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${BLUE}[STEP]${NC} $1"; }

# Check prerequisites
check_prerequisites() {
    log_step "Checking prerequisites..."

    # Check Rust
    if ! command -v rustc &> /dev/null; then
        log_error "Rust not found. Please install Rust."
        exit 1
    fi

    # Check for required tools
    local tools=("cargo" "zip")
    for tool in "${tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "$tool not found"
            exit 1
        fi
    done

    log_info "Prerequisites check passed"
}

# Build release binary
build_binary() {
    log_step "Building release binary..."

    cd platforms/windows/easyssh-winui

    # Build with optimizations
    RUSTFLAGS="-C target-cpu=x86-64-v3 -C opt-level=3 -C lto=fat -C codegen-units=1" \
        cargo build --release --bin EasySSH

    cd ../../..

    # Verify binary exists
    if [ ! -f "target/release/EasySSH.exe" ]; then
        log_error "Build failed: target/release/EasySSH.exe not found"
        exit 1
    fi

    log_info "Binary built successfully"
}

# Run tests
run_tests() {
    log_step "Running tests..."

    # Run core tests
    cargo test -p easyssh-core --release

    log_info "Tests passed"
}

# Create installers
create_installers() {
    log_step "Creating installers..."

    # Run installer build script
    if [ -f "scripts/build-windows-installer.sh" ]; then
        ./scripts/build-windows-installer.sh "$VERSION" "target/release"
    else
        log_warn "Installer build script not found"
    fi
}

# Sign binaries
sign_binaries() {
    log_step "Signing binaries..."

    if [ -z "$SIGN_CERT" ]; then
        log_warn "No signing certificate configured"
        log_info "Binaries will not be signed"
        return 0
    fi

    local signtool="/c/Program Files (x86)/Windows Kits/10/bin/10.0.19041.0/x64/signtool.exe"

    for binary in "${RELEASE_DIR}/windows/"*.{exe,msi}; do
        if [ -f "$binary" ]; then
            log_info "Signing: $binary"
            "$signtool" sign \
                /f "$SIGN_CERT" \
                /p "$SIGN_CERT_PASSWORD" \
                /tr "${SIGN_TIMESTAMP_URL:-http://timestamp.digicert.com}" \
                /td sha256 \
                /fd sha256 \
                /d "EasySSH v$VERSION" \
                "$binary" || log_warn "Signing failed for $binary"
        fi
    done
}

# Generate release notes
generate_release_notes() {
    log_step "Generating release notes..."

    cat > "${RELEASE_DIR}/windows/RELEASE_NOTES.md" << EOF
# EasySSH v${VERSION} for Windows

## Installation Packages

| Package | Type | Description |
|---------|------|-------------|
| EasySSH-${VERSION}-x64.msi | MSI | Windows Installer (recommended for enterprise) |
| EasySSH-${VERSION}-x64.exe | NSIS | Interactive installer (recommended for users) |
| EasySSH-${VERSION}-portable.zip | ZIP | Portable version (no installation) |

## Quick Start

### MSI (Silent Install)
\`\`\`powershell
msiexec /i EasySSH-${VERSION}-x64.msi /qn INSTALLDESKTOPSHORTCUT=1
\`\`\`

### NSIS (Interactive)
1. Run \`EasySSH-${VERSION}-x64.exe\`
2. Follow the installation wizard
3. Launch EasySSH from Start Menu

### Portable
1. Extract \`EasySSH-${VERSION}-portable.zip\`
2. Run \`EasySSH.exe\`

## System Requirements
- Windows 10 version 1607 or later
- Windows 11
- 64-bit processor
- 100 MB free disk space

## Verification

### Checksums
See \`SHA256SUMS.txt\` for file hashes.

### Code Signing
Official binaries are signed by AnixOps.
Verify with:
\`\`\`powershell
Get-AuthenticodeSignature EasySSH-${VERSION}-x64.exe
\`\`\`

## What's New in v${VERSION}

See full release notes at: https://github.com/anixops/easyssh/releases

## Support
- GitHub Issues: https://github.com/anixops/easyssh/issues
- Documentation: https://docs.anixops.com/easyssh
EOF

    log_info "Release notes generated"
}

# Create GitHub release draft (requires gh CLI)
create_github_release() {
    log_step "Creating GitHub release draft..."

    if ! command -v gh &> /dev/null; then
        log_warn "GitHub CLI (gh) not found. Skipping release creation."
        log_info "Install from: https://cli.github.com/"
        return 0
    fi

    # Check if already authenticated
    if ! gh auth status &> /dev/null; then
        log_warn "Not authenticated with GitHub"
        log_info "Run: gh auth login"
        return 0
    fi

    # Create draft release
    gh release create "v${VERSION}" \
        --draft \
        --title "EasySSH v${VERSION}" \
        --notes-file "${RELEASE_DIR}/windows/RELEASE_NOTES.md" \
        "${RELEASE_DIR}/windows/"*.{msi,exe,zip,txt,md} || {
        log_warn "Failed to create GitHub release"
        return 0
    }

    log_info "GitHub release draft created"
}

# Main release process
main() {
    echo "=========================================="
    echo "EasySSH Windows Release Process"
    echo "Version: $VERSION"
    echo "=========================================="
    echo ""

    check_prerequisites
    build_binary
    run_tests
    create_installers
    sign_binaries
    generate_release_notes
    create_github_release

    echo ""
    echo "=========================================="
    echo "Release Complete!"
    echo "=========================================="
    echo ""
    echo "Release artifacts:"
    ls -lh "${RELEASE_DIR}/windows/"
    echo ""
    echo "Next steps:"
    echo "  1. Test the installers on clean Windows VMs"
    echo "  2. Verify digital signatures"
    echo "  3. Publish the GitHub release draft"
    echo "  4. Update website download links"
}

# Show help
show_help() {
    cat << EOF
EasySSH Windows Release Script

Usage: $0 [version]

Arguments:
  version    Release version (default: 0.3.0)

Environment Variables:
  SIGN_CERT              Path to code signing certificate (.pfx)
  SIGN_CERT_PASSWORD     Certificate password
  SIGN_TIMESTAMP_URL     Timestamp server URL

Examples:
  $0              # Release version 0.3.0
  $0 0.4.0        # Release version 0.4.0

Prerequisites:
  - Rust toolchain
  - WiX Toolset (for MSI)
  - NSIS (for EXE installer)
  - GitHub CLI (optional, for release creation)
  - Code signing certificate (optional)

EOF
}

if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
    show_help
    exit 0
fi

main
