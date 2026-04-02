# EasySSH GTK4 UI

Modern GTK4/libadwaita native UI implementation for EasySSH on Linux. Features full GNOME Human Interface Guidelines (HIG) compliance with adaptive design for both small and large screens.

## Features

### Core Functionality
- **Server Management**: Add, edit, delete, and organize SSH servers
- **Server Grouping**: Organize servers into hierarchical groups
- **Quick Search**: Filter servers by name, host, or username with debounced search
- **Native Terminal Integration**: Auto-detects 20+ terminal emulators
- **Authentication Support**: Password, SSH key, and SSH agent authentication

### GNOME Integration
- **Adwaita 1.5 Support**: Full integration with GNOME 43+ design patterns
- **Color Schemes**: Automatic dark/light mode switching with manual override
- **Desktop Notifications**: Connection status and error notifications
- **System Tray**: Background status with quick actions (where supported)
- **Responsive Design**: Adaptive breakpoints for different screen sizes
- **Keyboard Shortcuts**: Comprehensive keyboard navigation

### Accessibility
- **Keyboard Navigation**: Full keyboard control with proper focus management
- **Screen Reader Support**: Proper ARIA labels and accessible roles
- **High Contrast Mode**: Support for accessibility themes
- **Reduced Motion**: Respects system animation preferences

## Architecture

```
src/
├── main.rs                 - Application entry point & CSS loading
├── application.rs          - GTK application lifecycle & actions
├── window.rs              - Main window with responsive three-panel layout
├── sidebar.rs             - Left sidebar (groups navigation)
├── server_list.rs         - Middle panel (server list with search)
├── server_detail.rs       - Right panel (server details & actions)
├── terminal_launcher.rs   - Enhanced terminal detection & launching
├── dialogs/               - Dialog modules
│   ├── mod.rs
│   ├── add_server_dialog.rs
│   ├── edit_server_dialog.rs
│   ├── group_dialog.rs
│   ├── master_password_dialog.rs
│   └── password_dialog.rs
├── theme.rs               - Theme management (dark/light mode)
├── tray.rs                - System tray & notification integration
├── settings.rs            - GSettings persistence
└── models.rs              - Data models & state management

resources/
├── styles.css                     - Adwaita-compliant CSS styles
├── com.easyssh.EasySSH.desktop    - Desktop entry for GNOME
├── com.easyssh.EasySSH.metainfo.xml  - AppStream metadata
└── com.easyssh.EasySSH.gschema.xml   - GSettings schema
```

## Design

### Three-Panel Adaptive Layout
- **Sidebar** (220px min): Groups navigation with collapsible design
- **Server List** (320px min): Searchable server list with status indicators
- **Detail Panel**: Server information with action buttons

### Responsive Breakpoints
| Width | Behavior |
|-------|----------|
| < 700px | Sidebar hidden, compact mode |
| < 900px | Reduced panel widths |
| >= 900px | Full three-panel layout |

### GNOME HIG Compliance
- libadwaita 1.5 widgets (ToolbarView, HeaderBar, etc.)
- Adwaita color palette and CSS variables
- Proper spacing and margins
- Semantic icons and color coding
- Dialog patterns with proper focus

## Keyboard Shortcuts

### Application
| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | Add new server |
| `Ctrl+G` | Add new group |
| `Ctrl+F` | Focus search |
| `Ctrl+R` / `F5` | Refresh server list |
| `F9` | Toggle sidebar |
| `Ctrl+Comma` | Preferences |
| `F1` | Help |
| `Ctrl+T` | Toggle color scheme |
| `Ctrl+Q` | Quit application |
| `Ctrl+?` | Show keyboard shortcuts |

### Server Management
| Shortcut | Action |
|----------|--------|
| `Ctrl+D` | Connect to selected server |
| `Ctrl+E` / `F2` | Edit selected server |
| `Delete` | Delete selected server (with confirmation) |

### Navigation
| Shortcut | Action |
|----------|--------|
| `Up/Down` | Navigate server list |
| `Enter` | Connect to selected server |
| `Esc` | Cancel/close dialog |

## Terminal Support

### Priority Detection Order

**GNOME Environments:**
1. GNOME Console (`kgx`)
2. GNOME Terminal (`gnome-terminal`)
3. Tilix (`tilix`)

**KDE Environments:**
1. Konsole (`konsole`)
2. Yakuake (`yakuake`)

**Modern Terminals (any DE):**
1. WezTerm (`wezterm`)
2. Alacritty (`alacritty`)
3. Kitty (`kitty`)
4. Foot (`footclient`)

**XFCE:**
1. XFCE Terminal (`xfce4-terminal`)

**MATE:**
1. MATE Terminal (`mate-terminal`)

**LXQt:**
1. QTerminal (`qterminal`)

**Wayland Compositors:**
1. Foot (`footclient`)

**Fallbacks:**
1. xterm
2. rxvt-unicode (`urxvt`)

### Terminal Arguments
Each terminal is launched with appropriate arguments for SSH command execution:
- Title setting
- Working directory preservation
- Shell execution flags

## Theme Support

### Color Schemes
- **System Default**: Follows system preference
- **Light Mode**: Bright backgrounds with dark text
- **Dark Mode**: Dark backgrounds with light text
- **High Contrast**: Enhanced visibility for accessibility

### CSS Features
- Adwaita CSS variables (`@accent_color`, `@card_bg_color`, etc.)
- `prefers-color-scheme` media queries
- `prefers-contrast` media queries
- `prefers-reduced-motion` support
- Custom component styling (cards, buttons, lists)

## GNOME Integration

### Desktop Notifications
- Connection success/failure notifications
- Server status change alerts
- Error notifications
- Custom notification IDs for grouping

### System Tray (AppIndicator)
- Quick connect menu
- Show/hide window toggle
- Direct access to preferences
- Quit action

### GSettings Schema
Settings are persisted using GSettings:
- Window size and position
- Sidebar visibility and width
- Color scheme preference
- Terminal emulator selection
- Connection timeout
- Notification preferences
- Search history

## Building

### Dependencies

**Debian/Ubuntu:**
```bash
sudo apt install libgtk-4-dev libadwaita-1-dev
```

**Fedora:**
```bash
sudo dnf install gtk4-devel libadwaita-devel
```

**Arch:**
```bash
sudo pacman -S gtk4 libadwaita
```

### Build Commands
```bash
# Debug build
cargo build -p easyssh-gtk4

# Release build
cargo build -p easyssh-gtk4 --release

# Run with logging
RUST_LOG=debug cargo run -p easyssh-gtk4

# Install gsettings schema (for settings persistence)
sudo cp resources/com.easyssh.EasySSH.gschema.xml /usr/share/glib-2.0/schemas/
sudo glib-compile-schemas /usr/share/glib-2.0/schemas/
```

### Installation
```bash
# Install binary
cargo install --path crates/easyssh-platforms/linux/easyssh-gtk4

# Install desktop file
sudo cp resources/com.easyssh.EasySSH.desktop /usr/share/applications/
sudo cp resources/com.easyssh.EasySSH.metainfo.xml /usr/share/metainfo/
sudo update-desktop-database
```

## Testing

```bash
# Run unit tests
cargo test -p easyssh-gtk4

# Run integration tests (requires display)
cargo test -p easyssh-gtk4 --features integration-tests

# Note: GTK4 tests require a display (X11 or Wayland)
# Use xvfb-run for headless testing:
xvfb-run cargo test -p easyssh-gtk4
```

## Implementation Notes

- **GTK4 Version**: 4.12+ (with v4_12 features)
- **libadwaita Version**: 1.5+ (with v1_5 features)
- **Rust Edition**: 2021
- **Async Runtime**: tokio (via easyssh-core)
- **Logging**: tracing

### Display Server Support
- **Wayland**: Primary target with proper fractional scaling
- **X11**: Full compatibility with XWayland fallback
- **Backend Detection**: Automatic detection of display server type

### Thread Safety
- All GTK operations on main thread
- Async I/O handled via glib::spawn_future_local
- Thread-safe state sharing via Arc<RefCell<>>

## Troubleshooting

### Terminal Not Found
If you see "No Terminal Found" error:
1. Install a supported terminal (see Terminal Support section)
2. Set preferred terminal in preferences
3. Check PATH environment variable

### Theme Not Applying
If dark/light mode doesn't switch:
1. Verify libadwaita version (1.5+ required)
2. Check system theme settings
3. Restart application

### Notifications Not Showing
If desktop notifications don't appear:
1. Verify notification daemon is running
2. Check GNOME notification settings
3. Ensure `X-GNOME-UsesNotifications=true` in desktop file

## License

MIT License - See LICENSE file for details.
