//! Configuration Validation
//!
//! Provides comprehensive validation for configuration values to ensure data integrity
//! and prevent invalid configurations from being saved.
//!
//! # Features
//! - Full configuration validation
//! - Section-level validation
//! - Field-level validation
//! - Custom validation rules
//! - Auto-fix capabilities
//! - Validation warnings and errors

use super::types::{AppConfig, FullConfig, SecuritySettings, UserPreferences, WindowGeometry};
use std::fmt;

/// Current configuration schema version for validation
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

/// Validation error details
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub severity: ValidationSeverity,
    pub code: ValidationErrorCode,
}

/// Validation error codes for programmatic handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ValidationErrorCode {
    Required,
    InvalidType,
    OutOfRange,
    InvalidFormat,
    InvalidValue,
    TooShort,
    TooLong,
    PatternMismatch,
    DependencyViolation,
    SecurityRisk,
    Deprecated,
    #[default]
    Unknown,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: ValidationSeverity::Error,
            code: ValidationErrorCode::Unknown,
        }
    }

    /// Create a warning-level validation issue
    pub fn warning(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: ValidationSeverity::Warning,
            code: ValidationErrorCode::Unknown,
        }
    }

    /// Create with specific error code
    pub fn with_code(mut self, code: ValidationErrorCode) -> Self {
        self.code = code;
        self
    }

    /// Create a required field error
    pub fn required(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: "This field is required".to_string(),
            severity: ValidationSeverity::Error,
            code: ValidationErrorCode::Required,
        }
    }

    /// Create an out of range error
    pub fn out_of_range(
        field: impl Into<String>,
        value: impl fmt::Display,
        min: impl fmt::Display,
        max: impl fmt::Display,
    ) -> Self {
        Self {
            field: field.into(),
            message: format!("Value {} is outside valid range [{}, {}]", value, min, max),
            severity: ValidationSeverity::Error,
            code: ValidationErrorCode::OutOfRange,
        }
    }

    /// Create an invalid format error
    pub fn invalid_format(field: impl Into<String>, expected: &str) -> Self {
        Self {
            field: field.into(),
            message: format!("Invalid format, expected: {}", expected),
            severity: ValidationSeverity::Error,
            code: ValidationErrorCode::InvalidFormat,
        }
    }

    /// Create a security risk warning
    pub fn security_risk(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: ValidationSeverity::Error,
            code: ValidationErrorCode::SecurityRisk,
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
    Info,
    Warning,
    Error,
}

impl fmt::Display for ValidationSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationSeverity::Info => write!(f, "INFO"),
            ValidationSeverity::Warning => write!(f, "WARNING"),
            ValidationSeverity::Error => write!(f, "ERROR"),
        }
    }
}

/// Validation result type
pub type ValidationResult = std::result::Result<(), Vec<ValidationError>>;

/// Configuration validation context
#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub strict_mode: bool,
    pub security_level: SecurityValidationLevel,
    pub edition: crate::edition::Edition,
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self {
            strict_mode: false,
            security_level: SecurityValidationLevel::Standard,
            edition: crate::edition::Edition::Lite,
        }
    }
}

/// Security validation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityValidationLevel {
    /// Minimal security checks
    Minimal,
    /// Standard security checks
    Standard,
    /// High security checks
    High,
    /// Maximum security checks (enterprise)
    Maximum,
}

/// Configuration validator with context
pub struct ConfigValidator {
    context: ValidationContext,
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigValidator {
    /// Create a new validator with default context
    pub fn new() -> Self {
        Self {
            context: ValidationContext::default(),
        }
    }

    /// Create with custom context
    pub fn with_context(context: ValidationContext) -> Self {
        Self { context }
    }

    /// Set strict mode
    pub fn strict_mode(mut self, strict: bool) -> Self {
        self.context.strict_mode = strict;
        self
    }

    /// Set security level
    pub fn security_level(mut self, level: SecurityValidationLevel) -> Self {
        self.context.security_level = level;
        self
    }

    /// Validate the full configuration
    pub fn validate(&self, config: &FullConfig) -> ValidationResult {
        let mut errors = Vec::new();

        // Validate schema version
        self.validate_schema_version(config, &mut errors);

        // Validate each section
        Self::validate_app_config(&config.app_config, &mut errors, &self.context);
        Self::validate_user_preferences(&config.user_preferences, &mut errors, &self.context);
        Self::validate_security_settings(&config.security_settings, &mut errors, &self.context);

        // Validate cross-section consistency
        self.validate_consistency(config, &mut errors);

        // Only fail on Error-severity issues, not warnings
        let has_errors = errors.iter().any(|e| e.severity == ValidationSeverity::Error);
        if has_errors {
            Err(errors)
        } else {
            Ok(())
        }
    }

    /// Validate schema version
    fn validate_schema_version(&self, config: &FullConfig, errors: &mut Vec<ValidationError>) {
        if config.version > CONFIG_SCHEMA_VERSION {
            errors.push(ValidationError::warning(
                "version",
                format!(
                    "Configuration version {} is newer than supported schema version {}. Some features may not work correctly.",
                    config.version, CONFIG_SCHEMA_VERSION
                ),
            ).with_code(ValidationErrorCode::Deprecated));
        }
    }

    /// Validate cross-section consistency
    fn validate_consistency(&self, config: &FullConfig, errors: &mut Vec<ValidationError>) {
        // Check that theme and language are consistent with available options
        if !matches!(
            config.app_config.theme,
            super::types::Theme::Light | super::types::Theme::Dark | super::types::Theme::System
        ) {
            errors.push(
                ValidationError::new("app_config.theme", "Invalid theme value")
                    .with_code(ValidationErrorCode::InvalidValue),
            );
        }

        // Check security setting consistency
        if config.security_settings.require_password_on_startup
            && config.security_settings.master_password_timeout == 0
        {
            errors.push(ValidationError::info(
                "security_settings",
                "Password on startup is enabled but timeout is 0 (never). Consider setting a timeout."
            ));
        }

        // Check for insecure combinations in high security mode
        if self.context.security_level >= SecurityValidationLevel::High {
            if !config.security_settings.strict_host_key_checking {
                errors.push(ValidationError::security_risk(
                    "security_settings.strict_host_key_checking",
                    "Strict host key checking should be enabled in high security mode",
                ));
            }

            if config.security_settings.clipboard_clear_time > 60 {
                errors.push(ValidationError::security_risk(
                    "security_settings.clipboard_clear_time",
                    "Clipboard clear time should be 60 seconds or less in high security mode",
                ));
            }
        }
    }

    /// Validate app configuration
    pub fn validate_app_config(
        config: &AppConfig,
        errors: &mut Vec<ValidationError>,
        context: &ValidationContext,
    ) {
        // Validate window geometry
        Self::validate_window_geometry(&config.window_geometry, errors, context);

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
            errors.push(
                ValidationError::new(
                    "app_config.shortcuts.new_connection",
                    "Shortcut key cannot be empty",
                )
                .with_code(ValidationErrorCode::Required),
            );
        }

        // Validate terminal setting
        if config.default_terminal.is_empty() {
            errors.push(ValidationError::warning(
                "app_config.default_terminal",
                "Default terminal is not set",
            ));
        }

        // Check for suspicious terminal path (potential security issue)
        if !config.default_terminal.is_empty() {
            let term_lower = config.default_terminal.to_lowercase();
            if term_lower.contains("tmp")
                || term_lower.contains("temp")
                || term_lower.contains("/var/")
            {
                errors.push(ValidationError::security_risk(
                    "app_config.default_terminal",
                    "Terminal path appears to be in a temporary directory, verify this is intentional",
                ));
            }
        }
    }

    /// Validate window geometry
    fn validate_window_geometry(
        geometry: &WindowGeometry,
        errors: &mut Vec<ValidationError>,
        _context: &ValidationContext,
    ) {
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

        // Check for minimum viable window size
        if geometry.width < 200 || geometry.height < 150 {
            errors.push(
                ValidationError::new(
                    "app_config.window_geometry",
                    "Window dimensions are too small to be usable",
                )
                .with_code(ValidationErrorCode::OutOfRange),
            );
        }
    }

    /// Validate user preferences
    pub fn validate_user_preferences(
        prefs: &UserPreferences,
        errors: &mut Vec<ValidationError>,
        context: &ValidationContext,
    ) {
        // Validate port range
        if prefs.default_port == 0 {
            errors.push(
                ValidationError::new("user_preferences.default_port", "Default port cannot be 0")
                    .with_code(ValidationErrorCode::OutOfRange),
            );
        }
        if prefs.default_port < 1024 {
            errors.push(ValidationError::warning(
                "user_preferences.default_port",
                "Default port is in the well-known port range (<1024), may require elevated privileges",
            ));
        }

        // Validate search history limits
        if prefs.max_search_history == 0 {
            errors.push(ValidationError::info(
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

            // Security check: key path permissions
            if context.security_level >= SecurityValidationLevel::High
                && (key_path.starts_with("/tmp/") || key_path.starts_with("C:\\Temp\\"))
            {
                errors.push(ValidationError::security_risk(
                    "user_preferences.default_key_path",
                    "SSH key should not be stored in a temporary directory",
                ));
            }
        }

        // Validate search history doesn't exceed max
        if prefs.search_history.len() > prefs.max_search_history {
            errors.push(ValidationError::new(
                "user_preferences.search_history",
                format!(
                    "Search history has {} items but max is {}",
                    prefs.search_history.len(),
                    prefs.max_search_history
                ),
            ));
        }

        // Validate recent connections doesn't exceed max
        if prefs.recent_connections.len() > prefs.max_recent_connections {
            errors.push(ValidationError::new(
                "user_preferences.recent_connections",
                format!(
                    "Recent connections has {} items but max is {}",
                    prefs.recent_connections.len(),
                    prefs.max_recent_connections
                ),
            ));
        }

        // Validate default username
        if prefs.default_username.contains(' ') {
            errors.push(
                ValidationError::new(
                    "user_preferences.default_username",
                    "Username cannot contain spaces",
                )
                .with_code(ValidationErrorCode::InvalidFormat),
            );
        }
    }

    /// Validate security settings
    pub fn validate_security_settings(
        settings: &SecuritySettings,
        errors: &mut Vec<ValidationError>,
        context: &ValidationContext,
    ) {
        // Validate clipboard clear time
        if settings.clipboard_clear_time > 600 {
            errors.push(ValidationError::security_risk(
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
            errors.push(ValidationError::info(
                "security_settings.lock_on_blur",
                "Lock on blur is enabled but lock on sleep is disabled, consider enabling both",
            ));
        }

        // High security validations
        if context.security_level >= SecurityValidationLevel::High {
            if settings.min_password_length < 8 {
                errors.push(ValidationError::security_risk(
                    "security_settings.min_password_length",
                    "Minimum password length should be at least 8 in high security mode",
                ));
            }

            if !settings.audit_sensitive_ops {
                errors.push(ValidationError::security_risk(
                    "security_settings.audit_sensitive_ops",
                    "Audit should be enabled in high security mode",
                ));
            }
        }

        // Maximum security validations
        if context.security_level >= SecurityValidationLevel::Maximum {
            if !settings.require_password_on_startup {
                errors.push(ValidationError::security_risk(
                    "security_settings.require_password_on_startup",
                    "Password on startup is required in maximum security mode",
                ));
            }

            if settings.auto_lock_after_idle == 0 {
                errors.push(ValidationError::security_risk(
                    "security_settings.auto_lock_after_idle",
                    "Auto-lock after idle is required in maximum security mode",
                ));
            }
        }
    }

    /// Validate a specific value against a range
    pub fn validate_range<T>(
        value: T,
        min: T,
        max: T,
        field: impl Into<String>,
        errors: &mut Vec<ValidationError>,
    ) where
        T: PartialOrd + fmt::Display,
    {
        if value < min || value > max {
            errors.push(ValidationError::out_of_range(field, value, min, max));
        }
    }

    /// Validate string pattern
    pub fn validate_pattern(
        value: &str,
        pattern: &regex::Regex,
        field: impl Into<String>,
        errors: &mut Vec<ValidationError>,
    ) {
        if !pattern.is_match(value) {
            errors.push(
                ValidationError::new(field, "Value does not match required pattern".to_string())
                    .with_code(ValidationErrorCode::PatternMismatch),
            );
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
            prefs
                .recent_connections
                .truncate(prefs.max_recent_connections);
        }

        // Remove empty search history entries
        prefs.search_history.retain(|s| !s.trim().is_empty());

        // Remove empty recent connection entries
        prefs.recent_connections.retain(|s| !s.trim().is_empty());

        // Normalize port
        if prefs.default_port == 0 {
            prefs.default_port = 22;
        }

        // Trim key path
        if let Some(ref mut key_path) = prefs.default_key_path {
            *key_path = key_path.trim().to_string();
        }
    }

    /// Sanitize app config
    pub fn sanitize_app_config(config: &mut AppConfig) {
        // Trim terminal path
        config.default_terminal = config.default_terminal.trim().to_string();

        // Clamp sidebar width
        config.sidebar_width = config.sidebar_width.clamp(50, 1000);

        // Clamp window dimensions
        config.window_geometry.width = config.window_geometry.width.clamp(200, 4096);
        config.window_geometry.height = config.window_geometry.height.clamp(150, 4096);

        // Ensure window position is reasonable
        const MAX_POS: i32 = 10000;
        config.window_geometry.x = config.window_geometry.x.clamp(-MAX_POS, MAX_POS);
        config.window_geometry.y = config.window_geometry.y.clamp(-MAX_POS, MAX_POS);
    }

    /// Sanitize security settings
    pub fn sanitize_security_settings(settings: &mut SecuritySettings) {
        // Clamp clipboard clear time
        settings.clipboard_clear_time = settings.clipboard_clear_time.min(3600);

        // Clamp password length (max 255 for u8) - always true since u8 max is 255
        // This check is kept for documentation purposes

        // Normalize timeouts
        if settings.master_password_timeout > 0 && settings.master_password_timeout < 1 {
            settings.master_password_timeout = 1;
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
            config
                .user_preferences
                .search_history
                .truncate(config.user_preferences.max_search_history);
            fixes.push("Truncated search history to max limit".to_string());
        }

        // Fix recent connections size
        if config.user_preferences.recent_connections.len()
            > config.user_preferences.max_recent_connections
        {
            config
                .user_preferences
                .recent_connections
                .truncate(config.user_preferences.max_recent_connections);
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
            config.app_config.default_terminal = super::defaults::get_system_default_terminal();
            fixes.push("Set default terminal to system default".to_string());
        }

        // Fix password length if too short but not zero
        if config.security_settings.min_password_length > 0
            && config.security_settings.min_password_length < 6
        {
            config.security_settings.min_password_length = 8;
            fixes.push("Set minimum password length to 8 (was too low)".to_string());
        }

        // Fix clipboard clear time if too long
        if config.security_settings.clipboard_clear_time > 3600 {
            config.security_settings.clipboard_clear_time = 300;
            fixes.push("Set clipboard clear time to 5 minutes (was too long)".to_string());
        }

        // Fix empty strings in lists
        let before_count = config.user_preferences.search_history.len();
        config
            .user_preferences
            .search_history
            .retain(|s| !s.trim().is_empty());
        if config.user_preferences.search_history.len() < before_count {
            fixes.push("Removed empty entries from search history".to_string());
        }

        fixes
    }

    /// Fix a specific field
    pub fn fix_field(config: &mut FullConfig, field: &str) -> Option<String> {
        match field {
            "user_preferences.default_port" => {
                if config.user_preferences.default_port == 0 {
                    config.user_preferences.default_port = 22;
                    Some("Fixed default port to 22".to_string())
                } else {
                    None
                }
            }
            "app_config.sidebar_width" => {
                if config.app_config.sidebar_width < 100 {
                    config.app_config.sidebar_width = 250;
                    Some("Fixed sidebar width to 250".to_string())
                } else if config.app_config.sidebar_width > 800 {
                    config.app_config.sidebar_width = 300;
                    Some("Fixed sidebar width to 300 (was too large)".to_string())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Validation helper methods for ValidationError
impl ValidationError {
    /// Create an info-level validation message
    pub fn info(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: ValidationSeverity::Info,
            code: ValidationErrorCode::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::*;
    use super::*;

    #[test]
    fn test_validate_valid_config() {
        let config = FullConfig::default();
        let validator = ConfigValidator::new();
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_validate_invalid_port() {
        let mut config = FullConfig::default();
        config.user_preferences.default_port = 0;
        let validator = ConfigValidator::new();
        let result = validator.validate(&config);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field.contains("default_port")));
        assert!(errors
            .iter()
            .any(|e| e.code == ValidationErrorCode::OutOfRange));
    }

    #[test]
    fn test_validate_small_window() {
        let mut config = FullConfig::default();
        config.app_config.window_geometry.width = 100;
        let validator = ConfigValidator::new();
        let result = validator.validate(&config);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field.contains("window_geometry.width")));
    }

    #[test]
    fn test_validate_high_security_mode() {
        let mut config = FullConfig::default();
        config.security_settings.min_password_length = 6;
        config.security_settings.strict_host_key_checking = false;

        let validator = ConfigValidator::new().security_level(SecurityValidationLevel::High);

        let result = validator.validate(&config);
        let errors = result.unwrap_err();

        assert!(errors
            .iter()
            .any(|e| e.code == ValidationErrorCode::SecurityRisk));
    }

    #[test]
    fn test_validate_schema_version() {
        let mut config = FullConfig::default();
        config.version = 999; // Future version

        let validator = ConfigValidator::new();
        let result = validator.validate(&config);
        // Should produce a warning but not an error
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitize_limits() {
        let mut config = FullConfig::default();
        config.app_config.sidebar_width = 2000;
        config.user_preferences.default_port = 0;

        ConfigValidator::sanitize(&mut config);

        assert_eq!(config.app_config.sidebar_width, 1000);
        assert_eq!(config.user_preferences.default_port, 22);
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

        let validator = ConfigValidator::new();
        let result = validator.validate(&config);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("has ") && e.message.contains("items but max is")));
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

    #[test]
    fn test_validation_with_code() {
        let error =
            ValidationError::new("field", "message").with_code(ValidationErrorCode::Required);
        assert_eq!(error.code, ValidationErrorCode::Required);
    }

    #[test]
    fn test_validation_context() {
        let context = ValidationContext {
            strict_mode: true,
            security_level: SecurityValidationLevel::High,
            edition: crate::edition::Edition::Standard,
        };

        let validator = ConfigValidator::with_context(context);
        let ctx = &validator.context;
        assert!(ctx.strict_mode);
        assert_eq!(ctx.security_level, SecurityValidationLevel::High);
    }

    #[test]
    fn test_security_risk_validation() {
        let mut errors = Vec::new();
        let error = ValidationError::security_risk("field", "security issue");
        errors.push(error);

        assert_eq!(errors[0].severity, ValidationSeverity::Error);
        assert_eq!(errors[0].code, ValidationErrorCode::SecurityRisk);
    }

    #[test]
    fn test_out_of_range_error() {
        let error = ValidationError::out_of_range("port", 70000, 1, 65535);
        assert!(error.message.contains("70000"));
        assert!(error.message.contains("65535"));
        assert_eq!(error.code, ValidationErrorCode::OutOfRange);
    }

    #[test]
    fn test_invalid_format_error() {
        let error = ValidationError::invalid_format("email", "user@example.com");
        assert!(error.message.contains("user@example.com"));
        assert_eq!(error.code, ValidationErrorCode::InvalidFormat);
    }

    #[test]
    fn test_required_error() {
        let error = ValidationError::required("username");
        assert_eq!(error.field, "username");
        assert_eq!(error.code, ValidationErrorCode::Required);
    }

    #[test]
    fn test_info_level() {
        let info = ValidationError::info("field", "info message");
        assert_eq!(info.severity, ValidationSeverity::Info);
        assert!(format!("{}", info).contains("INFO"));
    }

    #[test]
    fn test_auto_fix_field() {
        let mut config = FullConfig::default();
        config.user_preferences.default_port = 0;

        let result = ConfigAutoFix::fix_field(&mut config, "user_preferences.default_port");
        assert!(result.is_some());
        assert_eq!(config.user_preferences.default_port, 22);

        let no_fix = ConfigAutoFix::fix_field(&mut config, "unknown_field");
        assert!(no_fix.is_none());
    }

    #[test]
    fn test_maximum_security_validations() {
        let mut config = FullConfig::default();
        config.security_settings.require_password_on_startup = false;
        config.security_settings.auto_lock_after_idle = 0;

        let validator = ConfigValidator::new().security_level(SecurityValidationLevel::Maximum);

        let result = validator.validate(&config);
        let errors = result.unwrap_err();

        assert!(errors.iter().any(|e| {
            e.field == "security_settings.require_password_on_startup"
                && e.code == ValidationErrorCode::SecurityRisk
        }));
    }

    #[test]
    fn test_validate_range_helper() {
        let mut errors = Vec::new();
        ConfigValidator::validate_range(50, 0, 100, "field", &mut errors);
        assert!(errors.is_empty());

        ConfigValidator::validate_range(150, 0, 100, "field", &mut errors);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].code, ValidationErrorCode::OutOfRange);
    }

    #[test]
    fn test_consistency_validation() {
        let mut config = FullConfig::default();
        config.security_settings.require_password_on_startup = true;
        config.security_settings.master_password_timeout = 0;

        let validator = ConfigValidator::new();
        let result = validator.validate(&config);
        // Should produce info but not fail
        assert!(result.is_ok());
    }

    #[test]
    fn test_username_validation() {
        let mut config = FullConfig::default();
        config.user_preferences.default_username = "user name".to_string();

        let validator = ConfigValidator::new();
        let result = validator.validate(&config);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| { e.field.contains("username") && e.message.contains("spaces") }));
    }

    #[test]
    fn test_sanitize_removes_empty_history() {
        let mut config = FullConfig::default();
        config.user_preferences.search_history = vec![
            "valid".to_string(),
            "".to_string(),
            "   ".to_string(),
            "also valid".to_string(),
        ];

        ConfigValidator::sanitize(&mut config);

        assert_eq!(config.user_preferences.search_history.len(), 2);
        assert_eq!(config.user_preferences.search_history[0], "valid");
        assert_eq!(config.user_preferences.search_history[1], "also valid");
    }
}
