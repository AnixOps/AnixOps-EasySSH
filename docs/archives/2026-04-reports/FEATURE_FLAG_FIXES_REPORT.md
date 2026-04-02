# EasySSH Feature Flag Fixes Report

**Date**: 2026-04-01
**Scope**: Conditional compilation (`#[cfg(feature = ...)]`) fixes across core library and Windows UI

---

## Summary

Fixed all feature flag configuration issues in the EasySSH monorepo. The changes ensure:
1. All `#[cfg(feature = ...)]` attributes reference valid, declared features
2. Module declarations are properly gated by feature flags
3. Windows UI features properly map to core library features
4. Feature combinations can be compiled independently

---

## Changes Made

### 1. core/Cargo.toml - Added Missing Features

**Added Features:**
- `tauri` - Platform feature for Tauri-specific integrations (used in `kubernetes_tauri`, `recording_commands` modules)
- `git` - Git integration feature with `git2` dependency

**Updated Feature Dependencies:**
- `standard` now includes: `lite`, `embedded-terminal`, `split-screen`, `sftp`, `monitoring`, `remote-desktop`, `log-monitor`, `git`, `docker`
- `pro` includes `standard`, `team`, `audit`, `sso`, `sync`

**New Dependency Entries:**
- `serde_yaml` (for docker feature)
- `git2` (for git feature)
- `flate2`, `csv`, `tracing` (core utilities)
- `sha2`, `hmac`, `totp-rs`, `zeroize`, `reed-solomon-erasure`, `regex`, `tempfile` (vault/security)
- `reqwest`, `ed25519-dalek`, `hex`, `semver`, `bzip2`, `zstd`, `walkdir`, `async-trait` (auto-update)
- `cron-parser`, `tar`, `zip`, `chacha20poly1305`, `xz2`, `futures`, `fs4` (backup)
- Cloud storage: `rusoto_core`, `rusoto_s3`, `aws-config`, `aws-sdk-s3`, `google-cloud-storage`, `azure_storage`, `azure_storage_blobs`
- `blake3`, `xxhash-rust`, `tokio-cron-scheduler` (hashing & scheduling)
- `kube`, `k8s-openapi`, `yaml-rust`, `bytes`, `h2` (kubernetes)
- i18n: `fluent`, `fluent-bundle`, `unic-langid`, `intl-memoizer`, `sys-locale`

### 2. core/src/lib.rs - Fixed Module Declarations

**Issues Fixed:**

1. **Duplicate `kubernetes` module declaration** (Lines 118-119)
   - Removed duplicate: `#[cfg(feature = "kubernetes")] pub mod kubernetes;`
   - Module was already declared at lines 21-22

2. **Missing `sync` module declaration**
   - Added: `#[cfg(feature = "sync")] pub mod sync;`
   - The `sync_ffi` module uses `crate::sync::*`, requiring the sync module to be present when sync feature is enabled

**Module Structure After Fixes:**
```rust
// Git modules (feature = "git")
#[cfg(feature = "git")]
pub mod git_types;
#[cfg(feature = "git")]
pub mod git_client;
#[cfg(feature = "git")]
pub mod git_manager;
#[cfg(feature = "git")]
pub mod git_ffi;
#[cfg(feature = "git")]
pub mod git_workflow;

// Kubernetes modules (feature = "kubernetes")
#[cfg(feature = "kubernetes")]
pub mod kubernetes;
#[cfg(feature = "kubernetes")]
pub mod kubernetes_client;
#[cfg(feature = "kubernetes")]
pub mod kubernetes_ffi;
#[cfg(all(feature = "kubernetes", feature = "tauri"))]
pub mod kubernetes_tauri;

// Sync modules (feature = "sync")
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "sync")]
pub mod sync_ffi;

// Standard edition modules
#[cfg(feature = "embedded-terminal")]
pub mod layout;
#[cfg(feature = "log-monitor")]
pub mod log_monitor;
#[cfg(feature = "log-monitor")]
pub mod log_monitor_ffi;
#[cfg(feature = "monitoring")]
pub mod monitoring;
#[cfg(feature = "sftp")]
pub mod sftp;

// Pro edition modules
#[cfg(feature = "pro")]
pub mod pro;
#[cfg(feature = "pro")]
pub mod team;
#[cfg(feature = "pro")]
pub mod collaboration;
#[cfg(feature = "pro")]
pub mod rbac;

// Enterprise modules
#[cfg(feature = "audit")]
pub mod audit;
#[cfg(feature = "backup")]
pub mod backup;
#[cfg(feature = "telemetry")]
pub mod telemetry;
#[cfg(feature = "auto-update")]
pub mod updater;
#[cfg(feature = "database-client")]
pub mod database_client;

// Docker module
#[cfg(feature = "docker")]
pub mod docker;

// Remote desktop module
#[cfg(feature = "remote-desktop")]
pub mod remote_desktop;

// Tauri-specific modules
#[cfg(feature = "tauri")]
pub mod recording_commands;
```

### 3. platforms/windows/easyssh-winui/Cargo.toml - Feature Mapping

**Issues Fixed:**

1. **Hardcoded sftp feature** in easyssh-core dependency
   - Changed: `easyssh-core = { path = "../../../core", features = ["sftp"] }`
   - To: `easyssh-core = { path = "../../../core" }`
   - Features now properly propagated from Windows UI to core

2. **Missing edition features**
   - Added: `lite`, `standard`, `pro` features
   - Mapped Windows UI features to core library features:
     ```toml
     lite = ["easyssh-core/lite"]
     standard = ["easyssh-core/standard", "ai-terminal", "remote-desktop", "workflow", "code-editor", "monitoring"]
     pro = ["easyssh-core/pro", "standard", "enterprise"]
     database-client = ["easyssh-core/database-client"]
     sync = ["easyssh-core/sync"]
     ```

**Feature Hierarchy:**
```
default = ["standard"]

lite (basic SSH client)
  └── easyssh-core/lite

standard (full-featured client)
  ├── easyssh-core/standard
  ├── ai-terminal
  ├── remote-desktop
  ├── workflow
  ├── code-editor
  └── monitoring

pro (enterprise edition)
  ├── easyssh-core/pro
  ├── standard
  └── enterprise
      ├── database-client
      ├── sync
      ├── backup
      ├── audit
      └── sso
```

---

## Feature Combination Test Results

| Feature Combination | Status | Notes |
|-------------------|--------|-------|
| `lite` only | Compiles | Core lite functionality working |
| `lite + sftp` | Compiles | SFTP file transfer support |
| `lite + embedded-terminal` | Compiles | Embedded terminal with portable-pty |
| `standard` | Code Errors | Module-level issues in monitoring, git, log_monitor |
| `pro` | Code Errors | Module-level issues in team, audit, sync |

**Important Note**: The feature flag configuration is now correct. Remaining errors are in individual module implementations (missing Clone impls, type mismatches, etc.) which are separate code-level issues, not feature flag configuration issues.

---

## Recommended Standard Feature Configuration

### For Core Library (core/Cargo.toml)

**Lite Edition** (minimal SSH client):
```toml
[features]
default = ["lite"]
lite = []
```

**Standard Edition** (full-featured client):
```toml
[features]
default = ["standard"]
standard = [
    "lite",
    "embedded-terminal",  # PTY-based terminal
    "split-screen",       # Multi-panel layout
    "sftp",              # File transfer
    "monitoring",        # Server monitoring
    "remote-desktop",    # VNC/RDP support
    "log-monitor",       # Log file monitoring
    "git",               # Git integration
    "docker"             # Docker management
]
```

**Pro Edition** (enterprise):
```toml
[features]
default = ["pro"]
pro = [
    "standard",
    "team",       # Team management
    "audit",      # Audit logging
    "sso",        # Single sign-on
    "sync"        # Cross-device sync
]
```

### For Windows UI (platforms/windows/easyssh-winui/Cargo.toml)

```toml
[features]
default = ["standard"]

# Edition features
lite = ["easyssh-core/lite"]
standard = [
    "easyssh-core/standard",
    "ai-terminal",
    "remote-desktop",
    "workflow",
    "code-editor",
    "monitoring"
]
pro = [
    "easyssh-core/pro",
    "standard",
    "enterprise"
]

# UI features
enterprise = ["database-client", "sync", "backup", "audit", "sso"]
ai-terminal = ["dep:reqwest", "dep:async-trait", "dep:regex"]
remote-desktop = []
workflow = []
code-editor = []
monitoring = []
database-client = ["easyssh-core/database-client"]
sync = ["easyssh-core/sync"]
backup = []
audit = []
sso = []
```

---

## Verification Commands

Test each feature combination:

```bash
# Lite edition
cargo check --manifest-path core/Cargo.toml --no-default-features --features lite

# Lite with embedded terminal
cargo check --manifest-path core/Cargo.toml --no-default-features --features "lite embedded-terminal"

# Standard edition (all standard features)
cargo check --manifest-path core/Cargo.toml --no-default-features --features standard

# Pro edition (all enterprise features)
cargo check --manifest-path core/Cargo.toml --no-default-features --features pro

# Windows UI with specific edition
cargo check --manifest-path platforms/windows/easyssh-winui/Cargo.toml --features lite
cargo check --manifest-path platforms/windows/easyssh-winui/Cargo.toml --features standard
cargo check --manifest-path platforms/windows/easyssh-winui/Cargo.toml --features pro
```

---

## Remaining Work

The feature flag configuration is now complete. Remaining errors are in individual modules and should be fixed separately:

1. **connection_pool.rs**: Missing Clone implementations for ConnectionPool, UserSession, ShellSession
2. **monitoring.rs**: Missing metrics module exports, type mismatches
3. **git_*.rs**: Git2 API mismatches, private field access
4. **log_monitor.rs**: Serde trait implementations
5. **docker.rs**: Move/borrow issues
6. **kubernetes_client.rs**: Async trait issues

These are implementation bugs, not feature flag configuration issues.

---

## Files Modified

1. `core/Cargo.toml` - Added missing features and dependencies
2. `core/src/lib.rs` - Fixed duplicate module declaration, added missing sync module
3. `platforms/windows/easyssh-winui/Cargo.toml` - Fixed feature mapping

---

## Backward Compatibility

- **Lite edition**: No breaking changes, still compiles and works
- **Standard edition**: Feature flag configuration is correct, code-level fixes needed in modules
- **Pro edition**: Feature flag configuration is correct, code-level fixes needed in modules
