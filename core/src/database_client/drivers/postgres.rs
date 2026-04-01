//! PostgreSQL database driver

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::types::Type;
use tokio_postgres::{Client, Config, NoTls, Row as PostgresRow};

use crate::database_client::drivers::{
    ColumnInfo, ConnectionInfo, DatabaseStats, ForeignKeyInfo, IndexInfo, TableDetail, TableInfo,
    TableType,
};
use crate::database_client::{DatabaseDriver, DatabaseError, DatabaseType};
use crate::database_client::{
    DatabaseSchema, PerformanceMetrics, QueryCell, QueryResult, QueryRow, SchemaColumn, SchemaTable,
};

/// PostgreSQL driver
pub struct PostgresDriver {
    connection: Option<Arc<Mutex<Client>>>,
    info: Option<ConnectionInfo>,
}

impl PostgresDriver {
    pub fn new() -> Self {
        Self {
            connection: None,
            info: None,
        }
    }

    fn get_conn(&self) -> Result<Arc<Mutex<Client>>, DatabaseError> {
        self.connection
            .as_ref()
            .ok_or_else(|| DatabaseError::ConnectionError("Not connected".to_string()))
            .cloned()
    }

    fn map_postgres_value(&self, row: &PostgresRow, idx: usize, _type_: &Type) -> QueryCell {
        // Try different types based on PostgreSQL type
        if let Ok(val) = row.try_get::<_, String>(idx) {
            QueryCell::String(val)
        } else if let Ok(val) = row.try_get::<_, i32>(idx) {
            QueryCell::Integer(val as i64)
        } else if let Ok(val) = row.try_get::<_, i64>(idx) {
            QueryCell::Integer(val)
        } else if let Ok(val) = row.try_get::<_, f64>(idx) {
            QueryCell::Float(val)
        } else if let Ok(val) = row.try_get::<_, bool>(idx) {
            QueryCell::Boolean(val)
        } else if let Ok(val) = row.try_get::<_, Vec<u8>>(idx) {
            QueryCell::Blob(val)
        } else {
            // Try as string as fallback
            match row.try_get::<_, &str>(idx) {
                Ok(s) => QueryCell::String(s.to_string()),
                Err(_) => QueryCell::Null,
            }
        }
    }

    fn map_postgres_type(&self, type_: &str) -> String {
        let upper = type_.to_uppercase();
        if upper.starts_with("INT") || upper == "SERIAL" || upper == "BIGSERIAL" {
            "INTEGER".to_string()
        } else if upper.starts_with("VARCHAR") || upper.starts_with("CHAR") || upper == "TEXT" {
            "TEXT".to_string()
        } else if upper == "BYTEA" {
            "BLOB".to_string()
        } else if upper == "REAL" || upper == "FLOAT4" {
            "REAL".to_string()
        } else if upper == "DOUBLE PRECISION"
            || upper == "FLOAT8"
            || upper.starts_with("NUMERIC")
            || upper.starts_with("DECIMAL")
        {
            "NUMERIC".to_string()
        } else if upper == "DATE" {
            "DATE".to_string()
        } else if upper.starts_with("TIMESTAMP") {
            "DATETIME".to_string()
        } else if upper == "TIME" {
            "TIME".to_string()
        } else if upper == "BOOLEAN" || upper == "BOOL" {
            "BOOLEAN".to_string()
        } else if upper == "JSON" || upper == "JSONB" {
            "JSON".to_string()
        } else if upper == "UUID" {
            "UUID".to_string()
        } else {
            "TEXT".to_string()
        }
    }
}

#[async_trait]
impl DatabaseDriver for PostgresDriver {
    fn db_type(&self) -> DatabaseType {
        DatabaseType::PostgreSQL
    }

    async fn connect(&mut self, info: &ConnectionInfo) -> Result<(), DatabaseError> {
        let mut config = Config::new();
        config.host(&info.host);
        config.port(info.port);
        config.user(&info.username);
        config.dbname(&info.database);

        if let Some(ref pwd) = info.password {
            config.password(pwd);
        }

        let (client, connection) = config
            .connect(NoTls)
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        // Spawn connection in background
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        self.connection = Some(Arc::new(Mutex::new(client)));
        self.info = Some(info.clone());

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), DatabaseError> {
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
        let c = conn.lock().await;

        let start = std::time::Instant::now();

        let rows = c
            .query(query, &[])
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        if rows.is_empty() {
            return Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                affected_rows: None,
                warning_count: 0,
                info_message: None,
            });
        }

        // Get column names and types from first row
        let columns: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|col| col.name().to_string())
            .collect();

        let mut result_rows = Vec::new();
        for row in &rows {
            let mut cells = Vec::new();
            for (i, col) in row.columns().iter().enumerate() {
                let cell = self.map_postgres_value(row, i, col.type_());
                cells.push(cell);
            }
            result_rows.push(QueryRow { cells });
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            columns,
            rows: result_rows,
            execution_time_ms,
            affected_rows: None,
            warning_count: 0,
            info_message: None,
        })
    }

    async fn execute(&self, query: &str) -> Result<u64, DatabaseError> {
        let conn = self.get_conn()?;
        let c = conn.lock().await;

        let rows_affected = c
            .execute(query, &[])
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        Ok(rows_affected as u64)
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
        let result = self
            .query("SELECT datname FROM pg_database WHERE datistemplate = false")
            .await?;
        let databases: Vec<String> = result
            .rows
            .iter()
            .filter_map(|row| {
                row.cells.get(0).map(|cell| match cell {
                    QueryCell::String(s) => s.clone(),
                    _ => cell.to_string(),
                })
            })
            .collect();
        Ok(databases)
    }

    async fn get_tables(&self) -> Result<Vec<TableInfo>, DatabaseError> {
        let conn = self.get_conn()?;
        let c = conn.lock().await;

        let query = "SELECT c.relname as table_name,
                            c.relkind as table_type,
                            pg_catalog.pg_total_relation_size(c.oid) as size,
                            pg_catalog.obj_description(c.oid, 'pg_class') as comment
                     FROM pg_catalog.pg_class c
                     JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = 'public'
                     AND c.relkind IN ('r', 'v')
                     ORDER BY c.relname";

        let rows = c
            .query(query, &[])
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let mut tables = Vec::new();
        for row in rows {
            let name: String = row.get(0);
            let relkind: String = row.get(1);
            let size: Option<i64> = row.get(2);
            let comment: Option<String> = row.get(3);

            let table_type = if relkind == "v" {
                TableType::View
            } else {
                TableType::Table
            };

            tables.push(TableInfo {
                name,
                schema: Some("public".to_string()),
                table_type,
                columns: Vec::new(),
                indexes: Vec::new(),
                foreign_keys: Vec::new(),
                row_count: None,
                size_bytes: size.map(|s| s as u64),
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
        let c = conn.lock().await;

        // Get columns
        let query = "SELECT a.attname as column_name,
                            pg_catalog.format_type(a.atttypid, a.atttypmod) as data_type,
                            NOT a.attnotnull as is_nullable,
                            pg_catalog.pg_get_expr(d.adbin, d.adrelid) as default_value,
                            a.attnum as ordinal_position,
                            co.description as column_comment,
                            CASE WHEN pk.contype = 'p' THEN true ELSE false END as is_primary_key
                     FROM pg_catalog.pg_attribute a
                     JOIN pg_catalog.pg_class c ON c.oid = a.attrelid
                     JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
                     LEFT JOIN pg_catalog.pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
                     LEFT JOIN pg_catalog.pg_description co ON co.objoid = a.attrelid AND co.objsubid = a.attnum
                     LEFT JOIN pg_constraint pk ON pk.conrelid = c.oid AND pk.contype = 'p'
                         AND a.attnum = ANY(pk.conkey)
                     WHERE n.nspname = 'public'
                     AND c.relname = $1
                     AND a.attnum > 0
                     AND NOT a.attisdropped
                     ORDER BY a.attnum";

        let rows = c
            .query(query, &[&table_name])
            .await
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

        let mut columns = Vec::new();
        for row in rows {
            let name: String = row.get(0);
            let data_type: String = row.get(1);
            let is_nullable: bool = row.get(2);
            let default_value: Option<String> = row.get(3);
            let ordinal_position: i16 = row.get(4);
            let comment: Option<String> = row.get(5);
            let is_primary: bool = row.get(6);

            columns.push(ColumnInfo {
                name,
                data_type: self.map_postgres_type(&data_type),
                nullable: is_nullable,
                default_value,
                is_primary_key: is_primary,
                is_unique: false,
                is_auto_increment: false,
                max_length: None,
                numeric_precision: None,
                numeric_scale: None,
                ordinal_position: ordinal_position as u32,
                comment,
                collation: None,
                is_foreign_key: false,
            });
        }

        // Get indexes
        let query = "SELECT i.relname as index_name,
                            a.attname as column_name,
                            ix.indisunique as is_unique,
                            ix.indisprimary as is_primary
                     FROM pg_index ix
                     JOIN pg_class i ON i.oid = ix.indexrelid
                     JOIN pg_class c ON c.oid = ix.indrelid
                     JOIN pg_namespace n ON n.oid = c.relnamespace
                     JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum = ANY(ix.indkey)
                     WHERE n.nspname = 'public'
                     AND c.relname = $1";

        let rows = c
            .query(query, &[&table_name])
            .await
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

        let mut indexes_map: std::collections::HashMap<String, (bool, bool, Vec<String>)> =
            std::collections::HashMap::new();
        for row in rows {
            let name: String = row.get(0);
            let column: String = row.get(1);
            let is_unique: bool = row.get(2);
            let is_primary: bool = row.get(3);

            let entry = indexes_map
                .entry(name)
                .or_insert((is_unique, is_primary, Vec::new()));
            entry.2.push(column);
        }

        let indexes: Vec<IndexInfo> = indexes_map
            .into_iter()
            .map(|(name, (unique, primary, columns))| IndexInfo {
                name,
                columns,
                unique,
                primary,
                index_type: "BTREE".to_string(),
                cardinality: None,
            })
            .collect();

        // Get foreign keys
        let query = "SELECT tc.constraint_name,
                            kcu.column_name,
                            ccu.table_name as referenced_table,
                            ccu.column_name as referenced_column,
                            rc.update_rule,
                            rc.delete_rule
                     FROM information_schema.table_constraints tc
                     JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name
                     JOIN information_schema.constraint_column_usage ccu ON ccu.constraint_name = tc.constraint_name
                     JOIN information_schema.referential_constraints rc ON rc.constraint_name = tc.constraint_name
                     WHERE tc.constraint_type = 'FOREIGN KEY'
                     AND tc.table_name = $1";

        let rows = c
            .query(query, &[&table_name])
            .await
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

        let mut foreign_keys = Vec::new();
        for row in rows {
            let name: String = row.get(0);
            let column: String = row.get(1);
            let ref_table: String = row.get(2);
            let ref_column: String = row.get(3);
            let on_update: String = row.get(4);
            let on_delete: String = row.get(5);

            foreign_keys.push(ForeignKeyInfo {
                name,
                column,
                referenced_table: ref_table,
                referenced_column: ref_column,
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
            schema: Some("public".to_string()),
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
        let c = conn.lock().await;

        let version: String = c
            .query_one("SELECT version()", &[])
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?
            .get(0);

        let row: PostgresRow = c.query_one("SELECT pg_database_size(current_database()),
                                                   (SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public')",
                                   &[]).await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let size: i64 = row.get(0);
        let table_count: i64 = row.get(1);

        Ok(DatabaseStats {
            name: self
                .info
                .as_ref()
                .map(|i| i.database.clone())
                .unwrap_or_else(|| "postgresql".to_string()),
            size_bytes: size as u64,
            table_count: table_count as usize,
            index_count: 0,
            connection_count: None,
            uptime_seconds: None,
            version,
        })
    }

    async fn begin_transaction(&mut self) -> Result<(), DatabaseError> {
        let conn = self.get_conn()?;
        let c = conn.lock().await;
        c.execute("BEGIN", &[])
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;
        Ok(())
    }

    async fn commit(&mut self) -> Result<(), DatabaseError> {
        let conn = self.get_conn()?;
        let c = conn.lock().await;
        c.execute("COMMIT", &[])
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;
        Ok(())
    }

    async fn rollback(&mut self) -> Result<(), DatabaseError> {
        let conn = self.get_conn()?;
        let c = conn.lock().await;
        c.execute("ROLLBACK", &[])
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;
        Ok(())
    }

    async fn get_performance_metrics(&self) -> Result<PerformanceMetrics, DatabaseError> {
        let conn = self.get_conn()?;
        let c = conn.lock().await;

        let row = c
            .query_one(
                "SELECT sum(numbackends) as connections
                              FROM pg_stat_database",
                &[],
            )
            .await;

        let connections: i64 = row.map(|r| r.get(0)).unwrap_or(0);

        Ok(PerformanceMetrics {
            queries_per_second: 0.0,
            active_connections: connections as u32,
            total_connections: connections as u32,
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
        Err(DatabaseError::Unknown(
            "Query cancellation not supported for PostgreSQL".to_string(),
        ))
    }

    async fn ping(&self) -> Result<(), DatabaseError> {
        let conn = self.get_conn()?;
        let c = conn.lock().await;
        c.execute("SELECT 1", &[])
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        Ok(())
    }
}

impl Default for PostgresDriver {
    fn default() -> Self {
        Self::new()
    }
}
