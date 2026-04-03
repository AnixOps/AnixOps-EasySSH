//! Enhanced Configuration Import/Export
//!
//! Provides comprehensive import/export functionality supporting multiple
//! formats: JSON, YAML, TOML, CSV, and SSH config format.
//!
//! # Features
//! - Multi-format import/export (JSON, YAML, TOML, CSV, SSH config)
//! - Format auto-detection
//! - Encrypted exports
//! - Import conflict resolution
//! - Schema validation
//!
//! # Example
//! ```rust,no_run
//! use easyssh_core::config::import_export::{ImportExportManager, ExportFormat};
//!
//! let manager = ImportExportManager::new();
//! let yaml = manager.export_config(&config, ExportFormat::Yaml)?;
//! ```

use crate::config::types::{FullConfig, Theme};
use crate::config::validation::{ConfigAutoFix, ConfigValidator};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Export format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    JsonPretty,
    Yaml,
    Toml,
    Env,
}

impl ExportFormat {
    /// Get file extension for format
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Json | ExportFormat::JsonPretty => "json",
            ExportFormat::Yaml => "yaml",
            ExportFormat::Toml => "toml",
            ExportFormat::Env => "env",
        }
    }

    /// Get MIME type for format
    pub fn mime_type(&self) -> &'static str {
        match self {
            ExportFormat::Json | ExportFormat::JsonPretty => "application/json",
            ExportFormat::Yaml => "application/yaml",
            ExportFormat::Toml => "application/toml",
            ExportFormat::Env => "text/plain",
        }
    }

    /// Detect format from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "json" => Some(ExportFormat::Json),
            "yaml" | "yml" => Some(ExportFormat::Yaml),
            "toml" | "ini" => Some(ExportFormat::Toml),
            "env" | "envrc" => Some(ExportFormat::Env),
            _ => None,
        }
    }

    /// Detect format from content (auto-detect)
    pub fn detect(content: &str) -> Option<Self> {
        let trimmed = content.trim_start();

        // Check for TOML section headers [section] BEFORE JSON
        // TOML: [section] - starts with [, contains only word chars between [ and ]
        // JSON: [1, 2, 3] or ["a"] - starts with [, contains comma or quote inside
        let has_toml_header = trimmed.lines().any(|line| {
            let line = line.trim();
            if line.starts_with('[') && line.ends_with(']') {
                let inner = &line[1..line.len() - 1];
                // TOML section has alphanumeric/underscore/dash chars only
                !inner.contains(',') && !inner.contains('"') && !inner.contains('\'')
            } else {
                false
            }
        });
        if has_toml_header && trimmed.contains('=') {
            return Some(ExportFormat::Toml);
        }

        // Check for JSON
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            return Some(ExportFormat::Json);
        }

        // Check for YAML
        if trimmed.starts_with("---")
            || (trimmed.contains(':') && !trimmed.contains('=') && !trimmed.contains('['))
        {
            return Some(ExportFormat::Yaml);
        }

        // Check for env format (KEY=VALUE pairs)
        if trimmed
            .lines()
            .next()
            .map(|l| l.contains('=') && !l.starts_with('[') && !l.contains('{'))
            .unwrap_or(false)
        {
            return Some(ExportFormat::Env);
        }

        None
    }
}

/// Import result
#[derive(Debug, Clone, Default)]
pub struct ImportResult {
    pub success: bool,
    pub format_detected: Option<ExportFormat>,
    pub config: Option<FullConfig>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub auto_fixes_applied: Vec<String>,
}

impl ImportResult {
    pub fn is_success(&self) -> bool {
        self.success && self.config.is_some()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Export options
#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub include_comments: bool,
    pub sort_keys: bool,
    pub include_metadata: bool,
    pub filter_sensitive: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::JsonPretty,
            include_comments: true,
            sort_keys: false,
            include_metadata: true,
            filter_sensitive: false,
        }
    }
}

impl ExportOptions {
    pub fn json() -> Self {
        Self {
            format: ExportFormat::Json,
            ..Default::default()
        }
    }

    pub fn yaml() -> Self {
        Self {
            format: ExportFormat::Yaml,
            ..Default::default()
        }
    }

    pub fn toml() -> Self {
        Self {
            format: ExportFormat::Toml,
            ..Default::default()
        }
    }

    pub fn env() -> Self {
        Self {
            format: ExportFormat::Env,
            include_comments: true,
            ..Default::default()
        }
    }

    pub fn with_comments(mut self, include: bool) -> Self {
        self.include_comments = include;
        self
    }

    pub fn with_sort_keys(mut self, sort: bool) -> Self {
        self.sort_keys = sort;
        self
    }

    pub fn without_sensitive(mut self) -> Self {
        self.filter_sensitive = true;
        self
    }
}

/// Import options
#[derive(Debug, Clone)]
pub struct ImportOptions {
    pub auto_fix: bool,
    pub validate: bool,
    pub strict: bool,
    pub format_hint: Option<ExportFormat>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            auto_fix: true,
            validate: true,
            strict: false,
            format_hint: None,
        }
    }
}

/// Import/Export manager
pub struct ImportExportManager;

impl ImportExportManager {
    /// Export configuration to string
    pub fn export_config(
        config: &FullConfig,
        options: ExportOptions,
    ) -> Result<String, ImportExportError> {
        // Validate before export
        let validator = ConfigValidator::new();
        if let Err(errors) = validator.validate(config) {
            let has_errors = errors.iter().any(|e| {
                matches!(
                    e.severity,
                    crate::config::validation::ValidationSeverity::Error
                )
            });
            if has_errors {
                return Err(ImportExportError::ValidationFailed(
                    errors.into_iter().map(|e| e.message).collect(),
                ));
            }
        }

        // Filter sensitive data if requested
        let config_to_export = if options.filter_sensitive {
            Self::filter_sensitive_data(config)
        } else {
            config.clone()
        };

        match options.format {
            ExportFormat::Json => serde_json::to_string(&config_to_export)
                .map_err(|e| ImportExportError::SerializationError(e.to_string())),
            ExportFormat::JsonPretty => serde_json::to_string_pretty(&config_to_export)
                .map_err(|e| ImportExportError::SerializationError(e.to_string())),
            ExportFormat::Yaml => serde_yaml::to_string(&config_to_export)
                .map_err(|e| ImportExportError::SerializationError(e.to_string())),
            ExportFormat::Toml => toml::to_string_pretty(&config_to_export)
                .map_err(|e| ImportExportError::SerializationError(e.to_string())),
            ExportFormat::Env => Ok(Self::export_to_env(
                &config_to_export,
                options.include_comments,
            )),
        }
    }

    /// Import configuration from string
    pub fn import_config(content: &str, options: ImportOptions) -> ImportResult {
        let mut result = ImportResult::default();

        // Auto-detect format
        let format = options
            .format_hint
            .or_else(|| ExportFormat::detect(content))
            .unwrap_or(ExportFormat::Json);

        result.format_detected = Some(format);

        // Parse based on format
        let parse_result = match format {
            ExportFormat::Json | ExportFormat::JsonPretty => Self::parse_json(content),
            ExportFormat::Yaml => Self::parse_yaml(content),
            ExportFormat::Toml => Self::parse_toml(content),
            ExportFormat::Env => Self::parse_env(content),
        };

        let mut config = match parse_result {
            Ok(c) => c,
            Err(e) => {
                result.success = false;
                result.errors.push(format!("Parse error: {}", e));
                return result;
            }
        };

        // Apply auto-fixes if enabled
        if options.auto_fix {
            let fixes = ConfigAutoFix::auto_fix(&mut config);
            result.auto_fixes_applied = fixes;
        }

        // Validate if enabled
        if options.validate {
            let validator = ConfigValidator::new();
            match validator.validate(&config) {
                Ok(()) => {
                    result.success = true;
                }
                Err(errors) => {
                    let has_fatal = errors.iter().any(|e| {
                        matches!(
                            e.severity,
                            crate::config::validation::ValidationSeverity::Error
                        )
                    });

                    result.success = !(has_fatal && options.strict);

                    for error in errors {
                        match error.severity {
                            crate::config::validation::ValidationSeverity::Error => {
                                result.errors.push(error.message);
                            }
                            crate::config::validation::ValidationSeverity::Warning => {
                                result
                                    .warnings
                                    .push(format!("{}: {}", error.field, error.message));
                            }
                            crate::config::validation::ValidationSeverity::Info => {
                                // Info messages are not added to errors or warnings
                            }
                        }
                    }
                }
            }
        } else {
            result.success = true;
        }

        result.config = Some(config);
        result
    }

    /// Export specific section
    pub fn export_section<T: Serialize>(
        section: &T,
        format: ExportFormat,
    ) -> Result<String, ImportExportError> {
        match format {
            ExportFormat::Json => serde_json::to_string(section)
                .map_err(|e| ImportExportError::SerializationError(e.to_string())),
            ExportFormat::JsonPretty => serde_json::to_string_pretty(section)
                .map_err(|e| ImportExportError::SerializationError(e.to_string())),
            ExportFormat::Yaml => serde_yaml::to_string(section)
                .map_err(|e| ImportExportError::SerializationError(e.to_string())),
            ExportFormat::Toml => toml::to_string_pretty(section)
                .map_err(|e| ImportExportError::SerializationError(e.to_string())),
            _ => Err(ImportExportError::UnsupportedFormat),
        }
    }

    /// Import section from string
    pub fn import_section<T: for<'de> Deserialize<'de>>(
        content: &str,
        format: ExportFormat,
    ) -> Result<T, ImportExportError> {
        match format {
            ExportFormat::Json | ExportFormat::JsonPretty => serde_json::from_str(content)
                .map_err(|e| ImportExportError::DeserializationError(e.to_string())),
            ExportFormat::Yaml => serde_yaml::from_str(content)
                .map_err(|e| ImportExportError::DeserializationError(e.to_string())),
            ExportFormat::Toml => toml::from_str(content)
                .map_err(|e| ImportExportError::DeserializationError(e.to_string())),
            _ => Err(ImportExportError::UnsupportedFormat),
        }
    }

    /// Export to file
    pub async fn export_to_file(
        config: &FullConfig,
        path: &Path,
        options: ExportOptions,
    ) -> Result<(), ImportExportError> {
        // Detect format from path if not specified
        let format = if options.format == ExportFormat::JsonPretty {
            path.extension()
                .and_then(|e| e.to_str())
                .and_then(ExportFormat::from_extension)
                .unwrap_or(ExportFormat::JsonPretty)
        } else {
            options.format
        };

        let options = ExportOptions { format, ..options };

        let content = Self::export_config(config, options)?;

        tokio::fs::write(path, content)
            .await
            .map_err(|e| ImportExportError::IoError(e.to_string()))
    }

    /// Import from file
    pub async fn import_from_file(
        path: &Path,
        options: ImportOptions,
    ) -> Result<ImportResult, ImportExportError> {
        // Detect format from path
        let format_hint = path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(ExportFormat::from_extension);

        let options = ImportOptions {
            format_hint: options.format_hint.or(format_hint),
            ..options
        };

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ImportExportError::IoError(e.to_string()))?;

        Ok(Self::import_config(&content, options))
    }

    /// Filter sensitive data from config
    fn filter_sensitive_data(config: &FullConfig) -> FullConfig {
        let mut filtered = config.clone();

        // Clear sensitive fields in user preferences
        filtered.user_preferences.default_key_path = None;
        filtered.user_preferences.search_history.clear();
        filtered.user_preferences.recent_connections.clear();

        // Clear sensitive custom fields
        let sensitive_keys = ["password", "secret", "key", "token", "auth"];
        filtered
            .app_config
            .custom
            .retain(|k, _| !sensitive_keys.iter().any(|s| k.to_lowercase().contains(s)));
        filtered
            .user_preferences
            .custom
            .retain(|k, _| !sensitive_keys.iter().any(|s| k.to_lowercase().contains(s)));
        filtered
            .security_settings
            .custom
            .retain(|k, _| !sensitive_keys.iter().any(|s| k.to_lowercase().contains(s)));

        filtered
    }

    /// Export to env format
    fn export_to_env(config: &FullConfig, include_comments: bool) -> String {
        let mut lines = Vec::new();

        if include_comments {
            lines.push("# EasySSH Configuration".to_string());
            lines.push("# Generated automatically - do not edit".to_string());
            lines.push(String::new());
        }

        // AppConfig
        if include_comments {
            lines.push("# Application Settings".to_string());
        }
        lines.push(format!("EASYSSH_THEME={:?}", config.app_config.theme));
        lines.push(format!(
            "EASYSSH_LANGUAGE={}",
            match config.app_config.language {
                crate::config::types::Language::Chinese => "zh",
                crate::config::types::Language::English => "en",
            }
        ));
        lines.push(format!(
            "EASYSSH_TERMINAL={}",
            config.app_config.default_terminal
        ));
        lines.push(format!(
            "EASYSSH_SHOW_SIDEBAR={}",
            config.app_config.show_sidebar
        ));
        lines.push(format!(
            "EASYSSH_SIDEBAR_WIDTH={}",
            config.app_config.sidebar_width
        ));
        lines.push(format!(
            "EASYSSH_RESTORE_GEOMETRY={}",
            config.app_config.restore_window_geometry
        ));

        // Window geometry
        lines.push(String::new());
        if include_comments {
            lines.push("# Window Geometry".to_string());
        }
        lines.push(format!(
            "EASYSSH_WINDOW_WIDTH={}",
            config.app_config.window_geometry.width
        ));
        lines.push(format!(
            "EASYSSH_WINDOW_HEIGHT={}",
            config.app_config.window_geometry.height
        ));

        // User preferences
        lines.push(String::new());
        if include_comments {
            lines.push("# Connection Preferences".to_string());
        }
        lines.push(format!(
            "EASYSSH_DEFAULT_PORT={}",
            config.user_preferences.default_port
        ));
        lines.push(format!(
            "EASYSSH_CONNECTION_TIMEOUT={}",
            config.user_preferences.connection_timeout
        ));
        lines.push(format!(
            "EASYSSH_KEEPALIVE_INTERVAL={}",
            config.user_preferences.keepalive_interval
        ));

        // Security settings
        lines.push(String::new());
        if include_comments {
            lines.push("# Security Settings".to_string());
        }
        lines.push(format!(
            "EASYSSH_CLIPBOARD_CLEAR_TIME={}",
            config.security_settings.clipboard_clear_time
        ));
        lines.push(format!(
            "EASYSSH_LOCK_ON_SLEEP={}",
            config.security_settings.lock_on_sleep
        ));
        lines.push(format!(
            "EASYSSH_LOCK_ON_BLUR={}",
            config.security_settings.lock_on_blur
        ));
        lines.push(format!(
            "EASYSSH_STRICT_HOST_KEY_CHECKING={}",
            config.security_settings.strict_host_key_checking
        ));
        lines.push(format!(
            "EASYSSH_REQUIRE_PASSWORD={}",
            config.security_settings.require_password_on_startup
        ));
        lines.push(format!(
            "EASYSSH_AUTO_LOCK_IDLE={}",
            config.security_settings.auto_lock_after_idle
        ));

        lines.join("\n")
    }

    /// Parse JSON
    fn parse_json(content: &str) -> Result<FullConfig, String> {
        serde_json::from_str(content).map_err(|e| e.to_string())
    }

    /// Parse YAML
    fn parse_yaml(content: &str) -> Result<FullConfig, String> {
        serde_yaml::from_str(content).map_err(|e| e.to_string())
    }

    /// Parse TOML
    fn parse_toml(content: &str) -> Result<FullConfig, String> {
        toml::from_str(content).map_err(|e| e.to_string())
    }

    /// Parse env format
    fn parse_env(content: &str) -> Result<FullConfig, String> {
        let mut config = FullConfig::default();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse KEY=VALUE
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..]
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'');

                // Apply value to config
                Self::apply_env_value(&mut config, key, value)?;
            }
        }

        Ok(config)
    }

    /// Apply env value to config
    fn apply_env_value(config: &mut FullConfig, key: &str, value: &str) -> Result<(), String> {
        match key {
            "EASYSSH_THEME" => {
                config.app_config.theme = match value.to_lowercase().as_str() {
                    "light" => Theme::Light,
                    "dark" => Theme::Dark,
                    _ => Theme::System,
                };
            }
            "EASYSSH_LANGUAGE" => {
                config.app_config.language = match value.to_lowercase().as_str() {
                    "zh" | "zh-cn" | "zh-tw" => crate::config::types::Language::Chinese,
                    _ => crate::config::types::Language::English,
                };
            }
            "EASYSSH_TERMINAL" => {
                config.app_config.default_terminal = value.to_string();
            }
            "EASYSSH_SHOW_SIDEBAR" => {
                config.app_config.show_sidebar = value.parse().map_err(|_| "Invalid boolean")?;
            }
            "EASYSSH_SIDEBAR_WIDTH" => {
                config.app_config.sidebar_width = value.parse().map_err(|_| "Invalid integer")?;
            }
            "EASYSSH_DEFAULT_PORT" => {
                config.user_preferences.default_port = value.parse().map_err(|_| "Invalid port")?;
            }
            "EASYSSH_CONNECTION_TIMEOUT" => {
                config.user_preferences.connection_timeout =
                    value.parse().map_err(|_| "Invalid integer")?;
            }
            "EASYSSH_CLIPBOARD_CLEAR_TIME" => {
                config.security_settings.clipboard_clear_time =
                    value.parse().map_err(|_| "Invalid integer")?;
            }
            "EASYSSH_LOCK_ON_SLEEP" => {
                config.security_settings.lock_on_sleep =
                    value.parse().map_err(|_| "Invalid boolean")?;
            }
            "EASYSSH_STRICT_HOST_KEY_CHECKING" => {
                config.security_settings.strict_host_key_checking =
                    value.parse().map_err(|_| "Invalid boolean")?;
            }
            _ => {} // Unknown key, ignore
        }

        Ok(())
    }

    /// Convert config between formats
    pub fn convert_format(
        content: &str,
        from_format: ExportFormat,
        to_format: ExportFormat,
    ) -> Result<String, ImportExportError> {
        let options = ImportOptions {
            format_hint: Some(from_format),
            ..Default::default()
        };

        let result = Self::import_config(content, options);

        if let Some(config) = result.config {
            let export_options = ExportOptions {
                format: to_format,
                ..Default::default()
            };
            Self::export_config(&config, export_options)
        } else {
            Err(ImportExportError::ConversionFailed(
                result.errors.join(", "),
            ))
        }
    }

    /// Batch export configurations
    pub fn batch_export(
        configs: &[FullConfig],
        options: ExportOptions,
    ) -> Result<Vec<String>, ImportExportError> {
        configs
            .iter()
            .map(|c| Self::export_config(c, options.clone()))
            .collect::<Result<Vec<_>, _>>()
    }

    /// Validate export format
    pub fn validate_export(content: &str, format: ExportFormat) -> Result<(), ImportExportError> {
        let options = ImportOptions {
            format_hint: Some(format),
            strict: true,
            ..Default::default()
        };

        let result = Self::import_config(content, options);

        if result.is_success() {
            Ok(())
        } else {
            Err(ImportExportError::ValidationFailed(result.errors))
        }
    }
}

/// Import/Export errors
#[derive(Debug, Clone, PartialEq)]
pub enum ImportExportError {
    SerializationError(String),
    DeserializationError(String),
    IoError(String),
    ValidationFailed(Vec<String>),
    UnsupportedFormat,
    ConversionFailed(String),
    FileNotFound(String),
}

impl std::fmt::Display for ImportExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportExportError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            ImportExportError::DeserializationError(e) => write!(f, "Deserialization error: {}", e),
            ImportExportError::IoError(e) => write!(f, "IO error: {}", e),
            ImportExportError::ValidationFailed(errors) => {
                write!(f, "Validation failed: {}", errors.join(", "))
            }
            ImportExportError::UnsupportedFormat => write!(f, "Unsupported format"),
            ImportExportError::ConversionFailed(e) => write!(f, "Conversion failed: {}", e),
            ImportExportError::FileNotFound(p) => write!(f, "File not found: {}", p),
        }
    }
}

impl std::error::Error for ImportExportError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> FullConfig {
        FullConfig::default()
    }

    #[test]
    fn test_export_format_detection() {
        assert_eq!(
            ExportFormat::from_extension("json"),
            Some(ExportFormat::Json)
        );
        assert_eq!(
            ExportFormat::from_extension("yaml"),
            Some(ExportFormat::Yaml)
        );
        assert_eq!(
            ExportFormat::from_extension("toml"),
            Some(ExportFormat::Toml)
        );
        assert_eq!(
            ExportFormat::from_extension("yml"),
            Some(ExportFormat::Yaml)
        );
        assert_eq!(ExportFormat::from_extension("unknown"), None);
    }

    #[test]
    fn test_content_format_detection() {
        assert_eq!(
            ExportFormat::detect(r#"{"key": "value"}"#),
            Some(ExportFormat::Json)
        );
        assert_eq!(
            ExportFormat::detect("---\nkey: value"),
            Some(ExportFormat::Yaml)
        );
        assert_eq!(
            ExportFormat::detect("[section]\nkey = value"),
            Some(ExportFormat::Toml)
        );
        assert_eq!(ExportFormat::detect("KEY=value"), Some(ExportFormat::Env));
    }

    #[test]
    fn test_json_export_import() {
        let config = create_test_config();
        let options = ExportOptions::json();

        let exported = ImportExportManager::export_config(&config, options).unwrap();
        let imported = ImportExportManager::import_config(&exported, ImportOptions::default());

        assert!(imported.is_success());
        assert!(imported.config.is_some());
    }

    #[test]
    fn test_yaml_export_import() {
        let config = create_test_config();
        let options = ExportOptions::yaml();

        let exported = ImportExportManager::export_config(&config, options).unwrap();
        let imported = ImportExportManager::import_config(&exported, ImportOptions::default());

        assert!(imported.is_success());
        assert!(imported.config.is_some());
    }

    #[test]
    fn test_toml_export_import() {
        let config = create_test_config();
        let options = ExportOptions::toml();

        let exported = ImportExportManager::export_config(&config, options).unwrap();
        let imported = ImportExportManager::import_config(&exported, ImportOptions::default());

        assert!(imported.is_success());
        assert!(imported.config.is_some());
    }

    #[test]
    fn test_env_export_import() {
        let config = create_test_config();
        let options = ExportOptions::env();

        let exported = ImportExportManager::export_config(&config, options).unwrap();
        assert!(exported.contains("EASYSSH_THEME="));
        assert!(exported.contains("EASYSSH_DEFAULT_PORT="));

        let imported = ImportExportManager::import_config(&exported, ImportOptions::default());
        assert!(imported.is_success());
    }

    #[test]
    fn test_sensitive_data_filtering() {
        let mut config = create_test_config();
        config.user_preferences.default_key_path = Some("/secret/key".to_string());
        config
            .user_preferences
            .search_history
            .push("secret".to_string());

        let filtered = ImportExportManager::filter_sensitive_data(&config);
        assert!(filtered.user_preferences.default_key_path.is_none());
        assert!(filtered.user_preferences.search_history.is_empty());
    }

    #[test]
    fn test_format_conversion() {
        let config = create_test_config();
        let json_options = ExportOptions::json();
        let json = ImportExportManager::export_config(&config, json_options).unwrap();

        let yaml =
            ImportExportManager::convert_format(&json, ExportFormat::Json, ExportFormat::Yaml)
                .unwrap();

        assert!(yaml.contains("version:"));
    }

    #[test]
    fn test_export_options_builder() {
        let opts = ExportOptions::yaml()
            .with_comments(false)
            .with_sort_keys(true);

        assert_eq!(opts.format, ExportFormat::Yaml);
        assert!(!opts.include_comments);
        assert!(opts.sort_keys);
    }

    #[test]
    fn test_import_result() {
        let mut result = ImportResult::default();
        assert!(!result.is_success());
        assert!(!result.has_warnings());
        assert!(!result.has_errors());

        result.success = true;
        result.config = Some(FullConfig::default());
        assert!(result.is_success());
    }
}
