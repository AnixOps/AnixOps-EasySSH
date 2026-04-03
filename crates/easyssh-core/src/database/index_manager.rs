//! Database index management and optimization
//!
//! This module provides index management utilities for query optimization.
//! It defines recommended indexes and provides tools to analyze query performance.

use sqlx::SqlitePool;

use crate::database::error::{DatabaseError, Result};

/// Represents a database index definition
#[derive(Debug, Clone)]
pub struct IndexDefinition {
    /// Index name
    pub name: &'static str,
    /// Table name
    pub table: &'static str,
    /// Columns to index (comma-separated for composite indexes)
    pub columns: &'static str,
    /// Whether this is a unique index
    pub unique: bool,
    /// Human-readable description
    pub description: &'static str,
}

/// Recommended indexes for optimal query performance
pub const RECOMMENDED_INDEXES: &[IndexDefinition] = &[
    // Server indexes
    IndexDefinition {
        name: "idx_servers_group_id",
        table: "servers",
        columns: "group_id",
        unique: false,
        description: "Speed up filtering servers by group",
    },
    IndexDefinition {
        name: "idx_servers_name",
        table: "servers",
        columns: "name",
        unique: false,
        description: "Speed up searching servers by name",
    },
    IndexDefinition {
        name: "idx_servers_host",
        table: "servers",
        columns: "host",
        unique: false,
        description: "Speed up searching servers by host",
    },
    IndexDefinition {
        name: "idx_servers_created_at",
        table: "servers",
        columns: "created_at",
        unique: false,
        description: "Speed up sorting by creation time",
    },
    IndexDefinition {
        name: "idx_servers_updated_at",
        table: "servers",
        columns: "updated_at",
        unique: false,
        description: "Speed up sorting by update time",
    },
    // Composite index for common search pattern
    IndexDefinition {
        name: "idx_servers_name_host",
        table: "servers",
        columns: "name, host",
        unique: false,
        description: "Composite index for name/host searches",
    },
    // Group indexes
    IndexDefinition {
        name: "idx_groups_name",
        table: "groups",
        columns: "name",
        unique: true,
        description: "Enforce unique group names and speed up lookups",
    },
    IndexDefinition {
        name: "idx_groups_created_at",
        table: "groups",
        columns: "created_at",
        unique: false,
        description: "Speed up sorting groups by creation time",
    },
    // Config indexes
    IndexDefinition {
        name: "idx_app_config_key",
        table: "app_config",
        columns: "key",
        unique: true,
        description: "Enforce unique config keys and speed up lookups",
    },
    // Migration tracking
    IndexDefinition {
        name: "idx_schema_migrations_version",
        table: "schema_migrations",
        columns: "version",
        unique: true,
        description: "Speed up migration version lookups",
    },
];

/// Index manager for creating and managing database indexes
#[derive(Debug)]
pub struct IndexManager {
    pool: SqlitePool,
}

impl IndexManager {
    /// Create a new index manager
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a single index
    pub async fn create_index(&self, index: &IndexDefinition) -> Result<()> {
        let unique_clause = if index.unique { "UNIQUE " } else { "" };
        let sql = format!(
            "CREATE {}INDEX IF NOT EXISTS {} ON {} ({})",
            unique_clause, index.name, index.table, index.columns
        );

        sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        Ok(())
    }

    /// Create all recommended indexes
    pub async fn create_all_indexes(&self) -> Result<usize> {
        let mut created = 0;
        for index in RECOMMENDED_INDEXES {
            self.create_index(index).await?;
            created += 1;
        }
        Ok(created)
    }

    /// Drop a specific index
    pub async fn drop_index(&self, index_name: &str) -> Result<()> {
        let sql = format!("DROP INDEX IF EXISTS {}", index_name);

        sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        Ok(())
    }

    /// Drop all non-essential indexes (for bulk operations)
    pub async fn drop_non_essential_indexes(&self) -> Result<usize> {
        // These indexes can be safely dropped for bulk import/restore operations
        const NON_ESSENTIAL: &[&str] = &[
            "idx_servers_name",
            "idx_servers_host",
            "idx_servers_created_at",
            "idx_servers_updated_at",
            "idx_servers_name_host",
        ];

        let mut dropped = 0;
        for index_name in NON_ESSENTIAL {
            self.drop_index(index_name).await?;
            dropped += 1;
        }
        Ok(dropped)
    }

    /// Check if an index exists
    pub async fn index_exists(&self, index_name: &str) -> Result<bool> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?")
                .bind(index_name)
                .fetch_one(&self.pool)
                .await
                .map_err(DatabaseError::SqlError)?;

        Ok(count.0 > 0)
    }

    /// Get list of existing indexes
    pub async fn list_indexes(&self) -> Result<Vec<IndexInfo>> {
        let rows: Vec<(String, String, String, i64)> = sqlx::query_as(
            r#"
            SELECT
                name,
                tbl_name as table_name,
                sql,
                CASE WHEN sql LIKE '%UNIQUE%' THEN 1 ELSE 0 END as is_unique
            FROM sqlite_master
            WHERE type = 'index' AND name NOT LIKE 'sqlite_%'
            ORDER BY tbl_name, name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(DatabaseError::SqlError)?;

        Ok(rows
            .into_iter()
            .map(|(name, table, sql, unique)| IndexInfo {
                name,
                table,
                sql,
                unique: unique > 0,
            })
            .collect())
    }

    /// Analyze query plan for a given SQL query
    ///
    /// This is useful for debugging slow queries
    pub async fn analyze_query(&self, sql: &str) -> Result<Vec<QueryPlan>> {
        let explain_sql = format!("EXPLAIN QUERY PLAN {}", sql);
        let rows: Vec<(i64, i64, i64, String)> = sqlx::query_as(&explain_sql)
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        Ok(rows
            .into_iter()
            .map(|(id, parent, detail_id, detail)| QueryPlan {
                id,
                parent,
                detail_id,
                detail,
            })
            .collect())
    }

    /// Run ANALYZE to update statistics for query optimizer
    pub async fn analyze(&self) -> Result<()> {
        sqlx::query("ANALYZE")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        Ok(())
    }

    /// Get index usage statistics (requires SQLite compiled with SQLITE_ENABLE_STMTVTAB)
    pub async fn get_index_stats(&self) -> Result<Vec<IndexStat>> {
        // This uses the dbstat virtual table if available
        let result: Result<Vec<IndexStat>> = sqlx::query_as(
            r#"
            SELECT
                name,
                COUNT(*) as pages,
                SUM(pgsize) as bytes_used
            FROM sqlite_dbpage
            JOIN sqlite_schema ON sqlite_dbpage.pgno = sqlite_schema.rootpage
            WHERE type = 'index'
            GROUP BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.into());

        // If dbstat is not available, return empty list
        match result {
            Ok(stats) => Ok(stats),
            Err(_) => Ok(Vec::new()),
        }
    }

    /// Rebuild all indexes (useful after bulk operations)
    pub async fn rebuild_indexes(&self) -> Result<()> {
        sqlx::query("REINDEX")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::SqlError)?;

        Ok(())
    }
}

/// Information about a database index
#[derive(Debug, Clone)]
pub struct IndexInfo {
    /// Index name
    pub name: String,
    /// Table name
    pub table: String,
    /// SQL used to create the index
    pub sql: String,
    /// Whether this is a unique index
    pub unique: bool,
}

/// Query plan information from EXPLAIN QUERY PLAN
#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// Plan step ID
    pub id: i64,
    /// Parent step ID
    pub parent: i64,
    /// Detail ID
    pub detail_id: i64,
    /// Plan detail description
    pub detail: String,
}

/// Index usage statistics
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct IndexStat {
    /// Index name
    pub name: String,
    /// Number of pages used
    pub pages: i64,
    /// Bytes used by the index
    pub bytes_used: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn create_test_pool() -> SqlitePool {
        SqlitePoolOptions::new().connect(":memory:").await.unwrap()
    }

    async fn create_test_table(pool: &SqlitePool) {
        sqlx::query(
            r#"
            CREATE TABLE test_table (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_create_index() {
        let pool = create_test_pool().await;
        create_test_table(&pool).await;

        let manager = IndexManager::new(pool);

        let index = IndexDefinition {
            name: "idx_test_name",
            table: "test_table",
            columns: "name",
            unique: false,
            description: "Test index",
        };

        manager.create_index(&index).await.unwrap();
        assert!(manager.index_exists("idx_test_name").await.unwrap());
    }

    #[tokio::test]
    async fn test_drop_index() {
        let pool = create_test_pool().await;
        create_test_table(&pool).await;

        let manager = IndexManager::new(pool);

        let index = IndexDefinition {
            name: "idx_test_name",
            table: "test_table",
            columns: "name",
            unique: false,
            description: "Test index",
        };

        manager.create_index(&index).await.unwrap();
        assert!(manager.index_exists("idx_test_name").await.unwrap());

        manager.drop_index("idx_test_name").await.unwrap();
        assert!(!manager.index_exists("idx_test_name").await.unwrap());
    }

    #[tokio::test]
    async fn test_list_indexes() {
        let pool = create_test_pool().await;
        create_test_table(&pool).await;

        let manager = IndexManager::new(pool);

        // Create an index
        let index = IndexDefinition {
            name: "idx_test_name",
            table: "test_table",
            columns: "name",
            unique: false,
            description: "Test index",
        };
        manager.create_index(&index).await.unwrap();

        let indexes = manager.list_indexes().await.unwrap();
        assert!(!indexes.is_empty());
        assert!(indexes.iter().any(|i| i.name == "idx_test_name"));
    }

    #[tokio::test]
    async fn test_analyze_query() {
        let pool = create_test_pool().await;
        create_test_table(&pool).await;

        let manager = IndexManager::new(pool);

        let plan = manager
            .analyze_query("SELECT * FROM test_table WHERE name = 'test'")
            .await
            .unwrap();
        assert!(!plan.is_empty());
    }

    #[tokio::test]
    async fn test_analyze() {
        let pool = create_test_pool().await;
        create_test_table(&pool).await;

        let manager = IndexManager::new(pool);
        manager.analyze().await.unwrap();
        // No error = success
    }

    #[tokio::test]
    async fn test_rebuild_indexes() {
        let pool = create_test_pool().await;
        create_test_table(&pool).await;

        let manager = IndexManager::new(pool);

        // Create an index first
        let index = IndexDefinition {
            name: "idx_test_name",
            table: "test_table",
            columns: "name",
            unique: false,
            description: "Test index",
        };
        manager.create_index(&index).await.unwrap();

        // Rebuild should succeed
        manager.rebuild_indexes().await.unwrap();
    }
}
