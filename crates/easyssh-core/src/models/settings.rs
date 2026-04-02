//! Settings Model
//!
//! This module defines the Settings domain model for application configuration.
//! Settings are organized into logical groups for different aspects of the application.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Validatable, ValidationError, DEFAULT_CONNECTION_TIMEOUT, DEFAULT_HEARTBEAT_INTERVAL};

/// Application settings container
///
/// This is the root settings model that contains all configuration
/// for the EasySSH application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Settings ID (usually there's only one settings record per user)
    pub id: String,
    /// User ID these settings belong to
    pub user_id: String,
    /// General application settings
    #[serde(default)]
    pub application: ApplicationSettings,
    /// Terminal-related settings
    #[serde(default)]
    pub terminal: TerminalSettings,
    /// Network/connection settings
    #[serde(default)]
    pub network: NetworkSettings,
    /// Security settings
    #[serde(default)]
    pub security: SecuritySettings,
    /// UI/appearance settings
    #[serde(default)]
    pub appearance: AppearanceSettings,
    /// Backup settings
    #[serde(default)]
    pub backup: BackupSettings,
    /// Encryption settings for data storage
    #[serde(default)]
    pub encryption: EncryptionSettings,
    /// Creation timestamp
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
    /// Schema version for migrations
    #[serde(default)]
    pub schema_version: u32,
}

impl Settings {
    /// Create new settings for a user
    pub fn new(user_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            application: ApplicationSettings::default(),
            terminal: TerminalSettings::default(),
            network: NetworkSettings::default(),
            security: SecuritySettings::default(),
            appearance: AppearanceSettings::default(),
            backup: BackupSettings::default(),
            encryption: EncryptionSettings::default(),
            created_at: now,
            updated_at: now,
            schema_version: 1,
        }
    }

    /// Create settings with specific ID (for loading from database)
    pub fn with_id(
        id: String,
        user_id: String,
        application: ApplicationSettings,
        terminal: TerminalSettings,
        network: NetworkSettings,
        security: SecuritySettings,
        appearance: AppearanceSettings,
        backup: BackupSettings,
        encryption: EncryptionSettings,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            application,
            terminal,
            network,
            security,
            appearance,
            backup,
            encryption,
            created_at,
            updated_at,
            schema_version: 1,
        }
    }

    /// Update settings and refresh timestamp
    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.updated_at = Utc::now();
    }

    /// Update application settings
    pub fn update_application<F>(&mut self, f: F)
    where
        F: FnOnce(&mut ApplicationSettings),
    {
        f(&mut self.application);
        self.updated_at = Utc::now();
    }

    /// Update terminal settings
    pub fn update_terminal<F>(&mut self, f: F)
    where
        F: FnOnce(&mut TerminalSettings),
    {
        f(&mut self.terminal);
        self.updated_at = Utc::now();
    }

    /// Update network settings
    pub fn update_network<F>(&mut self, f: F)
    where
        F: FnOnce(&mut NetworkSettings),
    {
        f(&mut self.network);
        self.updated_at = Utc::now();
    }

    /// Update security settings
    pub fn update_security<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SecuritySettings),
    {
        f(&mut self.security);
        self.updated_at = Utc::now();
    }

    /// Update appearance settings
    pub fn update_appearance<F>(&mut self, f: F)
    where
        F: FnOnce(&mut AppearanceSettings),
    {
        f(&mut self.appearance);
        self.updated_at = Utc::now();
    }

    /// Get the theme setting (convenience method)
    pub fn theme(&self) -> &str {
        &self.appearance.theme
    }

    /// Get the language setting (convenience method)
    pub fn language(&self) -> &str {
        &self.appearance.language
    }

    /// Check if auto-connect is enabled
    pub fn auto_connect(&self) -> bool {
        self.application.auto_connect
    }

    /// Check if keychain integration is enabled
    pub fn use_keychain(&self) -> bool {
        self.security.use_keychain
    }
}

impl Validatable for Settings {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate user_id
        if self.user_id.is_empty() {
            return Err(ValidationError::MissingField("user_id".to_string()));
        }

        // Validate all sub-settings
        self.application.validate()?;
        self.terminal.validate()?;
        self.network.validate()?;
        self.security.validate()?;
        self.appearance.validate()?;
        self.backup.validate()?;
        self.encryption.validate()?;

        Ok(())
    }
}

/// General application settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplicationSettings {
    /// Whether to auto-connect to last server on startup
    #[serde(default)]
    pub auto_connect: bool,
    /// Whether to remember window position and size
    #[serde(default = "default_true")]
    pub remember_window_state: bool,
    /// Whether to minimize to system tray on close
    #[serde(default)]
    pub minimize_to_tray: bool,
    /// Whether to show connection notifications
    #[serde(default = "default_true")]
    pub show_notifications: bool,
    /// Whether to enable sound effects
    #[serde(default)]
    pub sound_enabled: bool,
    /// Whether to confirm before closing active connections
    #[serde(default = "default_true")]
    pub confirm_before_close: bool,
    /// Maximum number of recent connections to keep
    #[serde(default = "default_recent_connections")]
    pub max_recent_connections: u32,
    /// Whether to automatically save server changes
    #[serde(default = "default_true")]
    pub auto_save: bool,
    /// Idle timeout for connections (0 = no timeout)
    #[serde(default)]
    pub idle_timeout_minutes: u32,
    /// Whether to check for updates automatically
    #[serde(default = "default_true")]
    pub auto_check_updates: bool,
    /// Update channel (stable, beta, nightly)
    #[serde(default = "default_update_channel")]
    pub update_channel: String,
}

impl Default for ApplicationSettings {
    fn default() -> Self {
        Self {
            auto_connect: false,
            remember_window_state: true,
            minimize_to_tray: false,
            show_notifications: true,
            sound_enabled: false,
            confirm_before_close: true,
            max_recent_connections: 10,
            auto_save: true,
            idle_timeout_minutes: 0,
            auto_check_updates: true,
            update_channel: "stable".to_string(),
        }
    }
}

impl Validatable for ApplicationSettings {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate update channel
        let valid_channels = ["stable", "beta", "nightly"];
        if !valid_channels.contains(&self.update_channel.as_str()) {
            return Err(ValidationError::InvalidField {
                field: "application.update_channel".to_string(),
                message: format!(
                    "Invalid update channel: {}. Must be one of: {:?}",
                    self.update_channel, valid_channels
                ),
            });
        }

        Ok(())
    }
}

/// Terminal-related settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TerminalSettings {
    /// Default font family
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// Default font size (in points)
    #[serde(default = "default_font_size")]
    pub font_size: u16,
    /// Line height multiplier
    #[serde(default = "default_line_height")]
    pub line_height: f32,
    /// Whether to enable ligatures
    #[serde(default)]
    pub enable_ligatures: bool,
    /// Cursor style (block, line, bar)
    #[serde(default = "default_cursor_style")]
    pub cursor_style: String,
    /// Whether cursor blinks
    #[serde(default = "default_true")]
    pub cursor_blink: bool,
    /// Whether to enable mouse support
    #[serde(default = "default_true")]
    pub mouse_support: bool,
    /// Scrollback buffer size (in lines)
    #[serde(default = "default_scrollback_lines")]
    pub scrollback_lines: u32,
    /// Whether to copy on select
    #[serde(default)]
    pub copy_on_select: bool,
    /// Whether to paste on right-click
    #[serde(default)]
    pub paste_on_right_click: bool,
    /// Terminal color scheme name
    #[serde(default = "default_color_scheme")]
    pub color_scheme: String,
    /// Whether to use bold text
    #[serde(default = "default_true")]
    pub enable_bold: bool,
    /// Shell to use (empty = default shell)
    #[serde(default)]
    pub shell: String,
    /// Environment variables to set
    #[serde(default)]
    pub env_vars: std::collections::HashMap<String, String>,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            font_family: default_font_family(),
            font_size: default_font_size(),
            line_height: default_line_height(),
            enable_ligatures: false,
            cursor_style: default_cursor_style(),
            cursor_blink: true,
            mouse_support: true,
            scrollback_lines: default_scrollback_lines(),
            copy_on_select: false,
            paste_on_right_click: false,
            color_scheme: default_color_scheme(),
            enable_bold: true,
            shell: String::new(),
            env_vars: std::collections::HashMap::new(),
        }
    }
}

impl Validatable for TerminalSettings {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate font size
        if self.font_size < 8 || self.font_size > 72 {
            return Err(ValidationError::OutOfRange {
                field: "terminal.font_size".to_string(),
                min: 8,
                max: 72,
                actual: self.font_size as i64,
            });
        }

        // Validate cursor style
        let valid_styles = ["block", "line", "bar"];
        if !valid_styles.contains(&self.cursor_style.as_str()) {
            return Err(ValidationError::InvalidField {
                field: "terminal.cursor_style".to_string(),
                message: format!(
                    "Invalid cursor style: {}. Must be one of: {:?}",
                    self.cursor_style, valid_styles
                ),
            });
        }

        Ok(())
    }
}

/// Network and connection settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkSettings {
    /// Default connection timeout (in seconds)
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u64,
    /// SSH keepalive interval (in seconds, 0 = disabled)
    #[serde(default = "default_heartbeat_interval")]
    pub keepalive_interval: u64,
    /// Maximum retry attempts for failed connections
    #[serde(default = "default_retry_attempts")]
    pub max_retry_attempts: u32,
    /// Delay between retry attempts (in seconds)
    #[serde(default = "default_retry_delay")]
    pub retry_delay_seconds: u32,
    /// Whether to use compression
    #[serde(default = "default_true")]
    pub use_compression: bool,
    /// Compression level (1-9, where 9 is best compression)
    #[serde(default = "default_compression_level")]
    pub compression_level: u8,
    /// SOCKS proxy address (optional)
    pub proxy_address: Option<String>,
    /// SOCKS proxy port
    #[serde(default = "default_proxy_port")]
    pub proxy_port: u16,
    /// Proxy username (optional)
    pub proxy_username: Option<String>,
    /// Whether to use IPv4 only
    #[serde(default)]
    pub ipv4_only: bool,
    /// Whether to use IPv6 only
    #[serde(default)]
    pub ipv6_only: bool,
    /// ServerAliveCountMax (number of keepalive messages)
    #[serde(default = "default_alive_count_max")]
    pub alive_count_max: u32,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            connection_timeout: DEFAULT_CONNECTION_TIMEOUT,
            keepalive_interval: DEFAULT_HEARTBEAT_INTERVAL,
            max_retry_attempts: 3,
            retry_delay_seconds: 5,
            use_compression: true,
            compression_level: 6,
            proxy_address: None,
            proxy_port: 1080,
            proxy_username: None,
            ipv4_only: false,
            ipv6_only: false,
            alive_count_max: 3,
        }
    }
}

impl Validatable for NetworkSettings {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate compression level
        if self.compression_level > 9 {
            return Err(ValidationError::OutOfRange {
                field: "network.compression_level".to_string(),
                min: 1,
                max: 9,
                actual: self.compression_level as i64,
            });
        }

        // Validate proxy port if proxy is set
        if self.proxy_address.is_some() && (self.proxy_port == 0 || self.proxy_port > 65535) {
            return Err(ValidationError::InvalidField {
                field: "network.proxy_port".to_string(),
                message: format!("Invalid proxy port: {}", self.proxy_port),
            });
        }

        Ok(())
    }
}

/// Security settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecuritySettings {
    /// Whether to use system keychain for passwords
    #[serde(default = "default_true")]
    pub use_keychain: bool,
    /// Whether to lock the app after inactivity
    #[serde(default)]
    pub auto_lock: bool,
    /// Auto-lock timeout (in minutes)
    #[serde(default = "default_lock_timeout")]
    pub auto_lock_timeout: u32,
    /// Whether to require password to unlock
    #[serde(default)]
    pub require_password_unlock: bool,
    /// Whether to clear clipboard on exit
    #[serde(default)]
    pub clear_clipboard_on_exit: bool,
    /// Clipboard clear timeout (in seconds, 0 = immediate)
    #[serde(default)]
    pub clipboard_timeout: u32,
    /// Whether to enable fingerprint authentication (mobile/Windows Hello)
    #[serde(default)]
    pub enable_biometric: bool,
    /// Session recording retention days (0 = no retention)
    #[serde(default)]
    pub session_retention_days: u32,
    /// Whether to verify host keys strictly
    #[serde(default = "default_true")]
    pub strict_host_key_checking: bool,
    /// Path to known hosts file (empty = default)
    #[serde(default)]
    pub known_hosts_file: String,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            use_keychain: true,
            auto_lock: false,
            auto_lock_timeout: 15,
            require_password_unlock: false,
            clear_clipboard_on_exit: false,
            clipboard_timeout: 0,
            enable_biometric: false,
            session_retention_days: 0,
            strict_host_key_checking: true,
            known_hosts_file: String::new(),
        }
    }
}

impl Validatable for SecuritySettings {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate auto-lock timeout
        if self.auto_lock && self.auto_lock_timeout == 0 {
            return Err(ValidationError::InvalidField {
                field: "security.auto_lock_timeout".to_string(),
                message: "Auto-lock timeout must be > 0 when auto-lock is enabled".to_string(),
            });
        }

        Ok(())
    }
}

/// Appearance/UI settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppearanceSettings {
    /// UI theme (light, dark, system)
    #[serde(default = "default_theme")]
    pub theme: String,
    /// UI language code
    #[serde(default = "default_language")]
    pub language: String,
    /// Sidebar width (in pixels)
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u32,
    /// Whether to show server status indicators
    #[serde(default = "default_true")]
    pub show_status_indicators: bool,
    /// Whether to use animated transitions
    #[serde(default = "default_true")]
    pub enable_animations: bool,
    /// Whether to compact the UI (smaller paddings)
    #[serde(default)]
    pub compact_mode: bool,
    /// Date format string
    #[serde(default = "default_date_format")]
    pub date_format: String,
    /// Time format string
    #[serde(default = "default_time_format")]
    pub time_format: String,
    /// Whether to use 24-hour time
    #[serde(default = "default_true")]
    pub use_24_hour: bool,
    /// Density setting (compact, normal, spacious)
    #[serde(default = "default_density")]
    pub density: String,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            language: default_language(),
            sidebar_width: default_sidebar_width(),
            show_status_indicators: true,
            enable_animations: true,
            compact_mode: false,
            date_format: default_date_format(),
            time_format: default_time_format(),
            use_24_hour: true,
            density: default_density(),
        }
    }
}

impl Validatable for AppearanceSettings {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate theme
        let valid_themes = ["light", "dark", "system"];
        if !valid_themes.contains(&self.theme.as_str()) {
            return Err(ValidationError::InvalidField {
                field: "appearance.theme".to_string(),
                message: format!(
                    "Invalid theme: {}. Must be one of: {:?}",
                    self.theme, valid_themes
                ),
            });
        }

        // Validate density
        let valid_densities = ["compact", "normal", "spacious"];
        if !valid_densities.contains(&self.density.as_str()) {
            return Err(ValidationError::InvalidField {
                field: "appearance.density".to_string(),
                message: format!(
                    "Invalid density: {}. Must be one of: {:?}",
                    self.density, valid_densities
                ),
            });
        }

        Ok(())
    }
}

/// Backup settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupSettings {
    /// Whether automatic backup is enabled
    #[serde(default)]
    pub auto_backup: bool,
    /// Backup interval (in hours)
    #[serde(default = "default_backup_interval")]
    pub backup_interval_hours: u32,
    /// Maximum number of backups to keep
    #[serde(default = "default_max_backups")]
    pub max_backups: u32,
    /// Backup destination path (empty = default)
    #[serde(default)]
    pub backup_path: String,
    /// Whether to include connection history in backup
    #[serde(default)]
    pub include_history: bool,
    /// Whether to encrypt backups
    #[serde(default = "default_true")]
    pub encrypt_backups: bool,
    /// Last backup timestamp
    pub last_backup: Option<DateTime<Utc>>,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            auto_backup: false,
            backup_interval_hours: 24,
            max_backups: 7,
            backup_path: String::new(),
            include_history: false,
            encrypt_backups: true,
            last_backup: None,
        }
    }
}

impl Validatable for BackupSettings {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate backup interval if auto backup is enabled
        if self.auto_backup && self.backup_interval_hours == 0 {
            return Err(ValidationError::InvalidField {
                field: "backup.backup_interval_hours".to_string(),
                message: "Backup interval must be > 0 when auto backup is enabled".to_string(),
            });
        }

        // Validate max backups
        if self.max_backups == 0 {
            return Err(ValidationError::InvalidField {
                field: "backup.max_backups".to_string(),
                message: "Max backups must be > 0".to_string(),
            });
        }

        Ok(())
    }
}

/// Encryption settings for data storage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EncryptionSettings {
    /// Whether database encryption is enabled
    #[serde(default = "default_true")]
    pub enable_encryption: bool,
    /// Key derivation algorithm
    #[serde(default = "default_kdf_algorithm")]
    pub kdf_algorithm: String,
    /// Encryption algorithm
    #[serde(default = "default_cipher")]
    pub cipher: String,
    /// Key derivation iterations (higher = more secure but slower)
    #[serde(default = "default_kdf_iterations")]
    pub kdf_iterations: u32,
    /// Whether to use hardware security module if available
    #[serde(default)]
    pub use_hardware_security: bool,
}

impl Default for EncryptionSettings {
    fn default() -> Self {
        Self {
            enable_encryption: true,
            kdf_algorithm: default_kdf_algorithm(),
            cipher: default_cipher(),
            kdf_iterations: default_kdf_iterations(),
            use_hardware_security: false,
        }
    }
}

impl Validatable for EncryptionSettings {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate KDF algorithm
        let valid_kdfs = ["argon2id"];
        if !valid_kdfs.contains(&self.kdf_algorithm.as_str()) {
            return Err(ValidationError::InvalidField {
                field: "encryption.kdf_algorithm".to_string(),
                message: format!(
                    "Invalid KDF algorithm: {}. Must be one of: {:?}",
                    self.kdf_algorithm, valid_kdfs
                ),
            });
        }

        // Validate cipher
        let valid_ciphers = ["aes-256-gcm", "chacha20-poly1305"];
        if !valid_ciphers.contains(&self.cipher.as_str()) {
            return Err(ValidationError::InvalidField {
                field: "encryption.cipher".to_string(),
                message: format!(
                    "Invalid cipher: {}. Must be one of: {:?}",
                    self.cipher, valid_ciphers
                ),
            });
        }

        // Validate KDF iterations
        if self.kdf_iterations < 3 {
            return Err(ValidationError::OutOfRange {
                field: "encryption.kdf_iterations".to_string(),
                min: 3,
                max: i64::MAX,
                actual: self.kdf_iterations as i64,
            });
        }

        Ok(())
    }
}

// Default value functions for serde
fn default_true() -> bool {
    true
}

fn default_recent_connections() -> u32 {
    10
}

fn default_update_channel() -> String {
    "stable".to_string()
}

fn default_font_family() -> String {
    "JetBrains Mono, Fira Code, Consolas, monospace".to_string()
}

fn default_font_size() -> u16 {
    14
}

fn default_line_height() -> f32 {
    1.2
}

fn default_cursor_style() -> String {
    "block".to_string()
}

fn default_scrollback_lines() -> u32 {
    10000
}

fn default_color_scheme() -> String {
    "default".to_string()
}

fn default_connection_timeout() -> u64 {
    DEFAULT_CONNECTION_TIMEOUT
}

fn default_heartbeat_interval() -> u64 {
    DEFAULT_HEARTBEAT_INTERVAL
}

fn default_retry_attempts() -> u32 {
    3
}

fn default_retry_delay() -> u32 {
    5
}

fn default_compression_level() -> u8 {
    6
}

fn default_proxy_port() -> u16 {
    1080
}

fn default_alive_count_max() -> u32 {
    3
}

fn default_lock_timeout() -> u32 {
    15
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_language() -> String {
    "zh-CN".to_string()
}

fn default_sidebar_width() -> u32 {
    280
}

fn default_date_format() -> String {
    "YYYY-MM-DD".to_string()
}

fn default_time_format() -> String {
    "HH:mm:ss".to_string()
}

fn default_density() -> String {
    "normal".to_string()
}

fn default_backup_interval() -> u32 {
    24
}

fn default_max_backups() -> u32 {
    7
}

fn default_kdf_algorithm() -> String {
    "argon2id".to_string()
}

fn default_cipher() -> String {
    "aes-256-gcm".to_string()
}

fn default_kdf_iterations() -> u32 {
    3
}

/// Builder for creating Settings instances
#[derive(Debug, Default)]
pub struct SettingsBuilder {
    id: Option<String>,
    user_id: Option<String>,
    application: Option<ApplicationSettings>,
    terminal: Option<TerminalSettings>,
    network: Option<NetworkSettings>,
    security: Option<SecuritySettings>,
    appearance: Option<AppearanceSettings>,
    backup: Option<BackupSettings>,
    encryption: Option<EncryptionSettings>,
}

impl SettingsBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the settings ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the user ID
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set application settings
    pub fn application(mut self, settings: ApplicationSettings) -> Self {
        self.application = Some(settings);
        self
    }

    /// Set terminal settings
    pub fn terminal(mut self, settings: TerminalSettings) -> Self {
        self.terminal = Some(settings);
        self
    }

    /// Set network settings
    pub fn network(mut self, settings: NetworkSettings) -> Self {
        self.network = Some(settings);
        self
    }

    /// Set security settings
    pub fn security(mut self, settings: SecuritySettings) -> Self {
        self.security = Some(settings);
        self
    }

    /// Set appearance settings
    pub fn appearance(mut self, settings: AppearanceSettings) -> Self {
        self.appearance = Some(settings);
        self
    }

    /// Set backup settings
    pub fn backup(mut self, settings: BackupSettings) -> Self {
        self.backup = Some(settings);
        self
    }

    /// Set encryption settings
    pub fn encryption(mut self, settings: EncryptionSettings) -> Self {
        self.encryption = Some(settings);
        self
    }

    /// Build the Settings instance
    pub fn build(self) -> Settings {
        let now = Utc::now();
        Settings {
            id: self.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            user_id: self.user_id.expect("user_id is required"),
            application: self.application.unwrap_or_default(),
            terminal: self.terminal.unwrap_or_default(),
            network: self.network.unwrap_or_default(),
            security: self.security.unwrap_or_default(),
            appearance: self.appearance.unwrap_or_default(),
            backup: self.backup.unwrap_or_default(),
            encryption: self.encryption.unwrap_or_default(),
            created_at: now,
            updated_at: now,
            schema_version: 1,
        }
    }

    /// Build with validation
    pub fn build_validated(self) -> Result<Settings, ValidationError> {
        let settings = self.build();
        settings.validate()?;
        Ok(settings)
    }
}

/// DTO for creating settings
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSettingsDto {
    pub user_id: String,
    #[serde(default)]
    pub application: Option<ApplicationSettings>,
    #[serde(default)]
    pub terminal: Option<TerminalSettings>,
    #[serde(default)]
    pub network: Option<NetworkSettings>,
    #[serde(default)]
    pub security: Option<SecuritySettings>,
    #[serde(default)]
    pub appearance: Option<AppearanceSettings>,
    #[serde(default)]
    pub backup: Option<BackupSettings>,
    #[serde(default)]
    pub encryption: Option<EncryptionSettings>,
}

/// DTO for updating settings
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSettingsDto {
    #[serde(default)]
    pub application: Option<ApplicationSettings>,
    #[serde(default)]
    pub terminal: Option<TerminalSettings>,
    #[serde(default)]
    pub network: Option<NetworkSettings>,
    #[serde(default)]
    pub security: Option<SecuritySettings>,
    #[serde(default)]
    pub appearance: Option<AppearanceSettings>,
    #[serde(default)]
    pub backup: Option<BackupSettings>,
    #[serde(default)]
    pub encryption: Option<EncryptionSettings>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_new() {
        let settings = Settings::new("user-123".to_string());
        assert_eq!(settings.user_id, "user-123");
        assert_eq!(settings.schema_version, 1);
        assert_eq!(settings.appearance.theme, "system");
        assert_eq!(settings.appearance.language, "zh-CN");
    }

    #[test]
    fn test_settings_validation() {
        let settings = Settings::new("user-123".to_string());
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_settings_validation_empty_user_id() {
        let settings = Settings::new("".to_string());
        assert!(
            matches!(settings.validate(), Err(ValidationError::MissingField(field)) if field == "user_id")
        );
    }

    #[test]
    fn test_settings_builder() {
        let settings = SettingsBuilder::new()
            .user_id("user-456")
            .application(ApplicationSettings {
                auto_connect: true,
                ..Default::default()
            })
            .build();

        assert_eq!(settings.user_id, "user-456");
        assert!(settings.application.auto_connect);
    }

    #[test]
    fn test_settings_builder_validated() {
        let result = SettingsBuilder::new()
            .user_id("user-789")
            .appearance(AppearanceSettings {
                theme: "invalid_theme".to_string(),
                ..Default::default()
            })
            .build_validated();

        assert!(result.is_err());
    }

    #[test]
    fn test_application_settings_defaults() {
        let settings = ApplicationSettings::default();
        assert!(!settings.auto_connect);
        assert!(settings.remember_window_state);
        assert!(settings.show_notifications);
        assert!(!settings.sound_enabled);
        assert_eq!(settings.update_channel, "stable");
    }

    #[test]
    fn test_application_settings_validation() {
        let valid = ApplicationSettings::default();
        assert!(valid.validate().is_ok());

        let invalid = ApplicationSettings {
            update_channel: "invalid".to_string(),
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_terminal_settings_defaults() {
        let settings = TerminalSettings::default();
        assert_eq!(settings.font_size, 14);
        assert_eq!(settings.cursor_style, "block");
        assert!(settings.cursor_blink);
        assert_eq!(settings.scrollback_lines, 10000);
    }

    #[test]
    fn test_terminal_settings_validation() {
        let valid = TerminalSettings::default();
        assert!(valid.validate().is_ok());

        let invalid_font_size = TerminalSettings {
            font_size: 5,
            ..Default::default()
        };
        assert!(
            matches!(invalid_font_size.validate(), Err(ValidationError::OutOfRange { field, .. }) if field == "terminal.font_size")
        );

        let invalid_cursor = TerminalSettings {
            cursor_style: "invalid".to_string(),
            ..Default::default()
        };
        assert!(invalid_cursor.validate().is_err());
    }

    #[test]
    fn test_network_settings_defaults() {
        let settings = NetworkSettings::default();
        assert_eq!(settings.connection_timeout, DEFAULT_CONNECTION_TIMEOUT);
        assert_eq!(settings.keepalive_interval, DEFAULT_HEARTBEAT_INTERVAL);
        assert_eq!(settings.max_retry_attempts, 3);
        assert!(settings.use_compression);
    }

    #[test]
    fn test_network_settings_validation() {
        let valid = NetworkSettings::default();
        assert!(valid.validate().is_ok());

        let invalid_compression = NetworkSettings {
            compression_level: 10,
            ..Default::default()
        };
        assert!(
            matches!(invalid_compression.validate(), Err(ValidationError::OutOfRange { field, .. }) if field == "network.compression_level")
        );
    }

    #[test]
    fn test_security_settings_defaults() {
        let settings = SecuritySettings::default();
        assert!(settings.use_keychain);
        assert!(!settings.auto_lock);
        assert!(settings.strict_host_key_checking);
    }

    #[test]
    fn test_security_settings_validation() {
        let invalid = SecuritySettings {
            auto_lock: true,
            auto_lock_timeout: 0,
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_appearance_settings_defaults() {
        let settings = AppearanceSettings::default();
        assert_eq!(settings.theme, "system");
        assert_eq!(settings.language, "zh-CN");
        assert_eq!(settings.sidebar_width, 280);
        assert!(settings.enable_animations);
    }

    #[test]
    fn test_appearance_settings_validation() {
        let invalid_theme = AppearanceSettings {
            theme: "invalid".to_string(),
            ..Default::default()
        };
        assert!(invalid_theme.validate().is_err());

        let invalid_density = AppearanceSettings {
            density: "invalid".to_string(),
            ..Default::default()
        };
        assert!(invalid_density.validate().is_err());
    }

    #[test]
    fn test_backup_settings_defaults() {
        let settings = BackupSettings::default();
        assert!(!settings.auto_backup);
        assert_eq!(settings.backup_interval_hours, 24);
        assert_eq!(settings.max_backups, 7);
        assert!(settings.encrypt_backups);
    }

    #[test]
    fn test_backup_settings_validation() {
        let invalid = BackupSettings {
            auto_backup: true,
            backup_interval_hours: 0,
            ..Default::default()
        };
        assert!(invalid.validate().is_err());

        let zero_backups = BackupSettings {
            max_backups: 0,
            ..Default::default()
        };
        assert!(zero_backups.validate().is_err());
    }

    #[test]
    fn test_encryption_settings_defaults() {
        let settings = EncryptionSettings::default();
        assert!(settings.enable_encryption);
        assert_eq!(settings.kdf_algorithm, "argon2id");
        assert_eq!(settings.cipher, "aes-256-gcm");
        assert_eq!(settings.kdf_iterations, 3);
    }

    #[test]
    fn test_encryption_settings_validation() {
        let invalid_kdf = EncryptionSettings {
            kdf_algorithm: "invalid".to_string(),
            ..Default::default()
        };
        assert!(invalid_kdf.validate().is_err());

        let invalid_cipher = EncryptionSettings {
            cipher: "invalid".to_string(),
            ..Default::default()
        };
        assert!(invalid_cipher.validate().is_err());

        let invalid_iterations = EncryptionSettings {
            kdf_iterations: 2,
            ..Default::default()
        };
        assert!(invalid_iterations.validate().is_err());
    }

    #[test]
    fn test_settings_update() {
        let mut settings = Settings::new("user-123".to_string());
        let old_updated = settings.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));

        settings.update_application(|app| {
            app.auto_connect = true;
        });

        assert!(settings.application.auto_connect);
        assert!(settings.updated_at > old_updated);
    }

    #[test]
    fn test_settings_convenience_methods() {
        let settings = Settings::new("user-123".to_string());
        assert_eq!(settings.theme(), "system");
        assert_eq!(settings.language(), "zh-CN");
        assert!(!settings.auto_connect());
        assert!(settings.use_keychain());
    }

    #[test]
    fn test_settings_serialization() {
        let settings = Settings::new("user-123".to_string());
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("user-123"));
        assert!(json.contains("system"));

        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_id, "user-123");
    }

    #[test]
    fn test_create_settings_dto() {
        let json = r##"{
            "user_id": "user-456",
            "application": {
                "auto_connect": true,
                "theme": "dark"
            }
        }"##;

        let dto: CreateSettingsDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.user_id, "user-456");
        assert!(dto.application.is_some());
    }
}
