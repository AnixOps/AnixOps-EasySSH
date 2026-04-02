//! Database models
//!
//! This module defines the data structures used for database operations,
//! including servers, groups, and application configuration.

use chrono::{DateTime, Utc};
use sqlx::FromRow;

/// Server record stored in the database
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct Server {
    /// Unique identifier (UUID)
    pub id: String,

    /// Display name
    pub name: String,

    /// Hostname or IP address
    pub host: String,

    /// SSH port (default: 22)
    pub port: i64,

    /// SSH username
    pub username: String,

    /// Authentication method: 'password' or 'key'
    pub auth_method: String,

    /// Encrypted credentials (password or private key)
    pub encrypted_credentials: Vec<u8>,

    /// Optional group ID for organization
    pub group_id: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// New server to be inserted into the database
#[derive(Debug, Clone)]
pub struct NewServer {
    /// Unique identifier (UUID)
    pub id: String,

    /// Display name
    pub name: String,

    /// Hostname or IP address
    pub host: String,

    /// SSH port (default: 22)
    pub port: u16,

    /// SSH username
    pub username: String,

    /// Authentication method: 'password' or 'key'
    pub auth_method: String,

    /// Encrypted credentials
    pub encrypted_credentials: Vec<u8>,

    /// Optional group ID
    pub group_id: Option<String>,
}

impl NewServer {
    /// Validate the new server data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - id is empty
    /// - name is empty
    /// - host is empty
    /// - username is empty
    /// - port is 0
    /// - auth_method is not 'password' or 'key'
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Server ID cannot be empty".to_string());
        }
        if self.name.is_empty() {
            return Err("Server name cannot be empty".to_string());
        }
        if self.host.is_empty() {
            return Err("Server host cannot be empty".to_string());
        }
        if self.username.is_empty() {
            return Err("Server username cannot be empty".to_string());
        }
        if self.port == 0 {
            return Err("Server port cannot be 0".to_string());
        }
        if self.auth_method != "password" && self.auth_method != "key" {
            return Err("Auth method must be 'password' or 'key'".to_string());
        }
        Ok(())
    }
}

/// Update data for an existing server
#[derive(Debug, Clone, Default)]
pub struct UpdateServer {
    /// Server ID to update
    pub id: String,

    /// New display name (None = no change)
    pub name: Option<String>,

    /// New hostname or IP (None = no change)
    pub host: Option<String>,

    /// New port (None = no change)
    pub port: Option<u16>,

    /// New username (None = no change)
    pub username: Option<String>,

    /// New auth method (None = no change)
    pub auth_method: Option<String>,

    /// New encrypted credentials (None = no change)
    pub encrypted_credentials: Option<Vec<u8>>,

    /// New group ID (None = no change, Some(None) = remove from group)
    pub group_id: Option<Option<String>>,
}

/// Group record stored in the database
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct Group {
    /// Unique identifier (UUID)
    pub id: String,

    /// Display name
    pub name: String,

    /// Color code for UI (hex format, e.g., "#4A90D9")
    pub color: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// New group to be inserted into the database
#[derive(Debug, Clone)]
pub struct NewGroup {
    /// Unique identifier (UUID)
    pub id: String,

    /// Display name
    pub name: String,

    /// Color code for UI (optional, defaults to "#4A90D9")
    pub color: Option<String>,
}

impl NewGroup {
    /// Default color for new groups
    pub const DEFAULT_COLOR: &'static str = "#4A90D9";

    /// Validate the new group data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - id is empty
    /// - name is empty
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Group ID cannot be empty".to_string());
        }
        if self.name.is_empty() {
            return Err("Group name cannot be empty".to_string());
        }
        Ok(())
    }

    /// Get the color with default fallback
    pub fn color(&self) -> &str {
        self.color.as_deref().unwrap_or(Self::DEFAULT_COLOR)
    }
}

/// Update data for an existing group
#[derive(Debug, Clone, Default)]
pub struct UpdateGroup {
    /// Group ID to update
    pub id: String,

    /// New display name (None = no change)
    pub name: Option<String>,

    /// New color code (None = no change)
    pub color: Option<String>,
}

/// Application configuration entry
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct AppConfig {
    /// Configuration key
    pub key: String,

    /// Configuration value
    pub value: String,
}

/// Server with group information (for joined queries)
#[derive(Debug, Clone, FromRow)]
pub struct ServerWithGroup {
    /// Server fields
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_method: String,
    pub encrypted_credentials: Vec<u8>,
    pub server_created_at: DateTime<Utc>,
    pub server_updated_at: DateTime<Utc>,

    /// Group fields (optional)
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub group_color: Option<String>,
}

/// Query filters for server listing
#[derive(Debug, Clone, Default)]
pub struct ServerFilters {
    /// Filter by group ID
    pub group_id: Option<String>,

    /// Search by name or host (case-insensitive)
    pub search: Option<String>,
}

/// Query options for pagination and sorting
#[derive(Debug, Clone)]
pub struct QueryOptions {
    /// Number of records to return
    pub limit: Option<i64>,

    /// Number of records to skip
    pub offset: Option<i64>,

    /// Sort column
    pub sort_by: String,

    /// Sort direction
    pub sort_asc: bool,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            limit: None,
            offset: None,
            sort_by: "created_at".to_string(),
            sort_asc: false, // Newest first
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_server_validation() {
        let valid = NewServer {
            id: "test".to_string(),
            name: "Test".to_string(),
            host: "localhost".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_method: "password".to_string(),
            encrypted_credentials: vec![1, 2, 3],
            group_id: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = NewServer {
            id: "".to_string(),
            name: "Test".to_string(),
            host: "localhost".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_method: "password".to_string(),
            encrypted_credentials: vec![1, 2, 3],
            group_id: None,
        };
        assert!(invalid.validate().is_err());

        let invalid_port = NewServer {
            id: "test".to_string(),
            name: "Test".to_string(),
            host: "localhost".to_string(),
            port: 0,
            username: "admin".to_string(),
            auth_method: "password".to_string(),
            encrypted_credentials: vec![1, 2, 3],
            group_id: None,
        };
        assert!(invalid_port.validate().is_err());

        let invalid_auth = NewServer {
            id: "test".to_string(),
            name: "Test".to_string(),
            host: "localhost".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_method: "invalid".to_string(),
            encrypted_credentials: vec![1, 2, 3],
            group_id: None,
        };
        assert!(invalid_auth.validate().is_err());
    }

    #[test]
    fn test_new_group_validation() {
        let valid = NewGroup {
            id: "test".to_string(),
            name: "Test Group".to_string(),
            color: Some("#FF0000".to_string()),
        };
        assert!(valid.validate().is_ok());
        assert_eq!(valid.color(), "#FF0000");

        let with_default_color = NewGroup {
            id: "test".to_string(),
            name: "Test Group".to_string(),
            color: None,
        };
        assert_eq!(with_default_color.color(), NewGroup::DEFAULT_COLOR);

        let invalid = NewGroup {
            id: "".to_string(),
            name: "Test".to_string(),
            color: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_query_options_default() {
        let opts = QueryOptions::default();
        assert_eq!(opts.sort_by, "created_at");
        assert!(!opts.sort_asc);
        assert!(opts.limit.is_none());
        assert!(opts.offset.is_none());
    }
}
