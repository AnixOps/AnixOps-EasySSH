//! Config repository
//!
//! This module provides CRUD operations for application configuration.
//! Configuration values are stored as key-value pairs.

use crate::database::error::{DatabaseError, Result};
use sqlx::SqlitePool;

/// Repository for application configuration
#[derive(Debug, Clone)]
pub struct ConfigRepository {
    pub(super) pool: SqlitePool,
}

/// Default configuration keys
pub mod keys {
    /// Application version
    pub const APP_VERSION: &str = "app.version";

    /// Last database migration version
    pub const DB_VERSION: &str = "db.version";

    /// First run timestamp
    pub const FIRST_RUN_AT: &str = "app.first_run_at";

    /// Theme preference
    pub const THEME: &str = "ui.theme";

    /// Language/locale setting
    pub const LANGUAGE: &str = "ui.language";

    /// Window size - width
    pub const WINDOW_WIDTH: &str = "ui.window.width";

    /// Window size - height
    pub const WINDOW_HEIGHT: &str = "ui.window.height";

    /// Main password hash (for master password verification)
    pub const MASTER_PASSWORD_HASH: &str = "security.master_password_hash";

    /// Salt for key derivation
    pub const KEY_SALT: &str = "security.key_salt";

    /// Default SSH timeout (seconds)
    pub const SSH_TIMEOUT: &str = "ssh.timeout";

    /// Auto-reconnect setting
    pub const SSH_AUTO_RECONNECT: &str = "ssh.auto_reconnect";
}

impl ConfigRepository {
    /// Create a new config repository
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get a configuration value
    ///
    /// # Arguments
    ///
    /// * `key` - The configuration key
    ///
    /// # Returns
    ///
    /// Returns `Some(value)` if the key exists, `None` otherwise.
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let value: Option<(String,)> = sqlx::query_as("SELECT value FROM app_config WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        Ok(value.map(|v| v.0))
    }

    /// Get a configuration value with a default fallback
    ///
    /// # Arguments
    ///
    /// * `key` - The configuration key
    /// * `default` - The default value to return if key doesn't exist
    pub async fn get_or_default(&self, key: &str, default: &str) -> Result<String> {
        match self.get(key).await? {
            Some(value) => Ok(value),
            None => Ok(default.to_string()),
        }
    }

    /// Get a configuration value or return an error
    ///
    /// # Arguments
    ///
    /// * `key` - The configuration key
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NotFound` if the key doesn't exist.
    pub async fn get_required(&self, key: &str) -> Result<String> {
        match self.get(key).await? {
            Some(value) => Ok(value),
            None => Err(DatabaseError::NotFound {
                entity: "Config".to_string(),
                id: key.to_string(),
            }),
        }
    }

    /// Set a configuration value
    ///
    /// Creates the key if it doesn't exist, updates it otherwise.
    ///
    /// # Arguments
    ///
    /// * `key` - The configuration key
    /// * `value` - The value to store
    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO app_config (key, value) VALUES (?, ?)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Set multiple configuration values
    ///
    /// More efficient than calling `set` multiple times as it uses
    /// a single transaction.
    pub async fn set_many(&self, items: &[(&str, &str)]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await.map_err(DatabaseError::SqlError)?;

        for (key, value) in items {
            sqlx::query(
                r#"
                INSERT INTO app_config (key, value) VALUES (?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value
                "#,
            )
            .bind(*key)
            .bind(*value)
            .execute(&mut *tx)
            .await
            .map_err(DatabaseError::SqlError)?;
        }

        tx.commit().await.map_err(DatabaseError::SqlError)?;

        Ok(())
    }

    /// Delete a configuration value
    ///
    /// # Arguments
    ///
    /// * `key` - The configuration key to delete
    ///
    /// # Returns
    ///
    /// Returns `true` if a value was deleted, `false` if the key didn't exist.
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM app_config WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Check if a configuration key exists
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM app_config WHERE key = ?")
            .bind(key)
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0 > 0)
    }

    /// Get all configuration values
    pub async fn get_all(&self) -> Result<Vec<(String, String)>> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT key, value FROM app_config ORDER BY key ASC")
                .fetch_all(&self.pool)
                .await?;

        Ok(rows)
    }

    /// Get configuration values with keys matching a prefix
    ///
    /// # Arguments
    ///
    /// * `prefix` - The key prefix to match
    pub async fn get_by_prefix(&self, prefix: &str) -> Result<Vec<(String, String)>> {
        let pattern = format!("{}%", prefix);

        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT key, value FROM app_config WHERE key LIKE ? ORDER BY key ASC")
                .bind(&pattern)
                .fetch_all(&self.pool)
                .await?;

        Ok(rows)
    }

    /// Count total configuration entries
    pub async fn count(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM app_config")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    // Convenience methods for common config types

    /// Get a string value
    pub async fn get_string(&self, key: &str) -> Result<Option<String>> {
        self.get(key).await
    }

    /// Get an integer value
    ///
    /// Returns `None` if the key doesn't exist or the value can't be parsed.
    pub async fn get_int(&self, key: &str) -> Result<Option<i64>> {
        match self.get(key).await? {
            Some(value) => match value.parse::<i64>() {
                Ok(n) => Ok(Some(n)),
                Err(_) => Ok(None),
            },
            None => Ok(None),
        }
    }

    /// Get a boolean value
    ///
    /// Recognizes "true", "1", "yes" (case-insensitive) as true.
    /// Everything else is false.
    /// Returns `None` if the key doesn't exist.
    pub async fn get_bool(&self, key: &str) -> Result<Option<bool>> {
        match self.get(key).await? {
            Some(value) => {
                let normalized = value.to_lowercase();
                let bool_value =
                    matches!(normalized.as_str(), "true" | "1" | "yes" | "on" | "enabled");
                Ok(Some(bool_value))
            }
            None => Ok(None),
        }
    }

    /// Set an integer value
    pub async fn set_int(&self, key: &str, value: i64) -> Result<()> {
        self.set(key, &value.to_string()).await
    }

    /// Set a boolean value
    ///
    /// Stores as "true" or "false".
    pub async fn set_bool(&self, key: &str, value: bool) -> Result<()> {
        self.set(key, if value { "true" } else { "false" }).await
    }

    /// Initialize default configuration values
    ///
    /// Sets sensible defaults if they don't already exist.
    pub async fn init_defaults(&self) -> Result<()> {
        let defaults = [
            (keys::THEME, "system"),
            (keys::LANGUAGE, "en"),
            (keys::SSH_TIMEOUT, "30"),
            (keys::SSH_AUTO_RECONNECT, "false"),
        ];

        for (key, value) in &defaults {
            // Only set if not already exists
            if !self.exists(key).await? {
                self.set(key, value).await?;
            }
        }

        // Set first run timestamp if not exists
        if !self.exists(keys::FIRST_RUN_AT).await? {
            let now = chrono::Utc::now().to_rfc3339();
            self.set(keys::FIRST_RUN_AT, &now).await?;
        }

        Ok(())
    }

    /// Clear all configuration
    ///
    /// **Warning**: This deletes all configuration entries. Use with caution.
    pub async fn clear_all(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM app_config")
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::TempDir;

    async fn create_test_db() -> (ConfigRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::new(&db_path).await.unwrap();
        db.init().await.unwrap();

        (db.config_repository(), temp_dir)
    }

    #[tokio::test]
    async fn test_set_and_get() {
        let (repo, _temp) = create_test_db().await;

        repo.set("test.key", "test_value").await.unwrap();

        let value = repo.get("test.key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_update_value() {
        let (repo, _temp) = create_test_db().await;

        repo.set("test.key", "old_value").await.unwrap();
        repo.set("test.key", "new_value").await.unwrap();

        let value = repo.get("test.key").await.unwrap();
        assert_eq!(value, Some("new_value".to_string()));
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let (repo, _temp) = create_test_db().await;

        let value = repo.get("nonexistent.key").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_get_or_default() {
        let (repo, _temp) = create_test_db().await;

        // Non-existent key returns default
        let value = repo
            .get_or_default("nonexistent", "default_val")
            .await
            .unwrap();
        assert_eq!(value, "default_val");

        // Existing key returns stored value
        repo.set("existing.key", "stored_value").await.unwrap();
        let value = repo
            .get_or_default("existing.key", "default_val")
            .await
            .unwrap();
        assert_eq!(value, "stored_value");
    }

    #[tokio::test]
    async fn test_get_required() {
        let (repo, _temp) = create_test_db().await;

        // Non-existent key returns error
        let result = repo.get_required("nonexistent").await;
        assert!(matches!(result, Err(DatabaseError::NotFound { .. })));

        // Existing key returns value
        repo.set("required.key", "required_value").await.unwrap();
        let value = repo.get_required("required.key").await.unwrap();
        assert_eq!(value, "required_value");
    }

    #[tokio::test]
    async fn test_delete() {
        let (repo, _temp) = create_test_db().await;

        repo.set("test.key", "value").await.unwrap();
        assert!(repo.exists("test.key").await.unwrap());

        let deleted = repo.delete("test.key").await.unwrap();
        assert!(deleted);
        assert!(!repo.exists("test.key").await.unwrap());

        // Deleting non-existent key returns false
        let deleted = repo.delete("nonexistent").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_set_many() {
        let (repo, _temp) = create_test_db().await;

        let items = [("key1", "value1"), ("key2", "value2"), ("key3", "value3")];

        repo.set_many(&items).await.unwrap();

        assert_eq!(repo.get("key1").await.unwrap(), Some("value1".to_string()));
        assert_eq!(repo.get("key2").await.unwrap(), Some("value2".to_string()));
        assert_eq!(repo.get("key3").await.unwrap(), Some("value3".to_string()));
    }

    #[tokio::test]
    async fn test_get_all() {
        let (repo, _temp) = create_test_db().await;

        repo.set("key1", "value1").await.unwrap();
        repo.set("key2", "value2").await.unwrap();
        repo.set("aaa_key", "aaa_value").await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 3);

        // Should be sorted by key
        assert_eq!(all[0].0, "aaa_key");
        assert_eq!(all[1].0, "key1");
        assert_eq!(all[2].0, "key2");
    }

    #[tokio::test]
    async fn test_get_by_prefix() {
        let (repo, _temp) = create_test_db().await;

        repo.set("ui.theme", "dark").await.unwrap();
        repo.set("ui.language", "en").await.unwrap();
        repo.set("ui.window.width", "800").await.unwrap();
        repo.set("other.key", "other").await.unwrap();

        let ui_keys = repo.get_by_prefix("ui.").await.unwrap();
        assert_eq!(ui_keys.len(), 3);
    }

    #[tokio::test]
    async fn test_count() {
        let (repo, _temp) = create_test_db().await;

        assert_eq!(repo.count().await.unwrap(), 0);

        repo.set("key1", "value1").await.unwrap();
        assert_eq!(repo.count().await.unwrap(), 1);

        repo.set("key2", "value2").await.unwrap();
        assert_eq!(repo.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_get_int() {
        let (repo, _temp) = create_test_db().await;

        repo.set("number", "42").await.unwrap();
        let value = repo.get_int("number").await.unwrap();
        assert_eq!(value, Some(42));

        repo.set("not_a_number", "abc").await.unwrap();
        let value = repo.get_int("not_a_number").await.unwrap();
        assert_eq!(value, None);

        let value = repo.get_int("nonexistent").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_set_int() {
        let (repo, _temp) = create_test_db().await;

        repo.set_int("number", 42).await.unwrap();

        let value = repo.get("number").await.unwrap();
        assert_eq!(value, Some("42".to_string()));
    }

    #[tokio::test]
    async fn test_get_bool() {
        let (repo, _temp) = create_test_db().await;

        // True values
        for val in &["true", "TRUE", "True", "1", "yes", "YES", "on", "enabled"] {
            repo.set("bool_key", val).await.unwrap();
            let parsed = repo.get_bool("bool_key").await.unwrap();
            assert!(parsed.unwrap(), "Should be true for: {}", val);
        }

        // False values
        for val in &["false", "FALSE", "0", "no", "off", "disabled", "anything"] {
            repo.set("bool_key", val).await.unwrap();
            let parsed = repo.get_bool("bool_key").await.unwrap();
            assert!(!parsed.unwrap(), "Should be false for: {}", val);
        }

        // Nonexistent
        let parsed = repo.get_bool("nonexistent").await.unwrap();
        assert_eq!(parsed, None);
    }

    #[tokio::test]
    async fn test_set_bool() {
        let (repo, _temp) = create_test_db().await;

        repo.set_bool("flag", true).await.unwrap();
        assert_eq!(repo.get("flag").await.unwrap(), Some("true".to_string()));

        repo.set_bool("flag", false).await.unwrap();
        assert_eq!(repo.get("flag").await.unwrap(), Some("false".to_string()));
    }

    #[tokio::test]
    async fn test_init_defaults() {
        let (repo, _temp) = create_test_db().await;

        // Set some existing value
        repo.set(keys::THEME, "existing_theme").await.unwrap();

        repo.init_defaults().await.unwrap();

        // Existing value should not be overwritten
        assert_eq!(
            repo.get(keys::THEME).await.unwrap(),
            Some("existing_theme".to_string())
        );

        // Missing defaults should be set
        assert!(repo.get(keys::FIRST_RUN_AT).await.unwrap().is_some());
        assert_eq!(
            repo.get(keys::LANGUAGE).await.unwrap(),
            Some("en".to_string())
        );
        assert_eq!(repo.get_int(keys::SSH_TIMEOUT).await.unwrap(), Some(30));
        assert_eq!(
            repo.get_bool(keys::SSH_AUTO_RECONNECT).await.unwrap(),
            Some(false)
        );
    }

    #[tokio::test]
    async fn test_clear_all() {
        let (repo, _temp) = create_test_db().await;

        repo.set("key1", "value1").await.unwrap();
        repo.set("key2", "value2").await.unwrap();

        let deleted = repo.clear_all().await.unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(repo.count().await.unwrap(), 0);
    }
}
