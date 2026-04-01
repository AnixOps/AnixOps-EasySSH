# Enterprise CI/CD Pipeline

This repository features a production-grade, enterprise-level CI/CD pipeline built with GitHub Actions.

## Features

### Multi-Platform Parallel Builds
- **Windows**: Native Windows builds with MSVC toolchain
- **Linux**: x86_64 and ARM64 builds with cross-compilation
- **macOS**: Intel and Apple Silicon (Universal Binary)
- **Docker**: Multi-arch container images (amd64, arm64)

### Incremental Builds
- Path-based change detection triggers only affected components
- Separate workflows for Core, TUI, and Platform-specific code
- Smart caching minimizes rebuild time

### Cache Optimization
- **Cargo**: Registry and build artifacts cached across runs
- **Node.js**: npm packages cached with actions/setup-node
- **Docker**: Layer caching with BuildKit and registry cache
- **APT**: System dependencies cached on Linux
- **Cross**: Toolchain caching for cross-compilation

### Code Signing & Notarization
- **Windows**: Authenticode signing with certificate
- **macOS**: Apple Developer ID signing + Notarization
- **Linux**: GPG signing for AppImage and .deb packages
- All signatures verified in CI

### Automated Releases
- **GitHub Releases**: Automatic release creation on tags
- **Auto-Update**: Manifest generation for in-app updates
- **CDN Deployment**: Artifact distribution to CDN

### Canary Releases
- Gradual rollout (5% → 25% → 50% → 100%)
- Automatic health monitoring
- Auto-rollback on failure
- Manual promotion to stable

### Rollback Mechanism
- One-click rollback via workflow dispatch
- Rollback tags created for each release
- Automated issue creation on rollback
- Team notification via Slack

### Build Matrix
- Rust: stable, beta channels tested
- Node.js: LTS versions
- Multiple OS combinations

### Security Scanning
- **cargo-audit**: Vulnerability scanning
- **cargo-deny**: License and supply chain checks
- **Trivy**: Container image scanning
- **Dependabot**: Automated dependency updates

### Performance Benchmarks
- Criterion.rs benchmarks for core library
- Cross-platform performance tracking
- Regression detection (>10% threshold)
- Historical trend dashboard

## Workflow Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Enterprise CI/CD Pipeline                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Phase 1: Change Detection                                  │
│  ├── Detect modified paths                                  │
│  └── Determine build scope                                  │
│                                                              │
│  Phase 2: Security & Quality Gates                          │
│  ├── cargo audit (vulnerabilities)                          │
│  └── cargo deny (licenses/supply chain)                      │
│                                                              │
│  Phase 3: Matrix Build (Multi-OS/Toolchains)               │
│  └── Build & Test across platforms                         │
│                                                              │
│  Phase 4: Platform Builds                                  │
│  ├── Windows (with code signing)                            │
│  ├── Linux (x64, arm64)                                     │
│  └── macOS (Universal Binary + Notarization)               │
│                                                              │
│  Phase 5: Performance Benchmarks                            │
│  └── Regression detection                                   │
│                                                              │
│  Phase 6: Release Management                                │
│  └── Package & organize artifacts                           │
│                                                              │
│  Phase 7: Canary Release (Optional)                         │
│  ├── Deploy to canary channel                               │
│  ├── Health monitoring                                      │
│  └── Auto-rollback on failure                               │
│                                                              │
│  Phase 8: Production Release                                │
│  └── GitHub Release + CDN deployment                        │
│                                                              │
│  Phase 9: Auto-Update                                      │
│  └── Update manifest for in-app updates                    │
│                                                              │
│  Phase 10: Monitoring & Rollback                            │
│  └── Health checks + rollback capability                    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Workflows

### Main Workflows

| Workflow | File | Trigger | Purpose |
|----------|------|---------|---------|
| Enterprise CI/CD | `enterprise-cicd.yml` | Push, PR, Tags | Main pipeline |
| Docker Builds | `docker-builds.yml` | Workflow call | Container images |
| Code Signing | `code-signing.yml` | Workflow call | Sign binaries |
| Canary Release | `canary-release.yml` | Workflow call | Gradual rollout |
| Benchmarks | `benchmarks.yml` | PR to main | Performance tracking |
| Cache Dependencies | `cache-dependencies.yml` | Weekly | Pre-warm caches |

### Deployment Workflows

| Workflow | File | Target | Description |
|----------|------|--------|-------------|
| AWS Deploy | `aws/deploy.yml` | ECS/EKS | Blue/Green deployment |
| Azure Deploy | `azure/deploy.yml` | ACR/AKS | Container deployment |

## Usage

### Triggering a Build

Builds trigger automatically on:
- Push to `main`, `develop`, `release/*`, `hotfix/*`
- Pull requests to `main` or `develop`
- Tags starting with `v`

### Manual Workflow Dispatch

You can manually trigger the pipeline with custom parameters:

```bash
# Via GitHub CLI
gh workflow run enterprise-cicd.yml \
  -f build_type=canary \
  -f target_platform=all \
  -f enable_signing=true
```

### Creating a Canary Release

```bash
# Deploy to 5% of users
gh workflow run enterprise-cicd.yml \
  -f build_type=canary

# Or via canary workflow directly
gh workflow run canary-release.yml \
  -f percentage=25
```

### Rolling Back

```bash
# Trigger rollback
gh workflow run enterprise-cicd.yml \
  -f build_type=rollback
```

## Secrets Required

### Code Signing
- `WINDOWS_CERTIFICATE`: Base64-encoded PFX certificate
- `WINDOWS_CERTIFICATE_PASSWORD`: Certificate password
- `MACOS_CERTIFICATE`: Base64-encoded signing certificate
- `MACOS_CERTIFICATE_PASSWORD`: Certificate password
- `APPLE_ID`: Apple ID for notarization
- `APPLE_APP_PASSWORD`: App-specific password
- `APPLE_TEAM_ID`: Apple Developer Team ID
- `GPG_PRIVATE_KEY`: GPG key for Linux signing

### Deployment
- `AWS_ROLE_ARN`: IAM role for AWS deployment
- `AZURE_CLIENT_ID`: Azure service principal
- `AZURE_TENANT_ID`: Azure tenant
- `AZURE_SUBSCRIPTION_ID`: Azure subscription
- `CDN_API_KEY`: API key for CDN deployment

### Notifications
- `SLACK_WEBHOOK_URL`: Slack webhook for notifications
- `CANARY_ENDPOINT`: Canary server endpoint
- `CANARY_API_KEY`: API key for canary server

## Performance Metrics

Benchmark results are tracked and compared:

```
┌──────────────────────────────────────────────────────────────┐
│                    Performance Dashboard                     │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Core Library                                                │
│  ├── SSH Connection: 45ms (was 48ms, -6%) ✅                │
│  ├── Key Load: 12ms (was 11ms, +9%) ⚠️                     │
│  └── SFTP Transfer: 120MB/s (was 115MB/s, +4%) ✅         │
│                                                              │
│  Platform                                                    │
│  ├── Windows: 234MB binary (-5% from last) ✅               │
│  ├── Linux: 189MB AppImage (-2% from last) ✅             │
│  └── macOS: 201MB Universal (-3% from last) ✅              │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

Access the dashboard at: `https://anixops.github.io/easyssh/benchmarks/`

## Troubleshooting

### Build Failures
1. Check the `changes` job output to see what triggered the build
2. Review platform-specific logs for compilation errors
3. Verify dependencies are up to date with `cargo update`

### Cache Issues
```bash
# Clear all caches manually
gh cache list
gh cache delete <cache-key>

# Or use the cache-dependencies workflow to rebuild
gh workflow run cache-dependencies.yml
```

### Signing Failures
- Verify secrets are set correctly in repository settings
- Check certificate expiration dates
- For macOS, ensure notarization credentials are valid

## Best Practices

1. **Always use incremental builds**: Only build what changed
2. **Leverage caching**: Caches are pre-warmed weekly
3. **Test in canary first**: Use canary releases for risky changes
4. **Monitor benchmarks**: Watch for performance regressions
5. **Keep dependencies updated**: Dependabot runs weekly

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                        GitHub Actions                          │
├──────────────────────────────────────────────────────────────┤
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │  Build   │  │  Build   │  │  Build   │  │  Docker  │    │
│  │ Windows  │  │  Linux   │  │  macOS   │  │  Images  │    │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘    │
│       └─────────────┴─────────────┴─────────────┘            │
│                         │                                    │
│              ┌──────────┴──────────┐                        │
│              │   Code Signing      │                        │
│              │  (Windows/macOS)    │                        │
│              └──────────┬──────────┘                        │
│                         │                                    │
│              ┌──────────┴──────────┐                        │
│              │   Canary Release    │                        │
│              │   (5% → 100%)      │                        │
│              └──────────┬──────────┘                        │
│                         │                                    │
│              ┌──────────┴──────────┐                        │
│              │  GitHub Release     │                        │
│              │   + CDN Deploy      │                        │
│              └─────────────────────┘                        │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                      Distribution                              │
├──────────────────────────────────────────────────────────────┤
│  GitHub Releases  │  CDN  │  Container Registry  │  Homebrew  │
└──────────────────────────────────────────────────────────────┘
```

## License

This CI/CD configuration is part of the EasySSH project and follows the same license terms.
