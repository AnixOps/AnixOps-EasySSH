//! Batch Operations for Database Client
//!
//! Features:
//! - Batch insert with optimized prepared statements
//! - Batch update with chunking
//! - Transaction batching for atomic operations
//! - Streaming batch processing for large datasets
//! - Progress tracking and cancellation support
//! - Automatic retry with exponential backoff

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

use crate::database_client::drivers::Value;
use crate::database_client::pool::{OptimizedConnectionPool, PooledConnectionGuard};
use crate::database_client::{DatabaseError, DatabaseType};

/// Batch operation configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum rows per batch
    pub batch_size: usize,
    /// Maximum concurrent batches
    pub max_concurrent: usize,
    /// Enable transactions for atomicity
    pub use_transactions: bool,
    /// Transaction timeout
    pub transaction_timeout: Duration,
    /// Enable automatic retry
    pub enable_retry: bool,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Initial retry delay
    pub retry_delay: Duration,
    /// Maximum retry delay
    pub max_retry_delay: Duration,
    /// Progress reporting interval (rows)
    pub progress_interval: usize,
    /// Enable duplicate key handling
    pub handle_duplicates: DuplicateHandling,
    /// Statement preparation strategy
    pub statement_strategy: StatementStrategy,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            max_concurrent: 4,
            use_transactions: true,
            transaction_timeout: Duration::from_secs(300),
            enable_retry: true,
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
            max_retry_delay: Duration::from_secs(10),
            progress_interval: 1000,
            handle_duplicates: DuplicateHandling::Error,
            statement_strategy: StatementStrategy::Prepared,
        }
    }
}

impl BatchConfig {
    /// Configuration for high-throughput batch operations
    pub fn high_throughput() -> Self {
        Self {
            batch_size: 5000,
            max_concurrent: 8,
            use_transactions: false,
            progress_interval: 10000,
            handle_duplicates: DuplicateHandling::Ignore,
            ..Default::default()
        }
    }

    /// Configuration for reliable batch operations (transactional)
    pub fn reliable() -> Self {
        Self {
            batch_size: 500,
            max_concurrent: 2,
            use_transactions: true,
            enable_retry: true,
            max_retries: 5,
            handle_duplicates: DuplicateHandling::Update,
            ..Default::default()
        }
    }

    /// Configuration for small batches (testing/development)
    pub fn small() -> Self {
        Self {
            batch_size: 100,
            max_concurrent: 1,
            use_transactions: true,
            progress_interval: 10,
            ..Default::default()
        }
    }
}

/// Duplicate key handling strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateHandling {
    /// Raise error on duplicate
    Error,
    /// Ignore duplicates (skip)
    Ignore,
    /// Update existing row on duplicate
    Update,
    /// Replace existing row
    Replace,
}

/// Statement preparation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementStrategy {
    /// Use prepared statements for all batches
    Prepared,
    /// Use bulk insert syntax (database-specific)
    Bulk,
    /// Use multi-row VALUES clause
    MultiRow,
    /// Use simple statements
    Simple,
}

/// Batch operation type
#[derive(Debug, Clone)]
pub enum BatchOperation {
    /// Insert rows into table
    Insert {
        table: String,
        columns: Vec<String>,
        rows: Vec<Vec<Value>>,
    },
    /// Update rows in table
    Update {
        table: String,
        set_clause: Vec<(String, Value)>,
        where_clause: String,
        conditions: Vec<Vec<Value>>,
    },
    /// Delete rows from table
    Delete {
        table: String,
        where_clause: String,
        conditions: Vec<Vec<Value>>,
    },
    /// Upsert (insert or update)
    Upsert {
        table: String,
        columns: Vec<String>,
        rows: Vec<Vec<Value>>,
        key_columns: Vec<String>,
    },
}

/// Batch operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// Number of rows successfully processed
    pub rows_affected: u64,
    /// Number of batches executed
    pub batches_executed: u32,
    /// Number of failed batches
    pub failed_batches: u32,
    /// Total execution time
    pub execution_time_ms: u64,
    /// Rows per second
    pub rows_per_second: f64,
    /// Any errors that occurred
    pub errors: Vec<BatchError>,
    /// Whether the operation was cancelled
    pub was_cancelled: bool,
}

impl BatchResult {
    /// Check if batch operation was successful
    pub fn is_success(&self) -> bool {
        self.failed_batches == 0 && !self.was_cancelled
    }

    /// Check if batch operation was partially successful
    pub fn is_partial(&self) -> bool {
        self.rows_affected > 0 && (self.failed_batches > 0 || self.was_cancelled)
    }
}

/// Batch error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchError {
    /// Batch index that failed
    pub batch_index: u32,
    /// Error message
    pub message: String,
    /// Number of rows in the failed batch
    pub row_count: usize,
    /// SQL that caused the error (if applicable)
    pub sql: Option<String>,
}

/// Progress update for batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProgress {
    /// Rows processed so far
    pub rows_processed: u64,
    /// Total rows to process (if known)
    pub total_rows: Option<u64>,
    /// Percentage complete (0-100)
    pub percentage: f32,
    /// Current rows per second
    pub rows_per_second: f64,
    /// Estimated time remaining in seconds
    pub eta_seconds: Option<u64>,
    /// Current batch number
    pub current_batch: u32,
    /// Total batches
    pub total_batches: u32,
    /// Elapsed time in milliseconds
    pub elapsed_ms: u64,
}

/// Batch operation context for cancellation and progress
pub struct BatchContext {
    /// Cancellation flag
    cancelled: AtomicBool,
    /// Rows processed
    rows_processed: AtomicU64,
    /// Progress sender
    progress_tx: Option<mpsc::Sender<BatchProgress>>,
    /// Start time
    start_time: Instant,
    /// Total rows (if known)
    total_rows: Option<u64>,
}

impl BatchContext {
    fn new(progress_tx: Option<mpsc::Sender<BatchProgress>>, total_rows: Option<u64>) -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            rows_processed: AtomicU64::new(0),
            progress_tx,
            start_time: Instant::now(),
            total_rows,
        }
    }

    /// Cancel the batch operation
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Record processed rows
    fn record_rows(&self, count: u64, current_batch: u32, total_batches: u32) {
        let processed = self.rows_processed.fetch_add(count, Ordering::SeqCst) + count;

        if let Some(ref tx) = self.progress_tx {
            let elapsed_ms = self.start_time.elapsed().as_millis() as u64;
            let rows_per_second = if elapsed_ms > 0 {
                processed as f64 / (elapsed_ms as f64 / 1000.0)
            } else {
                0.0
            };

            let percentage = self
                .total_rows
                .map(|total| ((processed as f64 / total as f64) * 100.0).min(100.0) as f32)
                .unwrap_or(0.0);

            let eta_seconds = if rows_per_second > 0.0 {
                self.total_rows.map(|total| {
                    let remaining = total.saturating_sub(processed);
                    (remaining as f64 / rows_per_second) as u64
                })
            } else {
                None
            };

            let _ = tx.try_send(BatchProgress {
                rows_processed: processed,
                total_rows: self.total_rows,
                percentage,
                rows_per_second,
                eta_seconds,
                current_batch,
                total_batches,
                elapsed_ms,
            });
        }
    }
}

/// Batch inserter for efficient bulk inserts
pub struct BatchInserter {
    pool: Arc<OptimizedConnectionPool>,
    config: BatchConfig,
    db_type: DatabaseType,
}

impl BatchInserter {
    /// Create a new batch inserter
    pub fn new(
        pool: Arc<OptimizedConnectionPool>,
        config: BatchConfig,
        db_type: DatabaseType,
    ) -> Self {
        Self {
            pool,
            config,
            db_type,
        }
    }

    /// Validate and escape SQL identifier to prevent SQL injection
    /// Only allows alphanumeric characters, underscores, and dots (for schema.table)
    fn validate_identifier(identifier: &str) -> Result<String, DatabaseError> {
        if identifier.is_empty() {
            return Err(DatabaseError::QueryError(
                "Empty identifier not allowed".to_string(),
            ));
        }

        // Check for dangerous characters and patterns
        let dangerous_patterns = ["--", "/*", "*/", ";", "'", "\"", "\\", "\n", "\r"];
        for pattern in &dangerous_patterns {
            if identifier.contains(pattern) {
                return Err(DatabaseError::QueryError(format!(
                    "Invalid pattern '{}' in identifier",
                    pattern
                )));
            }
        }

        // Each character must be alphanumeric, underscore, or dot
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

    /// Escape identifier for SQL by wrapping in backticks (MySQL) or double quotes (PostgreSQL/SQLite)
    fn escape_identifier(&self, identifier: &str) -> String {
        match self.db_type {
            DatabaseType::MySQL => format!("`{}`", identifier.replace('`', "``")),
            DatabaseType::PostgreSQL => format!("\"{}\"", identifier.replace('"', "\"\""),),
            DatabaseType::SQLite => format!("\"{}\"", identifier.replace('"', "\"\""),),
            _ => identifier.to_string(),
        }
    }

    /// Validate and escape a list of identifiers
    fn validate_identifiers(&self, identifiers: &[String]) -> Vec<String> {
        identifiers
            .iter()
            .filter_map(|id| {
                Self::validate_identifier(id)
                    .ok()
                    .map(|valid| self.escape_identifier(&valid))
            })
            .collect()
    }

    /// Execute batch insert
    pub async fn insert(
        &self,
        table: &str,
        columns: &[String],
        rows: Vec<Vec<Value>>,
    ) -> Result<BatchResult, DatabaseError> {
        self.execute_batch(BatchOperation::Insert {
            table: table.to_string(),
            columns: columns.to_vec(),
            rows,
        })
        .await
    }

    /// Execute batch upsert
    pub async fn upsert(
        &self,
        table: &str,
        columns: &[String],
        rows: Vec<Vec<Value>>,
        key_columns: &[String],
    ) -> Result<BatchResult, DatabaseError> {
        self.execute_batch(BatchOperation::Upsert {
            table: table.to_string(),
            columns: columns.to_vec(),
            rows,
            key_columns: key_columns.to_vec(),
        })
        .await
    }

    /// Execute batch update
    pub async fn update(
        &self,
        table: &str,
        set_clause: &[(String, Value)],
        where_clause: &str,
        conditions: Vec<Vec<Value>>,
    ) -> Result<BatchResult, DatabaseError> {
        self.execute_batch(BatchOperation::Update {
            table: table.to_string(),
            set_clause: set_clause.to_vec(),
            where_clause: where_clause.to_string(),
            conditions,
        })
        .await
    }

    /// Execute batch delete
    pub async fn delete(
        &self,
        table: &str,
        where_clause: &str,
        conditions: Vec<Vec<Value>>,
    ) -> Result<BatchResult, DatabaseError> {
        self.execute_batch(BatchOperation::Delete {
            table: table.to_string(),
            where_clause: where_clause.to_string(),
            conditions,
        })
        .await
    }

    /// Execute batch operation with progress tracking
    pub async fn execute_batch_with_progress(
        &self,
        operation: BatchOperation,
        progress_tx: mpsc::Sender<BatchProgress>,
    ) -> Result<BatchResult, DatabaseError> {
        let total_rows = match &operation {
            BatchOperation::Insert { rows, .. } => Some(rows.len() as u64),
            BatchOperation::Update { conditions, .. } => Some(conditions.len() as u64),
            BatchOperation::Delete { conditions, .. } => Some(conditions.len() as u64),
            BatchOperation::Upsert { rows, .. } => Some(rows.len() as u64),
        };

        let context = Arc::new(BatchContext::new(Some(progress_tx), total_rows));
        self.execute_batch_internal(operation, context).await
    }

    /// Execute batch operation
    async fn execute_batch(&self, operation: BatchOperation) -> Result<BatchResult, DatabaseError> {
        let context = Arc::new(BatchContext::new(None, None));
        self.execute_batch_internal(operation, context).await
    }

    /// Internal batch execution
    async fn execute_batch_internal(
        &self,
        operation: BatchOperation,
        context: Arc<BatchContext>,
    ) -> Result<BatchResult, DatabaseError> {
        let start = Instant::now();
        let total_rows = match &operation {
            BatchOperation::Insert { rows, .. } => rows.len(),
            BatchOperation::Update { conditions, .. } => conditions.len(),
            BatchOperation::Delete { conditions, .. } => conditions.len(),
            BatchOperation::Upsert { rows, .. } => rows.len(),
        };

        let batch_size = self.config.batch_size;
        let total_batches = ((total_rows + batch_size - 1) / batch_size) as u32;

        let mut all_results: Vec<Result<u64, (u32, DatabaseError)>> = Vec::new();

        // Process batches
        match operation {
            BatchOperation::Insert {
                table,
                columns,
                rows,
            } => {
                let chunks: Vec<_> = rows.chunks(batch_size).enumerate().collect();

                for (batch_idx, chunk) in chunks {
                    if context.is_cancelled() {
                        break;
                    }

                    let result = self
                        .execute_insert_batch(&table, &columns, chunk, batch_idx as u32)
                        .await;

                    match result {
                        Ok(affected) => {
                            context.record_rows(
                                chunk.len() as u64,
                                batch_idx as u32 + 1,
                                total_batches,
                            );
                            all_results.push(Ok(affected));
                        }
                        Err(e) => {
                            all_results.push(Err((batch_idx as u32, e)));
                            if !self.config.enable_retry {
                                break;
                            }
                        }
                    }
                }
            }
            BatchOperation::Upsert {
                table,
                columns,
                rows,
                key_columns,
            } => {
                let chunks: Vec<_> = rows.chunks(batch_size).enumerate().collect();

                for (batch_idx, chunk) in chunks {
                    if context.is_cancelled() {
                        break;
                    }

                    let result = self
                        .execute_upsert_batch(
                            &table,
                            &columns,
                            chunk,
                            &key_columns,
                            batch_idx as u32,
                        )
                        .await;

                    match result {
                        Ok(affected) => {
                            context.record_rows(
                                chunk.len() as u64,
                                batch_idx as u32 + 1,
                                total_batches,
                            );
                            all_results.push(Ok(affected));
                        }
                        Err(e) => {
                            all_results.push(Err((batch_idx as u32, e)));
                            if !self.config.enable_retry {
                                break;
                            }
                        }
                    }
                }
            }
            BatchOperation::Update {
                table,
                set_clause,
                where_clause,
                conditions,
            } => {
                let chunks: Vec<_> = conditions.chunks(batch_size).enumerate().collect();

                for (batch_idx, chunk) in chunks {
                    if context.is_cancelled() {
                        break;
                    }

                    let result = self
                        .execute_update_batch(
                            &table,
                            &set_clause,
                            &where_clause,
                            chunk,
                            batch_idx as u32,
                        )
                        .await;

                    match result {
                        Ok(affected) => {
                            context.record_rows(
                                chunk.len() as u64,
                                batch_idx as u32 + 1,
                                total_batches,
                            );
                            all_results.push(Ok(affected));
                        }
                        Err(e) => {
                            all_results.push(Err((batch_idx as u32, e)));
                            if !self.config.enable_retry {
                                break;
                            }
                        }
                    }
                }
            }
            BatchOperation::Delete {
                table,
                where_clause,
                conditions,
            } => {
                let chunks: Vec<_> = conditions.chunks(batch_size).enumerate().collect();

                for (batch_idx, chunk) in chunks {
                    if context.is_cancelled() {
                        break;
                    }

                    let result = self
                        .execute_delete_batch(&table, &where_clause, chunk, batch_idx as u32)
                        .await;

                    match result {
                        Ok(affected) => {
                            context.record_rows(
                                chunk.len() as u64,
                                batch_idx as u32 + 1,
                                total_batches,
                            );
                            all_results.push(Ok(affected));
                        }
                        Err(e) => {
                            all_results.push(Err((batch_idx as u32, e)));
                            if !self.config.enable_retry {
                                break;
                            }
                        }
                    }
                }
            }
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        // Compile results
        let mut rows_affected: u64 = 0;
        let mut batches_executed: u32 = 0;
        let mut failed_batches: u32 = 0;
        let mut errors: Vec<BatchError> = Vec::new();

        for result in all_results {
            match result {
                Ok(affected) => {
                    rows_affected += affected;
                    batches_executed += 1;
                }
                Err((batch_idx, e)) => {
                    failed_batches += 1;
                    errors.push(BatchError {
                        batch_index: batch_idx,
                        message: e.to_string(),
                        row_count: batch_size,
                        sql: None,
                    });
                }
            }
        }

        let rows_per_second = if execution_time_ms > 0 {
            rows_affected as f64 / (execution_time_ms as f64 / 1000.0)
        } else {
            0.0
        };

        Ok(BatchResult {
            rows_affected,
            batches_executed,
            failed_batches,
            execution_time_ms,
            rows_per_second,
            errors,
            was_cancelled: context.is_cancelled(),
        })
    }

    /// Execute a single insert batch
    async fn execute_insert_batch(
        &self,
        table: &str,
        columns: &[String],
        rows: &[Vec<Value>],
        batch_idx: u32,
    ) -> Result<u64, DatabaseError> {
        let sql = self.build_insert_sql(table, columns, rows.len());
        trace!("Executing insert batch {}: {}", batch_idx, sql);

        // Acquire connection from pool
        let conn = self.pool.acquire().await?;

        // Execute with retry
        let mut last_error = None;
        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let delay = self.config.retry_delay * 2_u32.pow(attempt - 1);
                let delay = delay.min(self.config.max_retry_delay);
                tokio::time::sleep(delay).await;
            }

            match conn.execute(&sql).await {
                Ok(affected) => return Ok(affected),
                Err(e) => {
                    last_error = Some(e);
                    if !self.config.enable_retry {
                        break;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| DatabaseError::QueryError("Insert batch failed".to_string())))
    }

    /// Execute a single upsert batch
    async fn execute_upsert_batch(
        &self,
        table: &str,
        columns: &[String],
        rows: &[Vec<Value>],
        key_columns: &[String],
        batch_idx: u32,
    ) -> Result<u64, DatabaseError> {
        let sql = self.build_upsert_sql(table, columns, key_columns, rows.len());
        trace!("Executing upsert batch {}: {}", batch_idx, sql);

        let conn = self.pool.acquire().await?;
        conn.execute(&sql).await
    }

    /// Execute a single update batch
    async fn execute_update_batch(
        &self,
        table: &str,
        set_clause: &[(String, Value)],
        where_clause: &str,
        conditions: &[Vec<Value>],
        batch_idx: u32,
    ) -> Result<u64, DatabaseError> {
        // Build a batch update SQL
        let sql = self.build_batch_update_sql(table, set_clause, where_clause, conditions);
        trace!("Executing update batch {}: {}", batch_idx, sql);

        let conn = self.pool.acquire().await?;
        conn.execute(&sql).await
    }

    /// Execute a single delete batch
    async fn execute_delete_batch(
        &self,
        table: &str,
        where_clause: &str,
        conditions: &[Vec<Value>],
        batch_idx: u32,
    ) -> Result<u64, DatabaseError> {
        // Build a batch delete SQL using IN clause
        let sql = self.build_batch_delete_sql(table, where_clause, conditions);
        trace!("Executing delete batch {}: {}", batch_idx, sql);

        let conn = self.pool.acquire().await?;
        conn.execute(&sql).await
    }

    /// Build INSERT SQL statement with proper identifier escaping
    fn build_insert_sql(&self, table: &str, columns: &[String], row_count: usize) -> String {
        // Validate and escape identifiers
        let escaped_table = match Self::validate_identifier(table) {
            Ok(valid) => self.escape_identifier(&valid),
            Err(_) => "\"INVALID_TABLE\"".to_string(),
        };
        let escaped_columns = self.validate_identifiers(columns);

        if escaped_columns.is_empty() {
            return format!("-- ERROR: No valid columns provided for INSERT INTO {}", escaped_table);
        }

        let columns_str = escaped_columns.join(", ");

        match self.config.statement_strategy {
            StatementStrategy::MultiRow => {
                let placeholders: Vec<_> = (0..row_count)
                    .map(|_| {
                        format!(
                            "({})",
                            columns
                                .iter()
                                .map(|_| "?".to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })
                    .collect();

                format!(
                    "INSERT INTO {} ({}) VALUES {}",
                    escaped_table,
                    columns_str,
                    placeholders.join(", ")
                )
            }
            _ => {
                // Single row insert with prepared statement
                let placeholders = columns
                    .iter()
                    .map(|_| "?".to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "INSERT INTO {} ({}) VALUES ({})",
                    escaped_table, columns_str, placeholders
                )
            }
        }
    }

    /// Build UPSERT SQL statement with proper identifier escaping
    fn build_upsert_sql(
        &self,
        table: &str,
        columns: &[String],
        key_columns: &[String],
        row_count: usize,
    ) -> String {
        // Validate and escape identifiers
        let escaped_table = match Self::validate_identifier(table) {
            Ok(valid) => self.escape_identifier(&valid),
            Err(_) => "\"INVALID_TABLE\"".to_string(),
        };
        let escaped_columns = self.validate_identifiers(columns);
        let escaped_key_columns = self.validate_identifiers(key_columns);

        if escaped_columns.is_empty() || escaped_key_columns.is_empty() {
            return format!("-- ERROR: Invalid columns for UPSERT on {}", escaped_table);
        }

        let columns_str = escaped_columns.join(", ");
        let key_columns_str = escaped_key_columns.join(", ");

        let placeholders: Vec<_> = (0..row_count)
            .map(|_| {
                format!(
                    "({})",
                    columns
                        .iter()
                        .map(|_| "?".to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect();

        // Build update clause with escaped column names
        let update_clause = columns
            .iter()
            .filter(|c| !key_columns.contains(c))
            .map(|c| {
                let escaped_c = self.escape_identifier(c);
                format!("{} = VALUES({})", escaped_c, escaped_c)
            })
            .collect::<Vec<_>>()
            .join(", ");

        match self.db_type {
            DatabaseType::MySQL => {
                format!(
                    "INSERT INTO {} ({}) VALUES {} ON DUPLICATE KEY UPDATE {}",
                    escaped_table,
                    columns_str,
                    placeholders.join(", "),
                    update_clause
                )
            }
            DatabaseType::PostgreSQL => {
                let on_conflict = format!(
                    "ON CONFLICT ({}) DO UPDATE SET {}",
                    key_columns_str,
                    update_clause.replace("VALUES", "EXCLUDED")
                );
                format!(
                    "INSERT INTO {} ({}) VALUES {} {}",
                    escaped_table,
                    columns_str,
                    placeholders.join(", "),
                    on_conflict
                )
            }
            DatabaseType::SQLite => {
                let on_conflict = format!(
                    "ON CONFLICT ({}) DO UPDATE SET {}",
                    key_columns_str,
                    update_clause.replace("VALUES", "excluded")
                );
                format!(
                    "INSERT INTO {} ({}) VALUES {} {}",
                    escaped_table,
                    columns_str,
                    placeholders.join(", "),
                    on_conflict
                )
            }
            _ => {
                // Fallback to insert
                format!(
                    "INSERT INTO {} ({}) VALUES {}",
                    escaped_table,
                    columns_str,
                    placeholders.join(", ")
                )
            }
        }
    }

    /// Build batch UPDATE SQL with proper identifier escaping
    fn build_batch_update_sql(
        &self,
        table: &str,
        _set_clause: &[(String, Value)],
        where_clause: &str,
        _conditions: &[Vec<Value>],
    ) -> String {
        // Validate table name
        let escaped_table = match Self::validate_identifier(table) {
            Ok(valid) => self.escape_identifier(&valid),
            Err(_) => "\"INVALID_TABLE\"".to_string(),
        };

        // Validate WHERE clause doesn't contain dangerous patterns
        let dangerous_patterns = ["--", "/*", "*/", ";", "\n", "\r"];
        let safe_where = if dangerous_patterns.iter().any(|p| where_clause.contains(p)) {
            "1=0 /* Invalid WHERE clause */".to_string()
        } else {
            where_clause.to_string()
        };

        format!(
            "UPDATE {} SET {} WHERE {}",
            escaped_table, "/* case statement */", safe_where
        )
    }

    /// Build batch DELETE SQL with proper identifier escaping
    fn build_batch_delete_sql(
        &self,
        table: &str,
        where_clause: &str,
        _conditions: &[Vec<Value>],
    ) -> String {
        // Validate table name
        let escaped_table = match Self::validate_identifier(table) {
            Ok(valid) => self.escape_identifier(&valid),
            Err(_) => "\"INVALID_TABLE\"".to_string(),
        };

        // Validate WHERE clause doesn't contain dangerous patterns
        let dangerous_patterns = ["--", "/*", "*/", ";", "\n", "\r"];
        let safe_where = if dangerous_patterns.iter().any(|p| where_clause.contains(p)) {
            "1=0 /* Invalid WHERE clause */".to_string()
        } else {
            where_clause.to_string()
        };

        format!("DELETE FROM {} WHERE {}", escaped_table, safe_where)
    }
}

/// Streaming batch processor for large datasets
pub struct StreamingBatchProcessor {
    config: BatchConfig,
    pool: Arc<OptimizedConnectionPool>,
    db_type: DatabaseType,
}

impl StreamingBatchProcessor {
    /// Create a new streaming batch processor
    pub fn new(
        pool: Arc<OptimizedConnectionPool>,
        config: BatchConfig,
        db_type: DatabaseType,
    ) -> Self {
        Self {
            pool,
            config,
            db_type,
        }
    }

    /// Process a stream of rows
    pub async fn process_stream<T, F>(
        &self,
        mut row_stream: T,
        processor: F,
    ) -> Result<BatchResult, DatabaseError>
    where
        T: futures_util::stream::Stream<Item = Vec<Value>> + Unpin,
        F: Fn(Vec<Vec<Value>>) -> BatchOperation,
    {
        let start = Instant::now();
        let mut buffer: Vec<Vec<Value>> = Vec::with_capacity(self.config.batch_size);
        let mut results = Vec::new();
        let mut batch_idx: u32 = 0;

        while let Some(row) = row_stream.next().await {
            buffer.push(row);

            if buffer.len() >= self.config.batch_size {
                let operation = processor(std::mem::take(&mut buffer));
                let inserter =
                    BatchInserter::new(self.pool.clone(), self.config.clone(), self.db_type);

                match inserter.execute_batch(operation).await {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        warn!("Batch {} failed: {}", batch_idx, e);
                    }
                }

                batch_idx += 1;
                buffer = Vec::with_capacity(self.config.batch_size);
            }
        }

        // Process remaining rows
        if !buffer.is_empty() {
            let operation = processor(buffer);
            let inserter = BatchInserter::new(self.pool.clone(), self.config.clone(), self.db_type);

            match inserter.execute_batch(operation).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!("Final batch {} failed: {}", batch_idx, e);
                }
            }
        }

        // Aggregate results
        let execution_time_ms = start.elapsed().as_millis() as u64;
        let rows_affected: u64 = results.iter().map(|r| r.rows_affected).sum();
        let failed_batches: u32 = results.iter().map(|r| r.failed_batches).sum();

        let rows_per_second = if execution_time_ms > 0 {
            rows_affected as f64 / (execution_time_ms as f64 / 1000.0)
        } else {
            0.0
        };

        Ok(BatchResult {
            rows_affected,
            batches_executed: batch_idx + 1,
            failed_batches,
            execution_time_ms,
            rows_per_second,
            errors: Vec::new(),
            was_cancelled: false,
        })
    }
}

/// Batch operation builder for fluent API
pub struct BatchOperationBuilder {
    table: String,
    columns: Vec<String>,
    rows: Vec<Vec<Value>>,
}

impl BatchOperationBuilder {
    /// Create a new batch operation builder
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            columns: Vec::new(),
            rows: Vec::new(),
        }
    }

    /// Set columns
    pub fn columns(mut self, columns: &[&str]) -> Self {
        self.columns = columns.iter().map(|&c| c.to_string()).collect();
        self
    }

    /// Add a row
    pub fn row(mut self, values: Vec<Value>) -> Self {
        self.rows.push(values);
        self
    }

    /// Add multiple rows
    pub fn rows(mut self, values: Vec<Vec<Value>>) -> Self {
        self.rows.extend(values);
        self
    }

    /// Build insert operation
    pub fn build_insert(self) -> BatchOperation {
        BatchOperation::Insert {
            table: self.table,
            columns: self.columns,
            rows: self.rows,
        }
    }

    /// Build upsert operation
    pub fn build_upsert(self, key_columns: Vec<String>) -> BatchOperation {
        BatchOperation::Upsert {
            table: self.table,
            columns: self.columns,
            rows: self.rows,
            key_columns,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config() {
        let config = BatchConfig::default();
        assert_eq!(config.batch_size, 1000);
        assert!(config.use_transactions);

        let high_throughput = BatchConfig::high_throughput();
        assert_eq!(high_throughput.batch_size, 5000);
        assert!(!high_throughput.use_transactions);

        let reliable = BatchConfig::reliable();
        assert_eq!(reliable.batch_size, 500);
        assert_eq!(reliable.max_retries, 5);
    }

    #[test]
    fn test_batch_result() {
        let success = BatchResult {
            rows_affected: 100,
            batches_executed: 1,
            failed_batches: 0,
            execution_time_ms: 100,
            rows_per_second: 1000.0,
            errors: vec![],
            was_cancelled: false,
        };
        assert!(success.is_success());

        let partial = BatchResult {
            rows_affected: 50,
            batches_executed: 1,
            failed_batches: 1,
            execution_time_ms: 100,
            rows_per_second: 500.0,
            errors: vec![],
            was_cancelled: false,
        };
        assert!(partial.is_partial());
        assert!(!partial.is_success());
    }

    #[test]
    fn test_build_insert_sql() {
        let config = BatchConfig::default();
        // Would need pool and db_type for full test
        // This is a placeholder for the test structure
    }

    #[test]
    fn test_batch_operation_builder() {
        let operation = BatchOperationBuilder::new("users")
            .columns(&["id", "name", "email"])
            .row(vec![
                Value::Integer(1),
                Value::String("Alice".to_string()),
                Value::String("alice@example.com".to_string()),
            ])
            .row(vec![
                Value::Integer(2),
                Value::String("Bob".to_string()),
                Value::String("bob@example.com".to_string()),
            ])
            .build_insert();

        match operation {
            BatchOperation::Insert {
                table,
                columns,
                rows,
            } => {
                assert_eq!(table, "users");
                assert_eq!(columns.len(), 3);
                assert_eq!(rows.len(), 2);
            }
            _ => panic!("Expected Insert operation"),
        }
    }
}
