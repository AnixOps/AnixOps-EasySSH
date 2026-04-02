//! Enhanced Environment Variable Support
//!
//! Provides comprehensive environment variable integration for configuration
//! with support for type conversion, validation, and secure handling.
//!
//! # Features
//! - Automatic type conversion from environment strings
//! - Secure handling of sensitive values (passwords, keys)
//! - Environment variable validation
//! - Hierarchical overrides (env -> file -> defaults)
//! - Conditional environment loading based on profile
//!
//! # Example
//! ```rust,no_run
//! use easyssh_core::config::env::{EnvConfig, EnvProfile};
//!
//! let env_config = EnvConfig::new()
//!     .with_profile(EnvProfile::Development)
//!     .load();
//!
//! let config = env_config.apply_to(FullConfig::default());
//! ```

use crate::config::types::{AppConfig, FullConfig, Language, SecuritySettings, Theme, UserPreferences};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

/// Environment configuration loader
#[derive(Debug, Clone)]
pub struct EnvConfig {
    /// Environment profile
    pub profile: EnvProfile,
    /// Loaded environment variables
    pub variables: HashMap<String, EnvVariable>,
    /// Whether to include sensitive variables
    pub include_sensitive: bool,
    /// Validation strictness
    pub strict_mode: bool,
    /// Prefix for environment variables
    pub prefix: String,
}

/// Environment profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvProfile {
    /// Development environment
    Development,
    /// Testing environment
    #[default]
    Testing,
    /// Staging environment
    Staging,
    /// Production environment
    Production,
    /// CI/CD environment
    Ci,
    /// Custom environment
    Custom,
}

impl EnvProfile {
    /// Get profile from environment variable
    pub fn from_env() -> Self {
        if let Ok(profile) = env::var("EASYSSH_ENV") {
            match profile.to_lowercase().as_str() {
                "dev" | "development" => EnvProfile::Development,
                "test" | "testing" => EnvProfile::Testing,
                "stage" | "staging" => EnvProfile::Staging,
                "prod" | "production" => EnvProfile::Production,
                "ci" => EnvProfile::Ci,
                _ => EnvProfile::Custom,
            }
        } else {
            EnvProfile::default()
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            EnvProfile::Development => "Development",
            EnvProfile::Testing => "Testing",
            EnvProfile::Staging => "Staging",
            EnvProfile::Production => "Production",
            EnvProfile::Ci => "CI/CD",
            EnvProfile::Custom => "Custom",
        }
    }

    /// Check if this is a production-like environment
    pub fn is_production_like(&self) -> bool {
        matches!(self, EnvProfile::Staging | EnvProfile::Production)
    }

    /// Check if strict validation should be applied
    pub fn requires_strict_validation(&self) -> bool {
        matches!(self, EnvProfile::Staging | EnvProfile::Production)
    }
}

/// Environment variable with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVariable {
    /// Variable name
    pub name: String,
    /// Variable value
    pub value: String,
    /// Variable source
    pub source: EnvSource,
    /// Whether this is a sensitive value
    pub is_sensitive: bool,
    /// Variable type hint
    pub type_hint: EnvType,
    /// Validation status
    pub validated: bool,
    /// Error message if validation failed
    pub error: Option<String>,
}

/// Environment variable source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvSource {
    /// From actual environment variable
    Environment,
    /// From .env file
    DotEnv,
    /// From configuration file
    ConfigFile,
    /// Default value
    Default,
    /// User input
    UserInput,
}

/// Environment variable type hints
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvType {
    String,
    Integer,
    Boolean,
    Float,
    Path,
    Url,
    Port,
    IpAddress,
    Json,
    Secret,
}

impl Default for EnvType {
    fn default() -> Self {
        EnvType::String
    }
}

/// Environment variable definitions
#[derive(Debug, Clone)]
pub struct EnvDefinitions;

impl EnvDefinitions {
    /// Get all supported environment variables
    pub fn all() -> Vec<EnvVarDefinition> {
        vec![
            // Theme and appearance
            EnvVarDefinition::new("EASYSSH_THEME", EnvType::String, false)
                .with_description("Application theme: light, dark, or system")
                .with_allowed_values(&["light", "dark", "system"]),
            EnvVarDefinition::new("EASYSSH_LANGUAGE", EnvType::String, false)
                .with_description("Interface language: en, zh")
                .with_allowed_values(&["en", "zh", "zh-cn", "zh-tw"]),

            // Terminal settings
            EnvVarDefinition::new("EASYSSH_TERMINAL", EnvType::Path, false)
                .with_description("Default terminal emulator path"),
            EnvVarDefinition::new("EASYSSH_TERMINAL_ARGS", EnvType::String, false)
                .with_description("Default arguments for terminal emulator"),

            // Connection defaults
            EnvVarDefinition::new("EASYSSH_DEFAULT_PORT", EnvType::Port, false)
                .with_description("Default SSH port")
                .with_min(1)
                .with_max(65535),
            EnvVarDefinition::new("EASYSSH_DEFAULT_USER", EnvType::String, false)
                .with_description("Default SSH username"),
            EnvVarDefinition::new("EASYSSH_DEFAULT_KEY_PATH", EnvType::Path, false)
                .with_description("Default SSH key path"),
            EnvVarDefinition::new("EASYSSH_CONNECTION_TIMEOUT", EnvType::Integer, false)
                .with_description("Connection timeout in seconds")
                .with_min(1)
                .with_max(3600),
            EnvVarDefinition::new("EASYSSH_KEEPALIVE_INTERVAL", EnvType::Integer, false)
                .with_description("SSH keepalive interval in seconds")
                .with_min(0)
                .with_max(3600),

            // UI preferences
            EnvVarDefinition::new("EASYSSH_SIDEBAR_WIDTH", EnvType::Integer, false)
                .with_description("Sidebar width in pixels")
                .with_min(100)
                .with_max(800),
            EnvVarDefinition::new("EASYSSH_SHOW_SIDEBAR", EnvType::Boolean, false)
                .with_description("Show sidebar by default: true or false"),
            EnvVarDefinition::new("EASYSSH_RESTORE_GEOMETRY", EnvType::Boolean, false)
                .with_description("Restore window geometry on startup"),
            EnvVarDefinition::new("EASYSSH_WINDOW_WIDTH", EnvType::Integer, false)
                .with_min(400)
                .with_max(4096),
            EnvVarDefinition::new("EASYSSH_WINDOW_HEIGHT", EnvType::Integer, false)
                .with_min(300)
                .with_max(4096),

            // Security settings
            EnvVarDefinition::new("EASYSSH_CLIPBOARD_CLEAR_TIME", EnvType::Integer, false)
                .with_description("Clipboard clear time in seconds")
                .with_min(0)
                .with_max(3600),
            EnvVarDefinition::new("EASYSSH_LOCK_ON_SLEEP", EnvType::Boolean, false)
                .with_description("Lock application on sleep"),
            EnvVarDefinition::new("EASYSSH_LOCK_ON_BLUR", EnvType::Boolean, false)
                .with_description("Lock application when losing focus"),
            EnvVarDefinition::new("EASYSSH_STRICT_HOST_KEY_CHECKING", EnvType::Boolean, false)
                .with_description("Strict SSH host key checking"),
            EnvVarDefinition::new("EASYSSH_AUTO_LOCK_IDLE", EnvType::Integer, false)
                .with_description("Auto-lock after idle minutes (0 to disable)"),
            EnvVarDefinition::new("EASYSSH_REQUIRE_PASSWORD", EnvType::Boolean, false)
                .with_description("Require password on startup"),

            // Sensitive variables (passwords, keys)
            EnvVarDefinition::new("EASYSSH_MASTER_PASSWORD", EnvType::Secret, true)
                .with_description("Master password (use with caution)"),
            EnvVarDefinition::new("EASYSSH_SSH_KEY_PASSPHRASE", EnvType::Secret, true)
                .with_description("SSH key passphrase"),

            // Debug/Development
            EnvVarDefinition::new("EASYSSH_DEBUG", EnvType::Boolean, false)
                .with_description("Enable debug mode"),
            EnvVarDefinition::new("EASYSSH_LOG_LEVEL", EnvType::String, false)
                .with_description("Log level: trace, debug, info, warn, error")
                .with_allowed_values(&["trace", "debug", "info", "warn", "error"]),
            EnvVarDefinition::new("EASYSSH_CONFIG_PATH", EnvType::Path, false)
                .with_description("Custom configuration file path"),
            EnvVarDefinition::new("EASYSSH_DATA_DIR", EnvType::Path, false)
                .with_description("Custom data directory path"),

            // Profile-specific
            EnvVarDefinition::new("EASYSSH_PROFILE", EnvType::String, false)
                .with_description("Active environment profile")
                .with_allowed_values(&["development", "testing", "staging", "production", "ci"]),
        ]
    }

    /// Find definition by name
    pub fn find(name: &str) -> Option<EnvVarDefinition> {
        Self::all().into_iter().find(|d| d.name == name)
    }
}

/// Environment variable definition
#[derive(Debug, Clone)]
pub struct EnvVarDefinition {
    pub name: String,
    pub var_type: EnvType,
    pub is_sensitive: bool,
    pub description: Option<String>,
    pub allowed_values: Option<Vec<String>>,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
    pub default_value: Option<String>,
}

impl EnvVarDefinition {
    pub fn new(name: &str, var_type: EnvType, is_sensitive: bool) -> Self {
        Self {
            name: name.to_string(),
            var_type,
            is_sensitive,
            description: None,
            allowed_values: None,
            min_value: None,
            max_value: None,
            default_value: None,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn with_allowed_values(mut self, values: &[&str]) -> Self {
        self.allowed_values = Some(values.iter().map(|v| v.to_string()).collect());
        self
    }

    pub fn with_min(mut self, min: i64) -> Self {
        self.min_value = Some(min);
        self
    }

    pub fn with_max(mut self, max: i64) -> Self {
        self.max_value = Some(max);
        self
    }

    pub fn with_default(mut self, default: &str) -> Self {
        self.default_value = Some(default.to_string());
        self
    }

    /// Validate a value against this definition
    pub fn validate(&self, value: &str) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check allowed values
        if let Some(ref allowed) = self.allowed_values {
            if !allowed.iter().any(|a| a.eq_ignore_ascii_case(value)) {
                errors.push(format!(
                    "Value '{}' not in allowed values: {:?}",
                    value, allowed
                ));
            }
        }

        // Type-specific validation
        match self.var_type {
            EnvType::Integer | EnvType::Port => {
                if let Ok(num) = value.parse::<i64>() {
                    if let Some(min) = self.min_value {
                        if num < min {
                            errors.push(format!("Value {} is below minimum {}", num, min));
                        }
                    }
                    if let Some(max) = self.max_value {
                        if num > max {
                            errors.push(format!("Value {} is above maximum {}", num, max));
                        }
                    }
                    if self.var_type == EnvType::Port {
                        if num < 1 || num > 65535 {
                            errors.push("Port must be between 1 and 65535".to_string());
                        }
                    }
                } else {
                    errors.push(format!("Value '{}' is not a valid integer", value));
                }
            }
            EnvType::Boolean => {
                if !matches!(
                    value.to_lowercase().as_str(),
                    "true" | "false" | "1" | "0" | "yes" | "no" | "on" | "off"
                ) {
                    errors.push(format!("Value '{}' is not a valid boolean", value));
                }
            }
            EnvType::Float => {
                if value.parse::<f64>().is_err() {
                    errors.push(format!("Value '{}' is not a valid float", value));
                }
            }
            EnvType::Path => {
                // Basic path validation - just check it's not empty
                if value.is_empty() {
                    errors.push("Path cannot be empty".to_string());
                }
            }
            EnvType::Url => {
                if !value.starts_with("http://") && !value.starts_with("https://") && !value.starts_with("ssh://") {
                    errors.push("URL must start with http://, https://, or ssh://".to_string());
                }
            }
            EnvType::IpAddress => {
                // Basic IP validation
                if !is_valid_ip(value) {
                    errors.push(format!("Value '{}' is not a valid IP address", value));
                }
            }
            EnvType::Json => {
                if serde_json::from_str::<serde_json::Value>(value).is_err() {
                    errors.push(format!("Value '{}' is not valid JSON", value));
                }
            }
            _ => {} // String and Secret don't need validation
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Parse a value according to the type
    pub fn parse_value(&self, value: &str) -> Result<EnvValue, String> {
        match self.var_type {
            EnvType::Integer | EnvType::Port => {
                value.parse::<i64>()
                    .map(EnvValue::Integer)
                    .map_err(|e| e.to_string())
            }
            EnvType::Boolean => {
                let bool_val = matches!(
                    value.to_lowercase().as_str(),
                    "true" | "1" | "yes" | "on"
                );
                Ok(EnvValue::Boolean(bool_val))
            }
            EnvType::Float => {
                value.parse::<f64>()
                    .map(EnvValue::Float)
                    .map_err(|e| e.to_string())
            }
            EnvType::Json => {
                serde_json::from_str::<serde_json::Value>(value)
                    .map(EnvValue::Json)
                    .map_err(|e| e.to_string())
            }
            _ => Ok(EnvValue::String(value.to_string())),
        }
    }
}

/// Parsed environment value
#[derive(Debug, Clone)]
pub enum EnvValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    Float(f64),
    Json(serde_json::Value),
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvConfig {
    /// Create a new environment configuration
    pub fn new() -> Self {
        Self {
            profile: EnvProfile::default(),
            variables: HashMap::new(),
            include_sensitive: false,
            strict_mode: false,
            prefix: "EASYSSH_".to_string(),
        }
    }

    /// Set environment profile
    pub fn with_profile(mut self, profile: EnvProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Set whether to include sensitive variables
    pub fn with_sensitive(mut self, include: bool) -> Self {
        self.include_sensitive = include;
        self
    }

    /// Set strict mode
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Set variable prefix
    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    /// Load environment variables
    pub fn load(&mut self) -> Result<&mut Self, Vec<String>> {
        self.variables.clear();
        let mut errors = Vec::new();

        for definition in EnvDefinitions::all() {
            if definition.is_sensitive && !self.include_sensitive {
                continue;
            }

            if let Ok(value) = env::var(&definition.name) {
                // Validate if strict mode
                if self.strict_mode || self.profile.requires_strict_validation() {
                    if let Err(e) = definition.validate(&value) {
                        errors.extend(e);
                        continue;
                    }
                }

                let variable = EnvVariable {
                    name: definition.name.clone(),
                    value: value.clone(),
                    source: EnvSource::Environment,
                    is_sensitive: definition.is_sensitive,
                    type_hint: definition.var_type,
                    validated: true,
                    error: None,
                };

                self.variables.insert(definition.name.clone(), variable);
            }
        }

        if errors.is_empty() {
            Ok(self)
        } else {
            Err(errors)
        }
    }

    /// Load from .env file
    pub async fn load_dotenv(&mut self, path: &std::path::Path) -> Result<usize, String> {
        if !path.exists() {
            return Ok(0);
        }

        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| format!("Failed to read .env file: {}", e))?;

        let mut count = 0;

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse KEY=VALUE
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..].trim();

                // Remove quotes if present
                let value = value.trim_matches('"').trim_matches('\'');

                if let Some(definition) = EnvDefinitions::find(key) {
                    if definition.is_sensitive && !self.include_sensitive {
                        continue;
                    }

                    let variable = EnvVariable {
                        name: key.to_string(),
                        value: value.to_string(),
                        source: EnvSource::DotEnv,
                        is_sensitive: definition.is_sensitive,
                        type_hint: definition.var_type,
                        validated: false, // Will be validated when used
                        error: None,
                    };

                    self.variables.insert(key.to_string(), variable);
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Apply loaded environment variables to a configuration
    pub fn apply_to(&self, mut config: FullConfig) -> FullConfig {
        // Apply theme
        if let Some(theme_var) = self.get("EASYSSH_THEME") {
            config.app_config.theme = match theme_var.value.to_lowercase().as_str() {
                "light" => Theme::Light,
                "dark" => Theme::Dark,
                _ => Theme::System,
            };
        }

        // Apply language
        if let Some(lang_var) = self.get("EASYSSH_LANGUAGE") {
            config.app_config.language = match lang_var.value.to_lowercase().as_str() {
                "zh" | "zh-cn" | "zh-tw" => Language::Chinese,
                _ => Language::English,
            };
        }

        // Apply terminal
        if let Some(term_var) = self.get("EASYSSH_TERMINAL") {
            config.app_config.default_terminal = term_var.value.clone();
        }

        // Apply connection settings
        if let Some(port_var) = self.get("EASYSSH_DEFAULT_PORT") {
            if let Ok(port) = port_var.value.parse::<u16>() {
                config.user_preferences.default_port = port;
            }
        }

        if let Some(user_var) = self.get("EASYSSH_DEFAULT_USER") {
            config.user_preferences.default_username = user_var.value.clone();
        }

        if let Some(timeout_var) = self.get("EASYSSH_CONNECTION_TIMEOUT") {
            if let Ok(timeout) = timeout_var.value.parse::<u64>() {
                config.user_preferences.connection_timeout = timeout;
            }
        }

        if let Some(keepalive_var) = self.get("EASYSSH_KEEPALIVE_INTERVAL") {
            if let Ok(interval) = keepalive_var.value.parse::<u64>() {
                config.user_preferences.keepalive_interval = interval;
            }
        }

        // Apply UI settings
        if let Some(width_var) = self.get("EASYSSH_SIDEBAR_WIDTH") {
            if let Ok(width) = width_var.value.parse::<u32>() {
                config.app_config.sidebar_width = width.clamp(100, 800);
            }
        }

        if let Some(show_var) = self.get("EASYSSH_SHOW_SIDEBAR") {
            config.app_config.show_sidebar = parse_bool(&show_var.value);
        }

        // Apply window geometry
        if let Some(width_var) = self.get("EASYSSH_WINDOW_WIDTH") {
            if let Ok(width) = width_var.value.parse::<u32>() {
                config.app_config.window_geometry.width = width.clamp(400, 4096);
            }
        }

        if let Some(height_var) = self.get("EASYSSH_WINDOW_HEIGHT") {
            if let Ok(height) = height_var.value.parse::<u32>() {
                config.app_config.window_geometry.height = height.clamp(300, 4096);
            }
        }

        // Apply security settings
        if let Some(clipboard_var) = self.get("EASYSSH_CLIPBOARD_CLEAR_TIME") {
            if let Ok(time) = clipboard_var.value.parse::<u32>() {
                config.security_settings.clipboard_clear_time = time.clamp(0, 3600);
            }
        }

        if let Some(lock_sleep_var) = self.get("EASYSSH_LOCK_ON_SLEEP") {
            config.security_settings.lock_on_sleep = parse_bool(&lock_sleep_var.value);
        }

        if let Some(lock_blur_var) = self.get("EASYSSH_LOCK_ON_BLUR") {
            config.security_settings.lock_on_blur = parse_bool(&lock_blur_var.value);
        }

        if let Some(strict_var) = self.get("EASYSSH_STRICT_HOST_KEY_CHECKING") {
            config.security_settings.strict_host_key_checking = parse_bool(&strict_var.value);
        }

        if let Some(auto_lock_var) = self.get("EASYSSH_AUTO_LOCK_IDLE") {
            if let Ok(minutes) = auto_lock_var.value.parse::<u32>() {
                config.security_settings.auto_lock_after_idle = minutes;
            }
        }

        if let Some(require_pass_var) = self.get("EASYSSH_REQUIRE_PASSWORD") {
            config.security_settings.require_password_on_startup = parse_bool(&require_pass_var.value);
        }

        config
    }

    /// Get a specific environment variable
    pub fn get(&self, name: &str) -> Option<&EnvVariable> {
        self.variables.get(name)
    }

    /// Get all loaded variables
    pub fn get_all(&self) -> &HashMap<String, EnvVariable> {
        &self.variables
    }

    /// Get non-sensitive variables for display
    pub fn get_displayable(&self) -> Vec<&EnvVariable> {
        self.variables.values()
            .filter(|v| !v.is_sensitive)
            .collect()
    }

    /// Validate all loaded variables
    pub fn validate_all(&mut self) -> Vec<(String, Vec<String>)> {
        let mut results = Vec::new();

        for (name, variable) in &mut self.variables {
            if let Some(definition) = EnvDefinitions::find(name) {
                match definition.validate(&variable.value) {
                    Ok(()) => {
                        variable.validated = true;
                        variable.error = None;
                    }
                    Err(errors) => {
                        variable.validated = false;
                        variable.error = Some(errors.join(", "));
                        results.push((name.clone(), errors));
                    }
                }
            }
        }

        results
    }

    /// Check if specific variable is set
    pub fn is_set(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    /// Generate environment file content
    pub fn generate_env_file(&self) -> String {
        let mut content = String::new();
        content.push_str("# EasySSH Environment Configuration\n");
        content.push_str(&format!("# Generated at: {}\n", chrono_now()));
        content.push_str(&format!("# Profile: {}\n", self.profile.display_name()));
        content.push_str("#\n\n");

        for (name, variable) in &self.variables {
            if let Some(definition) = EnvDefinitions::find(name) {
                if let Some(ref desc) = definition.description {
                    content.push_str(&format!("# {}\n", desc));
                }

                if variable.is_sensitive {
                    content.push_str(&format!("# {}={}\n", name, mask_secret(&variable.value)));
                } else {
                    content.push_str(&format!("{}={}\n", name, variable.value));
                }

                content.push('\n');
            }
        }

        content
    }
}

/// Parse boolean from various string formats
fn parse_bool(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "true" | "1" | "yes" | "on" | "enabled"
    )
}

/// Mask a secret value
fn mask_secret(value: &str) -> String {
    if value.len() <= 4 {
        "****".to_string()
    } else {
        format!("{}****", &value[..2])
    }
}

/// Basic IP address validation
fn is_valid_ip(ip: &str) -> bool {
    // IPv4 validation
    if ip.split('.').count() == 4 {
        ip.split('.').all(|part| {
            part.parse::<u8>().is_ok()
        })
    } else {
        // IPv6 validation is complex, just check for colons
        ip.contains(':')
    }
}

/// Generate current timestamp
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_profile_from_env() {
        // Test default
        env::remove_var("EASYSSH_ENV");
        assert_eq!(EnvProfile::from_env(), EnvProfile::Testing);

        // Test various values
        env::set_var("EASYSSH_ENV", "development");
        assert_eq!(EnvProfile::from_env(), EnvProfile::Development);

        env::set_var("EASYSSH_ENV", "prod");
        assert_eq!(EnvProfile::from_env(), EnvProfile::Production);

        env::remove_var("EASYSSH_ENV");
    }

    #[test]
    fn test_env_profile_checks() {
        assert!(!EnvProfile::Development.is_production_like());
        assert!(EnvProfile::Production.is_production_like());
        assert!(EnvProfile::Production.requires_strict_validation());
    }

    #[test]
    fn test_env_var_definition_validation() {
        let def = EnvVarDefinition::new("TEST", EnvType::Integer, false)
            .with_min(0)
            .with_max(100);

        assert!(def.validate("50").is_ok());
        assert!(def.validate("-1").is_err());
        assert!(def.validate("101").is_err());
        assert!(def.validate("abc").is_err());
    }

    #[test]
    fn test_port_validation() {
        let def = EnvVarDefinition::new("PORT", EnvType::Port, false);

        assert!(def.validate("22").is_ok());
        assert!(def.validate("65535").is_ok());
        assert!(def.validate("0").is_err());
        assert!(def.validate("70000").is_err());
    }

    #[test]
    fn test_boolean_validation() {
        let def = EnvVarDefinition::new("FLAG", EnvType::Boolean, false);

        assert!(def.validate("true").is_ok());
        assert!(def.validate("false").is_ok());
        assert!(def.validate("1").is_ok());
        assert!(def.validate("yes").is_ok());
        assert!(def.validate("invalid").is_err());
    }

    #[test]
    fn test_allowed_values_validation() {
        let def = EnvVarDefinition::new("LEVEL", EnvType::String, false)
            .with_allowed_values(&["low", "medium", "high"]);

        assert!(def.validate("medium").is_ok());
        assert!(def.validate("MEDIUM").is_ok());
        assert!(def.validate("critical").is_err());
    }

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("true"));
        assert!(parse_bool("TRUE"));
        assert!(parse_bool("1"));
        assert!(parse_bool("yes"));
        assert!(!parse_bool("false"));
        assert!(!parse_bool("0"));
        assert!(!parse_bool("no"));
    }

    #[test]
    fn test_mask_secret() {
        assert_eq!(mask_secret("secret"), "se****");
        assert_eq!(mask_secret("ab"), "****");
    }

    #[test]
    fn test_ip_validation() {
        assert!(is_valid_ip("192.168.1.1"));
        assert!(is_valid_ip("10.0.0.1"));
        assert!(!is_valid_ip("256.1.1.1"));
        assert!(!is_valid_ip("192.168.1"));
    }

    #[test]
    fn test_env_definitions() {
        let defs = EnvDefinitions::all();
        assert!(!defs.is_empty());

        let theme_def = EnvDefinitions::find("EASYSSH_THEME");
        assert!(theme_def.is_some());
        assert_eq!(theme_def.unwrap().var_type, EnvType::String);
    }
}
