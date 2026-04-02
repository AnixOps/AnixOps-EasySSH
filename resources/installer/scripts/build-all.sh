#!/bin/bash
# Build all EasySSH installers for all platforms
# Usage: ./build-all.sh [version]
# Example: ./build-all.sh 0.3.0

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
VERSION="${1:-0.3.0}"

echo "========================================"
echo "EasySSH All-Platform Installer Build"
echo "Version: ${VERSION}"
echo "========================================"
echo ""

# Detect platform
PLATFORM=$(uname -s)

# Make all scripts executable
chmod +x "${SCRIPT_DIR}/windows/build-all.sh" 2>/dev/null || true
chmod +x "${SCRIPT_DIR}/windows/build-all.bat" 2>/dev/null || true
chmod +x "${SCRIPT_DIR}/linux/build-all.sh" 2>/dev/null || true
chmod +x "${SCRIPT_DIR}/linux/appimage/build-appimage.sh" 2>/dev/null || true
chmod +x "${SCRIPT_DIR}/linux/deb/build-deb.sh" 2>/dev/null || true
chmod +x "${SCRIPT_DIR}/linux/rpm/build-rpm.sh" 2>/dev/null || true
chmod +x "${SCRIPT_DIR}/macos/build-all.sh" 2>/dev/null || true
chmod +x "${SCRIPT_DIR}/macos/dmg/create-dmg.sh" 2>/dev/null || true
chmod +x "${SCRIPT_DIR}/macos/signing/sign-all.sh" 2>/dev/null || true

case "$PLATFORM" in
    Linux)
        echo "Detected Linux platform"
        echo ""
        "${SCRIPT_DIR}/linux/build-all.sh" "$VERSION"
        ;;
    Darwin)
        echo "Detected macOS platform"
        echo ""
        "${SCRIPT_DIR}/macos/build-all.sh" "$VERSION"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        echo "Detected Windows platform"
        echo ""
        if command -v bash >/dev/null 2>&1; then
            "${SCRIPT_DIR}/windows/build-all.sh" "$VERSION"
        else
            echo "Please run build-all.bat for Windows CMD/PowerShell"
            exit 1
        fi
        ;;
    *)
        echo "Unknown platform: $PLATFORM"
        exit 1
        ;;
esac

# Generate release notes
echo ""
echo "Generating release notes..."
RELEASE_ROOT="${PROJECT_ROOT}/releases/v${VERSION}"

cat > "${RELEASE_ROOT}/RELEASE_NOTES.md" << EOF
# EasySSH v${VERSION} Release Notes

## Downloads

### Windows
- **EasySSH Lite**: MSI installer, EXE installer, Portable ZIP
- **EasySSH Standard**: MSI installer, EXE installer, Portable ZIP
- **EasySSH Pro**: MSI installer, EXE installer, Portable ZIP

### macOS
- **EasySSH Lite**: DMG with signed app bundle
- **EasySSH Standard**: DMG with signed app bundle
- **EasySSH Pro**: DMG with signed app bundle

### Linux
- **EasySSH Lite**: AppImage, .deb, .rpm
- **EasySSH Standard**: AppImage, .deb, .rpm
- **EasySSH Pro**: AppImage, .deb, .rpm

## Installation Instructions

See platform-specific INSTALL.md files in each release directory.

## Checksums
SHA256 checksums are provided in SHA256SUMS.txt for all files.

## System Requirements

### Windows
- Windows 10/11 64-bit
- Edge WebView2 Runtime (Standard/Pro)

### macOS
- macOS 11.0+ (Big Sur)
- Apple Silicon or Intel

### Linux
- GTK 4.0+
- GLibc 2.31+
- WebKitGTK 4.1+ (Standard/Pro)

## Known Issues

None reported for this release.

## Support

For help or to report issues:
- GitHub: https://github.com/anixops/easyssh/issues
- Documentation: https://docs.anixops.com/easyssh
EOF

echo ""
echo "========================================"
echo "Build Complete!"
echo "========================================"
echo ""
echo "Release directory: ${RELEASE_ROOT}"
echo ""

# List all created files
find "${RELEASE_ROOT}" -type f -name "*.msi" -o -name "*.exe" -o -name "*.dmg" -o -name "*.AppImage" -o -name "*.deb" -o -name "*.rpm" 2>/dev/null | while read -r file; do
    ls -lh "$file"
done
