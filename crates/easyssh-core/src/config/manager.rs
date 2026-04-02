//! Configuration Manager
//!
//! Central manager for configuration loading, saving, validation, migration,
//! encryption, and change notifications. Provides thread-safe access to configuration data.
//!
//! # Features
//! - Multi-format import/export (JSON, YAML, TOML, ENV)
//! - Configuration encryption with AES-256-GCM
//! - Automatic migration
//! - Template-based configuration
//! - Environment variable integration
//! - Change notifications

use super::{
    defaults::{apply_system_defaults, ConfigPresets, EnvConfig},
    encryption::{ConfigEncryption, EncryptedConfig},
    import_export::{
        ExportFormat, ExportOptions, ImportExportManager, ImportOptions, ImportResult,
    },
    migration::{
        ConfigBackup, ConfigMigration, MigrationInfo, MigrationResult, CURRENT_CONFIG_VERSION,
    },
    templates::{Template, TemplateError, TemplateManager},
    types::*,
    validation::{ConfigAutoFix, ConfigValidator, SecurityValidationLevel, ValidationError},
    ConfigChangeCallback, ConfigChangeEvent, ConfigChangeListener,
};
use crate::error::EasySSHErrors;
use crate::EasySSHResult;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::{broadcast, RwLock};

/// Configuration manager that handles all configuration operations
pub struct ConfigManager {
    /// Current configuration
    config: FullConfig,
    /// Path to the configuration file
    config_path: PathBuf,
    /// Whether configuration has been modified since last save
    dirty: bool,
    /// Change notification channel
    change_sender: broadcast::Sender<ConfigChangeEvent>,
    /// Registered change listeners
    listeners: Vec<Arc<dyn ConfigChangeListener>>,
    /// Async callbacks for configuration changes
    callbacks: Arc<RwLock<Vec<ConfigChangeCallback>>>,
    /// Last validation errors
    last_validation_errors: Vec<ValidationError>,
    /// Auto-save enabled
    auto_save: bool,
    /// Template manager
    template_manager: Option<TemplateManager>,
    /// Encryption state
    encryption: Option<ConfigEncryption>,
    /// Whether config is encrypted
    is_encrypted: bool,
    /// Security validation level
    security_level: SecurityValidationLevel,
}

impl ConfigManager {
    /// Create a new configuration manager with default paths
    pub fn new() -> EasySSHResult<Self> {
        let config_path = ConfigPaths::config_file()
            .ok_or_else(|| EasySSHErrors::configuration("Could not determine config file path"))?;

        Self::with_path(config_path)
    }

    /// Create a configuration manager with a specific path
    pub fn with_path(config_path: PathBuf) -> EasySSHResult<Self> {
        let (change_sender, _) = broadcast::channel(100);

        Ok(Self {
            config: FullConfig::default(),
            config_path,
            dirty: false,
            change_sender,
            listeners: Vec::new(),
            callbacks: Arc::new(RwLock::new(Vec::new())),
            last_validation_errors: Vec::new(),
            auto_save: true,
            template_manager: None,
            encryption: None,
            is_encrypted: false,
            security_level: SecurityValidationLevel::Standard,
        })
    }

    /// Create a configuration manager with a specific preset
    pub fn with_preset(preset: ConfigPreset) -> EasySSHResult<Self> {
        let mut manager = Self::new()?;

        manager.config = match preset {
            ConfigPreset::Minimal => ConfigPresets::minimal(),
            ConfigPreset::Balanced => ConfigPresets::balanced(),
            ConfigPreset::Rich => ConfigPresets::rich(),
            ConfigPreset::Enterprise => ConfigPresets::enterprise(),
        };

        manager.dirty = true;
        Ok(manager)
    }

    /// Create with a template
    pub fn with_template(template_id: &str) -> EasySSHResult<Self> {
        let mut manager = Self::new()?;
        let mut template_manager = TemplateManager::new();
        template_manager.load_builtin_templates();

        if let Ok(config) = template_manager.apply_template(template_id) {
            manager.config = config;
            manager.template_manager = Some(template_manager);
            manager.dirty = true;
            Ok(manager)
        } else {
            Err(EasySSHErrors::configuration(format!(
                "Template '{}' not found",
                template_id
            )))
        }
    }

    /// Get the configuration file path
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Check if configuration has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Enable or disable auto-save
    pub fn set_auto_save(&mut self, enabled: bool) {
        self.auto_save = enabled;
    }

    /// Check if auto-save is enabled
    pub fn auto_save(&self) -> bool {
        self.auto_save
    }

    /// Set security validation level
    pub fn set_security_level(&mut self, level: SecurityValidationLevel) {
        self.security_level = level;
    }

    /// Get security level
    pub fn security_level(&self) -> SecurityValidationLevel {
        self.security_level
    }

    /// Load configuration from disk (synchronous)
    pub fn load(&mut self) -> EasySSHResult<()> {
        if !self.config_path.exists() {
            // No existing config, use defaults and save
            self.config = FullConfig::default();
            apply_system_defaults(&mut self.config);
            EnvConfig::apply_overrides(&mut self.config);
            self.dirty = true;

            if self.auto_save {
                self.save()?;
            }

            return Ok(());
        }

        // Check if file is encrypted
        let content = std::fs::read_to_string(&self.config_path).map_err(|e| {
            EasySSHErrors::configuration(format!("Failed to read config file: {}", e))
        })?;

        // Try to detect encrypted config
        if content.trim().starts_with('{')
            && content.contains("\"salt\"")
            && content.contains("\"data\"")
        {
            self.is_encrypted = true;
            // Don't try to decrypt without password - let caller handle this
        } else {
            // Read and parse configuration
            let mut config: FullConfig = serde_json::from_str(&content).map_err(|e| {
                EasySSHErrors::configuration(format!("Failed to parse config file: {}", e))
            })?;

            self.load_config_internal(&mut config)?;
        }

        Ok(())
    }

    /// Load configuration from disk (asynchronous)
    pub async fn load_async(&self) -> EasySSHResult<FullConfig> {
        let config_path = self.config_path.clone();

        if !config_path.exists() {
            let mut config = FullConfig::default();
            apply_system_defaults(&mut config);
            EnvConfig::apply_overrides(&mut config);
            return Ok(config);
        }

        let content = tokio::fs::read_to_string(&config_path).await.map_err(|e| {
            EasySSHErrors::configuration(format!("Failed to read config file: {}", e))
        })?;

        let mut config: FullConfig = serde_json::from_str(&content).map_err(|e| {
            EasySSHErrors::configuration(format!("Failed to parse config file: {}", e))
        })?;

        // Apply system defaults for new fields
        apply_system_defaults(&mut config);

        // Apply environment overrides
        EnvConfig::apply_overrides(&mut config);

        // Sanitize configuration
        ConfigValidator::sanitize(&mut config);

        // Migrate if needed
        if ConfigMigration::needs_migration(&config) {
            let mut migrated_config = config.clone();
            let result = ConfigMigration::migrate(&mut migrated_config);
            if result.success {
                config = migrated_config;
            }
        }

        // Validate and auto-fix
        let validator = ConfigValidator::new().security_level(self.security_level);
        if validator.validate(&config).is_err() {
            ConfigAutoFix::auto_fix(&mut config);
        }

        Ok(config)
    }

    /// Load and decrypt configuration
    pub fn load_encrypted(&mut self, master_password: &str) -> EasySSHResult<()> {
        if !self.config_path.exists() {
            return self.load();
        }

        let content = std::fs::read_to_string(&self.config_path).map_err(|e| {
            EasySSHErrors::configuration(format!("Failed to read config file: {}", e))
        })?;

        // Try to parse as encrypted config
        if let Ok(encrypted) = serde_json::from_str::<EncryptedConfig>(&content) {
            let encryption = ConfigEncryption::new(master_password).map_err(|e| {
                EasySSHErrors::configuration(format!("Failed to initialize encryption: {}", e))
            })?;

            self.config = encryption.decrypt_config(&encrypted).map_err(|e| {
                EasySSHErrors::configuration(format!("Failed to decrypt config: {}", e))
            })?;

            self.encryption = Some(encryption);
            self.is_encrypted = true;
            self.dirty = false;

            self.notify_change(ConfigChangeEvent::ConfigReloaded);
            Ok(())
        } else {
            // Not encrypted, load normally
            self.load()
        }
    }

    /// Save encrypted configuration
    pub fn save_encrypted(&mut self) -> EasySSHResult<()> {
        let encryption = self
            .encryption
            .as_ref()
            .ok_or_else(|| EasySSHErrors::configuration("Encryption not initialized"))?;

        let encrypted = encryption.encrypt_config(&self.config).map_err(|e| {
            EasySSHErrors::configuration(format!("Failed to encrypt config: {}", e))
        })?;

        // Ensure config directory exists
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                EasySSHErrors::configuration(format!("Failed to create config directory: {}", e))
            })?;
        }

        let json = serde_json::to_string_pretty(&encrypted).map_err(|e| {
            EasySSHErrors::configuration(format!("Failed to serialize encrypted config: {}", e))
        })?;

        std::fs::write(&self.config_path, json).map_err(|e| {
            EasySSHErrors::configuration(format!("Failed to write config file: {}", e))
        })?;

        self.dirty = false;
        self.is_encrypted = true;

        Ok(())
    }

    /// Initialize encryption for the current config
    pub fn initialize_encryption(&mut self, master_password: &str) -> EasySSHResult<()> {
        let encryption = ConfigEncryption::new(master_password).map_err(|e| {
            EasySSHErrors::configuration(format!("Failed to initialize encryption: {}", e))
        })?;

        self.encryption = Some(encryption);
        self.dirty = true;

        Ok(())
    }

    /// Check if configuration is encrypted
    pub fn is_encrypted(&self) -> bool {
        self.is_encrypted
    }

    /// Load configuration asynchronously and update the manager
    pub async fn reload(&mut self) -> EasySSHResult<()> {
        let config = self.load_async().await?;
        self.config = config;
        self.dirty = false;
        self.notify_change(ConfigChangeEvent::ConfigReloaded);
        Ok(())
    }

    /// Internal load logic
    fn load_config_internal(&mut self, config: &mut FullConfig) -> EasySSHResult<()> {
        // Apply system defaults for new fields
        apply_system_defaults(config);

        // Apply environment overrides
        EnvConfig::apply_overrides(config);

        // Sanitize configuration
        ConfigValidator::sanitize(config);

        // Migrate if needed
        if ConfigMigration::needs_migration(config) {
            let result = ConfigMigration::migrate(config);
            if !result.success {
                return Err(EasySSHErrors::configuration(format!(
                    "Configuration migration failed: {:?}",
                    result.errors
                )));
            }
        }

        // Validate configuration
        let validator = ConfigValidator::new().security_level(self.security_level);
        if let Err(_errors) = validator.validate(config) {
            // Try to auto-fix issues
            let fixes = ConfigAutoFix::auto_fix(config);

            // Re-validate after auto-fix
            if let Err(errors) = validator.validate(config) {
                self.last_validation_errors = errors;
                eprintln!(
                    "Configuration validation warnings: {:?}",
                    self.last_validation_errors
                );
            }

            if !fixes.is_empty() {
                eprintln!("Auto-fixed configuration issues: {:?}", fixes);
            }
        }

        self.config = config.clone();
        self.dirty = false;

        self.notify_change(ConfigChangeEvent::ConfigReloaded);

        Ok(())
    }

    /// Save configuration to disk (synchronous)
    pub fn save(&mut self) -> EasySSHResult<()> {
        // Validate before saving
        let validator = ConfigValidator::new().security_level(self.security_level);
        if let Err(errors) = validator.validate(&self.config) {
            let has_errors = errors
                .iter()
                .any(|e| matches!(e.severity, super::validation::ValidationSeverity::Error));
            if has_errors {
                return Err(EasySSHErrors::configuration(format!(
                    "Cannot save invalid configuration: {:?}",
                    errors
                )));
            }
            // Just warnings, proceed with save
            self.last_validation_errors = errors;
        }

        // If encrypted, save encrypted version
        if self.is_encrypted || self.encryption.is_some() {
            return self.save_encrypted();
        }

        // Ensure config directory exists
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                EasySSHErrors::configuration(format!("Failed to create config directory: {}", e))
            })?;
        }

        // Serialize and write
        let json = serde_json::to_string_pretty(&self.config).map_err(|e| {
            EasySSHErrors::configuration(format!("Failed to serialize config: {}", e))
        })?;

        std::fs::write(&self.config_path, json).map_err(|e| {
            EasySSHErrors::configuration(format!("Failed to write config file: {}", e))
        })?;

        self.dirty = false;

        Ok(())
    }

    /// Save configuration to disk (asynchronous)
    pub async fn save_async(&self) -> EasySSHResult<()> {
        // Validate before saving
        let validator = ConfigValidator::new().security_level(self.security_level);
        if let Err(errors) = validator.validate(&self.config) {
            let has_errors = errors
                .iter()
                .any(|e| matches!(e.severity, super::validation::ValidationSeverity::Error));
            if has_errors {
                return Err(EasySSHErrors::configuration(format!(
                    "Cannot save invalid configuration: {:?}",
                    errors
                )));
            }
        }

        // Ensure config directory exists
        if let Some(parent) = self.config_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                EasySSHErrors::configuration(format!("Failed to create config directory: {}", e))
            })?;
        }

        // Serialize and write
        let json = serde_json::to_string_pretty(&self.config).map_err(|e| {
            EasySSHErrors::configuration(format!("Failed to serialize config: {}", e))
        })?;

        tokio::fs::write(&self.config_path, json)
            .await
            .map_err(|e| {
                EasySSHErrors::configuration(format!("Failed to write config file: {}", e))
            })?;

        Ok(())
    }

    /// Save if dirty and auto-save is enabled
    pub async fn auto_save_if_needed(&mut self) -> EasySSHResult<()> {
        if self.dirty && self.auto_save {
            self.save()
                .map_err(|e| EasySSHErrors::configuration(format!("Auto-save failed: {}", e)))?;
        }
        Ok(())
    }

    /// Get the full configuration
    pub fn full_config(&self) -> &FullConfig {
        &self.config
    }

    /// Get mutable access to full configuration
    pub fn full_config_mut(&mut self) -> &mut FullConfig {
        self.dirty = true;
        &mut self.config
    }

    /// Get application configuration
    pub fn app_config(&self) -> &AppConfig {
        &self.config.app_config
    }

    /// Get mutable application configuration
    pub fn app_config_mut(&mut self) -> &mut AppConfig {
        self.dirty = true;
        &mut self.config.app_config
    }

    /// Set application configuration
    pub fn set_app_config(&mut self, config: AppConfig) {
        self.config.app_config = config;
        self.dirty = true;
        self.notify_change(ConfigChangeEvent::AppConfigChanged);
    }

    /// Get user preferences
    pub fn user_preferences(&self) -> &UserPreferences {
        &self.config.user_preferences
    }

    /// Get mutable user preferences
    pub fn user_preferences_mut(&mut self) -> &mut UserPreferences {
        self.dirty = true;
        &mut self.config.user_preferences
    }

    /// Set user preferences
    pub fn set_user_preferences(&mut self, prefs: UserPreferences) {
        self.config.user_preferences = prefs;
        self.dirty = true;
        self.notify_change(ConfigChangeEvent::UserPreferencesChanged);
    }

    /// Get security settings
    pub fn security_settings(&self) -> &SecuritySettings {
        &self.config.security_settings
    }

    /// Get mutable security settings
    pub fn security_settings_mut(&mut self) -> &mut SecuritySettings {
        self.dirty = true;
        &mut self.config.security_settings
    }

    /// Set security settings
    pub fn set_security_settings(&mut self, settings: SecuritySettings) {
        self.config.security_settings = settings;
        self.dirty = true;
        self.notify_change(ConfigChangeEvent::SecuritySettingsChanged);
    }

    /// Update a specific field using a closure
    pub fn update_app_config<F>(&mut self, f: F)
    where
        F: FnOnce(&mut AppConfig),
    {
        f(&mut self.config.app_config);
        self.dirty = true;
        self.notify_change(ConfigChangeEvent::AppConfigChanged);
    }

    /// Update user preferences using a closure
    pub fn update_user_preferences<F>(&mut self, f: F)
    where
        F: FnOnce(&mut UserPreferences),
    {
        f(&mut self.config.user_preferences);
        self.dirty = true;
        self.notify_change(ConfigChangeEvent::UserPreferencesChanged);
    }

    /// Update security settings using a closure
    pub fn update_security_settings<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SecuritySettings),
    {
        f(&mut self.config.security_settings);
        self.dirty = true;
        self.notify_change(ConfigChangeEvent::SecuritySettingsChanged);
    }

    /// Add a recent connection
    pub fn add_recent_connection(&mut self, server_id: impl Into<String>) {
        let server_id = server_id.into();
        let prefs = &mut self.config.user_preferences;

        // Remove if already exists (move to front)
        prefs.recent_connections.retain(|id| id != &server_id);

        // Add to front
        prefs.recent_connections.insert(0, server_id);

        // Trim to max
        if prefs.recent_connections.len() > prefs.max_recent_connections {
            prefs
                .recent_connections
                .truncate(prefs.max_recent_connections);
        }

        self.dirty = true;
        self.notify_change(ConfigChangeEvent::UserPreferencesChanged);
    }

    /// Add a search history entry
    pub fn add_search_history(&mut self, query: impl Into<String>) {
        let query = query.into().trim().to_string();

        if query.is_empty() {
            return;
        }

        let prefs = &mut self.config.user_preferences;

        // Remove if already exists (move to front)
        prefs.search_history.retain(|q| q != &query);

        // Add to front
        prefs.search_history.insert(0, query);

        // Trim to max
        if prefs.search_history.len() > prefs.max_search_history {
            prefs.search_history.truncate(prefs.max_search_history);
        }

        self.dirty = true;
        self.notify_change(ConfigChangeEvent::UserPreferencesChanged);
    }

    /// Update window geometry
    pub fn update_window_geometry(&mut self, geometry: WindowGeometry) {
        self.config.app_config.window_geometry = geometry;
        self.dirty = true;
        // Don't notify for window geometry changes (too frequent)
    }

    /// Validate current configuration
    pub fn validate(&self) -> EasySSHResult<()> {
        let validator = ConfigValidator::new().security_level(self.security_level);
        match validator.validate(&self.config) {
            Ok(()) => Ok(()),
            Err(errors) => {
                let msg = errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(EasySSHErrors::configuration(format!(
                    "Config validation failed: {}",
                    msg
                )))
            }
        }
    }

    /// Get last validation errors
    pub fn last_validation_errors(&self) -> &[ValidationError] {
        &self.last_validation_errors
    }

    /// Export configuration to a file
    pub fn export_to(&self, path: &Path, format: Option<ExportFormat>) -> EasySSHResult<()> {
        let format = format
            .or_else(|| {
                path.extension()
                    .and_then(|e| e.to_str())
                    .and_then(ExportFormat::from_extension)
            })
            .unwrap_or(ExportFormat::JsonPretty);

        let options = ExportOptions {
            format,
            ..Default::default()
        };

        let content = ImportExportManager::export_config(&self.config, options)
            .map_err(|e| EasySSHErrors::configuration(format!("Export failed: {}", e)))?;

        std::fs::write(path, content).map_err(|e| {
            EasySSHErrors::configuration(format!("Failed to write export file: {}", e))
        })?;

        Ok(())
    }

    /// Import configuration from a file
    pub fn import_from(&mut self, path: &Path) -> EasySSHResult<ImportResult> {
        let options = ImportOptions::default();
        let content = std::fs::read_to_string(path).map_err(|e| {
            EasySSHErrors::configuration(format!("Failed to read import file: {}", e))
        })?;

        let result = ImportExportManager::import_config(&content, options);

        if let Some(config) = result.config.clone() {
            self.config = config;
            self.dirty = true;
            self.notify_change(ConfigChangeEvent::ConfigReloaded);

            if self.auto_save {
                self.save()?;
            }
        }

        Ok(result)
    }

    /// Import from multiple formats
    pub fn import_from_format(
        &mut self,
        content: &str,
        format: ExportFormat,
    ) -> EasySSHResult<ImportResult> {
        let options = ImportOptions {
            format_hint: Some(format),
            ..Default::default()
        };

        let result = ImportExportManager::import_config(content, options);

        if let Some(config) = result.config.clone() {
            self.config = config;
            self.dirty = true;
            self.notify_change(ConfigChangeEvent::ConfigReloaded);

            if self.auto_save {
                self.save()?;
            }
        }

        Ok(result)
    }

    /// Reset configuration to defaults
    pub fn reset_to_defaults(&mut self) {
        self.config = FullConfig::default();
        apply_system_defaults(&mut self.config);
        self.dirty = true;
        self.notify_change(ConfigChangeEvent::ConfigReloaded);
    }

    /// Reset to a template
    pub fn reset_to_template(
        &mut self,
        template_id: &str,
    ) -> std::result::Result<(), TemplateError> {
        if let Some(ref manager) = self.template_manager {
            self.config = manager.apply_template(template_id)?;
            self.dirty = true;
            self.notify_change(ConfigChangeEvent::ConfigReloaded);
            Ok(())
        } else {
            let mut manager = TemplateManager::new();
            manager.load_builtin_templates();
            self.config = manager.apply_template(template_id)?;
            self.template_manager = Some(manager);
            self.dirty = true;
            self.notify_change(ConfigChangeEvent::ConfigReloaded);
            Ok(())
        }
    }

    /// Register a change listener
    pub fn add_listener(&mut self, listener: Arc<dyn ConfigChangeListener>) {
        self.listeners.push(listener);
    }

    /// Remove a change listener
    pub fn remove_listener(&mut self, listener: &Arc<dyn ConfigChangeListener>) {
        self.listeners.retain(|l| !Arc::ptr_eq(l, listener));
    }

    /// Register a change callback
    pub async fn add_callback<F>(&self, callback: F)
    where
        F: Fn(ConfigChangeEvent) + Send + Sync + 'static,
    {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(Box::new(callback));
    }

    /// Remove all callbacks
    pub async fn clear_callbacks(&self) {
        let mut callbacks = self.callbacks.write().await;
        callbacks.clear();
    }

    /// Subscribe to change events
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.change_sender.subscribe()
    }

    /// Notify all listeners of a change
    fn notify_change(&self, event: ConfigChangeEvent) {
        // Notify sync listeners
        for listener in &self.listeners {
            listener.on_config_changed(event.clone());
        }

        // Send broadcast
        let _ = self.change_sender.send(event.clone());

        // Spawn async notification for callbacks
        let callbacks = self.callbacks.clone();
        let event_clone = event.clone();
        tokio::spawn(async move {
            let callbacks = callbacks.read().await;
            for callback in callbacks.iter() {
                callback(event_clone.clone());
            }
        });
    }

    /// Create a backup of the current configuration
    pub async fn backup(&self) -> EasySSHResult<std::path::PathBuf> {
        if !self.config_path.exists() {
            return Err(EasySSHErrors::configuration(
                "No configuration file to backup",
            ));
        }

        ConfigBackup::create_backup(&self.config_path)
            .await
            .map_err(|e| EasySSHErrors::configuration(format!("Failed to create backup: {}", e)))
    }

    /// Restore from a backup file
    pub async fn restore(&mut self, backup_path: &Path) -> EasySSHResult<()> {
        ConfigBackup::restore_backup(backup_path, &self.config_path)
            .await
            .map_err(|e| {
                EasySSHErrors::configuration(format!("Failed to restore backup: {}", e))
            })?;

        // Reload after restore
        self.reload().await
    }

    /// Get migration status
    pub fn migration_status(&self) -> String {
        ConfigMigration::migration_status(&self.config)
    }

    /// Get detailed migration info
    pub fn migration_info(&self) -> MigrationInfo {
        ConfigMigration::get_migration_info(&self.config)
    }

    /// Run configuration migration
    pub fn migrate(&mut self) -> MigrationResult {
        let result = ConfigMigration::migrate(&mut self.config);
        if result.success {
            self.dirty = true;
            if self.auto_save {
                let _ = self.save();
            }
        }
        result
    }

    /// Get current configuration version
    pub fn config_version(&self) -> u32 {
        self.config.version
    }

    /// Check if running current version
    pub fn is_up_to_date(&self) -> bool {
        self.config.version == CURRENT_CONFIG_VERSION
    }

    /// Get available templates
    pub fn available_templates(&self) -> Vec<&Template> {
        if let Some(ref manager) = self.template_manager {
            manager.list_templates()
        } else {
            Vec::new()
        }
    }

    /// Load templates
    pub fn load_templates(&mut self) {
        if self.template_manager.is_none() {
            let mut manager = TemplateManager::new();
            manager.load_builtin_templates();
            self.template_manager = Some(manager);
        }
    }

    /// Apply template
    pub fn apply_template(&mut self, template_id: &str) -> std::result::Result<(), TemplateError> {
        self.load_templates();
        self.reset_to_template(template_id)
    }

    /// Export as YAML
    pub fn export_yaml(&self) -> EasySSHResult<String> {
        let options = ExportOptions::yaml();
        ImportExportManager::export_config(&self.config, options)
            .map_err(|e| EasySSHErrors::configuration(format!("YAML export failed: {}", e)))
    }

    /// Export as TOML
    pub fn export_toml(&self) -> EasySSHResult<String> {
        let options = ExportOptions::toml();
        ImportExportManager::export_config(&self.config, options)
            .map_err(|e| EasySSHErrors::configuration(format!("TOML export failed: {}", e)))
    }

    /// Export as ENV format
    pub fn export_env(&self) -> EasySSHResult<String> {
        let options = ExportOptions::env();
        ImportExportManager::export_config(&self.config, options)
            .map_err(|e| EasySSHErrors::configuration(format!("ENV export failed: {}", e)))
    }

    /// Import from YAML
    pub fn import_yaml(&mut self, yaml: &str) -> EasySSHResult<ImportResult> {
        self.import_from_format(yaml, ExportFormat::Yaml)
    }

    /// Import from TOML
    pub fn import_toml(&mut self, toml: &str) -> EasySSHResult<ImportResult> {
        self.import_from_format(toml, ExportFormat::Toml)
    }

    /// Import from ENV format
    pub fn import_env(&mut self, env: &str) -> EasySSHResult<ImportResult> {
        self.import_from_format(env, ExportFormat::Env)
    }
}

impl Clone for ConfigManager {
    fn clone(&self) -> Self {
        let (change_sender, _) = broadcast::channel(100);

        Self {
            config: self.config.clone(),
            config_path: self.config_path.clone(),
            dirty: self.dirty,
            change_sender,
            listeners: Vec::new(), // Listeners don't clone
            callbacks: Arc::new(RwLock::new(Vec::new())),
            last_validation_errors: self.last_validation_errors.clone(),
            auto_save: self.auto_save,
            template_manager: None,
            encryption: None,
            is_encrypted: false,
            security_level: self.security_level,
        }
    }
}

/// Configuration preset types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigPreset {
    Minimal,
    Balanced,
    Rich,
    Enterprise,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let manager = ConfigManager::with_path(config_path);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_preset_creation() {
        let minimal = ConfigManager::with_preset(ConfigPreset::Minimal).unwrap();
        assert!(!minimal.app_config().show_sidebar);

        let rich = ConfigManager::with_preset(ConfigPreset::Rich).unwrap();
        assert!(rich.app_config().show_sidebar);
    }

    #[test]
    fn test_template_creation() {
        let dev = ConfigManager::with_template("development").unwrap();
        assert_eq!(dev.app_config().theme, Theme::Dark);
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Create and save config
        let mut manager = ConfigManager::with_path(config_path.clone()).unwrap();
        manager.update_app_config(|c| {
            c.theme = Theme::Dark;
        });
        manager.save().unwrap();

        // Load in new manager
        let mut loaded = ConfigManager::with_path(config_path).unwrap();
        loaded.load().unwrap();

        assert_eq!(loaded.app_config().theme, Theme::Dark);
    }

    #[tokio::test]
    async fn test_recent_connections() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = ConfigManager::with_path(config_path).unwrap();

        manager.add_recent_connection("server1");
        manager.add_recent_connection("server2");
        manager.add_recent_connection("server1"); // Should move to front

        let prefs = manager.user_preferences();
        assert_eq!(prefs.recent_connections.len(), 2);
        assert_eq!(prefs.recent_connections[0], "server1");
    }

    #[tokio::test]
    async fn test_search_history() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = ConfigManager::with_path(config_path).unwrap();
        manager.update_user_preferences(|p| p.max_search_history = 3);

        manager.add_search_history("query1");
        manager.add_search_history("query2");
        manager.add_search_history("query3");
        manager.add_search_history("query4"); // Should cause truncation

        let prefs = manager.user_preferences();
        assert_eq!(prefs.search_history.len(), 3);
    }

    #[tokio::test]
    async fn test_config_subscription() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = ConfigManager::with_path(config_path).unwrap();
        let mut rx = manager.subscribe();

        manager.update_app_config(|c| c.theme = Theme::Light);

        // Should receive notification
        let event = rx.try_recv();
        assert!(event.is_ok());
        assert!(matches!(
            event.unwrap(),
            ConfigChangeEvent::AppConfigChanged
        ));
    }

    #[tokio::test]
    async fn test_export_import() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let export_path = temp_dir.path().join("export.json");

        let mut manager = ConfigManager::with_path(config_path).unwrap();
        manager.update_app_config(|c| c.theme = Theme::Dark);
        manager.save().unwrap();

        // Export
        manager.export_to(&export_path, None).unwrap();
        assert!(export_path.exists());

        // Modify and import back
        manager.update_app_config(|c| c.theme = Theme::Light);
        manager.import_from(&export_path).unwrap();

        assert_eq!(manager.app_config().theme, Theme::Dark);
    }

    #[tokio::test]
    async fn test_reset_to_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = ConfigManager::with_path(config_path).unwrap();
        manager.update_app_config(|c| c.theme = Theme::Dark);
        manager.update_user_preferences(|p| p.default_port = 2222);

        manager.reset_to_defaults();

        assert_eq!(manager.app_config().theme, Theme::System);
        assert_eq!(manager.user_preferences().default_port, 22);
    }

    #[tokio::test]
    async fn test_export_import_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = ConfigManager::with_path(config_path).unwrap();
        manager.update_app_config(|c| c.theme = Theme::Dark);
        manager.update_user_preferences(|p| p.default_port = 2222);

        // Export as YAML
        let yaml = manager.export_yaml().unwrap();
        assert!(!yaml.is_empty());

        // Import back
        let result = manager.import_yaml(&yaml).unwrap();
        assert!(result.is_success());
    }

    #[test]
    fn test_security_level() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = ConfigManager::with_path(config_path).unwrap();
        manager.set_security_level(SecurityValidationLevel::High);

        assert_eq!(manager.security_level(), SecurityValidationLevel::High);
    }

    #[test]
    fn test_migration_info() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let manager = ConfigManager::with_path(config_path).unwrap();
        let info = manager.migration_info();

        assert_eq!(info.target_version, CURRENT_CONFIG_VERSION);
        assert!(info.can_migrate);
    }

    #[test]
    fn test_template_manager() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = ConfigManager::with_path(config_path).unwrap();
        manager.load_templates();

        let templates = manager.available_templates();
        assert!(!templates.is_empty());
    }
}
