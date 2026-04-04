//! Windows Platform Integration
//!
//! Provides Windows-specific platform implementation for egui terminal.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::terminal::view::{TerminalView, TerminalViewTrait};
use crate::terminal::TerminalConfig;

/// Platform trait for terminal abstraction (from SYSTEM_INVARIANTS.md)
pub trait Platform: Send + Sync {
    /// Create a new terminal view with unique ID
    ///
    /// # Key Format
    /// Must use `{connection_id}-{session_id}` format per SYSTEM_INVARIANTS.md
    fn create_terminal_view(&self, connection_id: &str, session_id: &str) -> Box<dyn TerminalViewTrait>;

    /// Destroy terminal view and clean up all handles
    fn destroy_terminal_view(&self, id: &str);

    /// Show notification to user
    fn show_notification(&self, title: &str, message: &str);

    /// Show error notification
    fn show_error(&self, title: &str, message: &str);
}

/// Windows platform implementation
pub struct WindowsPlatform {
    /// Active terminal views
    terminals: Arc<Mutex<HashMap<String, Arc<Mutex<TerminalView>>>>>,
    /// Default terminal configuration
    default_config: TerminalConfig,
}

impl WindowsPlatform {
    /// Create new Windows platform
    pub fn new() -> Self {
        Self {
            terminals: Arc::new(Mutex::new(HashMap::new())),
            default_config: TerminalConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: TerminalConfig) -> Self {
        Self {
            terminals: Arc::new(Mutex::new(HashMap::new())),
            default_config: config,
        }
    }

    /// Get terminal view by ID
    pub fn get_terminal(&self, id: &str) -> Option<Arc<Mutex<TerminalView>>> {
        if let Ok(terminals) = self.terminals.lock() {
            terminals.get(id).cloned()
        } else {
            None
        }
    }

    /// Get all active terminal IDs
    pub fn active_terminals(&self) -> Vec<String> {
        if let Ok(terminals) = self.terminals.lock() {
            terminals.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for WindowsPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for WindowsPlatform {
    fn create_terminal_view(&self, connection_id: &str, session_id: &str) -> Box<dyn TerminalViewTrait> {
        let id = format!("{}-{}", connection_id, session_id);

        let view = TerminalView::with_config(connection_id, session_id, self.default_config.clone());
        let view_arc = Arc::new(Mutex::new(view));

        // Store in registry
        if let Ok(mut terminals) = self.terminals.lock() {
            terminals.insert(id.clone(), view_arc);
        }

        // Return boxed trait object
        Box::new(WindowsTerminalViewWrapper {
            id: id.clone(),
            terminals: self.terminals.clone(),
        })
    }

    fn destroy_terminal_view(&self, id: &str) {
        if let Ok(mut terminals) = self.terminals.lock() {
            if let Some(view) = terminals.remove(id) {
                // Drop the view to clean up handles per SYSTEM_INVARIANTS.md
                drop(view);
            }
        }
    }

    fn show_notification(&self, title: &str, message: &str) {
        // Windows notification using winrt or similar
        // For now, just log
        tracing::info!("Notification: {} - {}", title, message);
    }

    fn show_error(&self, title: &str, message: &str) {
        tracing::error!("Error: {} - {}", title, message);
    }
}

/// Wrapper for Windows terminal view implementing trait
struct WindowsTerminalViewWrapper {
    id: String,
    terminals: Arc<Mutex<HashMap<String, Arc<Mutex<TerminalView>>>>>,
}

impl TerminalViewTrait for WindowsTerminalViewWrapper {
    fn id(&self) -> &str {
        &self.id
    }

    fn write_output(&mut self, data: &[u8]) {
        if let Ok(terminals) = self.terminals.lock() {
            if let Some(view) = terminals.get(&self.id) {
                if let Ok(mut view) = view.lock() {
                    view.write_output(data);
                }
            }
        }
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        if let Ok(terminals) = self.terminals.lock() {
            if let Some(view) = terminals.get(&self.id) {
                if let Ok(mut view) = view.lock() {
                    view.resize(cols, rows);
                }
            }
        }
    }

    fn handle_input(&mut self, input: &str) {
        if let Ok(terminals) = self.terminals.lock() {
            if let Some(view) = terminals.get(&self.id) {
                if let Ok(mut view) = view.lock() {
                    view.handle_input(input);
                }
            }
        }
    }

    fn scroll_to_bottom(&mut self) {
        if let Ok(terminals) = self.terminals.lock() {
            if let Some(view) = terminals.get(&self.id) {
                if let Ok(mut view) = view.lock() {
                    view.scroll_to_bottom();
                }
            }
        }
    }

    fn copy_selection(&mut self) -> Option<String> {
        if let Ok(terminals) = self.terminals.lock() {
            if let Some(view) = terminals.get(&self.id) {
                if let Ok(mut view) = view.lock() {
                    return view.copy_selection();
                }
            }
        }
        None
    }

    fn paste(&mut self, text: &str) {
        if let Ok(terminals) = self.terminals.lock() {
            if let Some(view) = terminals.get(&self.id) {
                if let Ok(mut view) = view.lock() {
                    view.paste(text);
                }
            }
        }
    }

    fn clear(&mut self) {
        if let Ok(terminals) = self.terminals.lock() {
            if let Some(view) = terminals.get(&self.id) {
                if let Ok(mut view) = view.lock() {
                    view.clear();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_creation() {
        let platform = WindowsPlatform::new();
        assert!(platform.active_terminals().is_empty());
    }

    #[test]
    fn test_create_terminal_view() {
        let platform = WindowsPlatform::new();
        let view = platform.create_terminal_view("conn-123", "sess-456");

        assert_eq!(view.id(), "conn-123-sess-456");

        // Should be in registry
        assert_eq!(platform.active_terminals().len(), 1);
    }

    #[test]
    fn test_destroy_terminal_view() {
        let platform = WindowsPlatform::new();
        let view = platform.create_terminal_view("conn", "sess");

        platform.destroy_terminal_view(view.id());

        // Should be removed from registry
        assert!(platform.active_terminals().is_empty());
    }

    #[test]
    fn test_terminal_operations() {
        let platform = WindowsPlatform::new();
        let mut view = platform.create_terminal_view("conn", "sess");

        view.write_output(b"test output");
        view.resize(100, 30);

        // Verify terminal is in registry
        assert!(platform.get_terminal("conn-sess").is_some());
    }

    #[test]
    fn test_key_format_compliance() {
        let platform = WindowsPlatform::new();

        // Test various connection/session IDs
        let ids = [
            ("simple", "terminal"),
            ("conn-with-dash", "sess-123"),
            ("uuid-format", "abc123"),
        ];

        for (conn, sess) in ids {
            let view = platform.create_terminal_view(conn, sess);
            // Key should be in format: {connection_id}-{session_id}
            assert!(view.id().contains("-"));
            assert!(view.id().starts_with(conn));
            assert!(view.id().ends_with(sess));
        }
    }
}