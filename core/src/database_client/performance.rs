//! Performance analysis and monitoring

use crate::database_client::DatabaseError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub queries_per_second: f64,
    pub active_connections: u32,
    pub total_connections: u32,
    pub cache_hit_ratio: Option<f64>,
    pub slow_queries: u32,
    pub avg_query_time_ms: f64,
    pub total_bytes_received: u64,
    pub total_bytes_sent: u64,
    pub table_statistics: Vec<TableStatistics>,
    pub additional_metrics: HashMap<String, f64>,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            queries_per_second: 0.0,
            active_connections: 0,
            total_connections: 0,
            cache_hit_ratio: None,
            slow_queries: 0,
            avg_query_time_ms: 0.0,
            total_bytes_received: 0,
            total_bytes_sent: 0,
            table_statistics: Vec::new(),
            additional_metrics: HashMap::new(),
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Table statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStatistics {
    pub table_name: String,
    pub row_count: u64,
    pub data_size_bytes: u64,
    pub index_size_bytes: u64,
    pub total_size_bytes: u64,
    pub avg_row_length: u64,
    pub last_analyzed: Option<DateTime<Utc>>,
    pub seq_scans: Option<u64>,
    pub idx_scans: Option<u64>,
    pub n_tup_ins: Option<u64>,
    pub n_tup_upd: Option<u64>,
    pub n_tup_del: Option<u64>,
}

/// Slow query entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowQuery {
    pub query_id: String,
    pub query_text: String,
    pub execution_time_ms: u64,
    pub lock_time_ms: u64,
    pub rows_sent: u64,
    pub rows_examined: u64,
    pub timestamp: DateTime<Utc>,
    pub user: String,
    pub host: String,
    pub database: Option<String>,
}

/// Query execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub query_text: String,
    pub estimated_cost: f64,
    pub estimated_rows: u64,
    pub actual_time_ms: Option<u64>,
    pub actual_rows: Option<u64>,
    pub nodes: Vec<PlanNode>,
    pub warnings: Vec<String>,
}

/// Query plan node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNode {
    pub node_type: String,
    pub operation: String,
    pub estimated_cost: f64,
    pub estimated_rows: u64,
    pub actual_time_ms: Option<u64>,
    pub actual_rows: Option<u64>,
    pub width: u32,
    pub children: Vec<PlanNode>,
    pub properties: HashMap<String, String>,
}

/// Performance analyzer
pub struct PerformanceAnalyzer;

impl PerformanceAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze a query and provide optimization suggestions
    pub fn analyze_query(&self, query: &str, plan: &QueryPlan) -> QueryAnalysis {
        let mut suggestions = Vec::new();
        let mut warnings = Vec::new();
        let mut score: f32 = 100.0;

        // Check for SELECT *
        if query.to_uppercase().contains("SELECT *") {
            suggestions.push(OptimizationSuggestion {
                category: SuggestionCategory::BestPractice,
                severity: Severity::Warning,
                message: "Avoid using SELECT * - specify only needed columns".to_string(),
                impact: "Reduces network traffic and memory usage".to_string(),
                example_fix: Some("SELECT id, name FROM users".to_string()),
            });
            score -= 10.0;
        }

        // Check for missing WHERE clause on large tables
        if query.to_uppercase().starts_with("SELECT")
            && !query.to_uppercase().contains("WHERE")
            && !query.to_uppercase().contains("LIMIT")
        {
            warnings.push("Query without WHERE or LIMIT may return excessive data".to_string());
            score -= 15.0;
        }

        // Check for N+1 pattern
        if query.matches("SELECT").count() > 1 {
            suggestions.push(OptimizationSuggestion {
                category: SuggestionCategory::Pattern,
                severity: Severity::Warning,
                message: "Multiple SELECT statements detected - consider using JOINs".to_string(),
                impact: "Reduces round trips to database".to_string(),
                example_fix: Some("Use a single query with JOINs instead".to_string()),
            });
        }

        // Check plan cost
        if plan.estimated_cost > 10000.0 {
            suggestions.push(OptimizationSuggestion {
                category: SuggestionCategory::Index,
                severity: Severity::Critical,
                message: "Query has high estimated cost - consider adding indexes".to_string(),
                impact: format!("Cost: {:.0}", plan.estimated_cost),
                example_fix: None,
            });
            score -= 30.0;
        }

        // Check for full table scans in plan
        for node in &plan.nodes {
            if node.node_type.contains("Seq Scan") {
                suggestions.push(OptimizationSuggestion {
                    category: SuggestionCategory::Index,
                    severity: Severity::Warning,
                    message: format!("Full table scan on {} detected", node.operation),
                    impact: "Consider adding an index on frequently filtered columns".to_string(),
                    example_fix: None,
                });
                score -= 20.0;
            }
        }

        QueryAnalysis {
            query_text: query.to_string(),
            score: score.max(0.0) as u8,
            suggestions,
            warnings,
            plan: plan.clone(),
        }
    }

    /// Find missing indexes based on slow queries
    pub fn find_missing_indexes(&self, slow_queries: &[SlowQuery]) -> Vec<MissingIndex> {
        let mut candidates: HashMap<String, (String, u64)> = HashMap::new();

        for query in slow_queries {
            // Simple heuristic: look for WHERE clauses on columns
            if let Some(where_clause) = Self::extract_where_clause(&query.query_text) {
                for column in Self::extract_columns_from_where(&where_clause) {
                    let key = format!("{}", column);
                    let entry = candidates
                        .entry(key)
                        .or_insert((query.query_text.clone(), 0));
                    entry.1 += query.execution_time_ms;
                }
            }
        }

        candidates
            .iter()
            .filter(|(_, (_, total_time))| *total_time > 1000)
            .map(|(column, (query, total_time))| MissingIndex {
                table_name: Self::extract_table_name(query).unwrap_or_default(),
                column_name: column.clone(),
                estimated_benefit_ms: *total_time,
                sample_query: query.clone(),
                index_type: "BTREE".to_string(),
            })
            .collect()
    }

    /// Generate index creation SQL
    pub fn generate_index_sql(&self, missing: &MissingIndex) -> String {
        let index_name = format!("idx_{}_{}", missing.table_name, missing.column_name);
        format!(
            "CREATE INDEX {} ON {} ({});",
            index_name, missing.table_name, missing.column_name
        )
    }

    fn extract_where_clause(query: &str) -> Option<String> {
        let upper = query.to_uppercase();
        if let Some(start) = upper.find("WHERE") {
            let after_where = &query[start + 5..];
            // Find end of WHERE clause
            let end = after_where
                .to_uppercase()
                .find("ORDER BY")
                .or_else(|| after_where.to_uppercase().find("GROUP BY"))
                .or_else(|| after_where.to_uppercase().find("LIMIT"))
                .or_else(|| after_where.to_uppercase().find(";"))
                .unwrap_or(after_where.len());
            Some(after_where[..end].trim().to_string())
        } else {
            None
        }
    }

    fn extract_columns_from_where(where_clause: &str) -> Vec<String> {
        // Simple extraction - look for patterns like "column = value"
        let mut columns = Vec::new();
        let operators = vec!["=", "<>", "!=", "<", ">", "<=", ">=", "LIKE", "IN"];

        for op in operators {
            for part in where_clause.split(op) {
                let trimmed = part.trim();
                if !trimmed.is_empty()
                    && !trimmed.starts_with("'")
                    && !trimmed.parse::<f64>().is_ok()
                {
                    // Clean up the column name
                    let col = trimmed
                        .split_whitespace()
                        .last()
                        .unwrap_or(trimmed)
                        .trim_matches(&['(', ')', ',', ' '][..])
                        .to_string();
                    if !col.is_empty()
                        && col
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
                    {
                        columns.push(col);
                    }
                }
            }
        }

        columns
    }

    fn extract_table_name(query: &str) -> Option<String> {
        let upper = query.to_uppercase();
        if let Some(start) = upper.find("FROM") {
            let after_from = &query[start + 4..];
            let end = after_from
                .find(|c: char| c.is_whitespace() || c == ';')
                .unwrap_or(after_from.len());
            Some(after_from[..end].trim().to_string())
        } else {
            None
        }
    }
}

impl Default for PerformanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Query analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAnalysis {
    pub query_text: String,
    pub score: u8,
    pub suggestions: Vec<OptimizationSuggestion>,
    pub warnings: Vec<String>,
    pub plan: QueryPlan,
}

/// Optimization suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub category: SuggestionCategory,
    pub severity: Severity,
    pub message: String,
    pub impact: String,
    pub example_fix: Option<String>,
}

/// Suggestion category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionCategory {
    Index,
    BestPractice,
    Pattern,
    Structure,
    Configuration,
}

/// Severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// Missing index candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingIndex {
    pub table_name: String,
    pub column_name: String,
    pub estimated_benefit_ms: u64,
    pub sample_query: String,
    pub index_type: String,
}

/// Performance monitor
pub struct PerformanceMonitor {
    metrics_history: Vec<TimestampedMetrics>,
    max_history_size: usize,
}

#[derive(Debug, Clone)]
struct TimestampedMetrics {
    timestamp: DateTime<Utc>,
    metrics: PerformanceMetrics,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics_history: Vec::new(),
            max_history_size: 1000,
        }
    }

    pub fn record(&mut self, metrics: PerformanceMetrics) {
        self.metrics_history.push(TimestampedMetrics {
            timestamp: Utc::now(),
            metrics,
        });

        if self.metrics_history.len() > self.max_history_size {
            self.metrics_history.remove(0);
        }
    }

    pub fn get_trends(&self, duration_secs: u64) -> PerformanceTrends {
        let cutoff = Utc::now() - chrono::Duration::seconds(duration_secs as i64);

        let recent: Vec<_> = self
            .metrics_history
            .iter()
            .filter(|m| m.timestamp >= cutoff)
            .collect();

        if recent.is_empty() {
            return PerformanceTrends::default();
        }

        let qps_values: Vec<f64> = recent
            .iter()
            .map(|m| m.metrics.queries_per_second)
            .collect();

        PerformanceTrends {
            avg_queries_per_second: qps_values.iter().sum::<f64>() / qps_values.len() as f64,
            peak_queries_per_second: qps_values.iter().cloned().fold(0.0, f64::max),
            min_queries_per_second: qps_values.iter().cloned().fold(f64::MAX, f64::min),
            connection_growth: self.calculate_connection_growth(&recent),
            slow_query_trend: recent.iter().map(|m| m.metrics.slow_queries as u64).sum(),
            cache_efficiency: recent
                .last()
                .and_then(|m| m.metrics.cache_hit_ratio)
                .unwrap_or(0.0),
        }
    }

    fn calculate_connection_growth(&self, recent: &[&TimestampedMetrics]) -> f64 {
        if recent.len() < 2 {
            return 0.0;
        }

        let first = recent.first().unwrap().metrics.total_connections as f64;
        let last = recent.last().unwrap().metrics.total_connections as f64;

        if first == 0.0 {
            0.0
        } else {
            ((last - first) / first) * 100.0
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance trends
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceTrends {
    pub avg_queries_per_second: f64,
    pub peak_queries_per_second: f64,
    pub min_queries_per_second: f64,
    pub connection_growth: f64,
    pub slow_query_trend: u64,
    pub cache_efficiency: f64,
}

/// Explain formatter for query plans
pub struct ExplainFormatter;

impl ExplainFormatter {
    pub fn format_text(plan: &QueryPlan) -> String {
        let mut output = String::new();
        output.push_str(&format!(
            "Query Plan (Cost: {:.2}, Rows: {})\n",
            plan.estimated_cost, plan.estimated_rows
        ));
        output.push_str("========================================================\n\n");

        for node in &plan.nodes {
            Self::format_node(&mut output, node, 0);
        }

        if !plan.warnings.is_empty() {
            output.push_str("\nWarnings:\n");
            for warning in &plan.warnings {
                output.push_str(&format!("  - {}\n", warning));
            }
        }

        output
    }

    fn format_node(output: &mut String, node: &PlanNode, indent: usize) {
        let indent_str = "  ".repeat(indent);
        output.push_str(&format!(
            "{}{} (cost={:.2} rows={})\n",
            indent_str, node.node_type, node.estimated_cost, node.estimated_rows
        ));

        if let Some(time) = node.actual_time_ms {
            output.push_str(&format!("{}  actual time={}ms\n", indent_str, time));
        }

        for (key, value) in &node.properties {
            output.push_str(&format!("{}  {}: {}\n", indent_str, key, value));
        }

        for child in &node.children {
            Self::format_node(output, child, indent + 1);
        }
    }

    pub fn format_json(plan: &QueryPlan) -> Result<String, DatabaseError> {
        serde_json::to_string_pretty(plan)
            .map_err(|e| DatabaseError::ImportExportError(e.to_string()))
    }
}
