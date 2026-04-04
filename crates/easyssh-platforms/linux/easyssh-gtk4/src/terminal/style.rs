//! Terminal Style System for GTK4
//!
//! Provides styling configuration for the terminal view including:
//! - ANSI color palette (16 base colors)
//! - Cursor styles
//! - Font configuration
//! - Theme presets
//!
//! # Constraints (SYSTEM_INVARIANTS.md)
//!
//! - Themes must be compatible with core::terminal::theme module
//! - Colors must be convertible to gdk::RGBA

use gtk4::gdk;

/// Cursor style options for terminal display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    /// Block cursor (filled rectangle)
    Block,
    /// Underline cursor (horizontal line at bottom)
    Underline,
    /// Bar cursor (vertical line at left)
    Bar,
}

impl CursorStyle {
    /// Convert to string for CSS/config.
    pub fn as_str(&self) -> &'static str {
        match self {
            CursorStyle::Block => "block",
            CursorStyle::Underline => "underline",
            CursorStyle::Bar => "bar",
        }
    }

    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "block" => Some(CursorStyle::Block),
            "underline" => Some(CursorStyle::Underline),
            "bar" => Some(CursorStyle::Bar),
            _ => None,
        }
    }
}

impl Default for CursorStyle {
    fn default() -> Self {
        CursorStyle::Block
    }
}

/// Terminal style configuration.
///
/// Contains all visual settings for the terminal widget.
#[derive(Debug, Clone)]
pub struct TerminalStyle {
    /// Font family name
    pub font_family: String,
    /// Font size in points
    pub font_size: u16,
    /// Foreground (text) color
    pub foreground: gdk::RGBA,
    /// Background color
    pub background: gdk::RGBA,
    /// Cursor style
    pub cursor_style: CursorStyle,
    /// Cursor blink enabled
    pub cursor_blink: bool,
    /// ANSI 16-color palette
    pub colors: [gdk::RGBA; 16],
    /// Selection background color
    pub selection_bg: gdk::RGBA,
    /// Selection foreground color
    pub selection_fg: gdk::RGBA,
    /// Background opacity (0.0 - 1.0)
    pub background_opacity: f32,
}

impl Default for TerminalStyle {
    fn default() -> Self {
        Self::default_dark()
    }
}

impl TerminalStyle {
    /// Create a default dark theme (Dracula-inspired).
    pub fn default_dark() -> Self {
        Self {
            font_family: "JetBrains Mono".to_string(),
            font_size: 14,
            foreground: rgba_from_hex(0xF8F8F2),
            background: rgba_from_hex(0x282A36),
            cursor_style: CursorStyle::Block,
            cursor_blink: true,
            colors: [
                // Normal colors (0-7)
                rgba_from_hex(0x21222C), // Black
                rgba_from_hex(0xFF5555), // Red
                rgba_from_hex(0x50FA7B), // Green
                rgba_from_hex(0xF1FA8C), // Yellow
                rgba_from_hex(0xBD93F9), // Blue
                rgba_from_hex(0xFF79C6), // Magenta
                rgba_from_hex(0x8BE9FD), // Cyan
                rgba_from_hex(0xF8F8F2), // White
                // Bright colors (8-15)
                rgba_from_hex(0x6272A4), // Bright Black
                rgba_from_hex(0xFF6E6E), // Bright Red
                rgba_from_hex(0x69FF94), // Bright Green
                rgba_from_hex(0xFFFFA5), // Bright Yellow
                rgba_from_hex(0xD6ACFF), // Bright Blue
                rgba_from_hex(0xFF92DF), // Bright Magenta
                rgba_from_hex(0xA4FFFF), // Bright Cyan
                rgba_from_hex(0xFFFFFF), // Bright White
            ],
            selection_bg: rgba_from_hex(0x44475A),
            selection_fg: rgba_from_hex(0xF8F8F2),
            background_opacity: 1.0,
        }
    }

    /// Create a default light theme.
    pub fn default_light() -> Self {
        Self {
            font_family: "JetBrains Mono".to_string(),
            font_size: 14,
            foreground: rgba_from_hex(0x24292F),
            background: rgba_from_hex(0xFFFFFF),
            cursor_style: CursorStyle::Bar,
            cursor_blink: true,
            colors: [
                // Normal colors (0-7)
                rgba_from_hex(0x24292E), // Black
                rgba_from_hex(0xCF222E), // Red
                rgba_from_hex(0x116329), // Green
                rgba_from_hex(0x4D2D00), // Yellow
                rgba_from_hex(0x0969DA), // Blue
                rgba_from_hex(0x8250DF), // Magenta
                rgba_from_hex(0x1B7C83), // Cyan
                rgba_from_hex(0x6E7781), // White
                // Bright colors (8-15)
                rgba_from_hex(0x57606A), // Bright Black
                rgba_from_hex(0xA40E26), // Bright Red
                rgba_from_hex(0x1A7F37), // Bright Green
                rgba_from_hex(0x633C01), // Bright Yellow
                rgba_from_hex(0x218BFF), // Bright Blue
                rgba_from_hex(0xA475F9), // Bright Magenta
                rgba_from_hex(0x3192AA), // Bright Cyan
                rgba_from_hex(0x8C959F), // Bright White
            ],
            selection_bg: rgba_from_hex(0xDDF4FF),
            selection_fg: rgba_from_hex(0x24292F),
            background_opacity: 1.0,
        }
    }

    /// One Dark theme (Atom-inspired).
    pub fn one_dark() -> Self {
        Self {
            font_family: "JetBrains Mono".to_string(),
            font_size: 14,
            foreground: rgba_from_hex(0xABB2BF),
            background: rgba_from_hex(0x282C34),
            cursor_style: CursorStyle::Block,
            cursor_blink: true,
            colors: [
                rgba_from_hex(0x282C34), // Black
                rgba_from_hex(0xE06C75), // Red
                rgba_from_hex(0x98C379), // Green
                rgba_from_hex(0xE5C07B), // Yellow
                rgba_from_hex(0x61AFEF), // Blue
                rgba_from_hex(0xC678DD), // Magenta
                rgba_from_hex(0x56B6C2), // Cyan
                rgba_from_hex(0xABB2BF), // White
                rgba_from_hex(0x5C6370), // Bright Black
                rgba_from_hex(0xE06C75), // Bright Red
                rgba_from_hex(0x98C379), // Bright Green
                rgba_from_hex(0xE5C07B), // Bright Yellow
                rgba_from_hex(0x61AFEF), // Bright Blue
                rgba_from_hex(0xC678DD), // Bright Magenta
                rgba_from_hex(0x56B6C2), // Bright Cyan
                rgba_from_hex(0xFFFFFF), // Bright White
            ],
            selection_bg: rgba_from_hex(0x3E4451),
            selection_fg: rgba_from_hex(0xABB2BF),
            background_opacity: 1.0,
        }
    }

    /// Solarized Dark theme.
    pub fn solarized_dark() -> Self {
        Self {
            font_family: "JetBrains Mono".to_string(),
            font_size: 14,
            foreground: rgba_from_hex(0x839496),
            background: rgba_from_hex(0x002B36),
            cursor_style: CursorStyle::Block,
            cursor_blink: true,
            colors: [
                rgba_from_hex(0x002B36), // Black
                rgba_from_hex(0xDC322F), // Red
                rgba_from_hex(0x859900), // Green
                rgba_from_hex(0xB58900), // Yellow
                rgba_from_hex(0x268BD2), // Blue
                rgba_from_hex(0xD33682), // Magenta
                rgba_from_hex(0x2AA198), // Cyan
                rgba_from_hex(0xEEE8D5), // White
                rgba_from_hex(0x073642), // Bright Black
                rgba_from_hex(0xCB4B16), // Bright Red
                rgba_from_hex(0x586E75), // Bright Green
                rgba_from_hex(0x657B83), // Bright Yellow
                rgba_from_hex(0x839496), // Bright Blue
                rgba_from_hex(0x6C71C4), // Bright Magenta
                rgba_from_hex(0x93A1A1), // Bright Cyan
                rgba_from_hex(0xFDF6E3), // Bright White
            ],
            selection_bg: rgba_from_hex(0x073642),
            selection_fg: rgba_from_hex(0x93A1A1),
            background_opacity: 1.0,
        }
    }

    /// Monokai theme.
    pub fn monokai() -> Self {
        Self {
            font_family: "JetBrains Mono".to_string(),
            font_size: 14,
            foreground: rgba_from_hex(0xF8F8F2),
            background: rgba_from_hex(0x272822),
            cursor_style: CursorStyle::Block,
            cursor_blink: false,
            colors: [
                rgba_from_hex(0x272822), // Black
                rgba_from_hex(0xF92672), // Red
                rgba_from_hex(0xA6E22E), // Green
                rgba_from_hex(0xF4BF75), // Yellow
                rgba_from_hex(0x66D9EF), // Blue
                rgba_from_hex(0xAE81FF), // Magenta
                rgba_from_hex(0xA1EFE4), // Cyan
                rgba_from_hex(0xF8F8F2), // White
                rgba_from_hex(0x75715E), // Bright Black
                rgba_from_hex(0xF92672), // Bright Red
                rgba_from_hex(0xA6E22E), // Bright Green
                rgba_from_hex(0xF4BF75), // Bright Yellow
                rgba_from_hex(0x66D9EF), // Bright Blue
                rgba_from_hex(0xAE81FF), // Bright Magenta
                rgba_from_hex(0xA1EFE4), // Bright Cyan
                rgba_from_hex(0xF9F8F5), // Bright White
            ],
            selection_bg: rgba_from_hex(0x49483E),
            selection_fg: rgba_from_hex(0xF8F8F2),
            background_opacity: 1.0,
        }
    }

    /// Load theme from name.
    ///
    /// # Arguments
    ///
    /// * `name` - Theme name ("dracula", "one-dark", "solarized-dark", "monokai", "light")
    ///
    /// # Returns
    ///
    /// The matching theme or None if not found.
    pub fn from_theme(name: &str) -> Option<Self> {
        match name.to_lowercase().replace('-', "").replace(' ', "") {
            "dracula" | "default" | "dark" => Some(Self::default_dark()),
            "onedark" | "onedark" => Some(Self::one_dark()),
            "solarizeddark" | "solarized" => Some(Self::solarized_dark()),
            "monokai" => Some(Self::monokai()),
            "light" | "githublight" => Some(Self::default_light()),
            _ => None,
        }
    }

    /// Get ANSI color by index (0-15).
    ///
    /// # Arguments
    ///
    /// * `index` - ANSI color index (0-15 for base colors)
    ///
    /// # Returns
    ///
    /// The color or default foreground if index is invalid.
    pub fn get_ansi_color(&self, index: u8) -> gdk::RGBA {
        if index < 16 {
            self.colors[index as usize]
        } else {
            self.foreground
        }
    }

    /// Get 256-color by index.
    ///
    /// For indices 16-231, computes the 6x6x6 color cube.
    /// For indices 232-255, computes the grayscale ramp.
    pub fn get_256_color(&self, index: u8) -> gdk::RGBA {
        if index < 16 {
            self.get_ansi_color(index)
        } else if index < 232 {
            // 216-color cube (6x6x6)
            let idx = index - 16;
            let r = (idx / 36) as u8;
            let g = ((idx % 36) / 6) as u8;
            let b = (idx % 6) as u8;

            let r_val = if r == 0 { 0 } else { r * 40 + 55 };
            let g_val = if g == 0 { 0 } else { g * 40 + 55 };
            let b_val = if b == 0 { 0 } else { b * 40 + 55 };

            gdk::RGBA::new(
                r_val as f32 / 255.0,
                g_val as f32 / 255.0,
                b_val as f32 / 255.0,
                1.0,
            )
        } else {
            // 24-level grayscale
            let gray = 8 + (index - 232) * 10;
            let val = gray as f32 / 255.0;
            gdk::RGBA::new(val, val, val, 1.0)
        }
    }

    /// Set font family.
    pub fn with_font_family(mut self, family: &str) -> Self {
        self.font_family = family.to_string();
        self
    }

    /// Set font size.
    pub fn with_font_size(mut self, size: u16) -> Self {
        self.font_size = size;
        self
    }

    /// Set cursor style.
    pub fn with_cursor_style(mut self, style: CursorStyle) -> Self {
        self.cursor_style = style;
        self
    }

    /// Set background opacity.
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.background_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Generate CSS for this style.
    ///
    /// Creates GTK4 CSS rules for terminal styling.
    pub fn to_css(&self) -> String {
        let bg = rgba_to_css(&self.background, self.background_opacity);
        let fg = rgba_to_css(&self.foreground, 1.0);

        let selection_bg = rgba_to_css(&self.selection_bg, 1.0);
        let selection_fg = rgba_to_css(&self.selection_fg, 1.0);

        format!(
            ".terminal-view {{\n\
             background-color: {};\n\
             color: {};\n\
             font-family: '{}', monospace;\n\
             font-size: {}pt;\n\
             }}\n\
             .terminal-output {{\n\
             background-color: {};\n\
             color: {};\n\
             font-family: '{}', monospace;\n\
             font-size: {}pt;\n\
             }}\n\
             .terminal-output selection {{\n\
             background-color: {};\n\
             color: {};\n\
             }}\n\
             .terminal-input {{\n\
             background-color: {};\n\
             color: {};\n\
             font-family: '{}', monospace;\n\
             font-size: {}pt;\n\
             }}\n\
             ",
            bg, fg, self.font_family, self.font_size,
            bg, fg, self.font_family, self.font_size,
            selection_bg, selection_fg,
            bg, fg, self.font_family, self.font_size
        )
    }
}

/// Convert hex color to gdk::RGBA.
///
/// # Arguments
///
/// * `hex` - 24-bit RGB color value (e.g., 0xFF5555)
///
/// # Returns
///
/// gdk::RGBA with full opacity.
fn rgba_from_hex(hex: u32) -> gdk::RGBA {
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    gdk::RGBA::new(r, g, b, 1.0)
}

/// Convert gdk::RGBA to CSS color string.
///
/// # Arguments
///
/// * `rgba` - The color to convert
/// * `alpha` - Override alpha value (0.0 - 1.0)
///
/// # Returns
///
/// CSS color string (rgba format).
fn rgba_to_css(rgba: &gdk::RGBA, alpha: f32) -> String {
    let r = (rgba.red() * 255.0) as u8;
    let g = (rgba.green() * 255.0) as u8;
    let b = (rgba.blue() * 255.0) as u8;
    format!("rgba({}, {}, {}, {:.2})", r, g, b, alpha)
}

// ============ Unit Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_style_conversion() {
        assert_eq!(CursorStyle::Block.as_str(), "block");
        assert_eq!(CursorStyle::from_str("underline"), Some(CursorStyle::Underline));
        assert_eq!(CursorStyle::from_str("invalid"), None);
    }

    #[test]
    fn test_default_dark_theme() {
        let style = TerminalStyle::default_dark();
        assert_eq!(style.font_family, "JetBrains Mono");
        assert_eq!(style.font_size, 14);
        assert!(style.cursor_blink);
    }

    #[test]
    fn test_default_light_theme() {
        let style = TerminalStyle::default_light();
        assert!(!style.background_opacity.is_nan());
        assert_eq!(style.cursor_style, CursorStyle::Bar);
    }

    #[test]
    fn test_theme_from_name() {
        assert!(TerminalStyle::from_theme("dracula").is_some());
        assert!(TerminalStyle::from_theme("one-dark").is_some());
        assert!(TerminalStyle::from_theme("invalid").is_none());
    }

    #[test]
    fn test_ansi_colors() {
        let style = TerminalStyle::default_dark();

        // Test base colors
        let red = style.get_ansi_color(1);
        assert!(red.red() > 0.9); // Dracula red is bright

        let green = style.get_ansi_color(2);
        assert!(green.green() > 0.4);
    }

    #[test]
    fn test_256_colors() {
        let style = TerminalStyle::default_dark();

        // Test color cube (index 16-231)
        let cube_color = style.get_256_color(16); // First cube color
        assert!(cube_color.alpha() > 0.0);

        // Test grayscale (index 232-255)
        let gray = style.get_256_color(240);
        assert!(gray.red() == gray.green() && gray.green() == gray.blue());
    }

    #[test]
    fn test_rgba_from_hex() {
        let color = rgba_from_hex(0xFF5555);
        assert_eq!(color.red(), 1.0);
        assert!(color.green() > 0.3 && color.green() < 0.4);
        assert!(color.blue() > 0.3 && color.blue() < 0.4);
        assert_eq!(color.alpha(), 1.0);
    }

    #[test]
    fn test_rgba_to_css() {
        let color = rgba_from_hex(0xFF5555);
        let css = rgba_to_css(&color, 1.0);
        assert!(css.contains("rgba"));
        assert!(css.contains("255"));
    }

    #[test]
    fn test_style_modifiers() {
        let style = TerminalStyle::default_dark()
            .with_font_family("Fira Code")
            .with_font_size(16)
            .with_cursor_style(CursorStyle::Underline)
            .with_opacity(0.9);

        assert_eq!(style.font_family, "Fira Code");
        assert_eq!(style.font_size, 16);
        assert_eq!(style.cursor_style, CursorStyle::Underline);
        assert!((style.background_opacity - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_css_generation() {
        let style = TerminalStyle::default_dark();
        let css = style.to_css();

        assert!(css.contains(".terminal-view"));
        assert!(css.contains(".terminal-output"));
        assert!(css.contains("background-color"));
        assert!(css.contains("font-family"));
    }

    #[test]
    fn test_one_dark_theme() {
        let style = TerminalStyle::one_dark();
        // One Dark uses #282C34 for background
        let bg = style.background;
        assert!(bg.red() < 0.2);
    }

    #[test]
    fn test_monokai_theme() {
        let style = TerminalStyle::monokai();
        // Monokai has non-blinking cursor
        assert!(!style.cursor_blink);
    }
}