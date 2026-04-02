//! Server Service
//!
//! This module provides the ServerService which implements complete CRUD operations
//! for SSH server management including validation, import/export, and connection testing.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config_import_export::ServerExport;
use crate::db::{Database, NewServer, ServerRecord, UpdateServer};
use crate::error::LiteError;
use crate::models::server::{
    AuthMethod, CreateServerDto, Server, ServerBuilder, ServerStatus,
    UpdateServerDto,
};
use crate::models::{Validatable, ValidationError};

/// Result type for server service operations
pub type ServerResult<T> = Result<T, ServerServiceError>;

/// Error type for server service operations
#[derive(Debug, Clone, PartialEq)]
pub enum ServerServiceError {
    /// Server not found
    NotFound(String),
    /// Validation failed
    Validation(ValidationError),
    /// Database error
    Database(String),
    /// Import/Export error
    ImportExport(String),
    /// Connection test failed
    ConnectionTestFailed { host: String, message: String },
    /// Duplicate server name
    DuplicateName(String),
    /// Server already exists
    AlreadyExists(String),
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

/// Server service for managing SSH server configurations
pub struct ServerService {
    db: Arc<Mutex<Database>>,
}

impl ServerService {
    /// Create a new server service instance
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
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

    /// Delete a server by ID
    pub fn delete_server(&self, id: &str) -> ServerResult<()> {
        // Check if server exists
        self.get_server(id)?;

        self.db.lock().unwrap().delete_server(id)?;
        Ok(())
    }

    /// Get a single server by ID
    pub fn get_server(&self, id: &str) -> ServerResult<Server> {
        let record = self.db.lock().unwrap().get_server(id)?;
        Self::record_to_server(record)
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
        let conn_result = timeout(Duration::from_secs(timeout_secs), TcpStream::connect(&addr))
            .await;

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

        let exports: Vec<ServerExport> = servers.iter().map(|s| Self::server_to_export(s)).collect();

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
                Err(e) => result.errors.push(format!(
                    "Failed to import {}: {}",
                    host_for_error,
                    e
                )),
            }
        }

        Ok(result)
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
            let existing = self.list_servers()?.into_iter().find(|s| {
                s.host == host && s.username == username && s.port == port
            });

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

    /// Batch update server statuses
    pub fn update_server_statuses(&self, statuses: HashMap<String, ServerStatus>) -> ServerResult<usize> {
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

                if self.db.lock().unwrap().update_server(&update_record).is_ok() {
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

    /// Check if server name already exists
    fn is_duplicate_name(&self, name: &str, exclude_id: Option<&str>) -> ServerResult<bool> {
        let servers = self.list_servers()?;
        Ok(servers.iter().any(|s| {
            s.name == name && exclude_id.map(|id| s.id != id).unwrap_or(true)
        }))
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

        let auth_method = AuthMethod::from_db_string(&record.auth_type, record.identity_file.as_deref());

        Ok(Server {
            id: record.id,
            name: record.name,
            host: record.host,
            port: record.port as u16,
            username: record.username,
            auth_method,
            group_id: record.group_id,
            status: ServerStatus::from_str(&record.status),
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

    /// Update a server (async wrapper)
    pub fn update_server(&self, id: &str, dto: UpdateServerDto) -> ServerResult<Server> {
        self.inner.update_server(id, dto)
    }

    /// Delete a server (async wrapper)
    pub fn delete_server(&self, id: &str) -> ServerResult<()> {
        self.inner.delete_server(id)
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

    /// Test connection (async)
    pub async fn test_connection(
        &self,
        host: &str,
        port: u16,
        timeout_secs: u64,
    ) -> ServerResult<ConnectionTestResult> {
        self.inner.test_connection(host, port, timeout_secs).await
    }

    /// Export to JSON (async wrapper)
    pub fn export_to_json(&self, server_ids: Option<&[String]>) -> ServerResult<String> {
        self.inner.export_to_json(server_ids)
    }

    /// Import from JSON (async wrapper)
    pub fn import_from_json(&self, json: &str) -> ServerResult<ServerImportResult> {
        self.inner.import_from_json(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::server::ServerBuilder;
    use std::time::Duration;
    use tempfile::tempdir;

    fn create_test_db() -> Arc<Mutex<Database>> {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(db_path).unwrap();
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
        assert!(matches!(
            result,
            Err(ServerServiceError::DuplicateName(_))
        ));
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
}
