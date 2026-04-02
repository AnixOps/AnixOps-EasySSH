#!/bin/bash
# EasySSH macOS DMG Packaging Script
# Creates a professional DMG installer with custom styling

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default configuration
VERSION="${VERSION:-0.3.0}"
APP_NAME="${APP_NAME:-EasySSH}"
SOURCE_APP="${SOURCE_APP:-release/EasySSH.app}"
OUTPUT_DIR="${OUTPUT_DIR:-./release}"
VOLUME_NAME="${VOLUME_NAME:-EasySSH Installer}"
DMG_NAME="${DMG_NAME:-EasySSH-${VERSION}-macos-universal.dmg}"

# Visual configuration
WINDOW_WIDTH=600
WINDOW_HEIGHT=400
ICON_SIZE=100
APP_ICON_POS_X=150
APP_ICON_POS_Y=185
APPS_LINK_POS_X=450
APPS_LINK_POS_Y=185

# Signing
CODESIGN_IDENTITY="${CODESIGN_IDENTITY:--}"

# Logging
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    for tool in hdiutil codesign; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "Required tool '$tool' not found"
            exit 1
        fi
    done

    if [[ ! -d "$SOURCE_APP" ]]; then
        log_error "Source app not found: $SOURCE_APP"
        exit 1
    fi

    log_success "Prerequisites satisfied"
}

# Create temporary directory for DMG layout
setup_temp_dir() {
    TEMP_DIR=$(mktemp -d)
    log_info "Created temp directory: $TEMP_DIR"

    # Copy app bundle
    cp -R "$SOURCE_APP" "$TEMP_DIR/"

    # Create Applications symlink
    ln -s /Applications "$TEMP_DIR/Applications"

    echo "$TEMP_DIR"
}

# Create background image directory
setup_background() {
    local temp_dir="$1"

    # Create .background directory
    mkdir -p "$temp_dir/.background"

    # Check for custom background
    local bg_paths=(
        "./Resources/dmg-background.png"
        "./assets/dmg-background.png"
        "../Resources/dmg-background.png"
        "../../assets/dmg-background.png"
    )

    for bg_path in "${bg_paths[@]}"; do
        if [[ -f "$bg_path" ]]; then
            log_info "Using custom background: $bg_path"
            cp "$bg_path" "$temp_dir/.background/background.png"
            return 0
        fi
    done

    # Create simple background if none provided
    log_info "No custom background found, creating default..."

    # Try to create a simple background using sips if available
    if command -v sips &> /dev/null && command -v osascript &> /dev/null; then
        # Create a simple colored background image
        osascript << EOF > /dev/null 2>&1
            tell application "Finder"
                set desktopPath to POSIX file "$temp_dir/.background/background.png"
            end tell
EOF
    fi

    return 0
}

# Create the DMG using hdiutil
create_dmg_hdiutil() {
    local temp_dir="$1"
    local output_path="$OUTPUT_DIR/$DMG_NAME"

    log_info "Creating DMG with hdiutil..."

    # Create temporary DMG (uncompressed)
    local temp_dmg="$TEMP_DIR/temp.dmg"

    hdiutil create \
        -srcfolder "$temp_dir" \
        -volname "$VOLUME_NAME" \
        -fs HFS+ \
        -format UDRW \
        -size $(($(du -sm "$temp_dir" | cut -f1) + 50))m \
        "$temp_dmg"

    # Mount the DMG
    local device
    device=$(hdiutil attach -readwrite -noverify "$temp_dmg" | grep -o '/dev/disk[0-9]*' | head -1)
    local mount_point
    mount_point=$(df | grep "$device" | awk '{print $NF}')

    log_info "Mounted at: $mount_point"

    # Set window properties using AppleScript
    if command -v osascript &> /dev/null; then
        log_info "Configuring DMG appearance..."

        osascript << EOF
            tell application "Finder"
                tell disk "$VOLUME_NAME"
                    open
                    set current view of container window to icon view
                    set toolbar visible of container window to false
                    set statusbar visible of container window to false
                    set the bounds of container window to {100, 100, $(($WINDOW_WIDTH + 100)), $(($WINDOW_HEIGHT + 100))}
                    set theViewOptions to the icon view options of container window
                    set arrangement of theViewOptions to not arranged
                    set icon size of theViewOptions to $ICON_SIZE
                    set position of item "${APP_NAME}.app" of container window to {$APP_ICON_POS_X, $APP_ICON_POS_Y}
                    set position of item "Applications" of container window to {$APPS_LINK_POS_X, $APPS_LINK_POS_Y}
                    update without registering applications
                    delay 2
                    close
                end tell
            end tell
EOF
    fi

    # Unmount
    hdiutil detach "$device" -force

    # Compress the DMG
    log_info "Compressing DMG..."
    hdiutil convert "$temp_dmg" \
        -format UDZO \
        -imagekey zlib-level=9 \
        -ov \
        -o "$output_path"

    # Remove temporary DMG
    rm -f "$temp_dmg"

    log_success "DMG created: $output_path"
}

# Create DMG using create-dmg tool (if available)
create_dmg_create_dmg() {
    local temp_dir="$1"
    local output_path="$OUTPUT_DIR/$DMG_NAME"

    if ! command -v create-dmg &> /dev/null; then
        return 1
    fi

    log_info "Using create-dmg tool..."

    create-dmg \
        --volname "$VOLUME_NAME" \
        --window-pos 200 120 \
        --window-size $WINDOW_WIDTH $WINDOW_HEIGHT \
        --icon-size $ICON_SIZE \
        --app-drop-link $APPS_LINK_POS_X $APPS_LINK_POS_Y \
        --icon "${APP_NAME}.app" $APP_ICON_POS_X $APP_ICON_POS_Y \
        --hide-extension "${APP_NAME}.app" \
        "$output_path" \
        "$temp_dir"

    log_success "DMG created: $output_path"
    return 0
}

# Sign the DMG
sign_dmg() {
    local dmg_path="$OUTPUT_DIR/$DMG_NAME"

    if [[ "$CODESIGN_IDENTITY" == "-" ]]; then
        log_warn "Skipping DMG signing (ad-hoc)"
        return 0
    fi

    log_info "Signing DMG..."

    codesign --sign "$CODESIGN_IDENTITY" \
        --timestamp \
        "$dmg_path"

    log_success "DMG signed"
}

# Generate checksum
generate_checksum() {
    local dmg_path="$OUTPUT_DIR/$DMG_NAME"
    local checksum_file="${dmg_path}.sha256"

    log_info "Generating checksum..."

    shasum -a 256 "$dmg_path" > "$checksum_file"

    log_success "Checksum: $(cat $checksum_file)"
}

# Verify DMG contents
verify_dmg() {
    local dmg_path="$OUTPUT_DIR/$DMG_NAME"

    log_info "Verifying DMG..."

    # Mount and verify
    local mount_info
    mount_info=$(hdiutil attach "$dmg_path" -readonly -noverify)
    local mount_point
    mount_point=$(echo "$mount_info" | grep "Volumes" | awk '{print $NF}')

    if [[ -d "$mount_point/${APP_NAME}.app" ]]; then
        log_success "App bundle found in DMG"
    else
        log_error "App bundle not found in DMG"
        hdiutil detach "$mount_point" -force 2>/dev/null || true
        exit 1
    fi

    if [[ -L "$mount_point/Applications" ]]; then
        log_success "Applications symlink found"
    else
        log_warn "Applications symlink not found"
    fi

    # Detach
    hdiutil detach "$mount_point" -force

    log_success "DMG verification complete"
}

# Cleanup
cleanup() {
    local temp_dir="$1"
    log_info "Cleaning up..."
    rm -rf "$temp_dir"
    log_success "Cleanup complete"
}

# Main function
main() {
    echo "========================================"
    echo "EasySSH DMG Packaging Script"
    echo "Version: $VERSION"
    echo "========================================"

    # Check prerequisites
    check_prerequisites

    # Setup
    mkdir -p "$OUTPUT_DIR"
    TEMP_DIR=$(setup_temp_dir)
    setup_background "$TEMP_DIR"

    # Create DMG
    if ! create_dmg_create_dmg "$TEMP_DIR"; then
        create_dmg_hdiutil "$TEMP_DIR"
    fi

    # Sign and verify
    sign_dmg
    verify_dmg
    generate_checksum

    # Cleanup
    cleanup "$TEMP_DIR"

    echo ""
    echo "========================================"
    log_success "DMG packaging complete!"
    echo "Output: $OUTPUT_DIR/$DMG_NAME"
    echo "========================================"
}

# Command line options
case "${1:-}" in
    --help|-h)
        echo "Usage: $0 [OPTIONS]"
        echo ""
        echo "Creates a professional DMG installer for EasySSH"
        echo ""
        echo "Environment Variables:"
        echo "  VERSION              App version (default: 0.3.0)"
        echo "  APP_NAME             App name (default: EasySSH)"
        echo "  SOURCE_APP           Path to .app bundle (default: release/EasySSH.app)"
        echo "  OUTPUT_DIR           Output directory (default: ./release)"
        echo "  VOLUME_NAME          DMG volume name (default: 'EasySSH Installer')"
        echo "  DMG_NAME             Output DMG name"
        echo "  CODESIGN_IDENTITY    Signing identity (default: ad-hoc)"
        echo ""
        echo "Examples:"
        echo "  $0                                    # Build with defaults"
        echo "  VERSION=1.0.0 $0                      # Build specific version"
        echo "  SOURCE_APP=dist/MyApp.app $0          # Use custom app path"
        exit 0
        ;;
    *)
        main
        ;;
esac
