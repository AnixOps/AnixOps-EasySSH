#!/bin/bash
# Build EasySSH AppImage packages
# Usage: ./build-appimage.sh [version] [edition]
# Example: ./build-appimage.sh 0.3.0 lite

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
VERSION="${1:-0.3.0}"
EDITION="${2:-lite}"

APPIMAGE_DIR="${SCRIPT_DIR}"
BUILD_DIR="${PROJECT_ROOT}/target/appimage-${EDITION}"
RELEASE_DIR="${PROJECT_ROOT}/releases/v${VERSION}/linux"

echo "Building EasySSH ${EDITION} AppImage v${VERSION}..."

# Determine architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64) ARCH_SUFFIX="x86_64" ;;
    aarch64) ARCH_SUFFIX="aarch64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Edition-specific configuration
case $EDITION in
    lite)
        APP_NAME="EasySSH Lite"
        EXEC_NAME="easyssh-lite"
        DESKTOP_CATEGORIES="Network;System;Utility;"
        ;;
    standard)
        APP_NAME="EasySSH Standard"
        EXEC_NAME="easyssh-standard"
        DESKTOP_CATEGORIES="Network;System;Utility;"
        ;;
    pro)
        APP_NAME="EasySSH Pro"
        EXEC_NAME="easyssh-pro"
        DESKTOP_CATEGORIES="Network;System;Utility;"
        ;;
    *)
        echo "Unknown edition: $EDITION"
        exit 1
        ;;
esac

# Create build directory
mkdir -p "${BUILD_DIR}/AppDir/usr/bin"
mkdir -p "${BUILD_DIR}/AppDir/usr/share/applications"
mkdir -p "${BUILD_DIR}/AppDir/usr/share/icons/hicolor/256x256/apps"
mkdir -p "${BUILD_DIR}/AppDir/usr/share/metainfo"

# Copy binary
cp "${PROJECT_ROOT}/target/release-${EDITION}/${EXEC_NAME}" "${BUILD_DIR}/AppDir/usr/bin/"
chmod +x "${BUILD_DIR}/AppDir/usr/bin/${EXEC_NAME}"

# Copy icon
cp "${PROJECT_ROOT}/resources/icons/${EXEC_NAME}.png" "${BUILD_DIR}/AppDir/usr/share/icons/hicolor/256x256/apps/" 2>/dev/null || \
    cp "${PROJECT_ROOT}/design-system/assets/icon-256.png" "${BUILD_DIR}/AppDir/usr/share/icons/hicolor/256x256/apps/${EXEC_NAME}.png"

# Create .desktop file
cat > "${BUILD_DIR}/AppDir/usr/share/applications/${EXEC_NAME}.desktop" << EOF
[Desktop Entry]
Name=${APP_NAME}
Comment=Native SSH Client
Exec=${EXEC_NAME}
Icon=${EXEC_NAME}
Type=Application
Categories=${DESKTOP_CATEGORIES}
Terminal=false
StartupNotify=true
MimeType=x-scheme-handler/ssh;
EOF

# Create AppStream metadata
cat > "${BUILD_DIR}/AppDir/usr/share/metainfo/${EXEC_NAME}.appdata.xml" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>${EXEC_NAME}</id>
  <metadata_license>MIT</metadata_license>
  <project_license>MIT</project_license>
  <name>${APP_NAME}</name>
  <summary>Native SSH Client</summary>
  <description>
    <p>EasySSH is a native SSH client with team collaboration features.</p>
  </description>
  <launchable type="desktop-id">${EXEC_NAME}.desktop</launchable>
  <url type="homepage">https://github.com/anixops/easyssh</url>
  <releases>
    <release version="${VERSION}" date="$(date +%Y-%m-%d)"/>
  </releases>
</component>
EOF

# Create AppRun script
cat > "${BUILD_DIR}/AppDir/AppRun" << 'EOF'
#!/bin/bash
APPDIR="$(dirname "$(readlink -f "$0")")"
export PATH="${APPDIR}/usr/bin:${PATH}"
export XDG_DATA_DIRS="${APPDIR}/usr/share:${XDG_DATA_DIRS}"

# Set library path for bundled libraries
export LD_LIBRARY_PATH="${APPDIR}/usr/lib:${LD_LIBRARY_PATH}"

# Launch the application
exec "${APPDIR}/usr/bin/$(ls ${APPDIR}/usr/bin/ | head -1)" "$@"
EOF
chmod +x "${BUILD_DIR}/AppDir/AppRun"

# Copy desktop file to AppDir root for appimagetool
cp "${BUILD_DIR}/AppDir/usr/share/applications/${EXEC_NAME}.desktop" "${BUILD_DIR}/AppDir/"

# Download appimagetool if not present
APPIMAGETOOL="${BUILD_DIR}/appimagetool-${ARCH_SUFFIX}.AppImage"
if [ ! -f "${APPIMAGETOOL}" ]; then
    echo "Downloading appimagetool..."
    wget -q "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-${ARCH_SUFFIX}.AppImage" -O "${APPIMAGETOOL}"
    chmod +x "${APPIMAGETOOL}"
fi

# Build AppImage
echo "Building AppImage..."
mkdir -p "${RELEASE_DIR}"
cd "${BUILD_DIR}"
"${APPIMAGETOOL}" --appimage-extract-and-run AppDir "${RELEASE_DIR}/EasySSH-${EDITION}-${VERSION}-${ARCH_SUFFIX}.AppImage"

# Make AppImage executable
chmod +x "${RELEASE_DIR}/EasySSH-${EDITION}-${VERSION}-${ARCH_SUFFIX}.AppImage"

echo ""
echo "AppImage created: ${RELEASE_DIR}/EasySSH-${EDITION}-${VERSION}-${ARCH_SUFFIX}.AppImage"

# Create symlink for latest version
ln -sf "EasySSH-${EDITION}-${VERSION}-${ARCH_SUFFIX}.AppImage" "${RELEASE_DIR}/EasySSH-${EDITION}-latest-${ARCH_SUFFIX}.AppImage"
