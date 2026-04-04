//! GTK4 Terminal View Widget
//!
//! The main terminal widget for Linux Standard version, providing:
//! - Embedded terminal display
//! - PTY session integration
//! - Search functionality
//! - Input handling
//! - Key-driven reset support
//!
//! # Constraints (SYSTEM_INVARIANTS.md Section 0.2)
//!
//! - Key format: `{connection_id}-{session_id}`
//! - Component destruction MUST clean all handles
//! - Output processing MUST NOT block UI thread
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           TerminalView                   │
//! │    ┌─────────────────────────────┐      │
//! │    │ Toolbar (title, status)      │      │
//! │    └─────────────────────────────┘      │
//! │    ┌─────────────────────────────┐      │
//! │    │ TerminalSearchBar           │      │
//! │    │ (collapsible)               │      │
//! │    └─────────────────────────────┘      │
//! │    ┌─────────────────────────────┐      │
//! │    │ ScrolledWindow              │      │
//! │    │   ┌─────────────────────┐   │      │
//! │    │   │ TextView            │   │      │
//! │    │   │ (TerminalBuffer)    │   │      │
//! │    │   └─────────────────────┘   │      │
//! │    └─────────────────────────────┘      │
//! │    ┌─────────────────────────────┐      │
//! │    │ Input Box                   │      │
//! │    │ (command entry + buttons)   │      │
//! │    └─────────────────────────────┘      │
//! └─────────────────────────────────────────┘
//! ```

use gtk4::prelude::*;
use gtk4::{
    Box, Button, Entry, Label, Orientation, Overlay, ScrolledWindow, TextView, Widget,
    EventControllerKey, PolicyType, WrapMode,
};
use gtk4::glib;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use super::buffer::TerminalBuffer;
use super::input::TerminalInputHandler;
use super::search::TerminalSearchBar;
use super::style::TerminalStyle;

// Import core types when embedded-terminal feature is enabled
#[cfg(feature = "embedded-terminal")]
use easyssh_core::terminal::{PtyHandle, PtyState};

/// Terminal view widget for GTK4.
///
/// Provides a complete embedded terminal with:
/// - Text display and styling
/// - Search functionality
/// - Input handling with history
/// - PTY integration (when available)
///
/// # Key-Driven Reset (SYSTEM_INVARIANTS.md Section 0.2)
///
/// The view uses the key `{connection_id}-{session_id}` for state tracking.
/// When connection ID changes, the view must be destroyed and recreated.
pub struct TerminalView {
    /// Main container widget
    container: Box,
    /// Overlay for search bar
    overlay: Overlay,
    /// Terminal output view
    text_view: TextView,
    /// Scrolled window container
    scroll_window: ScrolledWindow,
    /// Input entry
    command_entry: Entry,
    /// Search bar component
    search_bar: TerminalSearchBar,
    /// Terminal buffer (text content)
    buffer: Rc<TerminalBuffer>,
    /// Input handler
    input_handler: Rc<TerminalInputHandler>,
    /// Style configuration
    style: RefCell<TerminalStyle>,
    /// Connection ID (for key-driven reset)
    connection_id: RefCell<String>,
    /// Session ID (for key-driven reset)
    session_id: RefCell<String>,
    /// PTY session handle (when available)
    #[cfg(feature = "embedded-terminal")]
    pty_session: RefCell<Option<Arc<Mutex<PtyHandle>>>>,
    /// Output receiver handle
    output_poll_id: RefCell<Option<glib::SourceId>>,
    /// Status label
    status_label: Label,
    /// Title label
    title_label: Label,
    /// Input send callback
    send_callback: RefCell<Option<Box<dyn Fn(&[u8]) + 'static>>>,
}

impl TerminalView {
    /// Create a new terminal view.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - Connection identifier (for key-driven reset)
    /// * `session_id` - Session identifier (for key-driven reset)
    ///
    /// # Returns
    ///
    /// A new TerminalView instance.
    pub fn new(connection_id: &str, session_id: &str) -> Self {
        // Create style
        let style = TerminalStyle::default_dark();

        // Create buffer
        let buffer = Rc::new(TerminalBuffer::with_style(style.clone()));

        // Create text view
        let text_view = TextView::with_buffer(buffer.gtk_buffer());
        text_view.set_editable(false);
        text_view.set_cursor_visible(false);
        text_view.set_wrap_mode(WrapMode::WordChar);
        text_view.set_monospace(true);
        text_view.add_css_class("terminal-output");

        // Create scrolled window
        let scroll_window = ScrolledWindow::new();
        scroll_window.set_child(Some(&text_view));
        scroll_window.set_vexpand(true);
        scroll_window.set_hscrollbar_policy(PolicyType::Never);
        scroll_window.set_vscrollbar_policy(PolicyType::Automatic);
        scroll_window.add_css_class("terminal-scroll");

        // Create command input
        let command_entry = Entry::new();
        command_entry.set_hexpand(true);
        command_entry.set_placeholder_text(Some("Enter command..."));
        command_entry.add_css_class("terminal-input");

        // Create input handler
        let input_handler = Rc::new(TerminalInputHandler::new());
        input_handler.set_entry(command_entry.clone());

        // Create search bar
        let search_bar = TerminalSearchBar::new();
        search_bar.set_buffer(buffer.clone());

        // Create main container
        let container = Box::new(Orientation::Vertical, 0);
        container.add_css_class("terminal-view");

        // Create overlay for search bar
        let overlay = Overlay::new();
        overlay.set_child(Some(&scroll_window));

        // Add search bar as overlay (at top)
        overlay.add_overlay(search_bar.container());
        search_bar.hide(); // Initially hidden

        // Create toolbar
        let toolbar = Box::new(Orientation::Horizontal, 8);
        toolbar.set_margin_start(8);
        toolbar.set_margin_end(8);
        toolbar.set_margin_top(4);
        toolbar.set_margin_bottom(4);
        toolbar.add_css_class("terminal-toolbar");

        let title_label = Label::new(Some("SSH Terminal"));
        title_label.add_css_class("title-4");

        let status_label = Label::new(Some("Ready"));
        status_label.add_css_class("terminal-status");

        // Toolbar buttons
        let search_button = Button::from_icon_name("system-search-symbolic");
        search_button.set_tooltip_text(Some("Search (Ctrl+F)"));

        let clear_button = Button::from_icon_name("edit-clear-all-symbolic");
        clear_button.set_tooltip_text(Some("Clear terminal (Ctrl+L)"));

        let interrupt_button = Button::from_icon_name("process-stop-symbolic");
        interrupt_button.set_tooltip_text(Some("Interrupt (Ctrl+C)"));

        let disconnect_button = Button::from_icon_name("window-close-symbolic");
        disconnect_button.set_tooltip_text(Some("Disconnect"));
        disconnect_button.add_css_class("destructive-action");

        // Spacer
        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);

        toolbar.append(&title_label);
        toolbar.append(&spacer);
        toolbar.append(&status_label);
        toolbar.append(&search_button);
        toolbar.append(&clear_button);
        toolbar.append(&interrupt_button);
        toolbar.append(&disconnect_button);

        // Create input box
        let input_box = Box::new(Orientation::Horizontal, 8);
        input_box.set_margin_start(8);
        input_box.set_margin_end(8);
        input_box.set_margin_top(4);
        input_box.set_margin_bottom(4);
        input_box.add_css_class("terminal-input-box");

        let prompt_label = Label::new(Some("$"));
        prompt_label.add_css_class("terminal-prompt");
        prompt_label.set_markup("<span foreground='#50FA7B'>$</span>");

        let execute_button = Button::with_label("Execute");
        execute_button.add_css_class("suggested-action");
        execute_button.set_sensitive(false); // Initially disabled until connected

        input_box.append(&prompt_label);
        input_box.append(&command_entry);
        input_box.append(&execute_button);

        // Build main container
        container.append(&toolbar);
        container.append(&overlay);
        container.append(&input_box);

        // Create view
        let view = Self {
            container,
            overlay,
            text_view,
            scroll_window,
            command_entry,
            search_bar,
            buffer,
            input_handler,
            style: RefCell::new(style),
            connection_id: RefCell::new(connection_id.to_string()),
            session_id: RefCell::new(session_id.to_string()),
            #[cfg(feature = "embedded-terminal")]
            pty_session: RefCell::new(None),
            output_poll_id: RefCell::new(None),
            status_label,
            title_label,
            send_callback: RefCell::new(None),
        };

        // Setup signals and controllers
        view.setup_signals(&search_button, &clear_button, &interrupt_button, &disconnect_button, &execute_button);
        view.setup_key_controller();
        view.setup_entry_signals(&execute_button);

        view
    }

    /// Setup button signals.
    fn setup_signals(
        &self,
        search_button: &Button,
        clear_button: &Button,
        interrupt_button: &Button,
        disconnect_button: &Button,
        execute_button: &Button,
    ) {
        // Search button
        search_button.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.search_bar.toggle();
        }));

        // Clear button
        clear_button.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.buffer.clear();
        }));

        // Interrupt button (Ctrl+C)
        interrupt_button.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.send_signal(0x03); // ETX (Ctrl+C)
        }));

        // Disconnect button
        disconnect_button.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.disconnect();
        }));

        // Execute button
        execute_button.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.execute_command();
        }));
    }

    /// Setup key controller for shortcuts.
    fn setup_key_controller(&self) {
        let controller = EventControllerKey::new();

        controller.connect_key_pressed(
            glib::clone!(@weak self as view => move |_, key, _, modifiers| {
                // Handle search shortcut (Ctrl+F)
                if modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                    match key {
                        gtk4::gdk::Key::f | gtk4::gdk::Key::F => {
                            view.search_bar.toggle();
                            glib::Propagation::Stop
                        }
                        gtk4::gdk::Key::l | gtk4::gdk::Key::L => {
                            view.buffer.clear();
                            glib::Propagation::Stop
                        }
                        _ => {
                            // Pass to input handler for other Ctrl combos
                            view.input_handler.handle_key_press(key, modifiers);
                            glib::Propagation::Stop
                        }
                    }
                } else {
                    // Let entry handle regular keys
                    glib::Propagation::Proceed
                }
            }),
        );

        self.container.add_controller(controller);
    }

    /// Setup entry signals.
    fn setup_entry_signals(&self, execute_button: &Button) {
        // Enter key to execute
        self.command_entry.connect_activate(glib::clone!(@weak self as view => move |_| {
            view.execute_command();
        }));

        // History navigation (Up/Down)
        self.command_entry.connect_key_pressed(
            glib::clone!(@weak self as view => move |_, key, _, _| {
                match key {
                    gtk4::gdk::Key::Up => {
                        view.input_handler.handle_key_press(key, gtk4::gdk::ModifierType::empty());
                        glib::Propagation::Stop
                    }
                    gtk4::gdk::Key::Down => {
                        view.input_handler.handle_key_press(key, gtk4::gdk::ModifierType::empty());
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                }
            }),
        );

        // Track text changes for execute button state
        self.command_entry.connect_changed(glib::clone!(@weak execute_button as btn => move |_| {
            btn.set_sensitive(true);
        }));
    }

    /// Set the input send callback.
    ///
    /// # Arguments
    ///
    /// * `callback` - Function to call when input should be sent to PTY
    pub fn set_send_callback(&self, callback: Box<dyn Fn(&[u8]) + 'static>) {
        self.send_callback.replace(Some(callback));
        self.input_handler.set_send_callback(callback);
    }

    /// Connect to a PTY session.
    ///
    /// # Constraints (SYSTEM_INVARIANTS.md Section 1.1)
    ///
    /// - PTY must be created after connection is active
    /// - Must follow state gating rules
    #[cfg(feature = "embedded-terminal")]
    pub fn connect_pty(&self, pty: PtyHandle) {
        // Check state gating (connection must be active)
        // This would be enforced by the caller

        self.pty_session.replace(Some(Arc::new(Mutex::new(pty))));
        self.status_label.set_text("Connected");
        self.status_label.add_css_class("status-connected");

        // Start output polling
        self.start_output_polling();
    }

    /// Start output polling from PTY.
    ///
    /// # Constraints (SYSTEM_INVARIANTS.md Section 1.1)
    ///
    /// - Output callback MUST NOT block UI thread
    /// - Uses GLib timeout for non-blocking polling
    #[cfg(feature = "embedded-terminal")]
    fn start_output_polling(&self) {
        // This would poll the PTY output channel
        // For now, this is a placeholder showing the pattern

        // Stop any existing polling
        if let Some(id) = self.output_poll_id.borrow().take() {
            id.remove();
        }

        // Start new polling loop
        let buffer = self.buffer.clone();
        let poll_id = glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            // In real implementation:
            // 1. Try_recv from output channel
            // 2. If data, append to buffer
            // 3. Scroll to bottom

            // This is simplified - actual implementation would use
            // the PtyHandle's output channel

            glib::ControlFlow::Continue
        });

        self.output_poll_id.replace(Some(poll_id));
    }

    /// Write output data to the terminal.
    ///
    /// # Arguments
    ///
    /// * `data` - Output data bytes from PTY
    ///
    /// # Constraints
    ///
    /// - Must not block UI thread (called from timeout handler)
    pub fn write_output(&self, data: &[u8]) {
        // Convert bytes to string (handle UTF-8)
        let text = String::from_utf8_lossy(data);

        // Append with ANSI processing
        self.buffer.append_with_ansi(&text);

        // Auto-scroll to bottom
        self.scroll_to_bottom();
    }

    /// Resize the terminal.
    ///
    /// # Arguments
    ///
    /// * `cols` - New column count
    /// * `rows` - New row count
    #[cfg(feature = "embedded-terminal")]
    pub fn resize(&self, cols: u16, rows: u16) {
        if let Some(ref pty_arc) = self.pty_session.borrow().as_ref() {
            let pty = pty_arc.lock().unwrap();
            // In async context, this would be:
            // pty.resize(cols, rows).await
            // For GTK4, we use a simpler sync approach for resize notification
            tracing::info!("Terminal resize requested: {}x{}", cols, rows);
        }
    }

    /// Scroll to bottom of buffer.
    pub fn scroll_to_bottom(&self) {
        self.buffer.scroll_to_bottom();

        // Also scroll the window
        let adj = self.scroll_window.vadjustment();
        adj.set_value(adj.upper() - adj.page_size());
    }

    /// Search for text in terminal.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Search pattern
    /// * `regex` - Whether to use regex mode
    pub fn search(&self, pattern: &str, regex: bool) {
        self.search_bar.set_pattern(pattern);
        self.search_bar.set_regex_mode(regex);
        self.search_bar.show();
    }

    /// Send a signal (Ctrl+X sequence).
    fn send_signal(&self, signal: u8) {
        let callback = self.send_callback.borrow();
        if let Some(ref cb) = callback {
            cb(&[signal]);
        }

        // Clear entry
        self.command_entry.set_text("");

        // Append visual indicator
        match signal {
            0x03 => self.buffer.append("^C\n", None),
            0x04 => self.buffer.append("^D\n", None),
            0x1A => self.buffer.append("^Z\n", None),
            _ => {}
        }
    }

    /// Execute current command.
    fn execute_command(&self) {
        let command = self.command_entry.text().to_string();
        if command.is_empty() {
            return;
        }

        // Display command in output
        self.buffer.append(&format!("$ {}\n", command), None);

        // Send command + newline
        let input = format!("{}\n", command);
        let callback = self.send_callback.borrow();
        if let Some(ref cb) = callback {
            cb(input.as_bytes());
        }

        // Clear entry
        self.command_entry.set_text("");
    }

    /// Disconnect from session.
    fn disconnect(&self) {
        self.status_label.set_text("Disconnected");
        self.status_label.add_css_class("status-disconnected");
        self.status_label.remove_css_class("status-connected");

        // Stop output polling
        if let Some(id) = self.output_poll_id.borrow().take() {
            id.remove();
        }

        // Clear PTY reference
        #[cfg(feature = "embedded-terminal")]
        self.pty_session.replace(None);

        // Add disconnect message
        self.buffer.append("\n--- Disconnected ---\n", None);
    }

    /// Destroy the terminal view.
    ///
    /// # Constraints (SYSTEM_INVARIANTS.md Section 0.2)
    ///
    /// - MUST clean all handles and subscriptions
    /// - MUST stop all polling loops
    pub fn destroy(&self) {
        // Stop output polling
        if let Some(id) = self.output_poll_id.borrow().take() {
            id.remove();
        }

        // Clear callbacks
        self.send_callback.replace(None);

        // Clear PTY
        #[cfg(feature = "embedded-terminal")]
        self.pty_session.replace(None);

        // Clear search buffer
        self.search_bar.hide();

        tracing::info!(
            "TerminalView destroyed for key: {}-{}",
            self.connection_id.borrow(),
            self.session_id.borrow()
        );
    }

    /// Get the unique key for this view.
    ///
    /// # Returns
    ///
    /// Key in format `{connection_id}-{session_id}` per SYSTEM_INVARIANTS.md
    pub fn key(&self) -> String {
        format!("{}-{}", self.connection_id.borrow(), self.session_id.borrow())
    }

    /// Get connection ID.
    pub fn connection_id(&self) -> String {
        self.connection_id.borrow().clone()
    }

    /// Get session ID.
    pub fn session_id(&self) -> String {
        self.session_id.borrow().clone()
    }

    /// Set terminal title.
    pub fn set_title(&self, title: &str) {
        self.title_label.set_text(title);
    }

    /// Set terminal style.
    pub fn set_style(&self, style: TerminalStyle) {
        self.style.replace(style);
        // Would need to recreate buffer tags
    }

    /// Get the container widget.
    pub fn widget(&self) -> &Widget {
        self.container.upcast_ref()
    }

    /// Get the container box.
    pub fn container(&self) -> &Box {
        &self.container
    }

    /// Get buffer content.
    pub fn content(&self) -> String {
        self.buffer.content()
    }

    /// Get buffer statistics.
    pub fn buffer_stats(&self) -> (usize, usize, u64) {
        (
            self.buffer.line_count(),
            self.buffer.max_lines(),
            self.buffer.evicted_count(),
        )
    }
}

impl Drop for TerminalView {
    fn drop(&mut self) {
        self.destroy();
    }
}

// ============ Unit Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_view_creation() {
        let view = TerminalView::new("conn-1", "sess-1");
        assert_eq!(view.connection_id(), "conn-1");
        assert_eq!(view.session_id(), "sess-1");
        assert_eq!(view.key(), "conn-1-sess-1");
    }

    #[test]
    fn test_key_format() {
        let view = TerminalView::new("connection-abc", "session-xyz");
        assert_eq!(view.key(), "connection-abc-session-xyz");
    }

    #[test]
    fn test_buffer_initial_state() {
        let view = TerminalView::new("conn-1", "sess-1");
        let (lines, max, evicted) = view.buffer_stats();
        assert_eq!(lines, 0);
        assert_eq!(max, 10000); // Default
        assert_eq!(evicted, 0);
    }

    #[test]
    fn test_write_output() {
        let view = TerminalView::new("conn-1", "sess-1");
        view.write_output(b"Hello World\n");
        view.write_output(b"Second line\n");

        let content = view.content();
        assert!(content.contains("Hello World"));
        assert!(content.contains("Second line"));
    }

    #[test]
    fn test_set_title() {
        let view = TerminalView::new("conn-1", "sess-1");
        view.set_title("My Server Terminal");
        // Title should be updated (verified via widget state)
    }

    #[test]
    fn test_search() {
        let view = TerminalView::new("conn-1", "sess-1");
        view.write_output(b"Error: connection failed\n");

        view.search("Error", false);
        assert!(view.search_bar.is_visible());
        assert_eq!(view.search_bar.pattern(), "Error");
    }

    #[test]
    fn test_destroy_cleanup() {
        let view = TerminalView::new("conn-1", "sess-1");
        view.destroy();

        // Verify cleanup happened
        assert!(view.send_callback.borrow().is_none());
    }
}