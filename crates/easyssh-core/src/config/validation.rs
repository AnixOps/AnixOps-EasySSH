//! Configuration Validation
//!
//! Provides validation for configuration values to ensure data integrity
//! and prevent invalid configurations from being saved.

use super::types::{AppConfig, FullConfig, SecuritySettings, UserPreferences, WindowGeometry};
use std::fmt;

/// Validation error details
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub severity: ValidationSeverity,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: ValidationSeverity::Error,
        }
    }

    /// Create a warning-level validation issue
    pub fn warning(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: ValidationSeverity::Warning,
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.field, self.message)
    }
}

/// Validation severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Warning,
    Error,
}

impl fmt::Display for ValidationSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationSeverity::Warning => write!(f, "WARNING"),
            ValidationSeverity::Error => write!(f, "ERROR"),
        }
    }
}

/// Validation result type
pub type ValidationResult = std::result::Result<(), Vec<ValidationError>>;

/// Configuration validator
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate the full configuration
    pub fn validate(config: &FullConfig) -> ValidationResult {
        let mut errors = Vec::new();

        // Validate each section
        Self::validate_app_config(&config.app_config, &mut errors);
        Self::validate_user_preferences(&config.user_preferences, &mut errors);
        Self::validate_security_settings(&config.security_settings, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate app configuration
    pub fn validate_app_config(config: &AppConfig, errors: &mut Vec<ValidationError>) {
        // Validate window geometry
        Self::validate_window_geometry(&config.window_geometry, errors);

        // Validate sidebar width
        if config.sidebar_width < 100 {
            errors.push(ValidationError::warning(
                "app_config.sidebar_width",
                "Sidebar width is very small (minimum recommended is 100)",
            ));
        }
        if config.sidebar_width > 800 {
            errors.push(ValidationError::warning(
                "app_config.sidebar_width",
                "Sidebar width is very large (maximum recommended is 800)",
            ));
        }

        // Validate shortcuts exist
        if config.shortcuts.new_connection.key.is_empty() {
            errors.push(ValidationError::new(
                "app_config.shortcuts.new_connection",
                "Shortcut key cannot be empty",
            ));
        }

        // Validate terminal setting
        if config.default_terminal.is_empty() {
            errors.push(ValidationError::warning(
                "app_config.default_terminal",
                "Default terminal is not set",
            ));
        }
    }

    /// Validate window geometry
    fn validate_window_geometry(geometry: &WindowGeometry, errors: &mut Vec<ValidationError>) {
        // Check for reasonable window dimensions
        if geometry.width < 400 {
            errors.push(ValidationError::warning(
                "app_config.window_geometry.width",
                "Window width is very small (minimum recommended is 400)",
            ));
        }
        if geometry.width > 10000 {
            errors.push(ValidationError::warning(
                "app_config.window_geometry.width",
                "Window width is unrealistically large",
            ));
        }
        if geometry.height < 300 {
            errors.push(ValidationError::warning(
                "app_config.window_geometry.height",
                "Window height is very small (minimum recommended is 300)",
            ));
        }
        if geometry.height > 10000 {
            errors.push(ValidationError::warning(
                "app_config.window_geometry.height",
                "Window height is unrealistically large",
            ));
        }

        // Check for off-screen positioning (allow some margin)
        const MARGIN: i32 = 10000;
        if geometry.x < -MARGIN || geometry.x > MARGIN {
            errors.push(ValidationError::warning(
                "app_config.window_geometry.x",
                "Window X position is far off-screen",
            ));
        }
        if geometry.y < -MARGIN || geometry.y > MARGIN {
            errors.push(ValidationError::warning(
                "app_config.window_geometry.y",
                "Window Y position is far off-screen",
            ));
        }
    }

    /// Validate user preferences
    pub fn validate_user_preferences(prefs: &UserPreferences, errors: &mut Vec<ValidationError>) {
        // Validate port range
        if prefs.default_port == 0 {
            errors.push(ValidationError::new(
                "user_preferences.default_port",
                "Default port cannot be 0",
            ));
        }
        if prefs.default_port < 1024 {
            errors.push(ValidationError::warning(
                "user_preferences.default_port",
                "Default port is in the well-known port range (<1024), may require elevated privileges",
            ));
        }

        // Validate search history limits
        if prefs.max_search_history == 0 {
            errors.push(ValidationError::warning(
                "user_preferences.max_search_history",
                "Search history limit is 0, history will not be saved",
            ));
        }
        if prefs.max_search_history > 10000 {
            errors.push(ValidationError::warning(
                "user_preferences.max_search_history",
                "Search history limit is very high, may impact performance",
            ));
        }

        // Validate recent connections limit
        if prefs.max_recent_connections > 1000 {
            errors.push(ValidationError::warning(
                "user_preferences.max_recent_connections",
                "Recent connections limit is very high, may impact performance",
            ));
        }

        // Validate timeout values
        if prefs.connection_timeout == 0 {
            errors.push(ValidationError::warning(
                "user_preferences.connection_timeout",
                "Connection timeout is 0, connections may hang indefinitely",
            ));
        }
        if prefs.connection_timeout > 300 {
            errors.push(ValidationError::warning(
                "user_preferences.connection_timeout",
                "Connection timeout is very long (>5 minutes)",
            ));
        }

        // Validate keepalive interval
        if prefs.keepalive_interval > 3600 {
            errors.push(ValidationError::warning(
                "user_preferences.keepalive_interval",
                "Keepalive interval is very long (>1 hour), connections may timeout",
            ));
        }

        // Validate key path exists (warning only, as it may be created later)
        if let Some(ref key_path) = prefs.default_key_path {
            if key_path.is_empty() {
                errors.push(ValidationError::warning(
                    "user_preferences.default_key_path",
                    "Key path is empty",
                ));
            }
        }

        // Validate search history doesn't exceed max
        if prefs.search_history.len() > prefs.max_search_history {
            errors.push(ValidationError::warning(
                "user_preferences.search_history",
                "Search history exceeds configured maximum",
            ));
        }

        // Validate recent connections doesn't exceed max
        if prefs.recent_connections.len() > prefs.max_recent_connections {
            errors.push(ValidationError::warning(
                "user_preferences.recent_connections",
                "Recent connections exceeds configured maximum",
            ));
        }
    }

    /// Validate security settings
    pub fn validate_security_settings(
        settings: &SecuritySettings,
        errors: &mut Vec<ValidationError>,
    ) {
        // Validate clipboard clear time
        if settings.clipboard_clear_time > 600 {
            errors.push(ValidationError::warning(
                "security_settings.clipboard_clear_time",
                "Clipboard clear time is very long (>10 minutes), may be a security risk",
            ));
        }

        // Validate master password timeout
        if settings.master_password_timeout > 0 && settings.master_password_timeout < 5 {
            errors.push(ValidationError::warning(
                "security_settings.master_password_timeout",
                "Master password timeout is very short (<5 minutes), may be inconvenient",
            ));
        }

        // Validate auto-lock idle time
        if settings.auto_lock_after_idle > 0 && settings.auto_lock_after_idle < 1 {
            errors.push(ValidationError::warning(
                "security_settings.auto_lock_after_idle",
                "Auto-lock idle time is very short (<1 minute), may be inconvenient",
            ));
        }

        // Validate password length
        if settings.min_password_length < 6 {
            errors.push(ValidationError::warning(
                "security_settings.min_password_length",
                "Minimum password length is very short (<6 characters)",
            ));
        }
        if settings.min_password_length > 128 {
            errors.push(ValidationError::warning(
                "security_settings.min_password_length",
                "Minimum password length is very high (>128 characters), may be impractical",
            ));
        }

        // Check security feature combinations
        if settings.lock_on_blur && !settings.lock_on_sleep {
            errors.push(ValidationError::warning(
                "security_settings.lock_on_blur",
                "Lock on blur is enabled but lock on sleep is disabled, consider enabling both",
            ));
        }
    }

    /// Validate a specific value against a range
    pub fn validate_range<T: PartialOrd>(
        value: T,
        min: T,
        max: T,
        field: impl Into<String>,
        errors: &mut Vec<ValidationError>,
    ) where
        T: fmt::Display,
    {
        if value < min || value > max {
            errors.push(ValidationError::new(
                field,
                format!("Value {} is outside valid range [{}, {}]", value, min, max),
            ));
        }
    }

    /// Sanitize user preferences by trimming and limiting
    pub fn sanitize_user_preferences(prefs: &mut UserPreferences) {
        // Trim username
        prefs.default_username = prefs.default_username.trim().to_string();

        // Limit search history to max
        if prefs.search_history.len() > prefs.max_search_history {
            prefs.search_history.truncate(prefs.max_search_history);
        }

        // Limit recent connections to max
        if prefs.recent_connections.len() > prefs.max_recent_connections {
            prefs.recent_connections.truncate(prefs.max_recent_connections);
        }

        // Remove empty search history entries
        prefs.search_history.retain(|s| !s.trim().is_empty());

        // Remove empty recent connection entries
        prefs.recent_connections.retain(|s| !s.trim().is_empty());
    }

    /// Sanitize app config
    pub fn sanitize_app_config(config: &mut AppConfig) {
        // Trim terminal path
        config.default_terminal = config.default_terminal.trim().to_string();

        // Clamp sidebar width
        if config.sidebar_width < 50 {
            config.sidebar_width = 50;
        }
        if config.sidebar_width > 1000 {
            config.sidebar_width = 1000;
        }

        // Clamp window dimensions
        if config.window_geometry.width < 200 {
            config.window_geometry.width = 200;
        }
        if config.window_geometry.width > 4096 {
            config.window_geometry.width = 4096;
        }
        if config.window_geometry.height < 150 {
            config.window_geometry.height = 150;
        }
        if config.window_geometry.height > 4096 {
            config.window_geometry.height = 4096;
        }
    }

    /// Sanitize security settings
    pub fn sanitize_security_settings(settings: &mut SecuritySettings) {
        // Clamp clipboard clear time
        if settings.clipboard_clear_time > 3600 {
            settings.clipboard_clear_time = 3600;
        }

        // Clamp password length (max 255 for u8)
        if settings.min_password_length > 255 {
            settings.min_password_length = 255;
        }
    }

    /// Sanitize full configuration
    pub fn sanitize(config: &mut FullConfig) {
        Self::sanitize_app_config(&mut config.app_config);
        Self::sanitize_user_preferences(&mut config.user_preferences);
        Self::sanitize_security_settings(&mut config.security_settings);
    }
}

/// Auto-fix configuration issues where possible
pub struct ConfigAutoFix;

impl ConfigAutoFix {
    /// Attempt to fix common configuration issues
    pub fn auto_fix(config: &mut FullConfig) -> Vec<String> {
        let mut fixes = Vec::new();

        // Fix port if 0
        if config.user_preferences.default_port == 0 {
            config.user_preferences.default_port = 22;
            fixes.push("Set default port to 22 (was 0)".to_string());
        }

        // Fix search history size
        if config.user_preferences.search_history.len() > config.user_preferences.max_search_history
        {
            config.user_preferences.search_history.truncate(
                config.user_preferences.max_search_history,
            );
            fixes.push("Truncated search history to max limit".to_string());
        }

        // Fix recent connections size
        if config.user_preferences.recent_connections.len()
            > config.user_preferences.max_recent_connections
        {
            config.user_preferences.recent_connections.truncate(
                config.user_preferences.max_recent_connections,
            );
            fixes.push("Truncated recent connections to max limit".to_string());
        }

        // Fix sidebar width
        if config.app_config.sidebar_width < 100 {
            config.app_config.sidebar_width = 250;
            fixes.push("Set sidebar width to default (was too small)".to_string());
        }

        // Fix window dimensions if too small
        if config.app_config.window_geometry.width < 400 {
            config.app_config.window_geometry.width = 1280;
            fixes.push("Set window width to default (was too small)".to_string());
        }
        if config.app_config.window_geometry.height < 300 {
            config.app_config.window_geometry.height = 720;
            fixes.push("Set window height to default (was too small)".to_string());
        }

        // Fix empty terminal
        if config.app_config.default_terminal.is_empty() {
            #[cfg(target_os = "windows")]
            {
                config.app_config.default_terminal = "powershell.exe".to_string();
            }
            #[cfg(target_os = "macos")]
            {
                config.app_config.default_terminal = "Terminal.app".to_string();
            }
            #[cfg(target_os = "linux")]
            {
                config.app_config.default_terminal = "gnome-terminal".to_string();
            }
            fixes.push("Set default terminal to system default".to_string());
        }

        fixes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::*;

    #[test]
    fn test_validate_valid_config() {
        let config = FullConfig::default();
        assert!(ConfigValidator::validate(&config).is_ok());
    }

    #[test]
    fn test_validate_invalid_port() {
        let mut config = FullConfig::default();
        config.user_preferences.default_port = 0;
        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field.contains("default_port")));
    }

    #[test]
    fn test_validate_small_window() {
        let mut config = FullConfig::default();
        config.app_config.window_geometry.width = 100;
        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field.contains("window_geometry.width")));
    }

    #[test]
    fn test_sanitize_limits() {
        let mut config = FullConfig::default();
        config.app_config.sidebar_width = 2000;
        config.user_preferences.default_port = 0;

        ConfigValidator::sanitize(&mut config);

        assert_eq!(config.app_config.sidebar_width, 1000);
    }

    #[test]
    fn test_auto_fix() {
        let mut config = FullConfig::default();
        config.user_preferences.default_port = 0;
        config.app_config.window_geometry.width = 100;

        let fixes = ConfigAutoFix::auto_fix(&mut config);

        assert!(!fixes.is_empty());
        assert_eq!(config.user_preferences.default_port, 22);
        assert_eq!(config.app_config.window_geometry.width, 1280);
    }

    #[test]
    fn test_search_history_limits() {
        let mut config = FullConfig::default();
        config.user_preferences.max_search_history = 5;
        config.user_preferences.search_history = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string(),
            "f".to_string(),
        ];

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("exceeds configured maximum")));
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::new("field", "message");
        assert_eq!(format!("{}", error), "[ERROR] field: message");
    }

    #[test]
    fn test_validation_warning() {
        let warning = ValidationError::warning("field", "warning message");
        assert_eq!(warning.severity, ValidationSeverity::Warning);
        assert!(format!("{}", warning).contains("WARNING"));
    }
}
