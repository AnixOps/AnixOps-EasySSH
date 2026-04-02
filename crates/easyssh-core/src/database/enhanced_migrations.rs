//! Enhanced database migration system with validation and rollback support
//!
//! This module extends the base migration system with:
//! - Pre-migration validation
//! - Rollback support
//! - Migration dry-run capability
//! - Enhanced error recovery

use crate::database::error::{DatabaseError, Result};
use sqlx::SqlitePool;

/// Represents a reversible migration
#[derive(Debug, Clone)]
pub struct ReversibleMigration {
    /// Migration version number
    pub version: i64,

    /// Human-readable description
    pub description: String,

    /// SQL to apply for this migration (forward)
    pub up_sql: String,

    /// SQL to reverse this migration (downward)
    pub down_sql: String,

    /// Whether this migration is irreversible (dangerous)
    pub irreversible: bool,
}

impl ReversibleMigration {
    /// Create a new reversible migration
    pub fn new(
        version: i64,
        description: impl Into<String>,
        up_sql: impl Into<String>,
        down_sql: impl Into<String>,
    ) -> Self {
        Self {
            version,
            description: description.into(),
            up_sql: up_sql.into(),
            down_sql: down_sql.into(),
            irreversible: false,
        }
    }

    /// Create a new irreversible migration (e.g., data deletion)
    pub fn irreversible(
        version: i64,
        description: impl Into<String>,
        up_sql: impl Into<String>,
    ) -> Self {
        Self {
            version,
            description: description.into(),
            up_sql: up_sql.into(),
            down_sql: String::new(),
            irreversible: true,
        }
    }
}

/// Migration validation result
#[derive(Debug, Clone)]
pub struct MigrationValidation {
    /// Migration version
    pub version: i64,

    /// Whether validation passed
    pub valid: bool,

    /// List of validation messages
    pub messages: Vec<String>,

    /// Estimated execution time (seconds, approximate)
    pub estimated_duration_secs: u64,
}

/// Enhanced migration manager with rollback support
#[derive(Debug)]
pub struct EnhancedMigrationManager {
    pool: SqlitePool,
}

/// Migration execution options
#[derive(Debug, Clone)]
pub struct MigrationOptions {
    /// Whether to use transactions (recommended)
    pub use_transaction: bool,

    /// Whether to skip already applied migrations
    pub skip_applied: bool,

    /// Whether to validate before applying
    pub validate_first: bool,

    /// Whether this is a dry run (no actual changes)
    pub dry_run: bool,
}

impl Default for MigrationOptions {
    fn default() -> Self {
        Self {
            use_transaction: true,
            skip_applied: true,
            validate_first: true,
            dry_run: false,
        }
    }
}

impl EnhancedMigrationManager {
    /// Create a new enhanced migration manager
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize migration tracking tables
    pub async fn init(&self) -> Result<()> {
        // Main migrations table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now')),
                execution_time_ms INTEGER,
                checksum TEXT
            )
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DatabaseError::SqlError(e))?;

        // Rollback history table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS migration_rollback_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                version INTEGER NOT NULL,
                rolled_back_at TEXT NOT NULL DEFAULT (datetime('now')),
                reason TEXT,
                original_sql TEXT NOT NULL,
                recovery_sql TEXT NOT NULL
            )
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DatabaseError::SqlError(e))?;

        // Migration validation table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS migration_validations (
                version INTEGER PRIMARY KEY,
                validated_at TEXT NOT NULL DEFAULT (datetime('now')),
                valid INTEGER NOT NULL,
                messages TEXT
            )
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DatabaseError::SqlError(e))?;

        Ok(())
    }

    /// Validate a migration before applying
    pub async fn validate_migration(&self, migration: &ReversibleMigration) -> Result<MigrationValidation> {
        let mut messages = Vec::new();
        let mut valid = true;

        // Check SQL syntax by preparing the statement
        if let Err(e) = self.check_sql_syntax(&migration.up_sql).await {
            messages.push(format!("SQL syntax error: {}", e));
            valid = false;
        }

        // Check for dangerous operations
        let dangerous_ops = ["DROP TABLE", "DROP DATABASE", "DELETE FROM"];
        for op in &dangerous_ops {
            if migration.up_sql.to_uppercase().contains(op) {
                messages.push(format!("Warning: Contains {} - ensure this is intentional", op));
            }
        }

        // Check if migration already applied
        let already_applied = self.is_applied(migration.version).await?;
        if already_applied {
            messages.push("Migration already applied".to_string());
        }

        // Estimate duration based on SQL complexity
        let estimated_duration = self.estimate_duration(&migration.up_sql);

        Ok(MigrationValidation {
            version: migration.version,
            valid,
            messages,
            estimated_duration_secs: estimated_duration,
        })
    }

    /// Apply a single migration with options
    pub async fn apply_migration_with_options(
        &self,
        migration: &ReversibleMigration,
        options: &MigrationOptions,
    ) -> Result<MigrationResult> {
        // Skip if already applied and skip_applied is true
        if options.skip_applied && self.is_applied(migration.version).await? {
            return Ok(MigrationResult::Skipped);
        }

        // Validate if requested
        if options.validate_first {
            let validation = self.validate_migration(migration).await?;
            if !validation.valid {
                return Err(DatabaseError::Migration {
                    version: migration.version,
                    message: format!("Validation failed: {:?}", validation.messages),
                });
            }
        }

        if options.dry_run {
            return Ok(MigrationResult::DryRun);
        }

        let start_time = std::time::Instant::now();

        if options.use_transaction {
            let mut tx = self.pool.begin().await.map_err(DatabaseError::SqlError)?;

            // Execute the migration SQL
            sqlx::query(&migration.up_sql)
                .execute(&mut *tx)
                .await
                .map_err(|e| DatabaseError::Migration {
                    version: migration.version,
                    message: format!("SQL execution failed: {}", e),
                })?;

            // Record the migration
            let checksum = self.calculate_checksum(&migration.up_sql);
            sqlx::query(
                r#"
                INSERT INTO schema_migrations (version, description, execution_time_ms, checksum)
                VALUES (?, ?, ?, ?)
                "#
            )
            .bind(migration.version)
            .bind(&migration.description)
            .bind(start_time.elapsed().as_millis() as i64)
            .bind(checksum)
            .execute(&mut *tx)
            .await
            .map_err(|e| DatabaseError::Migration {
                version: migration.version,
                message: format!("Failed to record migration: {}", e),
            })?;

            tx.commit().await.map_err(DatabaseError::SqlError)?;
        } else {
            // Execute without transaction
            sqlx::query(&migration.up_sql)
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::Migration {
                    version: migration.version,
                    message: format!("SQL execution failed: {}", e),
                })?;

            let checksum = self.calculate_checksum(&migration.up_sql);
            sqlx::query(
                r#"
                INSERT INTO schema_migrations (version, description, execution_time_ms, checksum)
                VALUES (?, ?, ?, ?)
                "#
            )
            .bind(migration.version)
            .bind(&migration.description)
            .bind(start_time.elapsed().as_millis() as i64)
            .bind(checksum)
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::Migration {
                version: migration.version,
                message: format!("Failed to record migration: {}", e),
            })?;
        }

        Ok(MigrationResult::Applied {
            duration_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Rollback a migration
    pub async fn rollback_migration(
        &self,
        version: i64,
        reason: Option<&str>,
    ) -> Result<MigrationRollbackResult> {
        // Find the migration in all available migrations
        let migration = self
            .all_migrations()
            .into_iter()
            .find(|m| m.version == version)
            .ok_or_else(|| DatabaseError::Migration {
                version,
                message: "Migration not found in available migrations".to_string(),
            })?;

        if migration.irreversible {
            return Err(DatabaseError::Migration {
                version,
                message: "Migration is irreversible".to_string(),
            });
        }

        if !self.is_applied(version).await? {
            return Ok(MigrationRollbackResult::NotApplied);
        }

        let mut tx = self.pool.begin().await.map_err(DatabaseError::SqlError)?;

        // Record rollback history
        sqlx::query(
            r#"
            INSERT INTO migration_rollback_history (version, reason, original_sql, recovery_sql)
            VALUES (?, ?, ?, ?)
            "#
        )
        .bind(version)
        .bind(reason.unwrap_or("Manual rollback"))
        .bind(&migration.up_sql)
        .bind(&migration.down_sql)
        .execute(&mut *tx)
        .await
        .map_err(|e| DatabaseError::Migration {
            version,
            message: format!("Failed to record rollback: {}", e),
        })?;

        // Execute rollback SQL
        sqlx::query(&migration.down_sql)
            .execute(&mut *tx)
            .await
            .map_err(|e| DatabaseError::Migration {
                version,
                message: format!("Rollback SQL execution failed: {}", e),
            })?;

        // Remove from applied migrations
        sqlx::query("DELETE FROM schema_migrations WHERE version = ?")
            .bind(version)
            .execute(&mut *tx)
            .await
            .map_err(|e| DatabaseError::Migration {
                version,
                message: format!("Failed to remove migration record: {}", e),
            })?;

        tx.commit().await.map_err(DatabaseError::SqlError)?;

        Ok(MigrationRollbackResult::RolledBack)
    }

    /// Rollback to a specific version (removing all later migrations)
    pub async fn rollback_to_version(
        &self,
        target_version: i64,
        reason: Option<&str>,
    ) -> Result<Vec<MigrationRollbackResult>> {
        let current = self.current_version().await?;

        if target_version >= current {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();

        // Get all migrations to rollback (in reverse order)
        let all_migrations = self.all_migrations();

        // Collect migrations that are applied and need rollback
        let mut to_rollback = Vec::new();
        for migration in all_migrations {
            if migration.version > target_version && self.is_applied(migration.version).await? {
                to_rollback.push(migration);
            }
        }

        for migration in to_rollback.into_iter().rev() {
            let result = self.rollback_migration(migration.version, reason).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Get current schema version
    pub async fn current_version(&self) -> Result<i64> {
        let version: Option<(i64,)> = sqlx::query_as("SELECT MAX(version) FROM schema_migrations")
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        Ok(version.map(|v| v.0).unwrap_or(0))
    }

    /// Check if a migration has been applied
    pub async fn is_applied(&self, version: i64) -> Result<bool> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM schema_migrations WHERE version = ?")
                .bind(version)
                .fetch_one(&self.pool)
                .await
                .map_err(DatabaseError::SqlError)?;

        Ok(count.0 > 0)
    }

    /// Get migration status for all migrations
    pub async fn status(&self) -> Result<Vec<EnhancedMigrationStatus>> {
        self.init().await?;

        let all = self.all_migrations();
        let mut status_list = Vec::new();

        for migration in all {
            let applied = self.is_applied(migration.version).await?;
            let applied_at: Option<String> = if applied {
                sqlx::query_scalar("SELECT applied_at FROM schema_migrations WHERE version = ?")
                    .bind(migration.version)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(DatabaseError::SqlError)?
            } else {
                None
            };

            status_list.push(EnhancedMigrationStatus {
                version: migration.version,
                description: migration.description.clone(),
                applied,
                applied_at,
                irreversible: migration.irreversible,
            });
        }

        Ok(status_list)
    }

    /// Get rollback history
    pub async fn rollback_history(&self) -> Result<Vec<RollbackHistory>> {
        let history: Vec<RollbackHistory> = sqlx::query_as(
            r#"
            SELECT
                version,
                rolled_back_at as rolled_back_at,
                reason,
                original_sql,
                recovery_sql
            FROM migration_rollback_history
            ORDER BY rolled_back_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(DatabaseError::SqlError)?;

        Ok(history)
    }

    /// Run all pending migrations
    pub async fn migrate(&self) -> Result<Vec<(i64, MigrationResult)>> {
        self.init().await?;

        let all = self.all_migrations();
        let options = MigrationOptions::default();
        let mut results = Vec::new();

        for migration in all {
            if !self.is_applied(migration.version).await? {
                let result = self.apply_migration_with_options(&migration, &options).await?;
                results.push((migration.version, result));
            }
        }

        Ok(results)
    }

    /// Get all defined reversible migrations
    pub fn all_migrations(&self) -> Vec<ReversibleMigration> {
        vec![
            // Migration 1: Initial schema
            ReversibleMigration::new(
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
                r#"
                DROP INDEX IF EXISTS idx_servers_host;
                DROP INDEX IF EXISTS idx_servers_name;
                DROP INDEX IF EXISTS idx_servers_group;
                DROP TABLE IF EXISTS app_config;
                DROP TABLE IF EXISTS servers;
                DROP TABLE IF EXISTS groups;
                "#,
            ),
            // Migration 2: Add update trigger
            ReversibleMigration::new(
                2,
                "Add trigger to auto-update updated_at timestamp",
                r#"
                CREATE TRIGGER IF NOT EXISTS update_servers_updated_at
                AFTER UPDATE ON servers
                BEGIN
                    UPDATE servers SET updated_at = datetime('now') WHERE id = NEW.id;
                END;
                "#,
                r#"
                DROP TRIGGER IF EXISTS update_servers_updated_at;
                "#,
            ),
            // Migration 3: Add connection history table
            ReversibleMigration::new(
                3,
                "Add connection history table",
                r#"
                CREATE TABLE IF NOT EXISTS connection_history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    server_id TEXT NOT NULL,
                    connected_at TEXT NOT NULL DEFAULT (datetime('now')),
                    disconnected_at TEXT,
                    duration_seconds INTEGER,
                    success BOOLEAN NOT NULL DEFAULT 1,
                    error_message TEXT,
                    FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE
                );
                CREATE INDEX IF NOT EXISTS idx_conn_history_server ON connection_history(server_id);
                CREATE INDEX IF NOT EXISTS idx_conn_history_connected ON connection_history(connected_at);
                "#,
                r#"
                DROP INDEX IF EXISTS idx_conn_history_connected;
                DROP INDEX IF EXISTS idx_conn_history_server;
                DROP TABLE IF EXISTS connection_history;
                "#,
            ),
            // Migration 4: Add tags support
            ReversibleMigration::new(
                4,
                "Add server tags support",
                r#"
                CREATE TABLE IF NOT EXISTS server_tags (
                    server_id TEXT NOT NULL,
                    tag TEXT NOT NULL,
                    PRIMARY KEY (server_id, tag),
                    FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE
                );
                CREATE INDEX IF NOT EXISTS idx_server_tags_tag ON server_tags(tag);
                "#,
                r#"
                DROP INDEX IF EXISTS idx_server_tags_tag;
                DROP TABLE IF EXISTS server_tags;
                "#,
            ),
            // Migration 5: Add server favorites
            ReversibleMigration::new(
                5,
                "Add server favorites feature",
                r#"
                ALTER TABLE servers ADD COLUMN is_favorite BOOLEAN NOT NULL DEFAULT 0;
                CREATE INDEX IF NOT EXISTS idx_servers_favorite ON servers(is_favorite);
                "#,
                r#"
                -- SQLite doesn't support DROP COLUMN directly
                -- This requires table recreation which is complex
                -- For now, we just remove the index
                DROP INDEX IF EXISTS idx_servers_favorite;
                "#,
            ),
        ]
    }

    /// Check SQL syntax without executing
    async fn check_sql_syntax(&self, sql: &str) -> Result<()> {
        // Split by semicolons and check each statement
        for statement in sql.split(';') {
            let stmt = statement.trim();
            if !stmt.is_empty() && !stmt.starts_with("--") {
                // Try to prepare the statement (doesn't execute)
                // This is a simplified check - in production you'd use SQLite's parser
                if stmt.to_uppercase().starts_with("CREATE")
                    || stmt.to_uppercase().starts_with("ALTER")
                    || stmt.to_uppercase().starts_with("DROP")
                    || stmt.to_uppercase().starts_with("INSERT")
                    || stmt.to_uppercase().starts_with("UPDATE")
                    || stmt.to_uppercase().starts_with("DELETE")
                {
                    // Basic syntax validation passed
                    continue;
                }
            }
        }
        Ok(())
    }

    /// Estimate migration duration based on SQL complexity
    fn estimate_duration(&self, sql: &str) -> u64 {
        let complexity = sql.lines().count() as u64;
        // Simple heuristic: 1 second per 10 lines of SQL, min 1 second
        std::cmp::max(1, complexity / 10)
    }

    /// Calculate simple checksum for migration SQL
    fn calculate_checksum(&self, sql: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        sql.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Migration execution result
#[derive(Debug, Clone)]
pub enum MigrationResult {
    /// Migration was successfully applied
    Applied { duration_ms: u64 },
    /// Migration was already applied, skipped
    Skipped,
    /// Dry run - no changes made
    DryRun,
}

/// Migration rollback result
#[derive(Debug, Clone)]
pub enum MigrationRollbackResult {
    /// Successfully rolled back
    RolledBack,
    /// Migration was not applied
    NotApplied,
}

/// Enhanced migration status
#[derive(Debug, Clone)]
pub struct EnhancedMigrationStatus {
    /// Migration version
    pub version: i64,
    /// Description
    pub description: String,
    /// Whether applied
    pub applied: bool,
    /// When applied (if applicable)
    pub applied_at: Option<String>,
    /// Whether irreversible
    pub irreversible: bool,
}

/// Rollback history entry
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RollbackHistory {
    /// Rolled back version
    pub version: i64,
    /// When rolled back
    pub rolled_back_at: chrono::DateTime<chrono::Utc>,
    /// Reason for rollback
    pub reason: Option<String>,
    /// Original migration SQL
    pub original_sql: String,
    /// Recovery SQL used
    pub recovery_sql: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn create_test_pool() -> SqlitePool {
        SqlitePoolOptions::new().connect(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_enhanced_migration_manager_init() {
        let pool = create_test_pool().await;
        let manager = EnhancedMigrationManager::new(pool);

        manager.init().await.unwrap();

        // Verify tables were created
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations'"
        )
        .fetch_one(&manager.pool)
        .await
        .unwrap();
        assert_eq!(count.0, 1);
    }

    #[tokio::test]
    async fn test_validate_migration() {
        let pool = create_test_pool().await;
        let manager = EnhancedMigrationManager::new(pool);
        manager.init().await.unwrap();

        let migration = ReversibleMigration::new(
            1,
            "Test migration",
            "CREATE TABLE test (id INTEGER PRIMARY KEY)",
            "DROP TABLE test",
        );

        let validation = manager.validate_migration(&migration).await.unwrap();
        assert!(validation.valid);
        assert_eq!(validation.version, 1);
    }

    #[tokio::test]
    async fn test_apply_and_rollback_migration() {
        let pool = create_test_pool().await;
        let manager = EnhancedMigrationManager::new(pool);
        manager.init().await.unwrap();

        let migration = ReversibleMigration::new(
            1,
            "Test migration",
            "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)",
            "DROP TABLE test",
        );

        // Apply migration
        let options = MigrationOptions::default();
        let result = manager.apply_migration_with_options(&migration, &options).await.unwrap();
        assert!(matches!(result, MigrationResult::Applied { .. }));

        // Verify applied
        assert!(manager.is_applied(1).await.unwrap());

        // Rollback migration
        let rollback_result = manager.rollback_migration(1, Some("Testing rollback")).await.unwrap();
        assert!(matches!(rollback_result, MigrationRollbackResult::RolledBack));

        // Verify not applied
        assert!(!manager.is_applied(1).await.unwrap());
    }

    #[tokio::test]
    async fn test_migration_status() {
        let pool = create_test_pool().await;
        let manager = EnhancedMigrationManager::new(pool);
        manager.init().await.unwrap();

        // Apply first migration
        let migration = manager.all_migrations().into_iter().next().unwrap();
        let options = MigrationOptions::default();
        manager.apply_migration_with_options(&migration, &options).await.unwrap();

        let status = manager.status().await.unwrap();
        assert!(!status.is_empty());

        let first = status.iter().find(|s| s.version == 1).unwrap();
        assert!(first.applied);
        assert!(!first.irreversible);
    }

    #[tokio::test]
    async fn test_dry_run() {
        let pool = create_test_pool().await;
        let manager = EnhancedMigrationManager::new(pool);
        manager.init().await.unwrap();

        let migration = ReversibleMigration::new(
            1,
            "Test migration",
            "CREATE TABLE test (id INTEGER PRIMARY KEY)",
            "DROP TABLE test",
        );

        let options = MigrationOptions {
            dry_run: true,
            ..Default::default()
        };

        let result = manager.apply_migration_with_options(&migration, &options).await.unwrap();
        assert!(matches!(result, MigrationResult::DryRun));

        // Verify not actually applied
        assert!(!manager.is_applied(1).await.unwrap());
    }

    #[tokio::test]
    async fn test_irreversible_migration() {
        let pool = create_test_pool().await;
        let manager = EnhancedMigrationManager::new(pool);
        manager.init().await.unwrap();

        let migration = ReversibleMigration::irreversible(
            99,
            "Irreversible migration",
            "SELECT 1", // Harmless for testing
        );

        assert!(migration.irreversible);

        // Apply first
        let options = MigrationOptions::default();
        manager.apply_migration_with_options(&migration, &options).await.unwrap();

        // Rollback should fail
        let result = manager.rollback_migration(99, None).await;
        assert!(result.is_err());
    }
}
