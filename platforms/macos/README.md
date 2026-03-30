# EasySSH for macOS

Native macOS SSH client built with SwiftUI and Rust core.

## Architecture

```
EasySSH.app
├── SwiftUI Frontend (Swift)
├── EasySSHCore Bridge (Swift + FFI)
└── easyssh_core (Rust static library)
```

## Requirements

- macOS 13.0+
- Xcode 15.0+
- Swift 5.9+
- Rust 1.75+

## Building

```bash
# 1. Build Rust core as static library
cd ../../core
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Create universal binary
lipo -create \
    target/x86_64-apple-darwin/release/libeasyssh_core.a \
    target/aarch64-apple-darwin/release/libeasyssh_core.a \
    -output target/universal/libeasyssh_core.a

# 2. Build Swift app
cd ../platforms/macos/EasySSH
swift build

# Or open in Xcode
open Package.swift
```

## Features

### Lite Mode (Default)
- Server list with search
- Quick connect to native Terminal.app/iTerm2
- Keychain integration for credentials
- Encrypted local storage

### Standard Mode (Future)
- Embedded terminal via SwiftTerm
- Split-pane support
- SFTP file browser
- Session management

### Pro Mode (Future)
- Team workspace
- Audit logging
- RBAC permissions
- Shared snippets

## Development

```bash
# Run tests
swift test

# Format code
swift-format -i Sources/**/*.swift

# Build release
swift build -c release
```
