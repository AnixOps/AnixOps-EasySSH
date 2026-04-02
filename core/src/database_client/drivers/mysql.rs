//! MySQL database driver

use async_trait::async_trait;
use mysql_async::prelude::Queryable;
use mysql_async::{params, Conn, OptsBuilder, Row as MysqlRow};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database_client::drivers::{
    ColumnInfo, ConnectionInfo, DatabaseStats, ForeignKeyInfo, IndexInfo, TableDetail, TableInfo,
    TableType,
};
use crate::database_client::{DatabaseDriver, DatabaseError, DatabaseType};
use crate::database_client::{
    DatabaseSchema, PerformanceMetrics, QueryCell, QueryResult, QueryRow, SchemaColumn, SchemaTable,
};

/// MySQL driver
pub struct MySqlDriver {
    connection: Option<Arc<Mutex<Conn>>>,
    info: Option<ConnectionInfo>,
}

impl MySqlDriver {
    pub fn new() -> Self {
        Self {
            connection: None,
            info: None,
        }
    }

    fn get_conn(&self) -> Result<Arc<Mutex<Conn>>, DatabaseError> {
        self.connection
            .as_ref()
            .ok_or_else(|| DatabaseError::ConnectionError("Not connected".to_string()))
            .cloned()
    }

    fn map_mysql_value(&self, value: mysql_async::Value) -> QueryCell {
        match value {
            mysql_async::Value::NULL => QueryCell::Null,
            mysql_async::Value::Bytes(b) => {
                // Try to parse as string, fallback to blob
                match String::from_utf8(b.clone()) {
                    Ok(s) => QueryCell::String(s),
                    Err(_) => QueryCell::Blob(b),
                }
            }
            mysql_async::Value::Int(i) => QueryCell::Integer(i),
            mysql_async::Value::UInt(u) => QueryCell::Integer(u as i64),
            mysql_async::Value::Float(f) => QueryCell::Float(f as f64),
            mysql_async::Value::Double(d) => QueryCell::Float(d),
            mysql_async::Value::Date(y, m, d, h, min, s, us) => {
                let date_str = format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
                    y, m, d, h, min, s, us
                );
                QueryCell::String(date_str)
            }
            mysql_async::Value::Time(is_neg, days, hours, minutes, seconds, micros) => {
                let time_str = format!(
                    "{}{:02}:{:02}:{:02}.{:06}",
                    if is_neg { "-" } else { "" },
                    days * 24 + (hours as u32),
                    minutes,
                    seconds,
                    micros
                );
                QueryCell::String(time_str)
            }
        }
    }

    fn map_mysql_type(&self, type_name: &str) -> String {
        let upper = type_name.to_uppercase();
        if upper.contains("INT") {
            "INTEGER".to_string()
        } else if upper.contains("VARCHAR") || upper.contains("CHAR") || upper.contains("TEXT") {
            "TEXT".to_string()
        } else if upper.contains("BLOB") || upper.contains("BINARY") {
            "BLOB".to_string()
        } else if upper.contains("FLOAT") {
            "REAL".to_string()
        } else if upper.contains("DOUBLE") || upper.contains("DECIMAL") || upper.contains("NUMERIC")
        {
            "NUMERIC".to_string()
        } else if upper.contains("DATE") {
            "DATE".to_string()
        } else if upper.contains("TIME") {
            "TIME".to_string()
        } else if upper.contains("BOOL") {
            "BOOLEAN".to_string()
        } else {
            "TEXT".to_string()
        }
    }
}

#[async_trait]
impl DatabaseDriver for MySqlDriver {
    fn db_type(&self) -> DatabaseType {
        DatabaseType::MySQL
    }

    async fn connect(&mut self, info: &ConnectionInfo) -> Result<(), DatabaseError> {
        let opts = OptsBuilder::default()
            .ip_or_hostname(&info.host)
            .tcp_port(info.port)
            .user(Some(&info.username))
            .pass(info.password.as_deref())
            .db_name(Some(&info.database));

        let conn = Conn::new(opts)
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        self.connection = Some(Arc::new(Mutex::new(conn)));
        self.info = Some(info.clone());

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), DatabaseError> {
        // Connection is dropped automatically when the Arc is dropped
        self.connection = None;
        self.info = None;
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
        let mut c = conn.lock().await;

        let start = std::time::Instant::now();

        let result: Vec<MysqlRow> = c
            .query(query)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        if result.is_empty() {
            return Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                affected_rows: Some(c.affected_rows()),
                warning_count: 0,
                info_message: None,
            });
        }

        // Get column names from first row
        let column_names: Vec<String> = result[0]
            .columns()
            .iter()
            .map(|col| col.name_str().to_string())
            .collect();

        let mut rows = Vec::new();
        for row in result {
            let mut cells = Vec::new();
            for i in 0..row.len() {
                let value = row.get(i).unwrap_or(mysql_async::Value::NULL);
                cells.push(self.map_mysql_value(value));
            }
            rows.push(QueryRow { cells });
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            columns: column_names,
            rows,
            execution_time_ms,
            affected_rows: Some(c.affected_rows()),
            warning_count: 0,
            info_message: None,
        })
    }

    async fn execute(&self, query: &str) -> Result<u64, DatabaseError> {
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;

        c.query_drop(query)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        Ok(c.affected_rows())
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
        let result = self.query("SHOW DATABASES").await?;
        let databases: Vec<String> = result
            .rows
            .iter()
            .filter_map(|row| {
                row.cells.get(0).map(|cell| match cell {
                    QueryCell::String(s) => s.clone(),
                    _ => cell.to_string(),
                })
            })
            .filter(|db| {
                db != "information_schema"
                    && db != "mysql"
                    && db != "performance_schema"
                    && db != "sys"
            })
            .collect();
        Ok(databases)
    }

    async fn get_tables(&self) -> Result<Vec<TableInfo>, DatabaseError> {
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;

        let query = "SELECT TABLE_NAME, TABLE_TYPE, TABLE_ROWS, TABLE_COMMENT
                     FROM INFORMATION_SCHEMA.TABLES
                     WHERE TABLE_SCHEMA = DATABASE()";

        let result: Vec<MysqlRow> = c
            .query(query)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let mut tables = Vec::new();
        for row in result {
            let name: String = row.get(0).unwrap_or_default();
            let type_str: String = row.get(1).unwrap_or_else(|| "BASE TABLE".to_string());
            let row_count: Option<u64> = row.get(2);
            let comment: Option<String> = row.get(3);

            let table_type = if type_str == "VIEW" {
                TableType::View
            } else {
                TableType::Table
            };

            tables.push(TableInfo {
                name,
                schema: None,
                table_type,
                columns: Vec::new(),
                indexes: Vec::new(),
                foreign_keys: Vec::new(),
                row_count,
                size_bytes: None,
                created_at: None,
                updated_at: None,
                comment,
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
        let mut c = conn.lock().await;

        // Get columns - Use parameterized query to prevent SQL injection
        let query = "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, COLUMN_DEFAULT, \
                    EXTRA, COLUMN_COMMENT, CHARACTER_MAXIMUM_LENGTH, \
                    NUMERIC_PRECISION, NUMERIC_SCALE, ORDINAL_POSITION \
             FROM INFORMATION_SCHEMA.COLUMNS \
             WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?";

        let result: Vec<MysqlRow> = c
            .exec_iter(query, params![table_name])
            .await
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?
            .collect_and_drop()
            .await
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

        let mut columns = Vec::new();
        for row in result {
            let name: String = row.get(0).unwrap_or_default();
            let data_type: String = row.get(1).unwrap_or_else(|| "TEXT".to_string());
            let is_nullable: String = row.get(2).unwrap_or_else(|| "YES".to_string());
            let default_value: Option<String> = row.get(3);
            let extra: String = row.get(4).unwrap_or_default();
            let comment: Option<String> = row.get(5);
            let max_length: Option<u32> = row.get(6);
            let numeric_precision: Option<u32> = row.get(7);
            let numeric_scale: Option<u32> = row.get(8);
            let ordinal_position: u32 = row.get(9).unwrap_or(0);

            columns.push(ColumnInfo {
                name,
                data_type: self.map_mysql_type(&data_type),
                nullable: is_nullable == "YES",
                default_value,
                is_primary_key: extra.contains("auto_increment"),
                is_unique: false,
                is_auto_increment: extra.contains("auto_increment"),
                max_length,
                numeric_precision,
                numeric_scale,
                ordinal_position,
                comment,
                collation: None,
                is_foreign_key: false,
            });
        }

        // Get indexes - Use parameterized query to prevent SQL injection
        let query = "SELECT INDEX_NAME, COLUMN_NAME, NON_UNIQUE, SEQ_IN_INDEX \
             FROM INFORMATION_SCHEMA.STATISTICS \
             WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?";

        let result: Vec<MysqlRow> = c
            .exec_iter(query, params![table_name])
            .await
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?
            .collect_and_drop()
            .await
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

        let mut indexes_map: std::collections::HashMap<String, (bool, Vec<String>, bool)> =
            std::collections::HashMap::new();
        for row in result {
            let name: String = row.get(0).unwrap_or_default();
            let column: String = row.get(1).unwrap_or_default();
            let non_unique: i64 = row.get(2).unwrap_or(1);
            let is_primary = name == "PRIMARY";

            let entry = indexes_map.entry(name.clone()).or_insert((
                non_unique == 0,
                Vec::new(),
                is_primary,
            ));
            entry.1.push(column);
        }

        let indexes: Vec<IndexInfo> = indexes_map
            .into_iter()
            .map(|(name, (unique, columns, primary))| IndexInfo {
                name,
                columns,
                unique,
                primary,
                index_type: "BTREE".to_string(),
                cardinality: None,
            })
            .collect();

        // Get foreign keys - Use parameterized query to prevent SQL injection
        let query = "SELECT kcu.CONSTRAINT_NAME, kcu.COLUMN_NAME, kcu.REFERENCED_TABLE_NAME, \
                    kcu.REFERENCED_COLUMN_NAME, rc.UPDATE_RULE, rc.DELETE_RULE \
             FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu \
             JOIN INFORMATION_SCHEMA.REFERENTIAL_CONSTRAINTS rc \
             ON kcu.CONSTRAINT_NAME = rc.CONSTRAINT_NAME AND kcu.TABLE_SCHEMA = rc.CONSTRAINT_SCHEMA \
             WHERE kcu.TABLE_SCHEMA = DATABASE() \
             AND kcu.TABLE_NAME = ? \
             AND kcu.REFERENCED_TABLE_NAME IS NOT NULL";

        let result: Vec<MysqlRow> = c
            .exec_iter(query, params![table_name])
            .await
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?
            .collect_and_drop()
            .await
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

        let mut foreign_keys = Vec::new();
        for row in result {
            let name: String = row.get(0).unwrap_or_default();
            let column: String = row.get(1).unwrap_or_default();
            let ref_table: String = row.get(2).unwrap_or_default();
            let ref_column: String = row.get(3).unwrap_or_default();
            let on_update: String = row.get(4).unwrap_or_else(|| "NO ACTION".to_string());
            let on_delete: String = row.get(5).unwrap_or_else(|| "NO ACTION".to_string());

            foreign_keys.push(ForeignKeyInfo {
                name,
                column,
                referenced_table: ref_table,
                referenced_column: ref_column,
                on_update,
                on_delete,
            });
        }

        // Update columns with primary key and foreign key info
        for col in &mut columns {
            col.is_primary_key = indexes
                .iter()
                .any(|idx| idx.primary && idx.columns.contains(&col.name));
            col.is_foreign_key = foreign_keys.iter().any(|fk| fk.column == col.name);
        }

        // Build table info
        let info = TableInfo {
            name: table_name.to_string(),
            schema: None,
            table_type: TableType::Table,
            columns: columns.clone(),
            indexes: indexes.clone(),
            foreign_keys: foreign_keys.clone(),
            row_count: None,
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
        let mut c = conn.lock().await;

        let version: String = c
            .query_first("SELECT VERSION()")
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?
            .unwrap_or_else(|| "unknown".to_string());

        let result: Vec<MysqlRow> = c
            .query(
                "SELECT SUM(DATA_LENGTH + INDEX_LENGTH) as size,
                    COUNT(*) as table_count
             FROM INFORMATION_SCHEMA.TABLES
             WHERE TABLE_SCHEMA = DATABASE()",
            )
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let (size, table_count) = if let Some(row) = result.first() {
            let size: Option<i64> = row.get(0);
            let count: Option<i64> = row.get(1);
            (size.unwrap_or(0) as u64, count.unwrap_or(0) as usize)
        } else {
            (0, 0)
        };

        Ok(DatabaseStats {
            name: self
                .info
                .as_ref()
                .map(|i| i.database.clone())
                .unwrap_or_else(|| "mysql".to_string()),
            size_bytes: size,
            table_count,
            index_count: 0,
            connection_count: None,
            uptime_seconds: None,
            version,
        })
    }

    async fn begin_transaction(&mut self) -> Result<(), DatabaseError> {
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;
        c.query_drop("START TRANSACTION")
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))
    }

    async fn commit(&mut self) -> Result<(), DatabaseError> {
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;
        c.query_drop("COMMIT")
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))
    }

    async fn rollback(&mut self) -> Result<(), DatabaseError> {
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;
        c.query_drop("ROLLBACK")
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))
    }

    async fn get_performance_metrics(&self) -> Result<PerformanceMetrics, DatabaseError> {
        // MySQL specific performance metrics
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;

        let status: Vec<(String, String)> = c
            .query("SHOW STATUS LIKE 'Threads_%'")
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let mut active_connections = 0;
        let mut total_connections = 0;

        for (name, value) in status {
            if name == "Threads_connected" {
                active_connections = value.parse().unwrap_or(0);
            } else if name == "Threads_created" {
                total_connections = value.parse().unwrap_or(0);
            }
        }

        Ok(PerformanceMetrics {
            queries_per_second: 0.0,
            active_connections,
            total_connections,
            cache_hit_ratio: None,
            slow_queries: 0,
            avg_query_time_ms: 0.0,
            total_bytes_received: 0,
            total_bytes_sent: 0,
            table_statistics: Vec::new(),
            additional_metrics: std::collections::HashMap::new(),
        })
    }

    async fn cancel(&self) -> Result<(), DatabaseError> {
        // MySQL doesn't support query cancellation directly
        Err(DatabaseError::Unknown(
            "Query cancellation not supported for MySQL".to_string(),
        ))
    }

    async fn ping(&self) -> Result<(), DatabaseError> {
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;
        c.query_drop("SELECT 1")
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))
    }
}

impl Default for MySqlDriver {
    fn default() -> Self {
        Self::new()
    }
}
