//! Terminal Clipboard Support
//!
//! Provides system clipboard integration for copy-paste operations
//! in the WebGL terminal. Supports Ctrl+C/Ctrl+V and right-click context menu.

use std::sync::{Arc, Mutex};
use tracing::{debug, error, warn};

/// System clipboard wrapper for terminal operations
pub struct TerminalClipboard {
    clipboard: Option<arboard::Clipboard>,
    last_error: Option<String>,
}

impl TerminalClipboard {
    /// Create a new clipboard instance
    pub fn new() -> Self {
        match arboard::Clipboard::new() {
            Ok(clipboard) => {
                debug!("Terminal clipboard initialized successfully");
                Self {
                    clipboard: Some(clipboard),
                    last_error: None,
                }
            }
            Err(e) => {
                warn!("Failed to initialize system clipboard: {}", e);
                Self {
                    clipboard: None,
                    last_error: Some(e.to_string()),
                }
            }
        }
    }

    /// Check if clipboard is available
    pub fn is_available(&self) -> bool {
        self.clipboard.is_some()
    }

    /// Copy text to system clipboard
    pub fn copy(&mut self, text: &str) -> Result<(), String> {
        if let Some(ref mut clipboard) = self.clipboard {
            match clipboard.set_text(text) {
                Ok(_) => {
                    debug!("Copied {} characters to clipboard", text.len());
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to copy to clipboard: {}", e);
                    Err(format!("Copy failed: {}", e))
                }
            }
        } else {
            Err("Clipboard not available".to_string())
        }
    }

    /// Paste text from system clipboard
    pub fn paste(&mut self) -> Result<String, String> {
        if let Some(ref mut clipboard) = self.clipboard {
            match clipboard.get_text() {
                Ok(text) => {
                    debug!("Pasted {} characters from clipboard", text.len());
                    Ok(text)
                }
                Err(e) => {
                    error!("Failed to paste from clipboard: {}", e);
                    Err(format!("Paste failed: {}", e))
                }
            }
        } else {
            Err("Clipboard not available".to_string())
        }
    }

    /// Get the last error if any
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Clear the last error
    pub fn clear_error(&mut self) {
        self.last_error = None;
    }
}

impl Default for TerminalClipboard {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe clipboard wrapper
pub struct SharedClipboard {
    inner: Arc<Mutex<TerminalClipboard>>,
}

impl SharedClipboard {
    /// Create a new shared clipboard
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TerminalClipboard::new())),
        }
    }

    /// Copy text to clipboard
    pub fn copy(&self, text: &str) -> Result<(), String> {
        if let Ok(mut clipboard) = self.inner.lock() {
            clipboard.copy(text)
        } else {
            Err("Failed to lock clipboard".to_string())
        }
    }

    /// Paste text from clipboard
    pub fn paste(&self) -> Result<String, String> {
        if let Ok(mut clipboard) = self.inner.lock() {
            clipboard.paste()
        } else {
            Err("Failed to lock clipboard".to_string())
        }
    }

    /// Check if clipboard is available
    pub fn is_available(&self) -> bool {
        if let Ok(clipboard) = self.inner.lock() {
            clipboard.is_available()
        } else {
            false
        }
    }
}

impl Default for SharedClipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SharedClipboard {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_creation() {
        let clipboard = TerminalClipboard::new();
        // Clipboard may or may not be available in test environment
        // Just ensure it doesn't panic
    }

    #[test]
    fn test_shared_clipboard() {
        let clipboard = SharedClipboard::new();
        let cloned = clipboard.clone();

        // Both should report same availability
        assert_eq!(clipboard.is_available(), cloned.is_available());
    }
}
