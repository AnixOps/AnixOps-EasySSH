# EasySSH Split Panel Layout System

## Overview

The Windows UI now includes a powerful split panel layout system similar to VS Code and Termius, allowing users to:

- Split panels horizontally and vertically
- View multiple content types simultaneously (Terminal, SFTP, Monitor, Server List)
- Drag and drop panels to rearrange
- Resize panels by dragging splitters
- Quick switch between panels with Alt+Number
- Save and restore layout configurations

## Architecture

### Core Components

```
src/
├── split_layout.rs      # Core layout tree and rendering
├── layout_manager.rs    # High-level layout management
```

### Module Structure

1. **split_layout.rs** - Core layout system
   - `PanelId`: Unique identifier for panels
   - `PanelType`: Enum of panel types (Terminal, SftpBrowser, Monitor, ServerList)
   - `PanelContent`: Metadata for panel content
   - `LayoutNode`: Tree structure for nested layouts
   - `SplitLayout`: Main state management
   - `LayoutPresets`: Common layout configurations

2. **layout_manager.rs** - Integration layer
   - `SplitLayoutManager`: High-level API for managing layouts
   - Helper functions for rendering different panel types

## Features

### 1. Horizontal and Vertical Splitting

```rust
// Split horizontally (creates side-by-side panels)
layout_manager.split_horizontal(panel_id, PanelType::Terminal, "Terminal");

// Split vertically (creates stacked panels)
layout_manager.split_vertical(panel_id, PanelType::SftpBrowser, "SFTP");
```

### 2. Panel Types

- **Terminal**: SSH terminal with command input and output
- **SftpBrowser**: File browser for remote servers
- **Monitor**: Real-time server resource monitoring (CPU, Memory, Disk, Network)
- **ServerList**: Server inventory and connection management

### 3. Drag and Drop

- Drag panel title bars to move them
- Drop on panel edges to split (left, right, top, bottom)
- Drop in center to replace/merge

### 4. Resizable Panels

- Drag splitters between panels to resize
- Minimum panel size enforced (50px)
- Ratios automatically normalized

### 5. Quick Panel Switching (Alt+Number)

- Alt+1: First panel
- Alt+2: Second panel
- ... up to Alt+9

### 6. Layout Persistence

```rust
// Save current layout
let layout_json = layout_manager.save_layout();

// Restore layout
layout_manager.load_layout(&layout_json)?;
```

### 7. Layout Presets

```rust
use layout_manager::LayoutPreset;

// Single panel (default)
layout_manager.apply_preset(LayoutPreset::Single);

// Terminal + SFTP side by side
layout_manager.apply_preset(LayoutPreset::TerminalSftp);

// Triple panel layout
layout_manager.apply_preset(LayoutPreset::Triple);
```

## Usage in EasySSHApp

The split layout system is integrated into `EasySSHApp`:

```rust
struct EasySSHApp {
    // ... other fields ...

    // Split layout system
    split_layout_manager: SplitLayoutManager,
    panel_states: HashMap<PanelId, PanelState>,
    show_layout_menu: bool,
    drag_drop_target: Option<(PanelId, DropTarget)>,
}
```

### Initialization

```rust
impl EasySSHApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // ... other init ...

        Self {
            // ... other fields ...
            split_layout_manager: SplitLayoutManager::default(),
            panel_states: HashMap::new(),
            show_layout_menu: false,
            drag_drop_target: None,
        }
    }
}
```

### Rendering

```rust
// In the update() method
self.split_layout_manager.render(ui, |ui, id, content, is_active| {
    // Get or create panel state
    let state = self.panel_states.entry(id).or_default();

    // Render based on panel type
    match content.panel_type {
        PanelType::Terminal => render_terminal_panel(ui, id, state, self),
        PanelType::SftpBrowser => render_sftp_panel(ui, id, state, self),
        PanelType::Monitor => render_monitor_panel(ui, id, state, self),
        PanelType::ServerList => render_server_list_panel(ui, id, state, self),
    }
});
```

### Keyboard Shortcuts

- **Alt+1..9**: Switch to panel by index
- **Alt+Shift+H**: Split current panel horizontally
- **Alt+Shift+V**: Split current panel vertically
- **Alt+W**: Close current panel
- **Ctrl+Shift+L**: Open layout menu

## Layout Tree Structure

The layout is stored as a tree:

```
Root (Horizontal)
├── Left Panel (Leaf: ServerList)
└── Right Container (Vertical)
    ├── Top Panel (Leaf: Terminal)
    └── Bottom Panel (Leaf: Monitor)
```

## Serialization Format

Layouts are serialized to JSON:

```json
{
  "Horizontal": {
    "children": [
      {
        "Leaf": {
          "id": 1,
          "content": {
            "panel_type": "ServerList",
            "title": "Servers"
          }
        }
      },
      {
        "Leaf": {
          "id": 2,
          "content": {
            "panel_type": "Terminal",
            "title": "Terminal - prod-web-01",
            "session_id": "sess-abc-123"
          }
        }
      }
    ],
    "ratios": [0.25, 0.75]
  }
}
```

## Future Enhancements

1. **Floating Panels**: Detach panels into floating windows
2. **Tab Groups**: Stack multiple panels as tabs within a container
3. **Layout Templates**: User-defined layout templates
4. **Per-Server Layouts**: Remember layout per server connection
5. **Mini-Map**: Visual overview of all panels

## Implementation Notes

- Split layout uses egui's immediate mode rendering
- Panel states are stored in a HashMap keyed by PanelId
- Each panel maintains its own state (terminal output, file list, etc.)
- The layout tree is recalculated each frame for flexibility
- Splitters use egui's drag detection for smooth resizing
