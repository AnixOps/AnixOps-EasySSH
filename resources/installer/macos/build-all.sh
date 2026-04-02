#!/bin/bash
# Build all EasySSH macOS packages
# Usage: ./build-all-macos.sh [version]
# Example: ./build-all-macos.sh 0.3.0

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
VERSION="${1:-0.3.0}"

echo "========================================"
echo "EasySSH macOS Package Build"
echo "Version: ${VERSION}"
echo "========================================"

RELEASE_DIR="${PROJECT_ROOT}/releases/v${VERSION}/macos"
mkdir -p "${RELEASE_DIR}"

# Function to build a specific edition
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

    # Create DMG
    echo "  Creating DMG..."
    "${SCRIPT_DIR}/dmg/create-dmg.sh" "${VERSION}" "${edition}"
}

# Build all editions
build_edition "lite"
build_edition "standard"
build_edition "pro"

# Sign and notarize if credentials are available
if [ -n "$APPLE_DEVELOPER_ID" ] && [ -n "$APPLE_APP_PASSWORD" ] && [ -n "$APPLE_TEAM_ID" ]; then
    echo ""
    echo "Signing and notarizing..."
    "${SCRIPT_DIR}/signing/sign-all.sh" "${VERSION}"
else
    echo ""
    echo "Warning: Apple credentials not set, skipping code signing."
    echo "To sign, set these environment variables:"
    echo "  export APPLE_DEVELOPER_ID='Developer ID Application: Your Name (TEAM_ID)'"
    echo "  export APPLE_APP_PASSWORD='your-app-specific-password'"
    echo "  export APPLE_TEAM_ID='XXXXXXXXXX'"
fi

# Generate combined checksums
echo ""
echo "Generating checksums..."
cd "${RELEASE_DIR}"
shasum -a 256 *.dmg > SHA256SUMS.txt 2>/dev/null || true

# Create installation instructions
cat > "${RELEASE_DIR}/INSTALL.md" << 'EOF'
# EasySSH macOS Installation

## Installation Methods

### DMG (Recommended)

1. Download the DMG for your edition:
   - `EasySSH-lite-*.dmg` - Minimal SSH vault
   - `EasySSH-standard-*.dmg` - Full-featured client
   - `EasySSH-pro-*.dmg` - Enterprise with team features

2. Open the DMG file

3. Drag the app to Applications folder

4. Launch from Applications or Spotlight

### Homebrew

```bash
# Tap the repository (once)
brew tap anixops/easyssh

# Install your edition
brew install --cask easyssh-lite
# or
brew install --cask easyssh-standard
# or
brew install --cask easyssh-pro
```

### Build from Source

```bash
# Clone repository
git clone https://github.com/anixops/easyssh.git
cd easyssh

# Build specific edition
cargo build --release --profile release-lite
# or
cargo build --release --profile release-standard
# or
cargo build --release --profile release-pro
```

## System Requirements

- macOS 11.0 (Big Sur) or later
- Apple Silicon or Intel Mac
- ~50MB disk space (Lite), ~100MB (Standard), ~150MB (Pro)

## First Launch

### Security Warning

When launching for the first time, macOS may show:
> "EasySSH can't be opened because it is from an unidentified developer"

**If signed:**
- Right-click the app and select "Open"
- Click "Open" in the dialog
- Future launches will work normally

**If unsigned:**
1. Go to System Preferences > Security & Privacy > General
2. Click "Open Anyway" next to the EasySSH message
3. Confirm in the dialog

Or use terminal:
```bash
xattr -dr com.apple.quarantine /Applications/EasySSH\ Lite.app
```

## Permissions

EasySSH requires these permissions:

### Accessibility (for terminal integration)
- System Preferences > Security & Privacy > Privacy > Accessibility
- Add EasySSH to the list

### Files and Folders
- Access to ~/.ssh/ for key management
- Access to ~/ for configuration
- Access to any directory for SFTP (Standard/Pro)

### Network
- Outgoing connections for SSH (always allowed)
- Incoming connections for Pro server (if using local mode)

### Keychain
- Access to store and retrieve passwords
- Automatically granted on first use

## Uninstallation

```bash
# Remove app
rm -rf /Applications/EasySSH\ Lite.app

# Remove configuration (optional)
rm -rf ~/Library/Application\ Support/EasySSH
rm -rf ~/Library/Caches/EasySSH
rm -rf ~/Library/Preferences/com.anixops.easyssh.*
```

Or use Homebrew:
```bash
brew uninstall --cask easyssh-lite
```

## Troubleshooting

### "App is damaged and can't be opened"
```bash
xattr -dr com.apple.quarantine /Applications/EasySSH\ Lite.app
```

### Terminal not opening
- Grant Accessibility permission
- Check default terminal in preferences

### Keychain issues
```bash
# Reset keychain access
security delete-generic-password -s "com.anixops.easyssh.lite"
```

### Crash on startup
1. Check Console.app for crash logs
2. Try resetting preferences:
   ```bash
   defaults delete com.anixops.easyssh.lite
   ```

## Updates

### Automatic
- EasySSH checks for updates on startup
- Download and install from within the app

### Manual
1. Download new DMG
2. Replace app in Applications
3. Configuration is preserved

### Homebrew
```bash
brew update
brew upgrade --cask easyssh-lite
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
