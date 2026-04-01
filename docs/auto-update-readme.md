# EasySSH Auto-Update System

Cross-platform automatic update system supporting Windows, macOS, and Linux with signature verification, delta updates, A/B testing, and rollback capabilities.

## Features

### Core Features
- ✅ **Background update checking** - Silent periodic checks
- ✅ **Manual update checks** - User-triggered checks
- ✅ **Delta updates** - Download only changed parts (70-90% savings)
- ✅ **Ed25519 signature verification** - Cryptographically secure
- ✅ **Automatic rollback** - Restore previous version on failure
- ✅ **A/B testing support** - Gradual feature rollouts
- ✅ **Multiple channels** - Stable, Beta, Nightly, Dev
- ✅ **Cross-platform** - Windows, macOS, Linux

### Platform-Specific

#### Windows
- MSI installer support with silent installation
- Portable EXE with in-place update
- NSIS installer support
- Scheduled task for running EXE replacement
- MoveFileEx for delayed replacement on reboot
- Windows S Mode detection
- UAC elevation handling

#### macOS
- DMG with app bundle replacement
- ZIP archive support
- Notarization verification
- Quarantine attribute removal
- Gatekeeper compliance
- Atomic app bundle replacement

#### Linux
- DEB package (APT) support
- RPM package (DNF/YUM) support
- Pacman package support
- AppImage support with desktop integration
- Flatpak bundle support
- Snap support
- Portable tar.gz support

## Quick Start

### Client Integration

```rust
use easyssh_core::updater::{
    AutoUpdater, UpdateConfig, UpdateResult, UpdateResponse,
    presets, init_auto_update, UpdateController
};

// Simple initialization with standard preset
let controller = init_auto_update().await?;

// Set up UI callback
controller.set_ui_callback(|event| {
    match event {
        UpdateUiEvent::UpdateAvailable { info, is_mandatory } => {
            // Show update dialog
        }
        UpdateUiEvent::DownloadProgress { percentage, .. } => {
            // Update progress bar
        }
        _ => {}
    }
}).await;

// Start background checks
let controller_arc = Arc::new(controller);
controller_arc.clone().start_background_checks().await;

// Manual check
controller.check_now().await?;
```

### Advanced Configuration

```rust
use easyssh_core::updater::{
    UpdateConfig, UpdateChannel, AutoUpdater
};

let config = UpdateConfig {
    server_url: "https://updates.easyssh.dev".to_string(),
    check_interval: 3600, // 1 hour
    channel: UpdateChannel::Stable,
    auto_download: true,
    auto_install: false,
    signature_public_key: "YOUR_PUBLIC_KEY_HEX".to_string(),
    ab_test_group: Some("group-a".to_string()),
    enable_delta: true,
    ..Default::default()
};

let updater = AutoUpdater::new(config).await?;
```

## Update Server API

### Check for Updates
```
GET https://updates.easyssh.dev/api/v1/update/check
?version=0.3.0
&channel=stable
&os=windows
&arch=x86_64
```

Response:
```json
{
  "update_available": true,
  "info": {
    "version": "0.4.0",
    "download_url": "https://cdn.easyssh.dev/releases/0.4.0/EasySSH-0.4.0-x86_64.msi",
    "signature_url": "https://cdn.easyssh.dev/releases/0.4.0/EasySSH-0.4.0-x86_64.msi.sig",
    "size": 52428800,
    "sha256": "abc123...",
    "delta_available": true,
    "delta_url": "https://cdn.easyssh.dev/delta/0.3.0-0.4.0.patch"
  }
}
```

### Report Update Status
```
POST https://updates.easyssh.dev/api/v1/update/report
```

```json
{
  "install_id": "uuid-here",
  "version": "0.4.0",
  "success": true,
  "duration_seconds": 120
}
```

## Admin CLI

### Installation
```bash
cargo install --path pro-server --bin update-admin --features admin-cli
```

### Commands

```bash
# Create a release
update-admin release create 0.4.0 100 stable --notes-file CHANGELOG.md

# Set rollout percentage
update-admin rollout set 0.4.0 50

# Check rollout status
update-admin rollout status 0.4.0

# Complete rollout (100%)
update-admin rollout complete 0.4.0

# Pause rollout
update-admin rollout pause 0.4.0

# List releases
update-admin release list --channel stable

# Get statistics
update-admin stats 0.4.0
```

## Deployment

### Docker Compose (Development)

```yaml
version: '3.8'
services:
  update-api:
    build:
      context: ./pro-server
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgresql://postgres:postgres@postgres:5432/updates
      - REDIS_URL=redis://redis:6379
```

### AWS Infrastructure

See [auto-update-deployment.md](auto-update-deployment.md) for CloudFormation templates.

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: easyssh-update-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: easyssh-update-api
  template:
    spec:
      containers:
      - name: api
        image: easyssh/update-api:latest
        ports:
        - containerPort: 8080
```

## Security

### Signature Verification

Updates are signed with Ed25519 signatures:

```rust
let verifier = SignatureVerifier::new(&public_key_hex)?;
let is_valid = verifier.verify(&package_data, &signature)?;
```

### Key Management

- Primary signing key (Ed25519)
- Backup keys for rotation
- Certificate chain support for enterprise

### Network Security

- HTTPS only for all endpoints
- CDN with edge locations
- Rate limiting per install ID

## Rollout Strategy

### Default Schedule

1. **Day 1-2**: 5% (Canary - internal users)
2. **Day 3-4**: 20% (Early adopters)
3. **Day 5-6**: 50% (General availability)
4. **Day 7+**: 100% (Full rollout)

### A/B Testing

```rust
// Check if feature is enabled
if ab_manager.is_feature_enabled("new_terminal").await {
    // Show new terminal UI
}

// Check if in rollout for version
if ab_manager.is_in_rollout("1.0.0", 50) {
    // Offer update
}
```

## Delta Updates

Delta patches are created using bsdiff algorithm:

```bash
# Create delta patch
bsdiff old.msi new.msi patch.bsdiff

# Client applies patch
bspatch old.msi new.msi patch.bsdiff
```

Typical savings: 70-90% bandwidth reduction for minor updates.

## Rollback

Automatic rollback on failed installation:

```rust
// After failed install
if let Err(e) = install_update(&info, &path).await {
    // Rollback automatically triggered
    rollback_manager.rollback().await?;
}
```

Manual rollback:

```rust
controller.rollback().await?;
```

## Configuration Presets

```rust
// Aggressive: auto-download and install
let config = presets::aggressive();

// Standard: auto-download, prompt install
let config = presets::standard();

// Conservative: prompt for everything
let config = presets::conservative();

// Beta tester: beta channel
let config = presets::beta_tester();

// Enterprise: manual updates
let config = presets::enterprise();
```

## Platform Integration

### Windows (WinUI)

```rust
#[cfg(feature = "windows-ui")]
let controller = ui_integration::windows_integration::setup_winui_updater().await?;
```

### macOS (SwiftUI)

```rust
#[cfg(feature = "swift")]
let controller = ui_integration::swift_integration::setup_swift_updater().await?;
```

### Linux (GTK4)

```rust
#[cfg(feature = "gtk")]
let controller = ui_integration::gtk_integration::setup_gtk_updater().await?;
```

### Tauri

```rust
#[cfg(feature = "tauri")]
let controller = ui_integration::tauri_integration::setup_tauri_updater(app_handle).await?;
```

## Monitoring

### Health Checks

```bash
# API health
curl https://updates.easyssh.dev/health

# Update server metrics
curl https://updates.easyssh.dev/api/v1/admin/stats
```

### Metrics

- Download count
- Installation success rate
- Error rate by category
- Rollback rate
- Update duration

## Troubleshooting

### Common Issues

1. **Signature verification fails**
   - Check public key configuration
   - Verify signature file format

2. **Download fails**
   - Check network connectivity
   - Verify CDN endpoint
   - Check rate limits

3. **Installation fails**
   - Check permissions (may need elevation)
   - Verify disk space
   - Check antivirus interference

4. **Rollback fails**
   - Check backup integrity
   - Verify backup directory permissions

## License

MIT License - See LICENSE file

## Contributing

See CONTRIBUTING.md for guidelines.
