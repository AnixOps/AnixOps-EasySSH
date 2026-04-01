#!/bin/bash
# EasySSH macOS Build Script
# Run this on a macOS machine with Xcode installed

set -e

VERSION="0.3.0"
echo "Building EasySSH v${VERSION} for macOS..."

# Build the core library first
cargo build --release -p easyssh-core

# Build Swift package
cd platforms/macos/EasySSH
swift build -c release

# Create app bundle
cd ../../..
mkdir -p "releases/v${VERSION}/macos/EasySSH-${VERSION}-macos-universal/EasySSH.app/Contents/MacOS"
mkdir -p "releases/v${VERSION}/macos/EasySSH-${VERSION}-macos-universal/EasySSH.app/Contents/Resources"

# Copy binary
cp platforms/macos/EasySSH/.build/release/EasySSH "releases/v${VERSION}/macos/EasySSH-${VERSION}-macos-universal/EasySSH.app/Contents/MacOS/"

# Create Info.plist
cat > "releases/v${VERSION}/macos/EasySSH-${VERSION}-macos-universal/EasySSH.app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>EasySSH</string>
    <key>CFBundleIdentifier</key>
    <string>com.anixops.easyssh</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>EasySSH</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
</dict>
</plist>
EOF

# Sign the app (ad-hoc signing for distribution outside App Store)
codesign --force --deep --sign - "releases/v${VERSION}/macos/EasySSH-${VERSION}-macos-universal/EasySSH.app"

# Create DMG
cd "releases/v${VERSION}/macos"

# Create temporary DMG layout
mkdir -p temp_dmg
cp -r "EasySSH-${VERSION}-macos-universal/EasySSH.app" temp_dmg/
ln -s /Applications temp_dmg/Applications

# Create DMG
hdiutil create -volname "EasySSH Installer" -srcfolder temp_dmg -ov -format UDZO "EasySSH-${VERSION}-macos-universal.dmg"

# Clean up
rm -rf temp_dmg

echo "macOS build complete: EasySSH-${VERSION}-macos-universal.dmg"
