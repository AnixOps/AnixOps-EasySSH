//! Configuration Templates
//!
//! Provides a template system for creating configuration presets,
//! user-defined templates, and environment-specific configurations.
//!
//! # Features
//! - Built-in templates for common use cases
//! - User-defined custom templates
//! - Environment-specific configurations (dev/staging/prod)
//! - Template inheritance and composition
//!
//! # Example
//! ```rust,no_run
//! use easyssh_core::config::templates::{TemplateManager, Template};
//!
//! let mut manager = TemplateManager::new();
//! manager.load_builtin_templates();
//!
//! let dev_template = manager.get_template("development").unwrap();
//! let config = dev_template.apply();
//! ```

use crate::config::types::{AppConfig, FullConfig, SecuritySettings, Theme, UserPreferences, WindowGeometry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Template identifier
pub type TemplateId = String;

/// Configuration template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: TemplateId,
    pub name: String,
    pub description: String,
    pub category: TemplateCategory,
    pub version: String,
    pub author: Option<String>,
    pub parent: Option<TemplateId>,
    pub overrides: ConfigOverrides,
    pub tags: Vec<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Template category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateCategory {
    BuiltIn,
    UserDefined,
    Environment,
    Team,
    Industry,
}

/// Configuration overrides (delta from defaults)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_terminal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidebar_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_sidebar: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clipboard_clear_time: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_on_sleep: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_host_key_checking: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_app_config: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_user_prefs: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_security: Option<HashMap<String, serde_json::Value>>,
}

/// Template manager
pub struct TemplateManager {
    templates: HashMap<TemplateId, Template>,
    user_templates_dir: Option<std::path::PathBuf>,
}

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateManager {
    /// Create a new template manager
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            user_templates_dir: None,
        }
    }

    /// Create with user templates directory
    pub fn with_user_dir(user_dir: std::path::PathBuf) -> Self {
        Self {
            templates: HashMap::new(),
            user_templates_dir: Some(user_dir),
        }
    }

    /// Load all built-in templates
    pub fn load_builtin_templates(&mut self) {
        let builtins = Self::get_builtin_templates();
        for template in builtins {
            self.templates.insert(template.id.clone(), template);
        }
    }

    /// Get a template by ID
    pub fn get_template(&self, id: &str) -> Option<&Template> {
        self.templates.get(id)
    }

    /// Get mutable reference to template
    pub fn get_template_mut(&mut self, id: &str) -> Option<&mut Template> {
        self.templates.get_mut(id)
    }

    /// Add or update a template
    pub fn add_template(&mut self, template: Template) {
        self.templates.insert(template.id.clone(), template);
    }

    /// Remove a template (only user-defined templates can be removed)
    pub fn remove_template(&mut self, id: &str) -> Result<(), TemplateError> {
        if let Some(template) = self.templates.get(id) {
            if template.category == TemplateCategory::BuiltIn {
                return Err(TemplateError::CannotRemoveBuiltin(id.to_string()));
            }
        }
        self.templates.remove(id);
        Ok(())
    }

    /// List all templates
    pub fn list_templates(&self) -> Vec<&Template> {
        self.templates.values().collect()
    }

    /// List templates by category
    pub fn list_by_category(&self, category: TemplateCategory) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|t| t.category == category)
            .collect()
    }

    /// Apply a template to create a FullConfig
    pub fn apply_template(&self, template_id: &str) -> Result<FullConfig, TemplateError> {
        let template = self.templates.get(template_id)
            .ok_or_else(|| TemplateError::TemplateNotFound(template_id.to_string()))?;

        let mut config = FullConfig::default();
        self.apply_template_recursive(&mut config, template_id)?;

        Ok(config)
    }

    /// Apply template recursively (handle parent templates)
    fn apply_template_recursive(
        &self,
        config: &mut FullConfig,
        template_id: &str,
    ) -> Result<(), TemplateError> {
        let template = self.templates.get(template_id)
            .ok_or_else(|| TemplateError::TemplateNotFound(template_id.to_string()))?;

        // Apply parent first (if any)
        if let Some(ref parent_id) = template.parent {
            self.apply_template_recursive(config, parent_id)?;
        }

        // Apply this template's overrides
        self.apply_overrides(config, &template.overrides);

        Ok(())
    }

    /// Apply overrides to a config
    fn apply_overrides(&self, config: &mut FullConfig, overrides: &ConfigOverrides) {
        if let Some(theme) = overrides.theme {
            config.app_config.theme = theme;
        }
        if let Some(ref lang) = overrides.language {
            config.app_config.language = match lang.as_str() {
                "zh" | "zh-cn" | "zh-tw" => crate::config::types::Language::Chinese,
                _ => crate::config::types::Language::English,
            };
        }
        if let Some(ref terminal) = overrides.default_terminal {
            config.app_config.default_terminal = terminal.clone();
        }
        if let Some(width) = overrides.sidebar_width {
            config.app_config.sidebar_width = width;
        }
        if let Some(show) = overrides.show_sidebar {
            config.app_config.show_sidebar = show;
        }
        if let Some(port) = overrides.default_port {
            config.user_preferences.default_port = port;
        }
        if let Some(timeout) = overrides.connection_timeout {
            config.user_preferences.connection_timeout = timeout;
        }
        if let Some(clear_time) = overrides.clipboard_clear_time {
            config.security_settings.clipboard_clear_time = clear_time;
        }
        if let Some(lock) = overrides.lock_on_sleep {
            config.security_settings.lock_on_sleep = lock;
        }
        if let Some(strict) = overrides.strict_host_key_checking {
            config.security_settings.strict_host_key_checking = strict;
        }

        // Apply custom fields
        if let Some(ref custom) = overrides.custom_app_config {
            config.app_config.custom.extend(custom.clone());
        }
        if let Some(ref custom) = overrides.custom_user_prefs {
            config.user_preferences.custom.extend(custom.clone());
        }
        if let Some(ref custom) = overrides.custom_security {
            config.security_settings.custom.extend(custom.clone());
        }
    }

    /// Create a new user-defined template
    pub fn create_template(
        &mut self,
        id: &str,
        name: &str,
        description: &str,
        category: TemplateCategory,
        base_config: &FullConfig,
    ) -> Result<Template, TemplateError> {
        // Calculate overrides from base config
        let overrides = Self::calculate_overrides(base_config);

        let template = Template {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            category,
            version: "1.0".to_string(),
            author: None,
            parent: None,
            overrides,
            tags: vec![],
            created_at: Some(chrono_now()),
            updated_at: Some(chrono_now()),
        };

        self.add_template(template.clone());
        Ok(template)
    }

    /// Calculate overrides from a full config (compared to defaults)
    fn calculate_overrides(config: &FullConfig) -> ConfigOverrides {
        let defaults = FullConfig::default();
        let mut overrides = ConfigOverrides::default();

        if config.app_config.theme != defaults.app_config.theme {
            overrides.theme = Some(config.app_config.theme);
        }
        if config.app_config.sidebar_width != defaults.app_config.sidebar_width {
            overrides.sidebar_width = Some(config.app_config.sidebar_width);
        }
        if config.app_config.show_sidebar != defaults.app_config.show_sidebar {
            overrides.show_sidebar = Some(config.app_config.show_sidebar);
        }
        if config.user_preferences.default_port != defaults.user_preferences.default_port {
            overrides.default_port = Some(config.user_preferences.default_port);
        }
        if config.user_preferences.connection_timeout != defaults.user_preferences.connection_timeout {
            overrides.connection_timeout = Some(config.user_preferences.connection_timeout);
        }
        if config.security_settings.clipboard_clear_time != defaults.security_settings.clipboard_clear_time {
            overrides.clipboard_clear_time = Some(config.security_settings.clipboard_clear_time);
        }
        if config.security_settings.lock_on_sleep != defaults.security_settings.lock_on_sleep {
            overrides.lock_on_sleep = Some(config.security_settings.lock_on_sleep);
        }
        if config.security_settings.strict_host_key_checking != defaults.security_settings.strict_host_key_checking {
            overrides.strict_host_key_checking = Some(config.security_settings.strict_host_key_checking);
        }

        overrides
    }

    /// Save user templates to disk
    pub async fn save_user_templates(&self) -> Result<(), TemplateError> {
        if let Some(ref dir) = self.user_templates_dir {
            tokio::fs::create_dir_all(dir).await
                .map_err(|e| TemplateError::SaveError(e.to_string()))?;

            for template in self.templates.values() {
                if template.category == TemplateCategory::UserDefined {
                    let path = dir.join(format!("{}.json", template.id));
                    let json = serde_json::to_string_pretty(template)
                        .map_err(|e| TemplateError::SerializationError(e.to_string()))?;
                    tokio::fs::write(path, json).await
                        .map_err(|e| TemplateError::SaveError(e.to_string()))?;
                }
            }
        }
        Ok(())
    }

    /// Load user templates from disk
    pub async fn load_user_templates(&mut self) -> Result<usize, TemplateError> {
        let mut count = 0;

        if let Some(ref dir) = self.user_templates_dir {
            if !dir.exists() {
                return Ok(0);
            }

            let mut entries = tokio::fs::read_dir(dir).await
                .map_err(|e| TemplateError::LoadError(e.to_string()))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| TemplateError::LoadError(e.to_string()))? {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    let content = tokio::fs::read_to_string(&path).await
                        .map_err(|e| TemplateError::LoadError(e.to_string()))?;
                    let template: Template = serde_json::from_str(&content)
                        .map_err(|e| TemplateError::DeserializationError(e.to_string()))?;
                    self.add_template(template);
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Get all built-in templates
    pub fn get_builtin_templates() -> Vec<Template> {
        vec![
            Self::minimal_template(),
            Self::balanced_template(),
            Self::rich_template(),
            Self::enterprise_template(),
            Self::development_template(),
            Self::production_template(),
        ]
    }

    fn minimal_template() -> Template {
        Template {
            id: "minimal".to_string(),
            name: "Minimal".to_string(),
            description: "Streamlined configuration for power users with minimal UI".to_string(),
            category: TemplateCategory::BuiltIn,
            version: "1.0".to_string(),
            author: Some("EasySSH".to_string()),
            parent: None,
            overrides: ConfigOverrides {
                theme: None,
                language: None,
                default_terminal: None,
                sidebar_width: Some(200),
                show_sidebar: Some(false),
                default_port: None,
                connection_timeout: None,
                clipboard_clear_time: Some(10),
                lock_on_sleep: Some(true),
                strict_host_key_checking: Some(true),
                custom_app_config: None,
                custom_user_prefs: None,
                custom_security: None,
            },
            tags: vec!["minimal".to_string(), "power-user".to_string()],
            created_at: None,
            updated_at: None,
        }
    }

    fn balanced_template() -> Template {
        Template {
            id: "balanced".to_string(),
            name: "Balanced".to_string(),
            description: "Balanced configuration suitable for most users".to_string(),
            category: TemplateCategory::BuiltIn,
            version: "1.0".to_string(),
            author: Some("EasySSH".to_string()),
            parent: None,
            overrides: ConfigOverrides::default(),
            tags: vec!["balanced".to_string(), "default".to_string()],
            created_at: None,
            updated_at: None,
        }
    }

    fn rich_template() -> Template {
        Template {
            id: "rich".to_string(),
            name: "Rich".to_string(),
            description: "Feature-rich configuration with all conveniences enabled".to_string(),
            category: TemplateCategory::BuiltIn,
            version: "1.0".to_string(),
            author: Some("EasySSH".to_string()),
            parent: None,
            overrides: ConfigOverrides {
                theme: None,
                language: None,
                default_terminal: None,
                sidebar_width: Some(300),
                show_sidebar: Some(true),
                default_port: None,
                connection_timeout: None,
                clipboard_clear_time: Some(60),
                lock_on_sleep: Some(true),
                strict_host_key_checking: None,
                custom_app_config: None,
                custom_user_prefs: None,
                custom_security: None,
            },
            tags: vec!["rich".to_string(), "beginner".to_string()],
            created_at: None,
            updated_at: None,
        }
    }

    fn enterprise_template() -> Template {
        Template {
            id: "enterprise".to_string(),
            name: "Enterprise".to_string(),
            description: "Enterprise-grade security with strict compliance settings".to_string(),
            category: TemplateCategory::BuiltIn,
            version: "1.0".to_string(),
            author: Some("EasySSH".to_string()),
            parent: None,
            overrides: ConfigOverrides {
                theme: None,
                language: None,
                default_terminal: None,
                sidebar_width: None,
                show_sidebar: Some(true),
                default_port: None,
                connection_timeout: None,
                clipboard_clear_time: Some(15),
                lock_on_sleep: Some(true),
                strict_host_key_checking: Some(true),
                custom_app_config: None,
                custom_user_prefs: None,
                custom_security: Some({
                    let mut map = HashMap::new();
                    map.insert("audit_mode".to_string(), serde_json::json!(true));
                    map.insert("require_password_change_90_days".to_string(), serde_json::json!(true));
                    map
                }),
            },
            tags: vec!["enterprise".to_string(), "security".to_string(), "compliance".to_string()],
            created_at: None,
            updated_at: None,
        }
    }

    fn development_template() -> Template {
        Template {
            id: "development".to_string(),
            name: "Development".to_string(),
            description: "Optimized for development environments with convenient defaults".to_string(),
            category: TemplateCategory::BuiltIn,
            version: "1.0".to_string(),
            author: Some("EasySSH".to_string()),
            parent: Some("balanced".to_string()),
            overrides: ConfigOverrides {
                theme: Some(Theme::Dark),
                language: None,
                default_terminal: None,
                sidebar_width: Some(280),
                show_sidebar: Some(true),
                default_port: Some(22),
                connection_timeout: Some(60),
                clipboard_clear_time: None,
                lock_on_sleep: Some(false),
                strict_host_key_checking: Some(false),
                custom_app_config: Some({
                    let mut map = HashMap::new();
                    map.insert("dev_mode".to_string(), serde_json::json!(true));
                    map.insert("auto_reconnect".to_string(), serde_json::json!(true));
                    map
                }),
                custom_user_prefs: None,
                custom_security: None,
            },
            tags: vec!["development".to_string(), "dev".to_string()],
            created_at: None,
            updated_at: None,
        }
    }

    fn production_template() -> Template {
        Template {
            id: "production".to_string(),
            name: "Production".to_string(),
            description: "Secure configuration optimized for production environments".to_string(),
            category: TemplateCategory::BuiltIn,
            version: "1.0".to_string(),
            author: Some("EasySSH".to_string()),
            parent: Some("enterprise".to_string()),
            overrides: ConfigOverrides {
                theme: None,
                language: None,
                default_terminal: None,
                sidebar_width: None,
                show_sidebar: Some(true),
                default_port: None,
                connection_timeout: Some(10),
                clipboard_clear_time: Some(5),
                lock_on_sleep: Some(true),
                strict_host_key_checking: Some(true),
                custom_app_config: None,
                custom_user_prefs: None,
                custom_security: Some({
                    let mut map = HashMap::new();
                    map.insert("session_timeout_minutes".to_string(), serde_json::json!(30));
                    map.insert("require_2fa".to_string(), serde_json::json!(true));
                    map.insert("allow_password_auth".to_string(), serde_json::json!(false));
                    map
                }),
            },
            tags: vec!["production".to_string(), "prod".to_string(), "security".to_string()],
            created_at: None,
            updated_at: None,
        }
    }
}

/// Template-related errors
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateError {
    TemplateNotFound(String),
    CannotRemoveBuiltin(String),
    SaveError(String),
    LoadError(String),
    SerializationError(String),
    DeserializationError(String),
    CircularReference(String),
    InvalidTemplate(String),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::TemplateNotFound(id) => write!(f, "Template not found: {}", id),
            TemplateError::CannotRemoveBuiltin(id) => write!(f, "Cannot remove built-in template: {}", id),
            TemplateError::SaveError(e) => write!(f, "Failed to save template: {}", e),
            TemplateError::LoadError(e) => write!(f, "Failed to load template: {}", e),
            TemplateError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            TemplateError::DeserializationError(e) => write!(f, "Deserialization error: {}", e),
            TemplateError::CircularReference(id) => write!(f, "Circular template reference detected: {}", id),
            TemplateError::InvalidTemplate(e) => write!(f, "Invalid template: {}", e),
        }
    }
}

impl std::error::Error for TemplateError {}

/// Template selector for UI
pub struct TemplateSelector {
    pub selected_id: Option<TemplateId>,
    pub preview_config: Option<FullConfig>,
}

impl TemplateSelector {
    pub fn new() -> Self {
        Self {
            selected_id: None,
            preview_config: None,
        }
    }

    pub fn select(&mut self, template_id: &str, manager: &TemplateManager) {
        self.selected_id = Some(template_id.to_string());
        self.preview_config = manager.apply_template(template_id).ok();
    }

    pub fn clear(&mut self) {
        self.selected_id = None;
        self.preview_config = None;
    }
}

/// Helper function to get current timestamp
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_manager_creation() {
        let mut manager = TemplateManager::new();
        manager.load_builtin_templates();

        assert!(manager.get_template("minimal").is_some());
        assert!(manager.get_template("balanced").is_some());
        assert!(manager.get_template("rich").is_some());
    }

    #[test]
    fn test_apply_template() {
        let mut manager = TemplateManager::new();
        manager.load_builtin_templates();

        let minimal = manager.apply_template("minimal").unwrap();
        assert!(!minimal.app_config.show_sidebar);
        assert_eq!(minimal.app_config.sidebar_width, 200);

        let rich = manager.apply_template("rich").unwrap();
        assert!(rich.app_config.show_sidebar);
        assert_eq!(rich.app_config.sidebar_width, 300);
    }

    #[test]
    fn test_template_inheritance() {
        let mut manager = TemplateManager::new();
        manager.load_builtin_templates();

        // Development template inherits from balanced
        let dev = manager.apply_template("development").unwrap();
        assert_eq!(dev.app_config.theme, Theme::Dark);
        assert_eq!(dev.app_config.sidebar_width, 280); // From development
    }

    #[test]
    fn test_create_user_template() {
        let mut manager = TemplateManager::new();

        let mut config = FullConfig::default();
        config.app_config.theme = Theme::Dark;
        config.user_preferences.default_port = 2222;

        let template = manager.create_template(
            "my-template",
            "My Template",
            "Custom template",
            TemplateCategory::UserDefined,
            &config,
        ).unwrap();

        assert_eq!(template.id, "my-template");
        assert_eq!(template.overrides.theme, Some(Theme::Dark));
        assert_eq!(template.overrides.default_port, Some(2222));
    }

    #[test]
    fn test_list_by_category() {
        let mut manager = TemplateManager::new();
        manager.load_builtin_templates();

        let builtins = manager.list_by_category(TemplateCategory::BuiltIn);
        assert!(!builtins.is_empty());

        for template in builtins {
            assert_eq!(template.category, TemplateCategory::BuiltIn);
        }
    }

    #[test]
    fn test_cannot_remove_builtin() {
        let mut manager = TemplateManager::new();
        manager.load_builtin_templates();

        let result = manager.remove_template("minimal");
        assert!(matches!(result, Err(TemplateError::CannotRemoveBuiltin(_))));
    }

    #[test]
    fn test_template_selector() {
        let mut manager = TemplateManager::new();
        manager.load_builtin_templates();

        let mut selector = TemplateSelector::new();
        selector.select("minimal", &manager);

        assert_eq!(selector.selected_id, Some("minimal".to_string()));
        assert!(selector.preview_config.is_some());

        selector.clear();
        assert!(selector.selected_id.is_none());
    }
}
