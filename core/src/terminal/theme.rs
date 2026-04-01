//! 终端主题系统
//! 提供完整的终端配色、光标样式和字体配置

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 终端主题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalTheme {
    /// 主题名称
    pub name: String,
    /// 主题描述
    pub description: String,
    /// 是否为深色主题
    pub is_dark: bool,
    /// 基础16色
    pub palette: ColorPalette,
    /// 光标样式
    pub cursor: CursorConfig,
    /// 字体配置
    pub font: FontConfig,
    /// 背景透明度 (0.0 - 1.0)
    pub background_opacity: f32,
    /// 选择高亮颜色
    pub selection_bg: u32,
    pub selection_fg: u32,
    /// 粗体是否为亮色
    pub bold_is_bright: bool,
    /// 是否使用自定义光标颜色
    pub use_custom_cursor_color: bool,
    pub custom_cursor_color: u32,
}

/// 16色配色方案
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ColorPalette {
    // 基本16色 (ANSI colors 0-15)
    pub black: u32,
    pub red: u32,
    pub green: u32,
    pub yellow: u32,
    pub blue: u32,
    pub magenta: u32,
    pub cyan: u32,
    pub white: u32,
    // 亮色版本
    pub bright_black: u32,
    pub bright_red: u32,
    pub bright_green: u32,
    pub bright_yellow: u32,
    pub bright_blue: u32,
    pub bright_magenta: u32,
    pub bright_cyan: u32,
    pub bright_white: u32,
    // 前景/背景
    pub foreground: u32,
    pub background: u32,
}

impl Default for ColorPalette {
    fn default() -> Self {
        // 默认使用 Dracula 配色
        Self::dracula()
    }
}

impl ColorPalette {
    /// Dracula 配色方案
    pub fn dracula() -> Self {
        Self {
            black: 0x21222C,
            red: 0xFF5555,
            green: 0x50FA7B,
            yellow: 0xF1FA8C,
            blue: 0xBD93F9,
            magenta: 0xFF79C6,
            cyan: 0x8BE9FD,
            white: 0xF8F8F2,
            bright_black: 0x6272A4,
            bright_red: 0xFF6E6E,
            bright_green: 0x69FF94,
            bright_yellow: 0xFFFFA5,
            bright_blue: 0xD6ACFF,
            bright_magenta: 0xFF92DF,
            bright_cyan: 0xA4FFFF,
            bright_white: 0xFFFFFF,
            foreground: 0xF8F8F2,
            background: 0x282A36,
        }
    }

    /// 暗色主题 (One Dark)
    pub fn one_dark() -> Self {
        Self {
            black: 0x282C34,
            red: 0xE06C75,
            green: 0x98C379,
            yellow: 0xE5C07B,
            blue: 0x61AFEF,
            magenta: 0xC678DD,
            cyan: 0x56B6C2,
            white: 0xABB2BF,
            bright_black: 0x5C6370,
            bright_red: 0xE06C75,
            bright_green: 0x98C379,
            bright_yellow: 0xE5C07B,
            bright_blue: 0x61AFEF,
            bright_magenta: 0xC678DD,
            bright_cyan: 0x56B6C2,
            bright_white: 0xFFFFFF,
            foreground: 0xABB2BF,
            background: 0x282C34,
        }
    }

    /// 暗色主题 (Monokai)
    pub fn monokai() -> Self {
        Self {
            black: 0x272822,
            red: 0xF92672,
            green: 0xA6E22E,
            yellow: 0xF4BF75,
            blue: 0x66D9EF,
            magenta: 0xAE81FF,
            cyan: 0xA1EFE4,
            white: 0xF8F8F2,
            bright_black: 0x75715E,
            bright_red: 0xF92672,
            bright_green: 0xA6E22E,
            bright_yellow: 0xF4BF75,
            bright_blue: 0x66D9EF,
            bright_magenta: 0xAE81FF,
            bright_cyan: 0xA1EFE4,
            bright_white: 0xF9F8F5,
            foreground: 0xF8F8F2,
            background: 0x272822,
        }
    }

    /// 暗色主题 (Solarized Dark)
    pub fn solarized_dark() -> Self {
        Self {
            black: 0x002B36,
            red: 0xDC322F,
            green: 0x859900,
            yellow: 0xB58900,
            blue: 0x268BD2,
            magenta: 0xD33682,
            cyan: 0x2AA198,
            white: 0xEEE8D5,
            bright_black: 0x073642,
            bright_red: 0xCB4B16,
            bright_green: 0x586E75,
            bright_yellow: 0x657B83,
            bright_blue: 0x839496,
            bright_magenta: 0x6C71C4,
            bright_cyan: 0x93A1A1,
            bright_white: 0xFDF6E3,
            foreground: 0x839496,
            background: 0x002B36,
        }
    }

    /// 亮色主题 (Solarized Light)
    pub fn solarized_light() -> Self {
        Self {
            black: 0xEEE8D5,
            red: 0xDC322F,
            green: 0x859900,
            yellow: 0xB58900,
            blue: 0x268BD2,
            magenta: 0xD33682,
            cyan: 0x2AA198,
            white: 0x002B36,
            bright_black: 0x93A1A1,
            bright_red: 0xCB4B16,
            bright_green: 0x586E75,
            bright_yellow: 0x657B83,
            bright_blue: 0x839496,
            bright_magenta: 0x6C71C4,
            bright_cyan: 0x073642,
            bright_white: 0xFDF6E3,
            foreground: 0x657B83,
            background: 0xFDF6E3,
        }
    }

    /// 亮色主题 (GitHub Light)
    pub fn github_light() -> Self {
        Self {
            black: 0x24292E,
            red: 0xCF222E,
            green: 0x116329,
            yellow: 0x4D2D00,
            blue: 0x0969DA,
            magenta: 0x8250DF,
            cyan: 0x1B7C83,
            white: 0x6E7781,
            bright_black: 0x57606A,
            bright_red: 0xA40E26,
            bright_green: 0x1A7F37,
            bright_yellow: 0x633C01,
            bright_blue: 0x218BFF,
            bright_magenta: 0xA475F9,
            bright_cyan: 0x3192AA,
            bright_white: 0x8C959F,
            foreground: 0x24292F,
            background: 0xFFFFFF,
        }
    }

    /// 获取256色支持的颜色
    pub fn get_256_color(&self, index: u8) -> u32 {
        if index < 16 {
            // 使用16色
            self.get_ansi_color(index)
        } else if index < 232 {
            // 216色立方
            let r = ((index - 16) / 36) as u32;
            let g = (((index - 16) % 36) / 6) as u32;
            let b = ((index - 16) % 6) as u32;

            let r = if r == 0 { 0 } else { r * 40 + 55 };
            let g = if g == 0 { 0 } else { g * 40 + 55 };
            let b = if b == 0 { 0 } else { b * 40 + 55 };

            (r << 16) | (g << 8) | b
        } else {
            // 24级灰度
            let gray = 8 + (index - 232) as u32 * 10;
            (gray << 16) | (gray << 8) | gray
        }
    }

    /// 获取ANSI颜色
    fn get_ansi_color(&self, index: u8) -> u32 {
        match index {
            0 => self.black,
            1 => self.red,
            2 => self.green,
            3 => self.yellow,
            4 => self.blue,
            5 => self.magenta,
            6 => self.cyan,
            7 => self.white,
            8 => self.bright_black,
            9 => self.bright_red,
            10 => self.bright_green,
            11 => self.bright_yellow,
            12 => self.bright_blue,
            13 => self.bright_magenta,
            14 => self.bright_cyan,
            15 => self.bright_white,
            _ => self.foreground,
        }
    }

    /// 获取真彩色 (24-bit RGB)
    pub fn get_true_color(r: u8, g: u8, b: u8) -> u32 {
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
}

/// 光标配置
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CursorConfig {
    pub style: CursorStyle,
    pub blink: bool,
    pub blink_interval_ms: u64,
    pub color: Option<u32>, // None = 使用前景色
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            style: CursorStyle::Block,
            blink: true,
            blink_interval_ms: 500,
            color: None,
        }
    }
}

/// 光标样式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

impl CursorStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            CursorStyle::Block => "block",
            CursorStyle::Underline => "underline",
            CursorStyle::Bar => "bar",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "block" => Some(CursorStyle::Block),
            "underline" => Some(CursorStyle::Underline),
            "bar" => Some(CursorStyle::Bar),
            _ => None,
        }
    }
}

/// 字体配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    pub family: String,
    pub size: f32,
    pub line_height: f32,
    pub weight: FontWeight,
    pub ligatures: bool,
    pub fallback_families: Vec<String>,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "JetBrains Mono".to_string(),
            size: 14.0,
            line_height: 1.2,
            weight: FontWeight::Normal,
            ligatures: true,
            fallback_families: vec![
                "Fira Code".to_string(),
                "Source Code Pro".to_string(),
                "Consolas".to_string(),
                "monospace".to_string(),
            ],
        }
    }
}

/// 字重
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

impl FontWeight {
    pub fn to_css(&self) -> &'static str {
        match self {
            FontWeight::Thin => "100",
            FontWeight::ExtraLight => "200",
            FontWeight::Light => "300",
            FontWeight::Normal => "400",
            FontWeight::Medium => "500",
            FontWeight::SemiBold => "600",
            FontWeight::Bold => "700",
            FontWeight::ExtraBold => "800",
            FontWeight::Black => "900",
        }
    }
}

impl Default for TerminalTheme {
    fn default() -> Self {
        Self::dracula()
    }
}

impl TerminalTheme {
    /// 创建 Dracula 主题
    pub fn dracula() -> Self {
        Self {
            name: "Dracula".to_string(),
            description: "A dark theme for hackers".to_string(),
            is_dark: true,
            palette: ColorPalette::dracula(),
            cursor: CursorConfig::default(),
            font: FontConfig::default(),
            background_opacity: 1.0,
            selection_bg: 0x44475A,
            selection_fg: 0xF8F8F2,
            bold_is_bright: true,
            use_custom_cursor_color: false,
            custom_cursor_color: 0xF8F8F2,
        }
    }

    /// 创建 One Dark 主题
    pub fn one_dark() -> Self {
        Self {
            name: "One Dark".to_string(),
            description: "Atom One Dark theme".to_string(),
            is_dark: true,
            palette: ColorPalette::one_dark(),
            cursor: CursorConfig::default(),
            font: FontConfig::default(),
            background_opacity: 1.0,
            selection_bg: 0x3E4451,
            selection_fg: 0xABB2BF,
            bold_is_bright: true,
            use_custom_cursor_color: false,
            custom_cursor_color: 0x528BFF,
        }
    }

    /// 创建 Monokai 主题
    pub fn monokai() -> Self {
        Self {
            name: "Monokai".to_string(),
            description: "Classic Monokai theme".to_string(),
            is_dark: true,
            palette: ColorPalette::monokai(),
            cursor: CursorConfig {
                style: CursorStyle::Block,
                blink: false,
                blink_interval_ms: 0,
                color: Some(0xF8F8F0),
            },
            font: FontConfig::default(),
            background_opacity: 1.0,
            selection_bg: 0x49483E,
            selection_fg: 0xF8F8F2,
            bold_is_bright: true,
            use_custom_cursor_color: true,
            custom_cursor_color: 0xF8F8F0,
        }
    }

    /// 创建 Solarized Dark 主题
    pub fn solarized_dark() -> Self {
        Self {
            name: "Solarized Dark".to_string(),
            description: "Solarized dark variant".to_string(),
            is_dark: true,
            palette: ColorPalette::solarized_dark(),
            cursor: CursorConfig::default(),
            font: FontConfig::default(),
            background_opacity: 1.0,
            selection_bg: 0x073642,
            selection_fg: 0x93A1A1,
            bold_is_bright: false,
            use_custom_cursor_color: false,
            custom_cursor_color: 0x93A1A1,
        }
    }

    /// 创建 Solarized Light 主题
    pub fn solarized_light() -> Self {
        Self {
            name: "Solarized Light".to_string(),
            description: "Solarized light variant".to_string(),
            is_dark: false,
            palette: ColorPalette::solarized_light(),
            cursor: CursorConfig::default(),
            font: FontConfig::default(),
            background_opacity: 1.0,
            selection_bg: 0xEEE8D5,
            selection_fg: 0x586E75,
            bold_is_bright: false,
            use_custom_cursor_color: false,
            custom_cursor_color: 0x586E75,
        }
    }

    /// 创建 GitHub Light 主题
    pub fn github_light() -> Self {
        Self {
            name: "GitHub Light".to_string(),
            description: "GitHub-inspired light theme".to_string(),
            is_dark: false,
            palette: ColorPalette::github_light(),
            cursor: CursorConfig {
                style: CursorStyle::Bar,
                blink: true,
                blink_interval_ms: 530,
                color: None,
            },
            font: FontConfig::default(),
            background_opacity: 1.0,
            selection_bg: 0xDDF4FF,
            selection_fg: 0x24292F,
            bold_is_bright: false,
            use_custom_cursor_color: false,
            custom_cursor_color: 0x24292F,
        }
    }

    /// 获取CSS颜色字符串
    pub fn get_css_color(color: u32) -> String {
        let r = ((color >> 16) & 0xFF) as u8;
        let g = ((color >> 8) & 0xFF) as u8;
        let b = (color & 0xFF) as u8;
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }

    /// 获取带透明度的CSS颜色
    pub fn get_css_color_with_alpha(color: u32, alpha: f32) -> String {
        let r = ((color >> 16) & 0xFF) as u8;
        let g = ((color >> 8) & 0xFF) as u8;
        let b = (color & 0xFF) as u8;
        format!("rgba({}, {}, {}, {})", r, g, b, alpha)
    }

    /// 转换为 CSS 变量映射
    pub fn to_css_variables(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        vars.insert(
            "--terminal-bg".to_string(),
            Self::get_css_color_with_alpha(self.palette.background, self.background_opacity),
        );
        vars.insert(
            "--terminal-fg".to_string(),
            Self::get_css_color(self.palette.foreground),
        );

        vars.insert(
            "--terminal-black".to_string(),
            Self::get_css_color(self.palette.black),
        );
        vars.insert(
            "--terminal-red".to_string(),
            Self::get_css_color(self.palette.red),
        );
        vars.insert(
            "--terminal-green".to_string(),
            Self::get_css_color(self.palette.green),
        );
        vars.insert(
            "--terminal-yellow".to_string(),
            Self::get_css_color(self.palette.yellow),
        );
        vars.insert(
            "--terminal-blue".to_string(),
            Self::get_css_color(self.palette.blue),
        );
        vars.insert(
            "--terminal-magenta".to_string(),
            Self::get_css_color(self.palette.magenta),
        );
        vars.insert(
            "--terminal-cyan".to_string(),
            Self::get_css_color(self.palette.cyan),
        );
        vars.insert(
            "--terminal-white".to_string(),
            Self::get_css_color(self.palette.white),
        );

        vars.insert(
            "--terminal-bright-black".to_string(),
            Self::get_css_color(self.palette.bright_black),
        );
        vars.insert(
            "--terminal-bright-red".to_string(),
            Self::get_css_color(self.palette.bright_red),
        );
        vars.insert(
            "--terminal-bright-green".to_string(),
            Self::get_css_color(self.palette.bright_green),
        );
        vars.insert(
            "--terminal-bright-yellow".to_string(),
            Self::get_css_color(self.palette.bright_yellow),
        );
        vars.insert(
            "--terminal-bright-blue".to_string(),
            Self::get_css_color(self.palette.bright_blue),
        );
        vars.insert(
            "--terminal-bright-magenta".to_string(),
            Self::get_css_color(self.palette.bright_magenta),
        );
        vars.insert(
            "--terminal-bright-cyan".to_string(),
            Self::get_css_color(self.palette.bright_cyan),
        );
        vars.insert(
            "--terminal-bright-white".to_string(),
            Self::get_css_color(self.palette.bright_white),
        );

        vars.insert(
            "--terminal-selection-bg".to_string(),
            Self::get_css_color(self.selection_bg),
        );
        vars.insert(
            "--terminal-selection-fg".to_string(),
            Self::get_css_color(self.selection_fg),
        );

        vars.insert(
            "--terminal-cursor".to_string(),
            Self::get_css_color(self.cursor.color.unwrap_or(self.palette.foreground)),
        );
        vars.insert(
            "--terminal-font-family".to_string(),
            self.font.family.clone(),
        );
        vars.insert(
            "--terminal-font-size".to_string(),
            format!("{}px", self.font.size),
        );

        vars
    }
}

/// 主题管理器
pub struct ThemeManager {
    themes: Arc<RwLock<HashMap<String, TerminalTheme>>>,
    current: Arc<RwLock<String>>,
    custom_themes: Arc<RwLock<HashMap<String, TerminalTheme>>>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let mut themes = HashMap::new();

        // 注册内置主题
        let dracula = TerminalTheme::dracula();
        themes.insert(dracula.name.clone(), dracula);

        let one_dark = TerminalTheme::one_dark();
        themes.insert(one_dark.name.clone(), one_dark);

        let monokai = TerminalTheme::monokai();
        themes.insert(monokai.name.clone(), monokai);

        let solarized_dark = TerminalTheme::solarized_dark();
        themes.insert(solarized_dark.name.clone(), solarized_dark);

        let solarized_light = TerminalTheme::solarized_light();
        themes.insert(solarized_light.name.clone(), solarized_light);

        let github_light = TerminalTheme::github_light();
        themes.insert(github_light.name.clone(), github_light);

        Self {
            themes: Arc::new(RwLock::new(themes)),
            current: Arc::new(RwLock::new("Dracula".to_string())),
            custom_themes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取所有可用主题名称
    pub async fn list_themes(&self) -> Vec<String> {
        let themes = self.themes.read().await;
        let custom = self.custom_themes.read().await;

        let mut all: Vec<String> = themes.keys().cloned().collect();
        all.extend(custom.keys().cloned());
        all.sort();
        all.dedup();
        all
    }

    /// 获取当前主题
    pub async fn current_theme(&self) -> TerminalTheme {
        let current_name = self.current.read().await.clone();

        // 先查内置主题
        let themes = self.themes.read().await;
        if let Some(theme) = themes.get(&current_name) {
            return theme.clone();
        }

        // 再查自定义主题
        let custom = self.custom_themes.read().await;
        if let Some(theme) = custom.get(&current_name) {
            return theme.clone();
        }

        // 回退到默认
        TerminalTheme::default()
    }

    /// 设置当前主题
    pub async fn set_theme(&self, name: &str) -> Result<(), String> {
        let themes = self.themes.read().await;
        if themes.contains_key(name) {
            let mut current = self.current.write().await;
            *current = name.to_string();
            return Ok(());
        }

        let custom = self.custom_themes.read().await;
        if custom.contains_key(name) {
            let mut current = self.current.write().await;
            *current = name.to_string();
            return Ok(());
        }

        Err(format!("Theme '{}' not found", name))
    }

    /// 获取指定主题
    pub async fn get_theme(&self, name: &str) -> Option<TerminalTheme> {
        let themes = self.themes.read().await;
        if let Some(theme) = themes.get(name) {
            return Some(theme.clone());
        }

        let custom = self.custom_themes.read().await;
        custom.get(name).cloned()
    }

    /// 添加自定义主题
    pub async fn add_custom_theme(&self, theme: TerminalTheme) {
        let mut custom = self.custom_themes.write().await;
        custom.insert(theme.name.clone(), theme);
    }

    /// 删除自定义主题
    pub async fn remove_custom_theme(&self, name: &str) -> Result<(), String> {
        // 不能删除内置主题
        let themes = self.themes.read().await;
        if themes.contains_key(name) {
            return Err(format!("Cannot delete built-in theme '{}'", name));
        }

        let mut custom = self.custom_themes.write().await;
        if custom.remove(name).is_none() {
            return Err(format!("Theme '{}' not found", name));
        }

        // 如果当前主题被删除，切换到默认
        let current_name = self.current.read().await.clone();
        if current_name == name {
            let mut current = self.current.write().await;
            *current = "Dracula".to_string();
        }

        Ok(())
    }

    /// 更新主题字体配置
    pub async fn update_font(&self, font_config: FontConfig) -> Result<(), String> {
        let current_name = self.current.read().await.clone();

        let mut custom = self.custom_themes.write().await;

        // 如果当前是内置主题，创建自定义副本
        let themes = self.themes.read().await;
        if let Some(theme) = themes.get(&current_name) {
            let mut new_theme = theme.clone();
            new_theme.name = format!("{} Custom", current_name);
            new_theme.font = font_config;
            custom.insert(new_theme.name.clone(), new_theme);

            drop(current_name);
            let mut current = self.current.write().await;
            *current = theme.name.clone();
            return Ok(());
        }

        // 更新已有自定义主题
        if let Some(theme) = custom.get_mut(&current_name) {
            theme.font = font_config;
            return Ok(());
        }

        Err("Failed to update font config".to_string())
    }

    /// 导出主题到JSON
    pub async fn export_theme(&self, name: &str) -> Result<String, String> {
        let theme = self
            .get_theme(name)
            .await
            .ok_or_else(|| format!("Theme '{}' not found", name))?;

        serde_json::to_string_pretty(&theme)
            .map_err(|e| format!("Failed to serialize theme: {}", e))
    }

    /// 从JSON导入主题
    pub async fn import_theme(&self, json: &str) -> Result<TerminalTheme, String> {
        let theme: TerminalTheme =
            serde_json::from_str(json).map_err(|e| format!("Failed to parse theme: {}", e))?;

        self.add_custom_theme(theme.clone()).await;
        Ok(theme)
    }

    /// 根据系统偏好自动选择主题
    pub async fn auto_select_theme(&self, prefers_dark: bool) {
        let name = if prefers_dark {
            "Dracula"
        } else {
            "Solarized Light"
        };
        let _ = self.set_theme(name).await;
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}
