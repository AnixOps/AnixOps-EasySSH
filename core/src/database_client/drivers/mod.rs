use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export DatabaseType from parent module to ensure consistency
pub use crate::database_client::DatabaseType;

pub mod sqlite;

#[cfg(feature = "mysql-driver")]
pub mod mysql;

#[cfg(feature = "postgres-driver")]
pub mod postgres;

#[cfg(feature = "mongodb-driver")]
pub mod mongodb;

#[cfg(feature = "redis-driver")]
pub mod redis;

/// Connection information for databases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: Option<String>,
    pub ssl_mode: SslMode,
    pub connection_timeout: u64,
    pub query_timeout: u64,
}

impl Default for ConnectionInfo {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 3306,
            database: "test".to_string(),
            username: "root".to_string(),
            password: None,
            ssl_mode: SslMode::Preferred,
            connection_timeout: 30,
            query_timeout: 300,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SslMode {
    Disabled,
    Preferred,
    Required,
    VerifyCa,
    VerifyIdentity,
}

/// Database driver error
/// Re-export DatabaseError from parent module
pub use crate::database_client::DatabaseError;

/// Result row from a query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultRow {
    pub columns: HashMap<String, Value>,
}

impl ResultRow {
    pub fn get(&self, column: &str) -> Option<&Value> {
        self.columns.get(column)
    }
}

/// Value types for database values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Bytes(Vec<u8>),
    Date(String),
    Time(String),
    DateTime(String),
    Json(serde_json::Value),
    Array(Vec<Value>),
}

/// Table type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableType {
    Table,
    View,
    SystemTable,
    Temporary,
    Foreign,
    MaterializedView,
    Partitioned,
}

/// Column information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
    pub is_unique: bool,
    pub is_auto_increment: bool,
    pub max_length: Option<u32>,
    pub numeric_precision: Option<u32>,
    pub numeric_scale: Option<u32>,
    pub ordinal_position: u32,
    pub comment: Option<String>,
    pub collation: Option<String>,
    pub default_value: Option<String>,
}

/// Index information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub primary: bool,
    pub index_type: String,
    pub cardinality: Option<u64>,
}

/// Foreign key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyInfo {
    pub name: String,
    pub column: String,
    pub referenced_table: String,
    pub referenced_column: String,
    pub on_update: String,
    pub on_delete: String,
}

/// Constraint information
#[derive(Debug, Clone)]
pub struct ConstraintInfo {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub columns: Vec<String>,
    pub definition: Option<String>,
}

/// Constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    PrimaryKey,
    Unique,
    ForeignKey,
    Check,
    NotNull,
    Default,
    Index,
}

/// Trigger information
#[derive(Debug, Clone)]
pub struct TriggerInfo {
    pub name: String,
    pub event: String,
    pub timing: String,
    pub table: String,
    pub definition: String,
    pub enabled: bool,
}

/// Detailed table information
#[derive(Debug, Clone)]
pub struct TableDetail {
    pub info: TableInfo,
    pub columns: Vec<ColumnInfo>,
    pub indexes: Vec<IndexInfo>,
    pub foreign_keys: Vec<ForeignKeyInfo>,
    pub constraints: Vec<ConstraintInfo>,
    pub triggers: Vec<TriggerInfo>,
    pub privileges: Vec<String>,
}

/// Table information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub schema: Option<String>,
    pub table_type: TableType,
    pub columns: Vec<ColumnInfo>,
    pub indexes: Vec<IndexInfo>,
    pub foreign_keys: Vec<ForeignKeyInfo>,
    pub row_count: Option<u64>,
    pub size_bytes: Option<u64>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub comment: Option<String>,
}

/// Re-export QueryResult from query module to avoid duplication
pub use crate::database_client::query::{QueryCell, QueryResult, QueryRow};

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub name: String,
    pub size_bytes: u64,
    pub table_count: usize,
    pub index_count: usize,
    pub connection_count: Option<u32>,
    pub uptime_seconds: Option<u64>,
    pub version: String,
}

#[async_trait]
pub trait DatabaseDriver: Send + Sync {
    /// Get the database type
    fn db_type(&self) -> DatabaseType;

    /// Connect to the database
    async fn connect(&mut self, info: &ConnectionInfo) -> Result<(), DatabaseError>;

    /// Disconnect from the database
    async fn disconnect(&mut self) -> Result<(), DatabaseError>;

    /// Check if connected
    fn is_connected(&self) -> bool;

    /// Execute a query and return results
    async fn query(&self, sql: &str) -> Result<QueryResult, DatabaseError>;

    /// Execute a query
    async fn execute_query(&self, sql: &str) -> Result<QueryResult, DatabaseError> {
        self.query(sql).await
    }

    /// Execute a statement (INSERT, UPDATE, DELETE) and return rows affected
    async fn execute(&self, sql: &str) -> Result<u64, DatabaseError>;

    /// Get database schema
    async fn get_schema(
        &self,
    ) -> Result<crate::database_client::schema::DatabaseSchema, DatabaseError> {
        Err(DatabaseError::DriverNotFound("Not implemented".to_string()))
    }

    /// List databases
    async fn list_databases(&self) -> Result<Vec<String>, DatabaseError> {
        Err(DatabaseError::DriverNotFound("Not implemented".to_string()))
    }

    /// Get list of tables
    async fn get_tables(&self) -> Result<Vec<TableInfo>, DatabaseError>;

    /// Get table information
    async fn get_table(&self, name: &str) -> Result<TableInfo, DatabaseError>;

    /// Get detailed table information
    async fn get_table_info(&self, name: &str) -> Result<TableDetail, DatabaseError> {
        let table = self.get_table(name).await?;
        Ok(TableDetail {
            info: table.clone(),
            columns: table.columns,
            indexes: table.indexes,
            foreign_keys: table.foreign_keys,
            constraints: Vec::new(),
            triggers: Vec::new(),
            privileges: Vec::new(),
        })
    }

    /// Get database statistics
    async fn get_stats(&self) -> Result<DatabaseStats, DatabaseError>;

    /// Get performance metrics
    async fn get_performance_metrics(
        &self,
    ) -> Result<crate::database_client::performance::PerformanceMetrics, DatabaseError> {
        Err(DatabaseError::DriverNotFound("Not implemented".to_string()))
    }

    /// Begin a transaction
    async fn begin_transaction(&mut self) -> Result<(), DatabaseError>;

    /// Commit a transaction
    async fn commit(&mut self) -> Result<(), DatabaseError>;

    /// Rollback a transaction
    async fn rollback(&mut self) -> Result<(), DatabaseError>;

    /// Cancel the current query
    async fn cancel(&self) -> Result<(), DatabaseError>;

    /// Ping the database
    async fn ping(&self) -> Result<(), DatabaseError> {
        self.query("SELECT 1").await.map(|_| ())
    }
}

/// Driver manager for managing multiple database drivers
pub struct DriverManager {
    drivers: HashMap<String, Box<dyn DatabaseDriver>>,
}

impl DriverManager {
    pub fn new() -> Self {
        Self {
            drivers: HashMap::new(),
        }
    }

    pub fn add_driver(&mut self, name: String, driver: Box<dyn DatabaseDriver>) {
        self.drivers.insert(name, driver);
    }

    pub fn get_driver(&self, name: &str) -> Option<&dyn DatabaseDriver> {
        self.drivers.get(name).map(|d| d.as_ref())
    }

    pub fn get_driver_mut(&mut self, name: &str) -> Option<&mut Box<dyn DatabaseDriver>> {
        self.drivers.get_mut(name)
    }

    pub fn remove_driver(&mut self, name: &str) -> Option<Box<dyn DatabaseDriver>> {
        self.drivers.remove(name)
    }

    pub fn list_drivers(&self) -> Vec<&str> {
        self.drivers.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for DriverManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
    pub test_on_checkout: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            acquire_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 1800,
            test_on_checkout: true,
        }
    }
}

/// SQL dialect specific functions
pub struct SqlDialect;

impl SqlDialect {
    /// Get the limit clause for a specific database type
    pub fn limit_clause(db_type: DatabaseType, limit: u32, offset: Option<u32>) -> String {
        match db_type {
            DatabaseType::MySQL | DatabaseType::PostgreSQL | DatabaseType::SQLite => {
                if let Some(off) = offset {
                    format!("LIMIT {} OFFSET {}", limit, off)
                } else {
                    format!("LIMIT {}", limit)
                }
            }
            _ => format!("LIMIT {}", limit),
        }
    }

    /// Get the current timestamp function for a specific database type
    pub fn current_timestamp(db_type: DatabaseType) -> &'static str {
        match db_type {
            DatabaseType::MySQL => "NOW()",
            DatabaseType::PostgreSQL => "CURRENT_TIMESTAMP",
            DatabaseType::SQLite => "datetime('now')",
            _ => "CURRENT_TIMESTAMP",
        }
    }

    /// Check if the database supports RETURNING clause
    pub fn supports_returning(db_type: DatabaseType) -> bool {
        matches!(db_type, DatabaseType::PostgreSQL | DatabaseType::SQLite)
    }

    /// Get placeholder style for parameterized queries
    pub fn placeholder_style(db_type: DatabaseType) -> PlaceholderStyle {
        match db_type {
            DatabaseType::PostgreSQL => PlaceholderStyle::Numbered,
            _ => PlaceholderStyle::Positional,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PlaceholderStyle {
    Positional, // ?, ?, ?
    Numbered,   // $1, $2, $3
    Named,      // :name, :age
}

/// Query builder for constructing SQL queries
pub struct QueryBuilder {
    db_type: DatabaseType,
    sql: String,
    params: Vec<Value>,
}

impl QueryBuilder {
    pub fn new(db_type: DatabaseType) -> Self {
        Self {
            db_type,
            sql: String::new(),
            params: Vec::new(),
        }
    }

    /// Validate SQL identifier to prevent injection
    /// Only allows alphanumeric characters and underscores
    fn validate_identifier(identifier: &str) -> Result<String, DatabaseError> {
        if identifier.is_empty() {
            return Err(DatabaseError::QueryError(
                "Empty identifier not allowed".to_string(),
            ));
        }

        // Check each character
        for c in identifier.chars() {
            if !c.is_alphanumeric() && c != '_' && c != '.' {
                return Err(DatabaseError::QueryError(format!(
                    "Invalid character '{}' in identifier '{}'",
                    c, identifier
                )));
            }
        }

        Ok(identifier.to_string())
    }

    pub fn select(mut self, columns: &[&str]) -> Self {
        // Validate each column name
        let valid_columns: Vec<String> = columns
            .iter()
            .filter_map(|&col| Self::validate_identifier(col).ok())
            .collect();

        if valid_columns.is_empty() {
            self.sql = "SELECT *".to_string();
        } else {
            self.sql = format!("SELECT {}", valid_columns.join(", "));
        }
        self
    }

    pub fn from(mut self, table: &str) -> Self {
        // Validate table name
        match Self::validate_identifier(table) {
            Ok(valid_table) => {
                self.sql.push_str(&format!(" FROM {}", valid_table));
            }
            Err(_) => {
                // Invalid table name - use placeholder that will cause error
                self.sql.push_str(" FROM \"INVALID_TABLE_NAME\"");
            }
        }
        self
    }

    pub fn where_clause(mut self, condition: &str) -> Self {
        // Validate WHERE condition to prevent SQL injection
        // WHERE conditions are complex expressions, but we can still validate for dangerous patterns
        let dangerous_patterns = ["--", "/*", "*/", ";", "'", "\"", "\n", "\r"];
        let safe_condition = if dangerous_patterns.iter().any(|p| condition.contains(p)) {
            // If dangerous pattern found, use a safe placeholder that won't match anything
            "1=0 /* Invalid WHERE condition */".to_string()
        } else {
            condition.to_string()
        };
        self.sql.push_str(&format!(" WHERE {}", safe_condition));
        self
    }

    pub fn order_by(mut self, column: &str, asc: bool) -> Self {
        let direction = if asc { "ASC" } else { "DESC" };
        // Validate column name
        match Self::validate_identifier(column) {
            Ok(valid_col) => {
                self.sql
                    .push_str(&format!(" ORDER BY {} {}", valid_col, direction));
            }
            Err(_) => {
                self.sql.push_str(" ORDER BY \"INVALID_COLUMN\" ASC");
            }
        }
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.sql.push_str(&format!(" LIMIT {}", limit));
        self
    }

    pub fn build(self) -> (String, Vec<Value>) {
        (self.sql, self.params)
    }
}

/// Performance monitoring for database operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryMetrics {
    pub query: String,
    pub execution_time_ms: u64,
    pub rows_returned: u64,
    pub rows_affected: u64,
    pub timestamp: String,
}

/// Slow query analyzer
pub struct SlowQueryAnalyzer {
    threshold_ms: u64,
    queries: Vec<QueryMetrics>,
}

impl SlowQueryAnalyzer {
    pub fn new(threshold_ms: u64) -> Self {
        Self {
            threshold_ms,
            queries: Vec::new(),
        }
    }

    pub fn record_query(&mut self, metrics: QueryMetrics) {
        if metrics.execution_time_ms >= self.threshold_ms {
            self.queries.push(metrics);
        }
    }

    pub fn get_slow_queries(&self) -> &[QueryMetrics] {
        &self.queries
    }

    pub fn clear(&mut self) {
        self.queries.clear();
    }

    pub fn analyze_patterns(&self) -> Vec<String> {
        // Simple pattern analysis - look for common query patterns
        let mut patterns: HashMap<String, (usize, u64)> = HashMap::new();

        for query in &self.queries {
            // Extract table name as pattern
            if let Some(table) = Self::extract_table_name(&query.query) {
                let entry = patterns.entry(table).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += query.execution_time_ms;
            }
        }

        // Sort by frequency and return suggestions
        let mut suggestions: Vec<(String, usize, u64)> = patterns
            .iter()
            .map(|(k, v)| (k.clone(), v.0, v.1))
            .collect();

        suggestions.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by frequency

        suggestions
            .into_iter()
            .map(|(table, count, total_time)| {
                format!(
                    "Table '{}' has {} slow queries with total {}ms - consider adding indexes",
                    table, count, total_time
                )
            })
            .collect()
    }

    fn extract_table_name(query: &str) -> Option<String> {
        let query_lower = query.to_uppercase();

        // Try to extract table name from different query types
        let patterns = [("FROM", 4), ("INTO", 4), ("UPDATE", 6), ("JOIN", 4)];

        for (keyword, len) in &patterns {
            if let Some(pos) = query_lower.find(keyword) {
                let start = pos + len;
                let remaining = &query[start..];
                if let Some(word) = remaining.split_whitespace().next() {
                    let cleaned = word.trim_matches(&['(', ')', ',', ' '][..]);
                    if !cleaned.is_empty() {
                        return Some(cleaned.to_string());
                    }
                }
            }
        }

        None
    }
}

/// Query plan analyzer
pub struct QueryPlanAnalyzer;

impl QueryPlanAnalyzer {
    pub fn analyze(plan: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        let plan_lower = plan.to_lowercase();

        // Check for common performance issues
        if plan_lower.contains("seq scan") && !plan_lower.contains("index") {
            suggestions.push("Sequential scan detected - consider adding an index".to_string());
        }

        if plan_lower.contains("nested loop") {
            suggestions.push(
                "Nested loop join detected - consider query optimization for large datasets"
                    .to_string(),
            );
        }

        if plan_lower.contains("temporary") || plan_lower.contains("temp") {
            suggestions.push("Temporary tables/disk usage detected - consider increasing work_mem or optimizing query".to_string());
        }

        if plan_lower.contains("sort") && plan_lower.contains("disk") {
            suggestions.push("Disk sort detected - consider increasing work_mem".to_string());
        }

        suggestions
    }
}

/// Schema analyzer
pub struct SchemaAnalyzer;

impl SchemaAnalyzer {
    pub fn analyze_table(table: &TableInfo) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Check for missing primary key
        let has_pk = table.columns.iter().any(|c| c.is_primary_key);
        if !has_pk {
            suggestions.push(format!(
                "Table '{}' has no primary key - consider adding one",
                table.name
            ));
        }

        // Check for tables without indexes
        if table.indexes.is_empty() {
            suggestions.push(format!("Table '{}' has no indexes - consider adding indexes for frequently queried columns", table.name));
        }

        // Check for missing foreign key indexes
        for fk in &table.foreign_keys {
            let has_fk_index = table
                .indexes
                .iter()
                .any(|idx| idx.columns.contains(&fk.column));
            if !has_fk_index {
                suggestions.push(format!(
                    "Foreign key column '{}' on table '{}' has no index - consider adding one",
                    fk.column, table.name
                ));
            }
        }

        // Check for large TEXT/VARCHAR columns without indexes
        for col in &table.columns {
            if col.data_type.to_uppercase().contains("TEXT")
                && col.data_type.to_uppercase().contains("VARCHAR")
            {
                let has_index = table
                    .indexes
                    .iter()
                    .any(|idx| idx.columns.contains(&col.name));
                if has_index {
                    suggestions.push(format!(
                        "Large text column '{}' on table '{}' has an index - consider using prefix indexes",
                        col.name, table.name
                    ));
                }
            }
        }

        suggestions
    }
}

/// Helper function to escape SQL identifiers
pub fn escape_identifier(ident: &str, db_type: DatabaseType) -> String {
    match db_type {
        DatabaseType::MySQL => format!("`{}`", ident.replace('`', "``")),
        DatabaseType::PostgreSQL => format!("\"{}\"", ident.replace('"', "\"\"")),
        DatabaseType::SQLite => format!("\"{}\"", ident.replace('"', "\"\"")),
        _ => ident.to_string(),
    }
}

// quote_string function removed due to escaping issues

/// Query cache for frequently executed queries
pub struct QueryCache {
    cache: HashMap<String, QueryResult>,
    max_size: usize,
}

impl QueryCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(max_size),
            max_size,
        }
    }

    pub fn get(&self, query: &str) -> Option<&QueryResult> {
        self.cache.get(query)
    }

    pub fn put(&mut self, query: String, result: QueryResult) {
        if self.cache.len() >= self.max_size {
            // Remove oldest entry (simple LRU)
            if let Some(oldest) = self.cache.keys().next().cloned() {
                self.cache.remove(&oldest);
            }
        }
        self.cache.insert(query, result);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn invalidate(&mut self, query: &str) {
        self.cache.remove(query);
    }
}

/// Query batching for efficient bulk operations
pub struct QueryBatcher {
    db_type: DatabaseType,
    queries: Vec<String>,
    max_batch_size: usize,
}

impl QueryBatcher {
    pub fn new(db_type: DatabaseType, max_batch_size: usize) -> Self {
        Self {
            db_type,
            queries: Vec::with_capacity(max_batch_size),
            max_batch_size,
        }
    }

    pub fn add(&mut self, query: String) -> Option<Vec<String>> {
        self.queries.push(query);
        if self.queries.len() >= self.max_batch_size {
            Some(std::mem::take(&mut self.queries))
        } else {
            None
        }
    }

    pub fn flush(&mut self) -> Vec<String> {
        std::mem::take(&mut self.queries)
    }

    pub fn combine(&self, queries: &[String]) -> String {
        match self.db_type {
            DatabaseType::MySQL | DatabaseType::SQLite => queries.join("; "),
            DatabaseType::PostgreSQL => queries.join("; "),
            _ => queries.join("; "),
        }
    }
}

/// Export configuration for data export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub include_headers: bool,
    pub delimiter: String,
    pub quote_char: char,
    pub escape_char: char,
    pub null_value: String,
    pub date_format: String,
    pub datetime_format: String,
    pub encoding: String,
    pub bom: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::Csv,
            include_headers: true,
            delimiter: ",".to_string(),
            quote_char: '"',
            escape_char: '"',
            null_value: "".to_string(),
            date_format: "%Y-%m-%d".to_string(),
            datetime_format: "%Y-%m-%d %H:%M:%S".to_string(),
            encoding: "UTF-8".to_string(),
            bom: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Csv,
    Tsv,
    Json,
    Xml,
    Sql,
    Excel,
    Markdown,
    Html,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportFormat::Csv => write!(f, "CSV"),
            ExportFormat::Tsv => write!(f, "TSV"),
            ExportFormat::Json => write!(f, "JSON"),
            ExportFormat::Xml => write!(f, "XML"),
            ExportFormat::Sql => write!(f, "SQL"),
            ExportFormat::Excel => write!(f, "Excel"),
            ExportFormat::Markdown => write!(f, "Markdown"),
            ExportFormat::Html => write!(f, "HTML"),
        }
    }
}
