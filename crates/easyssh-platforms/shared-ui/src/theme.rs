//! Theme System
//!
//! Comprehensive theming with support for:
//! - Light, Dark, and Auto (system) modes
//! - High contrast accessibility mode
//! - Custom accent colors
//! - Terminal color schemes
//! - Semantic color tokens

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Color scheme variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ColorScheme {
    /// Light mode
    Light,
    /// Dark mode
    Dark,
    /// Follow system preference (default)
    #[default]
    System,
    /// High contrast mode (accessibility)
    HighContrast,
}

impl ColorScheme {
    /// Check if this scheme is dark
    pub fn is_dark(&self) -> bool {
        matches!(self, Self::Dark)
    }

    /// Check if this scheme is high contrast
    pub fn is_high_contrast(&self) -> bool {
        matches!(self, Self::HighContrast)
    }

    /// Get all available schemes
    pub fn all() -> Vec<Self> {
        vec![
            ColorScheme::Light,
            ColorScheme::Dark,
            ColorScheme::System,
            ColorScheme::HighContrast,
        ]
    }

    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            ColorScheme::Light => "Light",
            ColorScheme::Dark => "Dark",
            ColorScheme::System => "Auto",
            ColorScheme::HighContrast => "High Contrast",
        }
    }
}

/// Complete theme definition with all color tokens
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Theme {
    /// Theme identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Base color scheme
    pub scheme: ColorScheme,
    /// All semantic colors
    pub colors: SemanticColors,
    /// Terminal color palette
    pub terminal: TerminalColors,
    /// Typography settings
    pub typography: TypographySettings,
    /// Spacing scale
    pub spacing: SpacingScale,
    /// Border radius scale
    pub border_radius: BorderRadiusScale,
    /// Shadow definitions
    pub shadows: ShadowDefinitions,
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

impl Theme {
    /// Create the default light theme
    pub fn light() -> Self {
        Self {
            id: "light".to_string(),
            name: "Light".to_string(),
            scheme: ColorScheme::Light,
            colors: SemanticColors::light(),
            terminal: TerminalColors::default(),
            typography: TypographySettings::default(),
            spacing: SpacingScale::default(),
            border_radius: BorderRadiusScale::default(),
            shadows: ShadowDefinitions::default(),
        }
    }

    /// Create the default dark theme
    pub fn dark() -> Self {
        Self {
            id: "dark".to_string(),
            name: "Dark".to_string(),
            scheme: ColorScheme::Dark,
            colors: SemanticColors::dark(),
            terminal: TerminalColors::default(),
            typography: TypographySettings::default(),
            spacing: SpacingScale::default(),
            border_radius: BorderRadiusScale::default(),
            shadows: ShadowDefinitions::dark(),
        }
    }

    /// Create high contrast theme
    pub fn high_contrast() -> Self {
        Self {
            id: "high_contrast".to_string(),
            name: "High Contrast".to_string(),
            scheme: ColorScheme::HighContrast,
            colors: SemanticColors::high_contrast(),
            terminal: TerminalColors::high_contrast(),
            typography: TypographySettings::high_contrast(),
            spacing: SpacingScale::default(),
            border_radius: BorderRadiusScale::sharp(),
            shadows: ShadowDefinitions::none(),
        }
    }

    /// Get theme based on color scheme preference
    pub fn from_scheme(scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Light => Self::light(),
            ColorScheme::Dark => Self::dark(),
            ColorScheme::HighContrast => Self::high_contrast(),
            ColorScheme::System => {
                // Check system preference
                if is_system_dark_mode() {
                    Self::dark()
                } else {
                    Self::light()
                }
            }
        }
    }
}

/// Semantic color tokens for UI elements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticColors {
    // Background colors
    pub background_primary: Color,
    pub background_secondary: Color,
    pub background_tertiary: Color,
    pub background_elevated: Color,
    pub background_overlay: Color,
    pub background_terminal: Color,

    // Text colors
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_tertiary: Color,
    pub text_inverted: Color,
    pub text_terminal: Color,

    // Interactive colors
    pub interactive_primary: Color,
    pub interactive_primary_hover: Color,
    pub interactive_primary_active: Color,
    pub interactive_secondary: Color,
    pub interactive_secondary_hover: Color,
    pub interactive_ghost_hover: Color,

    // Border colors
    pub border_subtle: Color,
    pub border_default: Color,
    pub border_strong: Color,

    // Status colors
    pub status_online: Color,
    pub status_offline: Color,
    pub status_connecting: Color,
    pub status_warning: Color,
    pub status_error: Color,
    pub status_info: Color,

    // Accent colors
    pub accent: Color,
    pub accent_light: Color,
    pub accent_dark: Color,

    // Focus ring
    pub focus_ring: Color,
}

/// Color representation (hex or rgba)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Color {
    /// Hex color (#RRGGBB or #RRGGBBAA)
    Hex(String),
    /// RGBA color (0-255)
    Rgba { r: u8, g: u8, b: u8, a: u8 },
    /// HSLA color
    Hsla { h: f32, s: f32, l: f32, a: f32 },
}

impl Color {
    /// Create a hex color
    pub fn hex(hex: &str) -> Self {
        Color::Hex(hex.to_string())
    }

    /// Create an RGBA color
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color::Rgba { r, g, b, a }
    }

    /// Create an RGB color (fully opaque)
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color::Rgba { r, g, b, a: 255 }
    }

    /// Convert to CSS string
    pub fn to_css(&self) -> String {
        match self {
            Color::Hex(h) => h.clone(),
            Color::Rgba { r, g, b, a } => format!("rgba({}, {}, {}, {})", r, g, b, a),
            Color::Hsla { h, s, l, a } => format!("hsla({}, {}%, {}%, {})", h, s * 100.0, l * 100.0, a),
        }
    }
}

impl SemanticColors {
    /// Light theme colors
    pub fn light() -> Self {
        Self {
            background_primary: Color::hex("#FFFFFF"),
            background_secondary: Color::hex("#F5F5F5"),
            background_tertiary: Color::hex("#EBEBEB"),
            background_elevated: Color::hex("#FFFFFF"),
            background_overlay: Color::rgba(0, 0, 0, 102), // 40% opacity
            background_terminal: Color::hex("#1E1E1E"),

            text_primary: Color::hex("#1A1A1A"),
            text_secondary: Color::hex("#525252"),
            text_tertiary: Color::hex("#757575"),
            text_inverted: Color::hex("#FFFFFF"),
            text_terminal: Color::hex("#DCDCDC"),

            interactive_primary: Color::hex("#2563EB"),
            interactive_primary_hover: Color::hex("#1D4ED8"),
            interactive_primary_active: Color::hex("#1E40AF"),
            interactive_secondary: Color::hex("#F5F5F5"),
            interactive_secondary_hover: Color::hex("#EBEBEB"),
            interactive_ghost_hover: Color::hex("#F5F5F5"),

            border_subtle: Color::hex("#EBEBEB"),
            border_default: Color::hex("#E0E0E0"),
            border_strong: Color::hex("#C4C4C4"),

            status_online: Color::hex("#22C55E"),
            status_offline: Color::hex("#EF4444"),
            status_connecting: Color::hex("#F59E0B"),
            status_warning: Color::hex("#F59E0B"),
            status_error: Color::hex("#EF4444"),
            status_info: Color::hex("#3B82F6"),

            accent: Color::hex("#2563EB"),
            accent_light: Color::hex("#93C5FD"),
            accent_dark: Color::hex("#1E40AF"),

            focus_ring: Color::hex("#3B82F6"),
        }
    }

    /// Dark theme colors
    pub fn dark() -> Self {
        Self {
            background_primary: Color::hex("#0D0D0D"),
            background_secondary: Color::hex("#1A1A1A"),
            background_tertiary: Color::hex("#363636"),
            background_elevated: Color::hex("#1A1A1A"),
            background_overlay: Color::rgba(0, 0, 0, 178), // 70% opacity
            background_terminal: Color::hex("#1E1E1E"),

            text_primary: Color::hex("#FAFAFA"),
            text_secondary: Color::hex("#C4C4C4"),
            text_tertiary: Color::hex("#757575"),
            text_inverted: Color::hex("#1A1A1A"),
            text_terminal: Color::hex("#DCDCDC"),

            interactive_primary: Color::hex("#3B82F6"),
            interactive_primary_hover: Color::hex("#60A5FA"),
            interactive_primary_active: Color::hex("#2563EB"),
            interactive_secondary: Color::hex("#363636"),
            interactive_secondary_hover: Color::hex("#525252"),
            interactive_ghost_hover: Color::hex("#363636"),

            border_subtle: Color::hex("#363636"),
            border_default: Color::hex("#525252"),
            border_strong: Color::hex("#757575"),

            status_online: Color::hex("#4ADE80"),
            status_offline: Color::hex("#F87171"),
            status_connecting: Color::hex("#FBBF24"),
            status_warning: Color::hex("#FBBF24"),
            status_error: Color::hex("#F87171"),
            status_info: Color::hex("#60A5FA"),

            accent: Color::hex("#3B82F6"),
            accent_light: Color::hex("#93C5FD"),
            accent_dark: Color::hex("#1E40AF"),

            focus_ring: Color::hex("#60A5FA"),
        }
    }

    /// High contrast theme colors
    pub fn high_contrast() -> Self {
        let mut colors = Self::light();
        // Override with high contrast values
        colors.text_primary = Color::hex("#000000");
        colors.text_secondary = Color::hex("#000000");
        colors.border_default = Color::hex("#000000");
        colors.border_strong = Color::hex("#000000");
        colors.interactive_primary = Color::hex("#0000FF");
        colors.focus_ring = Color::hex("#0000FF");
        colors
    }
}

/// Terminal color palette (ANSI colors)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TerminalColors {
    pub black: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub magenta: Color,
    pub cyan: Color,
    pub white: Color,
    pub bright_black: Color,
    pub bright_red: Color,
    pub bright_green: Color,
    pub bright_yellow: Color,
    pub bright_blue: Color,
    pub bright_magenta: Color,
    pub bright_cyan: Color,
    pub bright_white: Color,
    pub cursor: Color,
    pub selection: Color,
}

impl Default for TerminalColors {
    fn default() -> Self {
        Self {
            black: Color::hex("#1E1E1E"),
            red: Color::hex("#E06C75"),
            green: Color::hex("#98C379"),
            yellow: Color::hex("#E5C07B"),
            blue: Color::hex("#61AFEF"),
            magenta: Color::hex("#C678DD"),
            cyan: Color::hex("#56B6C2"),
            white: Color::hex("#DCDCDC"),
            bright_black: Color::hex("#5C6370"),
            bright_red: Color::hex("#FF6B7A"),
            bright_green: Color::hex("#B5E08D"),
            bright_yellow: Color::hex("#F0D58A"),
            bright_blue: Color::hex("#7BC3FF"),
            bright_magenta: Color::hex("#D78FE6"),
            bright_cyan: Color::hex("#6ED4E0"),
            bright_white: Color::hex("#FFFFFF"),
            cursor: Color::hex("#528BFF"),
            selection: Color::hex("#264F78"),
        }
    }
}

impl TerminalColors {
    /// High contrast terminal colors
    pub fn high_contrast() -> Self {
        Self {
            black: Color::hex("#000000"),
            red: Color::hex("#FF0000"),
            green: Color::hex("#00FF00"),
            yellow: Color::hex("#FFFF00"),
            blue: Color::hex("#0000FF"),
            magenta: Color::hex("#FF00FF"),
            cyan: Color::hex("#00FFFF"),
            white: Color::hex("#FFFFFF"),
            bright_black: Color::hex("#808080"),
            bright_red: Color::hex("#FF6B6B"),
            bright_green: Color::hex("#6BFF6B"),
            bright_yellow: Color::hex("#FFFF6B"),
            bright_blue: Color::hex("#6B6BFF"),
            bright_magenta: Color::hex("#FF6BFF"),
            bright_cyan: Color::hex("#6BFFFF"),
            bright_white: Color::hex("#FFFFFF"),
            cursor: Color::hex("#0000FF"),
            selection: Color::hex("#0000FF"),
        }
    }
}

/// Typography settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypographySettings {
    pub font_family_ui: String,
    pub font_family_mono: String,
    pub font_size_base: f32,
    pub line_height: f32,
    pub font_weight_normal: u16,
    pub font_weight_medium: u16,
    pub font_weight_bold: u16,
}

impl Default for TypographySettings {
    fn default() -> Self {
        Self {
            font_family_ui: "Inter, system-ui, -apple-system, sans-serif".to_string(),
            font_family_mono: "JetBrains Mono, SF Mono, Menlo, monospace".to_string(),
            font_size_base: 14.0,
            line_height: 1.5,
            font_weight_normal: 400,
            font_weight_medium: 500,
            font_weight_bold: 600,
        }
    }
}

impl TypographySettings {
    /// High contrast typography (larger base size)
    pub fn high_contrast() -> Self {
        Self {
            font_family_ui: "system-ui, -apple-system, sans-serif".to_string(),
            font_family_mono: "monospace".to_string(),
            font_size_base: 16.0,
            line_height: 1.6,
            font_weight_normal: 400,
            font_weight_medium: 600,
            font_weight_bold: 700,
        }
    }
}

/// Spacing scale (4px base grid)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpacingScale {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}

impl Default for SpacingScale {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 16.0,
            lg: 24.0,
            xl: 32.0,
            xxl: 48.0,
        }
    }
}

/// Border radius scale
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BorderRadiusScale {
    pub none: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub full: f32,
}

impl Default for BorderRadiusScale {
    fn default() -> Self {
        Self {
            none: 0.0,
            sm: 4.0,
            md: 6.0,
            lg: 8.0,
            xl: 12.0,
            full: 9999.0,
        }
    }
}

impl BorderRadiusScale {
    /// Sharp corners (for high contrast)
    pub fn sharp() -> Self {
        Self {
            none: 0.0,
            sm: 0.0,
            md: 0.0,
            lg: 0.0,
            xl: 0.0,
            full: 0.0,
        }
    }
}

/// Shadow definitions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShadowDefinitions {
    pub none: String,
    pub xs: String,
    pub sm: String,
    pub md: String,
    pub lg: String,
    pub xl: String,
}

impl Default for ShadowDefinitions {
    fn default() -> Self {
        Self {
            none: "none".to_string(),
            xs: "0 1px 2px 0 rgba(0, 0, 0, 0.03)".to_string(),
            sm: "0 1px 3px 0 rgba(0, 0, 0, 0.06), 0 1px 2px -1px rgba(0, 0, 0, 0.06)".to_string(),
            md: "0 4px 6px -1px rgba(0, 0, 0, 0.06), 0 2px 4px -2px rgba(0, 0, 0, 0.06)".to_string(),
            lg: "0 10px 15px -3px rgba(0, 0, 0, 0.06), 0 4px 6px -4px rgba(0, 0, 0, 0.06)".to_string(),
            xl: "0 20px 25px -5px rgba(0, 0, 0, 0.06), 0 8px 10px -6px rgba(0, 0, 0, 0.06)".to_string(),
        }
    }
}

impl ShadowDefinitions {
    /// Dark theme shadows (stronger)
    pub fn dark() -> Self {
        Self {
            none: "none".to_string(),
            xs: "0 1px 2px 0 rgba(0, 0, 0, 0.2)".to_string(),
            sm: "0 1px 3px 0 rgba(0, 0, 0, 0.3), 0 1px 2px -1px rgba(0, 0, 0, 0.3)".to_string(),
            md: "0 4px 6px -1px rgba(0, 0, 0, 0.3), 0 2px 4px -2px rgba(0, 0, 0, 0.3)".to_string(),
            lg: "0 10px 15px -3px rgba(0, 0, 0, 0.3), 0 4px 6px -4px rgba(0, 0, 0, 0.3)".to_string(),
            xl: "0 20px 25px -5px rgba(0, 0, 0, 0.3), 0 8px 10px -6px rgba(0, 0, 0, 0.3)".to_string(),
        }
    }

    /// No shadows (for high contrast)
    pub fn none() -> Self {
        Self {
            none: "none".to_string(),
            xs: "none".to_string(),
            sm: "none".to_string(),
            md: "none".to_string(),
            lg: "none".to_string(),
            xl: "none".to_string(),
        }
    }
}

/// Theme configuration options
#[derive(Debug, Clone)]
pub struct ThemeConfig {
    /// Preferred color scheme
    pub color_scheme: ColorScheme,
    /// Custom accent color (optional)
    pub custom_accent: Option<Color>,
    /// Enable system theme detection
    pub detect_system_theme: bool,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::System,
            custom_accent: None,
            detect_system_theme: true,
        }
    }
}

/// Theme manager for runtime theme switching
pub struct ThemeManager {
    current: Theme,
    config: ThemeConfig,
    listeners: Vec<Box<dyn Fn(&Theme)>>,
}

impl ThemeManager {
    /// Create a new theme manager
    pub fn new(config: &ThemeConfig) -> Self {
        let theme = Theme::from_scheme(config.color_scheme);
        Self {
            current: theme,
            config: config.clone(),
            listeners: Vec::new(),
        }
    }

    /// Get the current theme
    pub fn current(&self) -> &Theme {
        &self.current
    }

    /// Set the color scheme
    pub fn set_scheme(&mut self, scheme: ColorScheme) {
        self.config.color_scheme = scheme;
        self.current = Theme::from_scheme(scheme);
        self.notify_listeners();
    }

    /// Register a theme change listener
    pub fn on_change<F>(&mut self, callback: F)
    where
        F: Fn(&Theme) + 'static,
    {
        self.listeners.push(Box::new(callback));
    }

    fn notify_listeners(&self) {
        for listener in &self.listeners {
            listener(&self.current);
        }
    }
}

/// Check if system is in dark mode
fn is_system_dark_mode() -> bool {
    // Platform-specific implementation
    // For now, default to false
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_scheme_variants() {
        assert!(ColorScheme::Dark.is_dark());
        assert!(!ColorScheme::Light.is_dark());
        assert!(ColorScheme::HighContrast.is_high_contrast());
    }

    #[test]
    fn test_theme_creation() {
        let light = Theme::light();
        assert_eq!(light.id, "light");
        assert_eq!(light.scheme, ColorScheme::Light);

        let dark = Theme::dark();
        assert_eq!(dark.id, "dark");
        assert_eq!(dark.scheme, ColorScheme::Dark);
    }

    #[test]
    fn test_color_to_css() {
        let hex = Color::hex("#FF0000");
        assert_eq!(hex.to_css(), "#FF0000");

        let rgba = Color::rgba(255, 0, 0, 128);
        assert_eq!(rgba.to_css(), "rgba(255, 0, 0, 128)");
    }

    #[test]
    fn test_theme_manager() {
        let config = ThemeConfig::default();
        let mut manager = ThemeManager::new(&config);

        assert_eq!(manager.current().scheme, ColorScheme::System);

        manager.set_scheme(ColorScheme::Dark);
        assert_eq!(manager.current().scheme, ColorScheme::Dark);
    }
}
