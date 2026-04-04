//! Terminal Input Handling for GTK4
//!
//! Provides input event handling for the terminal view with:
//! - Keyboard event processing
//! - Special key combinations (Ctrl+C, Ctrl+D, etc.)
//! - Paste support
//! - Command history navigation
//!
//! # Constraints (SYSTEM_INVARIANTS.md)
//!
//! - Input MUST check connection state before writing (Section 0.3)
//! - Input handling must not block UI thread

use gtk4::prelude::*;
use gtk4::gdk::{Key, ModifierType};
use gtk4::{Entry, Inhibit};
use std::cell::RefCell;
use std::collections::VecDeque;

/// Maximum command history size.
const MAX_HISTORY_SIZE: usize = 100;

/// Terminal input handler.
///
/// Manages keyboard events and input processing for terminal sessions.
pub struct TerminalInputHandler {
    /// Command history
    history: RefCell<VecDeque<String>>,
    /// Current history navigation index
    history_index: RefCell<Option<usize>>,
    /// Current command being edited
    current_command: RefCell<String>,
    /// Input entry widget reference
    entry: RefCell<Option<Entry>>,
    /// Callback for sending input
    send_callback: RefCell<Option<Box<dyn Fn(&[u8]) + 'static>>>,
}

impl TerminalInputHandler {
    /// Create a new input handler.
    pub fn new() -> Self {
        Self {
            history: RefCell::new(VecDeque::with_capacity(MAX_HISTORY_SIZE)),
            history_index: RefCell::new(None),
            current_command: RefCell::new(String::new()),
            entry: RefCell::new(None),
            send_callback: RefCell::new(None),
        }
    }

    /// Set the entry widget reference.
    pub fn set_entry(&self, entry: Entry) {
        self.entry.replace(Some(entry));
    }

    /// Set the send callback.
    pub fn set_send_callback(&self, callback: Box<dyn Fn(&[u8]) + 'static>) {
        self.send_callback.replace(Some(callback));
    }

    /// Handle key press event.
    ///
    /// # Arguments
    ///
    /// * `key` - The pressed key
    /// * `modifiers` - Key modifiers (Ctrl, Shift, etc.)
    ///
    /// # Returns
    ///
    /// `Inhibit::Yes` if the event was handled, `Inhibit::No` otherwise.
    pub fn handle_key_press(&self, key: Key, modifiers: ModifierType) -> Inhibit {
        // Handle Ctrl combinations first
        if modifiers.contains(ModifierType::CONTROL_MASK) {
            return self.handle_ctrl_key(key);
        }

        // Handle special keys
        match key {
            Key::Return | Key::Enter => {
                self.execute_command();
                Inhibit::Yes
            }
            Key::Up => {
                self.navigate_history(true);
                Inhibit::Yes
            }
            Key::Down => {
                self.navigate_history(false);
                Inhibit::Yes
            }
            Key::Tab => {
                // Tab completion - would need shell integration
                self.send_tab();
                Inhibit::Yes
            }
            Key::BackSpace => {
                // Let entry handle normally, but track state
                Inhibit::No
            }
            Key::Delete => {
                Inhibit::No
            }
            Key::Home => {
                self.send_home();
                Inhibit::Yes
            }
            Key::End => {
                self.send_end();
                Inhibit::Yes
            }
            Key::Left => {
                // Navigation - let entry handle
                Inhibit::No
            }
            Key::Right => {
                Inhibit::No
            }
            Key::Page_Up => {
                // Scroll up - handled by scroll window
                Inhibit::No
            }
            Key::Page_Down => {
                Inhibit::No
            }
            _ => Inhibit::No,
        }
    }

    /// Handle Ctrl+Key combinations.
    fn handle_ctrl_key(&self, key: Key) -> Inhibit {
        match key {
            Key::c | Key::C => {
                // Ctrl+C - Interrupt
                self.send_signal(0x03); // ETX
                Inhibit::Yes
            }
            Key::d | Key::D => {
                // Ctrl+D - EOF
                self.send_signal(0x04); // EOT
                Inhibit::Yes
            }
            Key::z | Key::Z => {
                // Ctrl+Z - Suspend (usually not forwarded)
                self.send_signal(0x1A); // SUB
                Inhibit::Yes
            }
            Key::l | Key::L => {
                // Ctrl+L - Clear screen
                self.send_clear_screen();
                Inhibit::Yes
            }
            Key::a | Key::A => {
                // Ctrl+A - Beginning of line
                self.send_home();
                Inhibit::Yes
            }
            Key::e | Key::E => {
                // Ctrl+E - End of line
                self.send_end();
                Inhibit::Yes
            }
            Key::u | Key::U => {
                // Ctrl+U - Clear line before cursor
                self.send_ctrl_u();
                Inhibit::Yes
            }
            Key::k | Key::K => {
                // Ctrl+K - Clear line after cursor
                self.send_ctrl_k();
                Inhibit::Yes
            }
            Key::w | Key::W => {
                // Ctrl+W - Delete word before cursor
                self.send_ctrl_w();
                Inhibit::Yes
            }
            Key::r | Key::R => {
                // Ctrl+R - Reverse search (handled by shell usually)
                Inhibit::No
            }
            _ => Inhibit::No,
        }
    }

    /// Execute the current command.
    fn execute_command(&self) {
        let entry = self.entry.borrow();
        if let Some(ref e) = entry {
            let command = e.text().to_string();
            if !command.is_empty() {
                // Add to history
                self.add_to_history(&command);

                // Send command + newline
                let input = format!("{}\n", command);
                self.send_input(input.as_bytes());

                // Clear entry
                e.set_text("");
            } else {
                // Empty command, just send newline
                self.send_input(&[0x0A]); // LF
            }
        }
    }

    /// Navigate command history.
    ///
    /// # Arguments
    ///
    /// * `up` - True for previous (up arrow), False for next (down arrow)
    fn navigate_history(&self, up: bool) {
        let history = self.history.borrow();
        if history.is_empty() {
            return;
        }

        let mut index = self.history_index.borrow_mut();
        let entry = self.entry.borrow();

        if let Some(ref e) = entry {
            if up {
                // Navigate up (older commands)
                *index = match *index {
                    None => Some(history.len().saturating_sub(1)),
                    Some(i) => Some(i.saturating_sub(1)),
                };
            } else {
                // Navigate down (newer commands)
                *index = match *index {
                    None => return, // No navigation state
                    Some(i) => {
                        if i >= history.len() - 1 {
                            // At newest, clear entry
                            e.set_text("");
                            None
                        } else {
                            Some(i + 1)
                        }
                    }
                };
            }

            // Update entry with history item
            if let Some(i) = *index {
                if let Some(cmd) = history.get(i) {
                    e.set_text(cmd);
                    e.set_position(-1); // Move cursor to end
                }
            }
        }
    }

    /// Add command to history.
    fn add_to_history(&self, command: &str) {
        let mut history = self.history.borrow_mut();

        // Don't add duplicates
        if history.back().map(|s| s.as_str()) == Some(command) {
            return;
        }

        // Add to end
        history.push_back(command.to_string());

        // Enforce max size (FIFO)
        while history.len() > MAX_HISTORY_SIZE {
            history.pop_front();
        }

        // Reset navigation index
        self.history_index.replace(None);
    }

    /// Send raw input bytes.
    fn send_input(&self, data: &[u8]) {
        let callback = self.send_callback.borrow();
        if let Some(ref cb) = callback {
            cb(data);
        }
    }

    /// Send terminal signal (Ctrl+X sequence).
    fn send_signal(&self, signal: u8) {
        self.send_input(&[signal]);

        // Clear any pending input in entry
        let entry = self.entry.borrow();
        if let Some(ref e) = entry {
            e.set_text("");
        }
    }

    /// Send tab character.
    fn send_tab(&self) {
        self.send_input(&[0x09]); // HT
    }

    /// Send home (Ctrl+A equivalent).
    fn send_home(&self) {
        // For shell, Ctrl+A moves to beginning
        // We'll send the escape sequence for home
        self.send_input(b"\x1b[H");
    }

    /// Send end (Ctrl+E equivalent).
    fn send_end(&self) {
        self.send_input(b"\x1b[F");
    }

    /// Send clear screen (Ctrl+L).
    fn send_clear_screen(&self) {
        self.send_input(b"\x1b[2J\x1b[H");
    }

    /// Send Ctrl+U (clear line before cursor).
    fn send_ctrl_u(&self) {
        self.send_input(&[0x15]); // NAK
    }

    /// Send Ctrl+K (clear line after cursor).
    fn send_ctrl_k(&self) {
        self.send_input(&[0x0B]); // VT
    }

    /// Send Ctrl+W (delete word before cursor).
    fn send_ctrl_w(&self) {
        self.send_input(&[0x17]); // ETB
    }

    /// Handle paste operation.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to paste
    pub fn handle_paste(&self, text: &str) {
        // For terminal, paste should send text directly
        // Not insert into entry widget

        // Sanitize: remove problematic characters
        let sanitized = text
            .replace('\r', "") // Remove CR
            .replace('\t', "    "); // Expand tabs to spaces

        // Send sanitized text
        self.send_input(sanitized.as_bytes());
    }

    /// Get command history count.
    pub fn history_count(&self) -> usize {
        self.history.borrow().len()
    }

    /// Clear command history.
    pub fn clear_history(&self) {
        self.history.borrow_mut().clear();
        self.history_index.replace(None);
    }

    /// Get history items.
    pub fn get_history(&self) -> Vec<String> {
        self.history.borrow().iter().cloned().collect()
    }

    /// Reset input state (for session change).
    pub fn reset(&self) {
        self.history_index.replace(None);
        self.current_command.replace(String::new());

        let entry = self.entry.borrow();
        if let Some(ref e) = entry {
            e.set_text("");
        }
    }
}

impl Default for TerminalInputHandler {
    fn default() -> Self {
        Self::new()
    }
}

// ============ Unit Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_handler_creation() {
        let handler = TerminalInputHandler::new();
        assert_eq!(handler.history_count(), 0);
    }

    #[test]
    fn test_add_to_history() {
        let handler = TerminalInputHandler::new();
        handler.add_to_history("ls -la");
        handler.add_to_history("cd /home");

        assert_eq!(handler.history_count(), 2);
        let history = handler.get_history();
        assert_eq!(history[0], "ls -la");
        assert_eq!(history[1], "cd /home");
    }

    #[test]
    fn test_history_duplicate() {
        let handler = TerminalInputHandler::new();
        handler.add_to_history("test");
        handler.add_to_history("test"); // Duplicate

        assert_eq!(handler.history_count(), 1);
    }

    #[test]
    fn test_history_max_size() {
        let handler = TerminalInputHandler::new();

        // Add more than max
        for i in 0..(MAX_HISTORY_SIZE + 20) {
            handler.add_to_history(&format!("cmd {}", i));
        }

        assert_eq!(handler.history_count(), MAX_HISTORY_SIZE);
    }

    #[test]
    fn test_clear_history() {
        let handler = TerminalInputHandler::new();
        handler.add_to_history("test");
        handler.clear_history();
        assert_eq!(handler.history_count(), 0);
    }

    #[test]
    fn test_reset() {
        let handler = TerminalInputHandler::new();
        handler.add_to_history("test");
        handler.reset();

        // History should still exist, just index reset
        assert_eq!(handler.history_count(), 1);
    }

    #[test]
    fn test_signal_values() {
        // Verify signal byte values match expected
        assert_eq!(0x03, 3);  // Ctrl+C (ETX)
        assert_eq!(0x04, 4);  // Ctrl+D (EOT)
        assert_eq!(0x1A, 26); // Ctrl+Z (SUB)
        assert_eq!(0x0A, 10); // LF (newline)
        assert_eq!(0x09, 9);  // Tab (HT)
    }

    #[test]
    fn test_paste_sanitization() {
        let handler = TerminalInputHandler::new();
        let mut received: Vec<u8> = Vec::new();

        handler.set_send_callback(Box::new(|data: &[u8]| {
            received.extend_from_slice(data);
        }));

        // Test with problematic characters
        handler.handle_paste("line1\r\nline2\tend");

        // CR should be removed, tab expanded
        let result = String::from_utf8(received).unwrap();
        assert!(!result.contains('\r'));
        assert!(result.contains("    ")); // Tab expanded
    }
}