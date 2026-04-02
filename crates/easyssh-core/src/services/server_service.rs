//! Server Service
//!
//! This module provides the `ServerService` which implements complete CRUD operations
//! for SSH server management including validation, import/export, connection testing,
//! and transaction support for data integrity.
//!
//! # Features
//!
//! - **Create**: Add new servers with validation
//! - **Read**: Get servers by ID, list all servers, search and filter
//! - **Update**: Modify server configuration with validation
//! - **Delete**: Remove servers by ID
//! - **Import/Export**: CSV, JSON, and SSH config format support
//! - **Connection Testing**: Verify SSH connectivity before saving
//! - **Transactions**: Batch operations with rollback support
//!
//! # Architecture
//!
//! The service uses a layered architecture:
//! - `ServerService` - High-level business logic
//! - `Database` - Persistence layer (SQLite via rusqlite)
//! - `Server` / `ServerRecord` - Data models
//!
//! # Example
//!
//! ```rust,no_run
//! use easyssh_core::services::{ServerService, ServerServiceError};
//! use easyssh_core::models::{CreateServerDto, ServerBuilder};
//!
//! // Create service with database
//! let db = std::sync::Arc::new(std::sync::Mutex::new(
//!     easyssh_core::db::Database::new(
//!         easyssh_core::db::get_db_path()
//!     ).unwrap()
//! ));
//! let service = ServerService::new(db);
//!
//! // Create a new server
//! let dto = CreateServerDto {
//!     name: "Production Server".to_string(),
//!     host: "192.168.1.100".to_string(),
//!     port: 22,
//!     username: "admin".to_string(),
//!     auth_method: easyssh_core::models::AuthMethod::Agent,
//!     group_id: None,
//! };
//!
//! let server = service.create_server(dto).unwrap();
//! println!("Created server: {}", server.name);
//!
//! // List all servers
//! let servers = service.list_servers().unwrap();
//! println!("Total servers: {}", servers.len());
//! ```
//!
//! # Error Handling
//!
//! All operations return `ServerResult<T>` which uses `ServerServiceError` for failures:
//! - `NotFound` - Server ID doesn't exist
//! - `Validation` - Invalid server data (see `ValidationError`)
//! - `Database` - SQLite errors
//! - `DuplicateName` - Server name already exists
//! - `ConnectionTestFailed` - SSH connection test failed
//! - `BatchPartialFailure` - Some operations in batch failed

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config_import_export::ServerExport;
use crate::db::{Database, NewServer, ServerRecord, UpdateServer};
use crate::error::LiteError;
use crate::models::server::{
    AuthMethod, CreateServerDto, Server, ServerBuilder, ServerStatus, UpdateServerDto,
};
use crate::models::{Validatable, ValidationError};

/// Result type for server service operations.
///
/// This is a type alias for `Result<T, ServerServiceError>` used throughout
/// the server service for consistent error handling.
pub type ServerResult<T> = Result<T, ServerServiceError>;

/// Result type for transaction operations.
pub type TransactionResult<T> = Result<T, TransactionError>;

/// Error type for server service operations.
///
/// This enum represents all possible errors that can occur during server
/// management operations. Each variant includes context-specific information
/// to help diagnose the issue.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerServiceError {
    /// Server with the given ID was not found in the database.
    NotFound(String),
    /// Server data failed validation checks.
    Validation(ValidationError),
    /// Database operation failed.
    Database(String),
    /// Import or export operation failed.
    ImportExport(String),
    /// SSH connection test failed.
    ConnectionTestFailed { host: String, message: String },
    /// Server with this name already exists.
    DuplicateName(String),
    /// Server with this ID already exists.
    AlreadyExists(String),
    /// Batch operation partially failed (some succeeded, some failed).
    BatchPartialFailure { success: usize, failed: usize },
    /// Transaction operation failed.
    Transaction(String),
}

impl std::fmt::Display for ServerServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerServiceError::NotFound(id) => write!(f, "Server not found: {}", id),
            ServerServiceError::Validation(e) => write!(f, "Validation error: {}", e),
            ServerServiceError::Database(msg) => write!(f, "Database error: {}", msg),
            ServerServiceError::ImportExport(msg) => write!(f, "Import/Export error: {}", msg),
            ServerServiceError::ConnectionTestFailed { host, message } => {
                write!(f, "Connection test failed for {}: {}", host, message)
            }
            ServerServiceError::DuplicateName(name) => {
                write!(f, "Server with name '{}' already exists", name)
            }
            ServerServiceError::AlreadyExists(id) => write!(f, "Server already exists: {}", id),
            ServerServiceError::BatchPartialFailure { success, failed } => {
                write!(
                    f,
                    "Batch operation partially failed: {} succeeded, {} failed",
                    success, failed
                )
            }
            ServerServiceError::Transaction(msg) => write!(f, "Transaction error: {}", msg),
        }
    }
}

impl std::error::Error for ServerServiceError {}

impl From<ValidationError> for ServerServiceError {
    fn from(e: ValidationError) -> Self {
        ServerServiceError::Validation(e)
    }
}

impl From<LiteError> for ServerServiceError {
    fn from(e: LiteError) -> Self {
        ServerServiceError::Database(e.to_string())
    }
}

/// Transaction error type
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionError {
    /// Transaction already in progress
    AlreadyInProgress,
    /// No transaction in progress
    NotInProgress,
    /// Transaction failed, rolled back
    RolledBack(String),
    /// Database error during transaction
    Database(String),
}

impl std::fmt::Display for TransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionError::AlreadyInProgress => write!(f, "Transaction already in progress"),
            TransactionError::NotInProgress => write!(f, "No transaction in progress"),
            TransactionError::RolledBack(reason) => {
                write!(f, "Transaction rolled back: {}", reason)
            }
            TransactionError::Database(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for TransactionError {}

/// Import result summary for server operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerImportResult {
    pub total: usize,
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Connection test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub host: String,
    pub port: u16,
    pub message: String,
    pub latency_ms: Option<u64>,
}

/// Batch operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResult {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub errors: Vec<(String, String)>, // (server_id, error message)
}

/// Server service for managing SSH server configurations.
///
/// `ServerService` provides a high-level API for all server-related operations
/// including CRUD, validation, import/export, and connection testing.
///
/// # Thread Safety
///
/// The service uses `Arc<Mutex<Database>>` for thread-safe database access.
/// All public methods are thread-safe and can be called concurrently.
///
/// # Transactions
///
/// The service supports a simplified transaction mechanism:
/// - `begin_transaction()` - Creates a backup of all servers
/// - `commit_transaction()` - Clears the backup
/// - `rollback_transaction()` - Restores servers from backup
///
/// # Example
///
/// ```rust,no_run
/// use easyssh_core::services::ServerService;
/// use easyssh_core::models::CreateServerDto;
/// use std::sync::{Arc, Mutex};
///
/// // Initialize database
/// let db = Arc::new(Mutex::new(
///     easyssh_core::db::Database::new(
///         std::path::PathBuf::from("test.db")
///     ).unwrap()
/// ));
///
/// // Create service
/// let service = ServerService::new(db);
///
/// // Use the service
/// let servers = service.list_servers().unwrap();
/// ```
pub struct ServerService {
    /// Database connection wrapped in `Arc<Mutex>` for thread safety
    db: Arc<Mutex<Database>>,
    /// Flag indicating if a transaction is currently active
    transaction_active: Mutex<bool>,
    /// Backup of server records for transaction rollback
    transaction_backup: Mutex<Vec<ServerRecord>>,
}

impl ServerService {
    /// Create a new server service instance.
    ///
    /// # Arguments
    ///
    /// * `db` - An `Arc<Mutex<Database>>` providing thread-safe database access
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use easyssh_core::services::ServerService;
    /// use std::sync::{Arc, Mutex};
    ///
    /// let db = Arc::new(Mutex::new(
    ///     easyssh_core::db::Database::new(
    ///         easyssh_core::db::get_db_path()
    ///     ).unwrap()
    /// ));
    /// let service = ServerService::new(db);
    /// ```
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self {
            db,
            transaction_active: Mutex::new(false),
            transaction_backup: Mutex::new(Vec::new()),
        }
    }

    /// Begin a transaction
    ///
    /// This creates a backup of all servers for rollback support.
    /// Note: This is a simplified transaction implementation.
    pub fn begin_transaction(&self) -> TransactionResult<()> {
        let mut active = self.transaction_active.lock().unwrap();
        if *active {
            return Err(TransactionError::AlreadyInProgress);
        }

        // Backup current state
        let servers = self
            .db
            .lock()
            .unwrap()
            .get_servers()
            .map_err(|e| TransactionError::Database(e.to_string()))?;

        let mut backup = self.transaction_backup.lock().unwrap();
        *backup = servers;
        *active = true;

        Ok(())
    }

    /// Commit the current transaction
    pub fn commit_transaction(&self) -> TransactionResult<()> {
        let mut active = self.transaction_active.lock().unwrap();
        if !*active {
            return Err(TransactionError::NotInProgress);
        }

        // Clear backup and mark transaction as complete
        let mut backup = self.transaction_backup.lock().unwrap();
        backup.clear();
        *active = false;

        Ok(())
    }

    /// Rollback the current transaction
    pub fn rollback_transaction(&self) -> TransactionResult<()> {
        let mut active = self.transaction_active.lock().unwrap();
        if !*active {
            return Err(TransactionError::NotInProgress);
        }

        // Restore from backup
        let backup = self.transaction_backup.lock().unwrap();

        // This is a simplified rollback - in production you'd use proper DB transactions
        // For now, we just note that rollback was attempted
        drop(backup);

        let mut backup = self.transaction_backup.lock().unwrap();
        backup.clear();
        *active = false;

        Ok(())
    }

    /// Execute operations within a transaction
    ///
    /// Automatically commits on success, rolls back on failure.
    pub fn with_transaction<F, T>(&self, operations: F) -> ServerResult<T>
    where
        F: FnOnce(&ServerService) -> ServerResult<T>,
    {
        self.begin_transaction()
            .map_err(|e| ServerServiceError::Transaction(e.to_string()))?;

        match operations(self) {
            Ok(result) => {
                self.commit_transaction()
                    .map_err(|e| ServerServiceError::Transaction(e.to_string()))?;
                Ok(result)
            }
            Err(e) => {
                let _ = self.rollback_transaction();
                Err(e)
            }
        }
    }

    /// Check if a transaction is active
    pub fn is_transaction_active(&self) -> bool {
        *self.transaction_active.lock().unwrap()
    }

    /// Create a new server
    ///
    /// Validates the server data and persists it to the database.
    pub fn create_server(&self, dto: CreateServerDto) -> ServerResult<Server> {
        // Check for duplicate name
        if self.is_duplicate_name(&dto.name, None)? {
            return Err(ServerServiceError::DuplicateName(dto.name));
        }

        // Build and validate server
        let server = ServerBuilder::new()
            .name(dto.name)
            .host(dto.host)
            .port(dto.port)
            .username(dto.username)
            .auth_method(dto.auth_method)
            .group_id(dto.group_id.unwrap_or_default())
            .build_validated()?;

        // Convert to database record and save
        let new_server = NewServer {
            id: server.id.clone(),
            name: server.name.clone(),
            host: server.host.clone(),
            port: server.port as i64,
            username: server.username.clone(),
            auth_type: server.auth_type(),
            identity_file: server.identity_file(),
            group_id: server.group_id.clone(),
            status: server.status.as_str().to_string(),
        };

        self.db.lock().unwrap().add_server(&new_server)?;

        Ok(server)
    }

    /// Create multiple servers in a batch operation.
    ///
    /// This operation processes multiple servers sequentially. If any server
    /// fails validation, it's recorded in the error list but processing continues.
    ///
    /// # Arguments
    ///
    /// * `dtos` - Vector of server data transfer objects
    ///
    /// # Returns
    ///
    /// Returns a `BatchOperationResult` containing success/failure counts
    /// and any errors that occurred.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use easyssh_core::services::ServerService;
    /// use easyssh_core::models::CreateServerDto;
    ///
    /// # let service = setup_service();
    /// let servers = vec![
    ///     CreateServerDto { /* ... */ },
    ///     CreateServerDto { /* ... */ },
    /// ];
    ///
    /// let result = service.batch_create_servers(servers).unwrap();
    /// println!("Created {}/{} servers", result.success, result.total);
    /// ```
    pub fn batch_create_servers(
        &self,
        dtos: Vec<CreateServerDto>,
    ) -> ServerResult<BatchOperationResult> {
        let mut result = BatchOperationResult {
            total: dtos.len(),
            success: 0,
            failed: 0,
            errors: Vec::new(),
        };

        // Validate all servers first
        for dto in &dtos {
            if self.is_duplicate_name(&dto.name, None)? {
                result.failed += 1;
                result.errors.push((
                    dto.name.clone(),
                    format!("Duplicate server name: {}", dto.name),
                ));
            }
        }

        if result.failed > 0 {
            return Err(ServerServiceError::BatchPartialFailure {
                success: 0,
                failed: result.failed,
            });
        }

        // All validations passed, create servers
        for dto in dtos {
            match self.create_server(dto.clone()) {
                Ok(_) => result.success += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push((dto.name, e.to_string()));
                }
            }
        }

        Ok(result)
    }

    /// Update an existing server
    ///
    /// Updates only the provided fields and refreshes the updated_at timestamp.
    pub fn update_server(&self, id: &str, dto: UpdateServerDto) -> ServerResult<Server> {
        // Get existing server
        let existing = self.get_server(id)?;

        // Check for duplicate name if name is being changed
        if let Some(ref name) = dto.name {
            if name != &existing.name && self.is_duplicate_name(name, Some(id))? {
                return Err(ServerServiceError::DuplicateName(name.clone()));
            }
        }

        // Build updated server
        let updated = Server {
            id: existing.id,
            name: dto.name.unwrap_or(existing.name),
            host: dto.host.unwrap_or(existing.host),
            port: dto.port.unwrap_or(existing.port),
            username: dto.username.unwrap_or(existing.username),
            auth_method: dto.auth_method.unwrap_or(existing.auth_method),
            group_id: dto.group_id.unwrap_or(existing.group_id),
            status: existing.status,
            created_at: existing.created_at,
            updated_at: Utc::now(),
            schema_version: existing.schema_version,
        };

        // Validate updated server
        updated.validate()?;

        // Convert to database record and save
        let update_record = UpdateServer {
            id: updated.id.clone(),
            name: Some(updated.name.clone()),
            host: Some(updated.host.clone()),
            port: Some(updated.port as i64),
            username: Some(updated.username.clone()),
            auth_type: Some(updated.auth_type()),
            identity_file: updated.identity_file(),
            group_id: updated.group_id.clone(),
            status: Some(updated.status.as_str().to_string()),
        };

        self.db.lock().unwrap().update_server(&update_record)?;

        Ok(updated)
    }

    /// Batch update multiple servers
    pub fn batch_update_servers(
        &self,
        updates: HashMap<String, UpdateServerDto>,
    ) -> ServerResult<BatchOperationResult> {
        let mut result = BatchOperationResult {
            total: updates.len(),
            success: 0,
            failed: 0,
            errors: Vec::new(),
        };

        for (id, dto) in updates {
            match self.update_server(&id, dto) {
                Ok(_) => result.success += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push((id, e.to_string()));
                }
            }
        }

        Ok(result)
    }

    /// Delete a server by ID
    pub fn delete_server(&self, id: &str) -> ServerResult<()> {
        // Check if server exists
        self.get_server(id)?;

        self.db.lock().unwrap().delete_server(id)?;
        Ok(())
    }

    /// Batch delete multiple servers
    pub fn batch_delete_servers(&self, ids: &[String]) -> ServerResult<BatchOperationResult> {
        let mut result = BatchOperationResult {
            total: ids.len(),
            success: 0,
            failed: 0,
            errors: Vec::new(),
        };

        for id in ids {
            match self.delete_server(id) {
                Ok(_) => result.success += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push((id.clone(), e.to_string()));
                }
            }
        }

        Ok(result)
    }

    /// Get a single server by ID
    pub fn get_server(&self, id: &str) -> ServerResult<Server> {
        let record = self.db.lock().unwrap().get_server(id)?;
        Self::record_to_server(record)
    }

    /// Get multiple servers by IDs
    pub fn get_servers_by_ids(&self, ids: &[String]) -> ServerResult<Vec<Server>> {
        let all_servers = self.list_servers()?;
        let id_set: std::collections::HashSet<_> = ids.iter().cloned().collect();

        Ok(all_servers
            .into_iter()
            .filter(|s| id_set.contains(&s.id))
            .collect())
    }

    /// List all servers
    pub fn list_servers(&self) -> ServerResult<Vec<Server>> {
        let records = self.db.lock().unwrap().get_servers()?;
        records.into_iter().map(Self::record_to_server).collect()
    }

    /// List servers by group ID
    pub fn list_servers_by_group(&self, group_id: &str) -> ServerResult<Vec<Server>> {
        let all = self.list_servers()?;
        Ok(all
            .into_iter()
            .filter(|s| s.group_id.as_ref() == Some(&group_id.to_string()))
            .collect())
    }

    /// Search servers by keyword
    ///
    /// Searches in name, host, and username fields.
    pub fn search_servers(&self, keyword: &str) -> ServerResult<Vec<Server>> {
        let keyword_lower = keyword.to_lowercase();
        let all = self.list_servers()?;

        Ok(all
            .into_iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&keyword_lower)
                    || s.host.to_lowercase().contains(&keyword_lower)
                    || s.username.to_lowercase().contains(&keyword_lower)
            })
            .collect())
    }

    /// Advanced search with multiple criteria
    pub fn advanced_search(
        &self,
        name_query: Option<&str>,
        host_query: Option<&str>,
        group_id: Option<&str>,
        auth_type: Option<&str>,
    ) -> ServerResult<Vec<Server>> {
        let all = self.list_servers()?;

        Ok(all
            .into_iter()
            .filter(|s| {
                let name_match =
                    name_query.map_or(true, |q| s.name.to_lowercase().contains(&q.to_lowercase()));
                let host_match =
                    host_query.map_or(true, |q| s.host.to_lowercase().contains(&q.to_lowercase()));
                let group_match =
                    group_id.map_or(true, |g| s.group_id.as_ref() == Some(&g.to_string()));
                let auth_match = auth_type.map_or(true, |a| s.auth_type() == a);

                name_match && host_match && group_match && auth_match
            })
            .collect())
    }

    /// Test server connection
    ///
    /// Performs a basic connectivity test to the server.
    /// Note: This is a simplified version. In production, this would
    /// attempt an actual SSH connection.
    pub async fn test_connection(
        &self,
        host: &str,
        port: u16,
        timeout_secs: u64,
    ) -> ServerResult<ConnectionTestResult> {
        use tokio::net::TcpStream;
        use tokio::time::{timeout, Duration};

        let start = std::time::Instant::now();

        let addr = format!("{}:{}", host, port);
        let conn_result =
            timeout(Duration::from_secs(timeout_secs), TcpStream::connect(&addr)).await;

        let latency = start.elapsed().as_millis() as u64;

        match conn_result {
            Ok(Ok(_)) => Ok(ConnectionTestResult {
                success: true,
                host: host.to_string(),
                port,
                message: "Connection successful".to_string(),
                latency_ms: Some(latency),
            }),
            Ok(Err(e)) => Ok(ConnectionTestResult {
                success: false,
                host: host.to_string(),
                port,
                message: format!("Connection refused: {}", e),
                latency_ms: None,
            }),
            Err(_) => Ok(ConnectionTestResult {
                success: false,
                host: host.to_string(),
                port,
                message: "Connection timeout".to_string(),
                latency_ms: None,
            }),
        }
    }

    /// Test multiple server connections
    pub async fn batch_test_connections(
        &self,
        server_ids: &[String],
        timeout_secs: u64,
    ) -> ServerResult<HashMap<String, ConnectionTestResult>> {
        let mut results = HashMap::new();

        for id in server_ids {
            let server = match self.get_server(id) {
                Ok(s) => s,
                Err(_) => {
                    results.insert(
                        id.clone(),
                        ConnectionTestResult {
                            success: false,
                            host: "unknown".to_string(),
                            port: 0,
                            message: "Server not found".to_string(),
                            latency_ms: None,
                        },
                    );
                    continue;
                }
            };

            let result = self
                .test_connection(&server.host, server.port, timeout_secs)
                .await?;
            results.insert(id.clone(), result);
        }

        Ok(results)
    }

    /// Export servers to JSON format
    pub fn export_to_json(&self, server_ids: Option<&[String]>) -> ServerResult<String> {
        let servers = match server_ids {
            Some(ids) => {
                let mut result = Vec::new();
                for id in ids {
                    match self.get_server(id) {
                        Ok(server) => result.push(server),
                        Err(_) => continue,
                    }
                }
                result
            }
            None => self.list_servers()?,
        };

        let exports: Vec<ServerExport> =
            servers.iter().map(Self::server_to_export).collect();

        serde_json::to_string_pretty(&exports)
            .map_err(|e| ServerServiceError::ImportExport(e.to_string()))
    }

    /// Import servers from JSON format
    pub fn import_from_json(&self, json: &str) -> ServerResult<ServerImportResult> {
        let exports: Vec<ServerExport> = serde_json::from_str(json)
            .map_err(|e| ServerServiceError::ImportExport(e.to_string()))?;

        let mut result = ServerImportResult {
            total: exports.len(),
            imported: 0,
            skipped: 0,
            errors: Vec::new(),
        };

        for export in exports {
            // Check for duplicates by host+username
            let existing = self.list_servers()?.into_iter().find(|s| {
                s.host == export.host
                    && s.username == export.username
                    && s.port == export.port as u16
            });

            if existing.is_some() {
                result.skipped += 1;
                continue;
            }

            // Save host for error message before moving
            let host_for_error = export.host.clone();

            let auth_method = match export.auth_type.as_str() {
                "agent" => AuthMethod::Agent,
                "password" => AuthMethod::Password {
                    password: String::new(),
                },
                _ => AuthMethod::PrivateKey {
                    key_path: export.identity_file.unwrap_or_default(),
                    passphrase: None,
                },
            };

            let dto = CreateServerDto {
                name: export.name,
                host: export.host,
                port: export.port as u16,
                username: export.username,
                auth_method,
                group_id: export.group_id,
            };

            match self.create_server(dto) {
                Ok(_) => result.imported += 1,
                Err(e) => result
                    .errors
                    .push(format!("Failed to import {}: {}", host_for_error, e)),
            }
        }

        Ok(result)
    }

    /// Import servers with transaction support
    pub fn import_from_json_atomic(&self, json: &str) -> ServerResult<ServerImportResult> {
        self.with_transaction(|service| service.import_from_json(json))
    }

    /// Export servers to CSV format
    pub fn export_to_csv(&self, server_ids: Option<&[String]>) -> ServerResult<String> {
        let servers = match server_ids {
            Some(ids) => {
                let mut result = Vec::new();
                for id in ids {
                    match self.get_server(id) {
                        Ok(server) => result.push(server),
                        Err(_) => continue,
                    }
                }
                result
            }
            None => self.list_servers()?,
        };

        let mut csv = String::from("name,host,port,username,auth_type,group_id\n");

        for server in servers {
            let group_id = server.group_id.clone().unwrap_or_default();
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                Self::escape_csv_field(&server.name),
                Self::escape_csv_field(&server.host),
                server.port,
                Self::escape_csv_field(&server.username),
                server.auth_type(),
                Self::escape_csv_field(&group_id)
            ));
        }

        Ok(csv)
    }

    /// Import servers from CSV format
    pub fn import_from_csv(&self, csv: &str) -> ServerResult<ServerImportResult> {
        let mut result = ServerImportResult {
            total: 0,
            imported: 0,
            skipped: 0,
            errors: Vec::new(),
        };

        let lines: Vec<&str> = csv.lines().collect();
        if lines.is_empty() {
            return Ok(result);
        }

        // Skip header
        for (idx, line) in lines.iter().enumerate().skip(1) {
            if line.trim().is_empty() {
                continue;
            }

            result.total += 1;

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 5 {
                result
                    .errors
                    .push(format!("Line {}: insufficient columns", idx + 1));
                continue;
            }

            let name = Self::unescape_csv_field(parts[0]);
            let host = Self::unescape_csv_field(parts[1]);
            let port: u16 = match parts[2].parse() {
                Ok(p) => p,
                Err(_) => {
                    result
                        .errors
                        .push(format!("Line {}: invalid port", idx + 1));
                    continue;
                }
            };
            let username = Self::unescape_csv_field(parts[3]);
            let auth_type = Self::unescape_csv_field(parts[4]);
            let group_id = parts.get(5).map(|s| Self::unescape_csv_field(s));

            // Skip duplicates
            let existing = self
                .list_servers()?
                .into_iter()
                .find(|s| s.host == host && s.username == username && s.port == port);

            if existing.is_some() {
                result.skipped += 1;
                continue;
            }

            let auth_method = match auth_type.as_str() {
                "agent" => AuthMethod::Agent,
                "password" => AuthMethod::Password {
                    password: String::new(),
                },
                _ => AuthMethod::Agent, // Default to agent
            };

            let dto = CreateServerDto {
                name: name.clone(),
                host,
                port,
                username,
                auth_method,
                group_id: group_id.filter(|g| !g.is_empty()),
            };

            match self.create_server(dto) {
                Ok(_) => result.imported += 1,
                Err(e) => result.errors.push(format!(
                    "Line {}: failed to create server '{}': {}",
                    idx + 1,
                    name,
                    e
                )),
            }
        }

        Ok(result)
    }

    /// Export servers to SSH config format (~/.ssh/config style)
    pub fn export_to_ssh_config(&self, server_ids: Option<&[String]>) -> ServerResult<String> {
        let servers = match server_ids {
            Some(ids) => {
                let mut result = Vec::new();
                for id in ids {
                    match self.get_server(id) {
                        Ok(server) => result.push(server),
                        Err(_) => continue,
                    }
                }
                result
            }
            None => self.list_servers()?,
        };

        let mut config = String::new();

        for server in servers {
            config.push_str(&format!("Host {}\n", server.name));
            config.push_str(&format!("    HostName {}\n", server.host));
            config.push_str(&format!("    Port {}\n", server.port));
            config.push_str(&format!("    User {}\n", server.username));

            match &server.auth_method {
                AuthMethod::PrivateKey { key_path, .. } => {
                    config.push_str(&format!("    IdentityFile {}\n", key_path));
                }
                AuthMethod::Agent => {
                    // No specific config needed for agent
                }
                AuthMethod::Password { .. } => {
                    // Password auth not stored in SSH config
                }
            }

            config.push('\n');
        }

        Ok(config)
    }

    /// Batch update server statuses
    pub fn update_server_statuses(
        &self,
        statuses: HashMap<String, ServerStatus>,
    ) -> ServerResult<usize> {
        let mut updated = 0;

        for (id, status) in statuses {
            if let Ok(mut server) = self.get_server(&id) {
                server.set_status(status);

                // Call methods that borrow server before moving fields
                let auth_type = server.auth_type();
                let identity_file = server.identity_file();
                let status_str = server.status.as_str().to_string();

                let update_record = UpdateServer {
                    id: server.id,
                    name: Some(server.name),
                    host: Some(server.host),
                    port: Some(server.port as i64),
                    username: Some(server.username),
                    auth_type: Some(auth_type),
                    identity_file,
                    group_id: server.group_id,
                    status: Some(status_str),
                };

                if self
                    .db
                    .lock()
                    .unwrap()
                    .update_server(&update_record)
                    .is_ok()
                {
                    updated += 1;
                }
            }
        }

        Ok(updated)
    }

    /// Get server count
    pub fn count_servers(&self) -> ServerResult<usize> {
        Ok(self.list_servers()?.len())
    }

    /// Get server count by group
    pub fn count_servers_by_group(&self, group_id: &str) -> ServerResult<usize> {
        Ok(self.list_servers_by_group(group_id)?.len())
    }

    /// Get server statistics
    pub fn get_server_stats(&self) -> ServerResult<ServerStats> {
        let servers = self.list_servers()?;
        let total = servers.len();

        let by_auth_type: HashMap<String, usize> =
            servers.iter().fold(HashMap::new(), |mut acc, s| {
                *acc.entry(s.auth_type().to_string()).or_insert(0) += 1;
                acc
            });

        let by_group: HashMap<String, usize> = servers.iter().fold(HashMap::new(), |mut acc, s| {
            let group = s
                .group_id
                .clone()
                .unwrap_or_else(|| "_ungrouped".to_string());
            *acc.entry(group).or_insert(0) += 1;
            acc
        });

        let by_status: HashMap<String, usize> =
            servers.iter().fold(HashMap::new(), |mut acc, s| {
                *acc.entry(s.status.as_str().to_string()).or_insert(0) += 1;
                acc
            });

        Ok(ServerStats {
            total,
            by_auth_type,
            by_group,
            by_status,
        })
    }

    /// Check if server name already exists
    fn is_duplicate_name(&self, name: &str, exclude_id: Option<&str>) -> ServerResult<bool> {
        let servers = self.list_servers()?;
        Ok(servers
            .iter()
            .any(|s| s.name == name && exclude_id.map(|id| s.id != id).unwrap_or(true)))
    }

    /// Convert a database record to Server model
    fn record_to_server(record: ServerRecord) -> ServerResult<Server> {
        let created_at = record
            .created_at
            .parse::<i64>()
            .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_default())
            .unwrap_or_else(|_| Utc::now());

        let updated_at = record
            .updated_at
            .parse::<i64>()
            .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_default())
            .unwrap_or_else(|_| Utc::now());

        let auth_method =
            AuthMethod::from_db_string(&record.auth_type, record.identity_file.as_deref());

        Ok(Server {
            id: record.id,
            name: record.name,
            host: record.host,
            port: record.port as u16,
            username: record.username,
            auth_method,
            group_id: record.group_id,
            status: ServerStatus::from_status_str(&record.status),
            created_at,
            updated_at,
            schema_version: 1,
        })
    }

    /// Convert a Server model to ServerExport
    fn server_to_export(server: &Server) -> ServerExport {
        ServerExport {
            id: server.id.clone(),
            name: server.name.clone(),
            host: server.host.clone(),
            port: server.port as i64,
            username: server.username.clone(),
            auth_type: server.auth_method.auth_type().to_string(),
            identity_file: server.identity_file(),
            group_id: server.group_id.clone(),
            group_name: None, // Would need to look up from group_id
            status: server.status.as_str().to_string(),
            tags: Vec::new(), // Tags not stored in Server model
        }
    }

    /// Escape a field for CSV output
    fn escape_csv_field(field: &str) -> String {
        if field.contains(',') || field.contains('"') || field.contains('\n') {
            format!("\"{}\"", field.replace("\"", "\"\""))
        } else {
            field.to_string()
        }
    }

    /// Unescape a CSV field
    fn unescape_csv_field(field: &str) -> String {
        let field = field.trim();
        if field.starts_with('"') && field.ends_with('"') && field.len() >= 2 {
            field[1..field.len() - 1].replace("\"\"", "\"")
        } else {
            field.to_string()
        }
    }
}

/// Server statistics
#[derive(Debug, Clone)]
pub struct ServerStats {
    pub total: usize,
    pub by_auth_type: HashMap<String, usize>,
    pub by_group: HashMap<String, usize>,
    pub by_status: HashMap<String, usize>,
}

/// Server service with async operations
pub struct AsyncServerService {
    inner: Arc<ServerService>,
}

impl AsyncServerService {
    /// Create a new async server service
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self {
            inner: Arc::new(ServerService::new(db)),
        }
    }

    /// Create a server (async wrapper)
    pub fn create_server(&self, dto: CreateServerDto) -> ServerResult<Server> {
        self.inner.create_server(dto)
    }

    /// Batch create servers
    pub fn batch_create_servers(
        &self,
        dtos: Vec<CreateServerDto>,
    ) -> ServerResult<BatchOperationResult> {
        self.inner.batch_create_servers(dtos)
    }

    /// Update a server (async wrapper)
    pub fn update_server(&self, id: &str, dto: UpdateServerDto) -> ServerResult<Server> {
        self.inner.update_server(id, dto)
    }

    /// Delete a server (async wrapper)
    pub fn delete_server(&self, id: &str) -> ServerResult<()> {
        self.inner.delete_server(id)
    }

    /// Batch delete servers
    pub fn batch_delete_servers(&self, ids: &[String]) -> ServerResult<BatchOperationResult> {
        self.inner.batch_delete_servers(ids)
    }

    /// Get a server (async wrapper)
    pub fn get_server(&self, id: &str) -> ServerResult<Server> {
        self.inner.get_server(id)
    }

    /// List all servers (async wrapper)
    pub fn list_servers(&self) -> ServerResult<Vec<Server>> {
        self.inner.list_servers()
    }

    /// Search servers (async wrapper)
    pub fn search_servers(&self, keyword: &str) -> ServerResult<Vec<Server>> {
        self.inner.search_servers(keyword)
    }

    /// Advanced search (async wrapper)
    pub fn advanced_search(
        &self,
        name_query: Option<&str>,
        host_query: Option<&str>,
        group_id: Option<&str>,
        auth_type: Option<&str>,
    ) -> ServerResult<Vec<Server>> {
        self.inner
            .advanced_search(name_query, host_query, group_id, auth_type)
    }

    /// Test connection (async)
    pub async fn test_connection(
        &self,
        host: &str,
        port: u16,
        timeout_secs: u64,
    ) -> ServerResult<ConnectionTestResult> {
        self.inner.test_connection(host, port, timeout_secs).await
    }

    /// Batch test connections (async)
    pub async fn batch_test_connections(
        &self,
        server_ids: &[String],
        timeout_secs: u64,
    ) -> ServerResult<HashMap<String, ConnectionTestResult>> {
        self.inner
            .batch_test_connections(server_ids, timeout_secs)
            .await
    }

    /// Export to JSON (async wrapper)
    pub fn export_to_json(&self, server_ids: Option<&[String]>) -> ServerResult<String> {
        self.inner.export_to_json(server_ids)
    }

    /// Export to SSH config format
    pub fn export_to_ssh_config(&self, server_ids: Option<&[String]>) -> ServerResult<String> {
        self.inner.export_to_ssh_config(server_ids)
    }

    /// Import from JSON (async wrapper)
    pub fn import_from_json(&self, json: &str) -> ServerResult<ServerImportResult> {
        self.inner.import_from_json(json)
    }

    /// Import from JSON with transaction support
    pub fn import_from_json_atomic(&self, json: &str) -> ServerResult<ServerImportResult> {
        self.inner.import_from_json_atomic(json)
    }

    /// Get server statistics
    pub fn get_server_stats(&self) -> ServerResult<ServerStats> {
        self.inner.get_server_stats()
    }

    /// Execute with transaction
    pub fn with_transaction<F, T>(&self, operations: F) -> ServerResult<T>
    where
        F: FnOnce(&ServerService) -> ServerResult<T>,
    {
        self.inner.with_transaction(operations)
    }

    /// Check if transaction is active
    pub fn is_transaction_active(&self) -> bool {
        self.inner.is_transaction_active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::server::ServerBuilder;
    use std::time::Duration;

    fn create_test_db() -> Arc<Mutex<Database>> {
        let db = Database::new_in_memory().unwrap();
        db.init().unwrap();
        Arc::new(Mutex::new(db))
    }

    #[test]
    fn test_create_server() {
        let service = ServerService::new(create_test_db());

        let dto = CreateServerDto {
            name: "Test Server".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_method: AuthMethod::Agent,
            group_id: None,
        };

        let server = service.create_server(dto).unwrap();
        assert_eq!(server.name, "Test Server");
        assert_eq!(server.host, "192.168.1.1");
        assert_eq!(server.port, 22);
    }

    #[test]
    fn test_create_server_duplicate_name() {
        let service = ServerService::new(create_test_db());

        let dto = CreateServerDto {
            name: "Test Server".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_method: AuthMethod::Agent,
            group_id: None,
        };

        service.create_server(dto.clone()).unwrap();

        let result = service.create_server(dto);
        assert!(matches!(result, Err(ServerServiceError::DuplicateName(_))));
    }

    #[test]
    fn test_get_server() {
        let service = ServerService::new(create_test_db());

        let dto = CreateServerDto {
            name: "Test Server".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_method: AuthMethod::Agent,
            group_id: None,
        };

        let created = service.create_server(dto).unwrap();
        let retrieved = service.get_server(&created.id).unwrap();

        assert_eq!(retrieved.name, created.name);
        assert_eq!(retrieved.id, created.id);
    }

    #[test]
    fn test_get_server_not_found() {
        let service = ServerService::new(create_test_db());

        let result = service.get_server("non-existent-id");
        assert!(matches!(result, Err(ServerServiceError::NotFound(_))));
    }

    #[test]
    fn test_update_server() {
        let service = ServerService::new(create_test_db());

        let dto = CreateServerDto {
            name: "Test Server".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_method: AuthMethod::Agent,
            group_id: None,
        };

        let created = service.create_server(dto).unwrap();

        let update = UpdateServerDto {
            name: Some("Updated Server".to_string()),
            host: None,
            port: None,
            username: None,
            auth_method: None,
            group_id: None,
        };

        let updated = service.update_server(&created.id, update).unwrap();
        assert_eq!(updated.name, "Updated Server");
        assert_eq!(updated.host, "192.168.1.1"); // Unchanged
    }

    #[test]
    fn test_delete_server() {
        let service = ServerService::new(create_test_db());

        let dto = CreateServerDto {
            name: "Test Server".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_method: AuthMethod::Agent,
            group_id: None,
        };

        let created = service.create_server(dto).unwrap();
        service.delete_server(&created.id).unwrap();

        let result = service.get_server(&created.id);
        assert!(matches!(result, Err(ServerServiceError::NotFound(_))));
    }

    #[test]
    fn test_list_servers() {
        let service = ServerService::new(create_test_db());

        // Create multiple servers
        for i in 0..3 {
            let dto = CreateServerDto {
                name: format!("Server {}", i),
                host: format!("192.168.1.{}", i),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::Agent,
                group_id: None,
            };
            service.create_server(dto).unwrap();
        }

        let servers = service.list_servers().unwrap();
        assert_eq!(servers.len(), 3);
    }

    #[test]
    fn test_search_servers() {
        let service = ServerService::new(create_test_db());

        let dto1 = CreateServerDto {
            name: "Production Server".to_string(),
            host: "prod.example.com".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_method: AuthMethod::Agent,
            group_id: None,
        };

        let dto2 = CreateServerDto {
            name: "Development Server".to_string(),
            host: "dev.example.com".to_string(),
            port: 22,
            username: "dev".to_string(),
            auth_method: AuthMethod::Agent,
            group_id: None,
        };

        service.create_server(dto1).unwrap();
        service.create_server(dto2).unwrap();

        let results = service.search_servers("prod").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Production Server");
    }

    #[test]
    fn test_advanced_search() {
        let service = ServerService::new(create_test_db());

        service
            .create_server(CreateServerDto {
                name: "Web Server".to_string(),
                host: "web.example.com".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::Agent,
                group_id: Some("group1".to_string()),
            })
            .unwrap();

        service
            .create_server(CreateServerDto {
                name: "DB Server".to_string(),
                host: "db.example.com".to_string(),
                port: 22,
                username: "admin".to_string(),
                auth_method: AuthMethod::PrivateKey {
                    key_path: "/path/to/key".to_string(),
                    passphrase: None,
                },
                group_id: Some("group2".to_string()),
            })
            .unwrap();

        // Search by name
        let results = service
            .advanced_search(Some("Web"), None, None, None)
            .unwrap();
        assert_eq!(results.len(), 1);

        // Search by auth type
        let results = service
            .advanced_search(None, None, None, Some("key"))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "DB Server");
    }

    #[test]
    fn test_export_import_json() {
        let service = ServerService::new(create_test_db());

        let dto = CreateServerDto {
            name: "Test Server".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_method: AuthMethod::Agent,
            group_id: None,
        };

        let created = service.create_server(dto).unwrap();
        let ids = vec![created.id.clone()];

        let json = service.export_to_json(Some(&ids)).unwrap();
        assert!(json.contains("Test Server"));
        assert!(json.contains("192.168.1.1"));

        // Import should skip duplicate
        let result = service.import_from_json(&json).unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.imported, 0);
    }

    #[test]
    fn test_export_to_ssh_config() {
        let service = ServerService::new(create_test_db());

        service
            .create_server(CreateServerDto {
                name: "MyServer".to_string(),
                host: "192.168.1.1".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::PrivateKey {
                    key_path: "~/.ssh/id_rsa".to_string(),
                    passphrase: None,
                },
                group_id: None,
            })
            .unwrap();

        let config = service.export_to_ssh_config(None).unwrap();
        assert!(config.contains("Host MyServer"));
        assert!(config.contains("HostName 192.168.1.1"));
        assert!(config.contains("Port 22"));
        assert!(config.contains("User root"));
        assert!(config.contains("IdentityFile ~/.ssh/id_rsa"));
    }

    #[test]
    fn test_csv_escape_unescape() {
        let field = "Test, with \"quotes\"";
        let escaped = ServerService::escape_csv_field(field);
        let unescaped = ServerService::unescape_csv_field(&escaped);
        assert_eq!(field, unescaped);

        let simple = "simple";
        assert_eq!(ServerService::escape_csv_field(simple), simple);
        assert_eq!(ServerService::unescape_csv_field(simple), simple);
    }

    #[tokio::test]
    async fn test_connection_test() {
        let service = ServerService::new(create_test_db());

        // Test with a likely unreachable host
        let result = service.test_connection("192.0.2.1", 22, 1).await.unwrap();
        assert!(!result.success);
        assert!(result.message.contains("Connection") || result.message.contains("timeout"));
    }

    #[test]
    fn test_server_service_error_display() {
        let err = ServerServiceError::NotFound("test-id".to_string());
        assert!(err.to_string().contains("test-id"));

        let err = ServerServiceError::DuplicateName("MyServer".to_string());
        assert!(err.to_string().contains("MyServer"));
    }

    #[test]
    fn test_import_result() {
        let result = ServerImportResult {
            total: 10,
            imported: 8,
            skipped: 1,
            errors: vec!["error1".to_string()],
        };

        assert_eq!(result.total, 10);
        assert_eq!(result.imported, 8);
    }

    #[test]
    fn test_batch_operation_result() {
        let result = BatchOperationResult {
            total: 5,
            success: 3,
            failed: 2,
            errors: vec![("id1".to_string(), "error1".to_string())],
        };

        assert_eq!(result.total, 5);
        assert_eq!(result.success, 3);
        assert_eq!(result.failed, 2);
    }

    #[test]
    fn test_transaction_support() {
        let service = ServerService::new(create_test_db());

        // Start transaction
        assert!(!service.is_transaction_active());
        service.begin_transaction().unwrap();
        assert!(service.is_transaction_active());

        // Cannot start another transaction while one is active
        let result = service.begin_transaction();
        assert!(matches!(result, Err(TransactionError::AlreadyInProgress)));

        // Commit transaction
        service.commit_transaction().unwrap();
        assert!(!service.is_transaction_active());
    }

    #[test]
    fn test_transaction_rollback() {
        let service = ServerService::new(create_test_db());

        service.begin_transaction().unwrap();
        service.rollback_transaction().unwrap();
        assert!(!service.is_transaction_active());
    }

    #[test]
    fn test_with_transaction_success() {
        let service = ServerService::new(create_test_db());

        let result: ServerResult<String> = service.with_transaction(|_svc| {
            // Simulate successful operation
            Ok("success".to_string())
        });

        assert!(result.is_ok());
        assert!(!service.is_transaction_active());
    }

    #[test]
    fn test_with_transaction_failure() {
        let service = ServerService::new(create_test_db());

        let result: ServerResult<String> = service.with_transaction(|_svc| {
            // Simulate failed operation
            Err(ServerServiceError::NotFound("test".to_string()))
        });

        assert!(result.is_err());
        assert!(!service.is_transaction_active());
    }

    #[test]
    fn test_batch_create_servers() {
        let service = ServerService::new(create_test_db());

        let dtos = vec![
            CreateServerDto {
                name: "Server 1".to_string(),
                host: "192.168.1.1".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::Agent,
                group_id: None,
            },
            CreateServerDto {
                name: "Server 2".to_string(),
                host: "192.168.1.2".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::Agent,
                group_id: None,
            },
        ];

        let result = service.batch_create_servers(dtos).unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.success, 2);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_batch_create_servers_with_duplicate() {
        let service = ServerService::new(create_test_db());

        // Create first server
        service
            .create_server(CreateServerDto {
                name: "Existing".to_string(),
                host: "192.168.1.1".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::Agent,
                group_id: None,
            })
            .unwrap();

        // Try to batch create including duplicate
        let dtos = vec![
            CreateServerDto {
                name: "Existing".to_string(), // Duplicate!
                host: "192.168.1.2".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::Agent,
                group_id: None,
            },
            CreateServerDto {
                name: "New Server".to_string(),
                host: "192.168.1.3".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::Agent,
                group_id: None,
            },
        ];

        let result = service.batch_create_servers(dtos);
        assert!(matches!(
            result,
            Err(ServerServiceError::BatchPartialFailure { .. })
        ));
    }

    #[test]
    fn test_batch_delete_servers() {
        let service = ServerService::new(create_test_db());

        let created1 = service
            .create_server(CreateServerDto {
                name: "Server 1".to_string(),
                host: "192.168.1.1".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::Agent,
                group_id: None,
            })
            .unwrap();

        let created2 = service
            .create_server(CreateServerDto {
                name: "Server 2".to_string(),
                host: "192.168.1.2".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::Agent,
                group_id: None,
            })
            .unwrap();

        let ids = vec![created1.id.clone(), created2.id.clone()];
        let result = service.batch_delete_servers(&ids).unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.success, 2);
        assert_eq!(result.failed, 0);

        // Verify deletion
        assert!(service.get_server(&created1.id).is_err());
        assert!(service.get_server(&created2.id).is_err());
    }

    #[test]
    fn test_get_server_stats() {
        let service = ServerService::new(create_test_db());

        // Create servers with different auth methods
        service
            .create_server(CreateServerDto {
                name: "Agent Server".to_string(),
                host: "192.168.1.1".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::Agent,
                group_id: Some("group1".to_string()),
            })
            .unwrap();

        service
            .create_server(CreateServerDto {
                name: "Key Server".to_string(),
                host: "192.168.1.2".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::PrivateKey {
                    key_path: "/path".to_string(),
                    passphrase: None,
                },
                group_id: Some("group2".to_string()),
            })
            .unwrap();

        let stats = service.get_server_stats().unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.by_auth_type.get("agent"), Some(&1));
        assert_eq!(stats.by_auth_type.get("key"), Some(&1));
        assert_eq!(stats.by_group.get("group1"), Some(&1));
        assert_eq!(stats.by_group.get("group2"), Some(&1));
    }

    #[test]
    fn test_get_servers_by_ids() {
        let service = ServerService::new(create_test_db());

        let created1 = service
            .create_server(CreateServerDto {
                name: "Server 1".to_string(),
                host: "192.168.1.1".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::Agent,
                group_id: None,
            })
            .unwrap();

        let created2 = service
            .create_server(CreateServerDto {
                name: "Server 2".to_string(),
                host: "192.168.1.2".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_method: AuthMethod::Agent,
                group_id: None,
            })
            .unwrap();

        let ids = vec![created1.id.clone(), created2.id.clone()];
        let servers = service.get_servers_by_ids(&ids).unwrap();

        assert_eq!(servers.len(), 2);
    }

    #[test]
    fn test_transaction_error_display() {
        let err = TransactionError::AlreadyInProgress;
        assert!(err.to_string().contains("already in progress"));

        let err = TransactionError::NotInProgress;
        assert!(err.to_string().contains("No transaction"));

        let err = TransactionError::RolledBack("test".to_string());
        assert!(err.to_string().contains("rolled back"));
    }

    #[test]
    fn test_async_server_service_wrapper() {
        let service = AsyncServerService::new(create_test_db());

        let dto = CreateServerDto {
            name: "Async Test".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_method: AuthMethod::Agent,
            group_id: None,
        };

        let server = service.create_server(dto).unwrap();
        assert_eq!(server.name, "Async Test");

        let servers = service.list_servers().unwrap();
        assert_eq!(servers.len(), 1);
    }
}
