#!/bin/bash
# Build all EasySSH Linux packages
# Usage: ./build-all-linux.sh [version]
# Example: ./build-all-linux.sh 0.3.0

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
VERSION="${1:-0.3.0}"

echo "========================================"
echo "EasySSH Linux Package Build"
echo "Version: ${VERSION}"
echo "========================================"

RELEASE_DIR="${PROJECT_ROOT}/releases/v${VERSION}/linux"
mkdir -p "${RELEASE_DIR}"

# Function to build a specific edition for all formats
build_edition() {
    local edition=$1
    echo ""
    echo "Building ${edition} edition..."
    echo "----------------------------------------"

    # Check if binary exists
    if [ ! -f "${PROJECT_ROOT}/target/release-${edition}/easyssh-${edition}" ]; then
        echo "Warning: Binary not found for ${edition} edition, skipping..."
        return
    fi

    # Build AppImage
    echo "  Building AppImage..."
    "${SCRIPT_DIR}/appimage/build-appimage.sh" "${VERSION}" "${edition}"

    # Build .deb
    if command -v dpkg-deb >/dev/null 2>&1; then
        echo "  Building .deb package..."
        "${SCRIPT_DIR}/deb/build-deb.sh" "${VERSION}" "${edition}"
    else
        echo "  Skipping .deb (dpkg-deb not found)"
    fi

    # Build .rpm
    if command -v rpmbuild >/dev/null 2>&1; then
        echo "  Building .rpm package..."
        "${SCRIPT_DIR}/rpm/build-rpm.sh" "${VERSION}" "${edition}"
    else
        echo "  Skipping .rpm (rpmbuild not found)"
    fi
}

# Build all editions
build_edition "lite"
build_edition "standard"
build_edition "pro"

# Generate combined checksums
echo ""
echo "Generating combined checksums..."
cd "${RELEASE_DIR}"
sha256sum *.AppImage *.deb *.rpm 2>/dev/null > SHA256SUMS.txt || true

# Create installation instructions
cat > "${RELEASE_DIR}/INSTALL.md" << 'EOF'
# EasySSH Linux Installation

## Package Types

### AppImage (Recommended for most users)
- **Works on**: Any Linux distribution (Ubuntu, Fedora, Arch, etc.)
- **Install**: Just download, make executable, and run
- **Auto-updates**: Yes (built-in update checker)

```bash
# Download and run
chmod +x EasySSH-lite-*.AppImage
./EasySSH-lite-*.AppImage

# Optional: Move to applications folder
mv EasySSH-lite-*.AppImage ~/.local/bin/easyssh-lite
```

### .deb (Debian/Ubuntu)
- **Works on**: Debian, Ubuntu, Linux Mint, Pop!_OS, etc.
- **Install**: Double-click or use dpkg

```bash
sudo dpkg -i easyssh-lite_*.deb
sudo apt-get install -f  # Fix any missing dependencies
```

### .rpm (Fedora/RHEL)
- **Works on**: Fedora, RHEL, CentOS, openSUSE, etc.
- **Install**: Use dnf or rpm

```bash
# Fedora/RHEL 8+
sudo dnf install easyssh-lite-*.rpm

# Older systems
sudo rpm -i easyssh-lite-*.rpm
```

## System Requirements

### All Editions
- Linux kernel 3.10+
- GLibc 2.31+ (Ubuntu 20.04+, Fedora 32+)

### Edition-Specific

**Lite:**
- GTK 4.0+
- libsecret (keyring)

**Standard/Pro:**
- GTK 4.0+
- WebKitGTK 4.1+
- libsecret (keyring)
- SQLite 3

## Post-Installation

### Desktop Integration
All packages include:
- Desktop entry (.desktop file)
- Icon in applications menu
- SSH protocol handler registration
- MIME type associations

### Permissions
EasySSH requires:
- Network access (for SSH connections)
- Keyring access (for secure credential storage)
- Home directory access (for config files)

## Troubleshooting

### AppImage won't run
```bash
# Check FUSE
sudo apt install libfuse2  # Debian/Ubuntu
sudo dnf install fuse       # Fedora

# Or use --appimage-extract-and-run
./EasySSH-lite-*.AppImage --appimage-extract-and-run
```

### Missing dependencies (deb)
```bash
sudo apt-get install -f
```

### Missing dependencies (rpm)
```bash
sudo dnf install --allowerasing easyssh-lite-*.rpm
```

### Keyring issues
```bash
# Ensure gnome-keyring or kwallet is running
# For headless systems, install secret-tool:
sudo apt install libsecret-tools  # Debian/Ubuntu
sudo dnf install libsecret          # Fedora
```

## Uninstallation

### AppImage
Simply delete the AppImage file. Config remains in `~/.config/easyssh/`.

### .deb
```bash
sudo dpkg -r easyssh-lite
```

### .rpm
```bash
sudo dnf remove easyssh-lite
# or
sudo rpm -e easyssh-lite
```

## Support

For issues or questions:
- GitHub: https://github.com/anixops/easyssh/issues
- Documentation: https://docs.anixops.com/easyssh
EOF

echo ""
echo "========================================"
echo "Build Complete!"
echo "========================================"
echo ""
echo "Output directory: ${RELEASE_DIR}"
echo ""
ls -lh "${RELEASE_DIR}"
