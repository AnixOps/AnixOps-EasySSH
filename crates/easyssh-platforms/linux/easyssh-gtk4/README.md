# EasySSH Lite GTK4 UI

Native GTK4 UI implementation for EasySSH Lite on Linux.

## Architecture

```
main.rs              - Application entry point
application.rs       - GTK application setup and lifecycle
window.rs            - Main window with three-panel layout
sidebar.rs           - Left sidebar (groups navigation)
server_list.rs       - Middle panel (server list)
server_detail.rs     - Right panel (server details)
terminal_launcher.rs - Native terminal launcher
dialogs/             - Dialog modules
├── mod.rs           - Dialog exports
├── add_server_dialog.rs    - Add server dialog
├── edit_server_dialog.rs   - Edit server dialog
├── group_dialog.rs         - Add/Edit group dialog
└── password_dialog.rs      - Password input dialog
models.rs            - Data models and state management
styles.css           - GTK4 CSS styles
```

## Design

### Three-Panel Layout
- **Sidebar** (200px): Groups navigation (All Servers, group list)
- **Server List** (280px): Searchable server list with status icons
- **Detail Panel**: Server information and actions

### GNOME Design Guidelines
- Uses libadwaita for modern GNOME-style UI
- Responsive breakpoints for smaller screens
- Dark/light theme support via Adwaita
- Keyboard shortcuts throughout

### Features
- Add/Edit/Delete servers
- Server grouping
- Search/filter servers
- Native terminal launch (gnome-terminal, konsole, xfce4-terminal, etc.)
- Password/key/agent authentication support

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | Add new server |
| `Ctrl+D` | Connect to selected server |
| `Ctrl+E` | Edit selected server |
| `F2`     | Edit selected server |
| `Delete` | Delete selected server |
| `Ctrl+Q` | Quit application |
| `Ctrl+,` | Preferences |

## Terminal Support

Automatically detects and uses available terminal emulators:
1. gnome-terminal
2. konsole
3. xfce4-terminal
4. mate-terminal
5. xterm
6. alacritty
7. kitty
8. wezterm

## Building

```bash
# Install dependencies (Debian/Ubuntu)
sudo apt install libgtk-4-dev libadwaita-1-dev

# Build
cargo build -p easyssh-gtk4 --release

# Run
cargo run -p easyssh-gtk4
```

## Testing

```bash
cargo test -p easyssh-gtk4
```

Note: GTK4 tests require a display (X11 or Wayland).

## Implementation Notes

- Uses GTK4 v4.12 and libadwaita v1.5
- Pure Rust implementation with gtk4-rs bindings
- Integrates with easyssh-core for data operations
- No embedded terminal (Lite version uses native terminal)
