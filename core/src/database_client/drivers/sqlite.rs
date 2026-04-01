//! SQLite database driver

use crate::database_client::drivers::{
    ColumnInfo, ConnectionInfo, DatabaseStats, ForeignKeyInfo, IndexInfo, TableDetail, TableInfo,
    TableType,
};
use crate::database_client::{DatabaseDriver, DatabaseError, DatabaseType};
use crate::database_client::{
    DatabaseSchema, PerformanceMetrics, QueryCell, QueryResult, QueryRow, SchemaColumn, SchemaTable,
};
use async_trait::async_trait;
use rusqlite::{Connection, OpenFlags, Row};
use std::path::Path;
use std::sync::Mutex as StdMutex;

/// SQLite driver
pub struct SqliteDriver {
    connection: Option<StdMutex<Connection>>,
    info: Option<ConnectionInfo>,
    in_transaction: bool,
}

impl SqliteDriver {
    pub fn new() -> Self {
        Self {
            connection: None,
            info: None,
            in_transaction: false,
        }
    }

    fn get_conn(&self) -> Result<std::sync::MutexGuard<Connection>, DatabaseError> {
        self.connection
            .as_ref()
            .ok_or_else(|| DatabaseError::ConnectionError("Not connected".to_string()))?
            .lock()
            .map_err(|_| DatabaseError::Unknown("Lock poisoned".to_string()))
    }

    fn map_row_to_cells(
        &self,
        row: &Row,
        column_count: usize,
    ) -> Result<Vec<QueryCell>, rusqlite::Error> {
        let mut cells = Vec::with_capacity(column_count);

        for i in 0..column_count {
            let value = match row.get_ref(i)? {
                rusqlite::types::ValueRef::Null => QueryCell::Null,
                rusqlite::types::ValueRef::Integer(v) => QueryCell::Integer(v),
                rusqlite::types::ValueRef::Real(v) => QueryCell::Float(v),
                rusqlite::types::ValueRef::Text(v) => {
                    QueryCell::String(String::from_utf8_lossy(v).to_string())
                }
                rusqlite::types::ValueRef::Blob(v) => QueryCell::Blob(v.to_vec()),
            };
            cells.push(value);
        }

        Ok(cells)
    }

    fn map_sqlite_type(&self, type_name: &str) -> String {
        let upper = type_name.to_uppercase();
        if upper.contains("INT") {
            "INTEGER".to_string()
        } else if upper.contains("CHAR") || upper.contains("CLOB") || upper.contains("TEXT") {
            "TEXT".to_string()
        } else if upper.contains("BLOB") {
            "BLOB".to_string()
        } else if upper.contains("REAL") || upper.contains("FLOA") || upper.contains("DOUB") {
            "REAL".to_string()
        } else if upper.contains("NUMERIC") || upper.contains("DECIMAL") || upper.contains("BOOL") {
            "NUMERIC".to_string()
        } else {
            "TEXT".to_string()
        }
    }
}

#[async_trait]
impl DatabaseDriver for SqliteDriver {
    fn db_type(&self) -> DatabaseType {
        DatabaseType::SQLite
    }

    async fn connect(&mut self, info: &ConnectionInfo) -> Result<(), DatabaseError> {
        let path = Path::new(&info.database);

        let conn = if path.exists() {
            Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)
        } else {
            Connection::open(path)
        }
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        self.connection = Some(StdMutex::new(conn));
        self.info = Some(info.clone());

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), DatabaseError> {
        self.connection = None;
        self.info = None;
        self.in_transaction = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    async fn query(&self, sql: &str) -> Result<QueryResult, DatabaseError> {
        self.execute_query(sql).await
    }

    async fn execute_query(&self, query: &str) -> Result<QueryResult, DatabaseError> {
        let conn = self.get_conn()?;

        let start = std::time::Instant::now();

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let column_count = stmt.column_count();
        let column_names: Vec<String> =
            stmt.column_names().iter().map(|&s| s.to_string()).collect();

        let mut rows = Vec::new();

        let row_iter = stmt
            .query_map([], |row| self.map_row_to_cells(row, column_count))
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        for row_result in row_iter {
            let cells = row_result.map_err(|e| DatabaseError::QueryError(e.to_string()))?;
            rows.push(QueryRow { cells });
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            columns: column_names,
            rows,
            execution_time_ms,
            affected_rows: None,
            warning_count: 0,
            info_message: None,
        })
    }

    async fn execute(&self, query: &str) -> Result<u64, DatabaseError> {
        let conn = self.get_conn()?;

        let affected = conn
            .execute(query, [])
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        Ok(affected as u64)
    }

    async fn get_schema(&self) -> Result<DatabaseSchema, DatabaseError> {
        let tables = self.get_tables().await?;
        let mut schema_tables = Vec::new();

        for table in tables {
            let detail = self.get_table_info(&table.name).await?;

            let columns = detail
                .columns
                .iter()
                .map(|col| SchemaColumn {
                    name: col.name.clone(),
                    data_type: col.data_type.clone(),
                    nullable: col.nullable,
                    default: col.default_value.clone(),
                    is_primary_key: col.is_primary_key,
                    is_foreign_key: detail.foreign_keys.iter().any(|fk| fk.column == col.name),
                    comment: col.comment.clone(),
                    extra: None,
                })
                .collect();

            schema_tables.push(SchemaTable {
                name: table.name,
                schema: table.schema.clone(),
                table_type: match table.table_type {
                    TableType::Table => crate::database_client::schema::SchemaTableType::Table,
                    TableType::View => crate::database_client::schema::SchemaTableType::View,
                    TableType::SystemTable => {
                        crate::database_client::schema::SchemaTableType::System
                    }
                    _ => crate::database_client::schema::SchemaTableType::Table,
                },
                columns,
                indexes: detail
                    .indexes
                    .iter()
                    .map(|idx| crate::database_client::schema::SchemaIndex {
                        name: idx.name.clone(),
                        columns: idx.columns.clone(),
                        unique: idx.unique,
                        primary: idx.primary,
                        index_type: Some(idx.index_type.clone()),
                        comment: None,
                    })
                    .collect(),
                foreign_keys: detail
                    .foreign_keys
                    .iter()
                    .map(|fk| crate::database_client::schema::SchemaForeignKey {
                        name: fk.name.clone(),
                        column: fk.column.clone(),
                        referenced_table: fk.referenced_table.clone(),
                        referenced_column: fk.referenced_column.clone(),
                        on_update: Some(fk.on_update.clone()),
                        on_delete: Some(fk.on_delete.clone()),
                    })
                    .collect(),
                row_count: table.row_count,
                comment: table.comment,
            });
        }

        Ok(DatabaseSchema {
            database_name: self
                .info
                .as_ref()
                .map(|c| c.database.clone())
                .unwrap_or_default(),
            tables: schema_tables,
            views: Vec::new(),
            procedures: Vec::new(),
            functions: Vec::new(),
            sequences: Vec::new(),
            enums: Vec::new(),
        })
    }

    async fn list_databases(&self) -> Result<Vec<String>, DatabaseError> {
        // SQLite is file-based, single database per connection
        Ok(vec![])
    }

    async fn get_tables(&self) -> Result<Vec<TableInfo>, DatabaseError> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view') ORDER BY name"
        ).map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let type_str: String = row.get(1)?;
                let table_type = match type_str.as_str() {
                    "view" => TableType::View,
                    _ => TableType::Table,
                };
                Ok((name, table_type))
            })
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let mut tables = Vec::new();

        for row in rows {
            let (name, table_type) = row.map_err(|e| DatabaseError::QueryError(e.to_string()))?;

            // Get row count
            let count_query = format!("SELECT COUNT(*) FROM {}", name);
            let row_count: Option<u64> = conn
                .query_row(&count_query, [], |r| r.get::<_, i64>(0).map(|v| v as u64))
                .ok();

            tables.push(TableInfo {
                name,
                schema: Some("main".to_string()),
                table_type,
                columns: Vec::new(),
                indexes: Vec::new(),
                foreign_keys: Vec::new(),
                row_count,
                size_bytes: None,
                created_at: None,
                updated_at: None,
                comment: None,
            });
        }

        Ok(tables)
    }

    async fn get_table(&self, table_name: &str) -> Result<TableInfo, DatabaseError> {
        let tables = self.get_tables().await?;
        tables
            .into_iter()
            .find(|t| t.name == table_name)
            .ok_or_else(|| DatabaseError::SchemaError(format!("Table not found: {}", table_name)))
    }

    async fn get_table_info(&self, table_name: &str) -> Result<TableDetail, DatabaseError> {
        let conn = self.get_conn()?;

        // Get columns
        let mut columns_stmt = conn
            .prepare("PRAGMA table_info(?)")
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

        let columns_rows = columns_stmt
            .query_map([table_name], |row| {
                let cid: i32 = row.get(0)?;
                let name: String = row.get(1)?;
                let type_str: String = row.get(2)?;
                let not_null: i32 = row.get(3)?;
                let default_value: Option<String> = row.get(4)?;
                let primary_key: i32 = row.get(5)?;

                Ok((cid, name, type_str, not_null, default_value, primary_key))
            })
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

        let mut columns = Vec::new();
        for row in columns_rows {
            let (cid, name, type_str, not_null, default_value, primary_key) =
                row.map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

            columns.push(ColumnInfo {
                name,
                data_type: self.map_sqlite_type(&type_str),
                nullable: not_null == 0,
                default_value,
                is_primary_key: primary_key > 0,
                is_unique: false,
                is_auto_increment: type_str.to_uppercase().contains("INTEGER") && primary_key > 0,
                max_length: None,
                numeric_precision: None,
                numeric_scale: None,
                ordinal_position: (cid + 1) as u32,
                comment: None,
                collation: None,
                is_foreign_key: false,
            });
        }

        // Get indexes
        let mut indexes_stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND tbl_name=?")
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

        let index_names: Vec<String> = indexes_stmt
            .query_map([table_name], |row| row.get::<_, String>(0))
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        let mut indexes = Vec::new();
        for index_name in index_names {
            let mut idx_stmt = conn
                .prepare("PRAGMA index_info(?)")
                .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

            let idx_cols: Vec<String> = idx_stmt
                .query_map([&index_name], |row| row.get::<_, String>(2))
                .map_err(|e| DatabaseError::SchemaError(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            // Check if primary key index
            let is_primary = index_name.starts_with("sqlite_autoindex")
                && columns.iter().filter(|c| c.is_primary_key).count() == idx_cols.len();

            indexes.push(IndexInfo {
                name: index_name,
                columns: idx_cols,
                unique: false,
                primary: is_primary,
                index_type: "B-TREE".to_string(),
                cardinality: None,
            });
        }

        // Get foreign keys
        let mut fk_stmt = conn
            .prepare("PRAGMA foreign_key_list(?)")
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

        let fk_rows = fk_stmt
            .query_map([table_name], |row| {
                let id: i32 = row.get(0)?;
                let seq: i32 = row.get(1)?;
                let referenced_table: String = row.get(2)?;
                let from_col: String = row.get(3)?;
                let to_col: String = row.get(4)?;
                let on_update: String = row.get(5)?;
                let on_delete: String = row.get(6)?;

                Ok((
                    id,
                    seq,
                    referenced_table,
                    from_col,
                    to_col,
                    on_update,
                    on_delete,
                ))
            })
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

        let mut foreign_keys = Vec::new();
        for row in fk_rows {
            let (id, _, referenced_table, from_col, to_col, on_update, on_delete) =
                row.map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

            foreign_keys.push(ForeignKeyInfo {
                name: format!("fk_{}_{}", table_name, id),
                column: from_col,
                referenced_table,
                referenced_column: to_col,
                on_update,
                on_delete,
            });
        }

        // Update columns with foreign key info
        for col in &mut columns {
            col.is_foreign_key = foreign_keys.iter().any(|fk| fk.column == col.name);
        }

        // Build table info
        let info = TableInfo {
            name: table_name.to_string(),
            schema: Some("main".to_string()),
            table_type: TableType::Table,
            columns: columns.clone(),
            indexes: indexes.clone(),
            foreign_keys: foreign_keys.clone(),
            row_count: conn
                .query_row(&format!("SELECT COUNT(*) FROM {}", table_name), [], |r| {
                    Ok(r.get::<_, i64>(0).map(|v| v as u64).ok())
                })
                .ok()
                .flatten(),
            size_bytes: None,
            created_at: None,
            updated_at: None,
            comment: None,
        };

        Ok(TableDetail {
            info,
            columns,
            indexes,
            foreign_keys,
            constraints: Vec::new(),
            triggers: Vec::new(),
            privileges: Vec::new(),
        })
    }

    async fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        let conn = self.get_conn()?;

        let page_count: i64 = conn
            .query_row("PRAGMA page_count", [], |r| r.get(0))
            .unwrap_or(0);
        let page_size: i64 = conn
            .query_row("PRAGMA page_size", [], |r| r.get(0))
            .unwrap_or(0);
        let table_count = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;
        let index_count = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;

        let version: String = conn
            .query_row("SELECT sqlite_version()", [], |r| r.get(0))
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(DatabaseStats {
            name: self
                .info
                .as_ref()
                .map(|i| i.database.clone())
                .unwrap_or_else(|| "sqlite".to_string()),
            size_bytes: (page_count * page_size) as u64,
            table_count,
            index_count,
            connection_count: Some(1),
            uptime_seconds: None,
            version,
        })
    }

    async fn begin_transaction(&mut self) -> Result<(), DatabaseError> {
        {
            let conn = self.get_conn()?;
            conn.execute("BEGIN", [])
                .map_err(|e| DatabaseError::QueryError(e.to_string()))?;
        }
        self.in_transaction = true;
        Ok(())
    }

    async fn commit(&mut self) -> Result<(), DatabaseError> {
        {
            let conn = self.get_conn()?;
            conn.execute("COMMIT", [])
                .map_err(|e| DatabaseError::QueryError(e.to_string()))?;
        }
        self.in_transaction = false;
        Ok(())
    }

    async fn rollback(&mut self) -> Result<(), DatabaseError> {
        {
            let conn = self.get_conn()?;
            conn.execute("ROLLBACK", [])
                .map_err(|e| DatabaseError::QueryError(e.to_string()))?;
        }
        self.in_transaction = false;
        Ok(())
    }

    async fn get_performance_metrics(&self) -> Result<PerformanceMetrics, DatabaseError> {
        let conn = self.get_conn()?;

        let page_count: i64 = conn
            .query_row("PRAGMA page_count", [], |r| r.get(0))
            .unwrap_or(0);
        let page_size: i64 = conn
            .query_row("PRAGMA page_size", [], |r| r.get(0))
            .unwrap_or(0);
        let freelist_count: i64 = conn
            .query_row("PRAGMA freelist_count", [], |r| r.get(0))
            .unwrap_or(0);

        Ok(PerformanceMetrics {
            queries_per_second: 0.0,
            active_connections: 1,
            total_connections: 1,
            cache_hit_ratio: None,
            slow_queries: 0,
            avg_query_time_ms: 0.0,
            total_bytes_received: (page_count * page_size) as u64,
            total_bytes_sent: 0,
            table_statistics: Vec::new(),
            additional_metrics: [
                ("page_count".to_string(), page_count as f64),
                ("freelist_count".to_string(), freelist_count as f64),
                (
                    "database_size_bytes".to_string(),
                    (page_count * page_size) as f64,
                ),
            ]
            .into_iter()
            .collect(),
        })
    }

    async fn cancel(&self) -> Result<(), DatabaseError> {
        // SQLite doesn't support query cancellation directly
        // This would require interrupt handling at connection level
        Err(DatabaseError::Unknown(
            "Query cancellation not supported for SQLite".to_string(),
        ))
    }

    async fn ping(&self) -> Result<(), DatabaseError> {
        let conn = self.get_conn()?;
        conn.execute("SELECT 1", [])
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        Ok(())
    }
}

impl Default for SqliteDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SqliteDriver {
    fn drop(&mut self) {
        if self.in_transaction {
            if let Some(ref conn) = self.connection {
                let _ = conn.lock().unwrap().execute("ROLLBACK", []);
            }
        }
    }
}
