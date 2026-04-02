//! MongoDB database driver

use async_trait::async_trait;
use futures::stream::TryStreamExt;
use mongodb::options::ClientOptions;
use mongodb::{
    bson::{Bson, Document},
    Client as MongoClient, Database as MongoDatabase,
};
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

/// MongoDB driver
pub struct MongoDbDriver {
    client: Option<Arc<Mutex<MongoClient>>>,
    database: Option<Arc<Mutex<MongoDatabase>>>,
    info: Option<ConnectionInfo>,
    db_name: String,
}

impl MongoDbDriver {
    pub fn new() -> Self {
        Self {
            client: None,
            database: None,
            info: None,
            db_name: String::new(),
        }
    }

    fn get_db(&self) -> Result<Arc<Mutex<MongoDatabase>>, DatabaseError> {
        self.database
            .as_ref()
            .ok_or_else(|| DatabaseError::ConnectionError("Not connected".to_string()))
            .cloned()
    }

    fn bson_to_query_cell(&self, value: &Bson) -> QueryCell {
        match value {
            Bson::Null => QueryCell::Null,
            Bson::Int32(i) => QueryCell::Integer(*i as i64),
            Bson::Int64(i) => QueryCell::Integer(*i),
            Bson::Double(d) => QueryCell::Float(*d),
            Bson::String(s) => QueryCell::String(s.clone()),
            Bson::Boolean(b) => QueryCell::Boolean(*b),
            Bson::Binary(b) => QueryCell::Blob(b.bytes.to_vec()),
            Bson::DateTime(dt) => QueryCell::DateTime(dt.to_string()),
            Bson::Array(arr) => QueryCell::Json(serde_json::json!(arr)),
            Bson::Document(doc) => QueryCell::Json(serde_json::json!(doc)),
            _ => QueryCell::String(value.to_string()),
        }
    }
}

#[async_trait]
impl DatabaseDriver for MongoDbDriver {
    fn db_type(&self) -> DatabaseType {
        DatabaseType::MongoDB
    }

    async fn connect(&mut self, info: &ConnectionInfo) -> Result<(), DatabaseError> {
        let connection_string = if info.username.is_empty() {
            format!("mongodb://{}:{}/{}", info.host, info.port, info.database)
        } else {
            format!(
                "mongodb://{}:{}@{}:{}/{}?authSource=admin",
                info.username,
                info.password.as_deref().unwrap_or(""),
                info.host,
                info.port,
                info.database
            )
        };

        let client_options = ClientOptions::parse(&connection_string)
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        let client = MongoClient::with_options(client_options)
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        let database = client.database(&info.database);

        // Test connection
        let ping_doc = Document::new();
        database
            .run_command(ping_doc)
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        self.client = Some(Arc::new(Mutex::new(client)));
        self.database = Some(Arc::new(Mutex::new(database)));
        self.info = Some(info.clone());
        self.db_name = info.database.clone();

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), DatabaseError> {
        self.client = None;
        self.database = None;
        self.info = None;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.database.is_some()
    }

    async fn query(&self, sql: &str) -> Result<QueryResult, DatabaseError> {
        self.execute_query(sql).await
    }

    async fn execute_query(&self, query: &str) -> Result<QueryResult, DatabaseError> {
        let start = std::time::Instant::now();

        // Try to interpret as collection name for simple find
        let collection_name = query.trim();

        let db = self.get_db()?;
        let db_lock = db.lock().await;
        let collection = db_lock.collection::<Document>(collection_name);

        let cursor = collection
            .find(Document::new())
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let docs: Vec<Document> = cursor
            .try_collect()
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let mut columns = vec!["_id".to_string()];
        let mut rows = Vec::new();

        for doc in docs {
            let mut cells = Vec::new();
            // First, _id
            if let Some(id) = doc.get("_id") {
                cells.push(self.bson_to_query_cell(id));
            } else {
                cells.push(QueryCell::Null);
            }

            // Other fields
            for (key, value) in doc.iter() {
                if key != "_id" {
                    if !columns.contains(&key.to_string()) {
                        columns.push(key.to_string());
                    }
                    cells.push(self.bson_to_query_cell(value));
                }
            }

            rows.push(QueryRow { cells });
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            columns,
            rows,
            execution_time_ms,
            affected_rows: None,
            warning_count: 0,
            info_message: None,
        })
    }

    async fn execute(&self, _query: &str) -> Result<u64, DatabaseError> {
        Err(DatabaseError::QueryError(
            "MongoDB uses BSON commands, not SQL".to_string(),
        ))
    }

    async fn get_schema(&self) -> Result<DatabaseSchema, DatabaseError> {
        let collections = self.get_tables().await?;
        let mut schema_tables = Vec::new();

        for collection in collections {
            schema_tables.push(SchemaTable {
                name: collection.name,
                schema: None,
                table_type: crate::database_client::schema::SchemaTableType::Table,
                columns: Vec::new(),
                indexes: Vec::new(),
                foreign_keys: Vec::new(),
                row_count: collection.row_count,
                comment: None,
            });
        }

        Ok(DatabaseSchema {
            database_name: self.db_name.clone(),
            tables: schema_tables,
            views: Vec::new(),
            procedures: Vec::new(),
            functions: Vec::new(),
            sequences: Vec::new(),
            enums: Vec::new(),
        })
    }

    async fn list_databases(&self) -> Result<Vec<String>, DatabaseError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DatabaseError::ConnectionError("Not connected".to_string()))?
            .lock()
            .await;

        let databases = client
            .list_database_names()
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        Ok(databases)
    }

    async fn get_tables(&self) -> Result<Vec<TableInfo>, DatabaseError> {
        let db = self.get_db()?;
        let db_lock = db.lock().await;

        let collections = db_lock
            .list_collection_names()
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let mut tables = Vec::new();
        for name in collections {
            let collection = db_lock.collection::<Document>(&name);

            let row_count = collection
                .estimated_document_count()
                .await
                .ok()
                .map(|c| c as u64);

            tables.push(TableInfo {
                name,
                schema: None,
                table_type: TableType::Table,
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
            .ok_or_else(|| {
                DatabaseError::SchemaError(format!("Collection not found: {}", table_name))
            })
    }

    async fn get_table_info(&self, table_name: &str) -> Result<TableDetail, DatabaseError> {
        let db = self.get_db()?;
        let db_lock = db.lock().await;

        // Get indexes
        let collection = db_lock.collection::<Document>(table_name);
        let mut indexes = collection
            .list_indexes()
            .await
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

        let mut index_infos = Vec::new();
        while let Ok(Some(index)) = indexes.try_next().await {
            let name = index
                .keys
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            let keys: Vec<String> = index.keys.keys().cloned().collect();

            index_infos.push(IndexInfo {
                name,
                columns: keys,
                unique: false,
                primary: index.keys.contains_key("_id"),
                index_type: "B-TREE".to_string(),
                cardinality: None,
            });
        }

        // Get a sample document to infer schema
        let sample = collection
            .find_one(Document::new())
            .await
            .map_err(|e| DatabaseError::SchemaError(e.to_string()))?;

        let mut columns = Vec::new();
        if let Some(doc) = sample {
            for (key, value) in doc {
                let data_type = match value {
                    Bson::Int32(_) => "INTEGER",
                    Bson::Int64(_) => "BIGINT",
                    Bson::Double(_) => "DOUBLE",
                    Bson::String(_) => "STRING",
                    Bson::Boolean(_) => "BOOLEAN",
                    Bson::DateTime(_) => "DATETIME",
                    Bson::Array(_) => "ARRAY",
                    Bson::Document(_) => "OBJECT",
                    Bson::ObjectId(_) => "OBJECT_ID",
                    _ => "UNKNOWN",
                };

                columns.push(ColumnInfo {
                    name: key,
                    data_type: data_type.to_string(),
                    nullable: true,
                    default_value: None,
                    is_primary_key: false,
                    is_unique: false,
                    is_auto_increment: false,
                    max_length: None,
                    numeric_precision: None,
                    numeric_scale: None,
                    ordinal_position: columns.len() as u32 + 1,
                    comment: None,
                    collation: None,
                    is_foreign_key: false,
                });
            }
        }

        let info = TableInfo {
            name: table_name.to_string(),
            schema: None,
            table_type: TableType::Table,
            columns: columns.clone(),
            indexes: index_infos.clone(),
            foreign_keys: Vec::new(),
            row_count: None,
            size_bytes: None,
            created_at: None,
            updated_at: None,
            comment: None,
        };

        Ok(TableDetail {
            info,
            columns,
            indexes: index_infos,
            foreign_keys: Vec::new(),
            constraints: Vec::new(),
            triggers: Vec::new(),
            privileges: Vec::new(),
        })
    }

    async fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        let db = self.get_db()?;
        let db_lock = db.lock().await;

        let collections = db_lock
            .list_collection_names()
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let ping_doc = Document::new();
        let build_info = db_lock
            .run_command(ping_doc)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let version = build_info
            .get_str("version")
            .unwrap_or("unknown")
            .to_string();

        Ok(DatabaseStats {
            name: self.db_name.clone(),
            size_bytes: 0,
            table_count: collections.len(),
            index_count: 0,
            connection_count: None,
            uptime_seconds: None,
            version,
        })
    }

    async fn begin_transaction(&mut self) -> Result<(), DatabaseError> {
        Ok(())
    }

    async fn commit(&mut self) -> Result<(), DatabaseError> {
        Ok(())
    }

    async fn rollback(&mut self) -> Result<(), DatabaseError> {
        Ok(())
    }

    async fn get_performance_metrics(&self) -> Result<PerformanceMetrics, DatabaseError> {
        let db = self.get_db()?;
        let db_lock = db.lock().await;

        let server_status = db_lock
            .run_command(Document::new())
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        let connections = 0u32;

        Ok(PerformanceMetrics {
            queries_per_second: 0.0,
            active_connections: connections,
            total_connections: connections,
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
            "Query cancellation not supported for MongoDB".to_string(),
        ))
    }

    async fn ping(&self) -> Result<(), DatabaseError> {
        let db = self.get_db()?;
        let db_lock = db.lock().await;

        let ping_cmd = Document::new();
        db_lock
            .run_command(ping_cmd)
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        Ok(())
    }
}

impl Default for MongoDbDriver {
    fn default() -> Self {
        Self::new()
    }
}
