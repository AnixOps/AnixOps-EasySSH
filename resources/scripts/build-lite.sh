#!/bin/bash
# EasySSH Lite Version Build Script
# Supports Linux (x64, ARM64) and macOS (x64, Apple Silicon)
# Usage: ./build-lite.sh [version] [target]
#   version: semantic version (default: extracted from Cargo.toml)
#   target: specific target triple (default: native)

set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
WORKSPACE_ROOT="${PROJECT_ROOT}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${BLUE}[STEP]${NC} $1"; }

# ============================================================================
# Version Extraction
# ============================================================================
get_version() {
    local cargo_toml="${WORKSPACE_ROOT}/Cargo.toml"
    if [ -f "$cargo_toml" ]; then
        grep -E '^version\s*=\s*"[0-9]+\.[0-9]+\.[0-9]+"' "$cargo_toml" | head -1 | sed 's/.*"\([0-9]\+\.[0-9]\+\.[0-9]\+\)".*/\1/'
    else
        echo "0.3.0"
    fi
}

VERSION="${1:-$(get_version)}"
TARGET="${2:-native}"
RELEASE_DIR="${WORKSPACE_ROOT}/releases/lite-v${VERSION}"
BUILD_PROFILE="release-lite"

# Platform detection
 detect_platform() {
    local uname_out="$(uname -s)"
    case "$uname_out" in
        Linux*)     PLATFORM="linux";;
        Darwin*)    PLATFORM="macos";;
        *)          PLATFORM="unknown";;
    esac
    echo "$PLATFORM"
}

CURRENT_PLATFORM="$(detect_platform)"

# ============================================================================
# Dependency Checks
# ============================================================================
check_deps() {
    log_step "Checking dependencies..."

    # Check Rust
    if ! command -v rustc &>/dev/null; then
        log_error "Rust is not installed. Please install Rust: https://rustup.rs/"
        exit 1
    fi

    local rust_version=$(rustc --version | cut -d' ' -f2)
    log_info "Rust version: $rust_version"

    # Check Cargo
    if ! command -v cargo &>/dev/null; then
        log_error "Cargo is not installed"
        exit 1
    fi

    # Platform-specific dependency checks
    case "$CURRENT_PLATFORM" in
        linux)
            check_linux_deps
            ;;
        macos)
            check_macos_deps
            ;;
    esac

    # Check required tools for packaging
    if ! command -v zip &>/dev/null; then
        log_warn "zip not found, installing..."
        case "$CURRENT_PLATFORM" in
            linux)
                sudo apt-get update && sudo apt-get install -y zip
                ;;
            macos)
                brew install zip || log_warn "Could not install zip via brew"
                ;;
        esac
    fi

    log_info "All dependencies satisfied"
}

check_linux_deps() {
    log_info "Checking Linux dependencies..."

    # Check for GTK4 and libadwaita
    local missing_pkgs=()

    if ! pkg-config --exists gtk4; then
        missing_pkgs+=("libgtk-4-dev")
    fi

    if ! pkg-config --exists libadwaita-1; then
        missing_pkgs+=("libadwaita-1-dev")
    fi

    if ! pkg-config --exists openssl; then
        missing_pkgs+=("libssl-dev")
    fi

    if [ ${#missing_pkgs[@]} -ne 0 ]; then
        log_error "Missing packages: ${missing_pkgs[*]}"
        log_info "Install with: sudo apt-get install -y ${missing_pkgs[*]}"
        exit 1
    fi

    log_info "Linux dependencies OK"
}

check_macos_deps() {
    log_info "Checking macOS dependencies..."

    # Check for Xcode command line tools
    if ! xcode-select -p &>/dev/null; then
        log_error "Xcode command line tools not installed"
        log_info "Install with: xcode-select --install"
        exit 1
    fi

    # Check for Rust targets
    local targets=("x86_64-apple-darwin" "aarch64-apple-darwin")
    for target in "${targets[@]}"; do
        if ! rustup target list --installed | grep -q "$target"; then
            log_warn "Target $target not installed, adding..."
            rustup target add "$target"
        fi
    done

    log_info "macOS dependencies OK"
}

# ============================================================================
# Build Functions
# ============================================================================
build_linux() {
    local arch="${1:-x86_64}"
    local target
    local cross_compile=false

    case "$arch" in
        x86_64|x64|amd64)
            target="x86_64-unknown-linux-gnu"
            ;;
        aarch64|arm64)
            target="aarch64-unknown-linux-gnu"
            cross_compile=true
            ;;
        *)
            log_error "Unsupported architecture: $arch"
            exit 1
            ;;
    esac

    log_step "Building EasySSH Lite for Linux $arch..."

    local crate_dir="${WORKSPACE_ROOT}/crates/easyssh-platforms/linux/easyssh-gtk4"

    # Install cross-compilation tools if needed
    if [ "$cross_compile" = true ]; then
        log_info "Setting up cross compilation for ARM64..."
        if ! command -v cross &>/dev/null; then
            log_info "Installing cross tool..."
            cargo install cross --git https://github.com/cross-rs/cross
        fi
        BUILD_CMD="cross"
    else
        BUILD_CMD="cargo"
    fi

    # Set version injection flags
    local version_flags="-C link-arg=-Wl,--defsym,EASYSSH_VERSION=${VERSION}"

    # Build with Lite features
    cd "$crate_dir"
    RUSTFLAGS="${version_flags}" \
        ${BUILD_CMD} build \
        --profile ${BUILD_PROFILE} \
        --target "${target}" \
        --features "easyssh-core/lite" \
        --no-default-features

    cd "$WORKSPACE_ROOT"

    # Determine binary path
    local binary_path
    if [ "$cross_compile" = true ]; then
        binary_path="target/${target}/${BUILD_PROFILE}/easyssh-gtk4"
    else
        binary_path="target/${BUILD_PROFILE}/easyssh-gtk4"
    fi

    # Package the build
    package_linux "$arch" "$binary_path"
}

build_macos() {
    local arch="${1:-universal}"

    log_step "Building EasySSH Lite for macOS $arch..."

    local crate_dir="${WORKSPACE_ROOT}/crates/easyssh-platforms/macos"

    # Check if we have a SwiftUI project
    if [ ! -d "$crate_dir/EasySSH" ]; then
        log_warn "macOS SwiftUI project not found at $crate_dir/EasySSH"
        log_info "Building core library only..."

        # Build core library for macOS
        cd "${WORKSPACE_ROOT}/crates/easyssh-core"

        case "$arch" in
            x86_64|x64)
                cargo build --profile ${BUILD_PROFILE} --target x86_64-apple-darwin --features "lite"
                ;;
            aarch64|arm64)
                cargo build --profile ${BUILD_PROFILE} --target aarch64-apple-darwin --features "lite"
                ;;
            universal)
                cargo build --profile ${BUILD_PROFILE} --target x86_64-apple-darwin --features "lite"
                cargo build --profile ${BUILD_PROFILE} --target aarch64-apple-darwin --features "lite"
                ;;
        esac

        cd "$WORKSPACE_ROOT"
    else
        # Build SwiftUI app
        cd "$crate_dir/EasySSH"

        case "$arch" in
            x86_64|x64)
                swift build -c release --arch x86_64
                ;;
            aarch64|arm64)
                swift build -c release --arch arm64
                ;;
            universal)
                swift build -c release --arch x86_64
                swift build -c release --arch arm64
                # Create universal binary
                create_universal_macos_binary
                ;;
        esac

        cd "$WORKSPACE_ROOT"
    fi

    package_macos "$arch"
}

create_universal_macos_binary() {
    log_info "Creating universal binary..."

    local build_dir="${WORKSPACE_ROOT}/crates/easyssh-platforms/macos/EasySSH/.build/release"
    local output_dir="${RELEASE_DIR}/macos-universal"

    mkdir -p "$output_dir"

    # Lipo the binaries together
    lipo -create \
        "${build_dir}/x86_64-apple-macosx/release/EasySSH" \
        "${build_dir}/arm64-apple-macosx/release/EasySSH" \
        -output "${output_dir}/EasySSH"

    log_info "Universal binary created at ${output_dir}/EasySSH"
}

# ============================================================================
# Packaging Functions
# ============================================================================
package_linux() {
    local arch="$1"
    local binary_path="$2"

    log_step "Packaging for Linux $arch..."

    local pkg_name="easyssh-lite-v${VERSION}-linux-${arch}"
    local pkg_dir="${RELEASE_DIR}/${pkg_name}"

    # Create package structure
    mkdir -p "${pkg_dir}/usr/bin"
    mkdir -p "${pkg_dir}/usr/share/applications"
    mkdir -p "${pkg_dir}/usr/share/icons/hicolor/256x256/apps"
    mkdir -p "${pkg_dir}/usr/share/easyssh"

    # Copy binary
    cp "${binary_path}" "${pkg_dir}/usr/bin/easyssh-lite"
    chmod +x "${pkg_dir}/usr/bin/easyssh-lite"

    # Create desktop entry
    cat > "${pkg_dir}/usr/share/applications/easyssh-lite.desktop" << EOF
[Desktop Entry]
Name=EasySSH Lite
Comment=Lightweight SSH Configuration Manager
Exec=/usr/bin/easyssh-lite
Icon=easyssh-lite
Type=Application
Categories=Network;RemoteAccess;
Terminal=false
Version=${VERSION}
StartupNotify=true
EOF

    # Create AppDir structure for AppImage
    create_appimage_structure "$pkg_dir" "$arch"

    # Create install script
    cat > "${pkg_dir}/install.sh" << 'EOF'
#!/bin/bash
set -e

INSTALL_PREFIX="${1:-/usr/local}"

if [ "$EUID" -ne 0 ] && [ "$INSTALL_PREFIX" = "/usr/local" ]; then
    echo "Installing to $INSTALL_PREFIX (user)..."
    mkdir -p "$HOME/.local/bin"
    mkdir -p "$HOME/.local/share/applications"
    mkdir -p "$HOME/.local/share/icons/hicolor/256x256/apps"

    cp usr/bin/easyssh-lite "$HOME/.local/bin/"
    cp usr/share/applications/easyssh-lite.desktop "$HOME/.local/share/applications/"
    chmod +x "$HOME/.local/bin/easyssh-lite"
    echo "EasySSH Lite installed to ~/.local/bin/"
else
    echo "Installing to $INSTALL_PREFIX (system)..."
    sudo mkdir -p "${INSTALL_PREFIX}/bin"
    sudo cp usr/bin/easyssh-lite "${INSTALL_PREFIX}/bin/"
    sudo chmod +x "${INSTALL_PREFIX}/bin/easyssh-lite"

    if command -v update-desktop-database &>/dev/null; then
        sudo cp usr/share/applications/easyssh-lite.desktop /usr/share/applications/
        sudo update-desktop-database
    fi

    echo "EasySSH Lite installed to ${INSTALL_PREFIX}/bin/"
fi

echo "Run 'easyssh-lite' to start the application."
EOF
    chmod +x "${pkg_dir}/install.sh"

    # Create README
    cat > "${pkg_dir}/README.txt" << EOF
EasySSH Lite v${VERSION} for Linux
=====================================

Quick Start:
1. Run ./install.sh (user install) or sudo ./install.sh (system install)
2. Run 'easyssh-lite' to start

System Requirements:
- GTK4 runtime
- libadwaita runtime
- 64-bit Linux distribution

Features:
- Native GTK4 UI
- SSH connection management
- Password and key-based authentication
- Server grouping
- Secure credential storage

For support: https://github.com/anixops/easyssh
EOF

    # Create tarball
    cd "$RELEASE_DIR"
    tar -czf "${pkg_name}.tar.gz" "$pkg_name"
    cd "$WORKSPACE_ROOT"

    log_info "Package created: ${RELEASE_DIR}/${pkg_name}.tar.gz"
}

create_appimage_structure() {
    local pkg_dir="$1"
    local arch="$2"

    log_info "Creating AppImage structure..."

    local appdir="${pkg_dir}/AppDir"
    mkdir -p "${appdir}/usr/bin"
    mkdir -p "${appdir}/usr/share/applications"
    mkdir -p "${appdir}/usr/share/icons/hicolor/256x256/apps"

    # Copy files to AppDir
    cp "${pkg_dir}/usr/bin/easyssh-lite" "${appdir}/usr/bin/"
    cp "${pkg_dir}/usr/share/applications/easyssh-lite.desktop" "${appdir}/usr/share/applications/"

    # Create AppRun script
    cat > "${appdir}/AppRun" << 'EOF'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH}"
exec "${HERE}/usr/bin/easyssh-lite" "$@"
EOF
    chmod +x "${appdir}/AppRun"

    # Create .desktop file in AppDir root
    cp "${appdir}/usr/share/applications/easyssh-lite.desktop" "${appdir}/easyssh-lite.desktop"

    # Create placeholder icon (should be replaced with actual icon)
    touch "${appdir}/easyssh-lite.png"
}

package_macos() {
    local arch="$1"

    log_step "Packaging for macOS $arch..."

    local pkg_name="easyssh-lite-v${VERSION}-macos-${arch}"
    local pkg_dir="${RELEASE_DIR}/${pkg_name}"

    mkdir -p "${pkg_dir}/EasySSH Lite.app/Contents/MacOS"
    mkdir -p "${pkg_dir}/EasySSH Lite.app/Contents/Resources"

    # Create Info.plist
    cat > "${pkg_dir}/EasySSH Lite.app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>EasySSH Lite</string>
    <key>CFBundleIdentifier</key>
    <string>com.anixops.easyssh-lite</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>EasySSH Lite</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>LSUIElement</key>
    <false/>
</dict>
</plist>
EOF

    # Copy binary
    local binary_source
    if [ "$arch" = "universal" ]; then
        binary_source="${RELEASE_DIR}/macos-universal/EasySSH"
    else
        binary_source="${WORKSPACE_ROOT}/crates/easyssh-platforms/macos/EasySSH/.build/release/EasySSH"
    fi

    if [ -f "$binary_source" ]; then
        cp "$binary_source" "${pkg_dir}/EasySSH Lite.app/Contents/MacOS/EasySSH Lite"
        chmod +x "${pkg_dir}/EasySSH Lite.app/Contents/MacOS/EasySSH Lite"
    fi

    # Create DMG
    create_macos_dmg "$pkg_dir" "$pkg_name"

    log_info "Package created: ${RELEASE_DIR}/${pkg_name}.dmg"
}

create_macos_dmg() {
    local pkg_dir="$1"
    local pkg_name="$2"

    log_info "Creating DMG..."

    local dmg_path="${RELEASE_DIR}/${pkg_name}.dmg"
    local temp_dmg="${RELEASE_DIR}/temp_${pkg_name}.dmg"
    local mount_point="/Volumes/EasySSH Lite"

    # Remove old DMG if exists
    rm -f "$dmg_path"

    # Create temporary DMG
    hdiutil create -srcfolder "$pkg_dir" -volname "EasySSH Lite" -fs HFS+ -format UDRW -size 100m "$temp_dmg"

    # Mount the DMG
    local device
    device=$(hdiutil attach -readwrite -noverify "$temp_dmg" | grep -E '^/dev/' | sed 1q | awk '{print $1}')

    sleep 2

    # Add Applications folder symlink
    if [ -d "$mount_point" ]; then
        ln -s /Applications "${mount_point}/Applications"

        # Set up the DMG appearance
        echo '
           tell application "Finder"
             tell disk "EasySSH Lite"
                   open
                   set current view of container window to icon view
                   set toolbar visible of container window to false
                   set statusbar visible of container window to false
                   set the bounds of container window to {400, 100, 885, 430}
                   set theViewOptions to the icon view options of container window
                   set arrangement of theViewOptions to not arranged
                   set icon size of theViewOptions to 72
                   set position of item "EasySSH Lite.app" of container window to {100, 100}
                   set position of item "Applications" of container window to {375, 100}
                   update container window
                   close
             end tell
           end tell
        ' | osascript

        sync
        sleep 2

        # Unmount
        hdiutil detach "$mount_point"
    fi

    # Convert to compressed DMG
    hdiutil convert "$temp_dmg" -format UDZO -o "$dmg_path"

    # Cleanup
    rm -f "$temp_dmg"

    log_info "DMG created: $dmg_path"
}

# ============================================================================
# Code Signing Preparation
# ============================================================================
prepare_code_signing() {
    log_step "Preparing for code signing..."

    case "$CURRENT_PLATFORM" in
        macos)
            prepare_macos_signing
            ;;
        linux)
            log_info "Linux: Code signing preparation complete (AppImage signing via appimage-sign)"
            ;;
    esac
}

prepare_macos_signing() {
    log_info "macOS code signing preparation:"
    log_info "  - Use 'codesign --force --deep --sign \"Developer ID\" \"EasySSH Lite.app\"'
    log_info "  - For distribution, use 'productsign' for the PKG"
    log_info "  - Notarization: 'xcrun altool --notarize-app'"

    # Create a signing script template
    local sign_script="${RELEASE_DIR}/sign-macos.sh"
    cat > "$sign_script" << 'EOF'
#!/bin/bash
# macOS Code Signing Script
# Usage: ./sign-macos.sh "Developer ID Application: Your Name (TEAM_ID)"

DEVELOPER_ID="${1:-}"
APP_PATH="EasySSH Lite.app"
DMG_PATH="*.dmg"

if [ -z "$DEVELOPER_ID" ]; then
    echo "Error: Developer ID required"
    echo "Usage: $0 \"Developer ID Application: Your Name (TEAM_ID)\""
    exit 1
fi

# Sign the app
codesign --force --deep --sign "$DEVELOPER_ID" "$APP_PATH"

# Verify signature
codesign --verify --deep --strict "$APP_PATH"

# Notarize (requires app-specific password)
# xcrun altool --notarize-app --primary-bundle-id "com.anixops.easyssh-lite" \
#     --username "your@email.com" --password "@keychain:AC_PASSWORD" \
#     --file "$DMG_PATH"

echo "Signing complete. Manual notarization may be required."
EOF
    chmod +x "$sign_script"

    log_info "Signing script created: $sign_script"
}

# ============================================================================
# Checksum Generation
# ============================================================================
generate_checksums() {
    log_step "Generating checksums..."

    local checksum_file="${RELEASE_DIR}/SHA256SUMS.txt"

    echo "EasySSH Lite v${VERSION} Release Checksums" > "$checksum_file"
    echo "=========================================" >> "$checksum_file"
    echo "" >> "$checksum_file"
    echo "Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")" >> "$checksum_file"
    echo "" >> "$checksum_file"

    cd "$RELEASE_DIR"

    # Generate checksums for all packages
    for file in *.tar.gz *.dmg *.zip; do
        if [ -f "$file" ]; then
            echo "" >> "$checksum_file"
            echo "## $(echo "$file" | sed 's/.*\.//')" >> "$checksum_file"
            sha256sum "$file" >> "$checksum_file"
        fi
    done

    cd "$WORKSPACE_ROOT"

    log_info "Checksums written to: $checksum_file"
}

# ============================================================================
# Version Injection
# ============================================================================
inject_version() {
    log_step "Injecting version information..."

    # Create version info file
    local version_info="${RELEASE_DIR}/version.json"
    cat > "$version_info" << EOF
{
    "name": "EasySSH Lite",
    "version": "${VERSION}",
    "build_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
    "git_commit": "$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')",
    "git_branch": "$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown')",
    "rustc_version": "$(rustc --version | cut -d' ' -f2)",
    "features": ["lite"]
}
EOF

    log_info "Version info: $version_info"
}

# ============================================================================
# Main Build Process
# ============================================================================
main() {
    echo "=========================================="
    echo "  EasySSH Lite Build Script v${VERSION}"
    echo "=========================================="
    echo ""

    # Create release directory
    mkdir -p "$RELEASE_DIR"

    # Run dependency checks
    check_deps

    # Build based on platform
    case "$CURRENT_PLATFORM" in
        linux)
            if [ "$TARGET" = "native" ] || [ "$TARGET" = "all" ]; then
                build_linux x86_64
                build_linux aarch64
            elif [ "$TARGET" = "x86_64" ] || [ "$TARGET" = "x64" ]; then
                build_linux x86_64
            elif [ "$TARGET" = "aarch64" ] || [ "$TARGET" = "arm64" ]; then
                build_linux aarch64
            else
                build_linux "$TARGET"
            fi
            ;;
        macos)
            if [ "$TARGET" = "native" ] || [ "$TARGET" = "all" ]; then
                build_macos universal
            elif [ "$TARGET" = "x86_64" ] || [ "$TARGET" = "x64" ]; then
                build_macos x86_64
            elif [ "$TARGET" = "aarch64" ] || [ "$TARGET" = "arm64" ]; then
                build_macos aarch64
            else
                build_macos "$TARGET"
            fi
            ;;
        *)
            log_error "Unsupported platform: $CURRENT_PLATFORM"
            exit 1
            ;;
    esac

    # Post-build steps
    inject_version
    prepare_code_signing
    generate_checksums

    # Summary
    echo ""
    echo "=========================================="
    log_info "Build Complete!"
    echo "=========================================="
    echo ""
    echo "Output directory: $RELEASE_DIR"
    echo ""
    echo "Generated artifacts:"
    find "$RELEASE_DIR" -type f \( -name "*.tar.gz" -o -name "*.dmg" -o -name "*.zip" -o -name "*.json" -o -name "*.txt" \) -exec basename {} \; | sed 's/^/  - /'
    echo ""

    # Next steps
    echo "Next steps:"
    case "$CURRENT_PLATFORM" in
        macos)
            echo "  1. Sign the app: ${RELEASE_DIR}/sign-macos.sh \"Developer ID...\""
            ;;
        linux)
            echo "  1. Test the packages in a clean VM"
            echo "  2. (Optional) Create AppImage using appimage-builder"
            ;;
    esac
    echo "  3. Upload to GitHub releases"
    echo ""
}

# Run main if not sourced
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    main "$@"
fi
