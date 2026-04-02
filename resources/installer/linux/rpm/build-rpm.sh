#!/bin/bash
# Build EasySSH .rpm packages
# Usage: ./build-rpm.sh [version] [edition]
# Example: ./build-rpm.sh 0.3.0 lite

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
VERSION="${1:-0.3.0}"
EDITION="${2:-lite}"

echo "Building EasySSH ${EDITION} .rpm package v${VERSION}..."

# Edition-specific configuration
case $EDITION in
    lite)
        PKG_NAME="easyssh-lite"
        APP_NAME="EasySSH Lite"
        EXEC_NAME="easyssh-lite"
        DESCRIPTION="Minimal SSH configuration vault with native terminal launcher"
        REQUIRES="gtk4 libadwaita libsecret"
        ;;
    standard)
        PKG_NAME="easyssh-standard"
        APP_NAME="EasySSH Standard"
        EXEC_NAME="easyssh-standard"
        DESCRIPTION="Full-featured SSH client with embedded terminal"
        REQUIRES="gtk4 libadwaita webkit2gtk4.1 libsecret sqlite"
        ;;
    pro)
        PKG_NAME="easyssh-pro"
        APP_NAME="EasySSH Pro"
        EXEC_NAME="easyssh-pro"
        DESCRIPTION="Enterprise SSH client with team collaboration"
        REQUIRES="gtk4 libadwaita webkit2gtk4.1 libsecret sqlite openssl"
        ;;
    *)
        echo "Unknown edition: $EDITION"
        exit 1
        ;;
esac

# Determine architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64) RPM_ARCH="x86_64" ;;
    aarch64) RPM_ARCH="aarch64" ;;
    armv7l) RPM_ARCH="armv7hl" ;;
    *) RPM_ARCH="$ARCH" ;;
esac

BUILD_DIR="${PROJECT_ROOT}/target/rpm-${EDITION}"
RELEASE_DIR="${PROJECT_ROOT}/releases/v${VERSION}/linux"
RPMBUILD_DIR="${BUILD_DIR}/rpmbuild"

# Create rpmbuild structure
mkdir -p "${RPMBUILD_DIR}"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
mkdir -p "${RPMBUILD_DIR}/BUILDROOT/${PKG_NAME}-${VERSION}-1.${RPM_ARCH}"

# Create source tarball
echo "Creating source tarball..."
SRC_DIR="${RPMBUILD_DIR}/SOURCES/${PKG_NAME}-${VERSION}"
mkdir -p "${SRC_DIR}"
cp -r "${PROJECT_ROOT}/crates" "${SRC_DIR}/"
cp "${PROJECT_ROOT}/Cargo.toml" "${SRC_DIR}/"
cp "${PROJECT_ROOT}/Cargo.lock" "${SRC_DIR}/"
cp "${PROJECT_ROOT}/LICENSE" "${SRC_DIR}/"

cd "${RPMBUILD_DIR}/SOURCES"
tar czf "${PKG_NAME}-${VERSION}.tar.gz" "${PKG_NAME}-${VERSION}"
rm -rf "${PKG_NAME}-${VERSION}"

# Create RPM spec file
cat > "${RPMBUILD_DIR}/SPECS/${PKG_NAME}.spec" << EOF
Name:           ${PKG_NAME}
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        ${DESCRIPTION}
License:        MIT
URL:            https://github.com/anixops/easyssh
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.75
BuildRequires:  cargo
BuildRequires:  gtk4-devel
BuildRequires:  libadwaita-devel
BuildRequires:  openssl-devel
BuildRequires:  sqlite-devel
BuildRequires:  libsecret-devel

Requires:       ${REQUIRES}

%description
${APP_NAME} is a native SSH client built with Rust.

Features include:
- Secure credential storage using system keyring
- Native terminal integration
- Server organization and grouping
- SSH key management

%prep
%setup -q

%build
cargo build --release --profile release-${EDITION}

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/share/applications
mkdir -p %{buildroot}/usr/share/icons/hicolor/256x256/apps
mkdir -p %{buildroot}/usr/share/metainfo
mkdir -p %{buildroot}/usr/share/doc/%{name}

# Install binary
cp target/release-${EDITION}/${EXEC_NAME} %{buildroot}/usr/bin/

# Install icon
cp design-system/assets/icon-256.png %{buildroot}/usr/share/icons/hicolor/256x256/apps/${EXEC_NAME}.png

# Install desktop file
cat > %{buildroot}/usr/share/applications/${EXEC_NAME}.desktop << DESKTOP
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
DESKTOP

# Install AppStream metadata
cat > %{buildroot}/usr/share/metainfo/%{name}.appdata.xml << METADATA
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>%{name}</id>
  <metadata_license>MIT</metadata_license>
  <project_license>MIT</project_license>
  <name>${APP_NAME}</name>
  <summary>${DESCRIPTION}</summary>
  <description>
    <p>EasySSH is a native SSH client built with Rust.</p>
  </description>
  <launchable type="desktop-id">${EXEC_NAME}.desktop</launchable>
  <url type="homepage">https://github.com/anixops/easyssh</url>
  <releases>
    <release version="%{version}" date="$(date +%Y-%m-%d)"/>
  </releases>
  <content_rating type="oars-1.1" />
</component>
METADATA

# Install documentation
cp LICENSE %{buildroot}/usr/share/doc/%{name}/

%files
%license LICENSE
%doc README.md
/usr/bin/${EXEC_NAME}
/usr/share/applications/${EXEC_NAME}.desktop
/usr/share/icons/hicolor/256x256/apps/${EXEC_NAME}.png
/usr/share/metainfo/%{name}.appdata.xml

%post
touch --no-create /usr/share/icons/hicolor || :
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache /usr/share/icons/hicolor || :
fi
update-desktop-database /usr/share/applications || :

%postun
if [ \$1 -eq 0 ]; then
    touch --no-create /usr/share/icons/hicolor || :
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache /usr/share/icons/hicolor || :
    fi
    update-desktop-database /usr/share/applications || :
fi

%changelog
* $(date +"%a %b %d %Y") EasySSH Team <team@anixops.com> - ${VERSION}-1
- Release version ${VERSION}
EOF

# Build the RPM
echo "Building RPM package..."
mkdir -p "${RELEASE_DIR}"

rpmbuild --define "_topdir ${RPMBUILD_DIR}" \
         -bb "${RPMBUILD_DIR}/SPECS/${PKG_NAME}.spec"

# Copy RPM to release directory
cp "${RPMBUILD_DIR}/RPMS/${RPM_ARCH}/${PKG_NAME}-${VERSION}-1.${RPM_ARCH}.rpm" \
   "${RELEASE_DIR}/"

echo ""
echo ".rpm package created: ${RELEASE_DIR}/${PKG_NAME}-${VERSION}-1.${RPM_ARCH}.rpm"

# Generate checksum
cd "${RELEASE_DIR}"
sha256sum "${PKG_NAME}-${VERSION}-1.${RPM_ARCH}.rpm" > "${PKG_NAME}-${VERSION}-1.${RPM_ARCH}.rpm.sha256"

# Run rpmlint if available
if command -v rpmlint >/dev/null 2>&1; then
    echo ""
    echo "Running rpmlint checks..."
    rpmlint "${RELEASE_DIR}/${PKG_NAME}-${VERSION}-1.${RPM_ARCH}.rpm" || true
fi
