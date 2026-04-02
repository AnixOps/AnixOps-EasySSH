#!/bin/bash
# Build EasySSH .deb packages
# Usage: ./build-deb.sh [version] [edition]
# Example: ./build-deb.sh 0.3.0 lite

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
VERSION="${1:-0.3.0}"
EDITION="${2:-lite}"

echo "Building EasySSH ${EDITION} .deb package v${VERSION}..."

# Edition-specific configuration
case $EDITION in
    lite)
        PKG_NAME="easyssh-lite"
        APP_NAME="EasySSH Lite"
        EXEC_NAME="easyssh-lite"
        DESCRIPTION="Minimal SSH configuration vault with native terminal launcher"
        DEPENDS="libgtk-4-1, libadwaita-1-0, libkeyring1"
        ;;
    standard)
        PKG_NAME="easyssh-standard"
        APP_NAME="EasySSH Standard"
        EXEC_NAME="easyssh-standard"
        DESCRIPTION="Full-featured SSH client with embedded terminal"
        DEPENDS="libgtk-4-1, libadwaita-1-0, libwebkit2gtk-4.1-0, libkeyring1, libsqlite3-0"
        ;;
    pro)
        PKG_NAME="easyssh-pro"
        APP_NAME="EasySSH Pro"
        EXEC_NAME="easyssh-pro"
        DESCRIPTION="Enterprise SSH client with team collaboration"
        DEPENDS="libgtk-4-1, libadwaita-1-0, libwebkit2gtk-4.1-0, libkeyring1, libsqlite3-0, libssl3"
        ;;
    *)
        echo "Unknown edition: $EDITION"
        exit 1
        ;;
esac

# Determine architecture
ARCH=$(dpkg --print-architecture 2>/dev/null || echo "amd64")

BUILD_DIR="${PROJECT_ROOT}/target/deb-${EDITION}"
RELEASE_DIR="${PROJECT_ROOT}/releases/v${VERSION}/linux"

# Create package structure
PKG_ROOT="${BUILD_DIR}/${PKG_NAME}_${VERSION}_${ARCH}"
mkdir -p "${PKG_ROOT}/DEBIAN"
mkdir -p "${PKG_ROOT}/usr/bin"
mkdir -p "${PKG_ROOT}/usr/share/applications"
mkdir -p "${PKG_ROOT}/usr/share/icons/hicolor/256x256/apps"
mkdir -p "${PKG_ROOT}/usr/share/doc/${PKG_NAME}"
mkdir -p "${PKG_ROOT}/usr/share/metainfo"

# Copy binary
cp "${PROJECT_ROOT}/target/release-${EDITION}/${EXEC_NAME}" "${PKG_ROOT}/usr/bin/"
chmod 755 "${PKG_ROOT}/usr/bin/${EXEC_NAME}"

# Copy icon
cp "${PROJECT_ROOT}/resources/icons/${EXEC_NAME}.png" "${PKG_ROOT}/usr/share/icons/hicolor/256x256/apps/" 2>/dev/null || \
    cp "${PROJECT_ROOT}/design-system/assets/icon-256.png" "${PKG_ROOT}/usr/share/icons/hicolor/256x256/apps/${EXEC_NAME}.png"

# Create .desktop file
cat > "${PKG_ROOT}/usr/share/applications/${EXEC_NAME}.desktop" << EOF
[Desktop Entry]
Name=${APP_NAME}
GenericName=SSH Client
Comment=${DESCRIPTION}
Exec=${EXEC_NAME}
Icon=${EXEC_NAME}
Terminal=false
Type=Application
Categories=Network;System;Utility;RemoteAccess;
Keywords=ssh;terminal;remote;server;
StartupNotify=true
MimeType=x-scheme-handler/ssh;
EOF

# Create control file
cat > "${PKG_ROOT}/DEBIAN/control" << EOF
Package: ${PKG_NAME}
Version: ${VERSION}
Section: net
Priority: optional
Architecture: ${ARCH}
Depends: ${DEPENDS}
Recommends: gnome-terminal | konsole | xterm
Maintainer: EasySSH Team <team@anixops.com>
Description: ${APP_NAME}
 ${DESCRIPTION}
 .
 EasySSH is a native SSH client built with Rust and GTK4.
 Features include:
  - Secure credential storage using system keyring
  - Native terminal integration
  - Server organization and grouping
  - SSH key management
EOF

# Create postinst script
cat > "${PKG_ROOT}/DEBIAN/postinst" << 'EOF'
#!/bin/bash
set -e

# Update desktop database
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications
fi

# Update icon cache
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f /usr/share/icons/hicolor
fi

# Register SSH protocol handler
xdg-mime default $(basename $0).desktop x-scheme-handler/ssh 2>/dev/null || true

exit 0
EOF
chmod 755 "${PKG_ROOT}/DEBIAN/postinst"

# Create prerm script
cat > "${PKG_ROOT}/DEBIAN/prerm" << 'EOF'
#!/bin/bash
set -e

# Cleanup will be handled by dpkg

exit 0
EOF
chmod 755 "${PKG_ROOT}/DEBIAN/prerm"

# Create postrm script
cat > "${PKG_ROOT}/DEBIAN/postrm" << 'EOF'
#!/bin/bash
set -e

# Update desktop database
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications
fi

# Update icon cache
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f /usr/share/icons/hicolor
fi

exit 0
EOF
chmod 755 "${PKG_ROOT}/DEBIAN/postrm"

# Copy documentation
cp "${PROJECT_ROOT}/LICENSE" "${PKG_ROOT}/usr/share/doc/${PKG_NAME}/copyright"
cat > "${PKG_ROOT}/usr/share/doc/${PKG_NAME}/changelog" << EOF
easyssh (${VERSION}) unstable; urgency=medium

  * Release version ${VERSION}

 -- EasySSH Team <team@anixops.com>  $(date -R)
EOF
gzip -9 -n "${PKG_ROOT}/usr/share/doc/${PKG_NAME}/changelog"

# Create AppStream metadata
cat > "${PKG_ROOT}/usr/share/metainfo/${PKG_NAME}.appdata.xml" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>${PKG_NAME}</id>
  <metadata_license>MIT</metadata_license>
  <project_license>MIT</project_license>
  <name>${APP_NAME}</name>
  <summary>${DESCRIPTION}</summary>
  <description>
    <p>EasySSH is a native SSH client built with Rust.</p>
    <p>Features include:</p>
    <ul>
      <li>Secure credential storage using system keyring</li>
      <li>Native terminal integration</li>
      <li>Server organization and grouping</li>
      <li>SSH key management</li>
    </ul>
  </description>
  <launchable type="desktop-id">${EXEC_NAME}.desktop</launchable>
  <url type="homepage">https://github.com/anixops/easyssh</url>
  <url type="bugtracker">https://github.com/anixops/easyssh/issues</url>
  <releases>
    <release version="${VERSION}" date="$(date +%Y-%m-%d)"/>
  </releases>
  <content_rating type="oars-1.1" />
</component>
EOF

# Build the package
echo "Building .deb package..."
mkdir -p "${RELEASE_DIR}"
dpkg-deb --build "${PKG_ROOT}" "${RELEASE_DIR}/${PKG_NAME}_${VERSION}_${ARCH}.deb"

# Lint the package if lintian is available
if command -v lintian >/dev/null 2>&1; then
    echo "Running lintian checks..."
    lintian "${RELEASE_DIR}/${PKG_NAME}_${VERSION}_${ARCH}.deb" || true
fi

echo ""
echo ".deb package created: ${RELEASE_DIR}/${PKG_NAME}_${VERSION}_${ARCH}.deb"

# Generate checksum
cd "${RELEASE_DIR}"
sha256sum "${PKG_NAME}_${VERSION}_${ARCH}.deb" > "${PKG_NAME}_${VERSION}_${ARCH}.deb.sha256"
