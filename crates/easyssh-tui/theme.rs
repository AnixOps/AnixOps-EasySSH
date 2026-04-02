//! Theme System
//!
//! Provides comprehensive color theme support:
//! - 256 color support for modern terminals
//! - True Color (24-bit RGB) for supported terminals
//! - Multiple built-in themes (Dark, Light, Solarized, Monokai)
//! - Automatic terminal capability detection
//! - Inspired by ranger and htop color schemes

use ratatui::style::Color;

/// Terminal color capability
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorCapability {
    /// Basic 16 colors
    Basic,
    /// 256 color palette
    Extended,
    /// True Color (24-bit RGB)
    TrueColor,
}

/// Color palette for UI elements
#[derive(Debug, Clone)]
pub struct ColorPalette {
    // Background colors
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_selected: Color,
    pub bg_highlight: Color,
    pub bg_status_bar: Color,
    pub bg_dialog: Color,

    // Foreground colors
    pub fg_primary: Color,
    pub fg_secondary: Color,
    pub fg_muted: Color,
    pub fg_selected: Color,
    pub fg_highlight: Color,

    // Accent colors
    pub accent_primary: Color,
    pub accent_secondary: Color,
    pub accent_success: Color,
    pub accent_warning: Color,
    pub accent_error: Color,
    pub accent_info: Color,

    // Semantic colors
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub server_online: Color,
    pub server_offline: Color,
    pub server_connecting: Color,
    pub server_error: Color,
    pub server_unknown: Color,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::dark_theme()
    }
}

impl ColorPalette {
    /// Dark theme (default) - inspired by htop and ranger
    pub fn dark_theme() -> Self {
        Self {
            // Backgrounds
            bg_primary: Color::Black,
            bg_secondary: Color::Rgb(30, 30, 30),
            bg_selected: Color::Rgb(45, 45, 55),
            bg_highlight: Color::Rgb(60, 60, 70),
            bg_status_bar: Color::Rgb(30, 120, 180),
            bg_dialog: Color::Rgb(35, 35, 40),

            // Foregrounds
            fg_primary: Color::Rgb(248, 248, 242),
            fg_secondary: Color::Rgb(180, 180, 180),
            fg_muted: Color::Rgb(120, 120, 120),
            fg_selected: Color::Rgb(255, 255, 255),
            fg_highlight: Color::Rgb(100, 200, 255),

            // Accents - using bright, vibrant colors like htop
            accent_primary: Color::Rgb(100, 200, 255), // Bright blue
            accent_secondary: Color::Rgb(180, 120, 255), // Purple
            accent_success: Color::Rgb(80, 250, 123),  // Bright green
            accent_warning: Color::Rgb(255, 184, 108), // Orange
            accent_error: Color::Rgb(255, 85, 85),     // Red
            accent_info: Color::Rgb(139, 233, 253),    // Cyan

            // Semantic colors
            border_focused: Color::Rgb(100, 200, 255),
            border_unfocused: Color::Rgb(80, 80, 80),
            server_online: Color::Rgb(80, 250, 123),
            server_offline: Color::Rgb(120, 120, 120),
            server_connecting: Color::Rgb(255, 184, 108),
            server_error: Color::Rgb(255, 85, 85),
            server_unknown: Color::Rgb(180, 180, 180),
        }
    }

    /// Light theme
    pub fn light_theme() -> Self {
        Self {
            // Backgrounds
            bg_primary: Color::Rgb(250, 250, 250),
            bg_secondary: Color::Rgb(240, 240, 240),
            bg_selected: Color::Rgb(220, 230, 250),
            bg_highlight: Color::Rgb(200, 220, 240),
            bg_status_bar: Color::Rgb(30, 100, 160),
            bg_dialog: Color::Rgb(245, 245, 245),

            // Foregrounds
            fg_primary: Color::Rgb(40, 40, 40),
            fg_secondary: Color::Rgb(80, 80, 80),
            fg_muted: Color::Rgb(140, 140, 140),
            fg_selected: Color::Rgb(0, 0, 0),
            fg_highlight: Color::Rgb(20, 100, 180),

            // Accents
            accent_primary: Color::Rgb(30, 120, 200),
            accent_secondary: Color::Rgb(120, 80, 200),
            accent_success: Color::Rgb(40, 160, 80),
            accent_warning: Color::Rgb(220, 140, 40),
            accent_error: Color::Rgb(200, 60, 60),
            accent_info: Color::Rgb(40, 140, 180),

            // Semantic colors
            border_focused: Color::Rgb(30, 120, 200),
            border_unfocused: Color::Rgb(180, 180, 180),
            server_online: Color::Rgb(40, 160, 80),
            server_offline: Color::Rgb(140, 140, 140),
            server_connecting: Color::Rgb(220, 140, 40),
            server_error: Color::Rgb(200, 60, 60),
            server_unknown: Color::Rgb(120, 120, 120),
        }
    }

    /// Solarized Dark theme
    pub fn solarized_dark() -> Self {
        // Solarized palette
        let base03 = Color::Rgb(0, 43, 54);
        let base02 = Color::Rgb(7, 54, 66);
        let base01 = Color::Rgb(88, 110, 117);
        let base00 = Color::Rgb(131, 148, 150);
        let base0 = Color::Rgb(131, 148, 150);
        let base1 = Color::Rgb(147, 161, 161);
        let base2 = Color::Rgb(238, 232, 213);
        let base3 = Color::Rgb(253, 246, 227);
        let yellow = Color::Rgb(181, 137, 0);
        let orange = Color::Rgb(203, 75, 22);
        let red = Color::Rgb(220, 50, 47);
        let magenta = Color::Rgb(211, 54, 130);
        let violet = Color::Rgb(108, 113, 196);
        let blue = Color::Rgb(38, 139, 210);
        let cyan = Color::Rgb(42, 161, 152);
        let green = Color::Rgb(133, 153, 0);

        Self {
            bg_primary: base03,
            bg_secondary: base02,
            bg_selected: Color::Rgb(15, 70, 85),
            bg_highlight: Color::Rgb(25, 85, 100),
            bg_status_bar: blue,
            bg_dialog: base02,

            fg_primary: base0,
            fg_secondary: base00,
            fg_muted: base01,
            fg_selected: base3,
            fg_highlight: cyan,

            accent_primary: blue,
            accent_secondary: violet,
            accent_success: green,
            accent_warning: yellow,
            accent_error: red,
            accent_info: cyan,

            border_focused: blue,
            border_unfocused: base01,
            server_online: green,
            server_offline: base01,
            server_connecting: yellow,
            server_error: red,
            server_unknown: base0,
        }
    }

    /// Monokai theme - inspired by the popular code editor theme
    pub fn monokai() -> Self {
        Self {
            bg_primary: Color::Rgb(39, 40, 34),
            bg_secondary: Color::Rgb(48, 49, 42),
            bg_selected: Color::Rgb(60, 62, 55),
            bg_highlight: Color::Rgb(73, 76, 62),
            bg_status_bar: Color::Rgb(102, 217, 239),
            bg_dialog: Color::Rgb(45, 46, 40),

            fg_primary: Color::Rgb(248, 248, 242),
            fg_secondary: Color::Rgb(174, 175, 168),
            fg_muted: Color::Rgb(117, 113, 94),
            fg_selected: Color::Rgb(255, 255, 255),
            fg_highlight: Color::Rgb(102, 217, 239),

            accent_primary: Color::Rgb(102, 217, 239), // Cyan
            accent_secondary: Color::Rgb(174, 129, 255), // Purple
            accent_success: Color::Rgb(166, 226, 46),  // Green
            accent_warning: Color::Rgb(253, 151, 31),  // Orange
            accent_error: Color::Rgb(249, 38, 114),    // Pink/Red
            accent_info: Color::Rgb(102, 217, 239),    // Cyan

            border_focused: Color::Rgb(102, 217, 239),
            border_unfocused: Color::Rgb(117, 113, 94),
            server_online: Color::Rgb(166, 226, 46),
            server_offline: Color::Rgb(117, 113, 94),
            server_connecting: Color::Rgb(253, 151, 31),
            server_error: Color::Rgb(249, 38, 114),
            server_unknown: Color::Rgb(174, 175, 168),
        }
    }

    /// Convert to 256-color approximations if needed
    pub fn to_256_colors(&self) -> Self {
        Self {
            bg_primary: Self::rgb_to_256(self.bg_primary),
            bg_secondary: Self::rgb_to_256(self.bg_secondary),
            bg_selected: Self::rgb_to_256(self.bg_selected),
            bg_highlight: Self::rgb_to_256(self.bg_highlight),
            bg_status_bar: Self::rgb_to_256(self.bg_status_bar),
            bg_dialog: Self::rgb_to_256(self.bg_dialog),

            fg_primary: Self::rgb_to_256(self.fg_primary),
            fg_secondary: Self::rgb_to_256(self.fg_secondary),
            fg_muted: Self::rgb_to_256(self.fg_muted),
            fg_selected: Self::rgb_to_256(self.fg_selected),
            fg_highlight: Self::rgb_to_256(self.fg_highlight),

            accent_primary: Self::rgb_to_256(self.accent_primary),
            accent_secondary: Self::rgb_to_256(self.accent_secondary),
            accent_success: Self::rgb_to_256(self.accent_success),
            accent_warning: Self::rgb_to_256(self.accent_warning),
            accent_error: Self::rgb_to_256(self.accent_error),
            accent_info: Self::rgb_to_256(self.accent_info),

            border_focused: Self::rgb_to_256(self.border_focused),
            border_unfocused: Self::rgb_to_256(self.border_unfocused),
            server_online: Self::rgb_to_256(self.server_online),
            server_offline: Self::rgb_to_256(self.server_offline),
            server_connecting: Self::rgb_to_256(self.server_connecting),
            server_error: Self::rgb_to_256(self.server_error),
            server_unknown: Self::rgb_to_256(self.server_unknown),
        }
    }

    /// Convert RGB color to nearest 256-color
    fn rgb_to_256(color: Color) -> Color {
        match color {
            Color::Rgb(r, g, b) => {
                // Convert to 6x6x6 color cube (16-231)
                let r_index = (r as u16 * 5 / 255) as u8;
                let g_index = (g as u16 * 5 / 255) as u8;
                let b_index = (b as u16 * 5 / 255) as u8;
                let color_index = 16 + 36 * r_index + 6 * g_index + b_index;
                Color::Indexed(color_index)
            }
            Color::Rgb(r, g, b) if r == g && g == b => {
                // Grayscale (232-255)
                let gray_index = ((r as u16 * 23) / 255).min(23) as u8;
                Color::Indexed(232 + gray_index)
            }
            _ => color,
        }
    }

    /// Get server status color
    pub fn server_status_color(&self, status: &str) -> Color {
        match status {
            "online" => self.server_online,
            "offline" => self.server_offline,
            "connecting" => self.server_connecting,
            "error" => self.server_error,
            _ => self.server_unknown,
        }
    }
}

/// Theme manager
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub palette: ColorPalette,
    pub capability: ColorCapability,
}

impl Theme {
    pub fn new(name: &str, capability: ColorCapability) -> Self {
        let palette = match name {
            "light" => ColorPalette::light_theme(),
            "solarized" => ColorPalette::solarized_dark(),
            "monokai" => ColorPalette::monokai(),
            _ => ColorPalette::dark_theme(),
        };

        let palette = if capability != ColorCapability::TrueColor {
            palette.to_256_colors()
        } else {
            palette
        };

        Self {
            name: name.to_string(),
            palette,
            capability,
        }
    }

    /// Detect terminal color capability
    pub fn detect_capability() -> ColorCapability {
        // Check environment variables
        if let Ok(term) = std::env::var("TERM") {
            if term.contains("256color") || term.contains("256") {
                return ColorCapability::Extended;
            }
            if term.contains("truecolor") || term.contains("24bit") {
                return ColorCapability::TrueColor;
            }
        }

        if let Ok(colorterm) = std::env::var("COLORTERM") {
            if colorterm.contains("truecolor") || colorterm.contains("24bit") {
                return ColorCapability::TrueColor;
            }
        }

        // Default to extended (256) for modern terminals
        ColorCapability::Extended
    }

    /// Get available theme names
    pub fn available_themes() -> Vec<&'static str> {
        vec!["dark", "light", "solarized", "monokai"]
    }
}

impl Default for Theme {
    fn default() -> Self {
        let capability = Self::detect_capability();
        Self::new("dark", capability)
    }
}
