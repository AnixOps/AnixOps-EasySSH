//! Configuration Migration
//!
//! Handles configuration version migration to ensure compatibility
//! when upgrading between application versions.

use super::types::{FullConfig, Theme};
use std::collections::HashMap;

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
        }
    }

    /// Check if migration was successful
    pub fn is_success(&self) -> bool {
        self.success
    }
}

/// Configuration migration handler
pub struct ConfigMigration;

impl ConfigMigration {
    /// Migrate configuration to the latest version
    pub fn migrate(config: &mut FullConfig) -> MigrationResult {
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

        let mut applied = Vec::new();
        let mut current = from_version;

        while current < to_version {
            match Self::apply_migration(config, current, current + 1) {
                Ok(migration_name) => {
                    applied.push(migration_name);
                    current += 1;
                    config.version = current;
                }
                Err(e) => {
                    return MigrationResult::failure(
                        from_version,
                        to_version,
                        vec![format!("Migration from v{} to v{} failed: {}", current, current + 1, e)],
                    );
                }
            }
        }

        MigrationResult::success(from_version, to_version, applied)
    }

    /// Apply a single migration step
    fn apply_migration(
        config: &mut FullConfig,
        from: u32,
        to: u32,
    ) -> Result<String, String> {
        match (from, to) {
            (0, 1) => Self::migrate_v0_to_v1(config),
            _ => Err(format!("No migration path from v{} to v{}", from, to)),
        }
    }

    /// Migration from v0 (initial/unversioned) to v1
    fn migrate_v0_to_v1(config: &mut FullConfig) -> Result<String, String> {
        // v0 was the initial version before we added version tracking
        // This migration ensures all new fields have defaults

        // Ensure theme is valid
        let theme_valid = matches!(
            config.app_config.theme,
            Theme::Light | Theme::Dark | Theme::System
        );
        if !theme_valid {
            config.app_config.theme = Theme::System;
        }

        // Initialize custom HashMaps
        if config.app_config.custom.is_empty() && config.app_config.custom.capacity() == 0 {
            config.app_config.custom = HashMap::new();
        }
        if config.user_preferences.custom.is_empty() && config.user_preferences.custom.capacity() == 0 {
            config.user_preferences.custom = HashMap::new();
        }
        if config.security_settings.custom.is_empty() && config.security_settings.custom.capacity() == 0 {
            config.security_settings.custom = HashMap::new();
        }

        // Validate and fix port
        if config.user_preferences.default_port == 0 {
            config.user_preferences.default_port = 22;
        }

        // Ensure search history and recent connections exist
        if config.user_preferences.search_history.is_empty() {
            config.user_preferences.search_history = Vec::new();
        }
        if config.user_preferences.recent_connections.is_empty() {
            config.user_preferences.recent_connections = Vec::new();
        }

        // Set reasonable defaults for new fields
        if config.user_preferences.max_search_history == 0 {
            config.user_preferences.max_search_history = 100;
        }
        if config.user_preferences.max_recent_connections == 0 {
            config.user_preferences.max_recent_connections = 20;
        }
        if config.user_preferences.connection_timeout == 0 {
            config.user_preferences.connection_timeout = 30;
        }

        Ok("v0_to_v1_initial_schema".to_string())
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
}

/// Backup configuration before migration
pub struct ConfigBackup;

impl ConfigBackup {
    /// Create a backup of the configuration file
    pub async fn create_backup(config_path: &std::path::Path) -> Result<std::path::PathBuf, String> {
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

    /// List available backups
    pub async fn list_backups(
        backup_dir: &std::path::Path,
    ) -> Result<Vec<std::path::PathBuf>, String> {
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
                backups.push(path);
            }
        }

        // Sort by modification time (newest first)
        backups.sort_by(|a, b| {
            let meta_a = std::fs::metadata(a).ok();
            let meta_b = std::fs::metadata(b).ok();
            match (meta_a, meta_b) {
                (Some(ma), Some(mb)) => mb
                    .modified()
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                    .cmp(&ma.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)),
                _ => std::cmp::Ordering::Equal,
            }
        });

        Ok(backups)
    }

    /// Restore from a backup
    pub async fn restore_backup(
        backup_path: &std::path::Path,
        config_path: &std::path::Path,
    ) -> Result<(), String> {
        if !backup_path.exists() {
            return Err("Backup file does not exist".to_string());
        }

        tokio::fs::copy(backup_path, config_path)
            .await
            .map_err(|e| format!("Failed to restore backup: {}", e))?;

        Ok(())
    }

    /// Clean up old backups, keeping only the most recent N
    pub async fn cleanup_old_backups(
        backup_dir: &std::path::Path,
        keep_count: usize,
    ) -> Result<usize, String> {
        let backups = Self::list_backups(backup_dir).await?;

        if backups.len() <= keep_count {
            return Ok(0);
        }

        let to_remove = &backups[keep_count..];
        let mut removed = 0;

        for backup in to_remove {
            if let Err(e) = tokio::fs::remove_file(backup).await {
                eprintln!("Failed to remove old backup {:?}: {}", backup, e);
            } else {
                removed += 1;
            }
        }

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::*;
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

        // Restore backup
        let new_config_path = temp_dir.path().join("new_config.json");
        ConfigBackup::restore_backup(&backup_path, &new_config_path)
            .await
            .unwrap();
        assert!(new_config_path.exists());
    }

    #[tokio::test]
    async fn test_cleanup_old_backups() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");

        // Create multiple backup files
        for i in 0..5 {
            let backup_path = backup_dir.join(format!("config_backup_{}.json", i));
            tokio::fs::create_dir_all(&backup_dir).await.unwrap();
            tokio::fs::write(&backup_path, "{}")
                .await
                .unwrap();
            // Small delay to ensure different modification times
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let removed = ConfigBackup::cleanup_old_backups(&backup_dir, 3).await.unwrap();
        assert_eq!(removed, 2);

        let remaining = ConfigBackup::list_backups(&backup_dir).await.unwrap();
        assert_eq!(remaining.len(), 3);
    }
}
