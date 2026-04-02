#!/bin/bash
# EasySSH Windows Installer Build Script
# Creates both MSI (WiX) and EXE (NSIS) installers
#
# Usage:
#   ./build-windows-installer.sh [version] [source_dir]
#   ./build-windows-installer.sh 0.3.0 ../../target/release
#
# Prerequisites:
#   - WiX Toolset (candle.exe, light.exe)
#   - NSIS (makensis.exe)
#   - Visual Studio Build Tools (for code signing)
#   - Windows SDK (for signtool.exe)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Arguments
VERSION="${1:-0.3.0}"
SOURCE_DIR="${2:-../../target/release}"
INSTALLER_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${INSTALLER_DIR}/../.." && pwd)"
OUTPUT_DIR="${PROJECT_ROOT}/releases/v${VERSION}/windows"

# Code signing configuration
SIGN_CERT="${SIGN_CERT:-}"
SIGN_CERT_PASSWORD="${SIGN_CERT_PASSWORD:-}"
SIGN_TIMESTAMP_URL="${SIGN_TIMESTAMP_URL:-http://timestamp.digicert.com}"

# Utility functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_step "Checking prerequisites..."

    local missing=()

    # Check for WiX
    if ! command -v candle.exe &> /dev/null && ! [ -f "/c/Program Files (x86)/WiX Toolset v3.11/bin/candle.exe" ]; then
        missing+=("WiX Toolset")
    fi

    # Check for NSIS
    if ! command -v makensis.exe &> /dev/null && ! [ -f "/c/Program Files (x86)/NSIS/makensis.exe" ]; then
        missing+=("NSIS")
    fi

    # Check for source files
    if [ ! -f "${SOURCE_DIR}/EasySSH.exe" ]; then
        log_error "Source binary not found: ${SOURCE_DIR}/EasySSH.exe"
        log_info "Please build the release binary first:"
        log_info "  cd platforms/windows/easyssh-winui"
        log_info "  cargo build --release"
        exit 1
    fi

    if [ ${#missing[@]} -gt 0 ]; then
        log_error "Missing prerequisites: ${missing[*]}"
        log_info "Please install:"
        log_info "  1. WiX Toolset v3.11+ from https://wixtoolset.org/"
        log_info "  2. NSIS 3.0+ from https://nsis.sourceforge.io/"
        exit 1
    fi

    # Create output directory
    mkdir -p "${OUTPUT_DIR}"

    log_info "Prerequisites check passed"
}

# Prepare installer resources
prepare_resources() {
    log_step "Preparing installer resources..."

    local resources_dir="${INSTALLER_DIR}/resources"
    local wix_dir="${INSTALLER_DIR}/../wix"
    local nsis_dir="${INSTALLER_DIR}/../nsis"

    mkdir -p "${resources_dir}"

    # Copy required files to resources directory
    cp "${SOURCE_DIR}/EasySSH.exe" "${resources_dir}/"
    cp "${PROJECT_ROOT}/core/icons/icon.ico" "${resources_dir}/"
    cp "${PROJECT_ROOT}/LICENSE" "${resources_dir}/LICENSE.txt"

    # Generate README
    cat > "${resources_dir}/README.txt" << EOF
EasySSH v${VERSION}
=================

Quick Start:
1. Run EasySSH.exe
2. Add your SSH servers via the UI
3. Connect using password or key authentication

System Requirements:
- Windows 10/11 64-bit
- No additional dependencies required

For support, visit: https://github.com/anixops/easyssh
EOF

    # Create bitmap files for NSIS (placeholders - real images should be created)
    log_warn "Creating placeholder bitmaps for installer UI"
    log_warn "For production, create real images:"
    log_warn "  - header.bmp (150x57 pixels)"
    log_warn "  - welcome.bmp (164x314 pixels)"
    log_warn "  - banner.bmp (493x58 pixels)"
    log_warn "  - dialog.bmp (493x312 pixels)"
    log_warn "  - bg.bmp (493x312 pixels)"

    # Copy resources to WiX and NSIS directories
    cp "${resources_dir}/"* "${wix_dir}/" 2>/dev/null || true
    mkdir -p "${nsis_dir}/images"

    log_info "Resources prepared in ${resources_dir}"
}

# Sign a file
sign_file() {
    local file="$1"
    local description="$2"

    if [ -z "${SIGN_CERT}" ]; then
        log_warn "Skipping code signing (no certificate configured)"
        return 0
    fi

    if [ ! -f "${SIGN_CERT}" ]; then
        log_warn "Certificate not found: ${SIGN_CERT}"
        return 0
    fi

    log_step "Signing ${file}..."

    local signtool="/c/Program Files (x86)/Windows Kits/10/bin/10.0.19041.0/x64/signtool.exe"
    if [ ! -f "${signtool}" ]; then
        # Try alternative paths
        signtool=$(find "/c/Program Files (x86)/Windows Kits" -name "signtool.exe" 2>/dev/null | head -1)
    fi

    if [ -z "${signtool}" ] || [ ! -f "${signtool}" ]; then
        log_warn "signtool.exe not found, skipping signing"
        return 0
    fi

    # Sign with SHA256
    if [ -n "${SIGN_CERT_PASSWORD}" ]; then
        "${signtool}" sign \
            /f "${SIGN_CERT}" \
            /p "${SIGN_CERT_PASSWORD}" \
            /tr "${SIGN_TIMESTAMP_URL}" \
            /td sha256 \
            /fd sha256 \
            /d "${description}" \
            "${file}"
    else
        "${signtool}" sign \
            /f "${SIGN_CERT}" \
            /tr "${SIGN_TIMESTAMP_URL}" \
            /td sha256 \
            /fd sha256 \
            /d "${description}" \
            "${file}"
    fi

    # Verify signature
    "${signtool}" verify /pa "${file}"

    log_info "Successfully signed: ${file}"
}

# Build WiX MSI installer
build_wix_installer() {
    log_step "Building WiX MSI installer..."

    local wix_dir="${INSTALLER_DIR}/../wix"
    local build_dir="${wix_dir}/build"
    local msi_name="EasySSH-${VERSION}-x64.msi"

    mkdir -p "${build_dir}"

    # Set WiX path
    local wix_bin="/c/Program Files (x86)/WiX Toolset v3.11/bin"
    if [ ! -d "${wix_bin}" ]; then
        wix_bin="/c/Program Files/WiX Toolset v3.11/bin"
    fi

    # Compile
    log_info "Compiling WiX source..."
    "${wix_bin}/candle.exe" \
        -arch x64 \
        -dVersion="${VERSION}" \
        -dSourceDir="${wix_dir}" \
        -out "${build_dir}/" \
        "${wix_dir}/EasySSH.wxs"

    # Link
    log_info "Linking MSI..."
    "${wix_bin}/light.exe" \
        -ext WixUIExtension \
        -ext WixUtilExtension \
        -cultures:en-US \
        -dWixUILicenseRtf="${wix_dir}/LICENSE.rtf" \
        -out "${build_dir}/${msi_name}" \
        "${build_dir}/EasySSH.wixobj"

    # Sign MSI
    sign_file "${build_dir}/${msi_name}" "EasySSH Installer"

    # Copy to output
    cp "${build_dir}/${msi_name}" "${OUTPUT_DIR}/"

    log_info "MSI installer created: ${OUTPUT_DIR}/${msi_name}"
}

# Build NSIS installer
build_nsis_installer() {
    log_step "Building NSIS installer..."

    local nsis_dir="${INSTALLER_DIR}/../nsis"
    local exe_name="EasySSH-${VERSION}-x64.exe"

    # Set NSIS path
    local nsis_bin="/c/Program Files (x86)/NSIS"
    if [ ! -d "${nsis_bin}" ]; then
        nsis_bin="/c/Program Files/NSIS"
    fi

    # Build installer
    log_info "Compiling NSIS script..."
    "${nsis_bin}/makensis.exe" \
        /DPRODUCT_VERSION="${VERSION}" \
        /DOUTPUT_NAME="${exe_name}" \
        "${nsis_dir}/easyssh.nsi"

    # The NSIS script outputs to the current directory
    local exe_path="${nsis_dir}/${exe_name}"

    # Sign EXE
    sign_file "${exe_path}" "EasySSH Setup"

    # Copy to output
    cp "${exe_path}" "${OUTPUT_DIR}/"

    log_info "NSIS installer created: ${OUTPUT_DIR}/${exe_name}"
}

# Create portable ZIP package
create_portable_package() {
    log_step "Creating portable package..."

    local pkg_dir="${OUTPUT_DIR}/EasySSH-${VERSION}-portable"
    local zip_name="EasySSH-${VERSION}-windows-x64-portable.zip"

    mkdir -p "${pkg_dir}"

    # Copy files
    cp "${SOURCE_DIR}/EasySSH.exe" "${pkg_dir}/"
    cp "${PROJECT_ROOT}/core/icons/icon.ico" "${pkg_dir}/"
    cp "${PROJECT_ROOT}/LICENSE" "${pkg_dir}/LICENSE.txt"

    # Create portable README
    cat > "${pkg_dir}/README.txt" << EOF
EasySSH v${VERSION} Portable
===========================

This is a portable version of EasySSH.
No installation required.

Quick Start:
1. Run EasySSH.exe directly
2. Your data will be stored in:
   %LOCALAPPDATA%\AnixOps\EasySSH

System Requirements:
- Windows 10/11 64-bit
- No additional dependencies required

For support, visit: https://github.com/anixops/easyssh
EOF

    # Create batch launcher
    cat > "${pkg_dir}/EasySSH.bat" << 'EOF'
@echo off
start "" "%~dp0EasySSH.exe"
EOF

    # Create ZIP
    cd "${OUTPUT_DIR}"
    zip -r "${zip_name}" "EasySSH-${VERSION}-portable"
    cd - > /dev/null

    # Cleanup
    rm -rf "${pkg_dir}"

    log_info "Portable package created: ${OUTPUT_DIR}/${zip_name}"
}

# Generate checksums
generate_checksums() {
    log_step "Generating checksums..."

    local checksum_file="${OUTPUT_DIR}/SHA256SUMS.txt"

    echo "EasySSH v${VERSION} Windows Installer Checksums" > "${checksum_file}"
    echo "===============================================" >> "${checksum_file}"
    echo "" >> "${checksum_file}"
    echo "Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")" >> "${checksum_file}"
    echo "" >> "${checksum_file}"

    for file in "${OUTPUT_DIR}"/*.{msi,exe,zip}; do
        if [ -f "$file" ]; then
            local filename=$(basename "$file")
            local hash=$(sha256sum "$file" | cut -d' ' -f1)
            echo "${hash}  ${filename}" >> "${checksum_file}"
        fi
    done

    log_info "Checksums written to: ${checksum_file}"
}

# Create installation notes
create_installation_notes() {
    log_step "Creating installation notes..."

    cat > "${OUTPUT_DIR}/INSTALL.md" << EOF
# EasySSH v${VERSION} for Windows - Installation Guide

## Installation Options

### Option 1: MSI Installer (Recommended)
Best for enterprise deployments and automated installations.

\`\`\`powershell
# Silent installation
msiexec /i EasySSH-${VERSION}-x64.msi /qn INSTALLDESKTOPSHORTCUT=1

# Silent uninstallation
msiexec /x EasySSH-${VERSION}-x64.msi /qn
\`\`\`

**Features:**
- Windows Add/Remove Programs integration
- Automatic upgrade handling
- Start Menu shortcuts
- Optional desktop shortcut
- Registry entries for file associations

### Option 2: NSIS Installer
Best for standard users with interactive installation.

\`\`\`powershell
# Run installer
EasySSH-${VERSION}-x64.exe

# Silent installation
EasySSH-${VERSION}-x64.exe /S

# Silent uninstallation
%LOCALAPPDATA%\Programs\EasySSH\uninstall.exe /S
\`\`\`

**Features:**
- User-friendly wizard interface
- Multiple language support
- Component selection
- Automatic previous version removal

### Option 3: Portable ZIP
Best for USB drives or systems without installation privileges.

1. Extract \`EasySSH-${VERSION}-windows-x64-portable.zip\`
2. Run \`EasySSH.exe\`

**Note:** Settings are stored in \`%LOCALAPPDATA%\\AnixOps\\EasySSH\`

## System Requirements

- Windows 10 (version 1607) or Windows 11
- 64-bit processor
- 100 MB free disk space
- Internet connection (for automatic updates)

## Code Signing

Official EasySSH installers are signed with:
- **Certificate:** AnixOps Code Signing Certificate
- **Thumbprint:** (See release notes)

To verify signature:
\`\`\`powershell
Get-AuthenticodeSignature EasySSH-${VERSION}-x64.exe
\`\`\`

## Silent Installation Parameters

### MSI Parameters
| Parameter | Description | Default |
|-----------|-------------|---------|
| INSTALLDIR | Installation directory | \\`%LOCALAPPDATA%\\Programs\\EasySSH\\` |
| INSTALLDESKTOPSHORTCUT | Create desktop shortcut | 0 |
| ALLUSERS | Install for all users | 2 (per-user) |

### NSIS Parameters
| Parameter | Description |
|-----------|-------------|
| /S | Silent mode |
| /D=path | Installation directory |

## Uninstallation

### MSI
\`\`\`powershell
msiexec /x {ProductCode} /qn
# or
msiexec /x EasySSH-${VERSION}-x64.msi /qn
\`\`\`

### NSIS
\`\`\`powershell
%LOCALAPPDATA%\Programs\EasySSH\uninstall.exe /S
\`\`\`

## Troubleshooting

### SmartScreen Warning
If Windows SmartScreen shows a warning:
1. Click "More info"
2. Click "Run anyway"
3. This happens because the certificate is new or not yet widely distributed

### Antivirus Detection
Some antivirus software may flag new executables. If this occurs:
1. Add an exclusion for EasySSH.exe
2. Report false positive to your antivirus vendor

### Installation Logs
MSI logs are created in \`%TEMP%\\MSI*.log\`

## Support

- GitHub Issues: https://github.com/anixops/easyssh/issues
- Documentation: https://docs.anixops.com/easyssh
EOF

    log_info "Installation notes created: ${OUTPUT_DIR}/INSTALL.md"
}

# Main function
main() {
    echo "=========================================="
    echo "EasySSH Windows Installer Build Script"
    echo "Version: ${VERSION}"
    echo "Source: ${SOURCE_DIR}"
    echo "Output: ${OUTPUT_DIR}"
    echo "=========================================="
    echo ""

    check_prerequisites
    prepare_resources

    # Build installers
    build_wix_installer
    build_nsis_installer
    create_portable_package

    # Generate artifacts
    generate_checksums
    create_installation_notes

    echo ""
    echo "=========================================="
    echo "Build Complete!"
    echo "=========================================="
    echo ""
    echo "Output files:"
    ls -lh "${OUTPUT_DIR}"/*.{msi,exe,zip,txt,md} 2>/dev/null || true
    echo ""
    echo "Next steps:"
    echo "  1. Test installers on clean Windows VMs"
    echo "  2. Verify digital signatures (if configured)"
    echo "  3. Upload to GitHub releases"
    echo ""
}

# Show help
show_help() {
    cat << EOF
EasySSH Windows Installer Build Script

Usage: $0 [version] [source_dir]

Arguments:
  version      Application version (default: 0.3.0)
  source_dir   Directory containing EasySSH.exe (default: ../../target/release)

Environment Variables:
  SIGN_CERT              Path to code signing certificate (.pfx)
  SIGN_CERT_PASSWORD     Certificate password
  SIGN_TIMESTAMP_URL     Timestamp server URL (default: http://timestamp.digicert.com)

Examples:
  $0                           # Build with defaults
  $0 0.4.0                     # Build version 0.4.0
  $0 0.3.0 ./custom/release    # Build from custom directory

Prerequisites:
  - WiX Toolset v3.11+
  - NSIS 3.0+
  - Windows SDK (for signtool)
  - Built EasySSH.exe binary

EOF
}

# Handle arguments
if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
    show_help
    exit 0
fi

# Run main
main
