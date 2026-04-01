# EasySSH Lite v0.3.0

A lightweight, secure SSH configuration manager with native terminal integration.

## Features

- **Server Management**: Add, list, and organize SSH servers
- **Group Support**: Organize servers into groups
- **Native Terminal**: Launch your system's native terminal for SSH connections
- **SSH Config Import**: Import existing servers from `~/.ssh/config`
- **Secure Storage**: Encrypted database with keyring integration
- **Cross-Platform**: Works on Windows, macOS, and Linux

## Installation

1. Download `easyssh.exe` (Windows) or `easyssh` (macOS/Linux)
2. Place it in your PATH
3. Run `easyssh help` to get started

## Usage

```bash
# Show help
easyssh help

# Add a server
easyssh add-server <name> <host> <username> [port] [auth_type]
easyssh add-server myserver example.com admin 22 agent

# Add a group
easyssh add-group <name>
easyssh add-group production

# List all servers
easyssh list

# Import from ~/.ssh/config
easyssh import-ssh

# Connect to a server (launches native terminal)
easyssh connect <server-id-or-name>

# Show version
easyssh version
```

## Authentication Types

- `agent` (default): Use SSH agent
- `key`: Use SSH key file
- `password`: Use password authentication

## Configuration

Database location:
- Windows: `%APPDATA%\easyssh-lite\easyssh.db`
- macOS: `~/Library/Application Support/easyssh-lite/easyssh.db`
- Linux: `~/.config/easyssh-lite/easyssh.db`

## System Requirements

- Windows 10/11, macOS 10.15+, or Linux
- OpenSSH client installed
- 10 MB disk space

## License

MIT License - See LICENSE file

## Version

v0.3.0 (Lite Edition)
