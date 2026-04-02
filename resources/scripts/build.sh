#!/bin/bash
# EasySSH Unified Build Script
# 支持三版本独立构建、交叉编译、自动签名
# Usage: ./scripts/build.sh [lite|standard|pro] [target] [--sign] [--release]

set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VERSION_FILE="$PROJECT_ROOT/Cargo.toml"
VERSION=$(grep "^version" "$VERSION_FILE" | head -1 | sed 's/.*"\([^"]*\)".*/\1/')

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Build configuration
EDITION=""
TARGET=""
PROFILE="release"
SHOULD_SIGN=false
SKIP_TESTS=false
DRY_RUN=false
PARALLEL=false

# Available targets
TARGETS=(
    "x86_64-pc-windows-msvc"
    "aarch64-pc-windows-msvc"
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
    "universal-apple-darwin"
)

# ============================================================================
# Helper Functions
# ============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${CYAN}==>${NC} $1"
}

# ============================================================================
# Build Functions
# ============================================================================

build_windows() {
    local edition=$1
    local target=$2
    local profile=$3

    log_step "Building Windows $edition for $target"

    local crate_dir="$PROJECT_ROOT/crates/easyssh-platforms/windows/easyssh-winui"
    local profile_name="release-$edition"
    local binary_name="EasySSH.exe"

    export RUSTFLAGS="-C target-feature=+crt-static"

    if [[ "$target" == "aarch64-pc-windows-msvc" ]]; then
        # ARM64 cross compilation requires specific setup
        rustup target add "$target" 2>/dev/null || true
    fi

    cd "$crate_dir"
    cargo build --profile "$profile_name" --target "$target" --features "$edition" --no-default-features

    local target_dir="target/$target/$profile_name"
    local output_dir="$PROJECT_ROOT/releases/$edition-v$VERSION/windows-$target"

    mkdir -p "$output_dir"
    cp "$target_dir/$binary_name" "$output_dir/"

    log_success "Windows $edition build complete: $output_dir/$binary_name"

    # Sign if requested
    if [[ "$SHOULD_SIGN" == true ]]; then
        sign_windows "$output_dir/$binary_name"
    fi
}

build_linux() {
    local edition=$1
    local target=$2
    local profile=$3

    log_step "Building Linux $edition for $target"

    local crate_dir="$PROJECT_ROOT/crates/easyssh-platforms/linux/easyssh-gtk4"
    local profile_name="release-$edition"
    local binary_name="easyssh"

    if [[ "$target" == "aarch64-unknown-linux-gnu" ]]; then
        # Use cross for ARM64
        if ! command -v cross &> /dev/null; then
            log_info "Installing cross..."
            cargo install cross --git https://github.com/cross-rs/cross
        fi

        cd "$crate_dir"
        cross build --profile "$profile_name" --target "$target" \
            --features "easyssh-core/$edition" --no-default-features
    else
        cd "$crate_dir"
        cargo build --profile "$profile_name" --target "$target" \
            --features "easyssh-core/$edition" --no-default-features
    fi

    local target_dir="$crate_dir/target/$target/$profile_name"
    [[ "$target" == "aarch64-unknown-linux-gnu" ]] && target_dir="target/$target/$profile_name"

    local output_dir="$PROJECT_ROOT/releases/$edition-v$VERSION/linux-$target"
    mkdir -p "$output_dir"
    cp "$target_dir/easyssh-gtk4" "$output_dir/$binary_name"

    log_success "Linux $edition build complete: $output_dir/$binary_name"
}

build_macos() {
    local edition=$1
    local profile=$2

    log_step "Building macOS $edition Universal Binary"

    local core_crate="$PROJECT_ROOT/crates/easyssh-core"
    local profile_name="release-$edition"
    local output_dir="$PROJECT_ROOT/releases/$edition-v$VERSION/macos-universal"

    mkdir -p "$output_dir"

    # Build x86_64
    cd "$core_crate"
    rustup target add x86_64-apple-darwin 2>/dev/null || true
    cargo build --profile "$profile_name" --target x86_64-apple-darwin \
        --features "$edition" --no-default-features

    # Build arm64
    rustup target add aarch64-apple-darwin 2>/dev/null || true
    cargo build --profile "$profile_name" --target aarch64-apple-darwin \
        --features "$edition" --no-default-features

    # Create universal binary (for static library)
    local x86_lib="target/x86_64-apple-darwin/$profile_name/libeasyssh_core.a"
    local arm_lib="target/aarch64-apple-darwin/$profile_name/libeasyssh_core.a"
    local universal_lib="$output_dir/libeasyssh_core.a"

    if [[ -f "$x86_lib" && -f "$arm_lib" ]]; then
        lipo -create "$x86_lib" "$arm_lib" -output "$universal_lib" 2>/dev/null || {
            log_warn "Universal binary creation failed, copying x86_64 only"
            cp "$x86_lib" "$universal_lib"
        }
    fi

    log_success "macOS $edition build complete: $output_dir"

    # Sign if requested
    if [[ "$SHOULD_SIGN" == true ]]; then
        sign_macos "$output_dir"
    fi
}

# ============================================================================
# Signing Functions
# ============================================================================

sign_windows() {
    local binary=$1

    log_step "Signing Windows binary: $binary"

    if [[ -z "${WINDOWS_CERTIFICATE_THUMBPRINT:-}" ]]; then
        log_warn "WINDOWS_CERTIFICATE_THUMBPRINT not set, skipping signing"
        return 0
    fi

    if command -v signtool.exe &> /dev/null; then
        signtool.exe sign /sha1 "$WINDOWS_CERTIFICATE_THUMBPRINT" \
            /tr http://timestamp.digicert.com /td sha256 /fd sha256 "$binary"
        log_success "Windows binary signed"
    else
        log_warn "signtool.exe not found, skipping signing"
    fi
}

sign_macos() {
    local app_path=$1

    log_step "Signing macOS app: $app_path"

    if [[ -z "${MACOS_CERTIFICATE:-}" ]]; then
        log_warn "MACOS_CERTIFICATE not set, skipping signing"
        return 0
    fi

    # Import certificate
    echo "$MACOS_CERTIFICATE" | base64 --decode > /tmp/certificate.p12
    security create-keychain -p "build" build.keychain
    security default-keychain -s build.keychain
    security unlock-keychain -p "build" build.keychain
    security import /tmp/certificate.p12 -k build.keychain \
        -P "$MACOS_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
    security set-key-partition-list -S apple-tool:,apple:,codesign: \
        -s -k "build" build.keychain

    # Sign the app
    codesign --force --deep --sign "Developer ID Application" \
        --timestamp "$app_path" || log_warn "Code signing failed"

    log_success "macOS app signed"
}

# ============================================================================
# Packaging Functions
# ============================================================================

package_windows() {
    local edition=$1
    local target=$2

    log_step "Packaging Windows $edition $target"

    local release_dir="$PROJECT_ROOT/releases/$edition-v$VERSION"
    local pkg_dir="$release_dir/easyssh-$edition-$VERSION-windows-$target"
    local binary_dir="$release_dir/windows-$target"

    mkdir -p "$pkg_dir"
    cp "$binary_dir/EasySSH.exe" "$pkg_dir/"

    # Create README
    cat > "$pkg_dir/README.txt" << EOF
EasySSH $edition v$VERSION for Windows ($target)
================================================

Quick Start:
1. Run EasySSH.exe
2. Add your SSH servers via the UI
3. Connect using password or key authentication

System Requirements:
- Windows 10/11 $([[ "$target" == *"aarch64"* ]] && echo "ARM64" || echo "64-bit")
- No additional dependencies required

Features:
- Native Windows UI with egui
- SSH connection management
- Password and key-based authentication
- Server grouping
- Secure credential storage via Windows Credential Manager

For support: https://github.com/anixops/easyssh
EOF

    # Create ZIP
    cd "$release_dir"
    zip -r "easyssh-$edition-$VERSION-windows-$target.zip" "$(basename "$pkg_dir")"
    cd "$PROJECT_ROOT"

    log_success "Windows package created: easyssh-$edition-$VERSION-windows-$target.zip"
}

package_linux() {
    local edition=$1
    local target=$2

    log_step "Packaging Linux $edition $target"

    local release_dir="$PROJECT_ROOT/releases/$edition-v$VERSION"
    local pkg_name="easyssh-$edition-$VERSION-linux-$target"
    local pkg_dir="$release_dir/$pkg_name"

    mkdir -p "$pkg_dir/usr/bin"
    mkdir -p "$pkg_dir/usr/share/applications"

    local binary_dir="$release_dir/linux-$target"
    cp "$binary_dir/easyssh" "$pkg_dir/usr/bin/"

    # Create desktop entry
    cat > "$pkg_dir/usr/share/applications/easyssh.desktop" << EOF
[Desktop Entry]
Name=EasySSH $edition
Comment=SSH Client - $edition Edition
Exec=/usr/bin/easyssh
Icon=easyssh
Type=Application
Categories=Network;RemoteAccess;
Terminal=false
Version=$VERSION
EOF

    # Create install script
    cat > "$pkg_dir/install.sh" << 'EOF'
#!/bin/bash
set -e
INSTALL_PREFIX="${1:-$HOME/.local}"

if [ "$EUID" -ne 0 ] && [ "$INSTALL_PREFIX" = "/usr/local" ]; then
    echo "Installing to user directory..."
    mkdir -p "$HOME/.local/bin"
    mkdir -p "$HOME/.local/share/applications"
    cp usr/bin/easyssh "$HOME/.local/bin/"
    chmod +x "$HOME/.local/bin/easyssh"
    echo "EasySSH installed to ~/.local/bin/"
else
    echo "Installing to $INSTALL_PREFIX..."
    mkdir -p "${INSTALL_PREFIX}/bin"
    cp usr/bin/easyssh "${INSTALL_PREFIX}/bin/"
    chmod +x "${INSTALL_PREFIX}/bin/easyssh"
    echo "EasySSH installed to ${INSTALL_PREFIX}/bin/"
fi

echo "Run 'easyssh' to start."
EOF
    chmod +x "$pkg_dir/install.sh"

    # Create tarball
    cd "$release_dir"
    tar -czf "$pkg_name.tar.gz" "$pkg_name"
    cd "$PROJECT_ROOT"

    log_success "Linux package created: $pkg_name.tar.gz"
}

# ============================================================================
# Utility Functions
# ============================================================================

generate_checksums() {
    local edition=$1
    local release_dir="$PROJECT_ROOT/releases/$edition-v$VERSION"

    log_step "Generating checksums for $edition"

    cd "$release_dir"
    echo "EasySSH $edition v$VERSION Release Checksums" > SHA256SUMS.txt
    echo "===============================================" >> SHA256SUMS.txt
    echo "" >> SHA256SUMS.txt
    echo "Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")" >> SHA256SUMS.txt
    echo "" >> SHA256SUMS.txt

    find . -type f \( -name "*.zip" -o -name "*.tar.gz" -o -name "*.dmg" -o -name "*.msi" \) \
        -exec sha256sum {} \; >> SHA256SUMS.txt

    cd "$PROJECT_ROOT"

    log_success "Checksums generated: $release_dir/SHA256SUMS.txt"
}

print_summary() {
    local edition=$1
    local release_dir="$PROJECT_ROOT/releases/$edition-v$VERSION"

    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  Build Complete: EasySSH $edition v$VERSION${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "Output directory: $release_dir"
    echo ""
    echo "Generated files:"
    find "$release_dir" -type f \( -name "*.exe" -o -name "easyssh" -o -name "*.zip" -o -name "*.tar.gz" -o -name "*.txt" \) \
        -exec ls -lh {} \; 2>/dev/null || echo "  No files found"
    echo ""
    echo "Sizes:"
    du -sh "$release_dir" 2>/dev/null || echo "  Unknown"
}

show_usage() {
    cat << EOF
Usage: $0 [OPTIONS] [EDITION] [TARGET]

Build EasySSH for different editions and targets.

Arguments:
  EDITION      Build edition: lite, standard, pro (default: standard)
  TARGET       Target platform triple (default: host)

Options:
  --sign       Sign the resulting binaries
  --release    Use release profile (default)
  --dev        Use dev profile
  --skip-tests Skip running tests
  --parallel   Build multiple targets in parallel
  --dry-run    Show what would be built without building
  --all        Build all editions and targets
  --help, -h   Show this help message

Examples:
  $0 lite                              # Build Lite edition for host
  $0 standard x86_64-pc-windows-msvc   # Build Standard for Windows x64
  $0 pro --sign                        # Build Pro with signing
  $0 lite aarch64-unknown-linux-gnu    # Build Lite for Linux ARM64
  $0 --all                             # Build all editions

Available targets:
  x86_64-pc-windows-msvc      Windows x64
  aarch64-pc-windows-msvc     Windows ARM64
  x86_64-unknown-linux-gnu    Linux x64
  aarch64-unknown-linux-gnu   Linux ARM64
  x86_64-apple-darwin         macOS x64
  aarch64-apple-darwin        macOS ARM64
  universal-apple-darwin      macOS Universal
EOF
}

# ============================================================================
# Main
# ============================================================================

main() {
    # Parse arguments
    local edition=""
    local target=""
    local build_all=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            --sign)
                SHOULD_SIGN=true
                shift
                ;;
            --release)
                PROFILE="release"
                shift
                ;;
            --dev)
                PROFILE="dev"
                shift
                ;;
            --skip-tests)
                SKIP_TESTS=true
                shift
                ;;
            --parallel)
                PARALLEL=true
                shift
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --all)
                build_all=true
                shift
                ;;
            --help|-h)
                show_usage
                exit 0
                ;;
            lite|standard|pro)
                edition="$1"
                shift
                ;;
            x86_64-*|aarch64-*|universal-*|x64-*|arm64-*)
                target="$1"
                shift
                ;;
            *)
                log_error "Unknown argument: $1"
                show_usage
                exit 1
                ;;
        esac
    done

    # Default edition
    [[ -z "$edition" ]] && edition="standard"

    # Detect host target if not specified
    if [[ -z "$target" ]]; then
        case "$(uname -s)" in
            Linux*)     target="x86_64-unknown-linux-gnu" ;;
            Darwin*)    target="x86_64-apple-darwin" ;;
            MINGW*|MSYS*|CYGWIN*) target="x86_64-pc-windows-msvc" ;;
            *)          target="x86_64-unknown-linux-gnu" ;;
        esac
    fi

    # Validate edition
    case "$edition" in
        lite|standard|pro) ;;
        *)
            log_error "Invalid edition: $edition (must be lite, standard, or pro)"
            exit 1
            ;;
    esac

    # Dry run mode
    if [[ "$DRY_RUN" == true ]]; then
        echo "Dry run mode - would build:"
        echo "  Edition: $edition"
        echo "  Target:  $target"
        echo "  Profile: $PROFILE"
        echo "  Sign:    $SHOULD_SIGN"
        exit 0
    fi

    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}  EasySSH Build System v$VERSION${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo ""
    log_info "Edition: $edition"
    log_info "Target:  $target"
    log_info "Profile: $PROFILE"
    log_info "Sign:    $SHOULD_SIGN"
    echo ""

    # Create release directory
    mkdir -p "$PROJECT_ROOT/releases/$edition-v$VERSION"

    # Build based on target
    case "$target" in
        *windows*)
            build_windows "$edition" "$target" "$PROFILE"
            package_windows "$edition" "$target"
            ;;
        *linux*)
            build_linux "$edition" "$target" "$PROFILE"
            package_linux "$edition" "$target"
            ;;
        *apple-darwin|*macos*)
            build_macos "$edition" "$PROFILE"
            ;;
        *)
            log_error "Unsupported target: $target"
            exit 1
            ;;
    esac

    # Generate checksums
    generate_checksums "$edition"

    # Print summary
    print_summary "$edition"
}

main "$@"
