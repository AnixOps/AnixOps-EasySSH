//! Terminal Buffer - Scrollback and Line Management
//!
//! Manages terminal content with FIFO scrollback buffer.
//! Supports cell-based styling with ANSI escape sequence processing.

use std::collections::VecDeque;
use vte::{Parser, Perform, Params};
use egui::Color32;

use super::SearchMatch;

/// Maximum scrollback lines for Standard edition
const MAX_SCROLLBACK: usize = 10000;

/// Terminal buffer containing all lines and cursor state
pub struct TerminalBuffer {
    /// All lines in scrollback (FIFO when exceeding max)
    lines: VecDeque<TermLine>,
    /// Maximum number of lines to keep
    max_lines: usize,
    /// Current color scheme
    style: TerminalStyle,
    /// VTE parser for ANSI sequences
    parser: Parser,
    /// Current ANSI state machine
    ansi_state: AnsiState,
    /// Cursor position (col, row)
    cursor: (usize, usize),
    /// Terminal dimensions (cols, rows)
    dimensions: (usize, usize),
    /// Current line being built
    current_line: TermLine,
}

impl TerminalBuffer {
    /// Create new terminal buffer with default capacity
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_lines),
            max_lines,
            style: TerminalStyle::default(),
            parser: Parser::new(),
            ansi_state: AnsiState::default(),
            cursor: (0, 0),
            dimensions: (80, 24),
            current_line: TermLine::new(),
        }
    }

    /// Create with default scrollback (10000 lines)
    pub fn default_capacity() -> Self {
        Self::new(MAX_SCROLLBACK)
    }

    /// Write data to buffer, processing ANSI sequences
    pub fn write(&mut self, data: &[u8]) {
        for &byte in data {
            self.parser.advance(&mut self.ansi_state, byte);
        }
    }

    /// Write raw text without ANSI processing
    pub fn write_raw(&mut self, text: &str) {
        for ch in text.chars() {
            self.put_char(ch, self.style.default_cell_style());
        }
    }

    /// Resize terminal dimensions
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.dimensions = (cols, rows);
    }

    /// Get total number of lines in buffer
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Get visible lines for rendering (start offset, count)
    pub fn get_visible_lines(&self, start: usize, count: usize) -> Vec<&TermLine> {
        self.lines
            .iter()
            .skip(start)
            .take(count)
            .collect()
    }

    /// Get a specific line by index
    pub fn get_line(&self, index: usize) -> Option<&TermLine> {
        self.lines.get(index)
    }

    /// Get cursor position
    pub fn cursor_position(&self) -> (usize, usize) {
        self.cursor
    }

    /// Get terminal dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        self.dimensions
    }

    /// Clear all content
    pub fn clear(&mut self) {
        self.lines.clear();
        self.cursor = (0, 0);
        self.current_line = TermLine::new();
    }

    /// Clear current line
    pub fn clear_line(&mut self) {
        self.current_line.clear();
        self.cursor.0 = 0;
    }

    /// Search for pattern in buffer
    pub fn search(&self, pattern: &str, use_regex: bool) -> Vec<SearchMatch> {
        let matches = Vec::new();

        if pattern.is_empty() {
            return matches;
        }

        if use_regex {
            self.search_regex(pattern)
        } else {
            self.search_literal(pattern)
        }
    }

    /// Search with literal matching
    fn search_literal(&self, pattern: &str) -> Vec<SearchMatch> {
        let mut matches = Vec::new();
        let pattern_lower = pattern.to_lowercase();

        for (line_idx, line) in self.lines.iter().enumerate() {
            let text = line.text();
            let text_lower = text.to_lowercase();

            let mut start = 0;
            while let Some(pos) = text_lower[start..].find(&pattern_lower) {
                let match_start = start + pos;
                let match_end = match_start + pattern.len();

                matches.push(SearchMatch {
                    line: line_idx,
                    cols: (match_start, match_end),
                    text: text[match_start..match_end].to_string(),
                });

                start = match_end;
            }
        }

        matches
    }

    /// Search with regex pattern
    fn search_regex(&self, pattern: &str) -> Vec<SearchMatch> {
        let mut matches = Vec::new();

        let regex = regex::Regex::new(pattern);
        if regex.is_err() {
            return matches;
        }
        let regex = regex.unwrap();

        for (line_idx, line) in self.lines.iter().enumerate() {
            let text = line.text();

            for mat in regex.find_iter(&text) {
                matches.push(SearchMatch {
                    line: line_idx,
                    cols: (mat.start(), mat.end()),
                    text: mat.as_str().to_string(),
                });
            }
        }

        matches
    }

    /// Put a character at current cursor position
    fn put_char(&mut self, ch: char, style: CellStyle) {
        // Handle newline
        if ch == '\n' {
            self.commit_line();
            self.cursor.0 = 0;
            self.cursor.1 += 1;
            return;
        }

        // Handle carriage return
        if ch == '\r' {
            self.cursor.0 = 0;
            return;
        }

        // Handle tab
        if ch == '\t' {
            let next_tab = (self.cursor.0 + 8) & !7;
            self.cursor.0 = next_tab.min(self.dimensions.0 - 1);
            return;
        }

        // Handle backspace
        if ch == '\x08' {
            if self.cursor.0 > 0 {
                self.cursor.0 -= 1;
            }
            return;
        }

        // Put character in current line
        self.current_line.put_char(self.cursor.0, ch, style);
        self.cursor.0 += 1;

        // Handle line wrap
        if self.cursor.0 >= self.dimensions.0 {
            self.current_line.set_wrapped(true);
            self.commit_line();
            self.cursor.0 = 0;
            self.cursor.1 += 1;
        }
    }

    /// Commit current line to scrollback
    fn commit_line(&mut self) {
        // Add current line to buffer
        self.lines.push_back(self.current_line.clone());

        // FIFO eviction when exceeding max
        if self.lines.len() > self.max_lines {
            self.lines.pop_front();
            // Adjust cursor row if we removed lines before cursor
            if self.cursor.1 > 0 {
                self.cursor.1 -= 1;
            }
        }

        // Start new line
        self.current_line = TermLine::new();
    }

    /// Move cursor to position
    pub fn move_cursor(&mut self, col: usize, row: usize) {
        self.cursor.0 = col.min(self.dimensions.0 - 1);
        self.cursor.1 = row.min(self.dimensions.1 - 1);
    }
}

impl Default for TerminalBuffer {
    fn default() -> Self {
        Self::default_capacity()
    }
}

/// A single line in the terminal buffer
#[derive(Debug, Clone)]
pub struct TermLine {
    /// Cells in this line
    cells: Vec<Cell>,
    /// Whether this line is wrapped from previous line
    is_wrapped: bool,
}

impl TermLine {
    /// Create new empty line
    pub fn new() -> Self {
        Self {
            cells: Vec::with_capacity(80),
            is_wrapped: false,
        }
    }

    /// Create line with initial capacity
    pub fn with_capacity(cols: usize) -> Self {
        Self {
            cells: Vec::with_capacity(cols),
            is_wrapped: false,
        }
    }

    /// Put character at column position
    pub fn put_char(&mut self, col: usize, ch: char, style: CellStyle) {
        // Expand cells if needed
        while self.cells.len() <= col {
            self.cells.push(Cell::empty());
        }

        self.cells[col] = Cell::new(ch, style);
    }

    /// Get cell at position
    pub fn get_cell(&self, col: usize) -> Option<&Cell> {
        self.cells.get(col)
    }

    /// Get all cells
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Get text content of line
    pub fn text(&self) -> String {
        self.cells.iter().map(|c| c.char).collect()
    }

    /// Clear line
    pub fn clear(&mut self) {
        self.cells.clear();
        self.is_wrapped = false;
    }

    /// Set wrapped flag
    pub fn set_wrapped(&mut self, wrapped: bool) {
        self.is_wrapped = wrapped;
    }

    /// Check if line is wrapped
    pub fn is_wrapped(&self) -> bool {
        self.is_wrapped
    }

    /// Get line length (number of non-empty cells)
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Check if line is empty
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

impl Default for TermLine {
    fn default() -> Self {
        Self::new()
    }
}

/// A single cell in the terminal
#[derive(Debug, Clone, Copy)]
pub struct Cell {
    /// Character in this cell
    pub char: char,
    /// Cell style (colors, attributes)
    pub style: CellStyle,
}

impl Cell {
    /// Create new cell with character and style
    pub fn new(ch: char, style: CellStyle) -> Self {
        Self { char: ch, style }
    }

    /// Create empty cell (space)
    pub fn empty() -> Self {
        Self {
            char: ' ',
            style: CellStyle::default(),
        }
    }
}

/// Style attributes for a cell
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellStyle {
    /// Foreground color (text)
    pub fg: Color32,
    /// Background color
    pub bg: Color32,
    /// Bold attribute
    pub bold: bool,
    /// Underline attribute
    pub underline: bool,
    /// Italic attribute
    pub italic: bool,
    /// Reverse video (swap fg/bg)
    pub reverse: bool,
}

impl CellStyle {
    /// Create with specific colors
    pub fn new(fg: Color32, bg: Color32) -> Self {
        Self {
            fg,
            bg,
            bold: false,
            underline: false,
            italic: false,
            reverse: false,
        }
    }

    /// Create bold variant
    pub fn bold(self) -> Self {
        Self { bold: true, ..self }
    }

    /// Create underline variant
    pub fn underline(self) -> Self {
        Self { underline: true, ..self }
    }

    /// Create italic variant
    pub fn italic(self) -> Self {
        Self { italic: true, ..self }
    }

    /// Create reverse video variant
    pub fn reverse(self) -> Self {
        Self { reverse: true, ..self }
    }

    /// Get effective colors (accounting for reverse)
    pub fn effective_colors(&self) -> (Color32, Color32) {
        if self.reverse {
            (self.bg, self.fg)
        } else {
            (self.fg, self.bg)
        }
    }
}

impl Default for CellStyle {
    fn default() -> Self {
        Self::new(
            Color32::from_rgb(200, 200, 200), // Light gray text
            Color32::from_rgb(22, 25, 30),    // Dark background
        )
    }
}

/// Terminal style state (ANSI colors)
#[derive(Debug, Clone)]
pub struct TerminalStyle {
    /// Color scheme
    colors: ColorScheme,
    /// Current foreground color
    current_fg: Color32,
    /// Current background color
    current_bg: Color32,
    /// Bold flag
    bold: bool,
    /// Underline flag
    underline: bool,
}

impl TerminalStyle {
    /// Get default cell style from current state
    pub fn default_cell_style(&self) -> CellStyle {
        CellStyle::new(self.current_fg, self.current_bg)
            .bold_if(self.bold)
            .underline_if(self.underline)
    }

    /// Set foreground color from ANSI index
    pub fn set_fg(&mut self, index: u8) {
        self.current_fg = self.colors.ansi_color(index);
    }

    /// Set background color from ANSI index
    pub fn set_bg(&mut self, index: u8) {
        self.current_bg = self.colors.ansi_color(index);
    }

    /// Set bold attribute
    pub fn set_bold(&mut self, bold: bool) {
        self.bold = bold;
    }

    /// Set underline attribute
    pub fn set_underline(&mut self, underline: bool) {
        self.underline = underline;
    }

    /// Reset to default style
    pub fn reset(&mut self) {
        self.current_fg = self.colors.default_fg;
        self.current_bg = self.colors.default_bg;
        self.bold = false;
        self.underline = false;
    }
}

impl Default for TerminalStyle {
    fn default() -> Self {
        let colors = ColorScheme::dark();
        let default_fg = colors.default_fg;
        let default_bg = colors.default_bg;
        Self {
            colors,
            current_fg: default_fg,
            current_bg: default_bg,
            bold: false,
            underline: false,
        }
    }
}

/// ANSI color scheme
#[derive(Debug, Clone)]
pub struct ColorScheme {
    /// Default foreground color
    pub default_fg: Color32,
    /// Default background color
    pub default_bg: Color32,
    /// ANSI 16 colors (0-15)
    ansi16: [Color32; 16],
    /// ANSI 256 color palette (optional)
    ansi256: Option<Vec<Color32>>,
}

impl ColorScheme {
    /// Create dark color scheme
    pub fn dark() -> Self {
        Self {
            default_fg: Color32::from_rgb(200, 200, 200),
            default_bg: Color32::from_rgb(22, 25, 30),
            ansi16: [
                // Standard colors (0-7)
                Color32::from_rgb(0, 0, 0),       // Black
                Color32::from_rgb(205, 0, 0),     // Red
                Color32::from_rgb(0, 205, 0),     // Green
                Color32::from_rgb(205, 205, 0),   // Yellow
                Color32::from_rgb(0, 0, 205),     // Blue
                Color32::from_rgb(205, 0, 205),   // Magenta
                Color32::from_rgb(0, 205, 205),   // Cyan
                Color32::from_rgb(205, 205, 205), // White
                // Bright colors (8-15)
                Color32::from_rgb(128, 128, 128), // Bright Black (Gray)
                Color32::from_rgb(255, 85, 85),   // Bright Red
                Color32::from_rgb(85, 255, 85),   // Bright Green
                Color32::from_rgb(255, 255, 85),  // Bright Yellow
                Color32::from_rgb(85, 85, 255),   // Bright Blue
                Color32::from_rgb(255, 85, 255),  // Bright Magenta
                Color32::from_rgb(85, 255, 255),  // Bright Cyan
                Color32::from_rgb(255, 255, 255), // Bright White
            ],
            ansi256: None,
        }
    }

    /// Create light color scheme
    pub fn light() -> Self {
        Self {
            default_fg: Color32::from_rgb(30, 30, 30),
            default_bg: Color32::from_rgb(255, 255, 255),
            ansi16: [
                // Standard colors (0-7)
                Color32::from_rgb(0, 0, 0),       // Black
                Color32::from_rgb(205, 0, 0),     // Red
                Color32::from_rgb(0, 205, 0),     // Green
                Color32::from_rgb(205, 205, 0),   // Yellow
                Color32::from_rgb(0, 0, 205),     // Blue
                Color32::from_rgb(205, 0, 205),   // Magenta
                Color32::from_rgb(0, 205, 205),   // Cyan
                Color32::from_rgb(205, 205, 205), // White
                // Bright colors (8-15)
                Color32::from_rgb(128, 128, 128), // Bright Black
                Color32::from_rgb(255, 85, 85),   // Bright Red
                Color32::from_rgb(85, 255, 85),   // Bright Green
                Color32::from_rgb(255, 255, 85),  // Bright Yellow
                Color32::from_rgb(85, 85, 255),   // Bright Blue
                Color32::from_rgb(255, 85, 255),  // Bright Magenta
                Color32::from_rgb(85, 255, 255),  // Bright Cyan
                Color32::from_rgb(255, 255, 255), // Bright White
            ],
            ansi256: None,
        }
    }

    /// Get ANSI color by index (0-255)
    pub fn ansi_color(&self, index: u8) -> Color32 {
        if index < 16 {
            self.ansi16[index as usize]
        } else if let Some(palette) = &self.ansi256 {
            palette.get(index as usize).copied().unwrap_or(self.default_fg)
        } else {
            // Fallback: generate color for 16-255 range
            self.generate_ansi_color(index)
        }
    }

    /// Generate color for ANSI index 16-255
    fn generate_ansi_color(&self, index: u8) -> Color32 {
        if index < 16 {
            return self.ansi16[index as usize];
        }

        // 16-231: 6x6x6 color cube
        if index < 232 {
            let i = index - 16;
            let r = (i / 36) * 51;
            let g = ((i % 36) / 6) * 51;
            let b = (i % 6) * 51;
            return Color32::from_rgb(r, g, b);
        }

        // 232-255: grayscale
        let gray = (index - 232) * 10 + 8;
        Color32::from_rgb(gray, gray, gray)
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::dark()
    }
}

/// ANSI state machine for processing escape sequences
#[derive(Debug, Clone, Default)]
struct AnsiState {
    /// Current style
    style: TerminalStyle,
    /// Pending output
    pending: Vec<char>,
}

impl AnsiState {
    fn new() -> Self {
        Self::default()
    }
}

impl Perform for AnsiState {
    fn print(&mut self, c: char) {
        // Queue character for buffer processing
        self.pending.push(c);
    }

    fn execute(&mut self, byte: u8) {
        // Handle control characters (already handled in buffer)
        match byte {
            0x0A => self.pending.push('\n'), // LF
            0x0D => self.pending.push('\r'), // CR
            0x08 => self.pending.push('\x08'), // BS
            0x09 => self.pending.push('\t'), // TAB
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {
        // OSC/DCS hooks - not implemented for basic terminal
    }

    fn put(&mut self, _byte: u8) {
        // OSC/DCS data - not implemented
    }

    fn unhook(&mut self) {
        // OSC/DCS end - not implemented
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        // OSC sequences - not implemented for basic terminal
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        // CSI sequences (SGR, cursor movement, etc.)
        match action {
            'm' => self.handle_sgr_params(params), // SGR - Select Graphic Rendition
            'A' | 'B' | 'C' | 'D' | 'H' | 'J' | 'K' => {} // Cursor/erase - handled externally
            _ => {}
        }
    }
}

impl AnsiState {
    /// Handle SGR (Select Graphic Rendition) parameters from vte Params
    fn handle_sgr_params(&mut self, params: &Params) {
        for param_list in params.iter() {
            for param in param_list {
                match param {
                    0 => self.style.reset(),
                    1 => self.style.set_bold(true),
                    4 => self.style.set_underline(true),
                    22 => self.style.set_bold(false),
                    24 => self.style.set_underline(false),
                    30..=37 => self.style.set_fg((param - 30) as u8),   // Standard fg
                    38 => {} // Extended fg (256 color)
                    39 => self.style.set_fg(7), // Default fg
                    40..=47 => self.style.set_bg((param - 40) as u8),   // Standard bg
                    48 => {} // Extended bg (256 color)
                    49 => self.style.set_bg(0), // Default bg
                    90..=97 => self.style.set_fg((param - 90 + 8) as u8), // Bright fg
                    100..=107 => self.style.set_bg((param - 100 + 8) as u8), // Bright bg
                    _ => {}
                }
            }
        }
    }
}

// Helper trait for conditional style
trait ConditionalStyle {
    fn bold_if(self, cond: bool) -> Self;
    fn underline_if(self, cond: bool) -> Self;
}

impl ConditionalStyle for CellStyle {
    fn bold_if(self, cond: bool) -> Self {
        if cond { Self { bold: true, ..self } } else { self }
    }

    fn underline_if(self, cond: bool) -> Self {
        if cond { Self { underline: true, ..self } } else { self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_creation() {
        let buffer = TerminalBuffer::new(1000);
        assert_eq!(buffer.line_count(), 0);
        assert_eq!(buffer.dimensions(), (80, 24));
    }

    #[test]
    fn test_write_text() {
        let mut buffer = TerminalBuffer::new(100);
        buffer.write_raw("Hello, World!\n");

        assert_eq!(buffer.line_count(), 1);
        let line = buffer.get_line(0).expect("Line should exist");
        assert_eq!(line.text(), "Hello, World!");
    }

    #[test]
    fn test_fifo_eviction() {
        let mut buffer = TerminalBuffer::new(5);

        for i in 0..10 {
            buffer.write_raw(&format!("Line {}\n", i));
        }

        // Should only keep last 5 lines
        assert_eq!(buffer.line_count(), 5);

        let first = buffer.get_line(0).expect("First line");
        assert_eq!(first.text(), "Line 5");
    }

    #[test]
    fn test_search_literal() {
        let mut buffer = TerminalBuffer::new(100);
        buffer.write_raw("foo bar baz\n");
        buffer.write_raw("another foo here\n");
        buffer.write_raw("no match here\n");

        let matches = buffer.search("foo", false);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line, 0);
        assert_eq!(matches[1].line, 1);
    }

    #[test]
    fn test_cell_style() {
        let style = CellStyle::default().bold().underline();
        assert!(style.bold);
        assert!(style.underline);

        let (fg, bg) = style.effective_colors();
        assert_eq!(fg, Color32::from_rgb(200, 200, 200));
        assert_eq!(bg, Color32::from_rgb(22, 25, 30));
    }

    #[test]
    fn test_reverse_video() {
        let style = CellStyle::default().reverse();
        let (fg, bg) = style.effective_colors();
        assert_eq!(fg, Color32::from_rgb(22, 25, 30)); // Background color
        assert_eq!(bg, Color32::from_rgb(200, 200, 200)); // Foreground color
    }

    #[test]
    fn test_color_scheme() {
        let scheme = ColorScheme::dark();
        assert_eq!(scheme.ansi_color(0), Color32::from_rgb(0, 0, 0)); // Black
        assert_eq!(scheme.ansi_color(1), Color32::from_rgb(205, 0, 0)); // Red
        assert_eq!(scheme.ansi_color(9), Color32::from_rgb(255, 85, 85)); // Bright Red
    }
}