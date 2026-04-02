# EasySSH Linux Build Configuration

This directory contains the complete Linux build system for EasySSH, supporting multiple package formats and architectures.

## Overview

EasySSH for Linux is built as a native GTK4 application using the following components:

- **GTK4**: Modern UI toolkit with GPU acceleration
- **libadwaita**: GNOME-style adaptive widgets
- **Rust**: Core SSH functionality via `easyssh-core`
- **Cairo**: Graphics rendering

## Supported Platforms

| Distribution | Minimum Version | Package Formats |
|--------------|-----------------|-------------------|
| Ubuntu | 22.04 LTS | AppImage, deb, tarball |
| Debian | 12 (Bookworm) | AppImage, deb, tarball |
| Fedora | 38+ | AppImage, rpm, tarball |
| RHEL/CentOS/Rocky | 9+ | AppImage, rpm, tarball |
| Arch Linux | Rolling | AppImage, tarball (AUR ready) |
| openSUSE | Leap 15.4+ | AppImage, rpm, tarball |

## Supported Architectures

- **x86_64**: Primary architecture (AMD64)
- **aarch64**: ARM64 (servers, Apple Silicon, ARM devices)

## Build System

### Scripts

| Script | Purpose | Location |
|--------|---------|----------|
| `build-linux-ci.sh` | Comprehensive CI/CD build script | `scripts/` |
| `build-appimage.sh` | AppImage-specific builder | `platforms/linux/easyssh-gtk4/` |
| `build-linux.sh` | Simple local build | `scripts/` |

### GitHub Workflows

| Workflow | Purpose | Trigger |
|------------|---------|---------|
| `linux-release.yml` | Full Linux release pipeline | Manual, or called by main release |
| `ci.yml` | CI checks including Linux build | Push, PR |

## Quick Start

### Local Build

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt-get install libgtk-4-dev libadwaita-1-dev pkg-config

# Build
./scripts/build-linux.sh

# Run
cargo run --release
```

### CI/CD Build (All Formats)

```bash
# Build all package formats
./scripts/build-linux-ci.sh --install-deps
./scripts/build-linux-ci.sh appimage deb rpm tarball

# Build specific formats
./scripts/build-linux-ci.sh appimage

# With version override
./scripts/build-linux-ci.sh -v 1.2.3 deb rpm
```

### AppImage Only

```bash
cd platforms/linux/easyssh-gtk4
./build-appimage.sh
```

## Package Formats

### AppImage (Recommended)

**Advantages:**
- Universal binary (runs on any modern Linux distro)
- No installation required
- Portable
- Includes dependencies

**Build:**
```bash
./scripts/build-linux-ci.sh appimage
# Output: releases/vX.X.X/linux/EasySSH-X.X.X-x86_64.AppImage
```

**Usage:**
```bash
chmod +x EasySSH-X.X.X-x86_64.AppImage
./EasySSH-X.X.X-x86_64.AppImage
```

### Debian Package (.deb)

**Target:** Ubuntu 22.04+, Debian 12+

**Build:**
```bash
./scripts/build-linux-ci.sh deb
# Output: releases/vX.X.X/linux/easyssh_X.X.X_amd64.deb
```

**Install:**
```bash
sudo dpkg -i easyssh_X.X.X_amd64.deb
sudo apt-get install -f  # Fix dependencies if needed
```

**Dependencies:**
- libgtk-4-1 (>= 4.12)
- libadwaita-1-0 (>= 1.5)
- libssl3

### RPM Package (.rpm)

**Target:** Fedora 38+, RHEL 9+, openSUSE 15.4+

**Build:**
```bash
./scripts/build-linux-ci.sh rpm
# Output: releases/vX.X.X/linux/easyssh-X.X.X-1.x86_64.rpm
```

**Install:**
```bash
sudo rpm -i easyssh-X.X.X-1.x86_64.rpm
# Or on Fedora:
sudo dnf install easyssh-X.X.X-1.x86_64.rpm
```

### Generic Tarball

**Universal fallback for all distributions**

**Build:**
```bash
./scripts/build-linux-ci.sh tarball
# Output: releases/vX.X.X/linux/EasySSH-X.X.X-linux-x86_64.tar.gz
```

**Install:**
```bash
tar -xzf EasySSH-X.X.X-linux-x86_64.tar.gz
cd EasySSH-X.X.X-linux-x86_64
sudo ./install.sh
```

### Flatpak

**Build (via CI):**
```bash
# Trigger via GitHub Actions workflow
# Flatpak requires complex setup, use CI
```

**Install:**
```bash
flatpak install EasySSH-X.X.X-flatpak.flatpak
```

## CI/CD Configuration

### GitHub Actions Workflow

The `.github/workflows/linux-release.yml` workflow:

1. **Build Matrix:**
   - x86_64 native builds
   - aarch64 cross-compilation
   - Multiple package formats in parallel

2. **Stages:**
   - Dependency installation
   - Build (native or cross)
   - Package creation
   - Verification
   - Checksum generation
   - Artifact upload

3. **Manual Trigger:**
   ```yaml
   workflow_dispatch:
     inputs:
       version: "v1.2.3"
       channel: "stable"
       build_arch: "x86_64,aarch64"
       build_packages: "appimage,deb,rpm,tarball"
   ```

### Release Integration

The Linux release workflow is called by the main release workflow:

```yaml
# In .github/workflows/release.yml
build-linux:
  uses: ./.github/workflows/linux-release.yml
  with:
    version: ${{ needs.version.outputs.version }}
    channel: ${{ needs.version.outputs.channel }}
```

## Directory Structure

```
platforms/linux/
├── easyssh-gtk4/
│   ├── Cargo.toml           # GTK4 app configuration
│   ├── build-appimage.sh    # AppImage builder
│   ├── src/                 # Source code
│   ├── resources/           # GTK resources
│   └── tests/               # Integration tests
├── scripts/                 # Additional Linux scripts
└── systemd/                 # systemd service files (optional)

scripts/
├── build-linux.sh           # Simple build
├── build-linux-ci.sh        # Full CI build
└── ...

.github/workflows/
├── linux-release.yml        # Linux release pipeline
├── ci.yml                   # CI checks (includes Linux)
└── ...

releases/
└── vX.X.X/
    └── linux/
        ├── EasySSH-X.X.X-x86_64.AppImage
        ├── easyssh_X.X.X_amd64.deb
        ├── easyssh-X.X.X-1.x86_64.rpm
        ├── EasySSH-X.X.X-linux-x86_64.tar.gz
        └── SHA256SUMS
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VERSION` | Override version | From Cargo.toml |
| `CARGO_TARGET_DIR` | Override target directory | `target` |
| `ARCH` | Target architecture | Auto-detected |
| `PROFILE` | Build profile | `release` |

## Troubleshooting

### GTK4 Not Found

```bash
# Ubuntu/Debian
sudo apt-get install libgtk-4-dev libadwaita-1-dev

# Fedora
sudo dnf install gtk4-devel libadwaita-devel

# Arch
sudo pacman -S gtk4 libadwaita
```

### AppImage Build Fails

1. Install FUSE:
   ```bash
   sudo apt-get install libfuse2
   ```

2. Run with extract mode:
   ```bash
   APPIMAGE_EXTRACT_AND_RUN=1 ./build-appimage.sh
   ```

### Cross-Compilation (ARM64)

```bash
# Install cross
cargo install cross

# Build with cross
cross build --release --target aarch64-unknown-linux-gnu
```

## Development

### Running Tests

```bash
cd platforms/linux/easyssh-gtk4
cargo test

# With display (for UI tests)
export DISPLAY=:0
cargo test --features gtk-tests
```

### Code Quality

```bash
# Format
cargo fmt

# Lint
cargo clippy --all-targets --all-features

# Check
cargo check
```

## Security

- Binary stripping for release builds
- SHA256 checksums for all packages
- Reproducible builds (when possible)
- No hardcoded secrets in scripts

## License

The build scripts are released under the MIT License, same as EasySSH.
