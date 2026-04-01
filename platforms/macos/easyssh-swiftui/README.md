# EasySSH for macOS - SwiftUI Native Implementation

A modern, native macOS SSH client built with SwiftUI and integrated with the EasySSH Rust core.

## Features

- **Native macOS Design**: Built with SwiftUI following Apple's Human Interface Guidelines
- **NavigationSplitView**: Modern three-column layout optimized for macOS
- **Multiple Connection Modes**:
  - **Lite**: External terminal integration (Terminal.app, iTerm2, etc.)
  - **Standard**: Embedded terminal with xterm.js
  - **Pro**: Team collaboration features
- **Secure Credential Storage**: macOS Keychain integration
- **Server Organization**: Groups, tags, and favorites
- **Quick Connect**: Fast connection with connection string parsing
- **Import/Export**: SSH config import and JSON export
- **Comprehensive Settings**: Terminal, connection, and appearance preferences

## Architecture

```
Sources/EasySSH/
├── EasySSHApp.swift          # App entry point
├── AppState.swift            # Global state management
├── Models/
│   └── ServerModels.swift    # Data models
├── Views/
│   ├── AppShell.swift        # Main layout container
│   ├── Sidebar/
│   │   └── SidebarView.swift # Navigation sidebar
│   ├── Detail/
│   │   ├── ServerDetailView.swift  # Server detail
│   │   └── ServerForms.swift       # Add/Edit forms
│   └── Settings/
│       └── SettingsViews.swift     # Settings panels
├── Services/
│   └── KeychainService.swift # Keychain integration
└── PreviewContent/
    └── Previews.swift        # SwiftUI previews

Sources/EasySSHBridge/
└── EasySSHCoreBridge.swift   # Rust FFI bridge
```

## Requirements

- macOS 14.0 (Sonoma) or later
- Xcode 15.0 or later
- Swift 5.9 or later
- Rust toolchain (for building core)

## Building

### 1. Build the Rust Core First

```bash
cd ../../core
cargo build --release
```

### 2. Build the Swift Package

```bash
cd platforms/macos/easyssh-swiftui
swift build
```

### 3. Run the App

```bash
swift run EasySSH
```

Or open in Xcode:

```bash
open Package.swift
```

## Project Structure Details

### AppEntry (EasySSHApp.swift)
- `@main` struct with `App` protocol
- WindowGroup with default size
- Menu commands for all actions
- Settings window
- Quick Connect auxiliary window

### State Management (AppState.swift)
- `@MainActor` singleton for thread safety
- ObservableObject for SwiftUI binding
- Handles all server CRUD operations
- Connection management
- Search and filtering
- Import/Export coordination

### Bridge Layer (EasySSHCoreBridge.swift)
- Actor-based for Swift concurrency
- Async/await wrappers around FFI calls
- Comprehensive error handling
- Automatic JSON encoding/decoding

### Keychain Service (KeychainService.swift)
- Secure password storage
- SSH key management
- Generic data storage
- Bulk operations
- Comprehensive error handling

## Design Highlights

### NavigationSplitView Layout
```swift
NavigationSplitView {
    SidebarContainer()  // 260pt ideal width
} detail: {
    DetailContainer()   // Flexible width
}
```

### Sidebar Features
- Section picker (All, Recent, Favorites, Connected)
- Real-time search with scope filtering
- Group expansion/collapse
- Context menus
- Hover actions
- Tag display

### Server Detail View
- Animated status indicators
- Tab navigation (Overview, Terminal, Files, Logs, Settings)
- Connection history
- Tags and notes
- Quick action buttons

### Forms
- Sectioned layout with icons
- Real-time validation
- Connection testing
- Tag input with FlowLayout
- Secure password fields

### Settings
- Tab-based organization
- General, Appearance, Connection, Security, Terminal, Advanced
- @AppStorage for persistence
- Color picker for accent color
- Font selection

## FFI Functions

The bridge exposes these Rust core functions:

```c
// Core
easyssh_init()
easyssh_destroy()
easyssh_free_string()

// Servers
easyssh_get_servers()
easyssh_get_server()
easyssh_add_server()
easyssh_update_server()
easyssh_delete_server()

// Groups
easyssh_get_groups()
easyssh_add_group()

// Connections
easyssh_connect_native()
easyssh_ssh_connect()
easyssh_ssh_disconnect()
easyssh_ssh_execute()

// Import/Export
easyssh_import_ssh_config()
easyssh_export_servers()
```

## Customization

### Accent Colors
8 built-in accent colors with automatic UI updates.

### Terminal Options
- External: Terminal.app, iTerm2, Kitty, Alacritty, WezTerm, Hyper
- Embedded: Font family, size, ligatures, cursor style

### Connection Profiles
Save and reuse connection configurations.

## Security

- All passwords stored in macOS Keychain
- SSH keys can be stored securely
- Clipboard auto-clear option
- App lock on sleep/inactivity
- Password required for sensitive operations

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| ⌘N | Add Server |
| ⌘⇧N | Quick Connect |
| ⌘Return | Connect to Selected |
| ⌘⇧D | Disconnect |
| ⌘E | Edit Server |
| ⌘Delete | Delete Server |
| ⌘, | Settings |

## Development

### Adding a New View
1. Create SwiftUI view in appropriate folder
2. Add to Previews.swift for live preview
3. Connect to AppState if needed
4. Add any required FFI functions to bridge

### Adding FFI Functions
1. Add `@_silgen_name` declaration in bridge
2. Wrap in EasySSHCoreBridge actor method
3. Handle errors appropriately
4. Test with Swift Preview

### Testing
```bash
swift test
```

## License

Part of the EasySSH project - see main repository for license details.
