# EasySSH macOS SwiftUI

## Quick Start

```bash
# Build the Rust core first
cd ../../core
cargo build --release

# Build and run the SwiftUI app
cd platforms/macos/easyssh-swiftui
swift build
swift run EasySSH
```

## Project Overview

This is a modern, native macOS SSH client with:

- **SwiftUI** interface using NavigationSplitView
- **Rust core** integration via FFI
- **Keychain** for secure password storage
- **Three modes**: Lite, Standard, and Pro

## File Structure

```
Sources/EasySSH/
├── EasySSHApp.swift       # @main entry point
├── AppState.swift         # Global state management
├── Models/
│   └── ServerModels.swift # Data models
├── Views/
│   ├── AppShell.swift     # Main layout
│   ├── Sidebar/
│   │   └── SidebarView.swift
│   ├── Detail/
│   │   ├── ServerDetailView.swift
│   │   └── ServerForms.swift
│   └── Settings/
│       └── SettingsViews.swift
├── Services/
│   └── KeychainService.swift
└── PreviewContent/
    └── Previews.swift

Sources/EasySSHBridge/
└── EasySSHCoreBridge.swift  # Rust FFI bridge
```

## Building

### Requirements
- macOS 14.0+
- Xcode 15.0+
- Swift 5.9+

### Build Steps

1. Build Rust core:
```bash
cd core
cargo build --release
```

2. Build Swift package:
```bash
cd platforms/macos/easyssh-swiftui
swift build
```

3. Run in Xcode:
```bash
open Package.swift
```

## Features

- ✅ Modern NavigationSplitView layout
- ✅ Server CRUD operations
- ✅ Group management
- ✅ Search and filtering
- ✅ Quick Connect
- ✅ Keychain integration
- ✅ Comprehensive settings
- ✅ SwiftUI Previews

## Development

### Add a new FFI function:

1. Add declaration in `EasySSHCoreBridge.swift`:
```swift
@_silgen_name("easyssh_new_function")
func easyssh_new_function(_ handle: OpaquePointer?) -> Int32
```

2. Wrap in actor method:
```swift
public func newFunction() async throws {
    // implementation
}
```

### Run tests:
```bash
swift test
```

## License

See main project repository for license details.
