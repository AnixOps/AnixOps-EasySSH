#!/bin/bash
# Build Windows Installers for all EasySSH versions
# Usage: ./build-windows.sh [version]
# Example: ./build-windows.sh 0.3.0

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
INSTALLER_DIR="${PROJECT_ROOT}/resources/installer/windows"
RELEASE_DIR="${PROJECT_ROOT}/releases"

VERSION="${1:-0.3.0}"
echo "Building EasySSH Windows installers v${VERSION}..."

# Check prerequisites
check_prerequisites() {
    local missing=()

    if ! command -v candle.exe &> /dev/null && ! command -v "${WIX}bin/candle.exe" &> /dev/null; then
        missing+=("WiX Toolset (candle.exe)")
    fi

    if ! command -v makensis.exe &> /dev/null; then
        missing+=("NSIS (makensis.exe)")
    fi

    if [ ${#missing[@]} -ne 0 ]; then
        echo "Error: Missing prerequisites:"
        printf '  - %s\n' "${missing[@]}"
        echo ""
        echo "Please install:"
        echo "  - WiX Toolset v3.11+: https://wixtoolset.org/releases/"
        echo "  - NSIS 3.0+: https://nsis.sourceforge.io/Download"
        exit 1
    fi

    # Determine WiX paths
    if [ -n "$WIX" ]; then
        WIX_CANDLE="${WIX}bin/candle.exe"
        WIX_LIGHT="${WIX}bin/light.exe"
    else
        WIX_CANDLE="candle.exe"
        WIX_LIGHT="light.exe"
    fi
}

# Build WiX MSI for a version
build_wix_msi() {
    local version_name=$1
    local upgrade_code=$2
    local source_dir=$3
    local wxs_file=$4

    echo ""
    echo "Building WiX MSI for ${version_name}..."

    local build_dir="${INSTALLER_DIR}/wix/${version_name}/build"
    mkdir -p "${build_dir}"

    # Compile
    echo "  Compiling WiX source..."
    "${WIX_CANDLE}" -arch x64 \
        -dVersion="${VERSION}" \
        -dSourceDir="${source_dir}" \
        -out "${build_dir}/" \
        "${wxs_file}"

    # Link
    echo "  Linking MSI..."
    "${WIX_LIGHT}" -ext WixUIExtension \
        -out "${RELEASE_DIR}/v${VERSION}/windows/EasySSH-${version_name}-${VERSION}-x64.msi" \
        "${build_dir}/$(basename "${wxs_file}" .wxs).wixobj"

    echo "  MSI created: EasySSH-${version_name}-${VERSION}-x64.msi"
}

# Build NSIS installer for a version
build_nsis() {
    local version_name=$1
    local nsi_file=$2

    echo ""
    echo "Building NSIS installer for ${version_name}..."

    cd "$(dirname "${nsi_file}")"
    makensis.exe /DPRODUCT_VERSION="${VERSION}" "$(basename "${nsi_file}")"

    # Move output to releases
    mv "EasySSH-${version_name}-${VERSION}-x64.exe" "${RELEASE_DIR}/v${VERSION}/windows/"

    echo "  NSIS installer created: EasySSH-${version_name}-${VERSION}-x64.exe"
}

# Create portable ZIP
create_portable() {
    local version_name=$1
    local source_dir=$2
    local exe_name=$3

    echo ""
    echo "Creating portable ZIP for ${version_name}..."

    local portable_dir="${RELEASE_DIR}/v${VERSION}/windows/portable-${version_name}"
    mkdir -p "${portable_dir}"

    # Copy files
    cp "${source_dir}/${exe_name}" "${portable_dir}/"
    cp "${source_dir}/icon.ico" "${portable_dir}/" 2>/dev/null || true
    cp "${PROJECT_ROOT}/LICENSE" "${portable_dir}/"

    # Create README
    cat > "${portable_dir}/README.txt" << EOF
EasySSH ${version_name} v${VERSION} (Portable)
============================================

This is a portable version of EasySSH ${version_name}.
No installation required - just extract and run.

Usage:
1. Extract this ZIP to any location (e.g., USB drive)
2. Run ${exe_name}
3. Your data is stored in the same directory

Note: Some features may require administrator privileges.

For support, visit: https://github.com/anixops/easyssh
EOF

    # Create ZIP
    cd "${RELEASE_DIR}/v${VERSION}/windows"
    7z a -tzip "EasySSH-${version_name}-${VERSION}-portable.zip" "portable-${version_name}/"
    rm -rf "portable-${version_name}"

    echo "  Portable ZIP created: EasySSH-${version_name}-${VERSION}-portable.zip"
}

# Generate checksums
generate_checksums() {
    echo ""
    echo "Generating checksums..."

    cd "${RELEASE_DIR}/v${VERSION}/windows"

    sha256sum EasySSH-*.msi EasySSH-*.exe EasySSH-*-portable.zip > SHA256SUMS.txt

    echo "  Checksums saved to SHA256SUMS.txt"
}

# Code signing (if configured)
sign_binaries() {
    if [ -z "$SIGN_CERT" ] || [ -z "$SIGN_CERT_PASSWORD" ]; then
        echo ""
        echo "Warning: Code signing not configured. Set SIGN_CERT and SIGN_CERT_PASSWORD."
        return
    fi

    echo ""
    echo "Signing binaries..."

    local timestamp_url="${SIGN_TIMESTAMP_URL:-http://timestamp.digicert.com}"

    for file in "${RELEASE_DIR}/v${VERSION}/windows"/*.exe "${RELEASE_DIR}/v${VERSION}/windows"/*.msi; do
        if [ -f "$file" ]; then
            echo "  Signing $(basename "$file")..."
            osslsigncode sign \
                -pkcs12 "$SIGN_CERT" \
                -pass "$SIGN_CERT_PASSWORD" \
                -n "EasySSH" \
                -i "https://anixops.com" \
                -t "$timestamp_url" \
                -in "$file" \
                -out "${file}.signed"
            mv "${file}.signed" "$file"
        fi
    done

    echo "  All binaries signed."
}

# Main build process
main() {
    echo "========================================"
    echo "EasySSH Windows Installer Build"
    echo "Version: ${VERSION}"
    echo "========================================"

    check_prerequisites

    # Create release directory
    mkdir -p "${RELEASE_DIR}/v${VERSION}/windows"

    # Build Lite
    if [ -d "${PROJECT_ROOT}/target/release-lite" ]; then
        build_wix_msi "lite" "A8C3D4E5-F6B7-8901-2345-6789ABCDEF01" \
            "${PROJECT_ROOT}/target/release-lite" \
            "${INSTALLER_DIR}/wix/lite/EasySSH-Lite.wxs"
        build_nsis "Lite" "${INSTALLER_DIR}/nsis/easyssh-lite.nsi"
        create_portable "lite" "${PROJECT_ROOT}/target/release-lite" "easyssh-lite.exe"
    else
        echo "Warning: Lite build not found, skipping..."
    fi

    # Build Standard
    if [ -d "${PROJECT_ROOT}/target/release-standard" ]; then
        build_wix_msi "standard" "A8C3D4E5-F6B7-8901-2345-6789ABCDEF02" \
            "${PROJECT_ROOT}/target/release-standard" \
            "${INSTALLER_DIR}/wix/standard/EasySSH-Standard.wxs"
        build_nsis "Standard" "${INSTALLER_DIR}/nsis/easyssh-standard.nsi"
        create_portable "standard" "${PROJECT_ROOT}/target/release-standard" "easyssh-standard.exe"
    else
        echo "Warning: Standard build not found, skipping..."
    fi

    # Build Pro
    if [ -d "${PROJECT_ROOT}/target/release-pro" ]; then
        build_wix_msi "pro" "A8C3D4E5-F6B7-8901-2345-6789ABCDEF03" \
            "${PROJECT_ROOT}/target/release-pro" \
            "${INSTALLER_DIR}/wix/pro/EasySSH-Pro.wxs"
        build_nsis "Pro" "${INSTALLER_DIR}/nsis/easyssh-pro.nsi"
        create_portable "pro" "${PROJECT_ROOT}/target/release-pro" "easyssh-pro.exe"
    else
        echo "Warning: Pro build not found, skipping..."
    fi

    # Sign binaries
    sign_binaries

    # Generate checksums
    generate_checksums

    echo ""
    echo "========================================"
    echo "Build Complete!"
    echo "========================================"
    echo ""
    echo "Output directory: ${RELEASE_DIR}/v${VERSION}/windows/"
    echo ""
    ls -lh "${RELEASE_DIR}/v${VERSION}/windows/"
}

main "$@"
