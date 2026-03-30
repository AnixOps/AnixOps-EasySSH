# EasySSH Multi-Platform Native Architecture

## Project Structure

```
easyssh/
├── Cargo.toml              # Workspace root
│
├── core/                   # Shared Rust library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Library exports
│       ├── db.rs           # SQLite database
│       ├── ssh.rs          # SSH session management
│       ├── terminal.rs     # Native terminal integration
│       ├── crypto.rs       # Encryption (Argon2 + AES-256-GCM)
│       ├── keychain.rs     # Keychain/keyring integration
│       ├── edition.rs      # Version/edition info
│       ├── pro.rs          # Pro features (team/audit)
│       ├── sftp.rs         # SFTP support
│       ├── ai_programming.rs # AI self-improvement (debug builds)
│       └── debug_ws.rs     # Debug WebSocket server
│
├── tui/                    # Terminal UI (cross-platform CLI)
│   ├── Cargo.toml
│   └── main.rs
│
└── platforms/
    ├── macos/
    │   └── EasySSH/
    │       ├── Package.swift
    │       └── Sources/
    │           ├── EasySSH/         # SwiftUI app
    │           │   ├── EasySSHApp.swift
    │           │   ├── ContentView.swift
    │           │   ├── ServerViews.swift
    │           │   ├── Forms.swift
    │           │   └── Models.swift
    │           └── EasySSHCore/     # Rust FFI bridge
    │               └── EasySSHCoreBridge.swift
    │
    ├── linux/
    │   └── easyssh-gtk4/
    │       ├── Cargo.toml
    │       └── src/
    │           ├── main.rs          # GTK4 app entry
    │           ├── app.rs            # App state
    │           ├── models.rs         # GTK4 models
    │           ├── views/            # UI views
    │           │   ├── empty.rs
    │           │   ├── list.rs
    │           │   └── detail.rs
    │           └── styles.css        # GTK styles
    │
    ├── windows/
    │   └── easyssh-winui/
    │       ├── Cargo.toml
    │       └── src/
    │           ├── main.rs          # WinUI 3 app
    │           ├── pages/
    │           │   └── main.rs
    │           └── viewmodels/
    │               └── mod.rs
    │
    ├── ios/                    # (planned)
    └── android/                # (planned)
```

## Version Strategy

| Edition | Target Platform | Tech Stack | Status |
|---------|----------------|------------|--------|
| **Lite** | All desktop | Native terminal launch | ✅ Core ready |
| **Standard** | All desktop | Embedded terminal | 🔄 Planned |
| **Pro** | All desktop + cloud | Team features | 📋 Planned |

## Platform Status

| Platform | Status | Notes |
|----------|--------|-------|
| TUI (terminal) | ✅ Working | Cross-platform CLI tool |
| macOS | 🚧 Skeleton | SwiftUI + FFI bindings needed |
| Linux (GTK4) | 🚧 Skeleton | GTK4 + libadwaita, needs polish |
| Windows (WinUI) | 🚧 Skeleton | windows-rs bindings, needs XAML |
| iOS | 📋 Planned | SwiftUI, reuse macOS bridge |
| Android | 📋 Planned | Kotlin + JNI or UniFFI |

## Build Instructions

```bash
# Build shared core library
cd core && cargo build --release

# Build TUI (terminal interface)
cd tui && cargo build --release
./target/release/easyssh --help

# Build Linux GTK4 (requires: libgtk4-dev, libadwaita-dev)
cd platforms/linux/easyssh-gtk4
cargo build --release

# Build Windows (requires: Visual Studio)
cd platforms/windows/easyssh-winui
cargo build --release

# Build macOS (requires: Xcode)
cd platforms/macos/EasySSH
swift build
```

## Next Steps

### Phase 1: Core Stabilization
1. [ ] Add FFI bindings for core library (cbindgen/uniffi)
2. [ ] Create C header for platform interop
3. [ ] Implement real database operations in platform UIs

### Phase 2: Lite Mode
1. [ ] macOS: Complete SwiftUI + native terminal integration
2. [ ] Linux: Complete GTK4 list/details + terminal spawn
3. [ ] Windows: Complete WinUI 3 navigation + PowerShell spawn

### Phase 3: Standard Mode
1. [ ] Embedded terminal research
   - macOS: SwiftTerm or terminal.app embedding
   - Linux: VTE widget
   - Windows: ConPTY or terminal control
2. [ ] Split-pane layout implementation
3. [ ] SFTP file browser

### Phase 4: Pro Mode
1. [ ] Team workspace sync
2. [ ] Audit logging backend
3. [ ] RBAC implementation

## Key Design Decisions

1. **No Tauri/Electron**: Full native for best performance and UX
2. **Shared Rust Core**: Single source of truth for business logic
3. **Platform Native UI**: SwiftUI (macOS/iOS), GTK4 (Linux), WinUI 3 (Windows)
4. **FFI First**: C ABI for maximum language interop
5. **Lite Default**: Always start with native terminal, add embedded later

## Rust Core Features

- ✅ SQLite database with bundled feature
- ✅ Argon2id + AES-256-GCM encryption
- ✅ SSH agent/key/password auth
- ✅ Cross-platform keychain integration
- ✅ Native terminal spawn (macOS/Linux/Windows)
- ✅ Async SSH session management
- ✅ Debug WebSocket server (for AI tooling)
- ✅ Feature flags: lite/standard/pro

## Architecture Diagram

```
┌────────────────────────────────────────────────────────────┐
│                    Platform UI Layer                       │
├────────────────┬─────────────────┬───────────────────────┤
│   SwiftUI        │   GTK4          │   WinUI 3             │
│   (macOS/iOS)    │   (Linux)       │   (Windows)         │
└────────┬───────┴────────┬──────────┴─────────┬─────────────┘
         │                │                    │
         └────────────────┴────────────────────┘
                          │
                   ┌──────┴──────┐ FFI/C ABI
                   │  Rust Core  │
                   │ easyssh_core│
                   ├─────────────┤
                   │ • Database  │
                   │ • SSH/PTY   │
                   │ • Crypto    │
                   │ • Terminal  │
                   └─────────────┘
```
