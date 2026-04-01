# App Store Compliant Package.swift

This document describes the App Store-compliant setup for EasySSH macOS SwiftUI.

## Problem Solved

**Before (App Store Rejected):**
```swift
.unsafeFlags(["-I", "../../../../core/target/include"]),
.unsafeFlags(["-L", "../../../../core/target/release"]),
.unsafeFlags(["-leasyssh_core"]),
```

**After (App Store Approved):**
```swift
.systemLibrary(
    name: "CEasySSHCore",
    path: "Sources/CEasySSHCore",
    pkgConfig: nil,
    providers: nil
),
```

## Structure

```
Sources/CEasySSHCore/
├── include/
│   ├── module.modulemap    # Swift module definition
│   ├── shim.h              # Stable header path
│   └── easyssh_core.h      # Rust FFI header (copied/synced)
```

## Build Approaches

### 1. Development Build (Local)

```bash
# 1. Build the Rust core
cd core
cargo build --release

# 2. Set environment variable
export EASYSSH_CORE_PATH=$PWD

# 3. Build Swift package
cd platforms/macos/easyssh-swiftui
swift build
```

### 2. Using Build Script

```bash
cd platforms/macos/easyssh-swiftui
./scripts/build-rust-core.sh release
swift build
```

### 3. CI/CD Build (GitHub Actions)

```yaml
- name: Build Rust Core
  run: |
    cd core
    cargo build --release --features standard
    echo "EASYSSH_CORE_PATH=$PWD" >> $GITHUB_ENV

- name: Build Swift Package
  run: |
    cd platforms/macos/easyssh-swiftui
    swift build -Xlinker -L -Xlinker ${{ env.EASYSSH_CORE_PATH }}/target/release
```

### 4. App Store Distribution (XCFramework)

For App Store submission, use the binaryTarget approach:

```swift
// In Package.swift, comment out .systemLibrary and use:
.binaryTarget(
    name: "CEasySSHCore",
    path: "Frameworks/CEasySSHCore.xcframework"
)
```

Build the XCFramework:
```bash
./scripts/build-xcframework.sh
```

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `EASYSSH_CORE_PATH` | Absolute path to core directory | `/Users/dev/EasySSH/core` |
| `CI` | Set to "true" in CI environments | `true` |

## Why systemLibrary?

1. **App Store Compliant**: No unsafe compiler flags in release builds
2. **Modulemap-based**: Clean Swift/C interop via module.modulemap
3. **Header Management**: Shim header provides stable include path
4. **CI/CD Friendly**: Environment variables control paths
5. **No Hardcoded Paths**: Works from any directory structure

## Troubleshooting

### "Library not found: easyssh_core"

```bash
# Check library exists
ls $EASYSSH_CORE_PATH/target/release/libeasyssh_core.a

# Set correct path
export EASYSSH_CORE_PATH=/absolute/path/to/core
swift build
```

### "Header not found"

```bash
# Sync headers
cp core/target/include/easyssh_core.h \
   platforms/macos/easyssh-swiftui/Sources/CEasySSHCore/include/
```

### "Undefined symbols"

The Rust core may not be built for the correct architecture:
```bash
# Build universal binary
cd core
rustc --print target-list | grep apple
cargo build --target aarch64-apple-darwin --release
cargo build --target x86_64-apple-darwin --release
```
