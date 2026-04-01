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
- SFTP file management
- Terminal support

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
