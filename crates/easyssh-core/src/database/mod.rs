//! EasySSH Lite Database Module
//!
//! This module provides asynchronous SQLite database operations using sqlx.
//! It implements the storage layer for Lite version with:
//! - Server configuration storage
//! - Group management
//! - Application configuration
//! - Migration management
//! - Index management and optimization
//! - Backup and restore functionality
//! - Database maintenance and compression
//! - Query optimization and performance monitoring
//! - Enhanced migrations with rollback support
//!
//! # Example
//!
//! ```rust,no_run
//! use easyssh_core::database::{Database, ServerRepository, GroupRepository};
//! use easyssh_core::database::Result;
//!
//! async fn example() -> Result<()> {
//!     // Initialize database
//!     let db = Database::new("easyssh.db").await?;
//!     db.init().await?;
//!
//!     // Use repositories
//!     let servers = db.server_repository();
//!     let groups = db.group_repository();
//!
//!     Ok(())
//! }
//! ```

mod backup;
mod config_repository;
#[allow(clippy::module_inception)]
mod database;
mod enhanced_migrations;
mod error;
mod group_repository;
mod index_manager;
mod maintenance;
mod migrations;
mod models;
mod query_optimizer;
mod server_repository;

pub use backup::{AutoBackupConfig, BackupInfo, BackupManager, BackupStrategy};
pub use config_repository::ConfigRepository;
pub use database::{Database, DatabaseStats};
pub use enhanced_migrations::{
    EnhancedMigrationManager, EnhancedMigrationStatus, MigrationOptions, MigrationResult,
    MigrationRollbackResult, ReversibleMigration, RollbackHistory,
};
pub use error::{DatabaseError, Result};
pub use group_repository::{GroupRepository, GroupWithCount};
pub use index_manager::{IndexDefinition, IndexInfo, IndexManager, QueryPlan, RECOMMENDED_INDEXES};
pub use maintenance::{
    AutoMaintenanceConfig, CompressionResult, FragmentationInfo, MaintenanceManager, MaintenanceOp,
    MaintenanceResult, TableStats, WalInfo,
};
pub use migrations::{Migration, MigrationManager, MigrationStatus};
pub use models::{
    AppConfig, Group, NewGroup, NewServer, QueryOptions, Server, ServerFilters, ServerWithGroup,
    UpdateGroup, UpdateServer,
};
pub use query_optimizer::{
    IndexRecommendation, IndexUsage, OptimizationSuggestion, PerformanceConfig, PerformanceReport,
    Priority, QueryMetrics, QueryMonitor, QueryOptimizer, SuggestionSeverity,
};
pub use server_repository::ServerRepository;

use std::path::PathBuf;

/// Get the default database path for the application.
///
/// Returns the platform-appropriate path for the EasySSH database file.
/// On most systems, this will be in the user's data directory under
/// `EasySSH/easyssh.db`.
///
/// # Example
///
/// ```rust
/// use easyssh_core::database::get_default_db_path;
///
/// let path = get_default_db_path();
/// println!("Database path: {:?}", path);
/// ```
pub fn get_default_db_path() -> PathBuf {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    data_dir.join("EasySSH").join("easyssh.db")
}

/// Get the default backup directory path.
///
/// Returns the platform-appropriate path for database backups.
/// On most systems, this will be in the user's data directory under
/// `EasySSH/backups/`.
///
/// # Example
///
/// ```rust
/// use easyssh_core::database::get_default_backup_dir;
///
/// let path = get_default_backup_dir();
/// println!("Backup directory: {:?}", path);
/// ```
pub fn get_default_backup_dir() -> PathBuf {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    data_dir.join("EasySSH").join("backups")
}

/// Ensure the database directory exists.
///
/// Creates the parent directories for the database file if they don't exist.
///
/// # Errors
///
/// Returns `DatabaseError::Io` if directory creation fails.
pub fn ensure_db_directory(path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Database health check result
#[derive(Debug, Clone)]
pub struct DatabaseHealth {
    /// Whether database connection is healthy
    pub connected: bool,
    /// Database file exists
    pub file_exists: bool,
    /// Schema version
    pub schema_version: i64,
    /// Database size in bytes
    pub size_bytes: i64,
    /// Last backup timestamp (if available)
    pub last_backup: Option<chrono::DateTime<chrono::Utc>>,
    /// Any issues detected
    pub issues: Vec<String>,
}

/// Comprehensive database manager
///
/// This struct provides access to all database management utilities
/// including repositories, maintenance, backup, and optimization features.
#[derive(Debug)]
pub struct DatabaseManager {
    /// Connection pool
    pool: sqlx::SqlitePool,
    /// Database file path
    db_path: PathBuf,
}

impl DatabaseManager {
    /// Create a new database manager
    pub async fn new(db_path: impl AsRef<std::path::Path>) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create connection pool
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(30))
            .foreign_keys(true)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(10)
            .min_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect_with(options)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        Ok(Self { pool, db_path })
    }

    /// Initialize the database
    pub async fn init(&self) -> Result<()> {
        // Run migrations
        let migration_manager = EnhancedMigrationManager::new(self.pool.clone());
        migration_manager.migrate().await?;

        // Create recommended indexes
        let index_manager = IndexManager::new(self.pool.clone());
        index_manager.create_all_indexes().await?;

        // Initialize default config
        let config_repo = ConfigRepository::new(self.pool.clone());
        config_repo.init_defaults().await?;

        Ok(())
    }

    /// Get the connection pool
    pub fn pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }

    /// Get server repository
    pub fn server_repository(&self) -> ServerRepository {
        ServerRepository::new(self.pool.clone())
    }

    /// Get group repository
    pub fn group_repository(&self) -> GroupRepository {
        GroupRepository::new(self.pool.clone())
    }

    /// Get config repository
    pub fn config_repository(&self) -> ConfigRepository {
        ConfigRepository::new(self.pool.clone())
    }

    /// Get backup manager
    pub fn backup_manager(&self) -> BackupManager {
        BackupManager::new(self.pool.clone(), &self.db_path)
    }

    /// Get maintenance manager
    pub fn maintenance_manager(&self) -> MaintenanceManager {
        MaintenanceManager::new(self.pool.clone())
    }

    /// Get index manager
    pub fn index_manager(&self) -> IndexManager {
        IndexManager::new(self.pool.clone())
    }

    /// Get query optimizer
    pub fn query_optimizer(&self) -> QueryOptimizer {
        QueryOptimizer::new(self.pool.clone())
    }

    /// Get migration manager
    pub fn migration_manager(&self) -> EnhancedMigrationManager {
        EnhancedMigrationManager::new(self.pool.clone())
    }

    /// Perform health check
    pub async fn health_check(&self) -> Result<DatabaseHealth> {
        let mut issues = Vec::new();

        // Check connection
        let connected = self.pool.acquire().await.map(|_| true).unwrap_or_else(|e| {
            issues.push(format!("Connection failed: {}", e));
            false
        });

        // Check file existence
        let file_exists = std::path::Path::new(&self.db_path).exists();
        if !file_exists {
            issues.push("Database file does not exist".to_string());
        }

        // Get schema version
        let schema_version = if connected {
            sqlx::query_as::<_, (i64,)>("SELECT MAX(version) FROM schema_migrations")
                .fetch_optional(&self.pool)
                .await
                .map(|v| v.map(|r| r.0).unwrap_or(0))
                .unwrap_or(0)
        } else {
            0
        };

        // Get size
        let size_bytes = std::fs::metadata(&self.db_path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);

        // Check for large WAL file
        let wal_path = self.db_path.with_extension("db-wal");
        if let Ok(meta) = std::fs::metadata(&wal_path) {
            let wal_size = meta.len();
            if wal_size > 100 * 1024 * 1024 {
                // > 100MB
                issues.push(format!("Large WAL file: {} bytes", wal_size));
            }
        }

        Ok(DatabaseHealth {
            connected,
            file_exists,
            schema_version,
            size_bytes,
            last_backup: None, // Would be populated from config
            issues,
        })
    }

    /// Close database connections
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_get_default_db_path() {
        let path = get_default_db_path();
        assert!(path.to_string_lossy().contains("EasySSH"));
        assert!(path.to_string_lossy().contains("easyssh.db"));
    }

    #[tokio::test]
    async fn test_get_default_backup_dir() {
        let path = get_default_backup_dir();
        assert!(path.to_string_lossy().contains("EasySSH"));
        assert!(path.to_string_lossy().contains("backups"));
    }

    #[tokio::test]
    async fn test_ensure_db_directory() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("subdir").join("database.db");

        ensure_db_directory(&path).unwrap();
        assert!(path.parent().unwrap().exists());
    }

    #[tokio::test]
    async fn test_database_manager_new() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let manager = DatabaseManager::new(&db_path).await.unwrap();
        assert!(db_path.exists());

        manager.close().await;
    }

    #[tokio::test]
    async fn test_database_manager_init() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let manager = DatabaseManager::new(&db_path).await.unwrap();
        manager.init().await.unwrap();

        // Verify tables exist
        let repo = manager.server_repository();
        let count = repo.count().await.unwrap();
        assert_eq!(count, 0);

        manager.close().await;
    }

    #[tokio::test]
    async fn test_database_health_check() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let manager = DatabaseManager::new(&db_path).await.unwrap();
        manager.init().await.unwrap();

        let health = manager.health_check().await.unwrap();
        assert!(health.connected);
        assert!(health.file_exists);
        assert!(health.issues.is_empty());

        manager.close().await;
    }
}
