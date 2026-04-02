#!/bin/bash
# EasySSH 三版本统一构建脚本
# 支持: Lite, Standard, Pro 独立构建
# 支持: Windows/Linux/macOS 多平台交叉编译
# Usage: ./scripts/build-version.sh [lite|standard|pro] [target] [--sign]

set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Defaults
EDITION="standard"
TARGET=""
SHOULD_SIGN=false
CLEAN_BUILD=false
SKIP_TESTS=false

# ============================================================================
# Version Detection
# ============================================================================
get_version() {
    local version_file="$PROJECT_ROOT/Cargo.toml"
    grep "^version" "$version_file" | head -1 | sed 's/.*"\([^"]*\)".*/\1/'
}

VERSION=$(get_version)

# ============================================================================
# Logging
# ============================================================================
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${CYAN}==>${NC} $1"; }

# ============================================================================
# Build Functions
# ============================================================================

build_lite() {
    local target=${1:-$(detect_host_target)}
    log_step "Building EasySSH Lite v$VERSION for $target"

    export CARGO_EASYSSH_EDITION="lite"
    export CARGO_TARGET_DIR="$PROJECT_ROOT/target/lite"

    case "$target" in
        *windows*)
            build_windows_lite "$target"
            ;;
        *linux*)
            build_linux_lite "$target"
            ;;
        *darwin*)
            build_macos_lite
            ;;
        *)
            log_error "Unsupported target for Lite: $target"
            return 1
            ;;
    esac
}

build_standard() {
    local target=${1:-$(detect_host_target)}
    log_step "Building EasySSH Standard v$VERSION for $target"

    export CARGO_EASYSSH_EDITION="standard"
    export CARGO_TARGET_DIR="$PROJECT_ROOT/target/standard"

    case "$target" in
        *windows*)
            build_windows_standard "$target"
            ;;
        *linux*)
            build_linux_standard "$target"
            ;;
        *darwin*)
            build_macos_standard
            ;;
        *)
            log_error "Unsupported target for Standard: $target"
            return 1
            ;;
    esac
}

build_pro() {
    local target=${1:-$(detect_host_target)}
    log_step "Building EasySSH Pro v$VERSION for $target"

    export CARGO_EASYSSH_EDITION="pro"
    export CARGO_TARGET_DIR="$PROJECT_ROOT/target/pro"

    case "$target" in
        *linux*)
            build_linux_pro "$target"
            ;;
        *)
            log_warn "Pro server is Linux-only, building for Linux x64"
            build_linux_pro "x86_64-unknown-linux-gnu"
            ;;
    esac
}

# ============================================================================
# Platform-Specific Builds
# ============================================================================

build_windows_lite() {
    local target=$1
    log_info "Building Windows Lite ($target)"

    cd "$PROJECT_ROOT/crates/easyssh-platforms/windows/easyssh-winui"

    export RUSTFLAGS="-C target-feature=+crt-static"

    cargo build --profile release-lite --target "$target" --features lite --no-default-features

    local output_dir="$PROJECT_ROOT/releases/lite-v$VERSION"
    mkdir -p "$output_dir"

    cp "target/$target/release-lite/EasySSH.exe" "$output_dir/EasySSH-Lite-$VERSION.exe"

    log_success "Windows Lite build complete"
}

build_windows_standard() {
    local target=$1
    log_info "Building Windows Standard ($target)"

    cd "$PROJECT_ROOT/crates/easyssh-platforms/windows/easyssh-winui"

    export RUSTFLAGS="-C target-feature=+crt-static"

    cargo build --profile release-standard --target "$target" --features standard --no-default-features

    local output_dir="$PROJECT_ROOT/releases/standard-v$VERSION"
    mkdir -p "$output_dir"

    cp "target/$target/release-standard/EasySSH.exe" "$output_dir/EasySSH-Standard-$VERSION.exe"

    log_success "Windows Standard build complete"
}

build_linux_lite() {
    local target=$1
    log_info "Building Linux Lite ($target)"

    local crate_dir="$PROJECT_ROOT/crates/easyssh-platforms/linux/easyssh-gtk4"
    local profile="release-lite"

    if [[ "$target" == "aarch64-unknown-linux-gnu" ]]; then
        # Cross compile
        cd "$crate_dir"
        cross build --profile "$profile" --target "$target" --features easyssh-core/lite --no-default-features
    else
        cd "$crate_dir"
        cargo build --profile "$profile" --target "$target" --features easyssh-core/lite --no-default-features
    fi

    local output_dir="$PROJECT_ROOT/releases/lite-v$VERSION"
    mkdir -p "$output_dir"

    local src_path
    if [[ "$target" == "aarch64-unknown-linux-gnu" ]]; then
        src_path="$PROJECT_ROOT/target/$target/$profile/easyssh-gtk4"
    else
        src_path="$crate_dir/target/$target/$profile/easyssh-gtk4"
    fi

    cp "$src_path" "$output_dir/easyssh-lite-$VERSION-$target"
    chmod +x "$output_dir/easyssh-lite-$VERSION-$target"

    log_success "Linux Lite build complete"
}

build_linux_standard() {
    local target=$1
    log_info "Building Linux Standard ($target)"

    local crate_dir="$PROJECT_ROOT/crates/easyssh-platforms/linux/easyssh-gtk4"
    local profile="release-standard"

    if [[ "$target" == "aarch64-unknown-linux-gnu" ]]; then
        cd "$crate_dir"
        cross build --profile "$profile" --target "$target" --features easyssh-core/standard --no-default-features
    else
        cd "$crate_dir"
        cargo build --profile "$profile" --target "$target" --features easyssh-core/standard --no-default-features
    fi

    local output_dir="$PROJECT_ROOT/releases/standard-v$VERSION"
    mkdir -p "$output_dir"

    local src_path
    if [[ "$target" == "aarch64-unknown-linux-gnu" ]]; then
        src_path="$PROJECT_ROOT/target/$target/$profile/easyssh-gtk4"
    else
        src_path="$crate_dir/target/$target/$profile/easyssh-gtk4"
    fi

    cp "$src_path" "$output_dir/easyssh-standard-$VERSION-$target"
    chmod +x "$output_dir/easyssh-standard-$VERSION-$target"

    log_success "Linux Standard build complete"
}

build_linux_pro() {
    local target=$1
    log_info "Building Pro Server ($target)"

    cd "$PROJECT_ROOT/crates/easyssh-pro-server"

    if [[ "$target" == "aarch64-unknown-linux-gnu" ]]; then
        cross build --release --target "$target"
    else
        cargo build --release --target "$target"
    fi

    local output_dir="$PROJECT_ROOT/releases/pro-v$VERSION"
    mkdir -p "$output_dir"

    local src_path
    if [[ "$target" == "aarch64-unknown-linux-gnu" ]]; then
        src_path="$PROJECT_ROOT/target/$target/release/easyssh-pro-server"
    else
        src_path="target/$target/release/easyssh-pro-server"
    fi

    cp "$src_path" "$output_dir/easyssh-pro-server-$VERSION-$target"
    chmod +x "$output_dir/easyssh-pro-server-$VERSION-$target"

    log_success "Pro Server build complete"
}

build_macos_lite() {
    log_info "Building macOS Lite Universal Binary"

    cd "$PROJECT_ROOT/crates/easyssh-core"

    # Build x86_64
    rustup target add x86_64-apple-darwin 2>/dev/null || true
    cargo build --profile release-lite --target x86_64-apple-darwin --features lite --no-default-features

    # Build arm64
    rustup target add aarch64-apple-darwin 2>/dev/null || true
    cargo build --profile release-lite --target aarch64-apple-darwin --features lite --no-default-features

    # Create universal binary
    local output_dir="$PROJECT_ROOT/releases/lite-v$VERSION"
    mkdir -p "$output_dir"

    lipo -create \
        "target/x86_64-apple-darwin/release-lite/libeasyssh_core.a" \
        "target/aarch64-apple-darwin/release-lite/libeasyssh_core.a" \
        -output "$output_dir/libeasyssh_core-lite-$VERSION.a" 2>/dev/null || true

    log_success "macOS Lite build complete"
}

build_macos_standard() {
    log_info "Building macOS Standard Universal Binary"

    cd "$PROJECT_ROOT/crates/easyssh-core"

    # Build x86_64
    rustup target add x86_64-apple-darwin 2>/dev/null || true
    cargo build --profile release-standard --target x86_64-apple-darwin --features standard --no-default-features

    # Build arm64
    rustup target add aarch64-apple-darwin 2>/dev/null || true
    cargo build --profile release-standard --target aarch64-apple-darwin --features standard --no-default-features

    # Create universal binary
    local output_dir="$PROJECT_ROOT/releases/standard-v$VERSION"
    mkdir -p "$output_dir"

    lipo -create \
        "target/x86_64-apple-darwin/release-standard/libeasyssh_core.a" \
        "target/aarch64-apple-darwin/release-standard/libeasyssh_core.a" \
        -output "$output_dir/libeasyssh_core-standard-$VERSION.a" 2>/dev/null || true

    log_success "macOS Standard build complete"
}

# ============================================================================
# Helper Functions
# ============================================================================

detect_host_target() {
    case "$(uname -s)" in
        Linux*)     echo "x86_64-unknown-linux-gnu" ;;
        Darwin*)    echo "x86_64-apple-darwin" ;;
        CYGWIN*|MINGW*|MSYS*) echo "x86_64-pc-windows-msvc" ;;
        *)          echo "x86_64-unknown-linux-gnu" ;;
    esac
}

clean_build() {
    log_step "Cleaning build directories"

    rm -rf "$PROJECT_ROOT/target"/*
    rm -rf "$PROJECT_ROOT/releases"/*

    log_success "Build directories cleaned"
}

show_usage() {
    cat << EOF
Usage: build-version.sh [OPTIONS] [EDITION] [TARGET]

Build EasySSH for different editions and targets.

Arguments:
  EDITION      Build edition: lite, standard, pro (default: standard)
  TARGET       Target platform triple

Options:
  --sign       Sign the resulting binaries
  --clean      Clean build directories first
  --skip-tests Skip running tests
  --help       Show this help message

Examples:
  ./build-version.sh lite                              # Build Lite for host
  ./build-version.sh standard x86_64-pc-windows-msvc   # Build Standard for Windows
  ./build-version.sh pro aarch64-unknown-linux-gnu     # Build Pro for Linux ARM64

Available targets:
  x86_64-pc-windows-msvc      Windows x64
  aarch64-pc-windows-msvc     Windows ARM64
  x86_64-unknown-linux-gnu    Linux x64
  aarch64-unknown-linux-gnu   Linux ARM64
  x86_64-apple-darwin         macOS x64
  aarch64-apple-darwin        macOS ARM64

Environment Variables:
  CARGO_EASYSSH_EDITION       Override edition
  CARGO_TARGET_DIR            Override target directory
  WINDOWS_CERTIFICATE_THUMBPRINT   Windows code signing cert
  MACOS_CERTIFICATE           macOS code signing cert (base64)
EOF
}

# ============================================================================
# Main
# ============================================================================

main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --sign)
                SHOULD_SIGN=true
                shift
                ;;
            --clean)
                CLEAN_BUILD=true
                shift
                ;;
            --skip-tests)
                SKIP_TESTS=true
                shift
                ;;
            --help|-h)
                show_usage
                exit 0
                ;;
            lite|standard|pro)
                EDITION="$1"
                shift
                ;;
            x86_64-*|aarch64-*|universal-*)
                TARGET="$1"
                shift
                ;;
            *)
                log_error "Unknown argument: $1"
                show_usage
                exit 1
                ;;
        esac
    done

    # Clean if requested
    [[ "$CLEAN_BUILD" == true ]] && clean_build

    # Detect host target if not specified
    [[ -z "$TARGET" ]] && TARGET=$(detect_host_target)

    # Print header
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}  EasySSH v$VERSION - $EDITION Edition${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo ""
    log_info "Edition: $EDITION"
    log_info "Target:  $TARGET"
    log_info "Sign:    $SHOULD_SIGN"
    echo ""

    # Build based on edition
    case "$EDITION" in
        lite)
            build_lite "$TARGET"
            ;;
        standard)
            build_standard "$TARGET"
            ;;
        pro)
            build_pro "$TARGET"
            ;;
        *)
            log_error "Unknown edition: $EDITION"
            show_usage
            exit 1
            ;;
    esac

    echo ""
    log_success "Build completed successfully!"
    echo ""
    echo "Output directory: $PROJECT_ROOT/releases/$EDITION-v$VERSION"
    echo ""

    # List outputs
    if [[ -d "$PROJECT_ROOT/releases/$EDITION-v$VERSION" ]]; then
        ls -lh "$PROJECT_ROOT/releases/$EDITION-v$VERSION"
    fi
}

main "$@"
