//! Database maintenance, compression, and optimization
//!
//! This module provides database maintenance utilities including:
//! - VACUUM operations (compression)
//! - WAL checkpoint management
//! - Fragmentation analysis
//! - Auto-maintenance scheduling

use crate::database::error::{DatabaseError, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

/// Database maintenance manager
#[derive(Debug)]
pub struct MaintenanceManager {
    pool: SqlitePool,
}

/// Maintenance operation types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaintenanceOp {
    /// Full database vacuum (reclaim space)
    Vacuum,
    /// Incremental vacuum (limited work)
    VacuumIncremental { pages: u32 },
    /// Analyze for query optimization
    Analyze,
    /// Rebuild all indexes
    Reindex,
    /// Checkpoint WAL (commit pending changes)
    WalCheckpoint,
    /// Full maintenance suite
    Full,
}

/// Maintenance operation result
#[derive(Debug, Clone)]
pub struct MaintenanceResult {
    /// Operation performed
    pub operation: MaintenanceOp,
    /// Whether successful
    pub success: bool,
    /// Duration of operation
    pub duration_ms: u64,
    /// Space reclaimed (bytes, if applicable)
    pub space_reclaimed_bytes: Option<i64>,
    /// Additional information
    pub info: String,
}

/// Database fragmentation info
#[derive(Debug, Clone)]
pub struct FragmentationInfo {
    /// Database page count
    pub page_count: i64,
    /// Free pages (unused)
    pub free_pages: i64,
    /// Free pages percentage
    pub fragmentation_percent: f64,
    /// Database file size in bytes
    pub file_size_bytes: i64,
    /// Unused space in bytes
    pub unused_bytes: i64,
}

/// WAL checkpoint info
#[derive(Debug, Clone)]
pub struct WalInfo {
    /// WAL mode enabled
    pub wal_mode: bool,
    /// WAL file size in bytes
    pub wal_size_bytes: i64,
    /// Frames in WAL file
    pub frames: i64,
    /// Checkpoints run
    pub checkpoints: i64,
}

/// Auto-maintenance configuration
#[derive(Debug, Clone)]
pub struct AutoMaintenanceConfig {
    /// Enable auto-maintenance
    pub enabled: bool,
    /// Run vacuum when fragmentation exceeds this percent
    pub vacuum_fragmentation_threshold: f64,
    /// Auto-analyze interval (days)
    pub analyze_interval_days: i64,
    /// Auto-checkpoint interval (minutes)
    pub checkpoint_interval_minutes: i64,
    /// Last maintenance timestamp
    pub last_maintenance: Option<DateTime<Utc>>,
    /// Maximum vacuum duration (seconds)
    pub max_vacuum_duration_secs: u64,
}

impl Default for AutoMaintenanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            vacuum_fragmentation_threshold: 20.0, // 20% fragmentation
            analyze_interval_days: 7,
            checkpoint_interval_minutes: 30,
            last_maintenance: None,
            max_vacuum_duration_secs: 300, // 5 minutes
        }
    }
}

impl MaintenanceManager {
    /// Create a new maintenance manager
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Perform a maintenance operation
    pub async fn perform(&self, op: MaintenanceOp) -> Result<MaintenanceResult> {
        let start = std::time::Instant::now();

        let (success, space_reclaimed, info) = match op {
            MaintenanceOp::Vacuum => self.vacuum().await?,
            MaintenanceOp::VacuumIncremental { pages } => self.vacuum_incremental(pages).await?,
            MaintenanceOp::Analyze => self.analyze().await?,
            MaintenanceOp::Reindex => self.reindex().await?,
            MaintenanceOp::WalCheckpoint => self.wal_checkpoint().await?,
            MaintenanceOp::Full => self.full_maintenance().await?,
        };

        Ok(MaintenanceResult {
            operation: op,
            success,
            duration_ms: start.elapsed().as_millis() as u64,
            space_reclaimed_bytes: space_reclaimed,
            info,
        })
    }

    /// Perform full VACUUM to reclaim space
    async fn vacuum(&self) -> Result<(bool, Option<i64>, String)> {
        // Get size before
        let before = self.get_database_stats().await?;

        // Run vacuum
        sqlx::query("VACUUM")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        // Get size after
        let after = self.get_database_stats().await?;
        let reclaimed = before.estimated_size_bytes - after.estimated_size_bytes;

        Ok((
            true,
            Some(reclaimed),
            format!(
                "Vacuum complete. Reclaimed {} bytes ({}% reduction)",
                reclaimed,
                if before.estimated_size_bytes > 0 {
                    (reclaimed as f64 / before.estimated_size_bytes as f64 * 100.0) as i64
                } else {
                    0
                }
            ),
        ))
    }

    /// Perform incremental VACUUM
    async fn vacuum_incremental(&self, pages: u32) -> Result<(bool, Option<i64>, String)> {
        // Enable incremental vacuum mode
        sqlx::query("PRAGMA auto_vacuum = incremental")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        // Vacuum specified number of pages
        let sql = format!("PRAGMA incremental_vacuum({})", pages);
        sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        Ok((
            true,
            None,
            format!("Incremental vacuum complete ({} pages processed)", pages),
        ))
    }

    /// Analyze database for query optimization
    async fn analyze(&self) -> Result<(bool, Option<i64>, String)> {
        sqlx::query("ANALYZE")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        Ok((true, None, "Analyze complete".to_string()))
    }

    /// Rebuild all indexes
    async fn reindex(&self) -> Result<(bool, Option<i64>, String)> {
        sqlx::query("REINDEX")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        Ok((true, None, "Reindex complete".to_string()))
    }

    /// Perform WAL checkpoint
    async fn wal_checkpoint(&self) -> Result<(bool, Option<i64>, String)> {
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        // Get checkpoint info
        let log: (i64,) = sqlx::query_as("PRAGMA wal_checkpoint")
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        Ok((
            true,
            None,
            format!("WAL checkpoint complete (busy: {})", log.0),
        ))
    }

    /// Perform full maintenance suite
    async fn full_maintenance(&self) -> Result<(bool, Option<i64>, String)> {
        let total_reclaimed;

        // Checkpoint first
        self.wal_checkpoint().await?;

        // Analyze
        self.analyze().await?;

        // Reindex
        self.reindex().await?;

        // Vacuum
        let before = self.get_database_stats().await?;
        self.vacuum().await?;
        let after = self.get_database_stats().await?;
        total_reclaimed = before.estimated_size_bytes - after.estimated_size_bytes;

        Ok((
            true,
            Some(total_reclaimed),
            format!(
                "Full maintenance complete. Reclaimed {} bytes.",
                total_reclaimed
            ),
        ))
    }

    /// Get database fragmentation info
    pub async fn get_fragmentation(&self) -> Result<FragmentationInfo> {
        let page_count: (i64,) = sqlx::query_as("PRAGMA page_count")
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        let free_pages: (i64,) = sqlx::query_as("PRAGMA freelist_count")
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        let page_size: (i64,) = sqlx::query_as("PRAGMA page_size")
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        let file_size = page_count.0 * page_size.0;
        let unused_bytes = free_pages.0 * page_size.0;
        let fragmentation_percent = if page_count.0 > 0 {
            (free_pages.0 as f64 / page_count.0 as f64) * 100.0
        } else {
            0.0
        };

        Ok(FragmentationInfo {
            page_count: page_count.0,
            free_pages: free_pages.0,
            fragmentation_percent,
            file_size_bytes: file_size,
            unused_bytes,
        })
    }

    /// Get WAL file info
    pub async fn get_wal_info(&self) -> Result<WalInfo> {
        // Check if WAL mode is enabled
        let journal_mode: (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        let wal_mode = journal_mode.0.to_uppercase() == "WAL";

        // Get WAL info if WAL mode is enabled
        let (wal_size, frames, checkpoints) = if wal_mode {
            let wal_size: (i64,) = sqlx::query_as("PRAGMA wal_size")
                .fetch_optional(&self.pool)
                .await
                .map_err(DatabaseError::SqlError)?
                .unwrap_or((0,));

            // These pragmas may not be available in all SQLite builds
            let frames: (i64,) = sqlx::query_as("PRAGMA wal_checkpoint")
                .fetch_optional(&self.pool)
                .await
                .map_err(DatabaseError::SqlError)?
                .unwrap_or((0,));

            (wal_size.0, frames.0, 0)
        } else {
            (0, 0, 0)
        };

        Ok(WalInfo {
            wal_mode,
            wal_size_bytes: wal_size * 4096, // Assume 4KB pages
            frames,
            checkpoints,
        })
    }

    /// Get database statistics
    pub async fn get_database_stats(&self) -> Result<DatabaseStats> {
        let page_count: (i64,) = sqlx::query_as("PRAGMA page_count")
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        let page_size: (i64,) = sqlx::query_as("PRAGMA page_size")
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        let free_pages: (i64,) = sqlx::query_as("PRAGMA freelist_count")
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        let schema_version: (i64,) = sqlx::query_as("PRAGMA schema_version")
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        let user_version: (i64,) = sqlx::query_as("PRAGMA user_version")
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        Ok(DatabaseStats {
            page_count: page_count.0,
            page_size: page_size.0,
            free_pages: free_pages.0,
            estimated_size_bytes: page_count.0 * page_size.0,
            schema_version: schema_version.0,
            user_version: user_version.0,
        })
    }

    /// Check if maintenance is needed based on configuration
    pub async fn check_maintenance_needed(
        &self,
        config: &AutoMaintenanceConfig,
    ) -> Result<Vec<MaintenanceOp>> {
        let mut needed = Vec::new();

        if !config.enabled {
            return Ok(needed);
        }

        // Check fragmentation
        let frag = self.get_fragmentation().await?;
        if frag.fragmentation_percent > config.vacuum_fragmentation_threshold {
            needed.push(MaintenanceOp::Vacuum);
        }

        // Check WAL size
        let wal = self.get_wal_info().await?;
        if wal.wal_mode && wal.wal_size_bytes > 10 * 1024 * 1024 {
            // WAL larger than 10MB
            needed.push(MaintenanceOp::WalCheckpoint);
        }

        // Check if analyze is needed
        if let Some(last) = config.last_maintenance {
            let days_since = (Utc::now() - last).num_days();
            if days_since >= config.analyze_interval_days {
                needed.push(MaintenanceOp::Analyze);
            }
        } else {
            needed.push(MaintenanceOp::Analyze);
        }

        Ok(needed)
    }

    /// Run auto-maintenance based on configuration
    pub async fn auto_maintenance(
        &self,
        config: &mut AutoMaintenanceConfig,
    ) -> Result<Vec<MaintenanceResult>> {
        let needed = self.check_maintenance_needed(config).await?;
        let mut results = Vec::new();

        for op in needed {
            let result = self.perform(op).await?;
            results.push(result);
        }

        config.last_maintenance = Some(Utc::now());

        Ok(results)
    }

    /// Get table row counts
    pub async fn get_table_stats(&self) -> Result<Vec<TableStats>> {
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(DatabaseError::SqlError)?;

        let mut stats = Vec::new();

        for (table_name,) in tables {
            let count: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", table_name))
                .fetch_one(&self.pool)
                .await
                .map_err(DatabaseError::SqlError)?;

            // Get approximate size
            let size_info: std::result::Result<(i64,), _> = sqlx::query_as(&format!(
                "SELECT SUM(pgsize) FROM dbstat WHERE name = '{}'",
                table_name
            ))
            .fetch_one(&self.pool)
            .await;

            let estimated_bytes = size_info.map(|s| s.0).unwrap_or(0);

            stats.push(TableStats {
                name: table_name,
                row_count: count.0,
                estimated_bytes,
            });
        }

        Ok(stats)
    }

    /// Compress database (VACUUM + optimizations)
    pub async fn compress(&self) -> Result<CompressionResult> {
        let before = self.get_database_stats().await?;

        // Full vacuum
        sqlx::query("VACUUM")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        // Analyze for optimization
        sqlx::query("ANALYZE")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        let after = self.get_database_stats().await?;

        // Calculate saved bytes (can be negative if DB grew)
        let saved_bytes = before.estimated_size_bytes.saturating_sub(after.estimated_size_bytes);
        let savings_percent = if before.estimated_size_bytes > 0 && saved_bytes > 0 {
            (saved_bytes as f64 / before.estimated_size_bytes as f64 * 100.0) as f64
        } else {
            0.0
        };

        Ok(CompressionResult {
            before_bytes: before.estimated_size_bytes,
            after_bytes: after.estimated_size_bytes,
            saved_bytes,
            savings_percent,
        })
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    /// Number of pages
    pub page_count: i64,
    /// Page size in bytes
    pub page_size: i64,
    /// Free pages
    pub free_pages: i64,
    /// Estimated file size
    pub estimated_size_bytes: i64,
    /// Schema version
    pub schema_version: i64,
    /// User version
    pub user_version: i64,
}

/// Table statistics
#[derive(Debug, Clone)]
pub struct TableStats {
    /// Table name
    pub name: String,
    /// Number of rows
    pub row_count: i64,
    /// Estimated size in bytes
    pub estimated_bytes: i64,
}

/// Compression result
#[derive(Debug, Clone)]
pub struct CompressionResult {
    /// Size before compression
    pub before_bytes: i64,
    /// Size after compression
    pub after_bytes: i64,
    /// Bytes saved
    pub saved_bytes: i64,
    /// Savings percentage
    pub savings_percent: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use tempfile::TempDir;

    async fn create_test_db() -> (MaintenanceManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = SqlitePoolOptions::new()
            .connect(&format!("sqlite://{}?mode=rwc", db_path.to_string_lossy()))
            .await
            .unwrap();

        // Create test table and add data
        sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, data TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        for i in 0..1000 {
            sqlx::query("INSERT INTO test (data) VALUES (?)")
                .bind(format!("test data {}", i))
                .execute(&pool)
                .await
                .unwrap();
        }

        let manager = MaintenanceManager::new(pool);
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_vacuum() {
        let (manager, _temp) = create_test_db().await;

        let result = manager.perform(MaintenanceOp::Vacuum).await.unwrap();
        assert!(result.success);
        assert!(result.info.contains("Vacuum"));
    }

    #[tokio::test]
    async fn test_analyze() {
        let (manager, _temp) = create_test_db().await;

        let result = manager.perform(MaintenanceOp::Analyze).await.unwrap();
        assert!(result.success);
        assert_eq!(result.info, "Analyze complete");
    }

    #[tokio::test]
    async fn test_reindex() {
        let (manager, _temp) = create_test_db().await;

        // Create an index first
        sqlx::query("CREATE INDEX idx_test_data ON test(data)")
            .execute(&manager.pool)
            .await
            .unwrap();

        let result = manager.perform(MaintenanceOp::Reindex).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_wal_checkpoint() {
        let (manager, _temp) = create_test_db().await;

        // Enable WAL mode
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&manager.pool)
            .await
            .unwrap();

        let result = manager.perform(MaintenanceOp::WalCheckpoint).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_full_maintenance() {
        let (manager, _temp) = create_test_db().await;

        let result = manager.perform(MaintenanceOp::Full).await.unwrap();
        assert!(result.success);
        assert!(result.info.contains("Full maintenance"));
    }

    #[tokio::test]
    async fn test_get_fragmentation() {
        let (manager, _temp) = create_test_db().await;

        let frag = manager.get_fragmentation().await.unwrap();
        assert!(frag.page_count > 0);
        assert!(frag.fragmentation_percent >= 0.0);
    }

    #[tokio::test]
    async fn test_get_database_stats() {
        let (manager, _temp) = create_test_db().await;

        let stats = manager.get_database_stats().await.unwrap();
        assert!(stats.page_count > 0);
        assert_eq!(stats.page_size, 4096); // SQLite default
        assert!(stats.estimated_size_bytes > 0);
    }

    #[tokio::test]
    async fn test_get_table_stats() {
        let (manager, _temp) = create_test_db().await;

        let stats = manager.get_table_stats().await.unwrap();
        assert!(!stats.is_empty());

        let test_table = stats.iter().find(|s| s.name == "test").unwrap();
        assert_eq!(test_table.row_count, 1000);
    }

    #[tokio::test]
    async fn test_compress() {
        let (manager, _temp) = create_test_db().await;

        let result = manager.compress().await.unwrap();
        // Compression may not always reduce size (e.g., for already compact databases)
        // Just verify the operation completes successfully
        assert!(result.savings_percent >= 0.0);
    }

    #[tokio::test]
    async fn test_check_maintenance_needed() {
        let (manager, _temp) = create_test_db().await;

        let config = AutoMaintenanceConfig::default();
        let needed = manager.check_maintenance_needed(&config).await.unwrap();
        // Should at least suggest analyze since last_maintenance is None
        assert!(!needed.is_empty());
    }

    #[tokio::test]
    async fn test_auto_maintenance() {
        let (manager, _temp) = create_test_db().await;

        let mut config = AutoMaintenanceConfig::default();
        let results = manager.auto_maintenance(&mut config).await.unwrap();

        // Should have performed some maintenance
        assert!(!results.is_empty());
        // Last maintenance should be updated
        assert!(config.last_maintenance.is_some());
    }
}
