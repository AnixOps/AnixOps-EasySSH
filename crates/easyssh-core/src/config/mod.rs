//! Configuration Management Module
//!
//! Provides application configuration, user preferences, and security settings
//! with support for loading, saving, validation, migration, and change notifications.
//!
//! # Storage Locations
//! - Windows: `%APPDATA%/EasySSH/config.json`
//! - Linux: `~/.config/easyssh/config.json`
//! - macOS: `~/Library/Application Support/EasySSH/config.json`
//!
//! # Example
//! ```rust,no_run
//! use easyssh_core::config::{ConfigManager, AppConfig};
//!
//! let config = ConfigManager::new().expect("Failed to create config manager");
//! config.load().expect("Failed to load config");
//!
//! // Get application config
//! let app_config = config.app_config();
//! println!("Theme: {:?}", app_config.theme);
//! ```

pub mod defaults;
pub mod encryption;
pub mod env;
pub mod import_export;
pub mod manager;
pub mod migration;
pub mod templates;
pub mod types;
pub mod validation;

pub use defaults::*;
pub use encryption::{
    password, ConfigEncryption, EncryptionOptions, EncryptionResult, PasswordStrength,
    SecurityLevel,
};
pub use env::{
    EnvConfig, EnvDefinitions, EnvProfile, EnvSource, EnvType, EnvVarDefinition, EnvVariable,
};
pub use import_export::{
    ExportFormat, ExportOptions, ImportExportError, ImportExportManager, ImportOptions,
    ImportResult,
};
pub use manager::ConfigManager;
pub use migration::{ConfigMigration, MigrationResult};
pub use templates::{
    ConfigOverrides, Template, TemplateCategory, TemplateError, TemplateManager, TemplateSelector,
};
pub use types::*;
pub use validation::{ConfigAutoFix, ConfigValidator, ValidationError, ValidationResult};

use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe configuration handle
pub type ConfigHandle = Arc<RwLock<ConfigManager>>;

/// Creates a new thread-safe configuration handle
pub async fn create_config_handle() -> crate::error::EasySSHResult<ConfigHandle> {
    let manager = ConfigManager::new()?;
    manager.load_async().await?;
    Ok(Arc::new(RwLock::new(manager)))
}

/// Configuration change event
#[derive(Debug, Clone)]
pub enum ConfigChangeEvent {
    AppConfigChanged,
    UserPreferencesChanged,
    SecuritySettingsChanged,
    ConfigReloaded,
}

/// Configuration change listener trait
pub trait ConfigChangeListener: Send + Sync {
    fn on_config_changed(&self, event: ConfigChangeEvent);
}

/// Type alias for configuration change callback
pub type ConfigChangeCallback = Box<dyn Fn(ConfigChangeEvent) + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = FullConfig::default();
        assert_eq!(config.app_config.theme, Theme::System);
        assert_eq!(config.app_config.language, Language::English);
        assert_eq!(config.user_preferences.default_port, 22);
    }

    #[test]
    fn test_config_validation() {
        let mut config = FullConfig::default();
        let validator = ConfigValidator::new();
        // Valid config should pass
        assert!(validator.validate(&config).is_ok());

        // Invalid port should fail
        config.user_preferences.default_port = 0;
        let result = validator.validate(&config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_config_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.json");

        let mut config = FullConfig::default();
        config.app_config.theme = Theme::Dark;
        config.user_preferences.default_username = "testuser".to_string();

        // Save config
        let json = serde_json::to_string_pretty(&config).unwrap();
        tokio::fs::write(&config_path, json).await.unwrap();

        // Load config
        let loaded_json = tokio::fs::read_to_string(&config_path).await.unwrap();
        let loaded: FullConfig = serde_json::from_str(&loaded_json).unwrap();

        assert_eq!(loaded.app_config.theme, Theme::Dark);
        assert_eq!(loaded.user_preferences.default_username, "testuser");
    }
}
