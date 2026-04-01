//! Redis database driver

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use redis::{Client as RedisClient, aio::MultiplexedConnection};

use crate::database_client::{DatabaseDriver, DatabaseType, DatabaseError};
use crate::database_client::drivers::{ConnectionInfo, TableInfo, TableType, TableDetail, ColumnInfo, IndexInfo, ForeignKeyInfo, DatabaseStats};
use crate::database_client::{DatabaseSchema, SchemaTable, QueryResult, QueryRow, QueryCell, PerformanceMetrics};

/// Redis driver
pub struct RedisDriver {
    connection: Option<Arc<Mutex<MultiplexedConnection>>>,
    client: Option<RedisClient>,
    info: Option<ConnectionInfo>,
}

impl RedisDriver {
    pub fn new() -> Self {
        Self {
            connection: None,
            client: None,
            info: None,
        }
    }

    fn get_conn(&self) -> Result<Arc<Mutex<MultiplexedConnection>>, DatabaseError> {
        self.connection
            .as_ref()
            .ok_or_else(|| DatabaseError::ConnectionError("Not connected".to_string()))
            .cloned()
    }

    fn redis_value_to_cell(&self, value: &redis::Value) -> QueryCell {
        match value {
            redis::Value::Nil => QueryCell::Null,
            redis::Value::Int(i) => QueryCell::Integer(*i),
            redis::Value::Data(data) => {
                match String::from_utf8(data.clone()) {
                    Ok(s) => QueryCell::String(s),
                    Err(_) => QueryCell::Blob(data.clone()),
                }
            }
            redis::Value::Bulk(items) => {
                QueryCell::Json(serde_json::json!(items.iter().map(|v| format!("{:?}", v)).collect::<Vec<_>>()))
            }
            redis::Value::Status(s) => QueryCell::String(s.clone()),
            redis::Value::Okay => QueryCell::String("OK".to_string()),
            _ => QueryCell::String(format!("{:?}", value)),
        }
    }
}

#[async_trait]
impl DatabaseDriver for RedisDriver {
    fn db_type(&self) -> DatabaseType {
        DatabaseType::Redis
    }

    async fn connect(&mut self, info: &ConnectionInfo) -> Result<(), DatabaseError> {
        let connection_string = if let Some(ref pwd) = info.password {
            format!("redis://:{}@{}:{}", pwd, info.host, info.port)
        } else {
            format!("redis://{}:{}", info.host, info.port)
        };

        let client = RedisClient::open(connection_string)
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        let conn = client.get_multiplexed_async_connection().await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        self.connection = Some(Arc::new(Mutex::new(conn)));
        self.client = Some(client);
        self.info = Some(info.clone());

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), DatabaseError> {
        self.connection = None;
        self.client = None;
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

        // Parse Redis command
        let parts: Vec<&str> = query.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Err(DatabaseError::QueryError("Empty query".to_string()));
        }

        let cmd = parts[0].to_uppercase();

        // Execute command using redis::cmd
        let result: redis::Value = redis::cmd(&cmd)
            .arg(&parts[1..])
            .query_async(&mut *c)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            columns: vec!["result".to_string()],
            rows: vec![QueryRow { cells: vec![self.redis_value_to_cell(&result)] }],
            execution_time_ms,
            affected_rows: None,
            warning_count: 0,
            info_message: None,
        })
    }

    async fn execute(&self, query: &str) -> Result<u64, DatabaseError> {
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;

        let parts: Vec<&str> = query.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Err(DatabaseError::QueryError("Empty query".to_string()));
        }

        let cmd = parts[0].to_uppercase();

        // Execute command
        let _: redis::Value = redis::cmd(&cmd)
            .arg(&parts[1..])
            .query_async(&mut *c)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        Ok(1)
    }

    async fn get_schema(&self) -> Result<DatabaseSchema, DatabaseError> {
        // Redis doesn't have traditional schema
        Ok(DatabaseSchema {
            database_name: self.info.as_ref().map(|i| i.database.clone()).unwrap_or_default(),
            tables: Vec::new(),
            views: Vec::new(),
            procedures: Vec::new(),
            functions: Vec::new(),
            sequences: Vec::new(),
            enums: Vec::new(),
        })
    }

    async fn list_databases(&self) -> Result<Vec<String>, DatabaseError> {
        // Redis doesn't have multiple databases in the traditional sense
        // But it supports numbered databases (0-15 by default)
        Ok((0..16).map(|i| i.to_string()).collect())
    }

    async fn get_tables(&self) -> Result<Vec<TableInfo>, DatabaseError> {
        // Redis doesn't have tables
        Ok(Vec::new())
    }

    async fn get_table(&self, _table_name: &str) -> Result<TableInfo, DatabaseError> {
        Err(DatabaseError::SchemaError("Redis doesn't have tables".to_string()))
    }

    async fn get_table_info(&self, _table_name: &str) -> Result<TableDetail, DatabaseError> {
        Err(DatabaseError::SchemaError("Redis doesn't have tables".to_string()))
    }

    async fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;

        // Get INFO output
        let info_value: redis::Value = redis::cmd("INFO").query_async(&mut *c).await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let info_str = match info_value {
            redis::Value::Data(d) => String::from_utf8_lossy(&d).to_string(),
            redis::Value::Status(s) => s,
            _ => String::new(),
        };

        let version = info_str.lines()
            .find(|l| l.starts_with("redis_version:"))
            .and_then(|l| l.split(':').nth(1))
            .unwrap_or("unknown")
            .to_string();

        // Get DBSIZE
        let dbsize_value: redis::Value = redis::cmd("DBSIZE").query_async(&mut *c).await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let dbsize = match dbsize_value {
            redis::Value::Int(i) => i as i64,
            redis::Value::Data(d) => String::from_utf8_lossy(&d).parse().unwrap_or(0i64),
            _ => 0i64,
        };

        Ok(DatabaseStats {
            name: self.info.as_ref().map(|i| i.database.clone()).unwrap_or_else(|| "redis".to_string()),
            size_bytes: 0,
            table_count: dbsize as usize,
            index_count: 0,
            connection_count: None,
            uptime_seconds: None,
            version,
        })
    }

    async fn begin_transaction(&mut self) -> Result<(), DatabaseError> {
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;
        redis::cmd("MULTI").query_async(&mut *c).await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))
    }

    async fn commit(&mut self) -> Result<(), DatabaseError> {
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;
        redis::cmd("EXEC").query_async(&mut *c).await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))
    }

    async fn rollback(&mut self) -> Result<(), DatabaseError> {
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;
        redis::cmd("DISCARD").query_async(&mut *c).await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))
    }

    async fn get_performance_metrics(&self) -> Result<PerformanceMetrics, DatabaseError> {
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;

        // Get INFO stats output
        let info_value: redis::Value = redis::cmd("INFO").arg("stats").query_async(&mut *c).await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let info_str = match info_value {
            redis::Value::Data(d) => String::from_utf8_lossy(&d).to_string(),
            redis::Value::Status(s) => s,
            _ => String::new(),
        };

        let mut ops_per_sec = 0.0;

        for line in info_str.lines() {
            if line.starts_with("instantaneous_ops_per_sec:") {
                ops_per_sec = line.split(':').nth(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
            }
        }

        Ok(PerformanceMetrics {
            queries_per_second: ops_per_sec,
            active_connections: 0,
            total_connections: 0,
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
        Err(DatabaseError::Unknown("Query cancellation not supported for Redis".to_string()))
    }

    async fn ping(&self) -> Result<(), DatabaseError> {
        let conn = self.get_conn()?;
        let mut c = conn.lock().await;

        let _: redis::Value = redis::cmd("PING").query_async(&mut *c).await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        Ok(())
    }
}

impl Default for RedisDriver {
    fn default() -> Self {
        Self::new()
    }
}
