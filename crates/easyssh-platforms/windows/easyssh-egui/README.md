# EasySSH egui Terminal - Windows Standard Edition

## Overview

This crate provides a high-performance embedded terminal UI component using pure Rust with egui framework for the Standard edition of EasySSH on Windows.

## Features

- **Embedded Terminal**: Full terminal emulation with ANSI escape sequence support
- **Scrollback Buffer**: FIFO buffer with configurable size (default 10,000 lines)
- **Search**: Literal and regex search with result navigation
- **Selection**: Mouse selection with clipboard support
- **Key-Driven Reset**: Proper cleanup when terminal widgets are destroyed
- **60fps Rendering**: Smooth rendering with efficient painting

## Architecture

```
┌─────────────────────────────────────────────┐
│              EasySSHApp (app.rs)            │
│   - Terminal Tabs Management                │
│   - Sidebar / Connection List              │
│   - Search Panel                           │
└─────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────┐
│           TerminalView (terminal/view.rs)   │
│   - Key: {connection_id}-{session_id}      │
│   - Buffer Management                      │
│   - Input/Output Handling                  │
│   - Selection & Search                     │
└─────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────┐
│          TerminalBuffer (buffer.rs)        │
│   - VecDeque<TermLine>                     │
│   - FIFO scrollback (max 10000 lines)      │
│   - Cell-based styling                     │
└─────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────┐
│         TerminalRenderer (renderer.rs)     │
│   - egui Painter                           │
│   - Font metrics                           │
│   - Color scheme                           │
└─────────────────────────────────────────────┘
```

## Key-Driven Reset Pattern

All terminal views use a unique key format: `{connection_id}-{session_id}`

When the key changes, the old terminal widget is destroyed and a new one is created, ensuring proper cleanup of handles and subscriptions per SYSTEM_INVARIANTS.md.

## Usage

```rust
use easyssh_egui::{EasySSHApp, TerminalView};
use eframe;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "EasySSH Standard",
        options,
        Box::new(|cc| Ok(Box::new(EasySSHApp::new(cc)))),
    )
}
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+T | New terminal |
| Ctrl+W | Close terminal |
| Ctrl+B | Toggle sidebar |
| Ctrl+F | Toggle search |
| Ctrl+Tab | Next terminal |
| Ctrl+Shift+Tab | Previous terminal |
| Ctrl+C | Copy selection |
| Ctrl+V | Paste |
| Ctrl+A | Select all |
| F3 | Next search result |
| Shift+F3 | Previous search result |

## Platform Trait

The `Platform` trait provides abstraction for terminal management:

```rust
pub trait Platform: Send + Sync {
    fn create_terminal_view(&self, connection_id: &str, session_id: &str) -> Box<dyn TerminalViewTrait>;
    fn destroy_terminal_view(&self, id: &str);
    fn show_notification(&self, title: &str, message: &str);
    fn show_error(&self, title: &str, message: &str);
}
```

## Constraints (from SYSTEM_INVARIANTS.md)

1. **Key Format**: `{connection_id}-{session_id}`
2. **Handle Cleanup**: All handles must be cleaned up when widget is destroyed
3. **Output Processing**: Must not block UI thread (queued and processed during render)
4. **Scroll Buffer**: Maximum 10,000 lines (Standard edition)
5. **Clipboard**: Must support copy/paste operations

## Testing

```bash
# Run all tests
cargo test -p easyssh-egui

# Run integration tests
cargo test -p easyssh-egui --test integration_tests
```

## Dependencies

- `egui` / `eframe`: UI framework
- `vte`: ANSI escape sequence parsing
- `tokio`: Async runtime
- `arboard`: Clipboard support

## License

MIT