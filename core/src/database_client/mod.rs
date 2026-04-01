//! EasySSH Database Client Module
//!
//! Comprehensive database management supporting:
//! - MySQL, PostgreSQL, MongoDB, Redis, SQLite connections
//! - Query editor with syntax highlighting
//! - Table data visualization and editing
//! - ER diagram generation
//! - Data import/export (CSV, JSON, SQL)
//! - Query history management
//! - SSH tunnel support
//! - Performance analysis
//! - Backup and restore

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use thiserror::Error;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub mod connection;
pub mod query;
pub mod schema;
pub mod erdiagram;
pub mod import_export;
pub mod history;
pub mod tunnel;
pub mod performance;
pub mod backup;
pub mod drivers;
pub mod editor;
pub mod pool;
pub mod cache;
pub mod batch;

pub use connection::*;
pub use query::*;
pub use schema::*;
pub use erdiagram::*;
pub use import_export::*;
pub use history::*;
pub use tunnel::*;
pub use performance::*;
pub use backup::*;
pub use drivers::*;
pub use editor::*;
pub use pool::*;
pub use cache::*;
pub use batch::*;

/// Database client error types
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    ConnectionError(String),
    #[error("Query execution failed: {0}")]
    QueryError(String),
    #[error("Schema analysis failed: {0}")]
    SchemaError(String),
    #[error("Import/Export failed: {0}")]
    ImportExportError(String),
    #[error("SSH tunnel error: {0}")]
    TunnelError(String),
    #[error("Backup/Restore error: {0}")]
    BackupError(String),
    #[error("Driver not found: {0}")]
    DriverNotFound(String),
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),
    #[error("Timeout: {0}")]
    TimeoutError(String),
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
    #[error("Operation cancelled")]
    Cancelled,
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Supported database types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum DatabaseType {
    MySQL,
    PostgreSQL,
    MongoDB,
    Redis,
    SQLite,
}

impl DatabaseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseType::MySQL => "mysql",
            DatabaseType::PostgreSQL => "postgresql",
            DatabaseType::MongoDB => "mongodb",
            DatabaseType::Redis => "redis",
            DatabaseType::SQLite => "sqlite",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DatabaseType::MySQL => "MySQL",
            DatabaseType::PostgreSQL => "PostgreSQL",
            DatabaseType::MongoDB => "MongoDB",
            DatabaseType::Redis => "Redis",
            DatabaseType::SQLite => "SQLite",
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            DatabaseType::MySQL => 3306,
            DatabaseType::PostgreSQL => 5432,
            DatabaseType::MongoDB => 27017,
            DatabaseType::Redis => 6379,
            DatabaseType::SQLite => 0, // File-based
        }
    }

    pub fn supports_sql(&self) -> bool {
        matches!(self, DatabaseType::MySQL | DatabaseType::PostgreSQL | DatabaseType::SQLite)
    }

    pub fn supports_nosql(&self) -> bool {
        matches!(self, DatabaseType::MongoDB | DatabaseType::Redis)
    }
}

impl std::str::FromStr for DatabaseType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mysql" => Ok(DatabaseType::MySQL),
            "postgresql" | "postgres" => Ok(DatabaseType::PostgreSQL),
            "mongodb" | "mongo" => Ok(DatabaseType::MongoDB),
            "redis" => Ok(DatabaseType::Redis),
            "sqlite" => Ok(DatabaseType::SQLite),
            _ => Err(format!("Unknown database type: {}", s)),
        }
    }
}

/// Database connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub id: String,
    pub name: String,
    pub db_type: DatabaseType,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: Option<String>,
    pub use_keychain: bool,
    pub keychain_account: Option<String>,
    pub ssl_mode: SslMode,
    pub ssl_cert_path: Option<String>,
    pub ssl_key_path: Option<String>,
    pub ssl_ca_path: Option<String>,
    pub connection_timeout_secs: u64,
    pub query_timeout_secs: u64,
    pub max_connections: u32,
    pub ssh_tunnel: Option<SshTunnelConfig>,
    pub advanced_options: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub color: Option<String>,
    pub group_id: Option<String>,
}

impl DatabaseConfig {
    pub fn new(name: String, db_type: DatabaseType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            db_type,
            host: "localhost".to_string(),
            port: db_type.default_port(),
            database: String::new(),
            username: String::new(),
            password: None,
            use_keychain: true,
            keychain_account: None,
            ssl_mode: SslMode::Preferred,
            ssl_cert_path: None,
            ssl_key_path: None,
            ssl_ca_path: None,
            connection_timeout_secs: 30,
            query_timeout_secs: 300,
            max_connections: 10,
            ssh_tunnel: None,
            advanced_options: HashMap::new(),
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            color: None,
            group_id: None,
        }
    }

    pub fn with_host(mut self, host: String) -> Self {
        self.host = host;
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_database(mut self, database: String) -> Self {
        self.database = database;
        self
    }

    pub fn with_credentials(mut self, username: String, password: String) -> Self {
        self.username = username;
        self.password = Some(password);
        self
    }

    pub fn with_ssh_tunnel(mut self, tunnel: SshTunnelConfig) -> Self {
        self.ssh_tunnel = Some(tunnel);
        self
    }

    pub fn connection_string(&self) -> String {
        match self.db_type {
            DatabaseType::MySQL => {
                format!(
                    "mysql://{}:{}@{}:{}/{}",
                    self.username,
                    self.password.as_deref().unwrap_or(""),
                    self.host,
                    self.port,
                    self.database
                )
            }
            DatabaseType::PostgreSQL => {
                format!(
                    "postgresql://{}:{}@{}:{}/{}",
                    self.username,
                    self.password.as_deref().unwrap_or(""),
                    self.host,
                    self.port,
                    self.database
                )
            }
            DatabaseType::MongoDB => {
                format!(
                    "mongodb://{}:{}@{}:{}/{}",
                    self.username,
                    self.password.as_deref().unwrap_or(""),
                    self.host,
                    self.port,
                    self.database
                )
            }
            DatabaseType::Redis => {
                if let Some(ref pwd) = self.password {
                    format!("redis://:{}@{}:{}", pwd, self.host, self.port)
                } else {
                    format!("redis://{}:{}", self.host, self.port)
                }
            }
            DatabaseType::SQLite => {
                format!("sqlite://{}", self.database)
            }
        }
    }

    /// Convert to ConnectionInfo for driver connection
    pub fn to_connection_info(&self) -> ConnectionInfo {
        ConnectionInfo {
            host: self.host.clone(),
            port: self.port,
            database: self.database.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            ssl_mode: self.ssl_mode.to_driver_ssl_mode(),
            connection_timeout: self.connection_timeout_secs,
            query_timeout: self.query_timeout_secs,
        }
    }
}

/// SSL connection modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SslMode {
    Disabled,
    Preferred,
    Required,
    VerifyCa,
    VerifyIdentity,
}

impl SslMode {
    /// Convert to driver's SslMode
    fn to_driver_ssl_mode(&self) -> drivers::SslMode {
        match self {
            SslMode::Disabled => drivers::SslMode::Disabled,
            SslMode::Preferred => drivers::SslMode::Preferred,
            SslMode::Required => drivers::SslMode::Required,
            SslMode::VerifyCa => drivers::SslMode::VerifyCa,
            SslMode::VerifyIdentity => drivers::SslMode::VerifyIdentity,
        }
    }
}

/// SSH tunnel configuration for database connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshTunnelConfig {
    pub ssh_server_id: String,
    pub local_bind_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

/// Database connection pool entry
pub struct ConnectionPoolEntry {
    pub config: DatabaseConfig,
    pub driver: Arc<Mutex<Box<dyn DatabaseDriver>>>,
    pub connected_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub connection_count: u32,
}

impl std::fmt::Debug for ConnectionPoolEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionPoolEntry")
            .field("config", &self.config)
            .field("connected_at", &self.connected_at)
            .field("last_used", &self.last_used)
            .field("connection_count", &self.connection_count)
            .finish()
    }
}

/// Main database client manager
pub struct DatabaseClientManager {
    connections: Arc<RwLock<HashMap<String, ConnectionPoolEntry>>>,
    query_history: Arc<RwLock<QueryHistoryManager>>,
    tunnel_manager: Arc<RwLock<TunnelManager>>,
    schema_cache: Arc<RwLock<HashMap<String, SchemaCache>>>,
}

impl DatabaseClientManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            query_history: Arc::new(RwLock::new(QueryHistoryManager::new())),
            tunnel_manager: Arc::new(RwLock::new(TunnelManager::new())),
            schema_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connect to a database
    pub async fn connect(&self, config: DatabaseConfig) -> Result<String, DatabaseError> {
        let driver = create_driver(&config.db_type).await?;

        // TODO: Setup SSH tunnel if configured
        // For now, we skip tunnel setup to avoid type mismatches
        let effective_config = config.clone();

        // Convert to ConnectionInfo and connect using driver
        let conn_info = effective_config.to_connection_info();
        driver.lock().await.connect(&conn_info).await?;

        let connection_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let entry = ConnectionPoolEntry {
            config,
            driver,
            connected_at: now,
            last_used: now,
            connection_count: 1,
        };

        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), entry);

        Ok(connection_id)
    }

    /// Disconnect from a database
    pub async fn disconnect(&self, connection_id: &str) -> Result<(), DatabaseError> {
        let mut connections = self.connections.write().await;

        if let Some(entry) = connections.remove(connection_id) {
            // Close SSH tunnel if exists
            if let Some(ref tunnel) = entry.config.ssh_tunnel {
                let tunnel_mgr = self.tunnel_manager.write().await;
                tunnel_mgr.close_tunnel(&tunnel.ssh_server_id).await?;
            }

            // Close driver connection
            entry.driver.lock().await.disconnect().await?;
        }

        Ok(())
    }

    /// Execute a query
    pub async fn execute_query(
        &self,
        connection_id: &str,
        query: &str,
    ) -> Result<QueryResult, DatabaseError> {
        let connections = self.connections.read().await;
        let entry = connections
            .get(connection_id)
            .ok_or_else(|| DatabaseError::ConnectionError("Connection not found".to_string()))?;

        let result = entry.driver.lock().await.execute_query(query).await?;

        // Record in history
        let history_entry = QueryHistoryEntry::new(
            connection_id.to_string(),
            entry.config.db_type,
            query.to_string(),
            result.execution_time_ms,
            result.rows.len(),
        );

        let mut history = self.query_history.write().await;
        history.add_entry(history_entry).await?;

        Ok(result)
    }

    /// Get schema information
    pub async fn get_schema(&self, connection_id: &str) -> Result<DatabaseSchema, DatabaseError> {
        // Check cache first
        {
            let cache = self.schema_cache.read().await;
            if let Some(cached) = cache.get(connection_id) {
                if cached.is_fresh() {
                    return Ok(cached.schema.clone());
                }
            }
        }

        // Fetch from database
        let connections = self.connections.read().await;
        let entry = connections
            .get(connection_id)
            .ok_or_else(|| DatabaseError::ConnectionError("Connection not found".to_string()))?;

        let schema = entry.driver.lock().await.get_schema().await?;

        // Update cache
        let mut cache = self.schema_cache.write().await;
        cache.insert(
            connection_id.to_string(),
            SchemaCache::new(schema.clone()),
        );

        Ok(schema)
    }

    /// Get list of active connections
    pub async fn list_connections(&self) -> Vec<ConnectionSummary> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .map(|(id, entry)| ConnectionSummary {
                id: id.clone(),
                name: entry.config.name.clone(),
                db_type: entry.config.db_type,
                host: entry.config.host.clone(),
                database: entry.config.database.clone(),
                connected_at: entry.connected_at,
                last_used: entry.last_used,
                connection_count: entry.connection_count,
            })
            .collect()
    }

    /// Refresh schema cache
    pub async fn refresh_schema(&self, connection_id: &str) -> Result<(), DatabaseError> {
        let connections = self.connections.read().await;
        let entry = connections
            .get(connection_id)
            .ok_or_else(|| DatabaseError::ConnectionError("Connection not found".to_string()))?;

        let schema = entry.driver.lock().await.get_schema().await?;

        let mut cache = self.schema_cache.write().await;
        cache.insert(
            connection_id.to_string(),
            SchemaCache::new(schema),
        );

        Ok(())
    }

    /// Get query history
    pub async fn get_query_history(
        &self,
        connection_id: Option<&str>,
        limit: usize,
    ) -> Vec<QueryHistoryEntry> {
        let history = self.query_history.read().await;
        history.get_entries(connection_id, limit).await
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(
        &self,
        connection_id: &str,
    ) -> Result<PerformanceMetrics, DatabaseError> {
        let driver = {
            let connections = self.connections.read().await;
            let entry = connections
                .get(connection_id)
                .ok_or_else(|| DatabaseError::ConnectionError("Connection not found".to_string()))?;
            entry.driver.clone()
        };

        let mut driver_lock = driver.lock().await;
        driver_lock.get_performance_metrics().await
    }
}

impl Default for DatabaseClientManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection summary for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSummary {
    pub id: String,
    pub name: String,
    pub db_type: DatabaseType,
    pub host: String,
    pub database: String,
    pub connected_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub connection_count: u32,
}

/// Schema cache entry
#[derive(Debug, Clone)]
struct SchemaCache {
    schema: DatabaseSchema,
    cached_at: DateTime<Utc>,
}

impl SchemaCache {
    fn new(schema: DatabaseSchema) -> Self {
        Self {
            schema,
            cached_at: Utc::now(),
        }
    }

    fn is_fresh(&self) -> bool {
        Utc::now().signed_duration_since(self.cached_at).num_minutes() < 5
    }
}

/// Create appropriate driver for database type
async fn create_driver(db_type: &DatabaseType) -> Result<Arc<Mutex<Box<dyn DatabaseDriver>>>, DatabaseError> {
    match db_type {
        DatabaseType::MySQL => {
            #[cfg(feature = "mysql-driver")]
            {
                Ok(Arc::new(Mutex::new(Box::new(
                    drivers::mysql::MySqlDriver::new(),
                ))))
            }
            #[cfg(not(feature = "mysql-driver"))]
            Err(DatabaseError::DriverNotFound("MySQL driver not enabled".to_string()))
        }
        DatabaseType::PostgreSQL => {
            #[cfg(feature = "postgres-driver")]
            {
                Ok(Arc::new(Mutex::new(Box::new(
                    drivers::postgres::PostgresDriver::new(),
                ))))
            }
            #[cfg(not(feature = "postgres-driver"))]
            Err(DatabaseError::DriverNotFound("PostgreSQL driver not enabled".to_string()))
        }
        DatabaseType::MongoDB => {
            #[cfg(feature = "mongodb-driver")]
            {
                Ok(Arc::new(Mutex::new(Box::new(
                    drivers::mongodb::MongoDbDriver::new(),
                ))))
            }
            #[cfg(not(feature = "mongodb-driver"))]
            Err(DatabaseError::DriverNotFound("MongoDB driver not enabled".to_string()))
        }
        DatabaseType::Redis => {
            #[cfg(feature = "redis-driver")]
            {
                Ok(Arc::new(Mutex::new(Box::new(
                    drivers::redis::RedisDriver::new(),
                ))))
            }
            #[cfg(not(feature = "redis-driver"))]
            Err(DatabaseError::DriverNotFound("Redis driver not enabled".to_string()))
        }
        DatabaseType::SQLite => {
            Ok(Arc::new(Mutex::new(Box::new(
                drivers::sqlite::SqliteDriver::new(),
            ))))
        }
    }
}
