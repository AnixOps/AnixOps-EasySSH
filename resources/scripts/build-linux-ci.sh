#!/bin/bash
# EasySSH Linux CI/CD Build Script
# Comprehensive build script for Linux with multiple package formats

set -e

# Configuration
VERSION="${VERSION:-0.3.0}"
APP_NAME="EasySSH"
APP_ID="com.anixops.easyssh"
BIN_NAME="easyssh"
DESKTOP_FILE="easyssh.desktop"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Show usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS] [TARGETS]

EasySSH Linux CI/CD Build Script

OPTIONS:
    -v, --version VERSION    Set version (default: $VERSION)
    -h, --help              Show this help message
    -j, --jobs N            Number of parallel jobs (default: auto)
    --release               Build release profile (default)
    --debug                 Build debug profile
    --clean                 Clean before build
    --install-deps          Install system dependencies

TARGETS (build all if none specified):
    binary          Build binary only
    appimage        Build AppImage package
    deb             Build Debian package
    rpm             Build RPM package
    arch            Build Arch Linux package
    flatpak         Build Flatpak package
    snap            Build Snap package
    tarball         Build generic tarball

EXAMPLES:
    $0                              # Build all formats
    $0 binary deb                   # Build binary and Debian package
    $0 --version 1.0.0 appimage     # Build AppImage with specific version
    $0 --install-deps               # Install dependencies only

EOF
}

# Parse arguments
BUILD_TARGETS=()
CLEAN=false
INSTALL_DEPS=false
PROFILE="release"
JOBS=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--version)
            VERSION="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        -j|--jobs)
            JOBS="--jobs $2"
            shift 2
            ;;
        --release)
            PROFILE="release"
            shift
            ;;
        --debug)
            PROFILE="debug"
            shift
            ;;
        --clean)
            CLEAN=true
            shift
            ;;
        --install-deps)
            INSTALL_DEPS=true
            shift
            ;;
        binary|appimage|deb|rpm|arch|flatpak|snap|tarball)
            BUILD_TARGETS+=("$1")
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# If no targets specified, build all
if [ ${#BUILD_TARGETS[@]} -eq 0 ] && [ "$INSTALL_DEPS" = false ]; then
    BUILD_TARGETS=(binary appimage deb rpm tarball)
fi

# Detect OS and architecture
detect_system() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        DISTRO_ID="$ID"
        DISTRO_NAME="$NAME"
        DISTRO_VERSION="$VERSION_ID"
    else
        DISTRO_ID="unknown"
        DISTRO_NAME="Unknown"
        DISTRO_VERSION="unknown"
    fi

    ARCH=$(uname -m)
    case $ARCH in
        x86_64) TARGET_ARCH="x86_64" ;;
        aarch64|arm64) TARGET_ARCH="aarch64" ;;
        *) log_error "Unsupported architecture: $ARCH"; exit 1 ;;
    esac

    log_info "Detected: $DISTRO_NAME $DISTRO_VERSION ($TARGET_ARCH)"
}

# Install system dependencies
install_dependencies() {
    log_info "Installing system dependencies..."

    case $DISTRO_ID in
        ubuntu|debian)
            sudo apt-get update
            sudo apt-get install -y \
                libgtk-4-dev \
                libadwaita-1-dev \
                pkg-config \
                libssl-dev \
                build-essential \
                curl \
                git \
                cmake \
                fakeroot \
                rpm \
                lintian \
                devscripts \
                debhelper \
                dh-make \
                libfuse2 \
                appstream \
                desktop-file-utils
            ;;
        fedora|rhel|centos|rocky|almalinux)
            sudo dnf install -y \
                gtk4-devel \
                libadwaita-devel \
                pkgconf-pkg-config \
                openssl-devel \
                gcc \
                gcc-c++ \
                make \
                curl \
                git \
                cmake \
                rpm-build \
                rpmdevtools \
                desktop-file-utils \
                libappstream-glib \
                fakeroot
            ;;
        arch|manjaro)
            sudo pacman -Syu --noconfirm \
                gtk4 \
                libadwaita \
                pkgconf \
                openssl \
                base-devel \
                curl \
                git \
                cmake \
                fakeroot \
                desktop-file-utils \
                appstream-glib
            ;;
        opensuse*|suse*)
            sudo zypper install -y \
                gtk4-devel \
                libadwaita-devel \
                pkgconf \
                libopenssl-devel \
                gcc \
                gcc-c++ \
                make \
                curl \
                git \
                cmake \
                rpm-build \
                desktop-file-utils \
                appstream-glib
            ;;
        *)
            log_warning "Unknown distribution. Please install GTK4 and libadwaita development packages manually."
            ;;
    esac

    # Install Rust if not present
    if ! command -v cargo &> /dev/null; then
        log_info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source $HOME/.cargo/env
    fi

    # Install cargo-deb for Debian packaging
    if [[ " ${BUILD_TARGETS[*]} " =~ " deb " ]] && ! command -v cargo-deb &> /dev/null; then
        log_info "Installing cargo-deb..."
        cargo install cargo-deb
    fi

    # Install cargo-generate-rpm for RPM packaging
    if [[ " ${BUILD_TARGETS[*]} " =~ " rpm " ]] && ! command -v cargo-generate-rpm &> /dev/null; then
        log_info "Installing cargo-generate-rpm..."
        cargo install cargo-generate-rpm
    fi

    log_success "Dependencies installed"
}

# Clean build artifacts
clean_build() {
    log_info "Cleaning build artifacts..."
    cd platforms/linux/easyssh-gtk4
    cargo clean 2>/dev/null || true
    rm -rf build/ dist/ release/ *.AppImage *.deb *.rpm *.tar.gz 2>/dev/null || true
    cd ../../..
    log_success "Clean complete"
}

# Setup build environment
setup_build() {
    log_info "Setting up build environment..."

    # Create release directories
    mkdir -p "releases/v${VERSION}/linux"
    mkdir -p "build/linux/${TARGET_ARCH}"

    # Export version for cargo
    export EASYSSH_VERSION="$VERSION"

    log_success "Build environment ready"
}

# Build binary
build_binary() {
    log_info "Building EasySSH binary (profile: $PROFILE)..."

    cd platforms/linux/easyssh-gtk4

    # Build flags
    BUILD_FLAGS=""
    if [ "$PROFILE" = "release" ]; then
        BUILD_FLAGS="--release"
    fi

    # Build with optimizations
    RUSTFLAGS="-C target-cpu=native" cargo build $BUILD_FLAGS $JOBS

    # Verify binary was created
    BINARY_PATH="target/${PROFILE}/easyssh-gtk4"
    if [ ! -f "$BINARY_PATH" ]; then
        log_error "Binary not found at $BINARY_PATH"
        exit 1
    fi

    # Strip binary for release builds
    if [ "$PROFILE" = "release" ]; then
        strip "$BINARY_PATH"
    fi

    # Copy to release directory
    mkdir -p "../../../build/linux/${TARGET_ARCH}"
    cp "$BINARY_PATH" "../../../build/linux/${TARGET_ARCH}/easyssh"

    cd ../../..

    log_success "Binary built successfully"
    log_info "Binary size: $(stat -f%z "build/linux/${TARGET_ARCH}/easyssh" 2>/dev/null || stat -c%s "build/linux/${TARGET_ARCH}/easyssh") bytes"
}

# Create desktop entry
create_desktop_entry() {
    log_info "Creating desktop entry..."

    mkdir -p "build/linux/${TARGET_ARCH}/usr/share/applications"
    mkdir -p "build/linux/${TARGET_ARCH}/usr/share/icons/hicolor/256x256/apps"
    mkdir -p "build/linux/${TARGET_ARCH}/usr/share/icons/hicolor/128x128/apps"
    mkdir -p "build/linux/${TARGET_ARCH}/usr/share/icons/hicolor/64x64/apps"
    mkdir -p "build/linux/${TARGET_ARCH}/usr/share/metainfo"

    cat > "build/linux/${TARGET_ARCH}/usr/share/applications/${DESKTOP_FILE}" << EOF
[Desktop Entry]
Name=${APP_NAME}
GenericName=SSH Client
Comment=Modern SSH client with GTK4 interface
Exec=/usr/bin/${BIN_NAME} %F
Icon=${APP_ID}
Type=Application
Categories=Network;RemoteAccess;System;Utility;
Keywords=ssh;terminal;remote;sftp;scp;
MimeType=x-scheme-handler/ssh;
Terminal=false
StartupNotify=true
StartupWMClass=easyssh
Version=1.0
X-GNOME-UsesNotifications=true
X-Purism-FormFactor=Workstation;Mobile;
EOF

    # Create AppStream metadata
    cat > "build/linux/${TARGET_ARCH}/usr/share/metainfo/${APP_ID}.metainfo.xml" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>${APP_ID}</id>
  <metadata_license>MIT</metadata_license>
  <project_license>MIT</project_license>
  <name>${APP_NAME}</name>
  <summary>Modern SSH client for Linux</summary>
  <description>
    <p>
      EasySSH is a modern SSH client with a clean GTK4 interface.
      Features include secure connection management, SFTP file browser,
      terminal emulator, and key authentication.
    </p>
    <p>Key Features:</p>
    <ul>
      <li>Secure SSH connection management</li>
      <li>Built-in SFTP file browser</li>
      <li>Terminal emulator with tabs</li>
      <li>SSH key authentication</li>
      <li>Connection grouping and organization</li>
      <li>Import from ~/.ssh/config</li>
    </ul>
  </description>
  <screenshots>
    <screenshot type="default">
      <caption>Main window showing connection list</caption>
    </screenshot>
  </screenshots>
  <url type="homepage">https://github.com/anixops/easyssh</url>
  <url type="bugtracker">https://github.com/anixops/easyssh/issues</url>
  <developer_name>AnixOps</developer_name>
  <content_rating type="oars-1.1" />
  <releases>
    <release version="${VERSION}" date="$(date +%Y-%m-%d)">
      <description>
        <p>Release ${VERSION}</p>
      </description>
    </release>
  </releases>
</component>
EOF

    # Create placeholder icons (should be replaced with actual icons)
    touch "build/linux/${TARGET_ARCH}/usr/share/icons/hicolor/256x256/apps/${APP_ID}.png"
    touch "build/linux/${TARGET_ARCH}/usr/share/icons/hicolor/128x128/apps/${APP_ID}.png"
    touch "build/linux/${TARGET_ARCH}/usr/share/icons/hicolor/64x64/apps/${APP_ID}.png"

    log_success "Desktop entry created"
}

# Build AppImage
build_appimage() {
    log_info "Building AppImage..."

    # Ensure binary is built
    if [ ! -f "build/linux/${TARGET_ARCH}/easyssh" ]; then
        build_binary
    fi

    # Install appimagetool if needed
    if ! command -v appimagetool &> /dev/null; then
        log_info "Installing appimagetool..."
        if ! command -v appimagetool-x86_64.AppImage &> /dev/null; then
            curl -L -o /tmp/appimagetool "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
            chmod +x /tmp/appimagetool
            sudo mv /tmp/appimagetool /usr/local/bin/
        fi
        APPIMAGETOOL="/usr/local/bin/appimagetool"
    else
        APPIMAGETOOL="appimagetool"
    fi

    # Create AppDir structure
    APPDIR="build/linux/${TARGET_ARCH}/AppDir"
    mkdir -p "${APPDIR}/usr/bin"
    mkdir -p "${APPDIR}/usr/share/applications"
    mkdir -p "${APPDIR}/usr/share/icons/hicolor/256x256/apps"
    mkdir -p "${APPDIR}/usr/share/metainfo"

    # Copy binary
    cp "build/linux/${TARGET_ARCH}/easyssh" "${APPDIR}/usr/bin/"

    # Copy desktop entry and metadata
    cp "build/linux/${TARGET_ARCH}/usr/share/applications/${DESKTOP_FILE}" "${APPDIR}/usr/share/applications/"
    cp "build/linux/${TARGET_ARCH}/usr/share/metainfo/${APP_ID}.metainfo.xml" "${APPDIR}/usr/share/metainfo/"

    # Create AppRun script
    cat > "${APPDIR}/AppRun" << 'EOF'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH}"
exec "${HERE}/usr/bin/easyssh" "$@"
EOF
    chmod +x "${APPDIR}/AppRun"

    # Create icon symlink
    ln -s "usr/share/applications/${DESKTOP_FILE}" "${APPDIR}/${APP_ID}.desktop"

    # Build AppImage
    export ARCH="$TARGET_ARCH"
    if [ "$TARGET_ARCH" = "x86_64" ]; then
        export ARCH="x86_64"
    elif [ "$TARGET_ARCH" = "aarch64" ]; then
        export ARCH="aarch64"
    fi

    # Build the AppImage
    $APPIMAGETOOL "${APPDIR}" "releases/v${VERSION}/linux/${APP_NAME}-${VERSION}-${ARCH}.AppImage" 2>&1 || {
        log_warning "AppImage build may have issues, trying alternative method..."
        # Fallback: create a simple tarball that can be extracted and run
        tar -czf "releases/v${VERSION}/linux/${APP_NAME}-${VERSION}-${ARCH}-portable.tar.gz" -C "build/linux/${TARGET_ARCH}" easyssh usr/
    }

    if [ -f "releases/v${VERSION}/linux/${APP_NAME}-${VERSION}-${ARCH}.AppImage" ]; then
        chmod +x "releases/v${VERSION}/linux/${APP_NAME}-${VERSION}-${ARCH}.AppImage"
        log_success "AppImage built: releases/v${VERSION}/linux/${APP_NAME}-${VERSION}-${ARCH}.AppImage"
    else
        log_warning "AppImage creation failed, portable tarball created instead"
    fi
}

# Build Debian package
build_deb() {
    log_info "Building Debian package..."

    # Check for cargo-deb
    if ! command -v cargo-deb &> /dev/null; then
        log_info "Installing cargo-deb..."
        cargo install cargo-deb
    fi

    cd platforms/linux/easyssh-gtk4

    # Create Debian package using cargo-deb
    cargo deb --variant release \
        --output "../../../releases/v${VERSION}/linux/${APP_NAME,,}_${VERSION}_${TARGET_ARCH}.deb" \
        --deb-version "${VERSION}" \
        --deb-maintainer "EasySSH Team <team@anixops.com>" \
        --deb-description "Modern SSH client with GTK4 interface" \
        --deb-extended-description "EasySSH provides secure SSH connection management with an elegant GTK4 interface. Features include SFTP browser, terminal emulator, and key authentication." \
        --deb-section "net" \
        --deb-priority "optional" \
        --deb-depends "libgtk-4-1 (>= 4.12), libadwaita-1-0 (>= 1.5), libssl3" \
        --license-file ../../../LICENSE \
        2>&1 || {
        log_warning "cargo-deb failed, trying manual dpkg-deb..."
        build_deb_manual
        return
    }

    cd ../../..

    if [ -f "releases/v${VERSION}/linux/${APP_NAME,,}_${VERSION}_${TARGET_ARCH}.deb" ]; then
        log_success "Debian package built successfully"
    else
        log_error "Debian package build failed"
        return 1
    fi
}

# Manual Debian package build (fallback)
build_deb_manual() {
    log_info "Building Debian package manually..."

    local PKGDIR="../../../build/linux/${TARGET_ARCH}/deb-package"
    mkdir -p "${PKGDIR}/DEBIAN"
    mkdir -p "${PKGDIR}/usr/bin"
    mkdir -p "${PKGDIR}/usr/share/applications"
    mkdir -p "${PKGDIR}/usr/share/icons/hicolor/256x256/apps"

    # Copy files
    cp "../../../build/linux/${TARGET_ARCH}/easyssh" "${PKGDIR}/usr/bin/"
    cp "../../../build/linux/${TARGET_ARCH}/usr/share/applications/${DESKTOP_FILE}" "${PKGDIR}/usr/share/applications/"

    # Create control file
    cat > "${PKGDIR}/DEBIAN/control" << EOF
Package: ${APP_NAME,,}
Version: ${VERSION}
Section: net
Priority: optional
Architecture: ${TARGET_ARCH}
Depends: libgtk-4-1 (>= 4.12), libadwaita-1-0 (>= 1.5), libssl3
Maintainer: EasySSH Team <team@anixops.com>
Description: Modern SSH client with GTK4 interface
 EasySSH provides secure SSH connection management with an elegant GTK4 interface.
 Features include SFTP browser, terminal emulator, and key authentication.
EOF

    # Build package
    dpkg-deb --build "${PKGDIR}" "../../../releases/v${VERSION}/linux/${APP_NAME,,}_${VERSION}_${TARGET_ARCH}.deb"

    cd ../../..
}

# Build RPM package
build_rpm() {
    log_info "Building RPM package..."

    # Check for cargo-generate-rpm
    if ! command -v cargo-generate-rpm &> /dev/null; then
        log_info "Installing cargo-generate-rpm..."
        cargo install cargo-generate-rpm
    fi

    cd platforms/linux/easyssh-gtk4

    # Create RPM spec file
    mkdir -p "../../../build/linux/${TARGET_ARCH}/rpmbuild/SPECS"
    mkdir -p "../../../build/linux/${TARGET_ARCH}/rpmbuild/BUILD"
    mkdir -p "../../../build/linux/${TARGET_ARCH}/rpmbuild/RPMS"
    mkdir -p "../../../build/linux/${TARGET_ARCH}/rpmbuild/SOURCES"

    cat > "../../../build/linux/${TARGET_ARCH}/rpmbuild/SPECS/easyssh.spec" << EOF
Name:           ${BIN_NAME}
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        Modern SSH client with GTK4 interface

License:        MIT
URL:            https://github.com/anixops/easyssh
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  gtk4-devel >= 4.12
BuildRequires:  libadwaita-devel >= 1.5
BuildRequires:  pkgconfig
BuildRequires:  openssl-devel

Requires:       gtk4 >= 4.12
Requires:       libadwaita >= 1.5
Requires:       openssl

%description
EasySSH provides secure SSH connection management with an elegant GTK4 interface.
Features include SFTP browser, terminal emulator, and key authentication.

%prep
%setup -q

%build
# Binary already built

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/share/applications
mkdir -p %{buildroot}/usr/share/icons/hicolor/256x256/apps
mkdir -p %{buildroot}/usr/share/metainfo

cp %{_builddir}/easyssh %{buildroot}/usr/bin/
cp %{_builddir}/${DESKTOP_FILE} %{buildroot}/usr/share/applications/
cp %{_builddir}/${APP_ID}.metainfo.xml %{buildroot}/usr/share/metainfo/ 2>/dev/null || true

%files
/usr/bin/easyssh
/usr/share/applications/${DESKTOP_FILE}
/usr/share/metainfo/${APP_ID}.metainfo.xml

%changelog
* $(date +"%a %b %d %Y") EasySSH Team <team@anixops.com> - ${VERSION}-1
- Release ${VERSION}
EOF

    # Create tarball for RPM
    tar -czf "../../../build/linux/${TARGET_ARCH}/rpmbuild/SOURCES/${BIN_NAME}-${VERSION}.tar.gz" \
        -C "../../../build/linux/${TARGET_ARCH}" \
        easyssh usr/share/applications/${DESKTOP_FILE} usr/share/metainfo/${APP_ID}.metainfo.xml 2>/dev/null || true

    # Build RPM
    rpmbuild --define "_topdir $(pwd)/../../../build/linux/${TARGET_ARCH}/rpmbuild" \
        -bb "../../../build/linux/${TARGET_ARCH}/rpmbuild/SPECS/easyssh.spec" 2>&1 || {
        log_error "RPM build failed"
        cd ../../..
        return 1
    }

    # Copy RPM to releases
    find "../../../build/linux/${TARGET_ARCH}/rpmbuild/RPMS" -name "*.rpm" -exec cp {} "../../../releases/v${VERSION}/linux/" \;

    cd ../../..

    if ls "releases/v${VERSION}/linux/"*.rpm &> /dev/null; then
        log_success "RPM package built successfully"
    else
        log_error "RPM package build failed"
        return 1
    fi
}

# Build Arch Linux package
build_arch() {
    log_info "Building Arch Linux package..."

    mkdir -p "build/linux/${TARGET_ARCH}/arch"

    # Create PKGBUILD
    cat > "build/linux/${TARGET_ARCH}/arch/PKGBUILD" << EOF
# Maintainer: EasySSH Team <team@anixops.com>
pkgname=easyssh
pkgver=${VERSION}
pkgrel=1
pkgdesc="Modern SSH client with GTK4 interface"
arch=(${TARGET_ARCH})
url="https://github.com/anixops/easyssh"
license=('MIT')
depends=('gtk4>=4.12' 'libadwaita>=1.5' 'openssl')
makedepends=('cargo' 'rust')
source=("\$pkgname-\$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "\$srcdir"
    cargo build --release
}

package() {
    cd "\$srcdir"
    install -Dm755 "easyssh" "\$pkgdir/usr/bin/easyssh"
    install -Dm644 "${DESKTOP_FILE}" "\$pkgdir/usr/share/applications/${DESKTOP_FILE}"
    install -Dm644 "${APP_ID}.metainfo.xml" "\$pkgdir/usr/share/metainfo/${APP_ID}.metainfo.xml"
}
EOF

    # Create source tarball
    tar -czf "build/linux/${TARGET_ARCH}/arch/easyssh-${VERSION}.tar.gz" \
        -C "build/linux/${TARGET_ARCH}" \
        easyssh usr/share/applications/${DESKTOP_FILE} usr/share/metainfo/${APP_ID}.metainfo.xml 2>/dev/null || true

    cd "build/linux/${TARGET_ARCH}/arch"

    # Build package
    makepkg -f 2>&1 || {
        log_error "Arch package build failed"
        cd ../../../..
        return 1
    }

    # Copy to releases
    cp *.pkg.tar.zst "../../../releases/v${VERSION}/linux/" 2>/dev/null || true

    cd ../../../..

    if ls "releases/v${VERSION}/linux/"*.pkg.tar.zst &> /dev/null; then
        log_success "Arch package built successfully"
    else
        log_warning "Arch package build may have failed"
    fi
}

# Build tarball
build_tarball() {
    log_info "Building generic tarball..."

    # Ensure binary is built
    if [ ! -f "build/linux/${TARGET_ARCH}/easyssh" ]; then
        build_binary
    fi

    # Create install script
    cat > "build/linux/${TARGET_ARCH}/install.sh" << 'EOF'
#!/bin/bash
# EasySSH Installer

set -e

PREFIX="${PREFIX:-/usr/local}"

echo "Installing EasySSH to $PREFIX..."

# Install binary
install -Dm755 easyssh "$PREFIX/bin/easyssh"

# Install desktop entry
if [ -d "usr/share/applications" ]; then
    install -Dm644 usr/share/applications/easyssh.desktop "/usr/share/applications/easyssh.desktop"
fi

# Install metadata
if [ -d "usr/share/metainfo" ]; then
    install -Dm644 usr/share/metainfo/com.anixops.easyssh.metainfo.xml "/usr/share/metainfo/com.anixops.easyssh.metainfo.xml"
fi

echo "EasySSH installed successfully!"
echo "Run 'easyssh' to start the application."
EOF
    chmod +x "build/linux/${TARGET_ARCH}/install.sh"

    # Create uninstall script
    cat > "build/linux/${TARGET_ARCH}/uninstall.sh" << 'EOF'
#!/bin/bash
# EasySSH Uninstaller

set -e

PREFIX="${PREFIX:-/usr/local}"

echo "Uninstalling EasySSH from $PREFIX..."

rm -f "$PREFIX/bin/easyssh"
rm -f "/usr/share/applications/easyssh.desktop"
rm -f "/usr/share/metainfo/com.anixops.easyssh.metainfo.xml"

echo "EasySSH uninstalled."
EOF
    chmod +x "build/linux/${TARGET_ARCH}/uninstall.sh"

    # Create tarball
    tar -czf "releases/v${VERSION}/linux/${APP_NAME}-${VERSION}-linux-${TARGET_ARCH}.tar.gz" \
        -C "build/linux/${TARGET_ARCH}" \
        easyssh install.sh uninstall.sh usr/

    log_success "Tarball created: releases/v${VERSION}/linux/${APP_NAME}-${VERSION}-linux-${TARGET_ARCH}.tar.gz"
}

# Generate checksums
generate_checksums() {
    log_info "Generating checksums..."

    cd "releases/v${VERSION}/linux"

    # Create SHA256 checksums
    sha256sum * > SHA256SUMS

    cd ../../..

    log_success "Checksums generated"
}

# Print summary
print_summary() {
    echo ""
    echo "========================================"
    log_success "Linux Build Complete"
    echo "========================================"
    echo ""
    echo "Version: $VERSION"
    echo "Architecture: $TARGET_ARCH"
    echo ""
    echo "Output directory: releases/v${VERSION}/linux/"
    echo ""
    echo "Generated packages:"
    ls -lh "releases/v${VERSION}/linux/" 2>/dev/null || echo "No packages found"
    echo ""
    echo "To install:"
    echo "  - AppImage: Run the .AppImage file directly"
    echo "  - Debian/Ubuntu: sudo dpkg -i *.deb"
    echo "  - Fedora/RHEL: sudo rpm -i *.rpm"
    echo "  - Generic: tar -xzf *.tar.gz && cd easyssh-* && sudo ./install.sh"
    echo ""
}

# Main function
main() {
    echo "========================================"
    echo "EasySSH Linux CI/CD Build Script"
    echo "========================================"
    echo ""

    detect_system

    if [ "$INSTALL_DEPS" = true ]; then
        install_dependencies
        exit 0
    fi

    if [ "$CLEAN" = true ]; then
        clean_build
    fi

    setup_build

    # Install dependencies if needed
    if [ ${#BUILD_TARGETS[@]} -gt 0 ]; then
        install_dependencies
    fi

    # Build requested targets
    for target in "${BUILD_TARGETS[@]}"; do
        echo ""
        log_info "Building target: $target"
        echo "----------------------------------------"

        case $target in
            binary)
                build_binary
                ;;
            appimage)
                build_appimage
                ;;
            deb)
                build_deb
                ;;
            rpm)
                build_rpm
                ;;
            arch)
                build_arch
                ;;
            tarball)
                build_tarball
                ;;
            *)
                log_error "Unknown target: $target"
                ;;
        esac
    done

    # Generate checksums
    if [ ${#BUILD_TARGETS[@]} -gt 0 ]; then
        generate_checksums
        print_summary
    fi
}

# Run main function
main
