# macOS Code Signing Configuration

## Prerequisites

### Apple Developer Account
- [Apple Developer Program](https://developer.apple.com/programs/) - $99/year
- Required for code signing and notarization

### Certificates Required

1. **Developer ID Application Certificate**
   - Used to sign the application bundle
   - Create in Apple Developer Portal
   - Download and install in Keychain

2. **Developer ID Installer Certificate** (optional)
   - Used for PKG installers

## Environment Variables

```bash
# Required for signing
export APPLE_DEVELOPER_ID="Developer ID Application: Your Name (TEAM_ID)"
export APPLE_TEAM_ID="XXXXXXXXXX"

# Required for notarization
export APPLE_APP_PASSWORD="app-specific-password"
# Generate at: https://appleid.apple.com/account/manage -> App-Specific Passwords

# Optional: Keychain profile for notarytool
export APPLE_KEYCHAIN_PROFILE="notary-profile"
```

## Signing Commands

### Sign App Bundle
```bash
codesign --force --options runtime \
    --sign "$APPLE_DEVELOPER_ID" \
    --entitlements entitlements.plist \
    "EasySSH Lite.app"
```

### Verify Signature
```bash
codesign -dv --verbose=4 "EasySSH Lite.app"
codesign -vv "EasySSH Lite.app"
```

### Sign DMG
```bash
codesign --sign "$APPLE_DEVELOPER_ID" \
    "EasySSH-Lite-0.3.0.dmg"
```

## Notarization

### Submit for Notarization
```bash
xcrun notarytool submit "EasySSH-Lite-0.3.0.dmg" \
    --apple-id "$APPLE_DEVELOPER_ID" \
    --password "$APPLE_APP_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait
```

### Staple Notarization Ticket
```bash
xcrun stapler staple "EasySSH-Lite-0.3.0.dmg"
```

### Verify Notarization
```bash
xcrun stapler validate "EasySSH-Lite-0.3.0.dmg"
spctl -a -t open --context context:primary-signature -v "EasySSH-Lite-0.3.0.dmg"
```

## Automated Signing Script

See `sign-all.sh` for automated signing of all editions.

## CI/CD Integration

### GitHub Actions
```yaml
- name: Sign macOS binaries
  env:
    APPLE_DEVELOPER_ID: ${{ secrets.APPLE_DEVELOPER_ID }}
    APPLE_APP_PASSWORD: ${{ secrets.APPLE_APP_PASSWORD }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
  run: |
    ./resources/installer/macos/signing/sign-all.sh ${{ github.ref_name }}
```

## Troubleshooting

### "App can't be opened because it is from an unidentified developer"
- App is not signed or signature is invalid
- Run: `spctl --add "EasySSH Lite.app"` (temporary fix)
- Proper fix: Sign with valid Developer ID certificate

### Notarization fails
- Check app doesn't use deprecated APIs
- Verify hardened runtime entitlements
- Review notarization logs: `xcrun notarytool log <submission-id>`

### Certificate issues
- Renew certificates before expiration
- Keep private keys secure and backed up
- Revoke compromised certificates immediately

## Security Best Practices

1. **Private Key Protection**
   - Store signing certificates in secure keychain
   - Use hardware security modules (HSM) for enterprise
   - Never commit certificates to version control

2. **Access Control**
   - Limit certificate access to CI/CD systems
   - Use app-specific passwords for notarization
   - Enable two-factor authentication

3. **Audit Trail**
   - Log all signing operations
   - Track certificate usage
   - Monitor for unauthorized access
