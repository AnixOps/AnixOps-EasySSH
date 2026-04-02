#!/bin/bash
# Create EasySSH DMG installers for macOS
# Usage: ./create-dmg.sh [version] [edition]
# Example: ./create-dmg.sh 0.3.0 lite

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
VERSION="${1:-0.3.0}"
EDITION="${2:-lite}"

echo "Creating EasySSH ${EDITION} DMG v${VERSION}..."

# Edition-specific configuration
case $EDITION in
    lite)
        APP_NAME="EasySSH Lite"
        BUNDLE_ID="com.anixops.easyssh.lite"
        EXEC_NAME="easyssh-lite"
        ;;
    standard)
        APP_NAME="EasySSH Standard"
        BUNDLE_ID="com.anixops.easyssh.standard"
        EXEC_NAME="easyssh-standard"
        ;;
    pro)
        APP_NAME="EasySSH Pro"
        BUNDLE_ID="com.anixops.easyssh.pro"
        EXEC_NAME="easyssh-pro"
        ;;
    *)
        echo "Unknown edition: $EDITION"
        exit 1
        ;;
esac

BUILD_DIR="${PROJECT_ROOT}/target/macos-${EDITION}"
RELEASE_DIR="${PROJECT_ROOT}/releases/v${VERSION}/macos"
APP_BUNDLE="${BUILD_DIR}/${APP_NAME}.app"

# Create app bundle structure
mkdir -p "${APP_BUNDLE}/Contents/MacOS"
mkdir -p "${APP_BUNDLE}/Contents/Resources"
mkdir -p "${APP_BUNDLE}/Contents/Frameworks"

# Copy executable
cp "${PROJECT_ROOT}/target/release-${EDITION}/${EXEC_NAME}" "${APP_BUNDLE}/Contents/MacOS/${EXEC_NAME}"
chmod +x "${APP_BUNDLE}/Contents/MacOS/${EXEC_NAME}"

# Copy icon
cp "${PROJECT_ROOT}/resources/icons/${EXEC_NAME}.icns" "${APP_BUNDLE}/Contents/Resources/AppIcon.icns" 2>/dev/null || \
    cp "${PROJECT_ROOT}/design-system/assets/icon.icns" "${APP_BUNDLE}/Contents/Resources/AppIcon.icns"

# Create Info.plist
cat > "${APP_BUNDLE}/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>${EXEC_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSUIElement</key>
    <false/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2026 AnixOps. All rights reserved.</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>MacOSX</string>
    </array>
</dict>
</plist>
EOF

# Create PkgInfo
echo "APPL????" > "${APP_BUNDLE}/Contents/PkgInfo"

# Sign the app bundle if certificates are available
if [ -n "$APPLE_DEVELOPER_ID" ] && [ -n "$APPLE_APP_PASSWORD" ]; then
    echo "Signing app bundle..."
    codesign --force --options runtime --sign "$APPLE_DEVELOPER_ID" \
        --entitlements "${SCRIPT_DIR}/entitlements.plist" \
        "${APP_BUNDLE}"
fi

# Create DMG staging directory
DMG_STAGING="${BUILD_DIR}/dmg-staging"
mkdir -p "${DMG_STAGING}"

# Copy app bundle
cp -R "${APP_BUNDLE}" "${DMG_STAGING}/"

# Create Applications symlink
ln -s /Applications "${DMG_STAGING}/Applications"

# Copy background image if exists
if [ -f "${PROJECT_ROOT}/resources/installer/macos/dmg/background.tiff" ]; then
    mkdir -p "${DMG_STAGING}/.background"
    cp "${PROJECT_ROOT}/resources/installer/macos/dmg/background.tiff" "${DMG_STAGING}/.background/"
fi

# Create DS_Store for custom layout (if create-dmg is available)
if command -v create-dmg >/dev/null 2>&1; then
    echo "Using create-dmg..."
    mkdir -p "${RELEASE_DIR}"

    create-dmg \
        --volname "${APP_NAME} ${VERSION}" \
        --window-pos 200 120 \
        --window-size 800 400 \
        --icon-size 100 \
        --icon "${APP_NAME}.app" 200 200 \
        --icon "Applications" 600 200 \
        --hide-extension "${APP_NAME}.app" \
        --app-drop-link 600 200 \
        --no-internet-enable \
        "${RELEASE_DIR}/EasySSH-${EDITION}-${VERSION}.dmg" \
        "${DMG_STAGING}"
else
    echo "create-dmg not found, using hdiutil..."

    # Create temporary DMG
    TEMP_DMG="${BUILD_DIR}/temp.dmg"
    hdiutil create -srcfolder "${DMG_STAGING}" -volname "${APP_NAME} ${VERSION}" \
        -fs HFS+ -format UDRW -size 100m "${TEMP_DMG}"

    # Mount the DMG
    MOUNT_POINT="/Volumes/${APP_NAME} ${VERSION}"
    hdiutil attach "${TEMP_DMG}" -nobrowse -noverify

    # Set window properties using AppleScript (optional, for nicer layout)
    osascript << EOT 2>/dev/null || true
        tell application "Finder"
            tell disk "${APP_NAME} ${VERSION}"
                open
                set current view of container window to icon view
                set toolbar visible of container window to false
                set statusbar visible of container window to false
                set bounds of container window to {200, 120, 800, 520}
                set viewOptions to icon view options of container window
                set arrangement of viewOptions to not arranged
                set icon size of viewOptions to 100
                set position of item "${APP_NAME}.app" of container window to {200, 200}
                set position of item "Applications" of container window to {600, 200}
                update without registering applications
                delay 2
                close
            end tell
        end tell
EOT

    # Unmount
    hdiutil detach "${MOUNT_POINT}" -force || hdiutil detach "${MOUNT_POINT}"

    # Convert to compressed read-only DMG
    mkdir -p "${RELEASE_DIR}"
    hdiutil convert "${TEMP_DMG}" -format UDZO -o "${RELEASE_DIR}/EasySSH-${EDITION}-${VERSION}.dmg"

    # Clean up
    rm -f "${TEMP_DMG}"
fi

# Notarize if credentials available
if [ -n "$APPLE_DEVELOPER_ID" ] && [ -n "$APPLE_APP_PASSWORD" ] && [ -n "$APPLE_TEAM_ID" ]; then
    echo "Notarizing DMG..."

    # Submit for notarization
    xcrun notarytool submit "${RELEASE_DIR}/EasySSH-${EDITION}-${VERSION}.dmg" \
        --apple-id "$APPLE_DEVELOPER_ID" \
        --password "$APPLE_APP_PASSWORD" \
        --team-id "$APPLE_TEAM_ID" \
        --wait

    # Staple the notarization ticket
    xcrun stapler staple "${RELEASE_DIR}/EasySSH-${EDITION}-${VERSION}.dmg"
fi

echo ""
echo "DMG created: ${RELEASE_DIR}/EasySSH-${EDITION}-${VERSION}.dmg"

# Generate checksum
cd "${RELEASE_DIR}"
shasum -a 256 "EasySSH-${EDITION}-${VERSION}.dmg" > "EasySSH-${EDITION}-${VERSION}.dmg.sha256"
