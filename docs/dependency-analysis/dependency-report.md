# EasySSH Dependency Analysis Report

**Generated**: 2026-04-01
**Project**: EasySSH - Multi-platform SSH Client
**Version**: 0.3.0

---

## Executive Summary

This report provides a comprehensive analysis of the EasySSH project dependencies, including:
- Workspace crate dependency graph
- Module-level dependency relationships
- External crate dependencies
- Circular dependency analysis
- Optimization recommendations

---

## 1. Workspace Structure

```
EasySSH Workspace
├── core/                    # Core library (easyssh-core)
├── tui/                     # Terminal UI (easyssh-tui)
├── platforms/
│   ├── linux/easyssh-gtk4   # Linux GTK4 native client
│   └── windows/easyssh-winui # Windows native client (egui)
├── pro-server/              # Pro backend API server
├── api-tester/
│   ├── api-core/            # API testing core library
│   └── api-tauri/           # Tauri bindings for API tester
└── platforms/windows/fake-winui-app-sdk  # Fake WinUI SDK stub
```

### 1.1 Crate Dependency Graph

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Crate Dependencies                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────────┐      ┌──────────────┐      ┌──────────────┐            │
│   │  easyssh-tui │      │easyssh-gtk4  │      │easyssh-winui │            │
│   │   (binary)   │      │   (binary)   │      │   (binary)   │            │
│   └──────┬───────┘      └──────┬───────┘      └──────┬───────┘            │
│          │                     │                     │                      │
│          └─────────────────────┼─────────────────────┘                      │
│                                │                                            │
│                                ▼                                            │
│                    ┌──────────────────────┐                                 │
│                    │    easyssh-core    │◄──────────────┐                 │
│                    │     (library)      │               │                 │
│                    └─────────┬──────────┘               │                 │
│                              │                         │                 │
│                              │ uses                    │                 │
│                              ▼                         │                 │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │                 │
│   │ windows-app  │  │  api-tester  │  │  pro-server  │  │                 │
│   │     -sdk     │  │    -core     │  │              │  │                 │
│   │   (stub)     │  └──────────────┘  └──────┬───────┘  │                 │
│   └──────────────┘                            │          │                 │
│                                               │ uses     │                 │
│                                               ▼          │                 │
│                                   ┌──────────────────┐   │                 │
│                                   │  api-tester-     │   │                 │
│                                   │     tauri        │   │                 │
│                                   └──────────────────┘   │                 │
│                                                          │                 │
└──────────────────────────────────────────────────────────┴─────────────────┘
```

---

## 2. Module Dependency Analysis (easyssh-core)

### 2.1 Core Module Hierarchy

```
lib.rs (root)
├── error          (base - all modules depend on this)
├── crypto         (base - used by db, keychain, vault)
├── db             (core - uses crypto, error)
├── ssh            (core - uses db, error, connection_pool)
├── connection_pool (core - uses ssh, error)
├── keychain       (security - uses crypto, error)
├── vault          (security - uses crypto, error)
├── terminal       (standard feature)
│   ├── mod.rs
│   ├── embedded.rs
│   ├── multitab.rs
│   └── theme.rs
├── sftp           (sftp feature - uses ssh, error)
├── layout         (split-screen feature)
├── monitoring     (monitoring feature)
│   ├── mod.rs
│   ├── metrics.rs
│   ├── alerts.rs
│   ├── collector.rs
│   ├── dashboard.rs
│   ├── notifications.rs
│   ├── sla.rs
│   ├── storage.rs
│   ├── tests.rs
│   └── topology.rs
├── docker         (docker feature - uses ssh, error)
├── log_monitor    (log-monitor feature - uses ssh, error)
├── port_forward   (core - uses error)
├── team           (team feature - uses audit, rbac, error)
├── rbac           (pro feature - uses error)
├── audit          (audit feature - uses error)
├── sso            (sso feature - uses team, error)
├── sync           (sync feature - uses crypto, db, error)
├── collaboration  (pro feature - uses error)
├── pro            (pro feature - uses error)
├── backup         (backup feature)
│   ├── mod.rs
│   ├── compression.rs
│   ├── database.rs
│   ├── engine.rs
│   ├── incremental.rs
│   ├── remote.rs
│   ├── report.rs
│   ├── restore.rs
│   ├── scheduler.rs
│   ├── storage.rs
│   └── verification.rs
├── auto_update    (auto-update feature)
│   ├── mod.rs
│   ├── ab_testing.rs
│   ├── delta.rs
│   ├── platform/
│   │   ├── linux.rs
│   │   ├── macos.rs
│   │   └── windows.rs
│   ├── rollback.rs
│   ├── server.rs
│   ├── signature.rs
│   └── ui_integration.rs
├── workflow_engine (workflow feature)
├── workflow_executor (workflow feature - uses workflow_engine, workflow_variables)
├── workflow_scheduler (workflow feature)
├── workflow_variables (workflow feature)
├── macro_recorder  (workflow feature)
├── script_library  (workflow feature - uses workflow_engine, macro_recorder)
├── database_client (database-client feature)
│   ├── mod.rs
│   ├── connection.rs
│   ├── pool.rs
│   ├── query.rs
│   ├── schema.rs
│   ├── batch.rs
│   ├── cache.rs
│   ├── editor.rs
│   ├── erdiagram.rs
│   ├── history.rs
│   ├── import_export.rs
│   ├── performance.rs
│   ├── tunnel.rs
│   └── drivers/
│       ├── mod.rs
│       ├── mod_fix.rs
│       ├── sqlite.rs
│       ├── postgres.rs
│       ├── mysql.rs
│       ├── mongodb.rs
│       └── redis.rs
├── kubernetes     (kubernetes feature)
├── kubernetes_client (kubernetes feature)
├── kubernetes_ffi (kubernetes feature)
├── kubernetes_tauri (kubernetes+tauri features)
├── git_types      (git feature)
├── git_client     (git feature - uses git_types)
├── git_manager    (git feature - uses git_client, git_types)
├── git_ffi        (git feature - uses git_client, git_types)
├── git_workflow   (git feature - uses git_types)
├── git_workflow_executor (git feature - uses git_client, git_manager, git_types, git_workflow)
├── telemetry      (telemetry feature)
│   ├── mod.rs
│   ├── consent.rs
│   ├── collector.rs
│   ├── error_tracker.rs
│   ├── feature_flags.rs
│   ├── feedback.rs
│   ├── health_monitor.rs
│   ├── metrics.rs
│   ├── reporter.rs
│   └── storage.rs
├── remote_desktop (remote-desktop feature)
├── session_recording (tauri feature - uses error)
├── recording_commands (tauri feature - uses session_recording, error)
├── config_import_export (core - uses db, crypto, error)
├── i18n           (core)
├── i18n_ffi       (core - uses i18n)
├── edition        (core)
├── ffi            (core - uses db, error)
├── debug_ws       (core - uses ai_programming)
├── ai_programming (core)
├── security_tests (core)
├── windows_auth   (windows - uses error, vault)
└── linux_service  (linux+standard features)
```

### 2.2 Module Dependency Matrix

| Module | Depends On | Feature |
|--------|-----------|---------|
| error | - | base |
| crypto | error | base |
| db | crypto, error | base |
| ssh | db, error, connection_pool | base |
| connection_pool | ssh, error | base |
| keychain | crypto, error | base |
| vault | crypto, error | base |
| terminal | error | standard |
| sftp | ssh, error | sftp |
| layout | error | split-screen |
| docker | ssh, error | docker |
| log_monitor | ssh, error | log-monitor |
| team | audit, rbac, error | team |
| rbac | error | pro |
| audit | error | audit |
| sso | team, error | sso |
| sync | crypto, db, error | sync |
| collaboration | error | pro |
| backup | ssh (remote.rs) | backup |
| auto_update | - | auto-update |
| workflow_executor | workflow_engine, workflow_variables | workflow |
| script_library | workflow_engine, macro_recorder | workflow |
| database_client | ssh (tunnel.rs) | database-client |
| git_client | git_types | git |
| git_manager | git_client, git_types | git |
| git_ffi | git_client, git_types | git |
| git_workflow | git_types | git |
| git_workflow_executor | git_client, git_manager, git_types, git_workflow | git |
| config_import_export | db, crypto, error | base |
| kubernetes_client | - | kubernetes |
| kubernetes_ffi | kubernetes_client | kubernetes |
| session_recording | error | tauri |

---

## 3. External Dependencies Analysis

### 3.1 Core Dependencies (Always Included)

| Crate | Version | Purpose | Size Impact |
|-------|---------|---------|-------------|
| serde | 1.0 | Serialization | Medium |
| serde_json | 1.0 | JSON handling | Medium |
| rusqlite | 0.32 | SQLite database | Large |
| tokio | 1.50 | Async runtime | Large |
| thiserror | 2.0 | Error handling | Small |
| uuid | 1.0 | UUID generation | Small |
| dirs | 5.0 | Directory paths | Small |
| chrono | 0.4 | Date/time | Medium |

### 3.2 Security Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| argon2 | 0.5 | Password hashing |
| aes-gcm | 0.10 | Encryption |
| keyring | 3.0 | OS credential store |
| ssh2 | 0.9 | SSH protocol |
| rand | 0.8 | Random generation |
| blake3 | 1.5 | Hashing |
| sha2 | 0.10 | SHA hashing |
| hmac | 0.12 | HMAC |

### 3.3 Feature-Gated Dependencies

#### Standard Features
| Feature | Key Dependencies |
|---------|-----------------|
| embedded-terminal | portable-pty, async-trait |
| sftp | ssh2 (already included) |
| monitoring | reqwest, async-trait |
| docker | serde_yaml |
| split-screen | - (internal) |

#### Pro Features
| Feature | Key Dependencies |
|---------|-----------------|
| team | - (internal) |
| audit | - (internal) |
| sso | - (internal, uses totp-rs) |
| sync | reqwest, async-trait |
| backup | tokio-cron-scheduler, zstd, bzip2, chacha20poly1305 |

#### Cloud & Enterprise
| Feature | Key Dependencies |
|---------|-----------------|
| backup-aws | aws-config, aws-sdk-s3 |
| backup-gcp | google-cloud-storage |
| backup-azure | azure_storage, azure_storage_blobs |
| kubernetes | kube, k8s-openapi |
| database-client | sqlx, mysql_async, tokio-postgres, mongodb, redis |

### 3.4 Platform-Specific Dependencies

#### Windows (easyssh-winui)
- **UI Framework**: eframe 0.28, egui 0.28, egui-wgpu 0.28, wgpu 0.20
- **WebView**: wry 0.46, webview2-com 0.33
- **Windows API**: windows 0.56, windows-sys 0.59
- **GPU**: bytemuck, pollster

#### Linux (easyssh-gtk4)
- **UI Framework**: gtk4 0.8, libadwaita 0.6, cairo-rs 0.19
- **No heavy web dependencies**

### 3.5 Pro Server Dependencies

| Category | Crates |
|----------|--------|
| Web Framework | axum 0.7, actix-web 4 (optional), tower 0.4, hyper 1 |
| Database | sqlx 0.8, libsqlite3-sys 0.30 |
| Auth | jsonwebtoken 9, argon2 0.5, bcrypt 0.16 |
| API Docs | utoipa 4, utoipa-swagger-ui 6 |
| Email | lettre 0.11 |
| Rate Limiting | governor 0.6 |
| SSO | saml 0.0.16, openidconnect 3 |

---

## 4. Duplicate Dependencies Analysis

### 4.1 Version Conflicts Detected

| Crate | Versions | Source |
|-------|----------|--------|
| **ahash** | 0.7.8, 0.8.12 | reed-solomon-erasure vs egui/eframe |
| **base64** | 0.21.7, 0.22.1 | easyssh-core vs hyper-util/qrcodegen |
| **bitflags** | 1.3.2, 2.11.0 | nix/portable-pty vs modern crates |
| **hashbrown** | 0.12.3, 0.14.5, 0.16.1 | multiple sources |
| **getrandom** | 0.2.17, 0.3.4, 0.4.2 | dependency tree divergence |
| **windows-sys** | 0.48.0, 0.59.0, 0.61.2 | multiple Windows dependencies |

### 4.2 Impact Assessment

| Conflict | Severity | Notes |
|----------|----------|-------|
| ahash | Medium | 0.7.x vs 0.8.x API differences |
| base64 | Low | Mostly compatible |
| bitflags | Low | 1.x vs 2.x used by different ecosystems |
| hashbrown | Low | Internal implementation detail |
| getrandom | Medium | Could cause linking issues |
| windows-sys | High | Multiple versions of Windows APIs |

---

## 5. Circular Dependency Analysis

### 5.1 Findings

**No circular dependencies detected** in the current codebase.

All module dependencies form a directed acyclic graph (DAG):

```
error → crypto → db → ssh → [sftp, docker, log_monitor, backup/remote, database_client/tunnel]
                    ↓
              connection_pool → ssh (bidirectional but via Arc/Mutex, not compile-time cycle)
```

### 5.2 Runtime Dependency Patterns

The following runtime dependency patterns exist (not compile-time cycles):

1. **AppState** contains all managers via Arc/RwLock/Mutex
2. **SshSessionManager** and **ConnectionPool** reference each other at runtime
3. **DatabaseClient** may use **SshSessionManager** for tunnel connections

These are intentional runtime patterns, not circular compile dependencies.

---

## 6. Dependency Size Analysis

### 6.1 Estimated Binary Size Impact

| Feature Set | Estimated Size | Key Contributors |
|-------------|---------------|------------------|
| Lite (default) | ~15-20 MB | rusqlite, ssh2, tokio |
| Standard | ~25-35 MB | + wgpu, egui, portable-pty |
| Pro (all features) | ~50-80 MB | + AWS SDK, Kubernetes, SQLx |
| Pro Server | ~30-40 MB | axum, sqlx, utoipa |

### 6.2 Largest Dependencies

| Crate | Category | Notes |
|-------|----------|-------|
| aws-sdk-s3 | Cloud | Very large (~10MB+) |
| kube | Kubernetes | Large (~5MB+) |
| sqlx | Database | Medium-Large with all drivers |
| wgpu | GPU | Medium (~3MB) |
| egui/eframe | UI | Medium (~2MB) |
| rusqlite | Database | Medium (~2MB with bundled) |

---

## 7. Optimization Recommendations

### 7.1 High Priority

#### 1. Consolidate Windows System Dependencies
```toml
# Current - multiple versions
[target.'cfg(windows)'.dependencies]
windows = { version = "0.56", ... }
windows-sys = { version = "0.59", ... }

# Recommendation: Align all Windows dependencies to same version
windows = { version = "0.59", ... }
windows-sys = { version = "0.59", ... }
```

#### 2. Resolve ahash Version Conflict
```toml
# Consider updating reed-solomon-erasure or patching
[patch.crates-io]
ahash = { version = "0.8" }
```

#### 3. Feature-Gate Heavy Dependencies

Current `Cargo.toml` already has good feature gating, but consider:

```toml
[features]
# Split backup into more granular features
backup = ["dep:tokio-cron-scheduler", ...]
backup-compression = ["backup", "dep:zstd", "dep:bzip2", "dep:xz2"]
backup-cloud-aws = ["backup", "dep:aws-config", "dep:aws-sdk-s3"]
backup-cloud-gcp = ["backup", "dep:google-cloud-storage"]
backup-cloud-azure = ["backup", "dep:azure_storage", "dep:azure_storage_blobs"]
```

### 7.2 Medium Priority

#### 4. Optimize Database Client Drivers
Load database drivers dynamically or feature-gate individually:

```toml
[features]
database-client = ["dep:async-trait"]
database-sqlite = ["database-client"]  # Always included via rusqlite
database-mysql = ["database-client", "dep:mysql_async"]
database-postgres = ["database-client", "dep:tokio-postgres"]
database-mongodb = ["database-client", "dep:mongodb", "dep:futures"]
database-redis = ["database-client", "dep:redis"]
```

#### 5. Remove Unused Dependencies
Check if these are actually used:
- `git2` - only used in git feature, verify usage
- `whoami` - verify all usages
- `flate2` - may be covered by other compression crates

### 7.3 Low Priority

#### 6. Profile-Guided Optimization
```toml
[profile.release]
# Current settings are good, consider:
strip = true  # Already set
lto = "thin"  # Consider thin LTO for faster builds
```

#### 7. Dependency Auditing
```bash
# Run regularly
cargo audit
cargo tree --duplicates
cargo outdated
```

### 7.4 Suggested Cargo.toml Improvements

#### Core Cargo.toml
```toml
[dependencies]
# Group related dependencies
# Compression (choose one primary, others optional)
flate2 = "1.0"  # Already included
zstd = { version = "0.13", optional = true }
bzip2 = { version = "0.4", optional = true }
xz2 = { version = "0.1", optional = true }

# Consider replacing multiple hash crates with consistent ones
# blake3 is already included, could replace sha2 for new code
```

---

## 8. Security Dependency Analysis

### 8.1 Cryptographic Dependencies

| Algorithm | Implementation | Status |
|-----------|----------------|--------|
| AES-256-GCM | aes-gcm 0.10 | Secure |
| Argon2id | argon2 0.5 | Secure |
| Ed25519 | ed25519-dalek 2 | Secure |
| ChaCha20-Poly1305 | chacha20poly1305 0.10 | Secure |
| BLAKE3 | blake3 1.5 | Secure |
| SHA-256 | sha2 0.10 | Secure |

### 8.2 Vulnerability Check

Run the following to check for known vulnerabilities:
```bash
cargo install cargo-audit
cargo audit
```

---

## 9. Build Time Analysis

### 9.1 Slow-to-Build Dependencies

| Crate | Build Time | Notes |
|-------|------------|-------|
| aws-sdk-s3 | Very Slow | Many generated files |
| kube | Slow | Kubernetes API bindings |
| sqlx | Slow | Proc macros, query checking |
| wgpu | Medium | Shader compilation |
| libsqlite3-sys | Medium | C compilation |

### 9.2 Recommendations

1. **Use sccache** for faster rebuilds
2. **Enable incremental compilation** for development
3. **Feature-gate heavy dependencies** for faster Lite builds
4. **Pre-compile large dependencies** in CI cache

---

## 10. Appendix: Full Dependency Tree

### 10.1 Command to Generate
```bash
# Full tree
cargo tree --no-dedupe --all-features

# With duplicates only
cargo tree --duplicates

# Specific package
cargo tree --package easyssh-core --all-features
```

### 10.2 Dependency Graph Files

- `dependency-graph.svg` - Visual representation
- `dependency-graph.dot` - Graphviz source

---

## 11. Summary

### Key Findings

1. **Well-structured workspace** with clear separation of concerns
2. **Good feature-gating** enables Lite builds without heavy dependencies
3. **Some duplicate dependencies** need consolidation (ahash, base64, windows-sys)
4. **No circular compile-time dependencies** detected
5. **Security dependencies** are up-to-date and appropriate

### Action Items

| Priority | Action | Owner |
|----------|--------|-------|
| High | Consolidate windows-sys versions | Core Team |
| High | Resolve ahash 0.7/0.8 conflict | Core Team |
| Medium | Add more granular backup features | Core Team |
| Medium | Review and remove unused deps | Core Team |
| Low | Set up cargo-audit in CI | DevOps |
| Low | Document feature requirements | Docs Team |

---

*Report generated by cargo tree analysis and manual code review.*
