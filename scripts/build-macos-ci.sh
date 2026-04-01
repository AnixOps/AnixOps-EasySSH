#!/bin/bash
# EasySSH macOS CI/CD Build Script
# This script is designed to run in CI environments (GitHub Actions)
# Supports both local development builds and production signed builds

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
VERSION="${VERSION:-0.3.0}"
BUILD_TYPE="${BUILD_TYPE:-release}"
MACOS_DIR="${PROJECT_ROOT}/platforms/macos"
EASYSSH_SWIFTUI_DIR="${MACOS_DIR}/easyssh-swiftui"
EASYSSH_PACKAGE_DIR="${MACOS_DIR}/EasySSH"

# Signing configuration (from environment variables)
CODESIGN_IDENTITY="${CODESIGN_IDENTITY:--}"  # "-" for ad-hoc, or Developer ID
APPLE_ID="${APPLE_ID:-}"
APPLE_TEAM_ID="${APPLE_TEAM_ID:-}"
APPLE_APP_SPECIFIC_PASSWORD="${APPLE_APP_SPECIFIC_PASSWORD:-}"
NOTARIZE="${NOTARIZE:-false}"

# Build directories
BUILD_DIR="${PROJECT_ROOT}/releases/v${VERSION}/macos"
UNIVERSAL_DIR="${BUILD_DIR}/EasySSH-${VERSION}-macos-universal"
APP_BUNDLE="${UNIVERSAL_DIR}/EasySSH.app"
DMG_PATH="${BUILD_DIR}/EasySSH-${VERSION}-macos-universal.dmg"

# Logging functions
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

# Error handler
error_handler() {
    log_error "Build failed at line $1"
    exit 1
}
trap 'error_handler $LINENO' ERR

# Clean previous builds
clean_builds() {
    log_info "Cleaning previous builds..."
    rm -rf "${BUILD_DIR}"
    mkdir -p "${BUILD_DIR}"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check for Xcode
    if ! command -v xcodebuild &> /dev/null; then
        log_error "Xcode not found. Please install Xcode from the App Store."
        exit 1
    fi

    # Check for Swift
    if ! command -v swift &> /dev/null; then
        log_error "Swift not found. Please install Xcode Command Line Tools."
        exit 1
    fi

    # Check for Rust/Cargo
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo not found. Please install Rust."
        exit 1
    fi

    # Check for required tools
    for tool in hdiutil codesign spctl; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "Required tool '$tool' not found."
            exit 1
        fi
    done

    # Check signing credentials if notarization is requested
    if [[ "$NOTARIZE" == "true" ]]; then
        if [[ -z "$APPLE_ID" || -z "$APPLE_TEAM_ID" || -z "$APPLE_APP_SPECIFIC_PASSWORD" ]]; then
            log_error "Notarization requested but missing credentials."
            log_error "Please set: APPLE_ID, APPLE_TEAM_ID, APPLE_APP_SPECIFIC_PASSWORD"
            exit 1
        fi
        log_info "Notarization enabled for Apple ID: $APPLE_ID"
    fi

    log_success "All prerequisites satisfied"
}

# Build Rust core library for multiple architectures
build_rust_core() {
    log_info "Building Rust core library..."

    cd "${PROJECT_ROOT}/core"

    # Build for x86_64 (Intel)
    log_info "Building for x86_64..."
    rustup target add x86_64-apple-darwin 2>/dev/null || true
    cargo build --release --target x86_64-apple-darwin

    # Build for aarch64 (Apple Silicon)
    log_info "Building for aarch64 (Apple Silicon)..."
    rustup target add aarch64-apple-darwin 2>/dev/null || true
    cargo build --release --target aarch64-apple-darwin

    # Create universal binary
    log_info "Creating universal binary..."
    mkdir -p "${PROJECT_ROOT}/platforms/macos/EasySSH/libs"
    lipo -create \
        "${PROJECT_ROOT}/target/x86_64-apple-darwin/release/libeasyssh_core.a" \
        "${PROJECT_ROOT}/target/aarch64-apple-darwin/release/libeasyssh_core.a" \
        -output "${PROJECT_ROOT}/platforms/macos/EasySSH/libs/libeasyssh_core.a"

    log_success "Universal Rust core built"
}

# Build Swift Package Manager project
build_swift_package() {
    log_info "Building Swift Package..."

    cd "$EASYSSH_PACKAGE_DIR"

    # Build for both architectures
    swift build -c release --arch x86_64
    swift build -c release --arch arm64

    # Create universal binary
    mkdir -p .build/universal/release
    lipo -create \
        .build/release/EasySSH \
        -output .build/universal/release/EasySSH

    log_success "Swift package built"
}

# Build SwiftUI project (Xcode)
build_swiftui() {
    log_info "Building SwiftUI project..."

    cd "$EASYSSH_SWIFTUI_DIR"

    # Build for both architectures
    xcodebuild -scheme EasySSH \
        -destination 'generic/platform=macOS' \
        -configuration Release \
        -arch x86_64 \
        -arch arm64 \
        BUILD_DIR="${PWD}/build" \
        clean build

    log_success "SwiftUI project built"
}

# Create app bundle structure
create_app_bundle() {
    log_info "Creating app bundle..."

    mkdir -p "${APP_BUNDLE}/Contents/MacOS"
    mkdir -p "${APP_BUNDLE}/Contents/Resources"
    mkdir -p "${APP_BUNDLE}/Contents/Frameworks"

    # Copy binary
    cp "${EASYSSH_PACKAGE_DIR}/.build/universal/release/EasySSH" \
        "${APP_BUNDLE}/Contents/MacOS/"

    # Create Info.plist
    cat > "${APP_BUNDLE}/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>EasySSH</string>
    <key>CFBundleIdentifier</key>
    <string>com.anixops.easyssh</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>EasySSH</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>VERSION_PLACEHOLDER</string>
    <key>CFBundleVersion</key>
    <string>BUILD_NUMBER_PLACEHOLDER</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2024 AnixOps. All rights reserved.</string>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSUIElement</key>
    <false/>
</dict>
</plist>
EOF

    # Replace placeholders
    sed -i '' "s/VERSION_PLACEHOLDER/${VERSION}/g" "${APP_BUNDLE}/Contents/Info.plist"
    sed -i '' "s/BUILD_NUMBER_PLACEHOLDER/$(date +%Y%m%d)/g" "${APP_BUNDLE}/Contents/Info.plist"

    # Create PkgInfo
    echo "APPL????" > "${APP_BUNDLE}/Contents/PkgInfo"

    log_success "App bundle created"
}

# Sign the application
code_sign_app() {
    log_info "Code signing application..."

    if [[ "$CODESIGN_IDENTITY" == "-" ]]; then
        log_warn "Using ad-hoc signing (no Developer ID)"
    else
        log_info "Signing with identity: $CODESIGN_IDENTITY"
    fi

    # Sign the binary
    codesign --force --options runtime \
        --sign "$CODESIGN_IDENTITY" \
        --timestamp \
        "${APP_BUNDLE}/Contents/MacOS/EasySSH"

    # Deep sign the entire app bundle
    codesign --force --deep --options runtime \
        --sign "$CODESIGN_IDENTITY" \
        --timestamp \
        "${APP_BUNDLE}"

    # Verify signature
    codesign --verify --verbose "${APP_BUNDLE}"
    spctl --assess --type exec --verbose "${APP_BUNDLE}" || true

    log_success "Application signed"
}

# Notarize the application
notarize_app() {
    if [[ "$NOTARIZE" != "true" ]]; then
        log_info "Skipping notarization"
        return 0
    fi

    log_info "Starting notarization..."

    # Create a temporary zip for notarization
    local NOTARIZE_ZIP="${BUILD_DIR}/notarize.zip"
    ditto -c -k --keepParent "${APP_BUNDLE}" "$NOTARIZE_ZIP"

    # Submit for notarization
    log_info "Submitting to Apple for notarization..."
    local RESPONSE
    RESPONSE=$(xcrun notarytool submit "$NOTARIZE_ZIP" \
        --apple-id "$APPLE_ID" \
        --team-id "$APPLE_TEAM_ID" \
        --password "$APPLE_APP_SPECIFIC_PASSWORD" \
        --wait 2>&1)

    echo "$RESPONSE"

    # Check if submission was accepted
    if echo "$RESPONSE" | grep -q "Accepted"; then
        log_success "Notarization accepted"

        # Staple the ticket
        xcrun stapler staple "${APP_BUNDLE}"
        log_success "Ticket stapled to app"
    else
        log_error "Notarization failed"
        exit 1
    fi

    # Cleanup
    rm -f "$NOTARIZE_ZIP"
}

# Create DMG installer
create_dmg() {
    log_info "Creating DMG installer..."

    local DMG_TEMP="${BUILD_DIR}/temp_dmg"
    rm -rf "$DMG_TEMP"
    mkdir -p "$DMG_TEMP"

    # Copy app bundle
    cp -R "${APP_BUNDLE}" "$DMG_TEMP/"

    # Create Applications symlink
    ln -s /Applications "$DMG_TEMP/Applications"

    # Copy background image if exists
    if [[ -f "${PROJECT_ROOT}/assets/dmg-background.png" ]]; then
        mkdir -p "$DMG_TEMP/.background"
        cp "${PROJECT_ROOT}/assets/dmg-background.png" "$DMG_TEMP/.background/"
    fi

    # Create DMG
    hdiutil create \
        -volname "EasySSH Installer" \
        -srcfolder "$DMG_TEMP" \
        -ov \
        -format UDZO \
        -fs HFS+ \
        "$DMG_PATH"

    # Sign DMG if not using ad-hoc
    if [[ "$CODESIGN_IDENTITY" != "-" ]]; then
        codesign --sign "$CODESIGN_IDENTITY" --timestamp "$DMG_PATH"
    fi

    # Cleanup
    rm -rf "$DMG_TEMP"

    log_success "DMG created: $DMG_PATH"
}

# Verify the build
verify_build() {
    log_info "Verifying build..."

    # Check app bundle structure
    if [[ ! -f "${APP_BUNDLE}/Contents/MacOS/EasySSH" ]]; then
        log_error "Binary not found in app bundle"
        exit 1
    fi

    # Check Info.plist
    if [[ ! -f "${APP_BUNDLE}/Contents/Info.plist" ]]; then
        log_error "Info.plist not found in app bundle"
        exit 1
    fi

    # Verify code signature
    codesign --verify --verbose "${APP_BUNDLE}" || true

    # Check architecture support
    local ARCHS
    ARCHS=$(lipo -archs "${APP_BUNDLE}/Contents/MacOS/EasySSH" 2>&1)
    log_info "Supported architectures: $ARCHS"

    # Verify DMG
    if [[ -f "$DMG_PATH" ]]; then
        local DMG_SIZE
        DMG_SIZE=$(du -h "$DMG_PATH" | cut -f1)
        log_info "DMG size: $DMG_SIZE"
    fi

    log_success "Build verification complete"
}

# Generate build info
generate_build_info() {
    log_info "Generating build info..."

    cat > "${BUILD_DIR}/build-info.txt" << EOF
EasySSH macOS Build Information
================================
Version: ${VERSION}
Build Date: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
Build Type: ${BUILD_TYPE}
Xcode Version: $(xcodebuild -version | head -1)
Swift Version: $(swift --version | head -1)
Rust Version: $(rustc --version)
Commit: $(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
Branch: $(git branch --show-current 2>/dev/null || echo "unknown")
Signing Identity: ${CODESIGN_IDENTITY}
Notarized: ${NOTARIZE}
Architectures: x86_64, arm64
Minimum macOS Version: 13.0
EOF

    log_success "Build info generated"
}

# Main build function
main() {
    echo "========================================"
    echo "EasySSH macOS CI/CD Build Script"
    echo "Version: $VERSION"
    echo "========================================"

    # Clean previous builds
    clean_builds

    # Check prerequisites
    check_prerequisites

    # Build components
    build_rust_core
    build_swift_package
    # build_swiftui  # Uncomment when SwiftUI project is ready

    # Create and sign app bundle
    create_app_bundle
    code_sign_app
    notarize_app
    create_dmg

    # Verification and info
    verify_build
    generate_build_info

    echo ""
    echo "========================================"
    log_success "Build complete!"
    echo "Output: $DMG_PATH"
    echo "========================================"
}

# Parse command line arguments
case "${1:-}" in
    --clean)
        clean_builds
        exit 0
        ;;
    --verify)
        verify_build
        exit 0
        ;;
    --help|-h)
        echo "Usage: $0 [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  --clean     Clean previous builds"
        echo "  --verify    Verify existing build"
        echo "  --help      Show this help"
        echo ""
        echo "Environment Variables:"
        echo "  VERSION                      App version (default: 0.3.0)"
        echo "  CODESIGN_IDENTITY            Signing identity (default: ad-hoc)"
        echo "  APPLE_ID                     Apple ID for notarization"
        echo "  APPLE_TEAM_ID                Apple Team ID"
        echo "  APPLE_APP_SPECIFIC_PASSWORD  App-specific password"
        echo "  NOTARIZE                     Enable notarization (true/false)"
        exit 0
        ;;
    *)
        main
        ;;
esac
