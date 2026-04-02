#![allow(dead_code)]
#![allow(dead_code)]

//! EasySSH Professional Theme System
//!
//! Comprehensive theme customization with:
//! - 10+ Pre-built themes (One Dark, Dracula, Solarized, Monokai, Nord, etc.)
//! - Visual Theme Editor with real-time preview
//! - Background images with transparency support
//! - Font tuning (weight, line height, letter spacing)
//! - Cursor customization (block, line, underscore, blinking)
//! - Full ANSI color support (16, 256, True Color)
//! - Semantic syntax highlighting for shell commands
//! - Dynamic day/night auto-switching
//! - VS Code theme import/export
//! - Community theme store
//!
//! Reference: Windows Terminal, iTerm2, VS Code

use egui::{Color32, FontFamily, FontId, Rounding};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// SERDE HELPERS FOR EGUI TYPES
// ============================================================================

pub mod serde_color32 {
    use egui::Color32;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(color: &Color32, serializer: S) -> Result<S::Ok, S::Error> {
        let rgba = color.to_array();
        serializer.serialize_u32(u32::from_le_bytes([rgba[0], rgba[1], rgba[2], rgba[3]]))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Color32, D::Error> {
        let value = u32::deserialize(deserializer)?;
        let bytes = value.to_le_bytes();
        Ok(Color32::from_rgba_premultiplied(
            bytes[0], bytes[1], bytes[2], bytes[3],
        ))
    }
}

pub mod serde_vec_color32 {
    use egui::Color32;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(colors: &[Color32], serializer: S) -> Result<S::Ok, S::Error> {
        let values: Vec<u32> = colors
            .iter()
            .map(|c| {
                let rgba = c.to_array();
                u32::from_le_bytes([rgba[0], rgba[1], rgba[2], rgba[3]])
            })
            .collect();
        values.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<Color32>, D::Error> {
        let values: Vec<u32> = Vec::deserialize(deserializer)?;
        Ok(values
            .iter()
            .map(|&v| {
                let bytes = v.to_le_bytes();
                Color32::from_rgba_premultiplied(bytes[0], bytes[1], bytes[2], bytes[3])
            })
            .collect())
    }
}

// ============================================================================
// CURSOR STYLES
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash, Default)]
pub enum CursorStyle {
    #[default]
    Block,
    Line,
    Underscore,
    EmptyBox,
}

impl CursorStyle {
    pub fn all() -> Vec<Self> {
        vec![
            CursorStyle::Block,
            CursorStyle::Line,
            CursorStyle::Underscore,
            CursorStyle::EmptyBox,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            CursorStyle::Block => "Block █",
            CursorStyle::Line => "Line |",
            CursorStyle::Underscore => "Underscore ▁",
            CursorStyle::EmptyBox => "Empty Box ▯",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CursorBlinkMode {
    #[default]
    Blink,
    Solid,
    Smooth, // iTerm2-style smooth blink
}

// ============================================================================
// TERMINAL COLOR PALETTE
// ============================================================================

/// Complete 16-color ANSI palette + extended colors
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TerminalPalette {
    // Standard 16 ANSI colors
    #[serde(with = "serde_color32")]
    pub black: Color32,
    #[serde(with = "serde_color32")]
    pub red: Color32,
    #[serde(with = "serde_color32")]
    pub green: Color32,
    #[serde(with = "serde_color32")]
    pub yellow: Color32,
    #[serde(with = "serde_color32")]
    pub blue: Color32,
    #[serde(with = "serde_color32")]
    pub magenta: Color32,
    #[serde(with = "serde_color32")]
    pub cyan: Color32,
    #[serde(with = "serde_color32")]
    pub white: Color32,
    #[serde(with = "serde_color32")]
    pub bright_black: Color32,
    #[serde(with = "serde_color32")]
    pub bright_red: Color32,
    #[serde(with = "serde_color32")]
    pub bright_green: Color32,
    #[serde(with = "serde_color32")]
    pub bright_yellow: Color32,
    #[serde(with = "serde_color32")]
    pub bright_blue: Color32,
    #[serde(with = "serde_color32")]
    pub bright_magenta: Color32,
    #[serde(with = "serde_color32")]
    pub bright_cyan: Color32,
    #[serde(with = "serde_color32")]
    pub bright_white: Color32,

    // Extended 216 colors (6x6x6 RGB cube) - stored as 256-color lookup
    #[serde(with = "serde_vec_color32")]
    pub color_256: Vec<Color32>,

    // Theme-specific colors
    #[serde(with = "serde_color32")]
    pub background: Color32,
    #[serde(with = "serde_color32")]
    pub foreground: Color32,
    #[serde(with = "serde_color32")]
    pub cursor: Color32,
    #[serde(with = "serde_color32")]
    pub cursor_text: Color32,
    #[serde(with = "serde_color32")]
    pub selection: Color32,
    #[serde(with = "serde_color32")]
    pub selection_text: Color32,
}

impl Default for TerminalPalette {
    fn default() -> Self {
        Self::one_dark()
    }
}

impl TerminalPalette {
    /// One Dark Pro palette (Atom/VS Code default)
    pub fn one_dark() -> Self {
        let mut palette = Self {
            black: Color32::from_rgb(0x1e, 0x1e, 0x1e),
            red: Color32::from_rgb(0xe0, 0x6c, 0x75),
            green: Color32::from_rgb(0x98, 0xc3, 0x79),
            yellow: Color32::from_rgb(0xe5, 0xc0, 0x7b),
            blue: Color32::from_rgb(0x61, 0xaf, 0xef),
            magenta: Color32::from_rgb(0xc6, 0x78, 0xdd),
            cyan: Color32::from_rgb(0x56, 0xb6, 0xc2),
            white: Color32::from_rgb(0xab, 0xb2, 0xbf),
            bright_black: Color32::from_rgb(0x5c, 0x63, 0x70),
            bright_red: Color32::from_rgb(0xff, 0x6b, 0x7a),
            bright_green: Color32::from_rgb(0xb5, 0xe0, 0x8d),
            bright_yellow: Color32::from_rgb(0xf0, 0xd5, 0x8a),
            bright_blue: Color32::from_rgb(0x7b, 0xc3, 0xff),
            bright_magenta: Color32::from_rgb(0xd7, 0x8f, 0xe6),
            bright_cyan: Color32::from_rgb(0x6e, 0xd4, 0xe0),
            bright_white: Color32::from_rgb(0xff, 0xff, 0xff),
            color_256: Vec::with_capacity(256),
            background: Color32::from_rgb(0x28, 0x2c, 0x34),
            foreground: Color32::from_rgb(0xab, 0xb2, 0xbf),
            cursor: Color32::from_rgb(0x52, 0x8b, 0xff),
            cursor_text: Color32::from_rgb(0x28, 0x2c, 0x34),
            selection: Color32::from_rgb(0x3e, 0x44, 0x51),
            selection_text: Color32::from_rgb(0xff, 0xff, 0xff),
        };
        palette.generate_256_colors();
        palette
    }

    /// Dracula theme palette
    pub fn dracula() -> Self {
        let mut palette = Self {
            black: Color32::from_rgb(0x00, 0x00, 0x00),
            red: Color32::from_rgb(0xff, 0x55, 0x55),
            green: Color32::from_rgb(0x50, 0xfa, 0x7b),
            yellow: Color32::from_rgb(0xf1, 0xfa, 0x8c),
            blue: Color32::from_rgb(0xbd, 0x93, 0xf9),
            magenta: Color32::from_rgb(0xff, 0x79, 0xc6),
            cyan: Color32::from_rgb(0x8b, 0xe9, 0xfd),
            white: Color32::from_rgb(0xbf, 0xbf, 0xbf),
            bright_black: Color32::from_rgb(0x62, 0x72, 0xa4),
            bright_red: Color32::from_rgb(0xff, 0x6e, 0x6e),
            bright_green: Color32::from_rgb(0x69, 0xff, 0x94),
            bright_yellow: Color32::from_rgb(0xff, 0xff, 0xa5),
            bright_blue: Color32::from_rgb(0xd6, 0xac, 0xff),
            bright_magenta: Color32::from_rgb(0xff, 0x92, 0xdf),
            bright_cyan: Color32::from_rgb(0xa4, 0xff, 0xff),
            bright_white: Color32::from_rgb(0xff, 0xff, 0xff),
            color_256: Vec::with_capacity(256),
            background: Color32::from_rgb(0x28, 0x2a, 0x36),
            foreground: Color32::from_rgb(0xf8, 0xf8, 0xf2),
            cursor: Color32::from_rgb(0xff, 0x79, 0xc6),
            cursor_text: Color32::from_rgb(0x28, 0x2a, 0x36),
            selection: Color32::from_rgb(0x44, 0x47, 0x5a),
            selection_text: Color32::from_rgb(0xf8, 0xf8, 0xf2),
        };
        palette.generate_256_colors();
        palette
    }

    /// Solarized Dark palette
    pub fn solarized_dark() -> Self {
        let mut palette = Self {
            black: Color32::from_rgb(0x00, 0x2b, 0x36),
            red: Color32::from_rgb(0xdc, 0x32, 0x2f),
            green: Color32::from_rgb(0x85, 0x99, 0x00),
            yellow: Color32::from_rgb(0xb5, 0x89, 0x00),
            blue: Color32::from_rgb(0x26, 0x8b, 0xd2),
            magenta: Color32::from_rgb(0xd3, 0x36, 0x82),
            cyan: Color32::from_rgb(0x2a, 0xa1, 0x98),
            white: Color32::from_rgb(0xee, 0xe8, 0xd5),
            bright_black: Color32::from_rgb(0x07, 0x36, 0x42),
            bright_red: Color32::from_rgb(0xcb, 0x4b, 0x16),
            bright_green: Color32::from_rgb(0x58, 0x6e, 0x75),
            bright_yellow: Color32::from_rgb(0x65, 0x7b, 0x83),
            bright_blue: Color32::from_rgb(0x83, 0x94, 0x96),
            bright_magenta: Color32::from_rgb(0x6c, 0x71, 0xc4),
            bright_cyan: Color32::from_rgb(0x93, 0xa1, 0xa1),
            bright_white: Color32::from_rgb(0xfd, 0xf6, 0xe3),
            color_256: Vec::with_capacity(256),
            background: Color32::from_rgb(0x00, 0x2b, 0x36),
            foreground: Color32::from_rgb(0x83, 0x94, 0x96),
            cursor: Color32::from_rgb(0x93, 0xa1, 0xa1),
            cursor_text: Color32::from_rgb(0x00, 0x2b, 0x36),
            selection: Color32::from_rgb(0x07, 0x36, 0x42),
            selection_text: Color32::from_rgb(0xee, 0xe8, 0xd5),
        };
        palette.generate_256_colors();
        palette
    }

    /// Solarized Light palette
    pub fn solarized_light() -> Self {
        let mut palette = Self {
            black: Color32::from_rgb(0xee, 0xe8, 0xd5),
            red: Color32::from_rgb(0xdc, 0x32, 0x2f),
            green: Color32::from_rgb(0x85, 0x99, 0x00),
            yellow: Color32::from_rgb(0xb5, 0x89, 0x00),
            blue: Color32::from_rgb(0x26, 0x8b, 0xd2),
            magenta: Color32::from_rgb(0xd3, 0x36, 0x82),
            cyan: Color32::from_rgb(0x2a, 0xa1, 0x98),
            white: Color32::from_rgb(0x00, 0x2b, 0x36),
            bright_black: Color32::from_rgb(0x07, 0x36, 0x42),
            bright_red: Color32::from_rgb(0xcb, 0x4b, 0x16),
            bright_green: Color32::from_rgb(0x58, 0x6e, 0x75),
            bright_yellow: Color32::from_rgb(0x65, 0x7b, 0x83),
            bright_blue: Color32::from_rgb(0x83, 0x94, 0x96),
            bright_magenta: Color32::from_rgb(0x6c, 0x71, 0xc4),
            bright_cyan: Color32::from_rgb(0x93, 0xa1, 0xa1),
            bright_white: Color32::from_rgb(0xfd, 0xf6, 0xe3),
            color_256: Vec::with_capacity(256),
            background: Color32::from_rgb(0xfd, 0xf6, 0xe3),
            foreground: Color32::from_rgb(0x65, 0x7b, 0x83),
            cursor: Color32::from_rgb(0x58, 0x6e, 0x75),
            cursor_text: Color32::from_rgb(0xfd, 0xf6, 0xe3),
            selection: Color32::from_rgb(0xee, 0xe8, 0xd5),
            selection_text: Color32::from_rgb(0x07, 0x36, 0x42),
        };
        palette.generate_256_colors();
        palette
    }

    /// Monokai Pro palette
    pub fn monokai() -> Self {
        let mut palette = Self {
            black: Color32::from_rgb(0x2d, 0x2a, 0x2e),
            red: Color32::from_rgb(0xff, 0x61, 0x8b),
            green: Color32::from_rgb(0xa9, 0xdc, 0x76),
            yellow: Color32::from_rgb(0xff, 0xe9, 0xaa),
            blue: Color32::from_rgb(0x78, 0xd9, 0xec),
            magenta: Color32::from_rgb(0xff, 0x86, 0x78),
            cyan: Color32::from_rgb(0xab, 0xf1, 0x5b),
            white: Color32::from_rgb(0xfc, 0xfc, 0xfa),
            bright_black: Color32::from_rgb(0x72, 0x72, 0x72),
            bright_red: Color32::from_rgb(0xff, 0x87, 0x8a),
            bright_green: Color32::from_rgb(0xc3, 0xe8, 0x8d),
            bright_yellow: Color32::from_rgb(0xff, 0xf2, 0xa1),
            bright_blue: Color32::from_rgb(0x8a, 0xe2, 0xff),
            bright_magenta: Color32::from_rgb(0xff, 0x9f, 0x8a),
            bright_cyan: Color32::from_rgb(0xc1, 0xff, 0x7e),
            bright_white: Color32::from_rgb(0xff, 0xff, 0xff),
            color_256: Vec::with_capacity(256),
            background: Color32::from_rgb(0x2d, 0x2a, 0x2e),
            foreground: Color32::from_rgb(0xfc, 0xfc, 0xfa),
            cursor: Color32::from_rgb(0xab, 0xf1, 0x5b),
            cursor_text: Color32::from_rgb(0x2d, 0x2a, 0x2e),
            selection: Color32::from_rgb(0x52, 0x4f, 0x54),
            selection_text: Color32::from_rgb(0xfc, 0xfc, 0xfa),
        };
        palette.generate_256_colors();
        palette
    }

    /// Nord theme palette (Arctic-inspired)
    pub fn nord() -> Self {
        let mut palette = Self {
            black: Color32::from_rgb(0x2e, 0x34, 0x40),
            red: Color32::from_rgb(0xbf, 0x61, 0x6a),
            green: Color32::from_rgb(0xa3, 0xbe, 0x8c),
            yellow: Color32::from_rgb(0xe3, 0xcb, 0x96),
            blue: Color32::from_rgb(0x81, 0xa1, 0xc1),
            magenta: Color32::from_rgb(0xb4, 0x8e, 0xad),
            cyan: Color32::from_rgb(0x88, 0xc0, 0xd0),
            white: Color32::from_rgb(0xe5, 0xe9, 0xf0),
            bright_black: Color32::from_rgb(0x4c, 0x56, 0x6a),
            bright_red: Color32::from_rgb(0xbf, 0x61, 0x6a),
            bright_green: Color32::from_rgb(0xa3, 0xbe, 0x8c),
            bright_yellow: Color32::from_rgb(0xe3, 0xcb, 0x96),
            bright_blue: Color32::from_rgb(0x81, 0xa1, 0xc1),
            bright_magenta: Color32::from_rgb(0xb4, 0x8e, 0xad),
            bright_cyan: Color32::from_rgb(0x8f, 0xbc, 0xbb),
            bright_white: Color32::from_rgb(0xe5, 0xe9, 0xf0),
            color_256: Vec::with_capacity(256),
            background: Color32::from_rgb(0x2e, 0x34, 0x40),
            foreground: Color32::from_rgb(0xd8, 0xde, 0xe9),
            cursor: Color32::from_rgb(0xd8, 0xde, 0xe9),
            cursor_text: Color32::from_rgb(0x2e, 0x34, 0x40),
            selection: Color32::from_rgb(0x43, 0x4c, 0x5e),
            selection_text: Color32::from_rgb(0xd8, 0xde, 0xe9),
        };
        palette.generate_256_colors();
        palette
    }

    /// GitHub Dark palette
    pub fn github_dark() -> Self {
        let mut palette = Self {
            black: Color32::from_rgb(0x01, 0x01, 0x01),
            red: Color32::from_rgb(0xff, 0x7b, 0x72),
            green: Color32::from_rgb(0x3b, 0xd9, 0x5d),
            yellow: Color32::from_rgb(0xd2, 0x99, 0x22),
            blue: Color32::from_rgb(0x79, 0xc0, 0xff),
            magenta: Color32::from_rgb(0xd2, 0xa8, 0xff),
            cyan: Color32::from_rgb(0x56, 0xd4, 0xdd),
            white: Color32::from_rgb(0xc9, 0xd1, 0xd9),
            bright_black: Color32::from_rgb(0x48, 0x51, 0x58),
            bright_red: Color32::from_rgb(0xff, 0x97, 0x8a),
            bright_green: Color32::from_rgb(0x56, 0xd3, 0x64),
            bright_yellow: Color32::from_rgb(0xe3, 0xb3, 0x41),
            bright_blue: Color32::from_rgb(0x79, 0xc0, 0xff),
            bright_magenta: Color32::from_rgb(0xbc, 0x8c, 0xff),
            bright_cyan: Color32::from_rgb(0xb3, 0xf0, 0xff),
            bright_white: Color32::from_rgb(0xff, 0xff, 0xff),
            color_256: Vec::with_capacity(256),
            background: Color32::from_rgb(0x0d, 0x11, 0x17),
            foreground: Color32::from_rgb(0xc9, 0xd1, 0xd9),
            cursor: Color32::from_rgb(0x58, 0xa6, 0xff),
            cursor_text: Color32::from_rgb(0x0d, 0x11, 0x17),
            selection: Color32::from_rgb(0x27, 0x3c, 0x5c),
            selection_text: Color32::from_rgb(0xc9, 0xd1, 0xd9),
        };
        palette.generate_256_colors();
        palette
    }

    /// GitHub Light palette
    pub fn github_light() -> Self {
        let mut palette = Self {
            black: Color32::from_rgb(0x24, 0x29, 0x2f),
            red: Color32::from_rgb(0xcf, 0x22, 0x2e),
            green: Color32::from_rgb(0x11, 0x60, 0x32),
            yellow: Color32::from_rgb(0x4d, 0x2d, 0x00),
            blue: Color32::from_rgb(0x09, 0x60, 0x9e),
            magenta: Color32::from_rgb(0x82, 0x5d, 0xd1),
            cyan: Color32::from_rgb(0x1b, 0x7c, 0x83),
            white: Color32::from_rgb(0x6e, 0x77, 0x81),
            bright_black: Color32::from_rgb(0x57, 0x61, 0x6d),
            bright_red: Color32::from_rgb(0xa4, 0x0e, 0x22),
            bright_green: Color32::from_rgb(0x11, 0x60, 0x32),
            bright_yellow: Color32::from_rgb(0x66, 0x38, 0x00),
            bright_blue: Color32::from_rgb(0x09, 0x60, 0x9e),
            bright_magenta: Color32::from_rgb(0x82, 0x5d, 0xd1),
            bright_cyan: Color32::from_rgb(0x1b, 0x7c, 0x83),
            bright_white: Color32::from_rgb(0x24, 0x29, 0x2f),
            color_256: Vec::with_capacity(256),
            background: Color32::from_rgb(0xff, 0xff, 0xff),
            foreground: Color32::from_rgb(0x1f, 0x23, 0x28),
            cursor: Color32::from_rgb(0x09, 0x60, 0x9e),
            cursor_text: Color32::from_rgb(0xff, 0xff, 0xff),
            selection: Color32::from_rgb(0xbb, 0xd4, 0xff),
            selection_text: Color32::from_rgb(0x1f, 0x23, 0x28),
        };
        palette.generate_256_colors();
        palette
    }

    /// Tokyo Night palette
    pub fn tokyo_night() -> Self {
        let mut palette = Self {
            black: Color32::from_rgb(0x1a, 0x1b, 0x26),
            red: Color32::from_rgb(0xf7, 0x76, 0x8e),
            green: Color32::from_rgb(0x9e, 0xce, 0x6a),
            yellow: Color32::from_rgb(0xe0, 0xaf, 0x68),
            blue: Color32::from_rgb(0x7a, 0xa2, 0xf7),
            magenta: Color32::from_rgb(0xbb, 0x9a, 0xf7),
            cyan: Color32::from_rgb(0x73, 0xd0, 0xed),
            white: Color32::from_rgb(0x78, 0x79, 0x96),
            bright_black: Color32::from_rgb(0x41, 0x41, 0x53),
            bright_red: Color32::from_rgb(0xff, 0x7a, 0x93),
            bright_green: Color32::from_rgb(0xb9, 0xf0, 0x8c),
            bright_yellow: Color32::from_rgb(0xff, 0xe0, 0x6d),
            bright_blue: Color32::from_rgb(0x7d, 0xaf, 0xff),
            bright_magenta: Color32::from_rgb(0xdf, 0xbd, 0xff),
            bright_cyan: Color32::from_rgb(0x7d, 0xcf, 0xff),
            bright_white: Color32::from_rgb(0xdc, 0xdf, 0xe4),
            color_256: Vec::with_capacity(256),
            background: Color32::from_rgb(0x1a, 0x1b, 0x26),
            foreground: Color32::from_rgb(0xa9, 0xb1, 0xd6),
            cursor: Color32::from_rgb(0xc0, 0xca, 0xf5),
            cursor_text: Color32::from_rgb(0x1a, 0x1b, 0x26),
            selection: Color32::from_rgb(0x28, 0x3a, 0x4d),
            selection_text: Color32::from_rgb(0xa9, 0xb1, 0xd6),
        };
        palette.generate_256_colors();
        palette
    }

    /// Catppuccin Mocha palette
    pub fn catppuccin_mocha() -> Self {
        let mut palette = Self {
            black: Color32::from_rgb(0x1e, 0x1e, 0x2e),
            red: Color32::from_rgb(0xf3, 0x8b, 0xa8),
            green: Color32::from_rgb(0xa6, 0xe3, 0xa1),
            yellow: Color32::from_rgb(0xf9, 0xe2, 0xaf),
            blue: Color32::from_rgb(0x89, 0xb4, 0xfa),
            magenta: Color32::from_rgb(0xf5, 0xc2, 0xe7),
            cyan: Color32::from_rgb(0x94, 0xe2, 0xd5),
            white: Color32::from_rgb(0xba, 0xbe, 0xf2),
            bright_black: Color32::from_rgb(0x31, 0x32, 0x44),
            bright_red: Color32::from_rgb(0xf3, 0x8b, 0xa8),
            bright_green: Color32::from_rgb(0xa6, 0xe3, 0xa1),
            bright_yellow: Color32::from_rgb(0xf9, 0xe2, 0xaf),
            bright_blue: Color32::from_rgb(0x89, 0xb4, 0xfa),
            bright_magenta: Color32::from_rgb(0xf5, 0xc2, 0xe7),
            bright_cyan: Color32::from_rgb(0x94, 0xe2, 0xd5),
            bright_white: Color32::from_rgb(0xa6, 0xad, 0xc8),
            color_256: Vec::with_capacity(256),
            background: Color32::from_rgb(0x1e, 0x1e, 0x2e),
            foreground: Color32::from_rgb(0xcd, 0xd6, 0xf4),
            cursor: Color32::from_rgb(0xf5, 0xe0, 0xdc),
            cursor_text: Color32::from_rgb(0x1e, 0x1e, 0x2e),
            selection: Color32::from_rgb(0x35, 0x3b, 0x52),
            selection_text: Color32::from_rgb(0xcd, 0xd6, 0xf4),
        };
        palette.generate_256_colors();
        palette
    }

    /// Gruvbox Dark palette
    pub fn gruvbox_dark() -> Self {
        let mut palette = Self {
            black: Color32::from_rgb(0x28, 0x28, 0x28),
            red: Color32::from_rgb(0xcc, 0x24, 0x1d),
            green: Color32::from_rgb(0x98, 0x97, 0x1a),
            yellow: Color32::from_rgb(0xd7, 0x99, 0x21),
            blue: Color32::from_rgb(0x45, 0x85, 0x88),
            magenta: Color32::from_rgb(0xb1, 0x62, 0x86),
            cyan: Color32::from_rgb(0x68, 0x9d, 0x6a),
            white: Color32::from_rgb(0xa8, 0x99, 0x84),
            bright_black: Color32::from_rgb(0x92, 0x83, 0x74),
            bright_red: Color32::from_rgb(0xfb, 0x49, 0x34),
            bright_green: Color32::from_rgb(0xb8, 0xbb, 0x26),
            bright_yellow: Color32::from_rgb(0xfa, 0xbd, 0x2f),
            bright_blue: Color32::from_rgb(0x83, 0xa5, 0x98),
            bright_magenta: Color32::from_rgb(0xd3, 0x86, 0x9b),
            bright_cyan: Color32::from_rgb(0x8e, 0xc0, 0x7c),
            bright_white: Color32::from_rgb(0xeb, 0xdb, 0xb2),
            color_256: Vec::with_capacity(256),
            background: Color32::from_rgb(0x28, 0x28, 0x28),
            foreground: Color32::from_rgb(0xeb, 0xdb, 0xb2),
            cursor: Color32::from_rgb(0xeb, 0xdb, 0xb2),
            cursor_text: Color32::from_rgb(0x28, 0x28, 0x28),
            selection: Color32::from_rgb(0x68, 0x9d, 0x6a),
            selection_text: Color32::from_rgb(0x28, 0x28, 0x28),
        };
        palette.generate_256_colors();
        palette
    }

    /// Generate 256-color lookup table using xterm formula
    fn generate_256_colors(&mut self) {
        // First 16 are the standard colors
        self.color_256.push(self.black);
        self.color_256.push(self.red);
        self.color_256.push(self.green);
        self.color_256.push(self.yellow);
        self.color_256.push(self.blue);
        self.color_256.push(self.magenta);
        self.color_256.push(self.cyan);
        self.color_256.push(self.white);
        self.color_256.push(self.bright_black);
        self.color_256.push(self.bright_red);
        self.color_256.push(self.bright_green);
        self.color_256.push(self.bright_yellow);
        self.color_256.push(self.bright_blue);
        self.color_256.push(self.bright_magenta);
        self.color_256.push(self.bright_cyan);
        self.color_256.push(self.bright_white);

        // 216 colors: 16-231 (6x6x6 RGB cube)
        for r in 0..6 {
            for g in 0..6 {
                for b in 0..6 {
                    let red = if r == 0 { 0 } else { 55 + r * 40 };
                    let green = if g == 0 { 0 } else { 55 + g * 40 };
                    let blue = if b == 0 { 0 } else { 55 + b * 40 };
                    self.color_256.push(Color32::from_rgb(red, green, blue));
                }
            }
        }

        // 24 grays: 232-255
        for i in 0..24 {
            let gray = 8 + i * 10;
            self.color_256.push(Color32::from_rgb(gray, gray, gray));
        }
    }

    /// Get color by ANSI index (0-255)
    pub fn get_ansi_color(&self, index: u8) -> Color32 {
        if (index as usize) < self.color_256.len() {
            self.color_256[index as usize]
        } else {
            self.foreground
        }
    }

    /// Get color by name
    pub fn by_name(&self, name: &str) -> Option<Color32> {
        match name {
            "black" => Some(self.black),
            "red" => Some(self.red),
            "green" => Some(self.green),
            "yellow" => Some(self.yellow),
            "blue" => Some(self.blue),
            "magenta" => Some(self.magenta),
            "cyan" => Some(self.cyan),
            "white" => Some(self.white),
            "bright_black" => Some(self.bright_black),
            "bright_red" => Some(self.bright_red),
            "bright_green" => Some(self.bright_green),
            "bright_yellow" => Some(self.bright_yellow),
            "bright_blue" => Some(self.bright_blue),
            "bright_magenta" => Some(self.bright_magenta),
            "bright_cyan" => Some(self.bright_cyan),
            "bright_white" => Some(self.bright_white),
            "background" => Some(self.background),
            "foreground" => Some(self.foreground),
            "cursor" => Some(self.cursor),
            "selection" => Some(self.selection),
            _ => None,
        }
    }
}

// ============================================================================
// SEMANTIC HIGHLIGHTING
// ============================================================================

/// Semantic token types for shell syntax highlighting
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum SemanticTokenType {
    Command,   // Shell built-ins and commands
    Argument,  // Command arguments
    Path,      // File paths
    String,    // Quoted strings
    Variable,  // Environment variables
    Comment,   // Shell comments
    Keyword,   // Shell keywords (if, for, while, etc.)
    Operator,  // Operators (|, &&, ||, etc.)
    Number,    // Numeric values
    Error,     // Syntax errors
    Function,  // Function definitions
    Parameter, // Parameters ($1, $2, etc.)
}

/// Semantic highlighting configuration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SemanticHighlighting {
    pub enabled: bool,
    #[serde(with = "serde_color32")]
    pub command_color: Color32,
    #[serde(with = "serde_color32")]
    pub argument_color: Color32,
    #[serde(with = "serde_color32")]
    pub path_color: Color32,
    #[serde(with = "serde_color32")]
    pub string_color: Color32,
    #[serde(with = "serde_color32")]
    pub variable_color: Color32,
    #[serde(with = "serde_color32")]
    pub comment_color: Color32,
    #[serde(with = "serde_color32")]
    pub keyword_color: Color32,
    #[serde(with = "serde_color32")]
    pub operator_color: Color32,
    #[serde(with = "serde_color32")]
    pub number_color: Color32,
    #[serde(with = "serde_color32")]
    pub error_color: Color32,
    #[serde(with = "serde_color32")]
    pub function_color: Color32,
    #[serde(with = "serde_color32")]
    pub parameter_color: Color32,
    pub bold_commands: bool,
    pub italic_comments: bool,
    pub underline_paths: bool,
}

impl Default for SemanticHighlighting {
    fn default() -> Self {
        Self::one_dark()
    }
}

impl SemanticHighlighting {
    pub fn one_dark() -> Self {
        Self {
            enabled: true,
            command_color: Color32::from_rgb(0x61, 0xaf, 0xef), // Blue
            argument_color: Color32::from_rgb(0xab, 0xb2, 0xbf), // White/gray
            path_color: Color32::from_rgb(0xe5, 0xc0, 0x7b),    // Yellow
            string_color: Color32::from_rgb(0x98, 0xc3, 0x79),  // Green
            variable_color: Color32::from_rgb(0xe0, 0x6c, 0x75), // Red
            comment_color: Color32::from_rgb(0x5c, 0x63, 0x70), // Bright black
            keyword_color: Color32::from_rgb(0xc6, 0x78, 0xdd), // Magenta
            operator_color: Color32::from_rgb(0x56, 0xb6, 0xc2), // Cyan
            number_color: Color32::from_rgb(0xd1, 0x9a, 0x66),  // Orange
            error_color: Color32::from_rgb(0xff, 0x00, 0x00),   // Bright red
            function_color: Color32::from_rgb(0xd1, 0x9a, 0x66), // Orange
            parameter_color: Color32::from_rgb(0xe0, 0x6c, 0x75), // Red
            bold_commands: true,
            italic_comments: true,
            underline_paths: true,
        }
    }

    pub fn get_color(&self, token_type: SemanticTokenType) -> Color32 {
        match token_type {
            SemanticTokenType::Command => self.command_color,
            SemanticTokenType::Argument => self.argument_color,
            SemanticTokenType::Path => self.path_color,
            SemanticTokenType::String => self.string_color,
            SemanticTokenType::Variable => self.variable_color,
            SemanticTokenType::Comment => self.comment_color,
            SemanticTokenType::Keyword => self.keyword_color,
            SemanticTokenType::Operator => self.operator_color,
            SemanticTokenType::Number => self.number_color,
            SemanticTokenType::Error => self.error_color,
            SemanticTokenType::Function => self.function_color,
            SemanticTokenType::Parameter => self.parameter_color,
        }
    }
}

// ============================================================================
// FONT TUNING
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FontWeight {
    Thin,       // 100
    ExtraLight, // 200
    Light,      // 300
    Regular,    // 400
    Medium,     // 500
    SemiBold,   // 600
    Bold,       // 700
    ExtraBold,  // 800
    Black,      // 900
}

impl FontWeight {
    pub fn to_f32(&self) -> f32 {
        match self {
            FontWeight::Thin => 100.0,
            FontWeight::ExtraLight => 200.0,
            FontWeight::Light => 300.0,
            FontWeight::Regular => 400.0,
            FontWeight::Medium => 500.0,
            FontWeight::SemiBold => 600.0,
            FontWeight::Bold => 700.0,
            FontWeight::ExtraBold => 800.0,
            FontWeight::Black => 900.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FontTuning {
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub line_height: f32,    // Multiplier (1.0 = normal)
    pub letter_spacing: f32, // In pixels
    pub ligatures: bool,
}

impl Default for FontTuning {
    fn default() -> Self {
        Self {
            font_family: "Cascadia Code".to_string(),
            font_size: 14.0,
            font_weight: FontWeight::Regular,
            line_height: 1.2,
            letter_spacing: 0.0,
            ligatures: true,
        }
    }
}

// ============================================================================
// BACKGROUND SETTINGS
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BackgroundSettings {
    pub enabled: bool,
    pub image_path: Option<PathBuf>,
    pub opacity: f32, // 0.0 - 1.0
    pub stretch_mode: BackgroundStretchMode,
    pub blur_radius: f32, // Gaussian blur in pixels
    pub darkening: f32,   // 0.0 - 1.0 (for better text readability)
    pub blend_mode: BlendMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackgroundStretchMode {
    Fill,
    Uniform,
    UniformToFill,
    Tile,
    Center,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
}

impl Default for BackgroundSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            image_path: None,
            opacity: 0.3,
            stretch_mode: BackgroundStretchMode::UniformToFill,
            blur_radius: 8.0,
            darkening: 0.5,
            blend_mode: BlendMode::Normal,
        }
    }
}

// ============================================================================
// COMPLETE THEME DEFINITION
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TerminalTheme {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub version: String,
    pub palette: TerminalPalette,
    pub semantic_highlighting: SemanticHighlighting,
    pub cursor_style: CursorStyle,
    pub cursor_blink: CursorBlinkMode,
    pub cursor_blink_interval_ms: u64,
    pub font_tuning: FontTuning,
    pub background: BackgroundSettings,
    pub transparency: f32,     // Terminal background transparency (0.0 - 1.0)
    pub use_bright_bold: bool, // Show bold text in bright colors
    pub scrollback_lines: usize,
}

impl Default for TerminalTheme {
    fn default() -> Self {
        Self::one_dark()
    }
}

impl TerminalTheme {
    /// Create One Dark theme (default)
    pub fn one_dark() -> Self {
        Self {
            id: "one-dark".to_string(),
            name: "One Dark".to_string(),
            author: "Atom".to_string(),
            description: "Atom's iconic dark theme".to_string(),
            version: "1.0".to_string(),
            palette: TerminalPalette::one_dark(),
            semantic_highlighting: SemanticHighlighting::one_dark(),
            cursor_style: CursorStyle::Block,
            cursor_blink: CursorBlinkMode::Blink,
            cursor_blink_interval_ms: 500,
            font_tuning: FontTuning::default(),
            background: BackgroundSettings::default(),
            transparency: 0.0,
            use_bright_bold: true,
            scrollback_lines: 10000,
        }
    }

    /// Create Dracula theme
    pub fn dracula() -> Self {
        Self {
            id: "dracula".to_string(),
            name: "Dracula".to_string(),
            author: "Zeno Rocha".to_string(),
            description: "A dark theme for many editors, shells, and more".to_string(),
            version: "1.0".to_string(),
            palette: TerminalPalette::dracula(),
            semantic_highlighting: SemanticHighlighting::one_dark(),
            cursor_style: CursorStyle::Block,
            cursor_blink: CursorBlinkMode::Blink,
            cursor_blink_interval_ms: 500,
            font_tuning: FontTuning::default(),
            background: BackgroundSettings::default(),
            transparency: 0.0,
            use_bright_bold: true,
            scrollback_lines: 10000,
        }
    }

    /// Create Solarized Dark theme
    pub fn solarized_dark() -> Self {
        Self {
            id: "solarized-dark".to_string(),
            name: "Solarized Dark".to_string(),
            author: "Ethan Schoonover".to_string(),
            description: "Precision colors for machines and people".to_string(),
            version: "1.0".to_string(),
            palette: TerminalPalette::solarized_dark(),
            semantic_highlighting: SemanticHighlighting::one_dark(),
            cursor_style: CursorStyle::Block,
            cursor_blink: CursorBlinkMode::Blink,
            cursor_blink_interval_ms: 500,
            font_tuning: FontTuning::default(),
            background: BackgroundSettings::default(),
            transparency: 0.0,
            use_bright_bold: true,
            scrollback_lines: 10000,
        }
    }

    /// Create Solarized Light theme
    pub fn solarized_light() -> Self {
        Self {
            id: "solarized-light".to_string(),
            name: "Solarized Light".to_string(),
            author: "Ethan Schoonover".to_string(),
            description: "Precision colors for machines and people".to_string(),
            version: "1.0".to_string(),
            palette: TerminalPalette::solarized_light(),
            semantic_highlighting: SemanticHighlighting::one_dark(),
            cursor_style: CursorStyle::Block,
            cursor_blink: CursorBlinkMode::Blink,
            cursor_blink_interval_ms: 500,
            font_tuning: FontTuning::default(),
            background: BackgroundSettings::default(),
            transparency: 0.0,
            use_bright_bold: true,
            scrollback_lines: 10000,
        }
    }

    /// Create Monokai theme
    pub fn monokai() -> Self {
        Self {
            id: "monokai".to_string(),
            name: "Monokai".to_string(),
            author: "Wimer Hazenberg".to_string(),
            description: "A colorful, high-contrast theme".to_string(),
            version: "1.0".to_string(),
            palette: TerminalPalette::monokai(),
            semantic_highlighting: SemanticHighlighting::one_dark(),
            cursor_style: CursorStyle::Block,
            cursor_blink: CursorBlinkMode::Blink,
            cursor_blink_interval_ms: 500,
            font_tuning: FontTuning::default(),
            background: BackgroundSettings::default(),
            transparency: 0.0,
            use_bright_bold: true,
            scrollback_lines: 10000,
        }
    }

    /// Create Nord theme
    pub fn nord() -> Self {
        Self {
            id: "nord".to_string(),
            name: "Nord".to_string(),
            author: "Arctic Ice Studio".to_string(),
            description: "An arctic, north-bluish clean and elegant theme".to_string(),
            version: "1.0".to_string(),
            palette: TerminalPalette::nord(),
            semantic_highlighting: SemanticHighlighting::one_dark(),
            cursor_style: CursorStyle::Block,
            cursor_blink: CursorBlinkMode::Blink,
            cursor_blink_interval_ms: 500,
            font_tuning: FontTuning::default(),
            background: BackgroundSettings::default(),
            transparency: 0.0,
            use_bright_bold: true,
            scrollback_lines: 10000,
        }
    }

    /// Create GitHub Dark theme
    pub fn github_dark() -> Self {
        Self {
            id: "github-dark".to_string(),
            name: "GitHub Dark".to_string(),
            author: "GitHub".to_string(),
            description: "GitHub's dark theme".to_string(),
            version: "1.0".to_string(),
            palette: TerminalPalette::github_dark(),
            semantic_highlighting: SemanticHighlighting::one_dark(),
            cursor_style: CursorStyle::Block,
            cursor_blink: CursorBlinkMode::Blink,
            cursor_blink_interval_ms: 500,
            font_tuning: FontTuning::default(),
            background: BackgroundSettings::default(),
            transparency: 0.0,
            use_bright_bold: true,
            scrollback_lines: 10000,
        }
    }

    /// Create GitHub Light theme
    pub fn github_light() -> Self {
        Self {
            id: "github-light".to_string(),
            name: "GitHub Light".to_string(),
            author: "GitHub".to_string(),
            description: "GitHub's light theme".to_string(),
            version: "1.0".to_string(),
            palette: TerminalPalette::github_light(),
            semantic_highlighting: SemanticHighlighting::one_dark(),
            cursor_style: CursorStyle::Block,
            cursor_blink: CursorBlinkMode::Blink,
            cursor_blink_interval_ms: 500,
            font_tuning: FontTuning::default(),
            background: BackgroundSettings::default(),
            transparency: 0.0,
            use_bright_bold: true,
            scrollback_lines: 10000,
        }
    }

    /// Create Tokyo Night theme
    pub fn tokyo_night() -> Self {
        Self {
            id: "tokyo-night".to_string(),
            name: "Tokyo Night".to_string(),
            author: "enkia".to_string(),
            description: "A clean, dark theme that celebrates the lights of Downtown Tokyo"
                .to_string(),
            version: "1.0".to_string(),
            palette: TerminalPalette::tokyo_night(),
            semantic_highlighting: SemanticHighlighting::one_dark(),
            cursor_style: CursorStyle::Block,
            cursor_blink: CursorBlinkMode::Blink,
            cursor_blink_interval_ms: 500,
            font_tuning: FontTuning::default(),
            background: BackgroundSettings::default(),
            transparency: 0.0,
            use_bright_bold: true,
            scrollback_lines: 10000,
        }
    }

    /// Create Catppuccin Mocha theme
    pub fn catppuccin_mocha() -> Self {
        Self {
            id: "catppuccin-mocha".to_string(),
            name: "Catppuccin Mocha".to_string(),
            author: "Catppuccin".to_string(),
            description: "Soothing pastel theme".to_string(),
            version: "1.0".to_string(),
            palette: TerminalPalette::catppuccin_mocha(),
            semantic_highlighting: SemanticHighlighting::one_dark(),
            cursor_style: CursorStyle::Block,
            cursor_blink: CursorBlinkMode::Blink,
            cursor_blink_interval_ms: 500,
            font_tuning: FontTuning::default(),
            background: BackgroundSettings::default(),
            transparency: 0.0,
            use_bright_bold: true,
            scrollback_lines: 10000,
        }
    }

    /// Create Gruvbox Dark theme
    pub fn gruvbox_dark() -> Self {
        Self {
            id: "gruvbox-dark".to_string(),
            name: "Gruvbox Dark".to_string(),
            author: "Morhetz".to_string(),
            description: "Retro groove color scheme".to_string(),
            version: "1.0".to_string(),
            palette: TerminalPalette::gruvbox_dark(),
            semantic_highlighting: SemanticHighlighting::one_dark(),
            cursor_style: CursorStyle::Block,
            cursor_blink: CursorBlinkMode::Blink,
            cursor_blink_interval_ms: 500,
            font_tuning: FontTuning::default(),
            background: BackgroundSettings::default(),
            transparency: 0.0,
            use_bright_bold: true,
            scrollback_lines: 10000,
        }
    }

    /// Get all built-in themes
    pub fn all_built_in() -> Vec<Self> {
        vec![
            Self::one_dark(),
            Self::dracula(),
            Self::solarized_dark(),
            Self::solarized_light(),
            Self::monokai(),
            Self::nord(),
            Self::github_dark(),
            Self::github_light(),
            Self::tokyo_night(),
            Self::catppuccin_mocha(),
            Self::gruvbox_dark(),
        ]
    }
}

// ============================================================================
// DYNAMIC THEME SWITCHING
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DynamicThemeConfig {
    pub enabled: bool,
    pub day_theme_id: String,
    pub night_theme_id: String,
    pub day_start_hour: u8,   // 6 = 6:00 AM
    pub night_start_hour: u8, // 18 = 6:00 PM
    pub transition_duration_minutes: u8,
    pub use_system_theme: bool, // Use Windows light/dark mode
}

impl Default for DynamicThemeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            day_theme_id: "solarized-light".to_string(),
            night_theme_id: "one-dark".to_string(),
            day_start_hour: 7,
            night_start_hour: 19,
            transition_duration_minutes: 30,
            use_system_theme: true,
        }
    }
}

impl DynamicThemeConfig {
    /// Get current theme based on time
    pub fn current_theme_id(&self) -> &str {
        if !self.enabled {
            return &self.day_theme_id;
        }

        let now = chrono::Local::now();
        let hour = now.format("%H").to_string().parse::<u8>().unwrap_or(12);

        if hour >= self.day_start_hour && hour < self.night_start_hour {
            &self.day_theme_id
        } else {
            &self.night_theme_id
        }
    }

    /// Get system theme preference (Windows only)
    #[cfg(target_os = "windows")]
    pub fn get_system_theme() -> Option<bool> {
        // Simplified for now - return None to use default
        // The Windows registry API has changed significantly
        None
    }

    #[cfg(not(target_os = "windows"))]
    pub fn get_system_theme() -> Option<bool> {
        None
    }
}

// ============================================================================
// COMMUNITY THEME STORE
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CommunityTheme {
    pub id: String,
    pub name: String,
    pub author: String,
    pub author_url: Option<String>,
    pub description: String,
    pub version: String,
    pub downloads: u64,
    pub rating: f32,
    pub tags: Vec<String>,
    pub preview_url: Option<String>,
    pub download_url: String,
    pub theme_data: Option<TerminalTheme>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ThemeStore {
    pub themes: Vec<CommunityTheme>,
    pub last_updated: Option<String>,
    pub featured_ids: Vec<String>,
}

impl ThemeStore {
    /// Load store from cache
    pub fn load_cache() -> Option<Self> {
        let cache_path = Self::cache_path()?;
        if let Ok(content) = std::fs::read_to_string(&cache_path) {
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }

    /// Save store to cache
    pub fn save_cache(&self) -> anyhow::Result<()> {
        let cache_path =
            Self::cache_path().ok_or_else(|| anyhow::anyhow!("Could not determine cache path"))?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(cache_path, content)?;
        Ok(())
    }

    fn cache_path() -> Option<PathBuf> {
        dirs::cache_dir().map(|p| p.join("easyssh").join("theme-store.json"))
    }
}

// ============================================================================
// VS CODE THEME IMPORT/EXPORT
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VSCodeThemeImport {
    pub name: String,
    #[serde(rename = "type")]
    pub theme_type: String,
    pub colors: HashMap<String, serde_json::Value>,
    pub token_colors: Vec<TokenColor>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenColor {
    pub name: Option<String>,
    pub scope: Option<serde_json::Value>,
    pub settings: TokenSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TokenSettings {
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub font_style: Option<String>,
}

/// Convert VS Code theme to TerminalTheme
pub fn import_vscode_theme(json_content: &str) -> anyhow::Result<TerminalTheme> {
    let vscode_theme: VSCodeThemeImport = serde_json::from_str(json_content)?;

    let mut theme = TerminalTheme::one_dark();
    theme.name = vscode_theme.name;
    theme.id = sanitize_id(&theme.name);
    theme.author = "Imported from VS Code".to_string();

    // Map VS Code colors to terminal palette
    let colors = &vscode_theme.colors;

    if let Some(bg) = colors
        .get("terminal.background")
        .and_then(parse_vscode_color)
    {
        theme.palette.background = bg;
    }

    if let Some(fg) = colors
        .get("terminal.foreground")
        .and_then(parse_vscode_color)
    {
        theme.palette.foreground = fg;
    }

    // Map ANSI colors
    if let Some(c) = colors
        .get("terminal.ansiBlack")
        .and_then(parse_vscode_color)
    {
        theme.palette.black = c;
    }
    if let Some(c) = colors.get("terminal.ansiRed").and_then(parse_vscode_color) {
        theme.palette.red = c;
    }
    if let Some(c) = colors
        .get("terminal.ansiGreen")
        .and_then(parse_vscode_color)
    {
        theme.palette.green = c;
    }
    if let Some(c) = colors
        .get("terminal.ansiYellow")
        .and_then(parse_vscode_color)
    {
        theme.palette.yellow = c;
    }
    if let Some(c) = colors.get("terminal.ansiBlue").and_then(parse_vscode_color) {
        theme.palette.blue = c;
    }
    if let Some(c) = colors
        .get("terminal.ansiMagenta")
        .and_then(parse_vscode_color)
    {
        theme.palette.magenta = c;
    }
    if let Some(c) = colors.get("terminal.ansiCyan").and_then(parse_vscode_color) {
        theme.palette.cyan = c;
    }
    if let Some(c) = colors
        .get("terminal.ansiWhite")
        .and_then(parse_vscode_color)
    {
        theme.palette.white = c;
    }

    // Bright colors
    if let Some(c) = colors
        .get("terminal.ansiBrightBlack")
        .and_then(parse_vscode_color)
    {
        theme.palette.bright_black = c;
    }
    if let Some(c) = colors
        .get("terminal.ansiBrightRed")
        .and_then(parse_vscode_color)
    {
        theme.palette.bright_red = c;
    }
    if let Some(c) = colors
        .get("terminal.ansiBrightGreen")
        .and_then(parse_vscode_color)
    {
        theme.palette.bright_green = c;
    }
    if let Some(c) = colors
        .get("terminal.ansiBrightYellow")
        .and_then(parse_vscode_color)
    {
        theme.palette.bright_yellow = c;
    }
    if let Some(c) = colors
        .get("terminal.ansiBrightBlue")
        .and_then(parse_vscode_color)
    {
        theme.palette.bright_blue = c;
    }
    if let Some(c) = colors
        .get("terminal.ansiBrightMagenta")
        .and_then(parse_vscode_color)
    {
        theme.palette.bright_magenta = c;
    }
    if let Some(c) = colors
        .get("terminal.ansiBrightCyan")
        .and_then(parse_vscode_color)
    {
        theme.palette.bright_cyan = c;
    }
    if let Some(c) = colors
        .get("terminal.ansiBrightWhite")
        .and_then(parse_vscode_color)
    {
        theme.palette.bright_white = c;
    }

    // Cursor and selection
    if let Some(c) = colors
        .get("terminalCursor.background")
        .and_then(parse_vscode_color)
    {
        theme.palette.cursor = c;
    }
    if let Some(c) = colors
        .get("terminal.selectionBackground")
        .and_then(parse_vscode_color)
    {
        theme.palette.selection = c;
    }

    // Regenerate 256-color table
    theme.palette.generate_256_colors();

    Ok(theme)
}

/// Export TerminalTheme to VS Code format
pub fn export_vscode_theme(theme: &TerminalTheme) -> anyhow::Result<String> {
    let mut colors = HashMap::new();

    // Terminal colors
    colors.insert(
        "terminal.background".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.background)),
    );
    colors.insert(
        "terminal.foreground".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.foreground)),
    );

    // ANSI colors
    colors.insert(
        "terminal.ansiBlack".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.black)),
    );
    colors.insert(
        "terminal.ansiRed".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.red)),
    );
    colors.insert(
        "terminal.ansiGreen".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.green)),
    );
    colors.insert(
        "terminal.ansiYellow".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.yellow)),
    );
    colors.insert(
        "terminal.ansiBlue".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.blue)),
    );
    colors.insert(
        "terminal.ansiMagenta".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.magenta)),
    );
    colors.insert(
        "terminal.ansiCyan".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.cyan)),
    );
    colors.insert(
        "terminal.ansiWhite".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.white)),
    );

    // Bright colors
    colors.insert(
        "terminal.ansiBrightBlack".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.bright_black)),
    );
    colors.insert(
        "terminal.ansiBrightRed".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.bright_red)),
    );
    colors.insert(
        "terminal.ansiBrightGreen".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.bright_green)),
    );
    colors.insert(
        "terminal.ansiBrightYellow".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.bright_yellow)),
    );
    colors.insert(
        "terminal.ansiBrightBlue".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.bright_blue)),
    );
    colors.insert(
        "terminal.ansiBrightMagenta".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.bright_magenta)),
    );
    colors.insert(
        "terminal.ansiBrightCyan".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.bright_cyan)),
    );
    colors.insert(
        "terminal.ansiBrightWhite".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.bright_white)),
    );

    colors.insert(
        "terminalCursor.background".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.cursor)),
    );
    colors.insert(
        "terminal.selectionBackground".to_string(),
        serde_json::Value::String(color_to_hex(theme.palette.selection)),
    );

    let vscode_theme = VSCodeThemeImport {
        name: theme.name.clone(),
        theme_type: "dark".to_string(),
        colors,
        token_colors: vec![],
    };

    Ok(serde_json::to_string_pretty(&vscode_theme)?)
}

fn parse_vscode_color(value: &serde_json::Value) -> Option<Color32> {
    let hex = value.as_str()?;
    hex_to_color(hex)
}

fn hex_to_color(hex: &str) -> Option<Color32> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color32::from_rgb(r, g, b))
    } else {
        None
    }
}

fn color_to_hex(color: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}

fn sanitize_id(name: &str) -> String {
    name.to_lowercase()
        .replace([' ', '_'], "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "")
}

// ============================================================================
// THEME MANAGER
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ThemeManager {
    pub current_theme: TerminalTheme,
    pub custom_themes: HashMap<String, TerminalTheme>,
    pub dynamic_config: DynamicThemeConfig,
    pub favorites: Vec<String>,
    pub recently_used: Vec<String>,
    pub store: ThemeStore,
}

impl ThemeManager {
    pub fn new() -> Self {
        Self::load().unwrap_or_default()
    }

    /// Load themes from disk
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config path"))?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let manager: ThemeManager = serde_json::from_str(&content)?;
            Ok(manager)
        } else {
            Ok(Self::default())
        }
    }

    /// Save themes to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config path"))?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("easyssh").join("themes.json"))
    }

    /// Set current theme by ID
    pub fn set_theme(&mut self, theme_id: &str) -> bool {
        // Check custom themes first
        if let Some(theme) = self.custom_themes.get(theme_id).cloned() {
            self.current_theme = theme;
            self.add_to_recently_used(theme_id);
            // Save to disk immediately
            if let Err(e) = self.save() {
                tracing::warn!("Failed to save theme after set_theme: {}", e);
            }
            return true;
        }

        // Check built-in themes
        for theme in TerminalTheme::all_built_in() {
            if theme.id == theme_id {
                self.current_theme = theme;
                self.add_to_recently_used(theme_id);
                // Save to disk immediately
                if let Err(e) = self.save() {
                    tracing::warn!("Failed to save theme after set_theme: {}", e);
                }
                return true;
            }
        }
        false
    }

    fn add_to_recently_used(&mut self, theme_id: &str) {
        self.recently_used.retain(|id| id != theme_id);
        self.recently_used.insert(0, theme_id.to_string());
        if self.recently_used.len() > 10 {
            self.recently_used.truncate(10);
        }
    }

    /// Add or update custom theme with error handling
    pub fn save_custom_theme(&mut self, theme: TerminalTheme) -> anyhow::Result<()> {
        // Validate theme before saving
        if theme.id.is_empty() {
            return Err(anyhow::anyhow!("Theme ID cannot be empty"));
        }
        if theme.name.is_empty() {
            return Err(anyhow::anyhow!("Theme name cannot be empty"));
        }
        if !theme
            .id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(anyhow::anyhow!(
                "Theme ID contains invalid characters: {}",
                theme.id
            ));
        }

        self.custom_themes.insert(theme.id.clone(), theme);
        self.save()
    }

    /// Delete custom theme
    pub fn delete_custom_theme(&mut self, theme_id: &str) -> bool {
        let removed = self.custom_themes.remove(theme_id).is_some();
        if removed {
            let _ = self.save();
        }
        removed
    }

    /// Get all available themes (built-in + custom)
    pub fn all_themes(&self) -> Vec<TerminalTheme> {
        let mut themes = TerminalTheme::all_built_in();
        themes.extend(self.custom_themes.values().cloned());
        themes
    }

    /// Import theme from file
    pub fn import_theme(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(path)?;

        // Try VS Code format first
        let theme = if content.contains("tokenColors") || content.contains("colors") {
            import_vscode_theme(&content)?
        } else {
            // Try native format
            serde_json::from_str(&content)?
        };

        self.save_custom_theme(theme)?;
        Ok(())
    }

    /// Export theme to file
    pub fn export_theme(
        &self,
        theme_id: &str,
        path: &PathBuf,
        format: ExportFormat,
    ) -> anyhow::Result<()> {
        let theme = if let Some(theme) = self.custom_themes.get(theme_id) {
            theme.clone()
        } else {
            TerminalTheme::all_built_in()
                .iter()
                .find(|t| t.id == theme_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Theme not found: {}", theme_id))?
        };

        let content = match format {
            ExportFormat::VSCode => export_vscode_theme(&theme)?,
            ExportFormat::Native => serde_json::to_string_pretty(&theme)?,
        };

        std::fs::write(path, content)?;
        Ok(())
    }

    /// Update dynamic theme check
    pub fn update_dynamic_theme(&mut self) {
        if self.dynamic_config.enabled {
            let theme_id = self.dynamic_config.current_theme_id().to_string();
            if self.current_theme.id != theme_id {
                self.set_theme(&theme_id);
            }
        }
    }

    /// Toggle favorite
    pub fn toggle_favorite(&mut self, theme_id: &str) {
        if self.favorites.contains(&theme_id.to_string()) {
            self.favorites.retain(|id| id != theme_id);
        } else {
            self.favorites.push(theme_id.to_string());
        }
        let _ = self.save();
    }

    pub fn is_favorite(&self, theme_id: &str) -> bool {
        self.favorites.contains(&theme_id.to_string())
    }

    /// Get theme by ID
    pub fn get_theme(&self, theme_id: &str) -> Option<TerminalTheme> {
        self.custom_themes.get(theme_id).cloned().or_else(|| {
            TerminalTheme::all_built_in()
                .into_iter()
                .find(|t| t.id == theme_id)
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExportFormat {
    VSCode,
    Native,
}

// ============================================================================
// THEME EDITOR UI
// ============================================================================

pub struct ThemeEditor {
    pub is_open: bool,
    pub editing_theme: TerminalTheme,
    pub original_id: String,
    pub selected_tab: EditorTab,
    pub color_to_edit: Option<String>,
    pub show_color_picker: bool,
    pub preview_text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorTab {
    Colors,
    Cursor,
    Font,
    Background,
    Semantic,
}

impl Default for ThemeEditor {
    fn default() -> Self {
        Self {
            is_open: false,
            editing_theme: TerminalTheme::one_dark(),
            original_id: "one-dark".to_string(),
            selected_tab: EditorTab::Colors,
            color_to_edit: None,
            show_color_picker: false,
            preview_text: "ls -la /home/user/documents\n\ncd /etc/nginx/sites-enabled\nsudo nano config.conf\n\necho $PATH\nexport VAR=value\n# This is a comment\nfunction setup() {\n  echo \"Setting up...\"\n}".to_string(),
        }
    }
}

impl ThemeEditor {
    pub fn open(&mut self, theme: &TerminalTheme) {
        self.editing_theme = theme.clone();
        self.original_id = theme.id.clone();
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn render(&mut self, ctx: &egui::Context, manager: &mut ThemeManager) {
        if !self.is_open {
            return;
        }

        // Handle ESC key to close
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.close();
            return;
        }

        let mut should_close = false;

        egui::Window::new("Theme Editor")
            .collapsible(false)
            .resizable(true)
            .default_size([900.0, 650.0])
            .show(ctx, |ui| {
                // Close button in top-right corner
                ui.horizontal(|ui| {
                    ui.heading("Theme Editor");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").clicked() {
                            should_close = true;
                        }
                    });
                });
                ui.separator();

                self.render_editor_content(ui, manager);
            });

        if should_close {
            self.close();
        }

        if self.show_color_picker {
            self.render_color_picker(ctx);
        }
    }

    fn render_editor_content(&mut self, ui: &mut egui::Ui, manager: &mut ThemeManager) {
        ui.horizontal(|ui| {
            // Left sidebar - tabs
            ui.vertical(|ui| {
                ui.set_width(140.0);
                ui.set_min_height(500.0);

                self.render_tab_button(ui, "Colors", EditorTab::Colors, "🎨");
                self.render_tab_button(ui, "Cursor", EditorTab::Cursor, "▮");
                self.render_tab_button(ui, "Font", EditorTab::Font, "Aa");
                self.render_tab_button(ui, "Background", EditorTab::Background, "🖼");
                self.render_tab_button(ui, "Semantic", EditorTab::Semantic, "⚡");

                ui.add_space(30.0);

                // Action buttons
                if ui.button("💾 Save Theme").clicked() {
                    // Only modify ID/name if this is a built-in theme (original_id matches editing_theme.id)
                    // and we're not already editing a custom theme
                    if self.editing_theme.id == self.original_id
                        && !self.original_id.ends_with("-custom")
                    {
                        // Create a copy with custom suffix
                        self.editing_theme.id = format!("{}-custom", self.original_id);
                        self.editing_theme.name = format!("{} (Custom)", self.editing_theme.name);
                    }

                    // Save the theme and provide feedback
                    match manager.save_custom_theme(self.editing_theme.clone()) {
                        Ok(_) => {
                            tracing::info!("Theme saved successfully: {}", self.editing_theme.id);
                            manager.set_theme(&self.editing_theme.id);
                            self.close();
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to save theme '{}': {}",
                                self.editing_theme.id,
                                e
                            );
                        }
                    }
                }

                if ui.button("↩ Cancel").clicked() {
                    self.close();
                }
            });

            ui.separator();

            // Center - editing area
            ui.vertical(|ui| {
                ui.set_width(400.0);

                match self.selected_tab {
                    EditorTab::Colors => self.render_colors_tab(ui),
                    EditorTab::Cursor => self.render_cursor_tab(ui),
                    EditorTab::Font => self.render_font_tab(ui),
                    EditorTab::Background => self.render_background_tab(ui),
                    EditorTab::Semantic => self.render_semantic_tab(ui),
                }
            });

            ui.separator();

            // Right - live preview
            ui.vertical(|ui| {
                ui.set_width(300.0);
                ui.heading("Preview");
                ui.add_space(10.0);
                self.render_preview(ui);
            });
        });
    }

    fn render_tab_button(&mut self, ui: &mut egui::Ui, label: &str, tab: EditorTab, icon: &str) {
        let is_active = self.selected_tab == tab;
        let button = egui::Button::new(format!("{} {}", icon, label))
            .min_size([120.0, 36.0].into())
            .fill(if is_active {
                egui::Color32::from_rgb(64, 156, 255)
            } else {
                egui::Color32::TRANSPARENT
            });

        if ui.add(button).clicked() {
            self.selected_tab = tab;
        }
    }

    fn render_colors_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Terminal Colors");
        ui.add_space(10.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Background and foreground
            ui.label("Base Colors");
            if Self::render_color_button(ui, "Background", self.editing_theme.palette.background) {
                self.color_to_edit = Some("background".to_string());
                self.show_color_picker = true;
            }
            if Self::render_color_button(ui, "Foreground", self.editing_theme.palette.foreground) {
                self.color_to_edit = Some("foreground".to_string());
                self.show_color_picker = true;
            }
            if Self::render_color_button(ui, "Cursor", self.editing_theme.palette.cursor) {
                self.color_to_edit = Some("cursor".to_string());
                self.show_color_picker = true;
            }
            if Self::render_color_button(ui, "Selection", self.editing_theme.palette.selection) {
                self.color_to_edit = Some("selection".to_string());
                self.show_color_picker = true;
            }

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(15.0);

            // ANSI colors
            ui.label("ANSI Colors");
            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    ui.label("Normal");
                    if Self::render_color_button(ui, "Black", self.editing_theme.palette.black) {
                        self.color_to_edit = Some("black".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(ui, "Red", self.editing_theme.palette.red) {
                        self.color_to_edit = Some("red".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(ui, "Green", self.editing_theme.palette.green) {
                        self.color_to_edit = Some("green".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(ui, "Yellow", self.editing_theme.palette.yellow) {
                        self.color_to_edit = Some("yellow".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(ui, "Blue", self.editing_theme.palette.blue) {
                        self.color_to_edit = Some("blue".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(ui, "Magenta", self.editing_theme.palette.magenta)
                    {
                        self.color_to_edit = Some("magenta".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(ui, "Cyan", self.editing_theme.palette.cyan) {
                        self.color_to_edit = Some("cyan".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(ui, "White", self.editing_theme.palette.white) {
                        self.color_to_edit = Some("white".to_string());
                        self.show_color_picker = true;
                    }
                });

                cols[1].vertical(|ui| {
                    ui.label("Bright");
                    if Self::render_color_button(
                        ui,
                        "Bright Black",
                        self.editing_theme.palette.bright_black,
                    ) {
                        self.color_to_edit = Some("bright_black".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(
                        ui,
                        "Bright Red",
                        self.editing_theme.palette.bright_red,
                    ) {
                        self.color_to_edit = Some("bright_red".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(
                        ui,
                        "Bright Green",
                        self.editing_theme.palette.bright_green,
                    ) {
                        self.color_to_edit = Some("bright_green".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(
                        ui,
                        "Bright Yellow",
                        self.editing_theme.palette.bright_yellow,
                    ) {
                        self.color_to_edit = Some("bright_yellow".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(
                        ui,
                        "Bright Blue",
                        self.editing_theme.palette.bright_blue,
                    ) {
                        self.color_to_edit = Some("bright_blue".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(
                        ui,
                        "Bright Magenta",
                        self.editing_theme.palette.bright_magenta,
                    ) {
                        self.color_to_edit = Some("bright_magenta".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(
                        ui,
                        "Bright Cyan",
                        self.editing_theme.palette.bright_cyan,
                    ) {
                        self.color_to_edit = Some("bright_cyan".to_string());
                        self.show_color_picker = true;
                    }
                    if Self::render_color_button(
                        ui,
                        "Bright White",
                        self.editing_theme.palette.bright_white,
                    ) {
                        self.color_to_edit = Some("bright_white".to_string());
                        self.show_color_picker = true;
                    }
                });
            });
        });
    }

    fn render_color_button(ui: &mut egui::Ui, label: &str, color: Color32) -> bool {
        let mut clicked = false;
        ui.horizontal(|ui| {
            // Color swatch
            let (rect, response) =
                ui.allocate_exact_size(egui::Vec2::splat(24.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 4.0, color);
            ui.painter()
                .rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::GRAY));

            if response.clicked() {
                clicked = true;
            }

            ui.label(label);
        });
        clicked
    }

    fn render_color_picker(&mut self, ctx: &egui::Context) {
        let color_id = match &self.color_to_edit {
            Some(id) => id.clone(),
            None => return,
        };

        let color = self.get_color_value(&color_id);
        let mut rgb = [
            color.r() as f32 / 255.0,
            color.g() as f32 / 255.0,
            color.b() as f32 / 255.0,
        ];
        let mut should_close = false;

        egui::Window::new("Color Picker")
            .collapsible(false)
            .resizable(false)
            .default_size([300.0, 400.0])
            .show(ctx, |ui| {
                ui.color_edit_button_rgb(&mut rgb);

                ui.add_space(20.0);

                if ui.button("Done").clicked() {
                    should_close = true;
                }
            });

        // Apply changes after the window closes and borrows are released
        if should_close {
            self.show_color_picker = false;
            self.color_to_edit = None;
        }

        // Update the color value
        let new_color = Color32::from_rgb(
            (rgb[0] * 255.0) as u8,
            (rgb[1] * 255.0) as u8,
            (rgb[2] * 255.0) as u8,
        );
        self.set_color_value(&color_id, new_color);
    }

    fn get_color_value(&self, color_id: &str) -> Color32 {
        match color_id {
            "background" => self.editing_theme.palette.background,
            "foreground" => self.editing_theme.palette.foreground,
            "cursor" => self.editing_theme.palette.cursor,
            "selection" => self.editing_theme.palette.selection,
            "black" => self.editing_theme.palette.black,
            "red" => self.editing_theme.palette.red,
            "green" => self.editing_theme.palette.green,
            "yellow" => self.editing_theme.palette.yellow,
            "blue" => self.editing_theme.palette.blue,
            "magenta" => self.editing_theme.palette.magenta,
            "cyan" => self.editing_theme.palette.cyan,
            "white" => self.editing_theme.palette.white,
            "bright_black" => self.editing_theme.palette.bright_black,
            "bright_red" => self.editing_theme.palette.bright_red,
            "bright_green" => self.editing_theme.palette.bright_green,
            "bright_yellow" => self.editing_theme.palette.bright_yellow,
            "bright_blue" => self.editing_theme.palette.bright_blue,
            "bright_magenta" => self.editing_theme.palette.bright_magenta,
            "bright_cyan" => self.editing_theme.palette.bright_cyan,
            "bright_white" => self.editing_theme.palette.bright_white,
            _ => self.editing_theme.palette.foreground,
        }
    }

    fn set_color_value(&mut self, color_id: &str, color: Color32) {
        match color_id {
            "background" => self.editing_theme.palette.background = color,
            "foreground" => self.editing_theme.palette.foreground = color,
            "cursor" => self.editing_theme.palette.cursor = color,
            "selection" => self.editing_theme.palette.selection = color,
            "black" => self.editing_theme.palette.black = color,
            "red" => self.editing_theme.palette.red = color,
            "green" => self.editing_theme.palette.green = color,
            "yellow" => self.editing_theme.palette.yellow = color,
            "blue" => self.editing_theme.palette.blue = color,
            "magenta" => self.editing_theme.palette.magenta = color,
            "cyan" => self.editing_theme.palette.cyan = color,
            "white" => self.editing_theme.palette.white = color,
            "bright_black" => self.editing_theme.palette.bright_black = color,
            "bright_red" => self.editing_theme.palette.bright_red = color,
            "bright_green" => self.editing_theme.palette.bright_green = color,
            "bright_yellow" => self.editing_theme.palette.bright_yellow = color,
            "bright_blue" => self.editing_theme.palette.bright_blue = color,
            "bright_magenta" => self.editing_theme.palette.bright_magenta = color,
            "bright_cyan" => self.editing_theme.palette.bright_cyan = color,
            "bright_white" => self.editing_theme.palette.bright_white = color,
            _ => self.editing_theme.palette.foreground = color,
        }
    }

    fn get_color_mut(&mut self, color_id: &str) -> &mut Color32 {
        match color_id {
            "background" => &mut self.editing_theme.palette.background,
            "foreground" => &mut self.editing_theme.palette.foreground,
            "cursor" => &mut self.editing_theme.palette.cursor,
            "selection" => &mut self.editing_theme.palette.selection,
            "black" => &mut self.editing_theme.palette.black,
            "red" => &mut self.editing_theme.palette.red,
            "green" => &mut self.editing_theme.palette.green,
            "yellow" => &mut self.editing_theme.palette.yellow,
            "blue" => &mut self.editing_theme.palette.blue,
            "magenta" => &mut self.editing_theme.palette.magenta,
            "cyan" => &mut self.editing_theme.palette.cyan,
            "white" => &mut self.editing_theme.palette.white,
            "bright_black" => &mut self.editing_theme.palette.bright_black,
            "bright_red" => &mut self.editing_theme.palette.bright_red,
            "bright_green" => &mut self.editing_theme.palette.bright_green,
            "bright_yellow" => &mut self.editing_theme.palette.bright_yellow,
            "bright_blue" => &mut self.editing_theme.palette.bright_blue,
            "bright_magenta" => &mut self.editing_theme.palette.bright_magenta,
            "bright_cyan" => &mut self.editing_theme.palette.bright_cyan,
            "bright_white" => &mut self.editing_theme.palette.bright_white,
            _ => &mut self.editing_theme.palette.foreground,
        }
    }

    fn render_cursor_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Cursor Settings");
        ui.add_space(15.0);

        // Cursor style
        ui.label("Cursor Style");
        for style in CursorStyle::all() {
            let selected = self.editing_theme.cursor_style == style;
            if ui
                .selectable_label(selected, style.display_name())
                .clicked()
            {
                self.editing_theme.cursor_style = style;
            }
        }

        ui.add_space(20.0);

        // Blink mode
        ui.label("Blink Mode");
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.editing_theme.cursor_blink,
                CursorBlinkMode::Solid,
                "Solid (No Blink)",
            );
        });
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.editing_theme.cursor_blink,
                CursorBlinkMode::Blink,
                "Blink",
            );
        });
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.editing_theme.cursor_blink,
                CursorBlinkMode::Smooth,
                "Smooth (iTerm2 style)",
            );
        });

        ui.add_space(20.0);

        // Blink interval
        ui.horizontal(|ui| {
            ui.label("Blink Interval (ms):");
            ui.add(
                egui::DragValue::new(&mut self.editing_theme.cursor_blink_interval_ms)
                    .speed(50)
                    .range(100..=2000),
            );
        });
    }

    fn render_font_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Font Tuning");
        ui.add_space(15.0);

        // Font family
        ui.horizontal(|ui| {
            ui.label("Font Family:");
            ui.text_edit_singleline(&mut self.editing_theme.font_tuning.font_family);
        });

        ui.add_space(10.0);

        // Font size
        ui.horizontal(|ui| {
            ui.label("Font Size (px):");
            ui.add(
                egui::DragValue::new(&mut self.editing_theme.font_tuning.font_size)
                    .speed(0.5)
                    .range(8.0..=72.0),
            );
        });

        ui.add_space(10.0);

        // Line height
        ui.horizontal(|ui| {
            ui.label("Line Height:");
            ui.add(
                egui::DragValue::new(&mut self.editing_theme.font_tuning.line_height)
                    .speed(0.05)
                    .range(0.8..=2.0),
            );
        });

        ui.add_space(10.0);

        // Letter spacing
        ui.horizontal(|ui| {
            ui.label("Letter Spacing (px):");
            ui.add(
                egui::DragValue::new(&mut self.editing_theme.font_tuning.letter_spacing)
                    .speed(0.1)
                    .range(-2.0..=5.0),
            );
        });

        ui.add_space(10.0);

        // Ligatures
        ui.checkbox(
            &mut self.editing_theme.font_tuning.ligatures,
            "Enable Font Ligatures (=>, !=, etc.)",
        );

        ui.add_space(10.0);

        // Use bright for bold
        ui.checkbox(
            &mut self.editing_theme.use_bright_bold,
            "Use bright colors for bold text",
        );
    }

    fn render_background_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Background Settings");
        ui.add_space(15.0);

        // Enable background image
        ui.checkbox(
            &mut self.editing_theme.background.enabled,
            "Enable Background Image",
        );

        if self.editing_theme.background.enabled {
            ui.add_space(10.0);

            // Image path
            ui.horizontal(|ui| {
                ui.label("Image Path:");
                if let Some(ref path) = self.editing_theme.background.image_path {
                    ui.label(path.to_string_lossy().to_string());
                } else {
                    ui.label("(None selected)");
                }

                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp", "webp"])
                        .pick_file()
                    {
                        self.editing_theme.background.image_path = Some(path);
                    }
                }
            });

            ui.add_space(15.0);

            // Opacity
            ui.horizontal(|ui| {
                ui.label("Image Opacity:");
                ui.add(egui::Slider::new(
                    &mut self.editing_theme.background.opacity,
                    0.0..=1.0,
                ));
            });

            // Blur
            ui.horizontal(|ui| {
                ui.label("Blur Radius:");
                ui.add(
                    egui::DragValue::new(&mut self.editing_theme.background.blur_radius)
                        .speed(0.5)
                        .range(0.0..=50.0),
                );
            });

            // Darkening
            ui.horizontal(|ui| {
                ui.label("Darkening:");
                ui.add(egui::Slider::new(
                    &mut self.editing_theme.background.darkening,
                    0.0..=1.0,
                ));
            });

            ui.add_space(15.0);

            // Transparency (separate from background image)
            ui.label("Terminal Transparency");
            ui.horizontal(|ui| {
                ui.label("Opacity:");
                ui.add(egui::Slider::new(
                    &mut self.editing_theme.transparency,
                    0.0..=1.0,
                ));
            });
        }
    }

    fn render_semantic_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Semantic Highlighting");
        ui.add_space(15.0);

        ui.checkbox(
            &mut self.editing_theme.semantic_highlighting.enabled,
            "Enable Semantic Highlighting",
        );

        if self.editing_theme.semantic_highlighting.enabled {
            ui.add_space(15.0);

            // Borrow semantic_highlighting mutably
            let sem = &mut self.editing_theme.semantic_highlighting;

            // Render each color button directly without nested closures that capture self
            Self::render_sem_color_row(ui, "Commands", &mut sem.command_color);
            Self::render_sem_color_row(ui, "Arguments", &mut sem.argument_color);
            Self::render_sem_color_row(ui, "Paths", &mut sem.path_color);
            Self::render_sem_color_row(ui, "Strings", &mut sem.string_color);
            Self::render_sem_color_row(ui, "Variables", &mut sem.variable_color);
            Self::render_sem_color_row(ui, "Comments", &mut sem.comment_color);
            Self::render_sem_color_row(ui, "Keywords", &mut sem.keyword_color);
            Self::render_sem_color_row(ui, "Operators", &mut sem.operator_color);
            Self::render_sem_color_row(ui, "Numbers", &mut sem.number_color);
            Self::render_sem_color_row(ui, "Functions", &mut sem.function_color);
            Self::render_sem_color_row(ui, "Parameters", &mut sem.parameter_color);

            ui.add_space(15.0);

            ui.checkbox(&mut sem.bold_commands, "Bold Commands");
            ui.checkbox(&mut sem.italic_comments, "Italic Comments");
            ui.checkbox(&mut sem.underline_paths, "Underline Paths");
        }
    }

    fn render_sem_color_row(ui: &mut egui::Ui, label: &str, color: &mut Color32) {
        ui.horizontal(|ui| {
            let (rect, _response) =
                ui.allocate_exact_size(egui::Vec2::splat(20.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 3.0, *color);
            ui.painter()
                .rect_stroke(rect, 3.0, egui::Stroke::new(1.0, egui::Color32::GRAY));
            ui.label(label);
        });
    }

    fn render_sem_color_button(&self, ui: &mut egui::Ui, label: &str, color: &mut Color32) {
        ui.horizontal(|ui| {
            let (rect, response) =
                ui.allocate_exact_size(egui::Vec2::splat(20.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 3.0, *color);
            ui.painter()
                .rect_stroke(rect, 3.0, egui::Stroke::new(1.0, egui::Color32::GRAY));

            if response.clicked() {
                let _rgb = [
                    color.r() as f32 / 255.0,
                    color.g() as f32 / 255.0,
                    color.b() as f32 / 255.0,
                ];
                // Simple color picker dialog would open here
                // For now, use a simple input
                *color = egui::Color32::from_rgb(
                    (_rgb[0] * 255.0) as u8,
                    (_rgb[1] * 255.0) as u8,
                    (_rgb[2] * 255.0) as u8,
                );
            }

            ui.label(label);
        });
    }

    fn render_preview(&self, ui: &mut egui::Ui) {
        let bg = self.editing_theme.palette.background;
        let fg = self.editing_theme.palette.foreground;

        // Preview frame
        let available_width = ui.available_width();
        let height = 250.0;

        let (rect, _response) = ui.allocate_exact_size(
            egui::Vec2::new(available_width, height),
            egui::Sense::hover(),
        );

        // Background
        ui.painter().rect_filled(rect, 4.0, bg);

        // Sample content
        let font_size = self.editing_theme.font_tuning.font_size;
        let font = FontId::new(font_size, FontFamily::Monospace);
        let line_height = font_size * self.editing_theme.font_tuning.line_height;
        let letter_spacing = self.editing_theme.font_tuning.letter_spacing;

        let mut y = rect.min.y + 10.0;

        // Draw cursor
        let cursor_x = rect.min.x + 10.0;
        match self.editing_theme.cursor_style {
            CursorStyle::Block => {
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        egui::Pos2::new(cursor_x, y),
                        egui::Vec2::new(font_size * 0.6, line_height),
                    ),
                    2.0,
                    self.editing_theme.palette.cursor,
                );
                ui.painter().text(
                    egui::Pos2::new(cursor_x + 2.0, y),
                    egui::Align2::LEFT_TOP,
                    "$",
                    font.clone(),
                    self.editing_theme.palette.cursor_text,
                );
            }
            CursorStyle::Line => {
                ui.painter().line_segment(
                    [
                        egui::Pos2::new(cursor_x, y),
                        egui::Pos2::new(cursor_x, y + line_height),
                    ],
                    egui::Stroke::new(2.0, self.editing_theme.palette.cursor),
                );
            }
            CursorStyle::Underscore => {
                ui.painter().line_segment(
                    [
                        egui::Pos2::new(cursor_x, y + line_height - 2.0),
                        egui::Pos2::new(cursor_x + font_size * 0.6, y + line_height - 2.0),
                    ],
                    egui::Stroke::new(2.0, self.editing_theme.palette.cursor),
                );
            }
            CursorStyle::EmptyBox => {
                ui.painter().rect_stroke(
                    egui::Rect::from_min_size(
                        egui::Pos2::new(cursor_x, y),
                        egui::Vec2::new(font_size * 0.6, line_height),
                    ),
                    2.0,
                    egui::Stroke::new(2.0, self.editing_theme.palette.cursor),
                );
            }
        }

        // Draw sample text lines with semantic highlighting
        let sample_lines = vec![
            (
                "ls -la /home/user",
                vec![
                    (
                        "ls",
                        self.editing_theme.semantic_highlighting.command_color,
                        true,
                    ),
                    (
                        " -la",
                        self.editing_theme.semantic_highlighting.argument_color,
                        false,
                    ),
                    (
                        " /home/user",
                        self.editing_theme.semantic_highlighting.path_color,
                        false,
                    ),
                ],
            ),
            (
                "echo $PATH",
                vec![
                    (
                        "echo",
                        self.editing_theme.semantic_highlighting.command_color,
                        true,
                    ),
                    (
                        " $PATH",
                        self.editing_theme.semantic_highlighting.variable_color,
                        false,
                    ),
                ],
            ),
            (
                "# List files",
                vec![(
                    "# List files",
                    self.editing_theme.semantic_highlighting.comment_color,
                    false,
                )],
            ),
            (
                "git status",
                vec![
                    (
                        "git",
                        self.editing_theme.semantic_highlighting.command_color,
                        true,
                    ),
                    (
                        " status",
                        self.editing_theme.semantic_highlighting.argument_color,
                        false,
                    ),
                ],
            ),
        ];

        y += line_height + 5.0;

        for (_full_line, segments) in sample_lines {
            let mut x = rect.min.x + 10.0;
            for (text, color, bold) in segments {
                let _text_with_spacing = text
                    .chars()
                    .map(|c| {
                        if letter_spacing > 0.0 {
                            format!("{}", c)
                        } else {
                            c.to_string()
                        }
                    })
                    .collect::<String>();

                let text_color = if self.editing_theme.semantic_highlighting.enabled {
                    color
                } else {
                    fg
                };

                ui.painter().text(
                    egui::Pos2::new(x, y),
                    egui::Align2::LEFT_TOP,
                    text,
                    if bold && self.editing_theme.semantic_highlighting.bold_commands {
                        FontId::new(font_size * 1.1, FontFamily::Monospace)
                    } else {
                        font.clone()
                    },
                    text_color,
                );

                x += font_size * 0.6 * text.len() as f32 + letter_spacing * text.len() as f32;
            }
            y += line_height;
        }
    }
}

// ============================================================================
// THEME GALLERY UI
// ============================================================================

pub struct ThemeGallery {
    pub is_open: bool,
    pub selected_theme_id: String,
    pub search_query: String,
    pub filter_tag: Option<String>,
    pub view_mode: GalleryViewMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GalleryViewMode {
    Grid,
    List,
}

impl Default for ThemeGallery {
    fn default() -> Self {
        Self {
            is_open: false,
            selected_theme_id: "one-dark".to_string(),
            search_query: String::new(),
            filter_tag: None,
            view_mode: GalleryViewMode::Grid,
        }
    }
}

impl ThemeGallery {
    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        manager: &mut ThemeManager,
        editor: &mut ThemeEditor,
    ) {
        if !self.is_open {
            return;
        }

        // Handle ESC key to close
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.close();
            return;
        }

        let mut should_close = false;

        egui::Window::new("Theme Gallery")
            .collapsible(false)
            .resizable(true)
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                // Close button in top-right corner
                ui.horizontal(|ui| {
                    ui.heading("Theme Gallery");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").clicked() {
                            should_close = true;
                        }
                    });
                });
                ui.separator();

                self.render_gallery_content(ui, manager, editor);
            });

        if should_close {
            self.close();
        }
    }

    fn render_gallery_content(
        &mut self,
        ui: &mut egui::Ui,
        manager: &mut ThemeManager,
        editor: &mut ThemeEditor,
    ) {
        // Toolbar
        ui.horizontal(|ui| {
            // Search
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Search themes...")
                    .desired_width(200.0),
            );

            ui.add_space(20.0);

            // View mode toggle
            ui.horizontal(|ui| {
                let grid_selected = self.view_mode == GalleryViewMode::Grid;
                let list_selected = self.view_mode == GalleryViewMode::List;

                if ui.selectable_label(grid_selected, "⊞ Grid").clicked() {
                    self.view_mode = GalleryViewMode::Grid;
                }
                if ui.selectable_label(list_selected, "☰ List").clicked() {
                    self.view_mode = GalleryViewMode::List;
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✏ Edit Theme").clicked() {
                    if let Some(theme) = manager.get_theme(&self.selected_theme_id) {
                        editor.open(&theme);
                    }
                }

                if ui.button("➕ New Theme").clicked() {
                    let mut new_theme = TerminalTheme::one_dark();
                    new_theme.id = format!("custom-{}", uuid::Uuid::new_v4());
                    new_theme.name = "Custom Theme".to_string();
                    editor.open(&new_theme);
                }

                if ui.button("📥 Import...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Theme files", &["json"])
                        .pick_file()
                    {
                        let _ = manager.import_theme(&path);
                    }
                }
            });
        });

        ui.separator();

        // Categories
        ui.horizontal(|ui| {
            ui.label("Categories:");
            // Use static array with deterministic order to prevent flickering
            // HashSet was causing random iteration order each frame
            let tags = ["All", "Dark", "Light", "Favorites", "Custom"];

            for tag in &tags {
                let selected = self.filter_tag.as_deref() == Some(tag)
                    || (*tag == "All" && self.filter_tag.is_none());
                if ui.selectable_label(selected, *tag).clicked() {
                    self.filter_tag = if *tag == "All" {
                        None
                    } else {
                        Some(tag.to_string())
                    };
                }
            }
        });

        ui.add_space(10.0);

        // Theme grid/list
        let themes = manager.all_themes();
        let filtered_themes: Vec<_> = themes
            .iter()
            .filter(|t| {
                // Search filter
                let matches_search = self.search_query.is_empty()
                    || t.name
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                    || t.author
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase());

                // Tag filter
                let matches_tag = match self.filter_tag.as_deref() {
                    None | Some("All") => true,
                    Some("Dark") => t.palette.background.r() < 100,
                    Some("Light") => t.palette.background.r() > 200,
                    Some("Favorites") => manager.is_favorite(&t.id),
                    Some("Custom") => manager.custom_themes.contains_key(&t.id),
                    _ => true,
                };

                matches_search && matches_tag
            })
            .collect();

        egui::ScrollArea::vertical().show(ui, |ui| match self.view_mode {
            GalleryViewMode::Grid => self.render_grid_view(ui, &filtered_themes, manager),
            GalleryViewMode::List => self.render_list_view(ui, &filtered_themes, manager),
        });
    }

    fn render_grid_view(
        &mut self,
        ui: &mut egui::Ui,
        themes: &[&TerminalTheme],
        manager: &mut ThemeManager,
    ) {
        let available_width = ui.available_width();
        let card_width = 200.0;
        let columns = (available_width / card_width).max(1.0) as usize;

        let mut row_themes = Vec::new();
        for (i, theme) in themes.iter().enumerate() {
            row_themes.push(*theme);
            if row_themes.len() == columns || i == themes.len() - 1 {
                ui.horizontal(|ui| {
                    for theme in &row_themes {
                        self.render_theme_card(ui, theme, manager);
                    }
                });
                row_themes.clear();
            }
        }
    }

    fn render_list_view(
        &mut self,
        ui: &mut egui::Ui,
        themes: &[&TerminalTheme],
        manager: &mut ThemeManager,
    ) {
        for theme in themes {
            ui.horizontal(|ui| {
                // Preview swatch
                let (rect, response) =
                    ui.allocate_exact_size(egui::Vec2::new(60.0, 40.0), egui::Sense::click());
                ui.painter()
                    .rect_filled(rect, 4.0, theme.palette.background);
                ui.painter().rect_stroke(
                    rect,
                    4.0,
                    egui::Stroke::new(2.0, theme.palette.foreground),
                );

                // Sample text in theme colors
                ui.painter().text(
                    rect.min + egui::Vec2::new(5.0, 5.0),
                    egui::Align2::LEFT_TOP,
                    "Aa",
                    FontId::new(14.0, FontFamily::Monospace),
                    theme.palette.foreground,
                );

                if response.clicked() {
                    self.selected_theme_id = theme.id.clone();
                    manager.set_theme(&theme.id);
                }

                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(&theme.name).strong());
                    ui.label(egui::RichText::new(format!("by {}", theme.author)).small());
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Favorite button
                    let is_fav = manager.is_favorite(&theme.id);
                    let fav_text = if is_fav { "★" } else { "☆" };
                    if ui.button(fav_text).clicked() {
                        manager.toggle_favorite(&theme.id);
                    }

                    if ui.button("Apply").clicked() {
                        self.selected_theme_id = theme.id.clone();
                        manager.set_theme(&theme.id);
                    }
                });
            });
            ui.separator();
        }
    }

    fn render_theme_card(
        &mut self,
        ui: &mut egui::Ui,
        theme: &TerminalTheme,
        manager: &mut ThemeManager,
    ) {
        let card_width = 180.0;
        let card_height = 140.0;

        let card_response = egui::Frame::group(ui.style())
            .fill(theme.palette.background)
            .stroke(egui::Stroke::new(
                if self.selected_theme_id == theme.id {
                    3.0
                } else {
                    1.0
                },
                if self.selected_theme_id == theme.id {
                    egui::Color32::from_rgb(64, 156, 255)
                } else {
                    theme.palette.foreground
                },
            ))
            .rounding(Rounding::same(8.0))
            .show(ui, |ui| {
                ui.set_min_size(egui::Vec2::new(card_width, card_height));

                ui.vertical(|ui| {
                    // Color preview strip
                    ui.horizontal(|ui| {
                        let swatch_size = egui::Vec2::new(18.0, 18.0);
                        let colors = [
                            theme.palette.red,
                            theme.palette.green,
                            theme.palette.yellow,
                            theme.palette.blue,
                            theme.palette.magenta,
                            theme.palette.cyan,
                        ];
                        for color in colors {
                            let (rect, _) =
                                ui.allocate_exact_size(swatch_size, egui::Sense::hover());
                            ui.painter_at(rect).rect_filled(rect, 3.0, color);
                        }
                    });

                    ui.add_space(10.0);

                    // Theme name
                    ui.label(
                        egui::RichText::new(&theme.name)
                            .color(theme.palette.foreground)
                            .strong(),
                    );

                    ui.label(
                        egui::RichText::new(format!("by {}", theme.author))
                            .color(theme.palette.foreground.linear_multiply(0.7))
                            .small(),
                    );

                    ui.add_space(10.0);

                    // Sample text
                    ui.label(
                        egui::RichText::new("Hello, World!")
                            .color(theme.palette.foreground)
                            .font(FontId::new(12.0, FontFamily::Monospace)),
                    );
                });
            });

        if card_response.response.clicked() {
            self.selected_theme_id = theme.id.clone();
            manager.set_theme(&theme.id);
        }

        // Context menu
        card_response.response.context_menu(|ui| {
            if ui.button("Apply Theme").clicked() {
                manager.set_theme(&theme.id);
                ui.close_menu();
            }

            if ui.button("Edit Theme").clicked() {
                // Open editor
                ui.close_menu();
            }

            let is_fav = manager.is_favorite(&theme.id);
            if ui
                .button(if is_fav {
                    "Remove from Favorites"
                } else {
                    "Add to Favorites"
                })
                .clicked()
            {
                manager.toggle_favorite(&theme.id);
                ui.close_menu();
            }

            if manager.custom_themes.contains_key(&theme.id) && ui.button("Delete").clicked() {
                manager.delete_custom_theme(&theme.id);
                ui.close_menu();
            }

            if ui.button("Export...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_file_name(format!("{}.json", theme.id))
                    .save_file()
                {
                    let _ = manager.export_theme(&theme.id, &path, ExportFormat::Native);
                }
                ui.close_menu();
            }
        });
    }
}

// ============================================================================
// SETTINGS INTEGRATION
// ============================================================================

/// Extension to integrate themes into the existing Settings panel
pub fn render_theme_settings(
    ui: &mut egui::Ui,
    manager: &mut ThemeManager,
    gallery: &mut ThemeGallery,
    editor: &mut ThemeEditor,
) {
    ui.heading("Terminal Theme");
    ui.add_space(15.0);

    // Current theme display
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label("Current Theme:");
            ui.label(egui::RichText::new(&manager.current_theme.name).strong());

            let palette = &manager.current_theme.palette;
            let (rect, _) =
                ui.allocate_exact_size(egui::Vec2::new(60.0, 30.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 4.0, palette.background);
            ui.painter()
                .rect_stroke(rect, 4.0, egui::Stroke::new(1.0, palette.foreground));
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("🎨 Browse Themes...").clicked() {
                gallery.open();
            }

            if ui.button("✏ Edit Current Theme").clicked() {
                editor.open(&manager.current_theme);
            }
        });
    });

    ui.add_space(20.0);

    // Dynamic theme switching
    ui.heading("Dynamic Theme");
    ui.add_space(10.0);

    ui.group(|ui| {
        ui.checkbox(
            &mut manager.dynamic_config.enabled,
            "Enable automatic theme switching",
        );

        if manager.dynamic_config.enabled {
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Day Theme (7:00-19:00):");
                ui.label(manager.dynamic_config.day_theme_id.clone());
            });

            ui.horizontal(|ui| {
                ui.label("Night Theme (19:00-7:00):");
                ui.label(manager.dynamic_config.night_theme_id.clone());
            });

            ui.add_space(10.0);

            ui.checkbox(
                &mut manager.dynamic_config.use_system_theme,
                "Follow Windows theme preference",
            );
        }
    });

    ui.add_space(20.0);

    // Import/Export
    ui.heading("Import & Export");
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        if ui.button("📥 Import Theme...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Theme files", &["json"])
                .pick_file()
            {
                let _ = manager.import_theme(&path);
            }
        }

        if ui.button("📤 Export Current Theme...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name(format!("{}.json", manager.current_theme.id))
                .save_file()
            {
                let _ =
                    manager.export_theme(&manager.current_theme.id, &path, ExportFormat::Native);
            }
        }

        if ui.button("📋 Export as VS Code Theme...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name(format!("{}-vscode.json", manager.current_theme.id))
                .save_file()
            {
                let _ =
                    manager.export_theme(&manager.current_theme.id, &path, ExportFormat::VSCode);
            }
        }
    });

    ui.add_space(20.0);

    // Community themes
    ui.heading("Community Themes");
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        if ui.button("🌐 Browse Community Store").clicked() {
            // Open community store
        }

        ui.label("Browse and download themes shared by the community");
    });
}
