//! Server Model
//!
//! This module defines the Server domain model and authentication methods
//! for the EasySSH application.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::{Validatable, ValidationError, MAX_NAME_LENGTH, MAX_USERNAME_LENGTH, is_valid_host, is_valid_port as validate_port};

/// Server authentication method
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// SSH Agent authentication
    Agent,
    /// Password authentication
    Password { password: String },
    /// Private key authentication
    PrivateKey {
        key_path: String,
        passphrase: Option<String>,
    },
}

impl fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthMethod::Agent => write!(f, "agent"),
            AuthMethod::Password { .. } => write!(f, "password"),
            AuthMethod::PrivateKey { key_path, .. } => {
                write!(f, "key:{}", key_path)
            }
        }
    }
}

impl AuthMethod {
    /// Convert auth method to database string representation
    pub fn to_db_string(&self) -> String {
        match self {
            AuthMethod::Agent => "agent".to_string(),
            AuthMethod::Password { .. } => "password".to_string(),
            AuthMethod::PrivateKey { key_path, .. } => format!("key:{}", key_path),
        }
    }

    /// Create auth method from database string
    pub fn from_db_string(s: &str, identity_file: Option<&str>) -> Self {
        match s {
            "agent" => AuthMethod::Agent,
            "password" => AuthMethod::Password {
                password: String::new(),
            },
            _ => AuthMethod::PrivateKey {
                key_path: identity_file.map(|p| p.to_string()).unwrap_or_default(),
                passphrase: None,
            },
        }
    }

    /// Get the auth type string
    pub fn auth_type(&self) -> &'static str {
        match self {
            AuthMethod::Agent => "agent",
            AuthMethod::Password { .. } => "password",
            AuthMethod::PrivateKey { .. } => "key",
        }
    }
}

/// Server status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerStatus {
    /// Server status unknown
    Unknown,
    /// Server is online and reachable
    Online,
    /// Server is offline
    Offline,
    /// Server connection error
    Error,
    /// Currently connecting
    Connecting,
}

impl fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerStatus::Unknown => write!(f, "unknown"),
            ServerStatus::Online => write!(f, "online"),
            ServerStatus::Offline => write!(f, "offline"),
            ServerStatus::Error => write!(f, "error"),
            ServerStatus::Connecting => write!(f, "connecting"),
        }
    }
}

impl ServerStatus {
    /// Convert to database string
    pub fn as_str(&self) -> &'static str {
        match self {
            ServerStatus::Unknown => "unknown",
            ServerStatus::Online => "online",
            ServerStatus::Offline => "offline",
            ServerStatus::Error => "error",
            ServerStatus::Connecting => "connecting",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s {
            "online" => ServerStatus::Online,
            "offline" => ServerStatus::Offline,
            "error" => ServerStatus::Error,
            "connecting" => ServerStatus::Connecting,
            _ => ServerStatus::Unknown,
        }
    }
}

impl Default for ServerStatus {
    fn default() -> Self {
        ServerStatus::Unknown
    }
}

/// Server domain model
///
/// Represents an SSH server configuration with all its properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    /// Unique server identifier (UUID)
    pub id: String,
    /// Display name for the server
    pub name: String,
    /// Hostname or IP address
    pub host: String,
    /// SSH port (default: 22)
    pub port: u16,
    /// Username for authentication
    pub username: String,
    /// Authentication method
    pub auth_method: AuthMethod,
    /// Optional group ID for organization
    pub group_id: Option<String>,
    /// Server status
    pub status: ServerStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Schema version for migrations
    #[serde(default)]
    pub schema_version: u32,
}

impl Server {
    /// Create a new server with generated ID and timestamps
    pub fn new(
        name: String,
        host: String,
        port: u16,
        username: String,
        auth_method: AuthMethod,
        group_id: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            host,
            port,
            username,
            auth_method,
            group_id,
            status: ServerStatus::Unknown,
            created_at: now,
            updated_at: now,
            schema_version: 1,
        }
    }

    /// Create a server with a specific ID (used when loading from database)
    pub fn with_id(
        id: String,
        name: String,
        host: String,
        port: u16,
        username: String,
        auth_method: AuthMethod,
        group_id: Option<String>,
        status: ServerStatus,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            host,
            port,
            username,
            auth_method,
            group_id,
            status,
            created_at,
            updated_at,
            schema_version: 1,
        }
    }

    /// Update the server and refresh the updated_at timestamp
    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.updated_at = Utc::now();
    }

    /// Set the server status and update timestamp
    pub fn set_status(&mut self, status: ServerStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Get the identity file path if using key authentication
    pub fn identity_file(&self) -> Option<String> {
        match &self.auth_method {
            AuthMethod::PrivateKey { key_path, .. } => Some(key_path.clone()),
            _ => None,
        }
    }

    /// Get the authentication type as string
    pub fn auth_type(&self) -> String {
        self.auth_method.auth_type().to_string()
    }

    /// Convert to a brief summary string
    pub fn summary(&self) -> String {
        format!(
            "{} ({}@{}:{})",
            self.name, self.username, self.host, self.port
        )
    }

    /// Check if server is in a group
    pub fn is_in_group(&self, group_id: &str) -> bool {
        self.group_id.as_ref().map(|id| id == group_id).unwrap_or(false)
    }

    /// Get the connection string (username@host:port)
    pub fn connection_string(&self) -> String {
        format!("{}@{}:{}", self.username, self.host, self.port)
    }
}

impl Validatable for Server {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate name
        if self.name.trim().is_empty() {
            return Err(ValidationError::InvalidField {
                field: "name".to_string(),
                message: "Name cannot be empty".to_string(),
            });
        }
        if self.name.len() > MAX_NAME_LENGTH {
            return Err(ValidationError::InvalidField {
                field: "name".to_string(),
                message: format!("Name too long (max {} characters)", MAX_NAME_LENGTH),
            });
        }

        // Validate host
        if self.host.trim().is_empty() {
            return Err(ValidationError::InvalidField {
                field: "host".to_string(),
                message: "Host cannot be empty".to_string(),
            });
        }
        if !is_valid_host(&self.host) {
            return Err(ValidationError::InvalidFormat {
                field: "host".to_string(),
                expected: "valid hostname or IP address".to_string(),
            });
        }

        // Validate port
        if !validate_port(self.port) {
            return Err(ValidationError::InvalidField {
                field: "port".to_string(),
                message: format!("Invalid port: {}", self.port),
            });
        }

        // Validate username
        if self.username.trim().is_empty() {
            return Err(ValidationError::InvalidField {
                field: "username".to_string(),
                message: "Username cannot be empty".to_string(),
            });
        }
        if self.username.len() > MAX_USERNAME_LENGTH {
            return Err(ValidationError::InvalidField {
                field: "username".to_string(),
                message: format!("Username too long (max {} characters)", MAX_USERNAME_LENGTH),
            });
        }

        // Validate auth method
        match &self.auth_method {
            AuthMethod::PrivateKey { key_path, .. } => {
                if key_path.trim().is_empty() {
                    return Err(ValidationError::InvalidField {
                        field: "auth_method.key_path".to_string(),
                        message: "Private key path cannot be empty".to_string(),
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// Builder for creating Server instances
#[derive(Debug, Default)]
pub struct ServerBuilder {
    id: Option<String>,
    name: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    auth_method: Option<AuthMethod>,
    group_id: Option<String>,
    status: Option<ServerStatus>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

impl ServerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the server ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the server name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the host
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Set the port
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Set the username
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the authentication method
    pub fn auth_method(mut self, auth_method: AuthMethod) -> Self {
        self.auth_method = Some(auth_method);
        self
    }

    /// Set the group ID
    pub fn group_id(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = Some(group_id.into());
        self
    }

    /// Set the status
    pub fn status(mut self, status: ServerStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the created_at timestamp
    pub fn created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Set the updated_at timestamp
    pub fn updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// Build the Server instance
    ///
    /// Panics if required fields are not set
    pub fn build(self) -> Server {
        let now = Utc::now();
        Server {
            id: self.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: self.name.expect("name is required"),
            host: self.host.expect("host is required"),
            port: self.port.unwrap_or(22),
            username: self.username.expect("username is required"),
            auth_method: self.auth_method.unwrap_or(AuthMethod::Agent),
            group_id: self.group_id,
            status: self.status.unwrap_or_default(),
            created_at: self.created_at.unwrap_or(now),
            updated_at: self.updated_at.unwrap_or(now),
            schema_version: 1,
        }
    }

    /// Build with validation
    pub fn build_validated(self) -> Result<Server, ValidationError> {
        let server = self.build();
        server.validate()?;
        Ok(server)
    }
}

/// Server data transfer object for creating new servers
#[derive(Debug, Clone, Deserialize)]
pub struct CreateServerDto {
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub auth_method: AuthMethod,
    pub group_id: Option<String>,
}

fn default_port() -> u16 {
    22
}

/// Server data transfer object for updating servers
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateServerDto {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub auth_method: Option<AuthMethod>,
    pub group_id: Option<Option<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = Server::new(
            "Test Server".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            AuthMethod::Agent,
            None,
        );

        assert_eq!(server.name, "Test Server");
        assert_eq!(server.host, "192.168.1.1");
        assert_eq!(server.port, 22);
        assert_eq!(server.username, "root");
        assert!(matches!(server.auth_method, AuthMethod::Agent));
        assert_eq!(server.schema_version, 1);
    }

    #[test]
    fn test_server_validation_valid() {
        let server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert!(server.validate().is_ok());
    }

    #[test]
    fn test_server_validation_empty_name() {
        let server = Server::new(
            "".to_string(),
            "example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert!(matches!(
            server.validate(),
            Err(ValidationError::InvalidField { field, .. }) if field == "name"
        ));
    }

    #[test]
    fn test_server_validation_long_name() {
        let server = Server::new(
            "a".repeat(101),
            "example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert!(matches!(
            server.validate(),
            Err(ValidationError::InvalidField { field, .. }) if field == "name"
        ));
    }

    #[test]
    fn test_server_validation_invalid_port() {
        let server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            0,
            "user".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert!(matches!(
            server.validate(),
            Err(ValidationError::InvalidField { field, .. }) if field == "port"
        ));
    }

    #[test]
    fn test_server_validation_invalid_ip() {
        let server = Server::new(
            "Test".to_string(),
            "999.999.999.999".to_string(),
            22,
            "user".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert!(matches!(
            server.validate(),
            Err(ValidationError::InvalidFormat { field, .. }) if field == "host"
        ));
    }

    #[test]
    fn test_server_validation_empty_username() {
        let server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert!(matches!(
            server.validate(),
            Err(ValidationError::InvalidField { field, .. }) if field == "username"
        ));
    }

    #[test]
    fn test_server_validation_long_username() {
        let server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "a".repeat(33),
            AuthMethod::Agent,
            None,
        );
        assert!(matches!(
            server.validate(),
            Err(ValidationError::InvalidField { field, .. }) if field == "username"
        ));
    }

    #[test]
    fn test_server_validation_empty_key_path() {
        let server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::PrivateKey {
                key_path: "".to_string(),
                passphrase: None,
            },
            None,
        );
        assert!(matches!(
            server.validate(),
            Err(ValidationError::InvalidField { field, .. }) if field == "auth_method.key_path"
        ));
    }

    #[test]
    fn test_valid_ipv4() {
        let server = Server::new(
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "user".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert!(server.validate().is_ok());
    }

    #[test]
    fn test_valid_ipv6() {
        let server = Server::new(
            "Test".to_string(),
            "::1".to_string(),
            22,
            "user".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert!(server.validate().is_ok());
    }

    #[test]
    fn test_valid_hostname() {
        let server = Server::new(
            "Test".to_string(),
            "server.example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert!(server.validate().is_ok());
    }

    #[test]
    fn test_auth_method_display() {
        assert_eq!(AuthMethod::Agent.to_string(), "agent");
        assert_eq!(
            AuthMethod::Password {
                password: "secret".to_string()
            }
            .to_string(),
            "password"
        );
    }

    #[test]
    fn test_server_builder() {
        let server = ServerBuilder::new()
            .name("Test Server")
            .host("example.com")
            .port(2222)
            .username("admin")
            .auth_method(AuthMethod::Agent)
            .build();

        assert_eq!(server.name, "Test Server");
        assert_eq!(server.port, 2222);
        assert_eq!(server.username, "admin");
    }

    #[test]
    fn test_server_update() {
        let mut server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::Agent,
            None,
        );

        let old_updated = server.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));

        server.update(|s| {
            s.name = "Updated".to_string();
        });

        assert_eq!(server.name, "Updated");
        assert!(server.updated_at > old_updated);
    }

    #[test]
    fn test_server_status() {
        assert_eq!(ServerStatus::Online.as_str(), "online");
        assert_eq!(ServerStatus::from_str("online"), ServerStatus::Online);
        assert_eq!(ServerStatus::from_str("invalid"), ServerStatus::Unknown);
    }

    #[test]
    fn test_server_connection_string() {
        let server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "root".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert_eq!(server.connection_string(), "root@example.com:22");
    }

    #[test]
    fn test_server_export() {
        let server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "root".to_string(),
            AuthMethod::PrivateKey {
                key_path: "/path/to/key".to_string(),
                passphrase: None,
            },
            Some("group-1".to_string()),
        );

        let export = ServerExport::from(&server);
        assert_eq!(export.name, "Test");
        assert_eq!(export.auth_type, "key");
        assert_eq!(export.identity_file, Some("/path/to/key".to_string()));
    }
}
