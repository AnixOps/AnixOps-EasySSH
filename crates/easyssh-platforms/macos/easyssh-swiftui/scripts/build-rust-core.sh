#!/bin/bash
#
# build-rust-core.sh - Build the Rust core library for macOS SwiftUI integration
#
# This script builds the Rust core as a universal static library (x86_64 + arm64)
# for use with the Swift Package Manager systemLibrary target.
#
# Usage:
#   ./scripts/build-rust-core.sh [debug|release]
#
# Output:
#   - core/target/release/libeasyssh_core.a (fat binary)
#   - core/target/include/easyssh_core.h (auto-generated)
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
CORE_DIR="$ROOT_DIR/core"
TARGET_DIR="$CORE_DIR/target"
MODE="${1:-release}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🔨 Building EasySSH Core for macOS..."
echo "   Mode: $MODE"
echo "   Root: $ROOT_DIR"

# Check for required tools
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Error: cargo not found. Please install Rust.${NC}"
    exit 1
fi

# Install cbindgen if needed for header generation
if ! command -v cbindgen &> /dev/null; then
    echo -e "${YELLOW}⚠️  cbindgen not found. Installing...${NC}"
    cargo install cbindgen
fi

cd "$CORE_DIR"

# Clean previous builds if requested
if [ "$CLEAN" = "1" ]; then
    echo "🧹 Cleaning previous builds..."
    rm -rf "$TARGET_DIR"
fi

# Build for host architecture (native)
echo "📦 Building native target..."
if [ "$MODE" = "debug" ]; then
    cargo build --features standard
    LIB_PATH="$TARGET_DIR/debug/libeasyssh_core.a"
else
    cargo build --release --features standard
    LIB_PATH="$TARGET_DIR/release/libeasyssh_core.a"
fi

# Check if universal build is needed (Apple Silicon + Intel)
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "🍎 Building universal binary for macOS..."

    # Build for x86_64
    echo "   - Building x86_64 target..."
    if [ "$MODE" = "debug" ]; then
        cargo build --target x86_64-apple-darwin --features standard 2>/dev/null || \
            echo -e "${YELLOW}   (Skipping x86_64, toolchain may not be installed)${NC}"
    else
        cargo build --target x86_64-apple-darwin --release --features standard 2>/dev/null || \
            echo -e "${YELLOW}   (Skipping x86_64, toolchain may not be installed)${NC}"
    fi

    # Build for aarch64
    echo "   - Building arm64 target..."
    if [ "$MODE" = "debug" ]; then
        cargo build --target aarch64-apple-darwin --features standard 2>/dev/null || \
            echo -e "${YELLOW}   (Skipping arm64, toolchain may not be installed)${NC}"
    else
        cargo build --target aarch64-apple-darwin --release --features standard 2>/dev/null || \
            echo -e "${YELLOW}   (Skipping arm64, toolchain may not be installed)${NC}"
    fi

    # Create universal binary if both exist
    X86_PATH="$TARGET_DIR/x86_64-apple-darwin/${MODE/libeasyssh_core.a}"
    ARM_PATH="$TARGET_DIR/aarch64-apple-darwin/${MODE/libeasyssh_core.a}"

    if [ -f "$X86_PATH" ] && [ -f "$ARM_PATH" ]; then
        echo "🔗 Creating universal binary..."
        lipo -create "$X86_PATH" "$ARM_PATH" -output "$LIB_PATH" 2>/dev/null || \
            echo -e "${YELLOW}⚠️  lipo failed, using native build only${NC}"
    fi
fi

# Generate/update C header
if command -v cbindgen &> /dev/null; then
    echo "📝 Generating C headers..."
    mkdir -p "$TARGET_DIR/include"
    cbindgen --lang c --crate easyssh-core --output "$TARGET_DIR/include/easyssh_core.h" 2>/dev/null || \
        echo -e "${YELLOW}⚠️  cbindgen failed, using existing header${NC}"
fi

# Sync header to Swift package
SWIFT_INCLUDE="$ROOT_DIR/platforms/macos/easyssh-swiftui/Sources/CEasySSHCore/include"
if [ -f "$TARGET_DIR/include/easyssh_core.h" ]; then
    echo "📋 Syncing headers to Swift package..."
    mkdir -p "$SWIFT_INCLUDE"
    cp "$TARGET_DIR/include/easyssh_core.h" "$SWIFT_INCLUDE/"
fi

# Verify library exists
if [ ! -f "$LIB_PATH" ]; then
    echo -e "${RED}❌ Build failed: $LIB_PATH not found${NC}"
    exit 1
fi

# Print library info
echo -e "${GREEN}✅ Build successful!${NC}"
echo ""
echo "📊 Library Info:"
if command -v file &> /dev/null; then
    file "$LIB_PATH"
fi
if command -v lipo &> /dev/null; then
    lipo -info "$LIB_PATH" 2>/dev/null || true
fi
if command -v wc &> /dev/null; then
    echo "   Size: $(wc -c < "$LIB_PATH" | numfmt --to=iec 2>/dev/null || wc -c < "$LIB_PATH")"
fi

echo ""
echo "📍 Library location:"
echo "   $LIB_PATH"
echo ""
echo "🚀 Next steps:"
echo "   export EASYSSH_CORE_PATH=$CORE_DIR"
echo "   swift build"

exit 0
