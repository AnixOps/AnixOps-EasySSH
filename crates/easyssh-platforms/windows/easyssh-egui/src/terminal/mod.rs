//! Terminal Module
//!
//! Provides embedded terminal rendering for egui with:
//! - Scrollback buffer with FIFO line management
//! - Cell-based styling (ANSI colors, bold, underline)
//! - Search functionality (literal and regex)
//! - Selection and clipboard support
//! - 60fps rendering with efficient painting

pub mod buffer;
pub mod view;
pub mod renderer;

pub use buffer::{TerminalBuffer, TermLine, Cell, CellStyle, ColorScheme};
pub use view::{TerminalView, TerminalConfig};
pub use renderer::TerminalRenderer;

/// Default terminal color scheme (dark theme)
pub fn default_color_scheme() -> ColorScheme {
    ColorScheme::dark()
}

/// Terminal font configuration
#[derive(Debug, Clone)]
pub struct TerminalFontConfig {
    /// Font family name
    pub family: String,
    /// Font size in points
    pub size: f32,
    /// Line height multiplier (typically 1.2)
    pub line_height: f32,
}

impl Default for TerminalFontConfig {
    fn default() -> Self {
        Self {
            family: "JetBrains Mono".to_string(),
            size: 14.0,
            line_height: 1.2,
        }
    }
}

/// Cursor style configuration
#[derive(Debug, Clone, Copy)]
pub struct CursorConfig {
    /// Cursor shape
    pub shape: CursorShape,
    /// Whether cursor blinks
    pub blink: bool,
    /// Cursor color
    pub color: egui::Color32,
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            shape: CursorShape::Block,
            blink: true,
            color: egui::Color32::from_rgb(200, 200, 200),
        }
    }
}

/// Cursor shape options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    /// Block cursor (fills entire cell)
    Block,
    /// Underline cursor (line at bottom)
    Underline,
    /// Bar cursor (vertical line on left)
    Bar,
}

/// Search match result
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// Line number in buffer
    pub line: usize,
    /// Column range (start, end)
    pub cols: (usize, usize),
    /// Matched text
    pub text: String,
}