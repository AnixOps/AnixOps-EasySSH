//! Database connection management
//!
//! This module provides the main Database struct which manages the connection
//! pool and provides access to various repositories.

use crate::database::{
    error::Result,
    migrations::{MigrationManager},
    ConfigRepository, GroupRepository, ServerRepository,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;

/// Main database connection manager
///
/// This struct holds the connection pool and provides methods to access
/// individual repositories for different entity types.
///
/// # Example
///
/// ```rust,no_run
/// use easyssh_core::database::Database;
///
/// async fn example() -> anyhow::Result<()> {
///     let db = Database::new("easyssh.db").await?;
///     db.init().await?;
///
///     // Access repositories
///     let servers = db.server_repository();
///     let groups = db.group_repository();
///     let config = db.config_repository();
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Create a new database connection pool
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the SQLite database file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database file cannot be created
    /// - The connection pool cannot be established
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use easyssh_core::database::Database;
    ///
    /// async fn open_db() -> anyhow::Result<()> {
    ///     let db = Database::new("/path/to/db.sqlite").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Build connection options
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(30))
            .foreign_keys(true)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .min_connections(2)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect_with(options)
            .await
            .map_err(|e| crate::database::error::DatabaseError::Connection(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Create a database with an existing pool (for testing)
    #[cfg(test)]
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize the database schema
    ///
    /// Runs all pending migrations to bring the database to the latest version.
    ///
    /// # Errors
    ///
    /// Returns an error if migration fails.
    pub async fn init(&self) -> Result<()> {
        let migration_manager = MigrationManager::new(self.pool.clone());
        migration_manager.migrate().await
    }

    /// Get a server repository
    ///
    /// Returns a `ServerRepository` backed by this database connection pool.
    pub fn server_repository(&self) -> ServerRepository {
        ServerRepository::new(self.pool.clone())
    }

    /// Get a group repository
    ///
    /// Returns a `GroupRepository` backed by this database connection pool.
    pub fn group_repository(&self) -> GroupRepository {
        GroupRepository::new(self.pool.clone())
    }

    /// Get a config repository
    ///
    /// Returns a `ConfigRepository` backed by this database connection pool.
    pub fn config_repository(&self) -> ConfigRepository {
        ConfigRepository::new(self.pool.clone())
    }

    /// Get a raw connection from the pool
    ///
    /// Useful for executing custom queries. Prefer using repositories
    /// for common operations.
    ///
    /// # Errors
    ///
    /// Returns an error if no connection is available within the timeout.
    pub async fn acquire(&self) -> Result<sqlx::pool::PoolConnection<sqlx::Sqlite>> {
        self.pool
            .acquire()
            .await
            .map_err(crate::database::error::DatabaseError::SqlError)
    }

    /// Begin a transaction
    ///
    /// Returns a transaction object that can be used to execute multiple
    /// operations atomically.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use easyssh_core::database::Database;
    ///
    /// async fn transaction_example(db: &Database) -> anyhow::Result<()> {
    ///     let mut tx = db.begin_transaction().await?;
    ///
    ///     // Execute queries using tx...
    ///
    ///     tx.commit().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn begin_transaction(&self) -> Result<sqlx::Transaction<'_, sqlx::Sqlite>> {
        self.pool
            .begin()
            .await
            .map_err(crate::database::error::DatabaseError::SqlError)
    }

    /// Execute a raw SQL query
    ///
    /// This is a low-level method that should be used with caution.
    /// Prefer using repositories for type-safe operations.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the SQL is safe and properly parameterized.
    pub async fn execute(&self, sql: &str) -> Result<sqlx::sqlite::SqliteQueryResult> {
        sqlx::query(sql)
            .execute(&self.pool)
            .await
            .map_err(crate::database::error::DatabaseError::SqlError)
    }

    /// Check if the database connection is healthy
    ///
    /// Performs a simple query to verify connectivity.
    pub async fn health_check(&self) -> Result<bool> {
        let result: std::result::Result<(i64,), _> = sqlx::query_as("SELECT 1")
            .fetch_one(&self.pool)
            .await;

        Ok(result.is_ok())
    }

    /// Get database statistics
    ///
    /// Returns information about the database file and connection pool.
    pub async fn stats(&self) -> Result<DatabaseStats> {
        // Get page count and size
        let page_count: (i64,) = sqlx::query_as("PRAGMA page_count")
            .fetch_one(&self.pool)
            .await
            .map_err(crate::database::error::DatabaseError::SqlError)?;

        let page_size: (i64,) = sqlx::query_as("PRAGMA page_size")
            .fetch_one(&self.pool)
            .await
            .map_err(crate::database::error::DatabaseError::SqlError)?;

        // Get connection pool stats
        let pool_stats = self.pool.size();

        Ok(DatabaseStats {
            page_count: page_count.0,
            page_size: page_size.0,
            estimated_size_bytes: page_count.0 * page_size.0,
            pool_connections: pool_stats as u32,
        })
    }

    /// Close the database connection pool
    ///
    /// Waits for all connections to be returned to the pool and closes them.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    /// Number of pages in the database
    pub page_count: i64,

    /// Size of each page in bytes
    pub page_size: i64,

    /// Estimated database file size
    pub estimated_size_bytes: i64,

    /// Number of connections in the pool
    pub pool_connections: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::new(&db_path).await.unwrap();
        db.init().await.unwrap();

        (db, temp_dir)
    }

    #[tokio::test]
    async fn test_database_new() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::new(&db_path).await.unwrap();
        assert!(db_path.exists());

        // Verify we can get a connection
        assert!(db.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_database_init() {
        let (db, _temp) = create_test_db().await;

        // Health check should pass after init
        assert!(db.health_check().await.unwrap());

        // Tables should exist
        let result = db
            .execute("SELECT 1 FROM servers LIMIT 1")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_server_repository() {
        let (db, _temp) = create_test_db().await;

        let repo = db.server_repository();
        assert_eq!(repo.pool.size(), db.pool.size());
    }

    #[tokio::test]
    async fn test_group_repository() {
        let (db, _temp) = create_test_db().await;

        let repo = db.group_repository();
        assert_eq!(repo.pool.size(), db.pool.size());
    }

    #[tokio::test]
    async fn test_config_repository() {
        let (db, _temp) = create_test_db().await;

        let repo = db.config_repository();
        assert_eq!(repo.pool.size(), db.pool.size());
    }

    #[tokio::test]
    async fn test_transaction() {
        let (db, _temp) = create_test_db().await;

        let tx = db.begin_transaction().await;
        assert!(tx.is_ok());

        let mut tx = tx.unwrap();

        // Execute a simple query in transaction
        let result = sqlx::query("SELECT 1").fetch_one(&mut *tx).await;
        assert!(result.is_ok());

        // Commit the transaction
        let result = tx.commit().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        let (db, _temp) = create_test_db().await;

        assert!(db.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_stats() {
        let (db, _temp) = create_test_db().await;

        let stats = db.stats().await.unwrap();
        assert!(stats.page_count > 0);
        assert_eq!(stats.page_size, 4096); // SQLite default
        assert!(stats.estimated_size_bytes > 0);
        assert!(stats.pool_connections >= 2);
    }

    #[tokio::test]
    async fn test_database_close() {
        let (db, _temp) = create_test_db().await;

        db.close().await;

        // After closing, health check should fail
        // (but we can't easily test this because the pool is gone)
    }
}
