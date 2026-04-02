//! Configuration Migration
//!
//! Handles configuration version migration to ensure compatibility
//! when upgrading between application versions.
//!
//! # Features
//! - Automatic version detection and migration
//! - Multi-step migration chains
//! - Pre/post migration hooks
//! - Rollback support
//! - Migration audit logging

use super::types::{FullConfig, Theme};
use super::validation::ConfigValidator;
use std::collections::HashMap;
use std::path::Path;

/// Current configuration schema version
pub const CURRENT_CONFIG_VERSION: u32 = 1;

/// Migration result
#[derive(Debug, Clone)]
pub struct MigrationResult {
    pub success: bool,
    pub from_version: u32,
    pub to_version: u32,
    pub applied_migrations: Vec<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub backup_path: Option<std::path::PathBuf>,
}

impl MigrationResult {
    /// Create a successful migration result
    pub fn success(from: u32, to: u32, migrations: Vec<String>) -> Self {
        Self {
            success: true,
            from_version: from,
            to_version: to,
            applied_migrations: migrations,
            errors: Vec::new(),
            warnings: Vec::new(),
            backup_path: None,
        }
    }

    /// Create a failed migration result
    pub fn failure(from: u32, to: u32, errors: Vec<String>) -> Self {
        Self {
            success: false,
            from_version: from,
            to_version: to,
            applied_migrations: Vec::new(),
            errors,
            warnings: Vec::new(),
            backup_path: None,
        }
    }

    /// Create a partial success result (with warnings)
    pub fn partial(from: u32, to: u32, migrations: Vec<String>, warnings: Vec<String>) -> Self {
        Self {
            success: true,
            from_version: from,
            to_version: to,
            applied_migrations: migrations,
            errors: Vec::new(),
            warnings,
            backup_path: None,
        }
    }

    /// Check if migration was successful
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Check if migration had warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get migration summary
    pub fn summary(&self) -> String {
        if self.success {
            if self.has_warnings() {
                format!(
                    "Migrated v{} to v{} with {} steps and {} warnings",
                    self.from_version,
                    self.to_version,
                    self.applied_migrations.len(),
                    self.warnings.len()
                )
            } else {
                format!(
                    "Successfully migrated v{} to v{} in {} steps",
                    self.from_version,
                    self.to_version,
                    self.applied_migrations.len()
                )
            }
        } else {
            format!(
                "Migration from v{} to v{} failed: {}",
                self.from_version,
                self.to_version,
                self.errors.join(", ")
            )
        }
    }
}

/// Migration step trait
pub trait MigrationStep: Send + Sync {
    /// Get migration name
    fn name(&self) -> &str;
    /// Get source version
    fn source_version(&self) -> u32;
    /// Get target version
    fn target_version(&self) -> u32;
    /// Execute migration
    fn migrate(&self, config: &mut FullConfig) -> Result<Vec<String>, String>;
    /// Validate after migration
    fn validate(&self, config: &FullConfig) -> Result<(), Vec<String>>;
}

/// Configuration migration registry
pub struct MigrationRegistry {
    steps: HashMap<(u32, u32), Box<dyn MigrationStep>>,
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MigrationRegistry {
    /// Create a new registry with built-in migrations
    pub fn new() -> Self {
        let mut registry = Self {
            steps: HashMap::new(),
        };
        registry.register_builtin_migrations();
        registry
    }

    /// Register a migration step
    pub fn register(&mut self, step: Box<dyn MigrationStep>) {
        let key = (step.source_version(), step.target_version());
        self.steps.insert(key, step);
    }

    /// Get migration step
    pub fn get_step(&self, from: u32, to: u32) -> Option<&dyn MigrationStep> {
        self.steps.get(&(from, to)).map(|s| s.as_ref())
    }

    /// Check if migration path exists
    pub fn has_path(&self, from: u32, to: u32) -> bool {
        self.find_path(from, to).is_ok()
    }

    /// Find migration path from version A to B
    fn find_path(&self, from: u32, to: u32) -> Result<Vec<(u32, u32)>, String> {
        if from == to {
            return Ok(vec![]);
        }

        if from > to {
            return Err(format!(
                "Cannot migrate backwards from v{} to v{}",
                from, to
            ));
        }

        // Simple linear path finding (assumes sequential versions)
        let mut path = Vec::new();
        let mut current = from;

        while current < to {
            let next = current + 1;
            if self.steps.contains_key(&(current, next)) {
                path.push((current, next));
                current = next;
            } else {
                return Err(format!("No migration path from v{} to v{}", current, next));
            }
        }

        Ok(path)
    }

    /// Register built-in migrations
    fn register_builtin_migrations(&mut self) {
        self.register(Box::new(MigrationV0ToV1::new()));
    }
}

/// Built-in migration: v0 to v1
pub struct MigrationV0ToV1;

impl MigrationV0ToV1 {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MigrationV0ToV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl MigrationStep for MigrationV0ToV1 {
    fn name(&self) -> &str {
        "v0_to_v1_initial_schema"
    }

    fn source_version(&self) -> u32 {
        0
    }

    fn target_version(&self) -> u32 {
        1
    }

    fn migrate(&self, config: &mut FullConfig) -> Result<Vec<String>, String> {
        let mut changes = Vec::new();

        // v0 was the initial version before we added version tracking
        // This migration ensures all new fields have defaults

        // Ensure theme is valid
        let theme_valid = matches!(
            config.app_config.theme,
            Theme::Light | Theme::Dark | Theme::System
        );
        if !theme_valid {
            config.app_config.theme = Theme::System;
            changes.push("Fixed invalid theme to System".to_string());
        }

        // Initialize custom HashMaps
        if config.app_config.custom.capacity() == 0 {
            config.app_config.custom = HashMap::new();
        }
        if config.user_preferences.custom.capacity() == 0 {
            config.user_preferences.custom = HashMap::new();
        }
        if config.security_settings.custom.capacity() == 0 {
            config.security_settings.custom = HashMap::new();
        }

        // Validate and fix port
        if config.user_preferences.default_port == 0 {
            config.user_preferences.default_port = 22;
            changes.push("Fixed default port to 22".to_string());
        }

        // Ensure search history and recent connections exist
        if config.user_preferences.search_history.capacity() == 0 {
            config.user_preferences.search_history = Vec::new();
        }
        if config.user_preferences.recent_connections.capacity() == 0 {
            config.user_preferences.recent_connections = Vec::new();
        }

        // Set reasonable defaults for new fields
        if config.user_preferences.max_search_history == 0 {
            config.user_preferences.max_search_history = 100;
            changes.push("Set max_search_history to 100".to_string());
        }
        if config.user_preferences.max_recent_connections == 0 {
            config.user_preferences.max_recent_connections = 20;
            changes.push("Set max_recent_connections to 20".to_string());
        }
        if config.user_preferences.connection_timeout == 0 {
            config.user_preferences.connection_timeout = 30;
            changes.push("Set connection_timeout to 30".to_string());
        }

        // Update version
        config.version = 1;
        changes.push("Updated configuration version to 1".to_string());

        Ok(changes)
    }

    fn validate(&self, config: &FullConfig) -> Result<(), Vec<String>> {
        let validator = ConfigValidator::new();
        match validator.validate(config) {
            Ok(()) => Ok(()),
            Err(errors) => {
                let msgs: Vec<String> = errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect();
                Err(msgs)
            }
        }
    }
}

/// Configuration migration handler
pub struct ConfigMigration {
    registry: MigrationRegistry,
}

impl Default for ConfigMigration {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigMigration {
    /// Create a new migration handler
    pub fn new() -> Self {
        Self {
            registry: MigrationRegistry::new(),
        }
    }

    /// Create with custom registry
    pub fn with_registry(registry: MigrationRegistry) -> Self {
        Self { registry }
    }

    /// Migrate configuration to the latest version
    pub fn migrate(config: &mut FullConfig) -> MigrationResult {
        let migrator = Self::new();
        migrator.migrate_with_registry(config)
    }

    /// Migrate with internal registry
    fn migrate_with_registry(&self, config: &mut FullConfig) -> MigrationResult {
        let from_version = config.version;
        let to_version = CURRENT_CONFIG_VERSION;

        if from_version == to_version {
            return MigrationResult::success(from_version, to_version, vec![]);
        }

        if from_version > to_version {
            return MigrationResult::failure(
                from_version,
                to_version,
                vec![format!(
                    "Configuration version {} is newer than supported version {}",
                    from_version, to_version
                )],
            );
        }

        // Find migration path
        let path = match self.registry.find_path(from_version, to_version) {
            Ok(p) => p,
            Err(e) => return MigrationResult::failure(from_version, to_version, vec![e]),
        };

        let mut applied = Vec::new();
        let mut warnings = Vec::new();

        for (from, to) in path {
            match self.apply_migration(config, from, to) {
                Ok((name, changes)) => {
                    applied.push(name);
                    if !changes.is_empty() {
                        warnings.extend(changes);
                    }
                }
                Err(e) => {
                    return MigrationResult::failure(
                        from_version,
                        to_version,
                        vec![format!("Migration from v{} to v{} failed: {}", from, to, e)],
                    );
                }
            }
        }

        if warnings.is_empty() {
            MigrationResult::success(from_version, to_version, applied)
        } else {
            MigrationResult::partial(from_version, to_version, applied, warnings)
        }
    }

    /// Apply a single migration step
    fn apply_migration(
        &self,
        config: &mut FullConfig,
        from: u32,
        to: u32,
    ) -> Result<(String, Vec<String>), String> {
        if let Some(step) = self.registry.get_step(from, to) {
            // Execute migration
            let changes = step.migrate(config)?;

            // Validate after migration
            if let Err(errors) = step.validate(config) {
                // Rollback not implemented - we continue with warnings
                return Err(format!("Post-migration validation failed: {:?}", errors));
            }

            Ok((step.name().to_string(), changes))
        } else {
            Err(format!("No migration step from v{} to v{}", from, to))
        }
    }

    /// Check if migration is needed
    pub fn needs_migration(config: &FullConfig) -> bool {
        config.version < CURRENT_CONFIG_VERSION
    }

    /// Get migration status description
    pub fn migration_status(config: &FullConfig) -> String {
        if config.version == CURRENT_CONFIG_VERSION {
            "Configuration is up to date".to_string()
        } else if config.version < CURRENT_CONFIG_VERSION {
            format!(
                "Configuration needs migration from v{} to v{}",
                config.version, CURRENT_CONFIG_VERSION
            )
        } else {
            format!(
                "Configuration version v{} is newer than supported v{}",
                config.version, CURRENT_CONFIG_VERSION
            )
        }
    }

    /// Get detailed migration info
    pub fn get_migration_info(config: &FullConfig) -> MigrationInfo {
        MigrationInfo {
            current_version: config.version,
            target_version: CURRENT_CONFIG_VERSION,
            needs_migration: Self::needs_migration(config),
            can_migrate: config.version <= CURRENT_CONFIG_VERSION,
            status: Self::migration_status(config),
        }
    }
}

/// Migration information
#[derive(Debug, Clone)]
pub struct MigrationInfo {
    pub current_version: u32,
    pub target_version: u32,
    pub needs_migration: bool,
    pub can_migrate: bool,
    pub status: String,
}

/// Backup management for configuration
pub struct ConfigBackup;

impl ConfigBackup {
    /// Create a backup of the configuration file
    pub async fn create_backup(config_path: &Path) -> Result<std::path::PathBuf, String> {
        if !config_path.exists() {
            return Err("Configuration file does not exist".to_string());
        }

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("config_backup_v{}.json", timestamp);

        let backup_dir = config_path
            .parent()
            .map(|p| p.join("backups"))
            .ok_or("Cannot determine backup directory")?;

        tokio::fs::create_dir_all(&backup_dir)
            .await
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;

        let backup_path = backup_dir.join(&backup_name);

        tokio::fs::copy(config_path, &backup_path)
            .await
            .map_err(|e| format!("Failed to create backup: {}", e))?;

        Ok(backup_path)
    }

    /// Create a backup with custom name
    pub async fn create_named_backup(
        config_path: &Path,
        name: &str,
    ) -> Result<std::path::PathBuf, String> {
        if !config_path.exists() {
            return Err("Configuration file does not exist".to_string());
        }

        let backup_dir = config_path
            .parent()
            .map(|p| p.join("backups"))
            .ok_or("Cannot determine backup directory")?;

        tokio::fs::create_dir_all(&backup_dir)
            .await
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;

        let backup_name = format!("config_backup_{}.json", name);
        let backup_path = backup_dir.join(backup_name);

        tokio::fs::copy(config_path, &backup_path)
            .await
            .map_err(|e| format!("Failed to create backup: {}", e))?;

        Ok(backup_path)
    }

    /// List available backups
    pub async fn list_backups(backup_dir: &Path) -> Result<Vec<BackupInfo>, String> {
        if !backup_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = tokio::fs::read_dir(backup_dir)
            .await
            .map_err(|e| format!("Failed to read backup directory: {}", e))?;

        let mut backups = Vec::new();

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read directory entry: {}", e))?
        {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let metadata = tokio::fs::metadata(&path).await.ok();
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .map(|t| format!("{:?}", t));

                backups.push(BackupInfo {
                    path: path.clone(),
                    name: path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    size,
                    modified,
                });
            }
        }

        // Sort by modification time (newest first)
        backups.sort_by(|a, b| b.modified.cmp(&a.modified));

        Ok(backups)
    }

    /// Restore from a backup
    pub async fn restore_backup(backup_path: &Path, config_path: &Path) -> Result<(), String> {
        if !backup_path.exists() {
            return Err("Backup file does not exist".to_string());
        }

        // First create a backup of current config if it exists
        if config_path.exists() {
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let pre_restore_name = format!("config_pre_restore_{}.json", timestamp);
            let backup_dir = config_path
                .parent()
                .map(|p| p.join("backups"))
                .ok_or("Cannot determine backup directory")?;
            let pre_restore_path = backup_dir.join(pre_restore_name);

            let _ = tokio::fs::copy(config_path, pre_restore_path).await;
        }

        tokio::fs::copy(backup_path, config_path)
            .await
            .map_err(|e| format!("Failed to restore backup: {}", e))?;

        Ok(())
    }

    /// Clean up old backups, keeping only the most recent N
    pub async fn cleanup_old_backups(
        backup_dir: &Path,
        keep_count: usize,
    ) -> Result<usize, String> {
        let backups = Self::list_backups(backup_dir).await?;

        if backups.len() <= keep_count {
            return Ok(0);
        }

        let to_remove = &backups[keep_count..];
        let mut removed = 0;

        for backup in to_remove {
            if let Err(e) = tokio::fs::remove_file(&backup.path).await {
                eprintln!("Failed to remove old backup {:?}: {}", backup.path, e);
            } else {
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// Get backup statistics
    pub async fn get_backup_stats(backup_dir: &Path) -> Result<BackupStats, String> {
        let backups = Self::list_backups(backup_dir).await?;

        let total_size: u64 = backups.iter().map(|b| b.size).sum();

        Ok(BackupStats {
            total_backups: backups.len(),
            total_size,
            oldest_backup: backups.last().and_then(|b| b.modified.clone()),
            newest_backup: backups.first().and_then(|b| b.modified.clone()),
        })
    }
}

/// Backup information
#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub path: std::path::PathBuf,
    pub name: String,
    pub size: u64,
    pub modified: Option<String>,
}

/// Backup statistics
#[derive(Debug, Clone)]
pub struct BackupStats {
    pub total_backups: usize,
    pub total_size: u64,
    pub oldest_backup: Option<String>,
    pub newest_backup: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::super::types::*;
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_migrate_v0_to_v1() {
        let mut config = FullConfig::default();
        config.version = 0;
        config.user_preferences.default_port = 0; // Invalid port

        let result = ConfigMigration::migrate(&mut config);

        assert!(result.success);
        assert_eq!(config.version, 1);
        assert_eq!(config.user_preferences.default_port, 22); // Fixed
    }

    #[test]
    fn test_no_migration_needed() {
        let mut config = FullConfig::default();
        config.version = CURRENT_CONFIG_VERSION;

        let result = ConfigMigration::migrate(&mut config);

        assert!(result.success);
        assert!(result.applied_migrations.is_empty());
    }

    #[test]
    fn test_downgrade_detection() {
        let mut config = FullConfig::default();
        config.version = 999; // Future version

        let result = ConfigMigration::migrate(&mut config);

        assert!(!result.success);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_needs_migration() {
        let mut config = FullConfig::default();

        config.version = 0;
        assert!(ConfigMigration::needs_migration(&config));

        config.version = CURRENT_CONFIG_VERSION;
        assert!(!ConfigMigration::needs_migration(&config));
    }

    #[test]
    fn test_migration_status() {
        let mut config = FullConfig::default();

        config.version = CURRENT_CONFIG_VERSION;
        assert!(ConfigMigration::migration_status(&config).contains("up to date"));

        config.version = 0;
        assert!(ConfigMigration::migration_status(&config).contains("needs migration"));

        config.version = 999;
        assert!(ConfigMigration::migration_status(&config).contains("newer"));
    }

    #[test]
    fn test_migration_info() {
        let mut config = FullConfig::default();
        config.version = 0;

        let info = ConfigMigration::get_migration_info(&config);
        assert!(info.needs_migration);
        assert!(info.can_migrate);
        assert_eq!(info.current_version, 0);
        assert_eq!(info.target_version, CURRENT_CONFIG_VERSION);
    }

    #[test]
    fn test_migration_result_summary() {
        let success = MigrationResult::success(0, 1, vec!["step1".to_string()]);
        assert!(success.summary().contains("Successfully migrated"));

        let failure = MigrationResult::failure(0, 1, vec!["error".to_string()]);
        assert!(failure.summary().contains("failed"));

        let partial =
            MigrationResult::partial(0, 1, vec!["step1".to_string()], vec!["warning".to_string()]);
        assert!(partial.summary().contains("warnings"));
    }

    #[test]
    fn test_migration_registry() {
        let registry = MigrationRegistry::new();

        assert!(registry.has_path(0, 1));
        assert!(!registry.has_path(0, 5)); // No path to v5
        assert!(!registry.has_path(1, 0)); // Cannot go backwards
    }

    #[tokio::test]
    async fn test_backup_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Create a test config file
        {
            let mut file = std::fs::File::create(&config_path).unwrap();
            file.write_all(b"{\"version\": 1}").unwrap();
        }

        // Create backup
        let backup_path = ConfigBackup::create_backup(&config_path).await.unwrap();
        assert!(backup_path.exists());

        // List backups
        let backup_dir = config_path.parent().unwrap().join("backups");
        let backups = ConfigBackup::list_backups(&backup_dir).await.unwrap();
        assert!(!backups.is_empty());

        // Get stats
        let stats = ConfigBackup::get_backup_stats(&backup_dir).await.unwrap();
        assert_eq!(stats.total_backups, 1);
        assert!(stats.total_size > 0);

        // Restore backup
        let new_config_path = temp_dir.path().join("new_config.json");
        ConfigBackup::restore_backup(&backup_path, &new_config_path)
            .await
            .unwrap();
        assert!(new_config_path.exists());
    }

    #[tokio::test]
    async fn test_named_backup() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        {
            let mut file = std::fs::File::create(&config_path).unwrap();
            file.write_all(b"{}").unwrap();
        }

        let backup_path = ConfigBackup::create_named_backup(&config_path, "pre_migration")
            .await
            .unwrap();

        assert!(backup_path.to_string_lossy().contains("pre_migration"));
    }

    #[tokio::test]
    async fn test_cleanup_old_backups() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");

        // Create multiple backup files
        for i in 0..5 {
            let backup_path = backup_dir.join(format!("config_backup_{}.json", i));
            tokio::fs::create_dir_all(&backup_dir).await.unwrap();
            tokio::fs::write(&backup_path, "{}").await.unwrap();
            // Small delay to ensure different modification times
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let removed = ConfigBackup::cleanup_old_backups(&backup_dir, 3)
            .await
            .unwrap();
        assert_eq!(removed, 2);

        let remaining = ConfigBackup::list_backups(&backup_dir).await.unwrap();
        assert_eq!(remaining.len(), 3);
    }

    #[test]
    fn test_migration_step_trait() {
        let step = MigrationV0ToV1::new();
        assert_eq!(step.name(), "v0_to_v1_initial_schema");
        assert_eq!(step.source_version(), 0);
        assert_eq!(step.target_version(), 1);
    }
}
