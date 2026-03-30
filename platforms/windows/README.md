# EasySSH for Windows

Native Windows SSH client built with WinUI 3 and Rust core.

## Architecture

```
EasySSH.exe
├── WinUI 3 Frontend (Rust + windows-rs)
│   ├── WinUI 3 XAML Controls
│   ├── Windows App SDK
│   └── Modern Windows 11 style
├── easyssh-core (Rust library)
│   ├── SSH connections (ssh2 crate)
│   ├── Encrypted storage
│   └── Session management
```

## Requirements

- Windows 10 1809+ / Windows 11
- Windows App SDK 1.5+
- Rust 1.75+
- Visual Studio 2022 (with C++ build tools)

## Building

```powershell
# Install prerequisites
# - Visual Studio 2022 with "Desktop development with C++"
# - Rust toolchain

# Build Rust core
cd ..\..\core
cargo build --release

# Build Windows app
cd ..\platforms\windows\easyssh-winui
cargo build --release

# The binary will be at target\release\EasySSH.exe
```

## Features

### Lite Mode
- Server list with search
- One-click connect to Windows Terminal or built-in terminal
- Windows Hello / Credential Locker integration
- Encrypted local storage

### Standard Mode (Future)
- Embedded terminal (using conpty or terminal control)
- Split-pane support
- SFTP file browser
- PowerShell/WSL integration

### Pro Mode (Future)
- Azure AD / Entra ID integration
- Team workspace
- Audit logging
- RBAC

## Packaging

```powershell
# Create MSIX package
msbuild /t:publish /p:Configuration=Release

# Or use MSIX Packaging Tool
```
