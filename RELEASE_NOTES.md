# EasySSH v0.3.0 Release Notes

**Release Date:** April 1, 2026
**Version:** 0.3.0
**Codename:** Native Foundations

---

## Overview

EasySSH v0.3.0 marks a significant milestone with the introduction of native platform UIs and a complete architecture refactor. This release focuses on delivering native desktop experiences while maintaining a shared core library.

## What's New

### Windows Native Client

The Windows version now features a complete native UI built with egui:

- **Server Management**: Add, edit, and organize SSH server configurations
- **SSH Connections**: Direct SSH connection support with credential management
- **Modern Interface**: Native Windows look and feel with GPU acceleration
- **WebSocket API**: External control interface for automation and integration

### Cross-Platform FFI Layer

A new FFI (Foreign Function Interface) layer enables native platforms to leverage the shared Rust core:

- **Unified Core**: Single codebase for business logic across all platforms
- **Native Performance**: Platform-specific UI with native performance
- **Type Safety**: Rust's safety guarantees extended to platform boundaries

### CI/CD Automation

Automated build pipeline for consistent releases:

- **Multi-Platform Builds**: Windows, Linux (GTK4), and macOS
- **Automated Testing**: Continuous integration with cargo test
- **Release Automation**: Streamlined artifact generation

## Download

| Platform | File | Size | SHA-256 |
|----------|------|------|---------|
| Windows | EasySSH-v0.3.0-x86_64.exe | TBD | TBD |
| Source | easyssh-v0.3.0.tar.gz | TBD | TBD |

## System Requirements

### Windows
- Windows 10 or later (64-bit)
- Visual C++ Redistributable 2019 or later
- 100 MB free disk space

### Linux
- GTK 4.0 or later
- libadwaita 1.0 or later
- 100 MB free disk space

### macOS
- macOS 12.0 (Monterey) or later
- 100 MB free disk space

## Installation

### Windows
1. Download `EasySSH-v0.3.0-x86_64.exe`
2. Run the installer (or portable executable)
3. Launch EasySSH from the Start Menu or desktop shortcut

### Building from Source
```bash
git clone https://github.com/anixops/easyssh.git
cd easyssh
cargo build --release -p easyssh-winui  # Windows
cargo build --release -p easyssh-gtk4   # Linux
```

## Known Issues

- Linux GTK4 version requires manual theme configuration on some distributions
- macOS version is in early development
- SFTP file transfer UI is pending implementation

## Breaking Changes

This release introduces significant architectural changes:

- Configuration format has been updated (migration required from v0.2.0)
- API endpoints have changed for external integrations
- Database schema updated with new tables

## Security Notes

- All credentials are stored using OS-native keychain/keyring
- Master password uses Argon2id with secure parameters
- Memory is cleared using zeroize after credential operations

## Acknowledgments

Thank you to all contributors who helped make this release possible.

## Feedback and Support

- **Issues:** https://github.com/anixops/easyssh/issues
- **Discussions:** https://github.com/anixops/easyssh/discussions
- **Documentation:** https://docs.easyssh.dev

---

**Full Changelog:** [CHANGELOG.md](./CHANGELOG.md)
