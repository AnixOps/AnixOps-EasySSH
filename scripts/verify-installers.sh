#!/bin/bash
# Installer configuration verification script

set -e

echo "=========================================="
echo "EasySSH Windows Installer Verification"
echo "=========================================="
echo ""

INSTALLER_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../installer" && pwd)"
ERRORS=0

# Check file exists
check_file() {
    if [ -f "$1" ]; then
        echo "✓ $1"
        return 0
    else
        echo "✗ $1 (missing)"
        ((ERRORS++))
        return 1
    fi
}

# Check directory exists
check_dir() {
    if [ -d "$1" ]; then
        echo "✓ $1/"
        return 0
    else
        echo "✗ $1/ (missing)"
        ((ERRORS++))
        return 1
    fi
}

echo "Checking directory structure..."
check_dir "$INSTALLER_DIR/wix"
check_dir "$INSTALLER_DIR/nsis"

echo ""
echo "Checking WiX files..."
check_file "$INSTALLER_DIR/wix/EasySSH.wxs"
check_file "$INSTALLER_DIR/wix/LICENSE.rtf"

echo ""
echo "Checking NSIS files..."
check_file "$INSTALLER_DIR/nsis/easyssh.nsi"

echo ""
echo "Checking configuration..."
check_file "$INSTALLER_DIR/installer.config.json"
check_file "$INSTALLER_DIR/README.md"
check_file "$INSTALLER_DIR/CODE_SIGNING.md"

echo ""
echo "Checking build scripts..."
check_file "$(dirname "$INSTALLER_DIR")/scripts/build-windows-installer.sh"
check_file "$(dirname "$INSTALLER_DIR")/scripts/build-windows-installer.bat"
check_file "$(dirname "$INSTALLER_DIR")/scripts/release-windows.sh"

echo ""
echo "Validating JSON configuration..."
if command -v jq &> /dev/null; then
    if jq empty "$INSTALLER_DIR/installer.config.json" 2>/dev/null; then
        echo "✓ installer.config.json is valid JSON"
    else
        echo "✗ installer.config.json has invalid JSON"
        ((ERRORS++))
    fi
else
    echo "⚠ jq not installed, skipping JSON validation"
fi

echo ""
echo "=========================================="
if [ $ERRORS -eq 0 ]; then
    echo "✓ All checks passed!"
    echo "=========================================="
    exit 0
else
    echo "✗ Found $ERRORS issue(s)"
    echo "=========================================="
    exit 1
fi
