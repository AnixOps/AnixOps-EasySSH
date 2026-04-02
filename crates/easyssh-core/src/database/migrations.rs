//! Database migration management
//!
//! This module handles database schema migrations, ensuring that the database
/// is always at the correct version. Migrations are applied incrementally and
/// tracked in the schema_migrations table.

use crate::database::error::{DatabaseError, Result};
use sqlx::SqlitePool;

/// Represents a single database migration
#[derive(Debug, Clone)]
pub struct Migration {
    /// Migration version number (must be unique and increasing)
    pub version: i64,

    /// Human-readable description
    pub description: String,

    /// SQL to apply for this migration
    pub sql: String,
}

impl Migration {
    /// Create a new migration
    pub fn new(version: i64, description: impl Into<String>, sql: impl Into<String>) -> Self {
        Self {
            version,
            description: description.into(),
            sql: sql.into(),
        }
    }
}

/// Manages database migrations
#[derive(Debug)]
pub struct MigrationManager {
    pool: SqlitePool,
}

impl MigrationManager {
    /// Create a new migration manager
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize the migration tracking table
    ///
    /// Creates the schema_migrations table if it doesn't exist.
    pub async fn init(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::SqlError)?;

        Ok(())
    }

    /// Get the current schema version
    ///
    /// Returns 0 if no migrations have been applied yet.
    pub async fn current_version(&self) -> Result<i64> {
        let version: Option<(i64,)> = sqlx::query_as(
            "SELECT MAX(version) FROM schema_migrations",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::SqlError)?;

        Ok(version.map(|v| v.0).unwrap_or(0))
    }

    /// Check if a specific migration has been applied
    pub async fn is_applied(&self, version: i64) -> Result<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?",
        )
        .bind(version)
        .fetch_one(&self.pool)
        .await
        .map_err(DatabaseError::SqlError)?;

        Ok(count.0 > 0)
    }

    /// Apply a single migration
    ///
    /// The migration is executed within a transaction. If it fails, the
    /// transaction is rolled back and the error is returned.
    pub async fn apply_migration(&self, migration: &Migration) -> Result<()> {
        // Check if already applied
        if self.is_applied(migration.version).await? {
            return Ok(());
        }

        // Execute migration in a transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(DatabaseError::SqlError)?;

        // Execute the migration SQL
        sqlx::query(&migration.sql)
            .execute(&mut *tx)
            .await
            .map_err(|e| DatabaseError::Migration {
                version: migration.version,
                message: format!("SQL execution failed: {}", e),
            })?;

        // Record the migration
        sqlx::query(
            "INSERT INTO schema_migrations (version, description) VALUES (?, ?)",
        )
        .bind(migration.version)
        .bind(&migration.description)
        .execute(&mut *tx)
        .await
        .map_err(|e| DatabaseError::Migration {
            version: migration.version,
            message: format!("Failed to record migration: {}", e),
        })?;

        // Commit the transaction
        tx.commit().await.map_err(DatabaseError::SqlError)?;

        Ok(())
    }

    /// Apply multiple migrations
    ///
    /// Migrations are applied in order of their version numbers.
    /// Skips any migrations that have already been applied.
    pub async fn apply_migrations(&self, migrations: &[Migration]) -> Result<()> {
        // Sort migrations by version
        let mut sorted: Vec<_> = migrations.to_vec();
        sorted.sort_by_key(|m| m.version);

        // Apply each migration
        for migration in sorted {
            self.apply_migration(&migration).await?;
        }

        Ok(())
    }

    /// Run all migrations to bring database to latest version
    pub async fn migrate(&self) -> Result<()> {
        self.init().await?;
        self.apply_migrations(&Self::all_migrations()).await
    }

    /// Get all defined migrations
    pub fn all_migrations() -> Vec<Migration> {
        vec![
            // Migration 1: Initial schema
            Migration::new(
                1,
                "Create initial schema for Lite version",
                r#"
                -- Groups table
                CREATE TABLE IF NOT EXISTS groups (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    color TEXT NOT NULL DEFAULT '#4A90D9',
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                -- Servers table
                CREATE TABLE IF NOT EXISTS servers (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    host TEXT NOT NULL,
                    port INTEGER NOT NULL DEFAULT 22,
                    username TEXT NOT NULL,
                    auth_method TEXT NOT NULL,
                    encrypted_credentials BLOB NOT NULL,
                    group_id TEXT,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE SET NULL
                );

                -- App config table
                CREATE TABLE IF NOT EXISTS app_config (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                -- Indexes
                CREATE INDEX IF NOT EXISTS idx_servers_group ON servers(group_id);
                CREATE INDEX IF NOT EXISTS idx_servers_name ON servers(name);
                CREATE INDEX IF NOT EXISTS idx_servers_host ON servers(host);
                "#,
            ),
            // Migration 2: Add update trigger for servers
            Migration::new(
                2,
                "Add trigger to auto-update updated_at timestamp",
                r#"
                CREATE TRIGGER IF NOT EXISTS update_servers_updated_at
                AFTER UPDATE ON servers
                BEGIN
                    UPDATE servers SET updated_at = datetime('now') WHERE id = NEW.id;
                END;
                "#,
            ),
        ]
    }

    /// Get migration status for all migrations
    pub async fn status(&self) -> Result<Vec<MigrationStatus>> {
        self.init().await?;

        let all = Self::all_migrations();
        let mut status_list = Vec::new();

        for migration in all {
            let applied = self.is_applied(migration.version).await?;
            status_list.push(MigrationStatus {
                version: migration.version,
                description: migration.description,
                applied,
            });
        }

        Ok(status_list)
    }

    /// Reset all migrations (dangerous - for testing only)
    #[cfg(test)]
    pub async fn reset(&self) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(DatabaseError::SqlError)?;

        // Drop all known tables
        sqlx::query("DROP TABLE IF EXISTS servers")
            .execute(&mut *tx)
            .await
            .ok();
        sqlx::query("DROP TABLE IF EXISTS groups")
            .execute(&mut *tx)
            .await
            .ok();
        sqlx::query("DROP TABLE IF EXISTS app_config")
            .execute(&mut *tx)
            .await
            .ok();
        sqlx::query("DROP TABLE IF EXISTS schema_migrations")
            .execute(&mut *tx)
            .await
            .ok();

        tx.commit().await.map_err(DatabaseError::SqlError)?;

        Ok(())
    }
}

/// Status of a migration
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    pub version: i64,
    pub description: String,
    pub applied: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn create_test_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .connect(":memory:")
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_migration_manager_init() {
        let pool = create_test_pool().await;
        let manager = MigrationManager::new(pool);

        manager.init().await.unwrap();
        let version = manager.current_version().await.unwrap();
        assert_eq!(version, 0);
    }

    #[tokio::test]
    async fn test_apply_single_migration() {
        let pool = create_test_pool().await;
        let manager = MigrationManager::new(pool);

        let migration = Migration::new(
            1,
            "Test migration",
            "CREATE TABLE test_table (id INTEGER PRIMARY KEY)",
        );

        manager.init().await.unwrap();
        manager.apply_migration(&migration).await.unwrap();

        let version = manager.current_version().await.unwrap();
        assert_eq!(version, 1);
        assert!(manager.is_applied(1).await.unwrap());
    }

    #[tokio::test]
    async fn test_migration_idempotency() {
        let pool = create_test_pool().await;
        let manager = MigrationManager::new(pool);

        let migration = Migration::new(
            1,
            "Test migration",
            "CREATE TABLE test_table (id INTEGER PRIMARY KEY)",
        );

        manager.init().await.unwrap();

        // Apply twice - should succeed both times
        manager.apply_migration(&migration).await.unwrap();
        manager.apply_migration(&migration).await.unwrap();

        let version = manager.current_version().await.unwrap();
        assert_eq!(version, 1);
    }

    #[tokio::test]
    async fn test_migrate_creates_schema() {
        let pool = create_test_pool().await;
        let manager = MigrationManager::new(pool);

        manager.migrate().await.unwrap();

        let version = manager.current_version().await.unwrap();
        assert!(version >= 1);

        // Check that tables exist by trying to query them
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='servers'")
            .fetch_one(&manager.pool)
            .await
            .unwrap();
        assert_eq!(count.0, 1);
    }

    #[tokio::test]
    async fn test_migration_status() {
        let pool = create_test_pool().await;
        let manager = MigrationManager::new(pool);

        let migrations = vec![
            Migration::new(1, "Migration 1", "SELECT 1"),
            Migration::new(2, "Migration 2", "SELECT 2"),
        ];

        manager.init().await.unwrap();
        manager.apply_migration(&migrations[0]).await.unwrap();

        // Note: this test works with manually applied migrations,
        // not the static all_migrations() list
        let applied = manager.is_applied(1).await.unwrap();
        assert!(applied);
        let not_applied = manager.is_applied(2).await.unwrap();
        assert!(!not_applied);
    }
}
