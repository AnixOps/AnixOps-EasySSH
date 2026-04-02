#!/bin/bash
# EasySSH macOS Code Signing Setup and Verification Script
# Helps configure and verify code signing for development and distribution

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# List available code signing identities
list_identities() {
    log_info "Available code signing identities:"
    echo ""
    security find-identity -v -p codesigning
    echo ""
}

# Verify app signature
verify_signature() {
    local app_path="${1:-release/EasySSH.app}"

    if [[ ! -d "$app_path" ]]; then
        log_error "App not found: $app_path"
        exit 1
    fi

    log_info "Verifying signature for: $app_path"
    echo ""

    # Check signature
    log_info "Signature details:"
    codesign -d --verbose=4 "$app_path" 2>&1 || true
    echo ""

    # Verify
    log_info "Verifying:"
    if codesign --verify --verbose "$app_path" 2>&1; then
        log_success "Signature valid"
    else
        log_error "Signature invalid"
    fi

    # Check notarization status
    log_info "Notarization status:"
    spctl -a -vvvv "$app_path" 2>&1 || true
    echo ""

    # Check stapled ticket
    log_info "Stapled ticket:"
    if xcrun stapler validate "$app_path" 2>&1 | grep -q "validated"; then
        log_success "Notarization ticket stapled"
    else
        log_warn "No stapled ticket found"
    fi
}

# Check quarantine attributes
check_quarantine() {
    local app_path="${1:-release/EasySSH.app}"

    log_info "Quarantine attributes:"
    xattr -l "$app_path" | grep -E "quarantine|gatekeeper" || echo "No quarantine attributes found"
}

# Remove quarantine attributes
remove_quarantine() {
    local app_path="${1:-release/EasySSH.app}"

    log_info "Removing quarantine attributes from: $app_path"
    xattr -dr com.apple.quarantine "$app_path" 2>/dev/null || true
    log_success "Quarantine attributes removed"
}

# Show help
show_help() {
    cat << EOF
EasySSH macOS Code Signing Helper

Usage: $0 <command> [options]

Commands:
  list              List available code signing identities
  verify [path]     Verify app signature (default: release/EasySSH.app)
  quarantine [path] Check quarantine attributes
  unquarantine [path] Remove quarantine attributes
  notarize [path]   Check notarization status
  setup             Show setup instructions
  help              Show this help

Examples:
  $0 list
  $0 verify ./dist/EasySSH.app
  $0 unquarantine /Applications/EasySSH.app

Environment Variables:
  CODESIGN_IDENTITY    Signing identity to use
  APPLE_ID             Apple ID for notarization
  APPLE_TEAM_ID        Apple Developer Team ID

EOF
}

# Show setup instructions
show_setup() {
    cat << EOF

=== Code Signing Setup for EasySSH ===

1. Apple Developer Account
   - Join Apple Developer Program ($99/year)
   - https://developer.apple.com/programs/

2. Create Certificates
   - Open Xcode → Preferences → Accounts
   - Or visit: https://developer.apple.com/account/resources/certificates/list
   - Create "Developer ID Application" certificate
   - Download and install to Keychain

3. Verify Installation
   Run: security find-identity -v -p codesigning
   Should show: Developer ID Application: Your Name (TEAM_ID)

4. Configure Build
   Set environment variables:
   export CODESIGN_IDENTITY="Developer ID Application: Your Name"

   Or in CI/CD:
   Set GitHub secret: APPLE_DEVELOPER_ID

5. Test Build
   ./scripts/build-macos-ci.sh

=== Notarization Setup ===

1. Generate App-Specific Password
   - Visit: https://appleid.apple.com
   - Sign in → Security → App-Specific Passwords
   - Generate password (e.g., "EasySSH-CI")

2. Configure CI
   Set GitHub secrets:
   - APPLE_ID: your.email@example.com
   - APPLE_TEAM_ID: TEAM_ID
   - APPLE_APP_SPECIFIC_PASSWORD: the-generated-password

3. Enable Notarization
   export NOTARIZE=true
   ./scripts/build-macos-ci.sh

=== Troubleshooting ===

Certificate not found:
  - Ensure certificate is in "login" or "System" keychain
  - Check: security find-identity -v -p codesigning
  - May need to unlock keychain: security unlock-keychain

Notarization fails:
  - Check Apple ID and Team ID are correct
  - Verify app-specific password is current (they expire)
  - Review notarization logs in Apple Developer portal

Gatekeeper warnings:
  - Ensure notarization completed successfully
  - Check ticket is stapled: xcrun stapler validate MyApp.app
  - Remove quarantine: xattr -dr com.apple.quarantine MyApp.app

EOF
}

# Main
case "${1:-help}" in
    list|identities)
        list_identities
        ;;
    verify|check)
        verify_signature "${2:-release/EasySSH.app}"
        ;;
    quarantine)
        check_quarantine "${2:-release/EasySSH.app}"
        ;;
    unquarantine|remove-quarantine)
        remove_quarantine "${2:-release/EasySSH.app}"
        ;;
    notarize|notarization)
        log_info "Notarization status for: ${2:-release/EasySSH.app}"
        spctl -a -vvvv "${2:-release/EasySSH.app}" 2>&1 || true
        ;;
    setup|help)
        show_setup
        ;;
    *)
        show_help
        ;;
esac
