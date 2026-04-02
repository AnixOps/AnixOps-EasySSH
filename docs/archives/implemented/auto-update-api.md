# Auto-Update System API Specification

## Base URL
`https://updates.easyssh.dev/api/v1`

## Endpoints

### 1. Check for Updates
`GET /update/check`

Query Parameters:
- `version` (required): Current app version (e.g., "0.3.0")
- `channel` (required): Update channel (stable, beta, nightly, dev)
- `os` (required): Operating system (windows, macos, linux)
- `arch` (required): Architecture (x86_64, aarch64)
- `distro` (optional): Linux distribution (ubuntu, fedora, etc.)
- `distro_version` (optional): Distribution version
- `ab_group` (optional): A/B test group
- `install_id` (optional): Unique installation ID

Response (200 OK):
```json
{
  "update_available": true,
  "info": {
    "version": "0.4.0",
    "build_number": 100,
    "release_notes": "## Changelog\n\n- New features\n- Bug fixes",
    "release_date": "2026-03-31",
    "download_url": "https://cdn.easyssh.dev/releases/0.4.0/EasySSH-0.4.0-x86_64.msi",
    "signature_url": "https://cdn.easyssh.dev/releases/0.4.0/EasySSH-0.4.0-x86_64.msi.sig",
    "size": 52428800,
    "sha256": "a1b2c3d4e5f6...",
    "force_update": false,
    "min_version": null,
    "delta_available": true,
    "delta_url": "https://cdn.easyssh.dev/delta/0.3.0-0.4.0.patch",
    "delta_size": 10485760,
    "delta_from_version": "0.3.0",
    "platform": "windows",
    "channel": "stable",
    "ab_test_features": ["new_ui"],
    "rollout_percentage": 50
  },
  "critical": false,
  "message": null
}
```

Response (204 No Content): No update available

### 2. Get Release Notes
`GET /update/notes/{version}`

Query Parameters:
- `locale` (optional): Locale for localized notes (default: "en")

Response (200 OK):
```json
{
  "version": "0.4.0",
  "release_date": "2026-03-31",
  "notes": "## What's New\n\n...",
  "security_fixes": ["CVE-2026-1234"],
  "breaking_changes": [],
  "deprecated_features": []
}
```

### 3. Report Update Status
`POST /update/report`

Request Body:
```json
{
  "install_id": "uuid-here",
  "version": "0.4.0",
  "success": true,
  "error": null,
  "duration_seconds": 120,
  "platform": "windows",
  "update_method": "delta"
}
```

Response (200 OK): Acknowledged

### 4. Get Delta Patch
`GET /update/delta/{from_version}/{to_version}`

Response (200 OK):
```json
{
  "url": "https://cdn.easyssh.dev/delta/0.3.0-0.4.0.patch",
  "size": 10485760,
  "sha256": "hash-here"
}
```

Response (404 Not Found): Delta not available

### 5. Heartbeat
`POST /heartbeat`

Request Body:
```json
{
  "install_id": "uuid-here",
  "version": "0.3.0",
  "channel": "stable",
  "os": "windows",
  "arch": "x86_64",
  "session_duration": 3600,
  "features_used": ["ssh", "sftp"]
}
```

Response (200 OK):
```json
{
  "feature_flags": {
    "new_terminal": true,
    "dark_mode_default": false
  },
  "rollouts": {
    "0.4.0": 50
  }
}
```

## Update Package Structure

### Windows
- **MSI**: Standard Windows installer with silent mode support
- **EXE**: NSIS installer or portable executable
- Signature: Ed25519 detached signature (.sig file)

### macOS
- **DMG**: App bundle in disk image
- **ZIP**: Compressed app bundle
- **tar.gz**: Archive with app bundle
- Signature: Ed25519 detached signature
- Notarization: Stapled to app bundle

### Linux
- **DEB**: Debian/Ubuntu package
- **RPM**: Fedora/RHEL package
- **pkg.tar.zst**: Arch package
- **AppImage**: Portable executable
- **Flatpak**: Flatpak bundle
- **tar.gz**: Portable archive
- Signature: Ed25519 detached signature (for AppImage), GPG (for packages)

## Signature Format

### Ed25519 Signature
```
Binary format: 64 bytes
Hex encoding: 128 hex characters
JSON format: {"signature": "<hex>"}
```

### Verification
```rust
let verifier = SignatureVerifier::new(&public_key_hex)?;
let is_valid = verifier.verify(&package_data, &signature)?;
```

## A/B Testing

### Group Assignment
Groups are assigned deterministically based on `install_id`:
```
hash = hash(install_id + version)
bucket = hash % 100
if bucket < rollout_percentage: in_rollout = true
```

### Feature Flags
Features can be enabled/disabled per group:
- `new_ui`: New user interface
- `dark_mode_default`: Dark mode as default theme
- `experimental_sftp`: SFTP file manager
- `pro_features`: Early access to Pro features

## Rollout Strategy

1. **Day 1-2**: 5% (Canary)
2. **Day 3-4**: 20% (Early adopters)
3. **Day 5-6**: 50% (General availability)
4. **Day 7+**: 100% (Full rollout)

## Delta Updates

Delta patches are created using bsdiff/bspatch algorithm:
- Average savings: 70-90% compared to full download
- Only available for consecutive versions
- Server keeps last 5 version deltas

## Security

1. All packages signed with Ed25519
2. Public key embedded in application
3. Key rotation supported via backup keys
4. Certificate chain verification for enterprise
5. HTTPS only for all downloads

## Error Handling

### Client Errors
- `400 Bad Request`: Invalid parameters
- `401 Unauthorized`: Invalid signature in request
- `404 Not Found`: Version not found
- `429 Too Many Requests`: Rate limit exceeded

### Server Errors
- `500 Internal Server Error`: Server error, retry later
- `503 Service Unavailable`: Maintenance mode

## Rate Limiting

- Check for updates: 60 requests per hour per install_id
- Download: No limit (but CDN rate limiting applies)
- Report: 100 requests per hour per install_id

## CDN Integration

All downloads served via CDN:
- Edge locations worldwide
- HTTP/2 and HTTP/3 support
- Brotli compression for text files
- Signed URLs for premium users
