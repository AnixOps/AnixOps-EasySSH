#![allow(dead_code)]

//! WebGL Terminal Integration for egui with Copy-Paste Support
//!
//! This module provides integration between wry WebView and egui,
//! enabling 60fps WebGL terminal rendering within the native UI.
//! Includes full clipboard support (Ctrl+C/Ctrl+V) and context menu.
//!
//! IPC Communication:
//! - Rust -> JS: terminal output, resize events via evaluate_script
//! - JS -> Rust: user input, resize requests via window.ipc.postMessage

use std::sync::{Arc, Mutex};
use std::time::Instant;

use egui::{Color32, Key, Response, Rounding, Stroke, Ui};
use raw_window_handle::HasWindowHandle;
use tracing::{debug, info, warn};
use wry::{http::Request, Rect as WryRect, WebView, WebViewBuilder};

use super::clipboard::SharedClipboard;
use super::webgl_terminal::{RenderStats, TerminalConfig, WebGlTerminal};

/// Message types for WebView <-> egui communication
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum TerminalMessage {
    Input(String),
    Binary(Vec<u8>),
    SelectionChange(String),
    RenderStats(RenderStats),
    Resize {
        cols: usize,
        rows: usize,
    },
    Selection(String),
    Options(serde_json::Value),
    Ready {
        cols: usize,
        rows: usize,
    },
    ClipboardRequest {
        action: ClipboardAction,
    },
    ClipboardResponse {
        action: ClipboardAction,
        data: Option<String>,
    },
    /// WebGL context lost - need fallback
    WebGLContextLost,
    /// Fallback mode activated
    FallbackModeActivated,
}

/// Clipboard actions
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ClipboardAction {
    Copy,
    Paste,
}

/// Context menu actions for terminal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuAction {
    Copy,
    Paste,
    SelectAll,
    Clear,
}

/// WebView state for terminal
pub struct WebViewState {
    /// Whether WebView is initialized
    pub is_initialized: bool,
    /// Whether WebGL is available
    pub webgl_available: bool,
    /// Whether using fallback (Canvas2D)
    pub using_fallback: bool,
    /// Initialization error if any
    pub init_error: Option<String>,
}

impl WebViewState {
    pub fn new() -> Self {
        Self {
            is_initialized: false,
            webgl_available: true,
            using_fallback: false,
            init_error: None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.is_initialized
    }

    pub fn using_fallback(&self) -> bool {
        self.using_fallback
    }

    pub fn get_error(&self) -> Option<&str> {
        self.init_error.as_deref()
    }
}

/// High-performance WebGL terminal widget for egui with clipboard support
pub struct EguiWebGlTerminal {
    terminal: Arc<Mutex<WebGlTerminal>>,
    webview: Option<WebView>,
    message_queue: Arc<Mutex<Vec<TerminalMessage>>>,
    last_message_id: u64,
    pending_output: Arc<Mutex<Vec<String>>>,
    dimensions: (usize, usize),
    ready: bool,
    render_stats: RenderStats,
    last_stats_update: Instant,
    selection: String,
    clipboard: SharedClipboard,
    show_context_menu: bool,
    context_menu_pos: Option<egui::Pos2>,
    last_clipboard_paste: Option<String>,
}

impl EguiWebGlTerminal {
    /// Create new WebGL terminal widget with clipboard support
    pub fn new(config: TerminalConfig) -> Self {
        let terminal = Arc::new(Mutex::new(WebGlTerminal::new(config)));
        let message_queue = Arc::new(Mutex::new(Vec::new()));
        let pending_output = Arc::new(Mutex::new(Vec::new()));
        let clipboard = SharedClipboard::new();

        Self {
            terminal,
            webview: None,
            message_queue,
            last_message_id: 0,
            pending_output,
            dimensions: (80, 24),
            ready: false,
            render_stats: RenderStats::default(),
            last_stats_update: Instant::now(),
            selection: String::new(),
            clipboard,
            show_context_menu: false,
            context_menu_pos: None,
            last_clipboard_paste: None,
        }
    }

    /// Create with default configuration
    pub fn default_terminal() -> Self {
        Self::new(TerminalConfig::default())
    }

    /// Initialize WebView for this terminal with proper wry 0.46+ API
    ///
    /// This creates a child WebView with:
    /// - IPC handler for bidirectional communication
    /// - Clipboard access enabled
    /// - DevTools in debug builds
    /// - WebGL terminal HTML content
    pub fn init_webview(
        &mut self,
        window: &impl HasWindowHandle,
        rect: WryRect,
    ) -> anyhow::Result<()> {
        if self.webview.is_some() {
            return Ok(());
        }

        let html = self.terminal.lock().unwrap().get_webview_html().to_owned();
        let message_queue = self.message_queue.clone();

        // Create WebView with IPC handler for bidirectional communication
        let webview = WebViewBuilder::new()
            .with_html(html)
            .with_clipboard(true)
            .with_bounds(rect)
            .with_ipc_handler(move |request: Request<String>| {
                // Handle incoming IPC messages from JavaScript
                let body = request.body();
                if let Ok(msg) = serde_json::from_str::<TerminalMessage>(body) {
                    if let Ok(mut queue) = message_queue.lock() {
                        queue.push(msg);
                    }
                }
            })
            // Enable devtools in debug builds
            .with_devtools(cfg!(debug_assertions))
            .build_as_child(window)?;

        self.webview = Some(webview);
        info!("WebView terminal initialized successfully with IPC handler");

        // Flush any pending output that was queued before WebView was ready
        self.flush_pending();

        Ok(())
    }

    /// Initialize WebView with fallback (Canvas2D) mode
    /// Used when WebGL fails or is not available
    pub fn init_webview_fallback(
        &mut self,
        window: &impl HasWindowHandle,
        rect: WryRect,
    ) -> anyhow::Result<()> {
        if self.webview.is_some() {
            return Ok(());
        }

        warn!("Initializing WebView with Canvas2D fallback mode");

        let fallback_html = self.terminal.lock().unwrap().get_fallback_html();
        let message_queue = self.message_queue.clone();

        let webview = WebViewBuilder::new()
            .with_html(fallback_html)
            .with_clipboard(true)
            .with_bounds(rect)
            .with_ipc_handler(move |request: Request<String>| {
                let body = request.body();
                if let Ok(msg) = serde_json::from_str::<TerminalMessage>(body) {
                    if let Ok(mut queue) = message_queue.lock() {
                        queue.push(msg);
                    }
                }
            })
            .with_devtools(cfg!(debug_assertions))
            .build_as_child(window)?;

        self.webview = Some(webview);
        info!("WebView terminal initialized with Canvas2D fallback");

        self.flush_pending();
        Ok(())
    }

    /// Try to initialize WebView, falling back to Canvas2D if WebGL fails
    pub fn init_webview_with_fallback(
        &mut self,
        window: &impl HasWindowHandle,
        rect: WryRect,
    ) -> anyhow::Result<()> {
        // First try WebGL mode
        match self.init_webview(window, rect) {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!(
                    "WebGL WebView initialization failed: {}, trying fallback",
                    e
                );
                // Fall back to Canvas2D mode
                self.init_webview_fallback(window, rect)
            }
        }
    }

    /// Check if WebView is initialized and ready
    pub fn is_webview_ready(&self) -> bool {
        self.webview.is_some() && self.ready
    }

    /// Get WebView state information
    pub fn webview_state(&self) -> WebViewState {
        WebViewState {
            is_initialized: self.webview.is_some(),
            webgl_available: !self.using_fallback_mode(),
            using_fallback: self.using_fallback_mode(),
            init_error: None,
        }
    }

    /// Check if using fallback rendering mode
    fn using_fallback_mode(&self) -> bool {
        // This would be set based on messages from the JS side
        false
    }

    /// Resize WebView bounds
    pub fn resize_webview(&mut self, rect: WryRect) -> anyhow::Result<()> {
        if let Some(webview) = &self.webview {
            webview.set_bounds(rect)?;
        }
        Ok(())
    }

    /// Send message to WebView via evaluate_script
    fn send_to_webview(&self, msg: &serde_json::Value) -> anyhow::Result<()> {
        if let Some(webview) = &self.webview {
            let script = format!("window.postMessage({}, '*');", msg);
            webview.evaluate_script(&script)?;
        }
        Ok(())
    }

    /// Write data to terminal
    pub fn write(&mut self, data: &str) {
        if let Some(webview) = &self.webview {
            let msg = serde_json::json!({
                "type": "write",
                "data": data
            });
            if let Err(e) = self.send_to_webview(&msg) {
                warn!("Failed to write to WebView: {}", e);
                // Queue for retry
                if let Ok(mut queue) = self.pending_output.lock() {
                    queue.push(data.to_string());
                }
            }
        } else {
            // Queue for later
            if let Ok(mut queue) = self.pending_output.lock() {
                queue.push(data.to_string());
            }
        }

        self.terminal.lock().unwrap().write(data);
    }

    /// Write line to terminal
    pub fn writeln(&mut self, data: &str) {
        self.write(data);
        self.write("\r\n");
    }

    /// Clear terminal
    pub fn clear(&mut self) {
        let msg = serde_json::json!({"type": "clear"});
        if let Err(e) = self.send_to_webview(&msg) {
            warn!("Failed to clear WebView: {}", e);
        }
        self.terminal.lock().unwrap().clear();
    }

    /// Reset terminal
    pub fn reset(&mut self) {
        let msg = serde_json::json!({"type": "reset"});
        if let Err(e) = self.send_to_webview(&msg) {
            warn!("Failed to reset WebView: {}", e);
        }
        self.terminal.lock().unwrap().reset();
    }

    /// Focus terminal
    pub fn focus(&mut self) {
        if let Some(webview) = &self.webview {
            if let Err(e) = webview.focus() {
                warn!("Failed to focus WebView: {}", e);
            }
        }
        let msg = serde_json::json!({"type": "focus"});
        let _ = self.send_to_webview(&msg);
    }

    /// Blur terminal (remove focus)
    pub fn blur(&mut self) {
        let msg = serde_json::json!({"type": "blur"});
        let _ = self.send_to_webview(&msg);
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        let msg = serde_json::json!({"type": "scrollToBottom"});
        let _ = self.send_to_webview(&msg);
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        let msg = serde_json::json!({"type": "scrollToTop"});
        let _ = self.send_to_webview(&msg);
    }

    /// Scroll by specific lines
    pub fn scroll_lines(&mut self, lines: i32) {
        let msg = serde_json::json!({"type": "scrollLines", "data": {"lines": lines}});
        let _ = self.send_to_webview(&msg);
    }

    /// Get all pending messages from terminal
    pub fn poll_messages(&mut self) -> Vec<TerminalMessage> {
        if let Ok(mut queue) = self.message_queue.lock() {
            queue.drain(..).collect()
        } else {
            Vec::new()
        }
    }

    /// Get current selection
    pub fn get_selection(&self) -> &str {
        &self.selection
    }

    /// Select all text in terminal
    pub fn select_all(&mut self) {
        let msg = serde_json::json!({"type": "selectAll"});
        let _ = self.send_to_webview(&msg);
    }

    /// Copy current selection to clipboard
    pub fn copy_selection(&mut self) -> Result<(), String> {
        if self.selection.is_empty() {
            // If no local selection, request from webview
            self.request_selection_from_webview();
            return Err("No selection available - requesting from WebView".to_string());
        }

        self.clipboard.copy(&self.selection)
    }

    /// Paste from clipboard to terminal
    pub fn paste_from_clipboard(&mut self) -> Result<(), String> {
        match self.clipboard.paste() {
            Ok(text) => {
                // Send pasted text as terminal input
                self.send_input(&text);
                self.last_clipboard_paste = Some(text);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Request selection from webview (for copy)
    fn request_selection_from_webview(&self) {
        let msg = serde_json::json!({"type": "getSelection"});
        let _ = self.send_to_webview(&msg);
    }

    /// Send input to terminal (from SSH or paste)
    fn send_input(&mut self, data: &str) {
        // Use paste method for proper input handling
        let msg = serde_json::json!({"type": "paste", "data": data});
        if let Err(e) = self.send_to_webview(&msg) {
            warn!("Failed to send input to WebView: {}", e);
        }
        debug!("Sent input to terminal: {} bytes", data.len());
    }

    /// Get render stats
    pub fn render_stats(&self) -> &RenderStats {
        &self.render_stats
    }

    /// Check if terminal is ready
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Check if clipboard is available
    pub fn clipboard_available(&self) -> bool {
        self.clipboard.is_available()
    }

    /// Resize terminal dimensions
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.dimensions = (cols, rows);
        self.terminal.lock().unwrap().resize(cols, rows);

        let msg = serde_json::json!({
            "type": "resize",
            "data": {"cols": cols, "rows": rows}
        });
        let _ = self.send_to_webview(&msg);
    }

    /// Flush any pending output that was queued before WebView was ready
    fn flush_pending(&mut self) {
        let pending: Vec<String> = {
            if let Ok(mut queue) = self.pending_output.lock() {
                queue.drain(..).collect()
            } else {
                Vec::new()
            }
        };

        for data in pending {
            self.write(&data);
        }
    }

    /// Handle keyboard shortcuts for copy-paste
    pub fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        let mut handled = false;

        ctx.input(|i| {
            // Ctrl+C - Copy selection (when text is selected)
            if i.key_pressed(Key::C) && i.modifiers.ctrl && !self.selection.is_empty() {
                if let Err(e) = self.copy_selection() {
                    warn!("Copy failed: {}", e);
                } else {
                    info!("Copied {} characters to clipboard", self.selection.len());
                    handled = true;
                }
            }

            // Ctrl+V - Paste from clipboard
            if i.key_pressed(Key::V) && i.modifiers.ctrl {
                if let Err(e) = self.paste_from_clipboard() {
                    warn!("Paste failed: {}", e);
                } else {
                    handled = true;
                }
            }

            // Ctrl+A - Select all
            if i.key_pressed(Key::A) && i.modifiers.ctrl {
                self.select_all();
                handled = true;
            }

            // Ctrl+Shift+C - Copy (alternative shortcut, works even without selection)
            if i.key_pressed(Key::C) && i.modifiers.ctrl && i.modifiers.shift {
                self.request_selection_from_webview();
                handled = true;
            }

            // Ctrl+Shift+V - Paste (alternative shortcut)
            if i.key_pressed(Key::V) && i.modifiers.ctrl && i.modifiers.shift {
                if let Err(e) = self.paste_from_clipboard() {
                    warn!("Paste failed: {}", e);
                } else {
                    handled = true;
                }
            }
        });

        handled
    }

    /// Show context menu at position
    pub fn show_context_menu(&mut self, pos: egui::Pos2) {
        self.context_menu_pos = Some(pos);
        self.show_context_menu = true;
    }

    /// Render context menu if visible
    fn render_context_menu(&mut self, ctx: &egui::Context) {
        if !self.show_context_menu {
            return;
        }

        let screen_pos = self.context_menu_pos.unwrap_or(ctx.screen_rect().center());

        egui::Area::new(egui::Id::new("terminal_context_menu"))
            .fixed_pos(screen_pos)
            .order(egui::Order::Foreground)
            .interactable(true)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(Color32::from_rgb(35, 40, 48))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(60, 65, 75)))
                    .show(ui, |ui| {
                        ui.set_min_width(120.0);

                        let has_selection = !self.selection.is_empty();
                        let can_paste = self.clipboard.is_available();

                        // Copy option
                        ui.add_enabled_ui(has_selection, |ui| {
                            if ui.button("📋  Copy").clicked() {
                                if let Err(e) = self.copy_selection() {
                                    warn!("Copy from context menu failed: {}", e);
                                }
                                self.show_context_menu = false;
                            }
                        });

                        // Paste option
                        ui.add_enabled_ui(can_paste, |ui| {
                            if ui.button("📄  Paste").clicked() {
                                if let Err(e) = self.paste_from_clipboard() {
                                    warn!("Paste from context menu failed: {}", e);
                                }
                                self.show_context_menu = false;
                            }
                        });

                        ui.separator();

                        // Select All
                        if ui.button("☰  Select All").clicked() {
                            self.select_all();
                            self.show_context_menu = false;
                        }

                        // Clear
                        if ui.button("🗑  Clear").clicked() {
                            self.clear();
                            self.show_context_menu = false;
                        }

                        ui.separator();

                        // Cancel
                        if ui.button("✕  Cancel").clicked() {
                            self.show_context_menu = false;
                        }
                    });
            });

        // Close menu when clicking outside
        if ctx.input(|i| i.pointer.any_click()) {
            self.show_context_menu = false;
        }
    }
}

impl Drop for EguiWebGlTerminal {
    fn drop(&mut self) {
        if let Some(webview) = self.webview.take() {
            // WebView will be dropped automatically
            drop(webview);
        }
    }
}

/// WebGL Terminal widget for egui with full copy-paste support
pub struct WebGlTerminalWidget {
    terminal: Arc<Mutex<EguiWebGlTerminal>>,
    id: egui::Id,
}

impl WebGlTerminalWidget {
    pub fn new(terminal: Arc<Mutex<EguiWebGlTerminal>>) -> Self {
        Self {
            terminal,
            id: egui::Id::new("webgl_terminal"),
        }
    }

    pub fn with_id(mut self, id: egui::Id) -> Self {
        self.id = id;
        self
    }

    /// Show the terminal widget with copy-paste support
    pub fn show(self, ui: &mut Ui) -> Response {
        let available_size = ui.available_size();

        // Calculate terminal dimensions based on font metrics
        let font_size = 14.0;
        let line_height = font_size * 1.2;
        let char_width = font_size * 0.6;

        let cols = (available_size.x / char_width) as usize;
        let rows = (available_size.y / line_height) as usize;

        // Ensure minimum dimensions
        let cols = cols.max(40);
        let rows = rows.max(10);

        // Create container frame for terminal
        let response = ui.allocate_response(available_size, egui::Sense::click_and_drag());

        let rect = response.rect;

        // Paint terminal background
        ui.painter()
            .rect_filled(rect, Rounding::same(4.0), Color32::from_rgb(22, 25, 30));

        // Paint terminal border (highlight if focused)
        let border_color = if response.has_focus() {
            Color32::from_rgb(100, 150, 255)
        } else {
            Color32::from_rgb(50, 55, 65)
        };

        ui.painter().rect_stroke(
            rect,
            Rounding::same(4.0),
            Stroke::new(if response.has_focus() { 2.0 } else { 1.0 }, border_color),
        );

        // Get mutable access to terminal
        let mut term = self.terminal.lock().unwrap();

        // Resize terminal if needed
        let (current_cols, current_rows) = term.dimensions;
        if cols != current_cols || rows != current_rows {
            term.resize(cols, rows);
        }

        // Flush pending output
        term.flush_pending();

        // Poll messages
        let messages = term.poll_messages();
        for msg in messages {
            match msg {
                TerminalMessage::SelectionChange(text) => {
                    term.selection = text;
                }
                TerminalMessage::RenderStats(stats) => {
                    term.render_stats = stats;
                }
                TerminalMessage::Ready { cols, rows } => {
                    term.ready = true;
                    term.dimensions = (cols, rows);
                    info!("WebView terminal ready: {} cols x {} rows", cols, rows);
                }
                TerminalMessage::Selection(text) => {
                    // Selection received from webview, copy to clipboard
                    term.selection = text.clone();
                    if let Err(e) = term.clipboard.copy(&text) {
                        warn!("Failed to copy selection from webview: {}", e);
                    } else {
                        info!("Copied selection from webview: {} chars", text.len());
                    }
                }
                TerminalMessage::WebGLContextLost => {
                    warn!("WebGL context lost in terminal");
                    // Could trigger fallback here if needed
                }
                TerminalMessage::FallbackModeActivated => {
                    warn!("Terminal switched to fallback rendering mode");
                }
                TerminalMessage::Input(data) => {
                    // Input from terminal - this should be forwarded to SSH session
                    debug!("Terminal input: {} bytes", data.len());
                }
                TerminalMessage::Resize { cols, rows } => {
                    debug!("Terminal resize request: {} x {}", cols, rows);
                    term.dimensions = (cols, rows);
                }
                _ => {}
            }
        }

        // Handle keyboard shortcuts first
        let ctx = ui.ctx();
        let _shortcuts_handled = term.handle_keyboard_shortcuts(ctx);

        // Focus terminal on click
        if response.clicked() && !response.dragged() {
            term.focus();
            ui.memory_mut(|mem| mem.request_focus(response.id));
        }

        // Handle right-click for context menu - do this while we still have term lock
        let show_menu = response.secondary_clicked();
        let menu_pos = if show_menu {
            Some(
                ctx.input(|i| i.pointer.interact_pos())
                    .unwrap_or(rect.center()),
            )
        } else {
            None
        };

        // Drop the lock before showing context menu
        drop(term);

        // Now show context menu without holding the lock
        if let Some(pos) = menu_pos {
            let mut term = self.terminal.lock().unwrap();
            term.show_context_menu(pos);
            drop(term);
        }

        // Render context menu if visible (get fresh lock)
        {
            let mut term = self.terminal.lock().unwrap();
            term.render_context_menu(ctx);
        }

        // Request continuous repaint for 60fps
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(16));

        response
    }
}

/// Builder for WebGL terminal integration
pub struct WebGlTerminalBuilder {
    config: TerminalConfig,
}

impl WebGlTerminalBuilder {
    pub fn new() -> Self {
        Self {
            config: TerminalConfig::default(),
        }
    }

    pub fn with_config(config: TerminalConfig) -> Self {
        Self { config }
    }

    pub fn color_support(mut self, support: super::webgl_terminal::ColorSupport) -> Self {
        self.config.color_support = support;
        self
    }

    pub fn font_family(mut self, family: &str) -> Self {
        self.config.font.family = family.to_string();
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.config.font.size = size;
        self
    }

    pub fn cursor_blink(mut self, blink: bool) -> Self {
        self.config.cursor.blink = blink;
        self
    }

    pub fn scrollback(mut self, lines: usize) -> Self {
        self.config.scrollback_lines = lines;
        self
    }

    pub fn target_fps(mut self, fps: u32) -> Self {
        self.config.target_fps = fps;
        self
    }

    pub fn dimensions(mut self, cols: usize, rows: usize) -> Self {
        self.config.cols = cols;
        self.config.rows = rows;
        self
    }

    pub fn build(self) -> Arc<Mutex<EguiWebGlTerminal>> {
        Arc::new(Mutex::new(EguiWebGlTerminal::new(self.config)))
    }
}

impl Default for WebGlTerminalBuilder {
    fn default() -> Self {
        Self::new()
    }
}
