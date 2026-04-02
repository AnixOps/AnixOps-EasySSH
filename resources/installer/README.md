# EasySSH Installer Configuration

This directory contains all installer configurations for EasySSH's three editions (Lite, Standard, Pro) across all platforms (Windows, Linux, macOS).

## Directory Structure

```
resources/installer/
├── windows/
│   ├── wix/
│   │   ├── lite/EasySSH-Lite.wxs          # WiX MSI for Lite
│   │   ├── standard/EasySSH-Standard.wxs  # WiX MSI for Standard
│   │   └── pro/EasySSH-Pro.wxs            # WiX MSI for Pro
│   ├── nsis/
│   │   ├── easyssh-lite.nsi               # NSIS installer for Lite
│   │   ├── easyssh-standard.nsi           # NSIS installer for Standard
│   │   └── easyssh-pro.nsi                # NSIS installer for Pro
│   ├── build-all.sh                       # Bash build script
│   └── build-all.bat                      # Windows CMD build script
│
├── linux/
│   ├── appimage/build-appimage.sh         # AppImage builder
│   ├── deb/build-deb.sh                   # .deb package builder
│   ├── rpm/build-rpm.sh                   # .rpm package builder
│   └── build-all.sh                       # Linux master build script
│
├── macos/
│   ├── dmg/
│   │   ├── create-dmg.sh                  # DMG creator
│   │   └── entitlements.plist             # Code signing entitlements
│   ├── homebrew/
│   │   ├── easyssh-lite.rb                # Homebrew formula for Lite
│   │   ├── easyssh-standard.rb            # Homebrew formula for Standard
│   │   └── easyssh-pro.rb                 # Homebrew formula for Pro
│   ├── signing/
│   │   ├── README.md                      # Code signing guide
│   │   └── sign-all.sh                    # Automated signing script
│   └── build-all.sh                       # macOS master build script
│
└── scripts/
    └── build-all.sh                       # Universal build script
```

## Quick Start

### Build for Current Platform

```bash
# From project root
./resources/installer/scripts/build-all.sh 0.3.0
```

### Build for Specific Platform

**Windows:**
```bash
# Using Git Bash/MSYS2
./resources/installer/windows/build-all.sh 0.3.0

# Using CMD
resources\installer\windows\build-all.bat 0.3.0
```

**Linux:**
```bash
./resources/installer/linux/build-all.sh 0.3.0
```

**macOS:**
```bash
./resources/installer/macos/build-all.sh 0.3.0
```

## Installer Types by Platform

### Windows

| Type | Extension | Features |
|------|-----------|----------|
| **WiX MSI** | `.msi` | Standard Windows installer, auto-upgrades |
| **NSIS** | `.exe` | Custom wizard, portable mode option |
| **Portable** | `.zip` | No installation required, USB-ready |

### Linux

| Type | Extension | Best For |
|------|-----------|----------|
| **AppImage** | `.AppImage` | Universal, works on any distro |
| **Debian** | `.deb` | Ubuntu, Debian, Mint |
| **RPM** | `.rpm` | Fedora, RHEL, openSUSE |

### macOS

| Type | Extension | Distribution |
|------|-----------|--------------|
| **DMG** | `.dmg` | Direct download, app bundle |
| **Homebrew** | `brew install` | Package manager users |
| **Source** | `cargo build` | Developers |

## Build Prerequisites

### Windows
- [WiX Toolset v3.11+](https://wixtoolset.org/releases/)
- [NSIS 3.0+](https://nsis.sourceforge.io/Download)
- Windows SDK (optional, for signing)

### Linux
- `dpkg-deb` (for .deb packages)
- `rpmbuild` (for .rpm packages)
- `appimagetool` (for AppImage)

### macOS
- Xcode Command Line Tools
- Apple Developer account (for signing)
- `create-dmg` (optional, for nicer DMGs)

## Code Signing

### Windows
Set environment variables:
```bash
export SIGN_CERT="/path/to/certificate.pfx"
export SIGN_CERT_PASSWORD="password"
export SIGN_TIMESTAMP_URL="http://timestamp.digicert.com"
```

### macOS
Set environment variables:
```bash
export APPLE_DEVELOPER_ID="Developer ID Application: Name (TEAM_ID)"
export APPLE_APP_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="XXXXXXXXXX"
```

## Output Structure

After building, the release directory structure:

```
releases/v0.3.0/
├── windows/
│   ├── EasySSH-lite-0.3.0-x64.msi
│   ├── EasySSH-lite-0.3.0-x64.exe
│   ├── EasySSH-lite-0.3.0-portable.zip
│   ├── EasySSH-standard-0.3.0-x64.msi
│   ├── EasySSH-standard-0.3.0-x64.exe
│   ├── EasySSH-standard-0.3.0-portable.zip
│   ├── EasySSH-pro-0.3.0-x64.msi
│   ├── EasySSH-pro-0.3.0-x64.exe
│   ├── EasySSH-pro-0.3.0-portable.zip
│   ├── SHA256SUMS.txt
│   └── INSTALL.md
│
├── linux/
│   ├── EasySSH-lite-0.3.0-x86_64.AppImage
│   ├── EasySSH-lite-0.3.0_amd64.deb
│   ├── EasySSH-lite-0.3.0-1.x86_64.rpm
│   ├── EasySSH-standard-0.3.0-x86_64.AppImage
│   ├── EasySSH-standard-0.3.0_amd64.deb
│   ├── EasySSH-standard-0.3.0-1.x86_64.rpm
│   ├── EasySSH-pro-0.3.0-x86_64.AppImage
│   ├── EasySSH-pro-0.3.0_amd64.deb
│   ├── EasySSH-pro-0.3.0-1.x86_64.rpm
│   ├── SHA256SUMS.txt
│   └── INSTALL.md
│
├── macos/
│   ├── EasySSH-lite-0.3.0.dmg
│   ├── EasySSH-standard-0.3.0.dmg
│   ├── EasySSH-pro-0.3.0.dmg
│   ├── SHA256SUMS.txt
│   └── INSTALL.md
│
└── RELEASE_NOTES.md
```

## Edition-Specific Features

### Lite Edition
- Minimal dependencies
- Native terminal integration
- Small binary size (~5MB)
- Quick installation

### Standard Edition
- Embedded WebGL terminal
- WebView2 Runtime requirement (Windows)
- WebKitGTK requirement (Linux)
- Moderate binary size (~20MB)

### Pro Edition
- Includes local server mode
- Additional SSL/TLS dependencies
- Firewall rules (Windows)
- Largest binary size (~30MB)

## CI/CD Integration

### GitHub Actions Example

```yaml
- name: Build Installers
  run: |
    ./resources/installer/scripts/build-all.sh ${{ github.ref_name }}
  env:
    # Windows signing
    SIGN_CERT: ${{ secrets.CODE_SIGN_CERT }}
    SIGN_CERT_PASSWORD: ${{ secrets.CODE_SIGN_PASSWORD }}
    # macOS signing
    APPLE_DEVELOPER_ID: ${{ secrets.APPLE_DEVELOPER_ID }}
    APPLE_APP_PASSWORD: ${{ secrets.APPLE_APP_PASSWORD }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
```

## Troubleshooting

### Windows
- **WiX not found**: Ensure WiX is in PATH or set `WIX` environment variable
- **NSIS not found**: Add NSIS to PATH
- **Signing fails**: Verify certificate and password

### Linux
- **Permission denied**: Run `chmod +x` on shell scripts
- **Missing dependencies**: Install required build tools

### macOS
- **Notarization fails**: Check app uses only allowed APIs
- **Signing fails**: Verify certificates in Keychain

## Support

For issues or questions:
- GitHub Issues: https://github.com/anixops/easyssh/issues
- Documentation: https://docs.anixops.com/easyssh
