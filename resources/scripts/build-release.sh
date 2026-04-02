#!/bin/bash
# EasySSH v0.3.0 Release Build Script
# Builds optimized release binaries for all platforms

set -e

VERSION="0.3.0"
RELEASE_DIR="releases/v${VERSION}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Create release directories
mkdir -p "${RELEASE_DIR}"
mkdir -p "${RELEASE_DIR}/windows"
mkdir -p "${RELEASE_DIR}/linux"
mkdir -p "${RELEASE_DIR}/macos"

# ==================== Windows Build ====================
build_windows() {
    log_info "Building Windows release..."

    cd platforms/windows/easyssh-winui

    # Build with maximum optimizations
    RUSTFLAGS="-C target-cpu=x86-64-v3 -C opt-level=3 -C lto=fat -C codegen-units=1" \
        cargo build --release --bin EasySSH

    cd ../../..

    # Create package structure
    local PKG_DIR="${RELEASE_DIR}/windows/EasySSH-${VERSION}-windows-x64"
    mkdir -p "${PKG_DIR}"

    # Copy binary
    cp target/release/EasySSH.exe "${PKG_DIR}/EasySSH.exe"

    # Create README
    cat > "${PKG_DIR}/README.txt" << 'EOF'
EasySSH v0.3.0 for Windows
==========================

Quick Start:
1. Run EasySSH.exe
2. Add your SSH servers via the UI
3. Connect using password or key authentication

System Requirements:
- Windows 10/11 64-bit
- No additional dependencies required

Features:
- Native Windows UI with egui
- SSH connection management
- Password and key-based authentication
- Server grouping
- Secure credential storage via Windows Credential Manager

For support, visit: https://github.com/anixops/easyssh
EOF

    # Create ZIP archive
    cd "${RELEASE_DIR}/windows"
    zip -r "EasySSH-${VERSION}-windows-x64.zip" "EasySSH-${VERSION}-windows-x64"
    cd ../../..

    log_info "Windows build complete: ${RELEASE_DIR}/windows/EasySSH-${VERSION}-windows-x64.zip"
}

# ==================== Linux Build ====================
build_linux() {
    log_info "Building Linux release..."

    cd platforms/linux/easyssh-gtk4

    # Build with optimizations
    RUSTFLAGS="-C opt-level=3 -C lto=fat -C codegen-units=1" \
        cargo build --release

    cd ../../..

    # Create package structure
    local PKG_DIR="${RELEASE_DIR}/linux/easyssh-${VERSION}-linux-x64"
    mkdir -p "${PKG_DIR}/usr/bin"
    mkdir -p "${PKG_DIR}/usr/share/applications"
    mkdir -p "${PKG_DIR}/usr/share/icons/hicolor/256x256/apps"

    # Copy binary
    cp target/release/easyssh "${PKG_DIR}/usr/bin/easyssh"

    # Create desktop entry
    cat > "${PKG_DIR}/usr/share/applications/easyssh.desktop" << EOF
[Desktop Entry]
Name=EasySSH
Comment=Native SSH Client
Exec=/usr/bin/easyssh
Icon=easyssh
Type=Application
Categories=Network;RemoteAccess;
Terminal=false
Version=${VERSION}
EOF

    # Create install script
    cat > "${PKG_DIR}/install.sh" << 'EOF'
#!/bin/bash
set -e

echo "Installing EasySSH..."

# Copy binary
sudo cp usr/bin/easyssh /usr/local/bin/
sudo chmod +x /usr/local/bin/easyssh

# Copy desktop entry
sudo cp usr/share/applications/easyssh.desktop /usr/share/applications/

echo "EasySSH installed successfully!"
echo "Run 'easyssh' to start the application."
EOF
    chmod +x "${PKG_DIR}/install.sh"

    # Create README
    cat > "${PKG_DIR}/README.md" << 'EOF'
# EasySSH v0.3.0 for Linux

## Installation

### Quick Install
```bash
./install.sh
```

### Manual Install
```bash
sudo cp usr/bin/easyssh /usr/local/bin/
sudo chmod +x /usr/local/bin/easyssh
```

## System Requirements
- GTK4
- libadwaita
- 64-bit Linux distribution

## Features
- Native GTK4 UI
- SSH connection management
- Server grouping
- Secure credential storage
- SFTP support

## Running
```bash
easyssh
```
EOF

    # Create tarball
    cd "${RELEASE_DIR}/linux"
    tar -czf "easyssh-${VERSION}-linux-x64.tar.gz" "easyssh-${VERSION}-linux-x64"
    cd ../../..

    log_info "Linux build complete: ${RELEASE_DIR}/linux/easyssh-${VERSION}-linux-x64.tar.gz"
}

# ==================== macOS Build ====================
build_macos() {
    log_warn "macOS build requires macOS environment with Xcode"
    log_info "Creating macOS package template..."

    local PKG_DIR="${RELEASE_DIR}/macos/EasySSH-${VERSION}-macos-universal"
    mkdir -p "${PKG_DIR}/EasySSH.app/Contents/MacOS"
    mkdir -p "${PKG_DIR}/EasySSH.app/Contents/Resources"

    # Create Info.plist
    cat > "${PKG_DIR}/EasySSH.app/Contents/Info.plist" << EOF
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
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
</dict>
</plist>
EOF

    # Placeholder for actual binary
    cat > "${PKG_DIR}/EasySSH.app/Contents/MacOS/EasySSH" << 'EOF'
#!/bin/bash
echo "EasySSH macOS binary placeholder"
echo "Build on macOS with:"
echo "  cd platforms/macos/EasySSH"
echo "  swift build -c release"
EOF
    chmod +x "${PKG_DIR}/EasySSH.app/Contents/MacOS/EasySSH"

    # Create DMG layout script
    cat > "${PKG_DIR}/create-dmg.sh" << 'EOF'
#!/bin/bash
# Run this on macOS to create the DMG
create-dmg \
  --volname "EasySSH Installer" \
  --window-pos 200 120 \
  --window-size 800 400 \
  --icon-size 100 \
  --app-drop-link 600 185 \
  "EasySSH-0.3.0-macos-universal.dmg" \
  "EasySSH.app"
EOF
    chmod +x "${PKG_DIR}/create-dmg.sh"

    # Create README
    cat > "${PKG_DIR}/README.md" << 'EOF'
# EasySSH v0.3.0 for macOS

## Build Instructions

### Requirements
- macOS 13.0 or later
- Xcode 15.0 or later
- Swift 5.9 or later

### Building
```bash
cd platforms/macos/EasySSH
swift build -c release
```

### Creating the App Bundle
```bash
# Copy binary to app bundle
cp .build/release/EasySSH EasySSH.app/Contents/MacOS/

# Sign the application
codesign --force --deep --sign - EasySSH.app
```

### Creating the DMG
```bash
cd releases/v0.3.0/macos/EasySSH-0.3.0-macos-universal
./create-dmg.sh
```

## Features
- Native SwiftUI interface
- macOS Keychain integration
- Server management
- SSH and SFTP support
- Native terminal integration
EOF

    log_info "macOS package template created: ${RELEASE_DIR}/macos/"
}

# ==================== Generate Checksums ====================
generate_checksums() {
    log_info "Generating checksums..."

    local CHECKSUM_FILE="${RELEASE_DIR}/SHA256SUMS.txt"

    echo "EasySSH v${VERSION} Release Checksums" > "${CHECKSUM_FILE}"
    echo "=======================================" >> "${CHECKSUM_FILE}"
    echo "" >> "${CHECKSUM_FILE}"
    echo "Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")" >> "${CHECKSUM_FILE}"
    echo "" >> "${CHECKSUM_FILE}"

    # Windows
    if [ -f "${RELEASE_DIR}/windows/EasySSH-${VERSION}-windows-x64.zip" ]; then
        echo "" >> "${CHECKSUM_FILE}"
        echo "## Windows" >> "${CHECKSUM_FILE}"
        sha256sum "${RELEASE_DIR}/windows/EasySSH-${VERSION}-windows-x64.zip" >> "${CHECKSUM_FILE}"
    fi

    # Linux
    if [ -f "${RELEASE_DIR}/linux/easyssh-${VERSION}-linux-x64.tar.gz" ]; then
        echo "" >> "${CHECKSUM_FILE}"
        echo "## Linux" >> "${CHECKSUM_FILE}"
        sha256sum "${RELEASE_DIR}/linux/easyssh-${VERSION}-linux-x64.tar.gz" >> "${CHECKSUM_FILE}"
    fi

    # macOS
    if [ -f "${RELEASE_DIR}/macos/EasySSH-${VERSION}-macos-universal.dmg" ]; then
        echo "" >> "${CHECKSUM_FILE}"
        echo "## macOS" >> "${CHECKSUM_FILE}"
        sha256sum "${RELEASE_DIR}/macos/EasySSH-${VERSION}-macos-universal.dmg" >> "${CHECKSUM_FILE}"
    fi

    log_info "Checksums written to: ${CHECKSUM_FILE}"
}

# ==================== Create Release Notes ====================
create_release_notes() {
    log_info "Creating release notes..."

    cat > "${RELEASE_DIR}/RELEASE_NOTES.md" << 'EOF'
# EasySSH v0.3.0 Release Notes

## Overview

EasySSH v0.3.0 is a major milestone release featuring fully native multi-platform implementations for Windows, Linux, and macOS.

## What's New

### Major Features

- **Native Windows Client** - Complete egui-based native UI with WebSocket control API
- **Native Linux Client (GTK4)** - Modern GNOME-style interface with libadwaita
- **Native macOS Client (SwiftUI)** - Native macOS experience with SwiftData integration
- **FFI Bridge System** - Cross-platform Rust core with native UI bindings
- **Enhanced Security** - Argon2id + AES-256-GCM encryption, OS-native keychain integration

### Platform-Specific Features

#### Windows
- Native egui interface with GPU-accelerated rendering
- WebSocket control API for automation
- Windows Credential Manager integration
- Portable executable (no installation required)

#### Linux
- GTK4 + libadwaita modern interface
- Native terminal integration
- System tray support
- Desktop environment integration

#### macOS
- SwiftUI native interface
- SwiftData persistence
- Menu bar extra support
- macOS Keychain integration

### Core Library Features
- Server management with grouping
- SSH connection handling (password/key auth)
- SFTP file operations
- Encrypted configuration storage
- Import/Export functionality

## Downloads

| Platform | Package | Size |
|----------|---------|------|
| Windows | EasySSH-0.3.0-windows-x64.zip | ~20MB |
| Linux | easyssh-0.3.0-linux-x64.tar.gz | ~15MB |
| macOS | EasySSH-0.3.0-macos-universal.dmg | ~25MB |

## Installation

### Windows
1. Download `EasySSH-0.3.0-windows-x64.zip`
2. Extract to desired location
3. Run `EasySSH.exe`

### Linux
```bash
tar -xzf easyssh-0.3.0-linux-x64.tar.gz
cd easyssh-0.3.0-linux-x64
./install.sh
```

### macOS
1. Download `EasySSH-0.3.0-macos-universal.dmg`
2. Open the DMG
3. Drag EasySSH to Applications

## Known Issues

- macOS build requires manual signing for distribution
- Linux requires GTK4 and libadwaita installed
- Windows Defender may show SmartScreen warning (unsigned binary)

## System Requirements

- **Windows**: Windows 10/11 64-bit
- **Linux**: GTK4, libadwaita, 64-bit distribution
- **macOS**: macOS 13.0+, Apple Silicon or Intel

## Security

All binaries are built with:
- Strip enabled (no debug symbols in release)
- LTO (Link Time Optimization)
- Maximum optimization level
- Checksums provided for verification

## Checksums

See `SHA256SUMS.txt` for complete checksum list.

## Support

- GitHub Issues: https://github.com/anixops/easyssh/issues
- Documentation: https://docs.anixops.com/easyssh

---

**Full Changelog**: https://github.com/anixops/easyssh/compare/v0.2.0...v0.3.0
EOF

    log_info "Release notes created: ${RELEASE_DIR}/RELEASE_NOTES.md"
}

# ==================== Main ====================
main() {
    log_info "EasySSH v${VERSION} Release Build"
    log_info "=================================="

    # Detect platform
    case "$(uname -s)" in
        Linux*)     PLATFORM=Linux;;
        Darwin*)    PLATFORM=Mac;;
        CYGWIN*|MINGW*|MSYS*) PLATFORM=Windows;;
        *)          PLATFORM="Unknown"
    esac

    log_info "Detected platform: ${PLATFORM}"

    # Build based on platform
    case ${PLATFORM} in
        Windows)
            build_windows
            ;;
        Linux)
            build_linux
            ;;
        Mac)
            build_macos
            ;;
        *)
            log_warn "Unknown platform, creating all package templates..."
            build_windows
            build_linux
            build_macos
            ;;
    esac

    # Generate artifacts that don't require building
    generate_checksums
    create_release_notes

    log_info "=================================="
    log_info "Release build complete!"
    log_info "Output directory: ${RELEASE_DIR}/"
    log_info ""
    log_info "Files created:"
    find "${RELEASE_DIR}" -type f -name "*.zip" -o -name "*.tar.gz" -o -name "*.dmg" -o -name "*.txt" -o -name "*.md" | while read f; do
        echo "  - $(basename "$f")"
    done
}

# Run main
main
