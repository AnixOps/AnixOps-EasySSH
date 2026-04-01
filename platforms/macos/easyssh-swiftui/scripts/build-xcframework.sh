#!/bin/bash
#
# build-xcframework.sh - Create XCFramework for App Store distribution
#
# This script creates a signed XCFramework from the Rust core library,
# suitable for App Store submission and binaryTarget in Package.swift.
#
# Usage:
#   ./scripts/build-xcframework.sh [version]
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
CORE_DIR="$ROOT_DIR/core"
SWIFT_DIR="$ROOT_DIR/platforms/macos/easyssh-swiftui"
FRAMEWORKS_DIR="$SWIFT_DIR/Frameworks"
VERSION="${1:-0.3.0}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🍎 Building XCFramework for EasySSH Core v$VERSION${NC}"
echo ""

# Check requirements
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ cargo not found${NC}"
    exit 1
fi

# Setup directories
BUILD_DIR="$CORE_DIR/target/xcframework-build"
rm -rf "$BUILD_DIR" "$FRAMEWORKS_DIR"
mkdir -p "$BUILD_DIR/headers"
mkdir -p "$FRAMEWORKS_DIR"

# Copy header
cp "$CORE_DIR/target/include/easyssh_core.h" "$BUILD_DIR/headers/"

# Build for macOS architectures
echo "📦 Building Rust core for macOS..."

cd "$CORE_DIR"

# Build for x86_64
mkdir -p "$BUILD_DIR/macos-x86_64"
echo "   - Building x86_64..."
if cargo build --target x86_64-apple-darwin --release --features standard 2>/dev/null; then
    cp "$CORE_DIR/target/x86_64-apple-darwin/release/libeasyssh_core.a" \
       "$BUILD_DIR/macos-x86_64/"
    HAS_X86_64=1
else
    echo -e "${YELLOW}   ⚠️  x86_64 target not available${NC}"
    HAS_X86_64=0
fi

# Build for arm64
mkdir -p "$BUILD_DIR/macos-arm64"
echo "   - Building arm64..."
if cargo build --target aarch64-apple-darwin --release --features standard 2>/dev/null; then
    cp "$CORE_DIR/target/aarch64-apple-darwin/release/libeasyssh_core.a" \
       "$BUILD_DIR/macos-arm64/"
    HAS_ARM64=1
else
    echo -e "${YELLOW}   ⚠️  arm64 target not available${NC}"
    HAS_ARM64=0
fi

# Build for iOS Simulator (if needed for future iOS support)
# mkdir -p "$BUILD_DIR/ios-sim-arm64"
# cargo build --target aarch64-apple-ios-sim --release --features standard || true

# Check we have at least one architecture
if [ $HAS_X86_64 -eq 0 ] && [ $HAS_ARM64 -eq 0 ]; then
    echo -e "${RED}❌ No architectures built successfully${NC}"
    exit 1
fi

# Create XCFramework
echo ""
echo "🔗 Creating XCFramework..."

XCFRAMEWORK_NAME="CEasySSHCore.xcframework"
XCFRAMEWORK_PATH="$FRAMEWORKS_DIR/$XCFRAMEWORK_NAME"

# Build xcodebuild command
XCF_CMD="xcodebuild -create-xcframework"

if [ $HAS_X86_64 -eq 1 ]; then
    XCF_CMD="$XCF_CMD -library $BUILD_DIR/macos-x86_64/libeasyssh_core.a"
    XCF_CMD="$XCF_CMD -headers $BUILD_DIR/headers"
fi

if [ $HAS_ARM64 -eq 1 ]; then
    XCF_CMD="$XCF_CMD -library $BUILD_DIR/macos-arm64/libeasyssh_core.a"
    XCF_CMD="$XCF_CMD -headers $BUILD_DIR/headers"
fi

XCF_CMD="$XCF_CMD -output $XCFRAMEWORK_PATH"

echo "   Running: xcodebuild -create-xcframework ..."
eval $XCF_CMD

# Verify
if [ ! -d "$XCFRAMEWORK_PATH" ]; then
    echo -e "${RED}❌ XCFramework creation failed${NC}"
    exit 1
fi

echo -e "${GREEN}✅ XCFramework created successfully!${NC}"
echo ""
echo "📍 Location: $XCFRAMEWORK_PATH"
echo ""
echo "📊 Framework Info:"
ls -lh "$XCFRAMEWORK_PATH/"

# Create Info.plist for version tracking
cat > "$XCFRAMEWORK_PATH/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>CEasySSHCore</string>
    <key>CFBundleIdentifier</key>
    <string>com.easyssh.core</string>
    <key>CFBundleName</key>
    <string>CEasySSHCore</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>MinimumOSVersion</key>
    <string>14.0</string>
</dict>
</plist>
EOF

echo ""
echo "📝 Update Package.swift to use binaryTarget:"
echo ""
echo "    .binaryTarget("
echo "        name: \"CEasySSHCore\","
echo "        path: \"Frameworks/$XCFRAMEWORK_NAME\""
echo "    ),"
echo ""
echo "🔒 For App Store submission, codesign the framework:"
echo "    codesign --sign 'Developer ID' $XCFRAMEWORK_PATH"

# Cleanup
rm -rf "$BUILD_DIR"

echo ""
echo -e "${GREEN}🎉 Done!${NC}"
