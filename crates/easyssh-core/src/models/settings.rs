//! Settings Model
//!
//! This module defines the Settings domain model for application configuration.
//! Settings are organized into logical groups for different aspects of the application.
//!
//! # Architecture
//!
//! Settings are organized hierarchically:
//! - `Settings` - Root container with all configuration groups
//!   - `ApplicationSettings` - General app behavior
//!   - `TerminalSettings` - Terminal emulation settings
//!   - `NetworkSettings` - Connection and network settings
//!   - `SecuritySettings` - Security and privacy settings
//!   - `AppearanceSettings` - UI theme and appearance
//!   - `BackupSettings` - Backup and restore settings
//!   - `EncryptionSettings` - Data encryption configuration
//!
//! # Examples
//!
//! ```
//! use easyssh_core::models::{Settings, SettingsBuilder, Validatable};
//!
//! // Create settings for a user
//! let settings = Settings::new("user-123".to_string());
//! assert!(settings.validate().is_ok());
//! assert_eq!(settings.theme(), "system");
//!
//! // Build with custom settings
//! let settings = SettingsBuilder::new()
//!     .user_id("user-456")
//!     .build();
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    Validatable, ValidationError, DEFAULT_CONNECTION_TIMEOUT, DEFAULT_HEARTBEAT_INTERVAL,
    MAX_NAME_LENGTH,
};

/// Current settings schema version
pub const CURRENT_SETTINGS_SCHEMA_VERSION: u32 = 1;

/// Application settings container
///
/// This is the root settings model that contains all configuration
/// for the EasySSH application.
///
/// # Fields
///
/// * `id` - Settings record ID
/// * `user_id` - Owner user ID
/// * `application` - General application settings
/// * `terminal` - Terminal-related settings
/// * `network` - Network/connection settings
/// * `security` - Security settings
/// * `appearance` - UI/appearance settings
/// * `backup` - Backup settings
/// * `encryption` - Encryption settings
/// * `created_at` - Creation timestamp
/// * `updated_at` - Last modification timestamp
/// * `schema_version` - Data schema version
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
    ///
    /// # Arguments
    /// * `user_id` - The user ID these settings belong to
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::models::Settings;
    ///
    /// let settings = Settings::new("user-123".to_string());
    /// assert_eq!(settings.user_id, "user-123");
    /// ```
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
            schema_version: CURRENT_SETTINGS_SCHEMA_VERSION,
        }
    }

    /// Create settings with specific ID (for loading from database)
    ///
    /// # Arguments
    /// * `id` - Settings record ID
    /// * `user_id` - Owner user ID
    /// * All settings category structs
    /// * Timestamps
    #[allow(clippy::too_many_arguments)]
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
            schema_version: CURRENT_SETTINGS_SCHEMA_VERSION,
        }
    }

    /// Update settings and refresh timestamp
    ///
    /// # Arguments
    /// * `f` - Closure that performs the modifications
    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.updated_at = Utc::now();
    }

    /// Update application settings
    ///
    /// # Arguments
    /// * `f` - Closure that modifies application settings
    pub fn update_application<F>(&mut self, f: F)
    where
        F: FnOnce(&mut ApplicationSettings),
    {
        f(&mut self.application);
        self.updated_at = Utc::now();
    }

    /// Update terminal settings
    ///
    /// # Arguments
    /// * `f` - Closure that modifies terminal settings
    pub fn update_terminal<F>(&mut self, f: F)
    where
        F: FnOnce(&mut TerminalSettings),
    {
        f(&mut self.terminal);
        self.updated_at = Utc::now();
    }

    /// Update network settings
    ///
    /// # Arguments
    /// * `f` - Closure that modifies network settings
    pub fn update_network<F>(&mut self, f: F)
    where
        F: FnOnce(&mut NetworkSettings),
    {
        f(&mut self.network);
        self.updated_at = Utc::now();
    }

    /// Update security settings
    ///
    /// # Arguments
    /// * `f` - Closure that modifies security settings
    pub fn update_security<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SecuritySettings),
    {
        f(&mut self.security);
        self.updated_at = Utc::now();
    }

    /// Update appearance settings
    ///
    /// # Arguments
    /// * `f` - Closure that modifies appearance settings
    pub fn update_appearance<F>(&mut self, f: F)
    where
        F: FnOnce(&mut AppearanceSettings),
    {
        f(&mut self.appearance);
        self.updated_at = Utc::now();
    }

    /// Update backup settings
    ///
    /// # Arguments
    /// * `f` - Closure that modifies backup settings
    pub fn update_backup<F>(&mut self, f: F)
    where
        F: FnOnce(&mut BackupSettings),
    {
        f(&mut self.backup);
        self.updated_at = Utc::now();
    }

    /// Update encryption settings
    ///
    /// # Arguments
    /// * `f` - Closure that modifies encryption settings
    pub fn update_encryption<F>(&mut self, f: F)
    where
        F: FnOnce(&mut EncryptionSettings),
    {
        f(&mut self.encryption);
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

    /// Check if the UI is in compact mode
    pub fn is_compact_mode(&self) -> bool {
        self.appearance.compact_mode
    }

    /// Get font size for terminals
    pub fn terminal_font_size(&self) -> u16 {
        self.terminal.font_size
    }

    /// Get connection timeout
    pub fn connection_timeout(&self) -> u64 {
        self.network.connection_timeout
    }

    /// Check if auto-lock is enabled
    pub fn auto_lock_enabled(&self) -> bool {
        self.security.auto_lock
    }

    /// Clone settings without sensitive data
    ///
    /// Creates a copy suitable for logging or API responses.
    pub fn clone_redacted(&self) -> Self {
        Self {
            id: self.id.clone(),
            user_id: self.user_id.clone(),
            application: self.application.clone(),
            terminal: self.terminal.clone(),
            network: self.network.clone(),
            security: SecuritySettings {
                // Redact sensitive security settings
                use_keychain: self.security.use_keychain,
                auto_lock: self.security.auto_lock,
                auto_lock_timeout: self.security.auto_lock_timeout,
                require_password_unlock: self.security.require_password_unlock,
                clear_clipboard_on_exit: self.security.clear_clipboard_on_exit,
                clipboard_timeout: self.security.clipboard_timeout,
                enable_biometric: self.security.enable_biometric,
                session_retention_days: self.security.session_retention_days,
                strict_host_key_checking: self.security.strict_host_key_checking,
                known_hosts_file: self.security.known_hosts_file.clone(),
            },
            appearance: self.appearance.clone(),
            backup: self.backup.clone(),
            encryption: EncryptionSettings {
                // Redact encryption settings
                enable_encryption: self.encryption.enable_encryption,
                kdf_algorithm: self.encryption.kdf_algorithm.clone(),
                cipher: self.encryption.cipher.clone(),
                kdf_iterations: self.encryption.kdf_iterations,
                use_hardware_security: self.encryption.use_hardware_security,
            },
            created_at: self.created_at,
            updated_at: self.updated_at,
            schema_version: self.schema_version,
        }
    }
}

impl Validatable for Settings {
    fn validate(&self) -> Result<(), ValidationError> {
        let mut errors = Vec::new();

        // Validate user_id
        if self.user_id.trim().is_empty() {
            errors.push(ValidationError::missing_field("user_id"));
        }

        // Validate all sub-settings
        if let Err(e) = self.application.validate() {
            errors.push(e);
        }
        if let Err(e) = self.terminal.validate() {
            errors.push(e);
        }
        if let Err(e) = self.network.validate() {
            errors.push(e);
        }
        if let Err(e) = self.security.validate() {
            errors.push(e);
        }
        if let Err(e) = self.appearance.validate() {
            errors.push(e);
        }
        if let Err(e) = self.backup.validate() {
            errors.push(e);
        }
        if let Err(e) = self.encryption.validate() {
            errors.push(e);
        }

        // Validate schema version
        if self.schema_version > CURRENT_SETTINGS_SCHEMA_VERSION {
            errors.push(ValidationError::invalid_field(
                "schema_version",
                format!(
                    "Schema version {} is not supported (max: {})",
                    self.schema_version, CURRENT_SETTINGS_SCHEMA_VERSION
                ),
            ));
        }

        ValidationError::combine(errors)
    }
}

/// General application settings
///
/// Controls core application behavior.
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
        let mut errors = Vec::new();

        // Validate update channel
        let valid_channels = ["stable", "beta", "nightly"];
        if !valid_channels.contains(&self.update_channel.as_str()) {
            errors.push(ValidationError::invalid_field(
                "application.update_channel",
                format!(
                    "Invalid update channel: {}. Must be one of: {:?}",
                    self.update_channel, valid_channels
                ),
            ));
        }

        // Validate idle timeout
        if self.idle_timeout_minutes > 1440 {
            // Max 24 hours
            errors.push(ValidationError::invalid_field(
                "application.idle_timeout_minutes",
                "Idle timeout cannot exceed 24 hours (1440 minutes)",
            ));
        }

        ValidationError::combine(errors)
    }
}

/// Terminal-related settings
///
/// Controls terminal emulation behavior and appearance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TerminalSettings {
    /// Default font family
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// Default font size (in points, 8-72)
    #[serde(default = "default_font_size")]
    pub font_size: u16,
    /// Line height multiplier (1.0 = normal)
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
    /// Scrollback buffer size (in lines, max 100000)
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
        let mut errors = Vec::new();

        // Validate font size
        if self.font_size < 8 || self.font_size > 72 {
            errors.push(ValidationError::out_of_range(
                "terminal.font_size",
                8,
                72,
                self.font_size as i64,
            ));
        }

        // Validate cursor style
        let valid_styles = ["block", "line", "bar"];
        if !valid_styles.contains(&self.cursor_style.as_str()) {
            errors.push(ValidationError::invalid_field(
                "terminal.cursor_style",
                format!(
                    "Invalid cursor style: {}. Must be one of: {:?}",
                    self.cursor_style, valid_styles
                ),
            ));
        }

        // Validate scrollback lines
        if self.scrollback_lines > 100_000 {
            errors.push(ValidationError::invalid_field(
                "terminal.scrollback_lines",
                "Scrollback lines cannot exceed 100,000",
            ));
        }

        // Validate line height
        if self.line_height < 0.5 || self.line_height > 3.0 {
            errors.push(ValidationError::out_of_range(
                "terminal.line_height",
                0.5 as i64,
                3.0 as i64,
                (self.line_height * 10.0) as i64,
            ));
        }

        ValidationError::combine(errors)
    }
}

/// Network and connection settings
///
/// Controls SSH connection behavior and network options.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkSettings {
    /// Default connection timeout (in seconds, 1-300)
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u64,
    /// SSH keepalive interval (in seconds, 0 = disabled, max 3600)
    #[serde(default = "default_heartbeat_interval")]
    pub keepalive_interval: u64,
    /// Maximum retry attempts for failed connections (0-10)
    #[serde(default = "default_retry_attempts")]
    pub max_retry_attempts: u32,
    /// Delay between retry attempts (in seconds, 1-60)
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
    /// SOCKS proxy port (1-65535)
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
    /// ServerAliveCountMax (number of keepalive messages, 1-10)
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
        let mut errors = Vec::new();

        // Validate connection timeout
        if self.connection_timeout == 0 || self.connection_timeout > 300 {
            errors.push(ValidationError::out_of_range(
                "network.connection_timeout",
                1,
                300,
                self.connection_timeout as i64,
            ));
        }

        // Validate keepalive interval
        if self.keepalive_interval > 3600 {
            errors.push(ValidationError::out_of_range(
                "network.keepalive_interval",
                0,
                3600,
                self.keepalive_interval as i64,
            ));
        }

        // Validate compression level
        if self.compression_level > 9 {
            errors.push(ValidationError::out_of_range(
                "network.compression_level",
                1,
                9,
                self.compression_level as i64,
            ));
        }

        // Validate retry settings
        if self.max_retry_attempts > 10 {
            errors.push(ValidationError::out_of_range(
                "network.max_retry_attempts",
                0,
                10,
                self.max_retry_attempts as i64,
            ));
        }

        if self.retry_delay_seconds == 0 || self.retry_delay_seconds > 60 {
            errors.push(ValidationError::out_of_range(
                "network.retry_delay_seconds",
                1,
                60,
                self.retry_delay_seconds as i64,
            ));
        }

        // Validate alive count max
        if self.alive_count_max == 0 || self.alive_count_max > 10 {
            errors.push(ValidationError::out_of_range(
                "network.alive_count_max",
                1,
                10,
                self.alive_count_max as i64,
            ));
        }

        // Validate proxy settings
        if self.proxy_address.is_some() && (self.proxy_port == 0 || self.proxy_port > 65535) {
            errors.push(ValidationError::invalid_field(
                "network.proxy_port",
                format!("Invalid proxy port: {}", self.proxy_port),
            ));
        }

        // Validate IP settings
        if self.ipv4_only && self.ipv6_only {
            errors.push(ValidationError::invalid_field(
                "network.ip_family",
                "Cannot enable both ipv4_only and ipv6_only",
            ));
        }

        ValidationError::combine(errors)
    }
}

/// Security settings
///
/// Controls security-related features and behaviors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecuritySettings {
    /// Whether to use system keychain for passwords
    #[serde(default = "default_true")]
    pub use_keychain: bool,
    /// Whether to lock the app after inactivity
    #[serde(default)]
    pub auto_lock: bool,
    /// Auto-lock timeout (in minutes, 1-60)
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
        let mut errors = Vec::new();

        // Validate auto-lock timeout
        if self.auto_lock {
            if self.auto_lock_timeout == 0 || self.auto_lock_timeout > 60 {
                errors.push(ValidationError::out_of_range(
                    "security.auto_lock_timeout",
                    1,
                    60,
                    self.auto_lock_timeout as i64,
                ));
            }
        }

        // Validate clipboard timeout
        if self.clear_clipboard_on_exit && self.clipboard_timeout > 300 {
            // Max 5 minutes
            errors.push(ValidationError::out_of_range(
                "security.clipboard_timeout",
                0,
                300,
                self.clipboard_timeout as i64,
            ));
        }

        // Validate session retention
        if self.session_retention_days > 365 {
            // Max 1 year
            errors.push(ValidationError::out_of_range(
                "security.session_retention_days",
                0,
                365,
                self.session_retention_days as i64,
            ));
        }

        ValidationError::combine(errors)
    }
}

/// Appearance/UI settings
///
/// Controls the visual appearance of the application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppearanceSettings {
    /// UI theme (light, dark, system)
    #[serde(default = "default_theme")]
    pub theme: String,
    /// UI language code
    #[serde(default = "default_language")]
    pub language: String,
    /// Sidebar width (in pixels, 200-500)
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
        let mut errors = Vec::new();

        // Validate theme
        let valid_themes = ["light", "dark", "system"];
        if !valid_themes.contains(&self.theme.as_str()) {
            errors.push(ValidationError::invalid_field(
                "appearance.theme",
                format!(
                    "Invalid theme: {}. Must be one of: {:?}",
                    self.theme, valid_themes
                ),
            ));
        }

        // Validate density
        let valid_densities = ["compact", "normal", "spacious"];
        if !valid_densities.contains(&self.density.as_str()) {
            errors.push(ValidationError::invalid_field(
                "appearance.density",
                format!(
                    "Invalid density: {}. Must be one of: {:?}",
                    self.density, valid_densities
                ),
            ));
        }

        // Validate sidebar width
        if self.sidebar_width < 200 || self.sidebar_width > 500 {
            errors.push(ValidationError::out_of_range(
                "appearance.sidebar_width",
                200,
                500,
                self.sidebar_width as i64,
            ));
        }

        ValidationError::combine(errors)
    }
}

/// Backup settings
///
/// Controls automatic backup behavior.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupSettings {
    /// Whether automatic backup is enabled
    #[serde(default)]
    pub auto_backup: bool,
    /// Backup interval (in hours, 1-168)
    #[serde(default = "default_backup_interval")]
    pub backup_interval_hours: u32,
    /// Maximum number of backups to keep (1-100)
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
        let mut errors = Vec::new();

        // Validate backup interval if auto backup is enabled
        if self.auto_backup {
            if self.backup_interval_hours == 0 || self.backup_interval_hours > 168 {
                errors.push(ValidationError::out_of_range(
                    "backup.backup_interval_hours",
                    1,
                    168,
                    self.backup_interval_hours as i64,
                ));
            }
        }

        // Validate max backups
        if self.max_backups == 0 || self.max_backups > 100 {
            errors.push(ValidationError::out_of_range(
                "backup.max_backups",
                1,
                100,
                self.max_backups as i64,
            ));
        }

        // Validate backup path if specified
        if !self.backup_path.is_empty() {
            let path = std::path::Path::new(&self.backup_path);
            if path.file_name().is_none() {
                errors.push(ValidationError::invalid_format(
                    "backup.backup_path",
                    "valid filesystem path",
                ));
            }
        }

        ValidationError::combine(errors)
    }
}

/// Encryption settings for data storage
///
/// Controls how data is encrypted at rest.
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
    /// Key derivation iterations (higher = more secure but slower, min 3)
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
        let mut errors = Vec::new();

        // Validate KDF algorithm
        let valid_kdfs = ["argon2id"];
        if !valid_kdfs.contains(&self.kdf_algorithm.as_str()) {
            errors.push(ValidationError::invalid_field(
                "encryption.kdf_algorithm",
                format!(
                    "Invalid KDF algorithm: {}. Must be one of: {:?}",
                    self.kdf_algorithm, valid_kdfs
                ),
            ));
        }

        // Validate cipher
        let valid_ciphers = ["aes-256-gcm", "chacha20-poly1305"];
        if !valid_ciphers.contains(&self.cipher.as_str()) {
            errors.push(ValidationError::invalid_field(
                "encryption.cipher",
                format!(
                    "Invalid cipher: {}. Must be one of: {:?}",
                    self.cipher, valid_ciphers
                ),
            ));
        }

        // Validate KDF iterations
        if self.kdf_iterations < 3 {
            errors.push(ValidationError::out_of_range(
                "encryption.kdf_iterations",
                3,
                i64::MAX,
                self.kdf_iterations as i64,
            ));
        }

        // Validate reasonable upper bound for iterations
        if self.kdf_iterations > 1_000_000 {
            errors.push(ValidationError::invalid_field(
                "encryption.kdf_iterations",
                "KDF iterations exceed reasonable limit (max: 1000000)",
            ));
        }

        ValidationError::combine(errors)
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
///
/// Provides a fluent API for constructing Settings objects with validation.
///
/// # Example
///
/// ```
/// use easyssh_core::models::SettingsBuilder;
///
/// let settings = SettingsBuilder::new()
///     .user_id("user-123")
///     .build();
/// ```
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

    /// Set the user ID (required)
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
    ///
    /// # Panics
    ///
    /// Panics if `user_id` is not set.
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
            schema_version: CURRENT_SETTINGS_SCHEMA_VERSION,
        }
    }

    /// Build with validation
    ///
    /// Validates the settings after construction and returns an error if invalid.
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
        assert_eq!(settings.schema_version, CURRENT_SETTINGS_SCHEMA_VERSION);
        assert_eq!(settings.appearance.theme, "system");
        assert_eq!(settings.appearance.language, "zh-CN");
        assert!(!settings.id.is_empty());
    }

    #[test]
    fn test_settings_validation() {
        let settings = Settings::new("user-123".to_string());
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_settings_validation_empty_user_id() {
        let settings = Settings::new("".to_string());
        let result = settings.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("user_id"));
    }

    #[test]
    fn test_settings_validation_future_schema_version() {
        let mut settings = Settings::new("user-123".to_string());
        settings.schema_version = 999; // Future version
        let result = settings.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Schema version"));
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
        assert_eq!(settings.max_recent_connections, 10);
        assert!(settings.auto_save);
        assert_eq!(settings.idle_timeout_minutes, 0);
        assert!(settings.confirm_before_close);
        assert!(settings.auto_check_updates);
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

        let invalid_timeout = ApplicationSettings {
            idle_timeout_minutes: 2000, // > 24 hours
            ..Default::default()
        };
        assert!(invalid_timeout.validate().is_err());
    }

    #[test]
    fn test_terminal_settings_defaults() {
        let settings = TerminalSettings::default();
        assert_eq!(settings.font_size, 14);
        assert_eq!(settings.cursor_style, "block");
        assert!(settings.cursor_blink);
        assert_eq!(settings.scrollback_lines, 10000);
        assert!(settings.mouse_support);
        assert!(!settings.copy_on_select);
        assert_eq!(settings.line_height, 1.2);
        assert!(!settings.enable_ligatures);
    }

    #[test]
    fn test_terminal_settings_validation() {
        let valid = TerminalSettings::default();
        assert!(valid.validate().is_ok());

        let invalid_font_size = TerminalSettings {
            font_size: 5,
            ..Default::default()
        };
        assert!(invalid_font_size.validate().is_err());

        let invalid_font_size_high = TerminalSettings {
            font_size: 80,
            ..Default::default()
        };
        assert!(invalid_font_size_high.validate().is_err());

        let invalid_cursor = TerminalSettings {
            cursor_style: "invalid".to_string(),
            ..Default::default()
        };
        assert!(invalid_cursor.validate().is_err());

        let invalid_scrollback = TerminalSettings {
            scrollback_lines: 200_000, // > 100,000
            ..Default::default()
        };
        assert!(invalid_scrollback.validate().is_err());
    }

    #[test]
    fn test_network_settings_defaults() {
        let settings = NetworkSettings::default();
        assert_eq!(settings.connection_timeout, DEFAULT_CONNECTION_TIMEOUT);
        assert_eq!(settings.keepalive_interval, DEFAULT_HEARTBEAT_INTERVAL);
        assert_eq!(settings.max_retry_attempts, 3);
        assert!(settings.use_compression);
        assert_eq!(settings.compression_level, 6);
        assert_eq!(settings.proxy_port, 1080);
        assert_eq!(settings.alive_count_max, 3);
    }

    #[test]
    fn test_network_settings_validation() {
        let valid = NetworkSettings::default();
        assert!(valid.validate().is_ok());

        let invalid_compression = NetworkSettings {
            compression_level: 10,
            ..Default::default()
        };
        assert!(invalid_compression.validate().is_err());

        let invalid_timeout = NetworkSettings {
            connection_timeout: 0,
            ..Default::default()
        };
        assert!(invalid_timeout.validate().is_err());

        let invalid_retry = NetworkSettings {
            max_retry_attempts: 20, // > 10
            ..Default::default()
        };
        assert!(invalid_retry.validate().is_err());

        let conflicting_ip = NetworkSettings {
            ipv4_only: true,
            ipv6_only: true,
            ..Default::default()
        };
        assert!(conflicting_ip.validate().is_err());
    }

    #[test]
    fn test_security_settings_defaults() {
        let settings = SecuritySettings::default();
        assert!(settings.use_keychain);
        assert!(!settings.auto_lock);
        assert_eq!(settings.auto_lock_timeout, 15);
        assert!(settings.strict_host_key_checking);
        assert_eq!(settings.clipboard_timeout, 0);
        assert!(!settings.enable_biometric);
    }

    #[test]
    fn test_security_settings_validation() {
        let valid = SecuritySettings::default();
        assert!(valid.validate().is_ok());

        let invalid_lock_timeout = SecuritySettings {
            auto_lock: true,
            auto_lock_timeout: 0,
            ..Default::default()
        };
        assert!(invalid_lock_timeout.validate().is_err());

        let invalid_clipboard = SecuritySettings {
            clear_clipboard_on_exit: true,
            clipboard_timeout: 400, // > 300
            ..Default::default()
        };
        assert!(invalid_clipboard.validate().is_err());

        let invalid_retention = SecuritySettings {
            session_retention_days: 400, // > 365
            ..Default::default()
        };
        assert!(invalid_retention.validate().is_err());
    }

    #[test]
    fn test_appearance_settings_defaults() {
        let settings = AppearanceSettings::default();
        assert_eq!(settings.theme, "system");
        assert_eq!(settings.language, "zh-CN");
        assert_eq!(settings.sidebar_width, 280);
        assert!(settings.show_status_indicators);
        assert!(settings.enable_animations);
        assert!(!settings.compact_mode);
        assert_eq!(settings.density, "normal");
    }

    #[test]
    fn test_appearance_settings_validation() {
        let valid = AppearanceSettings::default();
        assert!(valid.validate().is_ok());

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

        let invalid_sidebar = AppearanceSettings {
            sidebar_width: 100, // < 200
            ..Default::default()
        };
        assert!(invalid_sidebar.validate().is_err());
    }

    #[test]
    fn test_backup_settings_defaults() {
        let settings = BackupSettings::default();
        assert!(!settings.auto_backup);
        assert_eq!(settings.backup_interval_hours, 24);
        assert_eq!(settings.max_backups, 7);
        assert!(settings.encrypt_backups);
        assert!(!settings.include_history);
        assert!(settings.backup_path.is_empty());
    }

    #[test]
    fn test_backup_settings_validation() {
        let valid = BackupSettings::default();
        assert!(valid.validate().is_ok());

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

        let too_many_backups = BackupSettings {
            max_backups: 200, // > 100
            ..Default::default()
        };
        assert!(too_many_backups.validate().is_err());

        let invalid_interval = BackupSettings {
            auto_backup: true,
            backup_interval_hours: 200, // > 168 (1 week)
            ..Default::default()
        };
        assert!(invalid_interval.validate().is_err());
    }

    #[test]
    fn test_encryption_settings_defaults() {
        let settings = EncryptionSettings::default();
        assert!(settings.enable_encryption);
        assert_eq!(settings.kdf_algorithm, "argon2id");
        assert_eq!(settings.cipher, "aes-256-gcm");
        assert_eq!(settings.kdf_iterations, 3);
        assert!(!settings.use_hardware_security);
    }

    #[test]
    fn test_encryption_settings_validation() {
        let valid = EncryptionSettings::default();
        assert!(valid.validate().is_ok());

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

        let too_many_iterations = EncryptionSettings {
            kdf_iterations: 2_000_000,
            ..Default::default()
        };
        assert!(too_many_iterations.validate().is_err());
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
    fn test_settings_update_methods() {
        let mut settings = Settings::new("user-123".to_string());

        settings.update_terminal(|t| {
            t.font_size = 16;
        });
        assert_eq!(settings.terminal.font_size, 16);

        settings.update_network(|n| {
            n.connection_timeout = 60;
        });
        assert_eq!(settings.network.connection_timeout, 60);

        settings.update_security(|s| {
            s.auto_lock = true;
        });
        assert!(settings.security.auto_lock);

        settings.update_appearance(|a| {
            a.theme = "dark".to_string();
        });
        assert_eq!(settings.appearance.theme, "dark");

        settings.update_backup(|b| {
            b.auto_backup = true;
        });
        assert!(settings.backup.auto_backup);

        settings.update_encryption(|e| {
            e.kdf_iterations = 5;
        });
        assert_eq!(settings.encryption.kdf_iterations, 5);
    }

    #[test]
    fn test_settings_convenience_methods() {
        let settings = Settings::new("user-123".to_string());
        assert_eq!(settings.theme(), "system");
        assert_eq!(settings.language(), "zh-CN");
        assert!(!settings.auto_connect());
        assert!(settings.use_keychain());
        assert!(!settings.is_compact_mode());
        assert_eq!(settings.terminal_font_size(), 14);
        assert_eq!(settings.connection_timeout(), 30);
        assert!(!settings.auto_lock_enabled());
    }

    #[test]
    fn test_settings_clone_redacted() {
        let settings = Settings::new("user-123".to_string());
        let redacted = settings.clone_redacted();
        assert_eq!(redacted.id, settings.id);
        assert_eq!(redacted.user_id, settings.user_id);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = Settings::new("user-123".to_string());
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("user-123"));
        assert!(json.contains("system"));

        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_id, "user-123");
        assert_eq!(deserialized.appearance.theme, "system");
    }

    #[test]
    fn test_settings_with_id() {
        let created_at = Utc::now();
        let updated_at = Utc::now();

        let settings = Settings::with_id(
            "settings-123".to_string(),
            "user-456".to_string(),
            ApplicationSettings::default(),
            TerminalSettings::default(),
            NetworkSettings::default(),
            SecuritySettings::default(),
            AppearanceSettings::default(),
            BackupSettings::default(),
            EncryptionSettings::default(),
            created_at,
            updated_at,
        );

        assert_eq!(settings.id, "settings-123");
        assert_eq!(settings.user_id, "user-456");
    }

    #[test]
    fn test_create_settings_dto() {
        let json = r##"{
            "user_id": "user-456",
            "application": {
                "auto_connect": true
            },
            "appearance": {
                "theme": "dark"
            }
        }"##;

        let dto: CreateSettingsDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.user_id, "user-456");
        assert!(dto.application.is_some());
        assert!(dto.appearance.is_some());
        assert!(dto.terminal.is_none());
    }

    #[test]
    fn test_update_settings_dto() {
        let json = r##"{
            "appearance": {
                "theme": "light"
            },
            "terminal": {
                "font_size": 16
            }
        }"##;

        let dto: UpdateSettingsDto = serde_json::from_str(json).unwrap();
        assert!(dto.appearance.is_some());
        assert!(dto.terminal.is_some());
        assert!(dto.network.is_none());
    }

    #[test]
    fn test_constants() {
        assert_eq!(CURRENT_SETTINGS_SCHEMA_VERSION, 1);
        assert_eq!(DEFAULT_CONNECTION_TIMEOUT, 30);
        assert_eq!(DEFAULT_HEARTBEAT_INTERVAL, 30);
    }
}
