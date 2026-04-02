# EasySSH for Linux

Native Linux SSH client built with GTK4 + libadwaita and Rust core.

## Architecture

```
easyssh-gtk4
├── GTK4 Frontend (Rust)
│   ├── libadwaita for modern GNOME-style UI
│   └── Responsive design with breakpoints
├── easyssh-core (Rust library)
│   ├── SSH connections
│   ├── Encrypted storage
│   └── Session management
```

## Requirements

- GTK 4.12+
- libadwaita 1.5+
- Rust 1.75+

## Building

```bash
# Install dependencies (Debian/Ubuntu)
sudo apt install libgtk-4-dev libadwaita-1-dev

# Install dependencies (Fedora)
sudo dnf install gtk4-devel libadwaita-devel

# Build
cd platforms/linux/easyssh-gtk4
cargo build --release

# Run
./target/release/easyssh
```

## Features

### Lite Mode
- Server list with search/filter
- One-click connect to native terminal (GNOME Terminal, Konsole, etc.)
- Group management
- Encrypted credential storage

### Standard Mode (Future)
- VTE-based embedded terminal
- Split-pane support
- SFTP file browser
- Command snippets

### Pro Mode (Future)
- Team workspace
- Audit logging
- RBAC

## Packaging

```bash
# Create Flatpak
cd packaging/flatpak
flatpak-builder --user --install build-dir com.easyssh.EasySSH.json

# Create Debian package
cargo deb
```
