# EasySSH macOS Build System

This directory contains the complete build system for packaging EasySSH for macOS, including code signing, notarization, and DMG creation.

## Overview

The macOS build system supports:
- **Universal Binary** (x86_64 + arm64/Apple Silicon)
- **Code Signing** (Developer ID or ad-hoc)
- **Apple Notarization** for Gatekeeper compatibility
- **Professional DMG** with custom styling

## Directory Structure

```
platforms/macos/
├── build-dmg.sh              # DMG packaging script
├── README.md                 # This file
├── EasySSH/                  # Swift Package Manager project
│   ├── Package.swift
│   └── Sources/
└── easyssh-swiftui/          # SwiftUI Xcode project
    ├── Package.swift
    └── Sources/
```

## Prerequisites

### Local Development
- macOS 13.0 or later
- Xcode 15.0 or later
- Swift 5.9 or later
- Rust toolchain with cargo
- `create-dmg` (optional, for enhanced DMG styling)

Install dependencies:
```bash
# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install create-dmg (optional, for better DMG appearance)
brew install create-dmg
```

### CI/CD (GitHub Actions)
The CI workflow requires the following secrets:
- `APPLE_DEVELOPER_ID` - Developer ID Application certificate name
- `APPLE_ID` - Apple ID email for notarization
- `APPLE_TEAM_ID` - Apple Developer Team ID
- `APPLE_APP_SPECIFIC_PASSWORD` - App-specific password for notarization

## Build Scripts

### 1. Local Build
```bash
# Basic build (ad-hoc signing)
./scripts/build-macos-ci.sh

# Build with specific version
VERSION=1.0.0 ./scripts/build-macos-ci.sh

# Production build with notarization
export CODESIGN_IDENTITY="Developer ID Application: Your Name"
export APPLE_ID="your@email.com"
export APPLE_TEAM_ID="TEAM_ID"
export APPLE_APP_SPECIFIC_PASSWORD="app-specific-password"
export NOTARIZE=true
./scripts/build-macos-ci.sh
```

### 2. DMG Packaging
```bash
cd platforms/macos

# Create DMG from existing app bundle
./build-dmg.sh

# With specific version
VERSION=1.0.0 ./build-dmg.sh

# With custom paths
SOURCE_APP=./dist/EasySSH.app OUTPUT_DIR=./dist ./build-dmg.sh
```

## CI/CD Workflow

### Manual Trigger
1. Go to Actions → macOS Release Build
2. Click "Run workflow"
3. Options:
   - **Version**: Enter version (e.g., `v1.0.0`)
   - **Enable notarization**: Check for production releases
   - **Create as draft**: Check to create draft release
   - **Upload artifact only**: Check to skip release creation

### Automatic Trigger
The workflow runs automatically when pushing tags:
- `v*.*.*` - Stable releases
- `v*.*.*-alpha.*` - Alpha releases
- `v*.*.*-beta.*` - Beta releases
- `v*.*.*-rc.*` - Release candidates
- `macos-*` - Development builds

## Code Signing

### Ad-Hoc Signing (Development)
Uses `codesign --sign -` for local testing. Users will see Gatekeeper warnings.

### Developer ID (Distribution)
Requires Apple Developer Program membership:
1. Create Developer ID Application certificate at https://developer.apple.com
2. Install in Keychain
3. Set `CODESIGN_IDENTITY` to certificate name (e.g., "Developer ID Application: Your Name (TEAM_ID)")

### Notarization
Required for apps distributed outside the App Store:
1. Generate app-specific password at https://appleid.apple.com
2. Configure GitHub secrets
3. Enable in workflow or script

## DMG Layout

The DMG is configured with:
- **Window size**: 600x400
- **Icon size**: 100px
- **App icon position**: Left side
- **Applications link**: Right side

To customize:
1. Add `dmg-background.png` to `Resources/` or `assets/`
2. Edit positions in `build-dmg.sh`
3. Rebuild

## Troubleshooting

### Build Failures

**Rust target not found:**
```bash
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

**Swift Package resolve errors:**
```bash
cd platforms/macos/EasySSH
swift package resolve
```

**Missing libraries:**
```bash
# Build core library first
cd core
cargo build --release
```

### Signing Issues

**Invalid Developer ID:**
```bash
# List available identities
security find-identity -v -p codesigning
```

**Notarization timeout:**
- Notarization can take 5-15 minutes
- Workflow timeout is set to 45 minutes

### DMG Issues

**"No mountable file systems" error:**
- Try rebuilding with `hdiutil` instead of `create-dmg`
- Check disk space

**DMG won't open:**
- Verify code signature: `codesign -dv --verbose=4 EasySSH.app`
- Check quarantine attribute: `xattr -l EasySSH.app`
- Remove quarantine: `xattr -dr com.apple.quarantine EasySSH.app`

## Output

Build artifacts are placed in:
```
releases/v{VERSION}/macos/
├── EasySSH-{VERSION}-macos-universal/
│   └── EasySSH.app
├── EasySSH-{VERSION}-macos-universal.dmg
├── build-info.txt
└── *.sha256
```

## GitHub Actions Workflow Jobs

| Job | Description |
|-----|-------------|
| `version` | Extracts version from tag or input |
| `build-macos` | Builds universal binary, signs, notarizes |
| `build-macos-arch` | Builds individual architectures (optional) |
| `create-release` | Creates GitHub release with artifacts |
| `update-manifest` | Updates release manifest JSON |
| `summary` | Posts build summary |

## Security

- All builds use hardened runtime (`--options runtime`)
- Notarization ensures Apple security scan
- SHA-256 checksums provided for all releases
- Code signing prevents tampering

## Resources

- [Apple Code Signing Guide](https://developer.apple.com/support/code-signing/)
- [Notarization Guide](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)
- [Hardened Runtime](https://developer.apple.com/documentation/security/hardened_runtime)
