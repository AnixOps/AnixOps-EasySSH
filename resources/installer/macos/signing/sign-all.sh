#!/bin/bash
# Sign all EasySSH macOS binaries
# Usage: ./sign-all.sh [version]
# Requires: APPLE_DEVELOPER_ID, APPLE_APP_PASSWORD, APPLE_TEAM_ID environment variables

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
VERSION="${1:-0.3.0}"

echo "Signing EasySSH macOS binaries v${VERSION}..."

# Check prerequisites
if [ -z "$APPLE_DEVELOPER_ID" ]; then
    echo "Error: APPLE_DEVELOPER_ID not set"
    exit 1
fi

if [ -z "$APPLE_APP_PASSWORD" ]; then
    echo "Error: APPLE_APP_PASSWORD not set"
    exit 1
fi

if [ -z "$APPLE_TEAM_ID" ]; then
    echo "Error: APPLE_TEAM_ID not set"
    exit 1
fi

RELEASE_DIR="${PROJECT_ROOT}/releases/v${VERSION}/macos"

if [ ! -d "$RELEASE_DIR" ]; then
    echo "Error: Release directory not found: $RELEASE_DIR"
    exit 1
fi

# Function to sign an app bundle
sign_app() {
    local app_path=$1
    local edition=$2

    echo "Signing ${edition} app bundle..."

    # Find the app bundle
    local app_bundle=$(find "$app_path" -name "*.app" -maxdepth 1 | head -1)

    if [ -z "$app_bundle" ]; then
        echo "Warning: No app bundle found for ${edition}"
        return
    fi

    # Sign with hardened runtime
    codesign --force --options runtime \
        --sign "$APPLE_DEVELOPER_ID" \
        --entitlements "${SCRIPT_DIR}/../dmg/entitlements.plist" \
        "$app_bundle"

    # Verify
    codesign -vv "$app_bundle"

    echo "  ${edition} app signed successfully"
}

# Function to notarize a DMG
notarize_dmg() {
    local dmg_path=$1
    local edition=$2

    echo "Notarizing ${edition} DMG..."

    # Submit for notarization
    local submission_id=$(xcrun notarytool submit "$dmg_path" \
        --apple-id "$APPLE_DEVELOPER_ID" \
        --password "$APPLE_APP_PASSWORD" \
        --team-id "$APPLE_TEAM_ID" \
        --output-format json | jq -r '.id')

    echo "  Submission ID: $submission_id"

    # Wait for notarization
    xcrun notarytool wait "$submission_id" \
        --apple-id "$APPLE_DEVELOPER_ID" \
        --password "$APPLE_APP_PASSWORD" \
        --team-id "$APPLE_TEAM_ID"

    # Check status
    local status=$(xcrun notarytool info "$submission_id" \
        --apple-id "$APPLE_DEVELOPER_ID" \
        --password "$APPLE_APP_PASSWORD" \
        --team-id "$APPLE_TEAM_ID" \
        --output-format json | jq -r '.status')

    if [ "$status" != "Accepted" ]; then
        echo "Error: Notarization failed with status: $status"
        xcrun notarytool log "$submission_id" \
            --apple-id "$APPLE_DEVELOPER_ID" \
            --password "$APPLE_APP_PASSWORD" \
            --team-id "$APPLE_TEAM_ID"
        exit 1
    fi

    # Staple the ticket
    xcrun stapler staple "$dmg_path"

    # Verify
    xcrun stapler validate "$dmg_path"

    echo "  ${edition} DMG notarized and stapled"
}

# Sign all editions
for edition in lite standard pro; do
    dmg_file="${RELEASE_DIR}/EasySSH-${edition}-${VERSION}.dmg"

    if [ -f "$dmg_file" ]; then
        sign_app "${RELEASE_DIR}" "$edition"
        notarize_dmg "$dmg_file" "$edition"
    else
        echo "Warning: DMG not found for ${edition} edition"
    fi
done

echo ""
echo "All binaries signed and notarized successfully!"
