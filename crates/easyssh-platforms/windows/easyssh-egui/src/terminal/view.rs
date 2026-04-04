//! Terminal View - egui Widget for Embedded Terminal
//!
//! Provides the main terminal widget for egui with:
//! - Key-driven reset pattern (connection_id-session_id)
//! - Output handling without blocking UI
//! - Selection and clipboard support
//! - Search functionality
//! - Scroll behavior

use std::sync::{Arc, Mutex};
use std::time::Instant;

use egui::{Color32, Key, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui};
use tracing::debug;

use super::buffer::{TerminalBuffer, ColorScheme};
use super::renderer::TerminalRenderer;
use super::{SearchMatch, TerminalFontConfig};

/// Unique key format for terminal view: `{connection_id}-{session_id}`
pub const KEY_FORMAT: &str = "{connection_id}-{session_id}";

/// Terminal configuration
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    /// Font configuration
    pub font: TerminalFontConfig,
    /// Color scheme
    pub colors: ColorScheme,
    /// Scrollback buffer size
    pub scrollback_lines: usize,
    /// Cursor configuration
    pub cursor: super::CursorConfig,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            font: TerminalFontConfig::default(),
            colors: ColorScheme::dark(),
            scrollback_lines: 10000,
            cursor: super::CursorConfig::default(),
        }
    }
}

/// Terminal view widget for egui
pub struct TerminalView {
    /// Unique ID for this terminal: `{connection_id}-{session_id}`
    id: String,
    /// Connection ID this terminal belongs to
    connection_id: String,
    /// Session ID for this terminal instance
    session_id: String,
    /// Terminal buffer containing all content
    buffer: Arc<Mutex<TerminalBuffer>>,
    /// Renderer for painting terminal content
    renderer: TerminalRenderer,
    /// Configuration
    config: TerminalConfig,
    /// Current scroll offset (in lines)
    scroll_offset: f32,
    /// Cursor position (col, row)
    cursor_pos: (usize, usize),
    /// Whether this terminal is focused
    focused: bool,
    /// Current selection (start, end positions)
    selection: Option<Selection>,
    /// Search query (if searching)
    search_query: Option<String>,
    /// Search results
    search_results: Vec<SearchMatch>,
    /// Current search result index
    search_result_idx: usize,
    /// Whether search uses regex
    search_use_regex: bool,
    /// Pending output to process
    pending_output: Arc<Mutex<Vec<Vec<u8>>>>,
    /// Last render time for cursor blink
    last_render: Instant,
    /// Output channel receiver (if connected)
    output_rx: Option<Arc<Mutex<Vec<u8>>>>,
    /// Dimensions cache
    dimensions: (usize, usize),
    /// Clipboard state
    clipboard_text: Option<String>,
}

/// Selection in terminal
#[derive(Debug, Clone, Copy)]
pub(super) struct Selection {
    /// Start position (line, col)
    start: (usize, usize),
    /// End position (line, col)
    end: (usize, usize),
}

impl Selection {
    /// Check if a position is within selection
    pub(super) fn contains(&self, line: usize, col: usize) -> bool {
        let start_after_end = self.start.0 > self.end.0
            || (self.start.0 == self.end.0 && self.start.1 > self.end.1);

        let (min, max) = if start_after_end {
            (self.end, self.start)
        } else {
            (self.start, self.end)
        };

        if line < min.0 || line > max.0 {
            return false;
        }

        if line == min.0 && col < min.1 {
            return false;
        }

        if line == max.0 && col > max.1 {
            return false;
        }

        true
    }

    /// Get ordered selection bounds (min, max)
    fn ordered(&self) -> ((usize, usize), (usize, usize)) {
        if self.start.0 > self.end.0
            || (self.start.0 == self.end.0 && self.start.1 > self.end.1)
        {
            (self.end, self.start)
        } else {
            (self.start, self.end)
        }
    }
}

impl TerminalView {
    /// Create new terminal view with unique ID
    ///
    /// # Arguments
    /// * `connection_id` - SSH connection ID
    /// * `session_id` - Terminal session ID
    ///
    /// # Key Format
    /// The ID is formatted as `{connection_id}-{session_id}` following
    /// the Key-Driven Reset pattern from SYSTEM_INVARIANTS.md
    pub fn new(connection_id: &str, session_id: &str) -> Self {
        let id = format!("{}-{}", connection_id, session_id);
        let config = TerminalConfig::default();
        let buffer = Arc::new(Mutex::new(TerminalBuffer::new(config.scrollback_lines)));
        let renderer = TerminalRenderer::new(&config);

        Self {
            id,
            connection_id: connection_id.to_string(),
            session_id: session_id.to_string(),
            buffer,
            renderer,
            config,
            scroll_offset: 0.0,
            cursor_pos: (0, 0),
            focused: false,
            selection: None,
            search_query: None,
            search_results: Vec::new(),
            search_result_idx: 0,
            search_use_regex: false,
            pending_output: Arc::new(Mutex::new(Vec::new())),
            last_render: Instant::now(),
            output_rx: None,
            dimensions: (80, 24),
            clipboard_text: None,
        }
    }

    /// Create with custom configuration
    pub fn with_config(connection_id: &str, session_id: &str, config: TerminalConfig) -> Self {
        let id = format!("{}-{}", connection_id, session_id);
        let buffer = Arc::new(Mutex::new(TerminalBuffer::new(config.scrollback_lines)));
        let renderer = TerminalRenderer::new(&config);

        Self {
            id,
            connection_id: connection_id.to_string(),
            session_id: session_id.to_string(),
            buffer,
            renderer,
            config,
            scroll_offset: 0.0,
            cursor_pos: (0, 0),
            focused: false,
            selection: None,
            search_query: None,
            search_results: Vec::new(),
            search_result_idx: 0,
            search_use_regex: false,
            pending_output: Arc::new(Mutex::new(Vec::new())),
            last_render: Instant::now(),
            output_rx: None,
            dimensions: (80, 24),
            clipboard_text: None,
        }
    }

    /// Get terminal ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get connection ID
    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    /// Write output to terminal (non-blocking)
    ///
    /// Output is queued and processed during render to avoid
    /// blocking the UI thread.
    pub fn write_output(&mut self, data: &[u8]) {
        if let Ok(mut pending) = self.pending_output.lock() {
            pending.push(data.to_vec());
        }
    }

    /// Resize terminal dimensions
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.dimensions = (cols as usize, rows as usize);
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.resize(cols as usize, rows as usize);
        }
    }

    /// Handle user input (keyboard events)
    pub fn handle_input(&mut self, input: &str) {
        // This would normally send to SSH session
        debug!("Terminal input: {:?}", input);
    }

    /// Scroll to bottom of buffer
    pub fn scroll_to_bottom(&mut self) {
        if let Ok(buffer) = self.buffer.lock() {
            let line_count = buffer.line_count();
            self.scroll_offset = (line_count.saturating_sub(self.dimensions.1)) as f32;
        }
    }

    /// Scroll to top of buffer
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0.0;
    }

    /// Scroll by delta lines
    pub fn scroll_by(&mut self, delta: f32) {
        if let Ok(buffer) = self.buffer.lock() {
            let max_scroll = buffer.line_count().saturating_sub(self.dimensions.1) as f32;
            self.scroll_offset = (self.scroll_offset + delta).clamp(0.0, max_scroll);
        }
    }

    /// Start search in terminal buffer
    pub fn start_search(&mut self, query: &str, use_regex: bool) {
        self.search_query = Some(query.to_string());
        self.search_use_regex = use_regex;
        self.search_results.clear();
        self.search_result_idx = 0;

        if let Ok(buffer) = self.buffer.lock() {
            self.search_results = buffer.search(query, use_regex);
        }
    }

    /// Clear search
    pub fn clear_search(&mut self) {
        self.search_query = None;
        self.search_results.clear();
        self.search_result_idx = 0;
    }

    /// Navigate to next search result
    pub fn next_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.search_result_idx = (self.search_result_idx + 1) % self.search_results.len();
            self.scroll_to_search_result();
        }
    }

    /// Navigate to previous search result
    pub fn prev_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.search_result_idx = if self.search_result_idx == 0 {
                self.search_results.len() - 1
            } else {
                self.search_result_idx - 1
            };
            self.scroll_to_search_result();
        }
    }

    /// Scroll to current search result
    fn scroll_to_search_result(&mut self) {
        if let Some(result) = self.search_results.get(self.search_result_idx) {
            let target_line = result.line;
            let visible_start = self.scroll_offset as usize;
            let visible_end = visible_start + self.dimensions.1;

            if target_line < visible_start {
                self.scroll_offset = target_line as f32;
            } else if target_line >= visible_end {
                self.scroll_offset = (target_line.saturating_sub(self.dimensions.1 / 2)) as f32;
            }
        }
    }

    /// Copy selection to clipboard
    pub fn copy_selection(&mut self) -> Option<String> {
        if let Some(selection) = &self.selection {
            let (min, max) = selection.ordered();

            if let Ok(buffer) = self.buffer.lock() {
                let mut text = String::new();

                for line_idx in min.0..=max.0 {
                    if let Some(line) = buffer.get_line(line_idx) {
                        let start_col = if line_idx == min.0 { min.1 } else { 0 };
                        let end_col = if line_idx == max.0 { max.1 } else { line.len() };

                        let cells = line.cells();
                        for col in start_col..end_col {
                            if let Some(cell) = cells.get(col) {
                                text.push(cell.char);
                            }
                        }

                        if line_idx < max.0 && !line.is_wrapped() {
                            text.push('\n');
                        }
                    }
                }

                self.clipboard_text = Some(text.clone());
                return Some(text);
            }
        }
        None
    }

    /// Paste from clipboard
    pub fn paste(&mut self, text: &str) {
        // This would normally send to SSH session
        debug!("Paste to terminal: {} bytes", text.len());
    }

    /// Clear terminal content
    pub fn clear(&mut self) {
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.clear();
        }
        self.scroll_offset = 0.0;
        self.selection = None;
        self.search_results.clear();
    }

    /// Process pending output (non-blocking)
    fn process_pending_output(&mut self) {
        let pending: Vec<Vec<u8>> = {
            if let Ok(mut queue) = self.pending_output.lock() {
                queue.drain(..).collect()
            } else {
                Vec::new()
            }
        };

        if !pending.is_empty() {
            if let Ok(mut buffer) = self.buffer.lock() {
                for data in pending {
                    buffer.write(&data);
                }
            }

            // Auto-scroll to bottom on new output if at bottom
            if self.is_at_bottom() {
                self.scroll_to_bottom();
            }
        }
    }

    /// Check if currently scrolled to bottom
    fn is_at_bottom(&self) -> bool {
        if let Ok(buffer) = self.buffer.lock() {
            let max_scroll = buffer.line_count().saturating_sub(self.dimensions.1) as f32;
            self.scroll_offset >= max_scroll - 1.0
        } else {
            false
        }
    }

    /// Show terminal widget in egui UI
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        // Process pending output first
        self.process_pending_output();

        // Get available space
        let available = ui.available_size();

        // Calculate dimensions based on font metrics
        let cell_size = self.renderer.cell_size();
        let cols = (available.x / cell_size.x).floor() as usize;
        let rows = (available.y / cell_size.y).floor() as usize;

        // Ensure minimum dimensions
        let cols = cols.max(40);
        let rows = rows.max(10);

        // Update dimensions if changed
        if cols != self.dimensions.0 || rows != self.dimensions.1 {
            self.resize(cols as u16, rows as u16);
        }

        // Allocate response
        let response = ui.allocate_response(available, Sense::click_and_drag());

        let rect = response.rect;

        // Render terminal background
        ui.painter().rect_filled(
            rect,
            Rounding::same(4.0),
            self.config.colors.default_bg,
        );

        // Render terminal border (highlighted if focused)
        let border_color = if self.focused {
            Color32::from_rgb(100, 150, 255)
        } else {
            Color32::from_rgb(50, 55, 65)
        };

        ui.painter().rect_stroke(
            rect,
            Rounding::same(4.0),
            Stroke::new(if self.focused { 2.0 } else { 1.0 }, border_color),
        );

        // Render terminal content
        if let Ok(buffer) = self.buffer.lock() {
            self.renderer.render(
                ui,
                &buffer,
                self.scroll_offset as usize,
                rows,
                self.focused,
                &self.selection,
                self.cursor_pos,
                &self.search_results,
                self.search_result_idx,
                self.last_render,
            );

            // Update cursor from buffer
            self.cursor_pos = buffer.cursor_position();
        }

        // Handle keyboard shortcuts
        self.handle_keyboard(ui.ctx());

        // Handle focus
        if response.clicked() {
            self.focused = true;
            ui.memory_mut(|mem| mem.request_focus(response.id));
        }

        // Handle scroll
        if response.dragged() {
            let drag_delta = response.drag_delta().y;
            self.scroll_by(-drag_delta / cell_size.y);
        }

        // Handle mouse scroll wheel
        let scroll_delta = ui.input(|i| i.raw_scroll_delta);
        if scroll_delta.y != 0.0 {
            self.scroll_by(-scroll_delta.y / cell_size.y);
        }

        // Handle selection with mouse drag
        if response.dragged() && ui.input(|i| i.pointer.primary_down()) {
            let pointer_pos = ui.input(|i| i.pointer.interact_pos());
            if let Some(pos) = pointer_pos {
                let (col, row) = self.position_to_cell(pos, rect);
                self.extend_selection_to(row, col);
            }
        }

        // Start selection on click
        if response.clicked() {
            let pointer_pos = ui.input(|i| i.pointer.interact_pos());
            if let Some(pos) = pointer_pos {
                let (col, row) = self.position_to_cell(pos, rect);
                self.start_selection(row, col);
            }
        }

        // Handle double-click for word selection
        if response.double_clicked() {
            let pointer_pos = ui.input(|i| i.pointer.interact_pos());
            if let Some(pos) = pointer_pos {
                let (col, row) = self.position_to_cell(pos, rect);
                self.select_word_at(row, col);
            }
        }

        // Update last render time
        self.last_render = Instant::now();

        // Request continuous repaint for smooth rendering
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));

        response
    }

    /// Convert screen position to cell position
    fn position_to_cell(&self, pos: Pos2, rect: Rect) -> (usize, usize) {
        let cell_size = self.renderer.cell_size();

        let relative = pos - rect.min;
        let col = (relative.x / cell_size.x).floor() as usize;
        let row = (self.scroll_offset as usize) + (relative.y / cell_size.y).floor() as usize;

        (col.min(self.dimensions.0 - 1), row)
    }

    /// Start selection at position
    fn start_selection(&mut self, row: usize, col: usize) {
        self.selection = Some(Selection {
            start: (row, col),
            end: (row, col),
        });
    }

    /// Extend selection to position
    fn extend_selection_to(&mut self, row: usize, col: usize) {
        if let Some(selection) = &mut self.selection {
            selection.end = (row, col);
        } else {
            self.start_selection(row, col);
        }
    }

    /// Select word at position
    fn select_word_at(&mut self, row: usize, col: usize) {
        if let Ok(buffer) = self.buffer.lock() {
            if let Some(line) = buffer.get_line(row) {
                let cells = line.cells();

                // Find word boundaries
                let mut start = col;
                let mut end = col;

                // Expand left
                while start > 0 {
                    if let Some(cell) = cells.get(start - 1) {
                        if cell.char.is_whitespace() {
                            break;
                        }
                        start -= 1;
                    } else {
                        break;
                    }
                }

                // Expand right
                while end < cells.len() {
                    if let Some(cell) = cells.get(end) {
                        if cell.char.is_whitespace() {
                            break;
                        }
                        end += 1;
                    } else {
                        break;
                    }
                }

                self.selection = Some(Selection {
                    start: (row, start),
                    end: (row, end),
                });
            }
        }
    }

    /// Handle keyboard shortcuts
    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Ctrl+C - Copy
            if i.key_pressed(Key::C) && i.modifiers.ctrl && self.selection.is_some() {
                if let Some(text) = self.copy_selection() {
                    debug!("Copied {} characters", text.len());
                }
            }

            // Ctrl+V - Paste
            if i.key_pressed(Key::V) && i.modifiers.ctrl {
                // Request clipboard content (would be handled by app)
                debug!("Paste requested");
            }

            // Ctrl+A - Select all
            if i.key_pressed(Key::A) && i.modifiers.ctrl {
                if let Ok(buffer) = self.buffer.lock() {
                    let line_count = buffer.line_count();
                    if line_count > 0 {
                        if let Some(last_line) = buffer.get_line(line_count - 1) {
                            self.selection = Some(Selection {
                                start: (0, 0),
                                end: (line_count - 1, last_line.len()),
                            });
                        }
                    }
                }
            }

            // Ctrl+F - Focus search
            if i.key_pressed(Key::F) && i.modifiers.ctrl {
                // App should show search panel
                debug!("Search requested");
            }

            // Escape - Clear selection/search
            if i.key_pressed(Key::Escape) {
                self.selection = None;
                self.clear_search();
            }

            // Page Up / Page Down
            if i.key_pressed(Key::PageUp) {
                self.scroll_by(-(self.dimensions.1 as f32) / 2.0);
            }
            if i.key_pressed(Key::PageDown) {
                self.scroll_by((self.dimensions.1 as f32) / 2.0);
            }

            // Home / End
            if i.key_pressed(Key::Home) {
                self.scroll_to_top();
            }
            if i.key_pressed(Key::End) {
                self.scroll_to_bottom();
            }

            // F3 / Shift+F3 - Next/Prev search result
            if i.key_pressed(Key::F3) {
                if i.modifiers.shift {
                    self.prev_search_result();
                } else {
                    self.next_search_result();
                }
            }
        });
    }
}

impl Drop for TerminalView {
    fn drop(&mut self) {
        // Clean up all handles per SYSTEM_INVARIANTS.md
        debug!("Dropping terminal view: {}", self.id);

        // Clear pending output
        if let Ok(mut pending) = self.pending_output.lock() {
            pending.clear();
        }

        // Clear selection and search state
        self.selection = None;
        self.search_results.clear();
        self.search_query = None;
    }
}

/// Trait for terminal view (for Platform abstraction)
pub trait TerminalViewTrait: Send + Sync {
    /// Get terminal ID
    fn id(&self) -> &str;

    /// Write output to terminal
    fn write_output(&mut self, data: &[u8]);

    /// Resize terminal
    fn resize(&mut self, cols: u16, rows: u16);

    /// Handle user input
    fn handle_input(&mut self, input: &str);

    /// Scroll to bottom
    fn scroll_to_bottom(&mut self);

    /// Copy selection
    fn copy_selection(&mut self) -> Option<String>;

    /// Paste content
    fn paste(&mut self, text: &str);

    /// Clear terminal
    fn clear(&mut self);
}

impl TerminalViewTrait for TerminalView {
    fn id(&self) -> &str {
        &self.id
    }

    fn write_output(&mut self, data: &[u8]) {
        self.write_output(data);
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        self.resize(cols, rows);
    }

    fn handle_input(&mut self, input: &str) {
        self.handle_input(input);
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_to_bottom();
    }

    fn copy_selection(&mut self) -> Option<String> {
        self.copy_selection()
    }

    fn paste(&mut self, text: &str) {
        self.paste(text);
    }

    fn clear(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_view_creation() {
        let view = TerminalView::new("conn-123", "sess-456");
        assert_eq!(view.id(), "conn-123-sess-456");
        assert_eq!(view.connection_id(), "conn-123");
    }

    #[test]
    fn test_key_format() {
        let view = TerminalView::new("my-connection", "my-session");
        assert!(view.id().contains("-"));
        assert!(view.id().starts_with("my-connection"));
    }

    #[test]
    fn test_write_output() {
        let mut view = TerminalView::new("conn", "sess");
        view.write_output(b"Hello, World!");
        // Output is queued internally
    }

    #[test]
    fn test_resize() {
        let mut view = TerminalView::new("conn", "sess");
        view.resize(120, 40);
        // Resize should work without panic
    }

    #[test]
    fn test_search() {
        let mut view = TerminalView::new("conn", "sess");

        // Add some content
        view.write_output(b"foo bar baz\n");
        view.write_output(b"another foo line\n");

        view.start_search("foo", false);
        // Search should work
    }

    #[test]
    fn test_drop_cleanup() {
        let mut view = TerminalView::new("conn", "sess");
        view.write_output(b"test");
        view.start_search("test", false);

        // Drop should clean up without panic
        drop(view);
    }
}