//! Query optimization and performance monitoring
//!
//! This module provides query optimization utilities:
//! - Query plan analysis
//! - Slow query detection
//! - Query caching
//! - Performance metrics

use crate::database::error::{DatabaseError, Result};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Query optimizer for performance analysis and optimization
#[derive(Debug)]
pub struct QueryOptimizer {
    pool: SqlitePool,
}

/// Query plan analysis result
#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// Query plan steps
    pub steps: Vec<PlanStep>,
    /// Whether query uses indexes
    pub uses_index: bool,
    /// Estimated cost (lower is better)
    pub estimated_cost: f64,
    /// Full scan detected
    pub full_scan: bool,
}

/// Single step in query plan
#[derive(Debug, Clone)]
pub struct PlanStep {
    /// Step ID
    pub id: i64,
    /// Parent step ID
    pub parent: i64,
    /// Not used column
    pub not_used: i64,
    /// Plan detail
    pub detail: String,
}

/// Query performance metrics
#[derive(Debug, Clone)]
pub struct QueryMetrics {
    /// Query text (sanitized)
    pub query: String,
    /// Execution count
    pub execution_count: u64,
    /// Total execution time
    pub total_duration: Duration,
    /// Average execution time
    pub avg_duration: Duration,
    /// Maximum execution time
    pub max_duration: Duration,
    /// Minimum execution time
    pub min_duration: Duration,
    /// Last executed timestamp
    pub last_executed: Option<Instant>,
}

/// Performance threshold configuration
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Slow query threshold (milliseconds)
    pub slow_query_threshold_ms: u64,
    /// Very slow query threshold (milliseconds)
    pub very_slow_threshold_ms: u64,
    /// Enable query caching
    pub enable_query_cache: bool,
    /// Cache size (number of queries)
    pub cache_size: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            slow_query_threshold_ms: 100,
            very_slow_threshold_ms: 1000,
            enable_query_cache: true,
            cache_size: 100,
        }
    }
}

/// Query cache entry
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Query text
    query: String,
    /// Cached results (serialized)
    results: Vec<u8>,
    /// Cached at timestamp
    cached_at: Instant,
    /// TTL seconds
    ttl_secs: u64,
}

/// Query performance monitor
#[derive(Debug)]
pub struct QueryMonitor {
    pool: SqlitePool,
    metrics: HashMap<String, QueryMetrics>,
    config: PerformanceConfig,
    cache: HashMap<String, CacheEntry>,
}

impl QueryOptimizer {
    /// Create a new query optimizer
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Analyze query execution plan
    pub async fn analyze(&self, query: &str) -> Result<QueryPlan> {
        let explain_sql = format!("EXPLAIN QUERY PLAN {}", query);

        let rows: Vec<(i64, i64, i64, String)> = sqlx::query_as(&explain_sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DatabaseError::SqlError(e))?;

        let steps: Vec<PlanStep> = rows
            .into_iter()
            .map(|(id, parent, not_used, detail)| PlanStep {
                id,
                parent,
                not_used,
                detail: detail.clone(),
            })
            .collect();

        // Analyze plan for optimization opportunities
        let uses_index = steps.iter().any(|s| {
            s.detail.to_uppercase().contains("INDEX")
                || s.detail.to_uppercase().contains("USING")
        });

        let full_scan = steps.iter().any(|s| {
            s.detail.to_uppercase().contains("SCAN TABLE")
                && !s.detail.to_uppercase().contains("USING INDEX")
        });

        // Estimate cost (simplified)
        let estimated_cost = if full_scan {
            1000.0 // High cost for full scan
        } else if uses_index {
            10.0 // Low cost for indexed access
        } else {
            100.0 // Medium cost
        };

        Ok(QueryPlan {
            steps,
            uses_index,
            estimated_cost,
            full_scan,
        })
    }

    /// Get optimization suggestions for a query
    pub async fn suggest_optimizations(&self, query: &str) -> Result<Vec<OptimizationSuggestion>> {
        let plan = self.analyze(query).await?;
        let mut suggestions = Vec::new();

        if plan.full_scan {
            suggestions.push(OptimizationSuggestion {
                severity: SuggestionSeverity::Warning,
                message: "Query performs full table scan".to_string(),
                recommendation: "Add an index on frequently filtered columns".to_string(),
            });
        }

        if !plan.uses_index && query.to_uppercase().contains("WHERE") {
            suggestions.push(OptimizationSuggestion {
                severity: SuggestionSeverity::Info,
                message: "Query does not use indexes".to_string(),
                recommendation: "Consider adding indexes for WHERE clause columns".to_string(),
            });
        }

        if query.to_uppercase().contains("ORDER BY") && !query.to_uppercase().contains("INDEX") {
            suggestions.push(OptimizationSuggestion {
                severity: SuggestionSeverity::Info,
                message: "ORDER BY without index may be slow for large datasets".to_string(),
                recommendation: "Consider adding an index on ORDER BY columns".to_string(),
            });
        }

        // Check for common anti-patterns
        let upper = query.to_uppercase();
        if upper.contains("SELECT *") {
            suggestions.push(OptimizationSuggestion {
                severity: SuggestionSeverity::Info,
                message: "SELECT * may return unnecessary columns".to_string(),
                recommendation: "Specify only needed columns for better performance".to_string(),
            });
        }

        if upper.contains("LIKE '%") {
            suggestions.push(OptimizationSuggestion {
                severity: SuggestionSeverity::Warning,
                message: "Leading wildcard LIKE pattern prevents index usage".to_string(),
                recommendation: "Consider using FTS (Full Text Search) for text search".to_string(),
            });
        }

        Ok(suggestions)
    }

    /// Check if a query is using an index efficiently
    pub async fn is_indexed(&self, query: &str) -> Result<bool> {
        let plan = self.analyze(query).await?;
        Ok(plan.uses_index)
    }

    /// Get database index usage statistics
    pub async fn get_index_usage_stats(&self) -> Result<Vec<IndexUsage>> {
        // Note: This requires SQLite compiled with SQLITE_ENABLE_DBSTAT_VTAB
        // If not available, returns empty results
        let result: Result<Vec<IndexUsage>> = sqlx::query_as(
            r#"
            SELECT
                name,
                tbl_name as table_name,
                SUM(pgsize) as total_bytes,
                COUNT(*) as pages
            FROM dbstat
            JOIN sqlite_master ON dbstat.name = sqlite_master.name
            WHERE type = 'index'
            GROUP BY name
            ORDER BY total_bytes DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.into());

        match result {
            Ok(stats) => Ok(stats),
            Err(_) => {
                // dbstat not available, try alternative approach
                let indexes: Vec<(String, String)> = sqlx::query_as(
                    "SELECT name, tbl_name FROM sqlite_master WHERE type = 'index'"
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| DatabaseError::SqlError(e))?;

                Ok(indexes
                    .into_iter()
                    .map(|(name, table)| IndexUsage {
                        name,
                        table_name: table,
                        total_bytes: 0,
                        pages: 0,
                    })
                    .collect())
            }
        }
    }

    /// Recommend indexes based on query patterns
    pub async fn recommend_indexes(&self) -> Result<Vec<IndexRecommendation>> {
        let mut recommendations = Vec::new();

        // Get all tables
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DatabaseError::SqlError(e))?;

        for (table,) in tables {
            // Get table info
            let columns: Vec<(i64, String, String, i64, Option<String>, i64)> = sqlx::query_as(
                &format!("PRAGMA table_info({})", table)
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DatabaseError::SqlError(e))?;

            // Check for foreign key columns
            for (_, name, _, _, _, _) in &columns {
                if name.ends_with("_id") && !name.starts_with("id") {
                    recommendations.push(IndexRecommendation {
                        table: table.clone(),
                        columns: vec![name.clone()],
                        reason: format!("Foreign key column: {}", name),
                        priority: Priority::High,
                    });
                }
            }

            // Check for name columns (often searched)
            if columns.iter().any(|(_, name, _, _, _, _)| name == "name") {
                recommendations.push(IndexRecommendation {
                    table: table.clone(),
                    columns: vec!["name".to_string()],
                    reason: "Common search column".to_string(),
                    priority: Priority::Medium,
                });
            }
        }

        // Remove duplicates
        recommendations.sort_by(|a, b| {
            a.table
                .cmp(&b.table)
                .then_with(|| a.columns.join(",").cmp(&b.columns.join(",")))
        });
        recommendations.dedup_by(|a, b| {
            a.table == b.table && a.columns.join(",") == b.columns.join(",")
        });

        Ok(recommendations)
    }
}

/// Optimization suggestion
#[derive(Debug, Clone)]
pub struct OptimizationSuggestion {
    /// Severity level
    pub severity: SuggestionSeverity,
    /// Problem description
    pub message: String,
    /// Recommended solution
    pub recommendation: String,
}

/// Suggestion severity
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SuggestionSeverity {
    /// Information only
    Info,
    /// Warning - performance impact
    Warning,
    /// Critical - severe performance issue
    Critical,
}

/// Index usage statistics
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct IndexUsage {
    /// Index name
    pub name: String,
    /// Table name
    pub table_name: String,
    /// Total bytes used
    pub total_bytes: i64,
    /// Number of pages
    pub pages: i64,
}

/// Index recommendation
#[derive(Debug, Clone)]
pub struct IndexRecommendation {
    /// Target table
    pub table: String,
    /// Columns to index
    pub columns: Vec<String>,
    /// Reason for recommendation
    pub reason: String,
    /// Priority level
    pub priority: Priority,
}

/// Priority levels
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl QueryMonitor {
    /// Create a new query monitor
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            metrics: HashMap::new(),
            config: PerformanceConfig::default(),
            cache: HashMap::new(),
        }
    }

    /// Record query execution time
    pub fn record_query(&mut self, query: &str, duration: Duration) {
        let sanitized = self.sanitize_query(query);

        let metrics = self.metrics.entry(sanitized.clone()).or_insert_with(|| {
            QueryMetrics {
                query: sanitized.clone(),
                execution_count: 0,
                total_duration: Duration::ZERO,
                avg_duration: Duration::ZERO,
                max_duration: Duration::ZERO,
                min_duration: Duration::MAX,
                last_executed: None,
            }
        });

        metrics.execution_count += 1;
        metrics.total_duration += duration;
        metrics.avg_duration = metrics.total_duration / metrics.execution_count as u32;

        if duration > metrics.max_duration {
            metrics.max_duration = duration;
        }
        if duration < metrics.min_duration {
            metrics.min_duration = duration;
        }

        metrics.last_executed = Some(Instant::now());

        // Log slow queries
        let duration_ms = duration.as_millis() as u64;
        if duration_ms >= self.config.very_slow_threshold_ms {
            tracing::warn!(
                "Very slow query ({}ms): {}",
                duration_ms,
                &sanitized[..sanitized.len().min(100)]
            );
        } else if duration_ms >= self.config.slow_query_threshold_ms {
            tracing::warn!(
                "Slow query ({}ms): {}",
                duration_ms,
                &sanitized[..sanitized.len().min(100)]
            );
        }
    }

    /// Get query metrics
    pub fn get_metrics(&self) -> &HashMap<String, QueryMetrics> {
        &self.metrics
    }

    /// Get slow queries (above threshold)
    pub fn get_slow_queries(&self) -> Vec<&QueryMetrics> {
        self.metrics
            .values()
            .filter(|m| {
                m.avg_duration.as_millis() as u64 >= self.config.slow_query_threshold_ms
            })
            .collect()
    }

    /// Get most frequently executed queries
    pub fn get_hot_queries(&self, limit: usize) -> Vec<&QueryMetrics> {
        let mut metrics: Vec<&QueryMetrics> = self.metrics.values().collect();
        metrics.sort_by(|a, b| b.execution_count.cmp(&a.execution_count));
        metrics.into_iter().take(limit).collect()
    }

    /// Clear metrics
    pub fn clear_metrics(&mut self) {
        self.metrics.clear();
    }

    /// Sanitize query for metrics (remove parameters)
    fn sanitize_query(&self, query: &str) -> String {
        // Replace string literals with placeholder
        let mut result = query.to_string();

        // Simple regex-like replacement for string literals
        let mut in_string = false;
        let mut escaped = false;
        let mut output = String::new();

        for ch in result.chars() {
            if escaped {
                escaped = false;
                if !in_string {
                    output.push(ch);
                }
                continue;
            }

            if ch == '\\' && in_string {
                escaped = true;
                continue;
            }

            if ch == '\'' {
                in_string = !in_string;
                if !in_string {
                    output.push_str("'?'"); // Placeholder
                }
                continue;
            }

            if !in_string {
                output.push(ch);
            }
        }

        // Collapse whitespace
        output.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Generate performance report
    pub fn generate_report(&self) -> PerformanceReport {
        let total_queries: u64 = self.metrics.values().map(|m| m.execution_count).sum();

        let total_duration: Duration = self
            .metrics
            .values()
            .map(|m| m.total_duration)
            .fold(Duration::ZERO, |acc, d| acc + d);

        let avg_duration = if total_queries > 0 {
            total_duration / total_queries as u32
        } else {
            Duration::ZERO
        };

        let slow_queries = self.get_slow_queries();

        PerformanceReport {
            total_queries,
            unique_queries: self.metrics.len() as u64,
            avg_query_duration: avg_duration,
            slow_query_count: slow_queries.len() as u64,
            top_slow_queries: slow_queries
                .into_iter()
                .take(10)
                .cloned()
                .collect(),
            hot_queries: self.get_hot_queries(10).into_iter().cloned().collect(),
        }
    }
}

/// Performance report
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    /// Total query executions
    pub total_queries: u64,
    /// Unique query count
    pub unique_queries: u64,
    /// Average query duration
    pub avg_query_duration: Duration,
    /// Number of slow queries
    pub slow_query_count: u64,
    /// Top slow queries
    pub top_slow_queries: Vec<QueryMetrics>,
    /// Most frequently executed queries
    pub hot_queries: Vec<QueryMetrics>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn create_test_pool() -> SqlitePool {
        SqlitePoolOptions::new().connect(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_analyze_query() {
        let pool = create_test_pool().await;

        // Create test table
        sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        let optimizer = QueryOptimizer::new(pool);

        let plan = optimizer.analyze("SELECT * FROM test WHERE id = 1").await.unwrap();
        assert!(!plan.steps.is_empty());
    }

    #[tokio::test]
    async fn test_suggest_optimizations() {
        let pool = create_test_pool().await;

        sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT, data TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        let optimizer = QueryOptimizer::new(pool);

        // Full scan query
        let suggestions = optimizer
            .suggest_optimizations("SELECT * FROM test WHERE name = 'test'")
            .await
            .unwrap();

        // Should suggest adding index
        assert!(
            suggestions.iter().any(|s| s.message.contains("full table scan")
                || s.recommendation.contains("index"))
        );
    }

    #[tokio::test]
    async fn test_recommend_indexes() {
        let pool = create_test_pool().await;

        // Create test table with foreign key-like column
        sqlx::query(
            "CREATE TABLE servers (id TEXT PRIMARY KEY, group_id TEXT, name TEXT, data TEXT)"
        )
        .execute(&pool)
        .await
        .unwrap();

        let optimizer = QueryOptimizer::new(pool);

        let recommendations = optimizer.recommend_indexes().await.unwrap();
        assert!(!recommendations.is_empty());

        // Should recommend index on group_id
        assert!(recommendations.iter().any(|r| r.columns.contains(&"group_id".to_string())));
    }

    #[tokio::test]
    async fn test_query_monitor() {
        let pool = create_test_pool().await;
        let mut monitor = QueryMonitor::new(pool);

        // Record some queries
        monitor.record_query("SELECT * FROM test", Duration::from_millis(50));
        monitor.record_query("SELECT * FROM test", Duration::from_millis(60));
        monitor.record_query("INSERT INTO test VALUES (1)", Duration::from_millis(200));

        let metrics = monitor.get_metrics();
        assert_eq!(metrics.len(), 2);

        let slow = monitor.get_slow_queries();
        assert!(!slow.is_empty()); // The INSERT should be slow

        let hot = monitor.get_hot_queries(10);
        assert_eq!(hot[0].query, "SELECT * FROM test");
    }

    #[tokio::test]
    async fn test_performance_report() {
        let pool = create_test_pool().await;
        let mut monitor = QueryMonitor::new(pool);

        monitor.record_query("SELECT 1", Duration::from_millis(10));
        monitor.record_query("SELECT 2", Duration::from_millis(20));

        let report = monitor.generate_report();
        assert_eq!(report.total_queries, 2);
        assert_eq!(report.unique_queries, 2);
    }
}
