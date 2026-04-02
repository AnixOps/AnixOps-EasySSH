//! Configuration Types
//!
//! Defines all configuration data structures including:
//! - AppConfig: Application-level settings
//! - UserPreferences: User-specific preferences
//! - SecuritySettings: Security-related configurations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Theme setting for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    #[default]
    System,
    Dark,
}

impl Theme {
    /// Get all available themes
    pub fn all() -> &'static [Theme] {
        &[Theme::Light, Theme::System, Theme::Dark]
    }

    /// Get display name for the theme
    pub fn display_name(&self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::System => "System",
            Theme::Dark => "Dark",
        }
    }
}

/// Language setting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[serde(rename = "zh")]
    Chinese,
    #[default]
    #[serde(rename = "en")]
    English,
}

impl Language {
    /// Get all available languages
    pub fn all() -> &'static [Language] {
        &[Language::Chinese, Language::English]
    }

    /// Get language code
    pub fn code(&self) -> &'static str {
        match self {
            Language::Chinese => "zh",
            Language::English => "en",
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::Chinese => "中文",
            Language::English => "English",
        }
    }
}

/// Window position and size
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 1280,
            height: 720,
            maximized: false,
        }
    }
}

/// Keyboard shortcut definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shortcut {
    pub key: String,
    pub modifiers: Vec<String>,
    pub description: String,
}

impl Shortcut {
    /// Create a new shortcut
    pub fn new(key: &str, modifiers: &[&str], description: &str) -> Self {
        Self {
            key: key.to_string(),
            modifiers: modifiers.iter().map(|s| s.to_string()).collect(),
            description: description.to_string(),
        }
    }
}

/// Default shortcuts configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutsConfig {
    pub new_connection: Shortcut,
    pub close_connection: Shortcut,
    pub next_tab: Shortcut,
    pub previous_tab: Shortcut,
    pub search: Shortcut,
    pub settings: Shortcut,
    pub quit: Shortcut,
}

impl Default for ShortcutsConfig {
    fn default() -> Self {
        Self {
            new_connection: Shortcut::new("N", &["Ctrl"], "New Connection"),
            close_connection: Shortcut::new("W", &["Ctrl"], "Close Connection"),
            next_tab: Shortcut::new("Tab", &["Ctrl"], "Next Tab"),
            previous_tab: Shortcut::new("Tab", &["Ctrl", "Shift"], "Previous Tab"),
            search: Shortcut::new("F", &["Ctrl"], "Search"),
            settings: Shortcut::new(",", &["Ctrl"], "Settings"),
            quit: Shortcut::new("Q", &["Ctrl"], "Quit"),
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application theme
    #[serde(default)]
    pub theme: Theme,

    /// Interface language
    #[serde(default)]
    pub language: Language,

    /// Default terminal emulator to use (Lite version)
    #[serde(default = "default_terminal")]
    pub default_terminal: String,

    /// Keyboard shortcuts
    #[serde(default)]
    pub shortcuts: ShortcutsConfig,

    /// Window geometry (last known position and size)
    #[serde(default)]
    pub window_geometry: WindowGeometry,

    /// Whether to restore window geometry on startup
    #[serde(default = "default_true")]
    pub restore_window_geometry: bool,

    /// Show sidebar by default
    #[serde(default = "default_true")]
    pub show_sidebar: bool,

    /// Sidebar width in pixels
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u32,

    /// Custom application settings (extensibility)
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            language: Language::default(),
            default_terminal: default_terminal(),
            shortcuts: ShortcutsConfig::default(),
            window_geometry: WindowGeometry::default(),
            restore_window_geometry: true,
            show_sidebar: true,
            sidebar_width: 250,
            custom: HashMap::new(),
        }
    }
}

fn default_terminal() -> String {
    #[cfg(target_os = "windows")]
    {
        "powershell.exe".to_string()
    }
    #[cfg(target_os = "macos")]
    {
        "Terminal.app".to_string()
    }
    #[cfg(target_os = "linux")]
    {
        "gnome-terminal".to_string()
    }
}

fn default_true() -> bool {
    true
}

fn default_sidebar_width() -> u32 {
    250
}

/// User preferences for connections
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Default username for new connections
    #[serde(default)]
    pub default_username: String,

    /// Default port for new connections
    #[serde(default = "default_port")]
    pub default_port: u16,

    /// Default SSH key path
    #[serde(default)]
    pub default_key_path: Option<String>,

    /// Search history (last 100 searches)
    #[serde(default)]
    pub search_history: Vec<String>,

    /// Maximum search history entries
    #[serde(default = "default_max_search_history")]
    pub max_search_history: usize,

    /// Recent connections (server IDs, last 20)
    #[serde(default)]
    pub recent_connections: Vec<String>,

    /// Maximum recent connections to keep
    #[serde(default = "default_max_recent_connections")]
    pub max_recent_connections: usize,

    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u64,

    /// Keepalive interval in seconds (0 to disable)
    #[serde(default = "default_keepalive_interval")]
    pub keepalive_interval: u64,

    /// Default connection group
    #[serde(default)]
    pub default_group: Option<String>,

    /// Auto-save new connections
    #[serde(default = "default_true")]
    pub auto_save_connections: bool,

    /// Confirm before closing active connections
    #[serde(default = "default_true")]
    pub confirm_close_active: bool,

    /// Show connection notifications
    #[serde(default = "default_true")]
    pub show_notifications: bool,

    /// Enable sound effects
    #[serde(default)]
    pub sound_effects: bool,

    /// Custom preferences (extensibility)
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            default_username: String::new(),
            default_port: 22,
            default_key_path: None,
            search_history: Vec::new(),
            max_search_history: 100,
            recent_connections: Vec::new(),
            max_recent_connections: 20,
            connection_timeout: 30,
            keepalive_interval: 60,
            default_group: None,
            auto_save_connections: true,
            confirm_close_active: true,
            show_notifications: true,
            sound_effects: false,
            custom: HashMap::new(),
        }
    }
}

fn default_port() -> u16 {
    22
}

fn default_max_search_history() -> usize {
    100
}

fn default_max_recent_connections() -> usize {
    20
}

fn default_connection_timeout() -> u64 {
    30
}

fn default_keepalive_interval() -> u64 {
    60
}

/// Security settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecuritySettings {
    /// Master password timeout in minutes (0 = never timeout)
    #[serde(default)]
    pub master_password_timeout: u32,

    /// Auto-lock after idle time in minutes (0 = disable)
    #[serde(default)]
    pub auto_lock_after_idle: u32,

    /// Clear clipboard after seconds (0 = disable)
    #[serde(default = "default_clipboard_clear_time")]
    pub clipboard_clear_time: u32,

    /// Lock on sleep/screen lock
    #[serde(default = "default_true")]
    pub lock_on_sleep: bool,

    /// Lock on window blur (lose focus)
    #[serde(default)]
    pub lock_on_blur: bool,

    /// Require password on startup
    #[serde(default)]
    pub require_password_on_startup: bool,

    /// Enable biometric authentication (if available)
    #[serde(default = "default_true")]
    pub enable_biometric: bool,

    /// Enable hardware security key (YubiKey, etc.)
    #[serde(default)]
    pub enable_hardware_key: bool,

    /// SSH agent forwarding enabled by default
    #[serde(default)]
    pub ssh_agent_forwarding: bool,

    /// Verify host keys strictly
    #[serde(default = "default_true")]
    pub strict_host_key_checking: bool,

    /// Log sensitive operations
    #[serde(default = "default_true")]
    pub audit_sensitive_ops: bool,

    /// Encrypt configuration backups
    #[serde(default = "default_true")]
    pub encrypt_backups: bool,

    /// Minimum password length (for local encryption)
    #[serde(default = "default_min_password_length")]
    pub min_password_length: u8,

    /// Password complexity requirements
    #[serde(default)]
    pub require_password_complexity: bool,

    /// Custom security settings (extensibility)
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            master_password_timeout: 0,
            auto_lock_after_idle: 0,
            clipboard_clear_time: 30,
            lock_on_sleep: true,
            lock_on_blur: false,
            require_password_on_startup: false,
            enable_biometric: true,
            enable_hardware_key: false,
            ssh_agent_forwarding: false,
            strict_host_key_checking: true,
            audit_sensitive_ops: true,
            encrypt_backups: true,
            min_password_length: 8,
            require_password_complexity: false,
            custom: HashMap::new(),
        }
    }
}

fn default_clipboard_clear_time() -> u32 {
    30
}

fn default_min_password_length() -> u8 {
    8
}

/// Complete configuration container
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FullConfig {
    /// Configuration format version for migration
    #[serde(default = "current_config_version")]
    pub version: u32,

    /// Application configuration
    #[serde(default)]
    pub app_config: AppConfig,

    /// User preferences
    #[serde(default)]
    pub user_preferences: UserPreferences,

    /// Security settings
    #[serde(default)]
    pub security_settings: SecuritySettings,
}

impl Default for FullConfig {
    fn default() -> Self {
        Self {
            version: current_config_version(),
            app_config: AppConfig::default(),
            user_preferences: UserPreferences::default(),
            security_settings: SecuritySettings::default(),
        }
    }
}

fn current_config_version() -> u32 {
    1
}

/// Configuration sections for partial updates
#[derive(Debug, Clone)]
pub enum ConfigSection {
    AppConfig(AppConfig),
    UserPreferences(UserPreferences),
    SecuritySettings(SecuritySettings),
}

/// Platform-specific configuration paths
#[derive(Debug, Clone)]
pub struct ConfigPaths;

impl ConfigPaths {
    /// Get the base configuration directory
    pub fn config_dir() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|p| p.join("EasySSH"))
    }

    /// Get the full path to the config file
    pub fn config_file() -> Option<std::path::PathBuf> {
        Self::config_dir().map(|p| p.join("config.json"))
    }

    /// Get the backup config directory
    pub fn backup_dir() -> Option<std::path::PathBuf> {
        Self::config_dir().map(|p| p.join("backups"))
    }

    /// Ensure config directory exists
    pub fn ensure_config_dir() -> std::io::Result<std::path::PathBuf> {
        let dir = Self::config_dir().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Could not determine config directory")
        })?;
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_display_name() {
        assert_eq!(Theme::Light.display_name(), "Light");
        assert_eq!(Theme::Dark.display_name(), "Dark");
        assert_eq!(Theme::System.display_name(), "System");
    }

    #[test]
    fn test_language_code() {
        assert_eq!(Language::Chinese.code(), "zh");
        assert_eq!(Language::English.code(), "en");
    }

    #[test]
    fn test_window_geometry_default() {
        let geom = WindowGeometry::default();
        assert_eq!(geom.width, 1280);
        assert_eq!(geom.height, 720);
        assert!(!geom.maximized);
    }

    #[test]
    fn test_user_preferences_default() {
        let prefs = UserPreferences::default();
        assert_eq!(prefs.default_port, 22);
        assert_eq!(prefs.max_search_history, 100);
        assert_eq!(prefs.max_recent_connections, 20);
        assert_eq!(prefs.connection_timeout, 30);
        assert!(prefs.auto_save_connections);
    }

    #[test]
    fn test_security_settings_default() {
        let sec = SecuritySettings::default();
        assert_eq!(sec.clipboard_clear_time, 30);
        assert!(sec.lock_on_sleep);
        assert!(sec.strict_host_key_checking);
        assert_eq!(sec.min_password_length, 8);
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = FullConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: FullConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_config_paths() {
        // Just verify it doesn't panic and returns Some
        let _ = ConfigPaths::config_dir();
        let _ = ConfigPaths::config_file();
    }
}
