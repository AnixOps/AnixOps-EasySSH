//! Server Model
//!
//! This module defines the Server domain model and authentication methods
//! for the EasySSH application. Servers represent SSH hosts that can be
//! connected to, with various authentication options.
//!
//! # Examples
//!
//! ```
//! use easyssh_core::models::{Server, AuthMethod, Validatable};
//!
//! // Create a new server with password authentication
//! let server = Server::new(
//!     "Production Server".to_string(),
//!     "192.168.1.100".to_string(),
//!     22,
//!     "admin".to_string(),
//!     AuthMethod::Password { password: "secret".to_string() },
//!     None,
//! );
//!
//! assert!(server.validate().is_ok());
//! assert_eq!(server.connection_string(), "admin@192.168.1.100:22");
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::{
    is_valid_host, is_valid_port as validate_port, Validatable, ValidationError, MAX_NAME_LENGTH,
    MAX_USERNAME_LENGTH,
};
use crate::models::validation::{is_valid_ssh_key_path, is_valid_ssh_username};

/// Server authentication method
///
/// Represents the different ways to authenticate with an SSH server.
/// Each variant contains the necessary credentials for that method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum AuthMethod {
    /// SSH Agent authentication
    ///
    /// Uses the local SSH agent to provide authentication.
    /// No credentials are stored.
    #[default]
    Agent,

    /// Password authentication
    ///
    /// Uses a plaintext password. Note: This stores the password in memory.
    /// Consider using SSH agent or key-based auth for better security.
    Password {
        /// The password to use for authentication
        #[serde(skip_serializing_if = "String::is_empty", default)]
        password: String,
    },

    /// Private key authentication
    ///
    /// Uses an SSH private key file, optionally with a passphrase.
    PrivateKey {
        /// Path to the private key file
        key_path: String,
        /// Optional passphrase for the private key
        #[serde(skip_serializing_if = "Option::is_none", default)]
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
    ///
    /// This is a compact representation suitable for storage.
    pub fn to_db_string(&self) -> String {
        match self {
            AuthMethod::Agent => "agent".to_string(),
            AuthMethod::Password { .. } => "password".to_string(),
            AuthMethod::PrivateKey { key_path, .. } => format!("key:{}", key_path),
        }
    }

    /// Create auth method from database string
    ///
    /// # Arguments
    /// * `s` - The database string representation
    /// * `identity_file` - Optional path to identity file (used when reconstructing key auth)
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

    /// Get the auth type string (without sensitive data)
    pub fn auth_type(&self) -> &'static str {
        match self {
            AuthMethod::Agent => "agent",
            AuthMethod::Password { .. } => "password",
            AuthMethod::PrivateKey { .. } => "key",
        }
    }

    /// Check if this auth method requires a credential refresh
    ///
    /// Password-based auth may need periodic refresh in some environments.
    pub fn needs_refresh(&self) -> bool {
        matches!(self, AuthMethod::Password { .. })
    }

    /// Check if this auth method stores credentials in memory
    pub fn stores_credentials(&self) -> bool {
        !matches!(self, AuthMethod::Agent)
    }

    /// Validate the auth method configuration
    fn validate(&self) -> Result<(), ValidationError> {
        match self {
            AuthMethod::PrivateKey { key_path, .. } => {
                if key_path.trim().is_empty() {
                    return Err(ValidationError::invalid_field(
                        "auth_method.key_path",
                        "Private key path cannot be empty",
                    ));
                }
                if !is_valid_ssh_key_path(key_path) {
                    return Err(ValidationError::invalid_format(
                        "auth_method.key_path",
                        "valid SSH key file path",
                    ));
                }
                Ok(())
            }
            AuthMethod::Password { password } => {
                if password.is_empty() {
                    // Empty password is allowed for passwordless auth
                    // but we validate the field exists
                    Ok(())
                } else {
                    Ok(())
                }
            }
            AuthMethod::Agent => Ok(()), // Agent auth is always valid
        }
    }
}

/// Server connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ServerStatus {
    /// Server status unknown (default state)
    #[default]
    Unknown,
    /// Server is online and reachable
    Online,
    /// Server is offline or unreachable
    Offline,
    /// Server connection encountered an error
    Error,
    /// Currently attempting to connect
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
    pub fn from_status_str(s: &str) -> Self {
        match s {
            "online" => ServerStatus::Online,
            "offline" => ServerStatus::Offline,
            "error" => ServerStatus::Error,
            "connecting" => ServerStatus::Connecting,
            _ => ServerStatus::Unknown,
        }
    }

    /// Check if the server is currently connectable
    pub fn is_connectable(&self) -> bool {
        matches!(
            self,
            ServerStatus::Unknown | ServerStatus::Online | ServerStatus::Offline
        )
    }

    /// Check if the server is in an active connecting state
    pub fn is_connecting(&self) -> bool {
        matches!(self, ServerStatus::Connecting)
    }
}

/// Server domain model
///
/// Represents an SSH server configuration with all its properties.
/// This is the core entity for managing SSH connections.
///
/// # Fields
///
/// * `id` - Unique identifier (UUID v4)
/// * `name` - Human-readable display name
/// * `host` - Hostname or IP address
/// * `port` - SSH port (default: 22)
/// * `username` - Authentication username
/// * `auth_method` - How to authenticate
/// * `group_id` - Optional organization group
/// * `status` - Current connection status
/// * `created_at` - Creation timestamp
/// * `updated_at` - Last modification timestamp
/// * `schema_version` - Data schema version for migrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    /// Unique server identifier (UUID)
    pub id: String,
    /// Display name for the server
    pub name: String,
    /// Hostname or IP address
    pub host: String,
    /// SSH port (default: 22)
    #[serde(default = "default_port")]
    pub port: u16,
    /// Username for authentication
    pub username: String,
    /// Authentication method
    #[serde(default)]
    pub auth_method: AuthMethod,
    /// Optional group ID for organization
    pub group_id: Option<String>,
    /// Server status
    #[serde(default)]
    pub status: ServerStatus,
    /// Creation timestamp
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
    /// Schema version for migrations
    #[serde(default)]
    pub schema_version: u32,
}

fn default_port() -> u16 {
    22
}

impl Server {
    /// Create a new server with generated ID and timestamps
    ///
    /// # Arguments
    /// * `name` - Display name for the server
    /// * `host` - Hostname or IP address
    /// * `port` - SSH port number
    /// * `username` - Authentication username
    /// * `auth_method` - Authentication method to use
    /// * `group_id` - Optional group ID for organization
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::models::{Server, AuthMethod};
    ///
    /// let server = Server::new(
    ///     "Web Server".to_string(),
    ///     "192.168.1.1".to_string(),
    ///     22,
    ///     "root".to_string(),
    ///     AuthMethod::Agent,
    ///     None,
    /// );
    /// ```
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
    ///
    /// This is typically used when reconstructing a Server from persistent storage.
    #[allow(clippy::too_many_arguments)]
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
    ///
    /// # Arguments
    /// * `f` - Closure that performs the modifications
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
    ///
    /// Format: "Name (username@host:port)"
    pub fn summary(&self) -> String {
        format!(
            "{} ({}@{}:{})",
            self.name, self.username, self.host, self.port
        )
    }

    /// Check if server is in a group
    pub fn is_in_group(&self, group_id: &str) -> bool {
        self.group_id
            .as_ref()
            .map(|id| id == group_id)
            .unwrap_or(false)
    }

    /// Get the connection string (username@host:port)
    pub fn connection_string(&self) -> String {
        format!("{}@{}:{}", self.username, self.host, self.port)
    }

    /// Get a display label for UI lists
    pub fn display_label(&self) -> String {
        format!("{} - {}", self.name, self.connection_string())
    }

    /// Check if this server is using secure authentication
    ///
    /// Returns true if using SSH agent or key-based auth (not password)
    pub fn uses_secure_auth(&self) -> bool {
        !matches!(self.auth_method, AuthMethod::Password { .. })
    }

    /// Clone without sensitive data (for logging/display)
    ///
    /// Returns a clone with auth credentials redacted
    pub fn clone_redacted(&self) -> Self {
        let safe_auth = match &self.auth_method {
            AuthMethod::Password { .. } => AuthMethod::Password {
                password: "***".to_string(),
            },
            AuthMethod::PrivateKey { key_path, .. } => AuthMethod::PrivateKey {
                key_path: key_path.clone(),
                passphrase: Some("***".to_string()),
            },
            auth => auth.clone(),
        };

        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            host: self.host.clone(),
            port: self.port,
            username: self.username.clone(),
            auth_method: safe_auth,
            group_id: self.group_id.clone(),
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
            schema_version: self.schema_version,
        }
    }

    /// Convert to SSH command arguments
    ///
    /// Returns the arguments needed for `ssh` command
    pub fn to_ssh_args(&self) -> Vec<String> {
        let mut args = vec!["-p".to_string(), self.port.to_string()];

        if let Some(key_path) = self.identity_file() {
            args.push("-i".to_string());
            args.push(key_path);
        }

        args.push(format!("{}@{}", self.username, self.host));
        args
    }
}

impl Validatable for Server {
    fn validate(&self) -> Result<(), ValidationError> {
        let mut errors = Vec::new();

        // Validate name
        if self.name.trim().is_empty() {
            errors.push(ValidationError::invalid_field(
                "name",
                "Name cannot be empty",
            ));
        } else if self.name.len() > MAX_NAME_LENGTH {
            errors.push(ValidationError::invalid_field(
                "name",
                format!("Name too long (max {} characters)", MAX_NAME_LENGTH),
            ));
        }

        // Validate host
        if self.host.trim().is_empty() {
            errors.push(ValidationError::invalid_field(
                "host",
                "Host cannot be empty",
            ));
        } else if !is_valid_host(&self.host) {
            errors.push(ValidationError::invalid_format(
                "host",
                "valid hostname or IP address",
            ));
        }

        // Validate port
        if !validate_port(self.port) {
            errors.push(ValidationError::invalid_field(
                "port",
                format!("Invalid port: {}. Must be 1-65535", self.port),
            ));
        }

        // Validate username
        if self.username.trim().is_empty() {
            errors.push(ValidationError::invalid_field(
                "username",
                "Username cannot be empty",
            ));
        } else if self.username.len() > MAX_USERNAME_LENGTH {
            errors.push(ValidationError::invalid_field(
                "username",
                format!("Username too long (max {} characters)", MAX_USERNAME_LENGTH),
            ));
        } else if !is_valid_ssh_username(&self.username) {
            errors.push(ValidationError::invalid_format(
                "username",
                "valid SSH username (alphanumeric, underscore, hyphen, dot; must start with letter or underscore)",
            ));
        }

        // Validate auth method
        if let Err(e) = self.auth_method.validate() {
            errors.push(e);
        }

        ValidationError::combine(errors)
    }

    fn validate_all(&self) -> Result<(), ValidationError> {
        self.validate()
    }
}

/// Builder for creating Server instances
///
/// Provides a fluent API for constructing Server objects with validation.
///
/// # Example
///
/// ```
/// use easyssh_core::models::{ServerBuilder, AuthMethod};
///
/// let server = ServerBuilder::new()
///     .name("Production")
///     .host("192.168.1.1")
///     .port(2222)
///     .username("admin")
///     .auth_method(AuthMethod::Agent)
///     .build();
/// ```
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
    ///
    /// If not set, a new UUID will be generated.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the server name (required)
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the host (required)
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Set the port
    ///
    /// Defaults to 22 if not set.
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Set the username (required)
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the authentication method
    ///
    /// Defaults to `AuthMethod::Agent` if not set.
    pub fn auth_method(mut self, auth_method: AuthMethod) -> Self {
        self.auth_method = Some(auth_method);
        self
    }

    /// Set the group ID
    ///
    /// If `None` is passed, the group_id is cleared (ungrouped).
    pub fn group_id(mut self, group_id: Option<String>) -> Self {
        self.group_id = group_id;
        self
    }

    /// Set the status
    ///
    /// Defaults to `ServerStatus::Unknown` if not set.
    pub fn status(mut self, status: ServerStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the created_at timestamp
    ///
    /// Defaults to current time if not set.
    pub fn created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Set the updated_at timestamp
    ///
    /// Defaults to current time if not set.
    pub fn updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// Build the Server instance
    ///
    /// # Panics
    ///
    /// Panics if required fields (`name`, `host`, `username`) are not set.
    pub fn build(self) -> Server {
        let now = Utc::now();
        Server {
            id: self.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: self.name.expect("name is required"),
            host: self.host.expect("host is required"),
            port: self.port.unwrap_or(22),
            username: self.username.expect("username is required"),
            auth_method: self.auth_method.unwrap_or_default(),
            group_id: self.group_id,
            status: self.status.unwrap_or_default(),
            created_at: self.created_at.unwrap_or(now),
            updated_at: self.updated_at.unwrap_or(now),
            schema_version: 1,
        }
    }

    /// Build with validation
    ///
    /// Validates the server after construction and returns an error if invalid.
    pub fn build_validated(self) -> Result<Server, ValidationError> {
        let server = self.build();
        server.validate()?;
        Ok(server)
    }
}

/// Server data transfer object for creating new servers
///
/// Used for API requests and deserialization.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateServerDto {
    /// Display name for the server (required)
    pub name: String,
    /// Hostname or IP address (required)
    pub host: String,
    /// SSH port (default: 22)
    #[serde(default = "default_port_dto")]
    pub port: u16,
    /// Username for authentication (required)
    pub username: String,
    /// Authentication method (default: Agent)
    #[serde(default)]
    pub auth_method: AuthMethod,
    /// Optional group ID
    pub group_id: Option<String>,
}

fn default_port_dto() -> u16 {
    22
}

/// Server data transfer object for updating servers
///
/// All fields are optional, allowing partial updates.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateServerDto {
    /// New display name
    pub name: Option<String>,
    /// New host
    pub host: Option<String>,
    /// New port
    pub port: Option<u16>,
    /// New username
    pub username: Option<String>,
    /// New authentication method
    pub auth_method: Option<AuthMethod>,
    /// New group ID (None = ungrouped)
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
        assert!(!server.id.is_empty());
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
        let result = server.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field(), Some("name"));
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
        let result = server.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("name"));
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
        let result = server.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("port"));
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
        let result = server.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("host"));
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
        let result = server.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("username"));
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
        let result = server.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("username"));
    }

    #[test]
    fn test_server_validation_invalid_username_start() {
        let server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "123user".to_string(),
            AuthMethod::Agent,
            None,
        );
        let result = server.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("username"));
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
        let result = server.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("auth_method.key_path"));
    }

    #[test]
    fn test_server_validation_dangerous_key_path() {
        let server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::PrivateKey {
                key_path: "../etc/passwd".to_string(),
                passphrase: None,
            },
            None,
        );
        let result = server.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("auth_method.key_path"));
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

        let server2 = Server::new(
            "Test".to_string(),
            "2001:db8::1".to_string(),
            22,
            "user".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert!(server2.validate().is_ok());
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
        assert_eq!(
            AuthMethod::PrivateKey {
                key_path: "/path/to/key".to_string(),
                passphrase: None,
            }
            .to_string(),
            "key:/path/to/key"
        );
    }

    #[test]
    fn test_auth_method_auth_type() {
        assert_eq!(AuthMethod::Agent.auth_type(), "agent");
        assert_eq!(
            AuthMethod::Password {
                password: "secret".to_string()
            }
            .auth_type(),
            "password"
        );
        assert_eq!(
            AuthMethod::PrivateKey {
                key_path: "/path/to/key".to_string(),
                passphrase: None,
            }
            .auth_type(),
            "key"
        );
    }

    #[test]
    fn test_auth_method_db_string() {
        assert_eq!(AuthMethod::Agent.to_db_string(), "agent");
        assert_eq!(
            AuthMethod::Password {
                password: "secret".to_string()
            }
            .to_db_string(),
            "password"
        );
        assert_eq!(
            AuthMethod::PrivateKey {
                key_path: "/path/to/key".to_string(),
                passphrase: None,
            }
            .to_db_string(),
            "key:/path/to/key"
        );
    }

    #[test]
    fn test_auth_method_from_db_string() {
        assert!(matches!(
            AuthMethod::from_db_string("agent", None),
            AuthMethod::Agent
        ));
        assert!(matches!(
            AuthMethod::from_db_string("password", None),
            AuthMethod::Password { .. }
        ));

        let key_auth = AuthMethod::from_db_string("key", Some("/path/to/key"));
        assert!(matches!(key_auth, AuthMethod::PrivateKey { .. }));
    }

    #[test]
    fn test_auth_method_needs_refresh() {
        assert!(!AuthMethod::Agent.needs_refresh());
        assert!(AuthMethod::Password {
            password: "".to_string()
        }
        .needs_refresh());
        assert!(!AuthMethod::PrivateKey {
            key_path: "".to_string(),
            passphrase: None,
        }
        .needs_refresh());
    }

    #[test]
    fn test_auth_method_stores_credentials() {
        assert!(!AuthMethod::Agent.stores_credentials());
        assert!(AuthMethod::Password {
            password: "".to_string()
        }
        .stores_credentials());
        assert!(AuthMethod::PrivateKey {
            key_path: "".to_string(),
            passphrase: None,
        }
        .stores_credentials());
    }

    #[test]
    fn test_auth_method_default() {
        assert!(matches!(AuthMethod::default(), AuthMethod::Agent));
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
        assert_eq!(server.host, "example.com");
        assert_eq!(server.port, 2222);
        assert_eq!(server.username, "admin");
    }

    #[test]
    fn test_server_builder_validated() {
        let result = ServerBuilder::new()
            .name("Test")
            .host("invalid host with spaces")
            .username("admin")
            .build_validated();

        assert!(result.is_err());

        let valid = ServerBuilder::new()
            .name("Test")
            .host("192.168.1.1")
            .username("admin")
            .build_validated();

        assert!(valid.is_ok());
    }

    #[test]
    fn test_server_builder_defaults() {
        let server = ServerBuilder::new()
            .name("Test")
            .host("example.com")
            .username("user")
            .build();

        assert_eq!(server.port, 22); // Default port
        assert!(matches!(server.auth_method, AuthMethod::Agent)); // Default auth
        assert!(matches!(server.status, ServerStatus::Unknown)); // Default status
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
        assert_eq!(
            ServerStatus::from_status_str("online"),
            ServerStatus::Online
        );
        assert_eq!(
            ServerStatus::from_status_str("invalid"),
            ServerStatus::Unknown
        );

        assert!(ServerStatus::Unknown.is_connectable());
        assert!(ServerStatus::Online.is_connectable());
        assert!(!ServerStatus::Connecting.is_connectable());

        assert!(!ServerStatus::Unknown.is_connecting());
        assert!(ServerStatus::Connecting.is_connecting());
    }

    #[test]
    fn test_server_status_default() {
        assert!(matches!(ServerStatus::default(), ServerStatus::Unknown));
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
    fn test_server_summary() {
        let server = Server::new(
            "Production".to_string(),
            "192.168.1.1".to_string(),
            22,
            "admin".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert_eq!(server.summary(), "Production (admin@192.168.1.1:22)");
    }

    #[test]
    fn test_server_display_label() {
        let server = Server::new(
            "Web Server".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert_eq!(server.display_label(), "Web Server - root@192.168.1.1:22");
    }

    #[test]
    fn test_server_is_in_group() {
        let mut server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::Agent,
            None,
        );

        assert!(!server.is_in_group("group-1"));

        server.group_id = Some("group-1".to_string());
        assert!(server.is_in_group("group-1"));
        assert!(!server.is_in_group("group-2"));
    }

    #[test]
    fn test_server_identity_file() {
        let server_with_key = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::PrivateKey {
                key_path: "/path/to/key".to_string(),
                passphrase: None,
            },
            None,
        );
        assert_eq!(
            server_with_key.identity_file(),
            Some("/path/to/key".to_string())
        );

        let server_with_agent = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert_eq!(server_with_agent.identity_file(), None);
    }

    #[test]
    fn test_server_uses_secure_auth() {
        let password_server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::Password {
                password: "secret".to_string(),
            },
            None,
        );
        assert!(!password_server.uses_secure_auth());

        let key_server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::PrivateKey {
                key_path: "/path/to/key".to_string(),
                passphrase: None,
            },
            None,
        );
        assert!(key_server.uses_secure_auth());

        let agent_server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::Agent,
            None,
        );
        assert!(agent_server.uses_secure_auth());
    }

    #[test]
    fn test_server_clone_redacted() {
        let server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "root".to_string(),
            AuthMethod::Password {
                password: "supersecret".to_string(),
            },
            None,
        );

        let redacted = server.clone_redacted();
        match redacted.auth_method {
            AuthMethod::Password { password } => {
                assert_eq!(password, "***");
            }
            _ => panic!("Expected Password auth method"),
        }
    }

    #[test]
    fn test_server_to_ssh_args() {
        let server = Server::new(
            "Test".to_string(),
            "192.168.1.1".to_string(),
            2222,
            "root".to_string(),
            AuthMethod::PrivateKey {
                key_path: "/home/user/.ssh/id_rsa".to_string(),
                passphrase: None,
            },
            None,
        );

        let args = server.to_ssh_args();
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"2222".to_string()));
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"/home/user/.ssh/id_rsa".to_string()));
        assert!(args.contains(&"root@192.168.1.1".to_string()));
    }

    #[test]
    fn test_server_serialization() {
        let server = Server::new(
            "Test Server".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            AuthMethod::Agent,
            Some("group-1".to_string()),
        );

        let json = serde_json::to_string(&server).unwrap();
        assert!(json.contains("Test Server"));
        assert!(json.contains("192.168.1.1"));
        assert!(json.contains("agent"));

        let deserialized: Server = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, server.name);
        assert_eq!(deserialized.host, server.host);
        assert_eq!(deserialized.port, server.port);
    }

    #[test]
    fn test_server_serialization_with_auth() {
        let server = Server::new(
            "Test".to_string(),
            "example.com".to_string(),
            22,
            "user".to_string(),
            AuthMethod::PrivateKey {
                key_path: "/path/to/key".to_string(),
                passphrase: Some("secret".to_string()),
            },
            None,
        );

        let json = serde_json::to_string(&server).unwrap();
        let deserialized: Server = serde_json::from_str(&json).unwrap();

        match deserialized.auth_method {
            AuthMethod::PrivateKey {
                key_path,
                passphrase,
            } => {
                assert_eq!(key_path, "/path/to/key");
                assert_eq!(passphrase, Some("secret".to_string()));
            }
            _ => panic!("Expected PrivateKey auth method"),
        }
    }

    #[test]
    fn test_server_deserialization_missing_fields() {
        // Test with minimal fields and defaults
        let json = r#"{
            "id": "test-id",
            "name": "Test",
            "host": "example.com",
            "username": "root",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let server: Server = serde_json::from_str(json).unwrap();
        assert_eq!(server.port, 22); // Default port
        assert!(matches!(server.auth_method, AuthMethod::Agent)); // Default auth
        assert!(matches!(server.status, ServerStatus::Unknown)); // Default status
    }

    #[test]
    fn test_create_server_dto() {
        let json = r##"{
            "name": "Production",
            "host": "192.168.1.100",
            "port": 2222,
            "username": "admin",
            "auth_method": {"type": "agent"},
            "group_id": "group-1"
        }"##;

        let dto: CreateServerDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.name, "Production");
        assert_eq!(dto.host, "192.168.1.100");
        assert_eq!(dto.port, 2222);
        assert_eq!(dto.username, "admin");
        assert!(dto.group_id.is_some());
    }

    #[test]
    fn test_create_server_dto_defaults() {
        let json = r##"{
            "name": "Minimal",
            "host": "example.com",
            "username": "root"
        }"##;

        let dto: CreateServerDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.port, 22); // Default port
        assert!(matches!(dto.auth_method, AuthMethod::Agent)); // Default auth
    }

    #[test]
    fn test_update_server_dto() {
        let json = r##"{
            "name": "Updated Name",
            "port": 2222
        }"##;

        let dto: UpdateServerDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.name, Some("Updated Name".to_string()));
        assert_eq!(dto.port, Some(2222));
        assert_eq!(dto.host, None);
    }

    #[test]
    fn test_update_server_dto_group_id() {
        // Setting group_id to a value
        let json = r##"{
            "group_id": "new-group"
        }"##;
        let dto: UpdateServerDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.group_id, Some(Some("new-group".to_string())));

        // Setting group_id to null (ungroup)
        let json = r##"{
            "group_id": null
        }"##;
        let dto: UpdateServerDto = serde_json::from_str(json).unwrap();
        // Note: serde deserializes null as None for Option<Option<T>>, not Some(None)
        assert_eq!(dto.group_id, None);
    }

    #[test]
    fn test_server_with_id() {
        let created_at = Utc::now();
        let updated_at = Utc::now();

        let server = Server::with_id(
            "specific-id".to_string(),
            "Test".to_string(),
            "example.com".to_string(),
            2222,
            "admin".to_string(),
            AuthMethod::Agent,
            Some("group-1".to_string()),
            ServerStatus::Online,
            created_at,
            updated_at,
        );

        assert_eq!(server.id, "specific-id");
        assert_eq!(server.port, 2222);
        assert!(matches!(server.status, ServerStatus::Online));
    }

    #[test]
    fn test_server_set_status() {
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

        server.set_status(ServerStatus::Online);
        assert!(matches!(server.status, ServerStatus::Online));
        assert!(server.updated_at > old_updated);
    }
}
