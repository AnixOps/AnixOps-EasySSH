#!/bin/bash
# EasySSH GTK4 AppImage Build Script
# Creates a portable AppImage package for Linux

set -e

# Configuration
APP_NAME="EasySSH"
APP_ID="com.anixops.easyssh"
DESKTOP_FILE="easyssh.desktop"
LOWERAPP="easyssh"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[AppImage]${NC} $1"; }
log_success() { echo -e "${GREEN}[AppImage]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[AppImage]${NC} $1"; }
log_error() { echo -e "${RED}[AppImage]${NC} $1"; }

# Show help
show_help() {
    cat << EOF
EasySSH AppImage Builder

Usage: $0 [OPTIONS]

OPTIONS:
    -h, --help              Show this help message
    -v, --version VERSION   Set version (default: from Cargo.toml)
    -o, --output DIR        Output directory (default: ../../releases)
    -b, --build-type TYPE   Build type: release|debug (default: release)
    --skip-build            Skip cargo build (use existing binary)
    --appimagetool PATH     Path to appimagetool (auto-download if not specified)
    --clean                 Clean previous build artifacts

ENVIRONMENT:
    VERSION                 Override version
    CARGO_TARGET_DIR        Override cargo target directory
    APPIMAGE_EXTRACT_AND_RUN    Extract and run (for testing)

EXAMPLES:
    $0                      # Build AppImage with default settings
    $0 -v 1.2.3             # Build with specific version
    $0 --skip-build         # Use existing binary
    $0 --clean              # Clean and rebuild

EOF
}

# Parse arguments
VERSION=""
OUTPUT_DIR=""
BUILD_TYPE="release"
SKIP_BUILD=false
APPIMAGETOOL=""
CLEAN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -v|--version)
            VERSION="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -b|--build-type)
            BUILD_TYPE="$2"
            shift 2
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --appimagetool)
            APPIMAGETOOL="$2"
            shift 2
            ;;
        --clean)
            CLEAN=true
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Detect version from Cargo.toml if not specified
if [ -z "$VERSION" ]; then
    VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
    if [ -z "$VERSION" ]; then
        VERSION="0.3.0"
    fi
fi

log_info "Building EasySSH AppImage v${VERSION}"

# Set output directory
if [ -z "$OUTPUT_DIR" ]; then
    OUTPUT_DIR="../../releases/v${VERSION}/linux"
fi

# Detect architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64)
        ARCH_NAME="x86_64"
        APPIMAGE_ARCH="x86_64"
        ;;
    aarch64|arm64)
        ARCH_NAME="aarch64"
        APPIMAGE_ARCH="aarch64"
        ;;
    *)
        log_error "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Clean if requested
if [ "$CLEAN" = true ]; then
    log_info "Cleaning build artifacts..."
    rm -rf build/AppDir
    rm -f "$OUTPUT_DIR"/*.AppImage
    cargo clean 2>/dev/null || true
fi

# Build binary
if [ "$SKIP_BUILD" = false ]; then
    log_info "Building EasySSH binary (${BUILD_TYPE})..."

    if [ "$BUILD_TYPE" = "release" ]; then
        cargo build --release
        BINARY_PATH="target/release/easyssh-gtk4"
    else
        cargo build
        BINARY_PATH="target/debug/easyssh-gtk4"
    fi

    if [ ! -f "$BINARY_PATH" ]; then
        log_error "Binary not found at $BINARY_PATH"
        exit 1
    fi

    # Strip binary for release builds
    if [ "$BUILD_TYPE" = "release" ]; then
        strip "$BINARY_PATH" 2>/dev/null || true
    fi
else
    log_info "Using existing binary..."
    if [ "$BUILD_TYPE" = "release" ]; then
        BINARY_PATH="target/release/easyssh-gtk4"
    else
        BINARY_PATH="target/debug/easyssh-gtk4"
    fi

    if [ ! -f "$BINARY_PATH" ]; then
        log_error "Binary not found at $BINARY_PATH"
        exit 1
    fi
fi

# Get appimagetool
if [ -z "$APPIMAGETOOL" ]; then
    log_info "Downloading appimagetool..."

    APPIMAGETOOL_URL="https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${APPIMAGE_ARCH}.AppImage"
    APPIMAGETOOL="/tmp/appimagetool-${APPIMAGE_ARCH}.AppImage"

    if [ ! -f "$APPIMAGETOOL" ]; then
        curl -L -o "$APPIMAGETOOL" "$APPIMAGETOOL_URL"
        chmod +x "$APPIMAGETOOL"
    fi
fi

if [ ! -f "$APPIMAGETOOL" ]; then
    log_error "appimagetool not found at $APPIMAGETOOL"
    exit 1
fi

# Create AppDir structure
log_info "Creating AppDir structure..."
APPDIR="build/AppDir"

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/lib"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"
mkdir -p "$APPDIR/usr/share/icons/hicolor/128x128/apps"
mkdir -p "$APPDIR/usr/share/icons/hicolor/64x64/apps"
mkdir -p "$APPDIR/usr/share/icons/hicolor/scalable/apps"
mkdir -p "$APPDIR/usr/share/metainfo"
mkdir -p "$APPDIR/usr/share/doc/easyssh"

# Copy binary
cp "$BINARY_PATH" "$APPDIR/usr/bin/easyssh"
chmod +x "$APPDIR/usr/bin/easyssh"

# Create desktop entry
cat > "$APPDIR/usr/share/applications/${DESKTOP_FILE}" << EOF
[Desktop Entry]
Name=${APP_NAME}
GenericName=SSH Client
Comment=Modern SSH client with GTK4 interface
Exec=easyssh %F
Icon=${APP_ID}
Type=Application
Categories=Network;RemoteAccess;System;Utility;
Keywords=ssh;terminal;remote;sftp;scp;
MimeType=x-scheme-handler/ssh;
Terminal=false
StartupNotify=true
StartupWMClass=easyssh
Version=1.0
X-GNOME-UsesNotifications=true
EOF

# Create AppStream metadata
cat > "$APPDIR/usr/share/metainfo/${APP_ID}.metainfo.xml" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>${APP_ID}</id>
  <metadata_license>MIT</metadata_license>
  <project_license>MIT</project_license>
  <name>${APP_NAME}</name>
  <summary>Modern SSH client for Linux</summary>
  <description>
    <p>
      EasySSH is a modern SSH client with a clean GTK4 interface.
      Features include secure connection management, SFTP file browser,
      terminal emulator, and key authentication.
    </p>
  </description>
  <screenshots>
    <screenshot type="default">
      <caption>Main window</caption>
    </screenshot>
  </screenshots>
  <url type="homepage">https://github.com/anixops/easyssh</url>
  <developer_name>AnixOps</developer_name>
  <content_rating type="oars-1.1" />
  <releases>
    <release version="${VERSION}" date="$(date +%Y-%m-%d)" />
  </releases>
</component>
EOF

# Create AppRun script
cat > "$APPDIR/AppRun" << 'APPRUN_EOF'
#!/bin/bash
# AppRun script for EasySSH AppImage

SELF=$(readlink -f "$0")
HERE=${SELF%/*}

# Export library paths
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH}"
export PATH="${HERE}/usr/bin:${PATH}"

# GTK4 theming
export GTK_THEME="Adwaita:dark"

# XDG paths for portable config
if [ -z "$XDG_CONFIG_HOME" ]; then
    export XDG_CONFIG_HOME="$HOME/.config"
fi

# Execute the application
exec "${HERE}/usr/bin/easyssh" "$@"
APPRUN_EOF
chmod +x "$APPDIR/AppRun"

# Create symlinks for desktop integration
ln -sf "usr/share/applications/${DESKTOP_FILE}" "$APPDIR/${APP_ID}.desktop"
ln -sf "usr/share/icons/hicolor/256x256/apps/${APP_ID}.png" "$APPDIR/.DirIcon" 2>/dev/null || true

# Create placeholder icons (replace with actual icons if available)
touch "$APPDIR/usr/share/icons/hicolor/256x256/apps/${APP_ID}.png"
touch "$APPDIR/usr/share/icons/hicolor/128x128/apps/${APP_ID}.png"
touch "$APPDIR/usr/share/icons/hicolor/64x64/apps/${APP_ID}.png"

# Copy GTK4 and libadwaita libraries if available (for better compatibility)
log_info "Bundling GTK4 libraries..."

bundle_library() {
    local lib_name="$1"
    local lib_path=$(ldconfig -p | grep "$lib_name" | head -1 | awk '{print $4}')

    if [ -n "$lib_path" ] && [ -f "$lib_path" ]; then
        cp -L "$lib_path" "$APPDIR/usr/lib/" 2>/dev/null || true
        log_info "Bundled: $lib_name"
    fi
}

# Bundle critical GTK4 libraries
bundle_library "libgtk-4.so"
bundle_library "libadwaita-1.so"
bundle_library "libgio-2.0.so"
bundle_library "libgobject-2.0.so"
bundle_library "libglib-2.0.so"
bundle_library "libcairo.so"
bundle_library "libpango-1.0.so"
bundle_library "libgdk_pixbuf-2.0.so"
bundle_library "libgraphene-1.0.so"

# Create license file
cat > "$APPDIR/usr/share/doc/easyssh/LICENSE" << EOF
MIT License

Copyright (c) 2026 AnixOps

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
EOF

# Build AppImage
log_info "Building AppImage..."

OUTPUT_NAME="${APP_NAME}-${VERSION}-${APPIMAGE_ARCH}.AppImage"

# Set environment for appimagetool
export ARCH="$APPIMAGE_ARCH"

# Build the AppImage
if "$APPIMAGETOOL" "$APPDIR" "$OUTPUT_DIR/$OUTPUT_NAME" 2>&1; then
    log_success "AppImage built successfully!"
    log_info "Output: $OUTPUT_DIR/$OUTPUT_NAME"

    # Make executable
    chmod +x "$OUTPUT_DIR/$OUTPUT_NAME"

    # Show file info
    ls -lh "$OUTPUT_DIR/$OUTPUT_NAME"

    # Generate SHA256 checksum
    cd "$OUTPUT_DIR"
    sha256sum "$OUTPUT_NAME" > "${OUTPUT_NAME}.sha256"
    log_info "Checksum: $(cat ${OUTPUT_NAME}.sha256 | cut -d' ' -f1)"

    log_success "Build complete!"
    echo ""
    echo "To use:"
    echo "  chmod +x $OUTPUT_NAME"
    echo "  ./$OUTPUT_NAME"
    echo ""
else
    log_error "AppImage build failed!"

    # Create fallback portable tarball
    log_info "Creating fallback portable tarball..."
    TARBALL_NAME="${APP_NAME}-${VERSION}-${ARCH_NAME}-portable.tar.gz"
    tar -czf "$OUTPUT_DIR/$TARBALL_NAME" -C build easyssh
    log_info "Portable tarball created: $OUTPUT_DIR/$TARBALL_NAME"

    exit 1
fi
