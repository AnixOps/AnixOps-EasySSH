//! EasySSH Data Models
//!
//! This module provides the core data structures used throughout the EasySSH application.
//! Models are organized by domain and provide serialization support via serde.
//!
//! # Model Versions
//!
//! Each model includes a `schema_version` field for forward compatibility:
//! - Version 1: Initial schema
//! - Version 2+: Future migrations

pub mod connection;
pub mod group;
pub mod server;
pub mod settings;
pub mod user;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

// Re-export commonly used types
pub use connection::{
    Connection, ConnectionFilter, ConnectionHistory, ConnectionRecord, ConnectionStatus,
    CreateConnectionDto, UpdateConnectionDto,
};

pub use group::{
    CreateGroupRequest, Group, GroupId, GroupStats, GroupWithServers, MoveServerRequest,
    ServerReference, UpdateGroupRequest, DEFAULT_COLOR_PALETTE, PRESET_GROUPS, UNGROUPED_COLOR,
    UNGROUPED_ID, UNGROUPED_NAME,
};

pub use server::{
    AuthMethod, CreateServerDto, Server, ServerBuilder, ServerStatus, UpdateServerDto,
};

pub use settings::{
    AppearanceSettings, ApplicationSettings, BackupSettings, CreateSettingsDto,
    EncryptionSettings, NetworkSettings, SecuritySettings, Settings, SettingsBuilder,
    TerminalSettings, UpdateSettingsDto,
};

pub use user::{
    CreateUserDto, LocalUser, UpdateUserDto, User, UserPreferences, UserProfile, UserStatus,
};

/// Current schema version for all models
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Validation error for model data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationError {
    /// Field is empty or invalid
    InvalidField { field: String, message: String },
    /// Value is out of range
    OutOfRange { field: String, min: i64, max: i64, actual: i64 },
    /// Format is invalid
    InvalidFormat { field: String, expected: String },
    /// Required field is missing
    MissingField(String),
    /// Custom validation error
    Custom(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InvalidField { field, message } => {
                write!(f, "Invalid field '{}': {}", field, message)
            }
            ValidationError::OutOfRange {
                field,
                min,
                max,
                actual,
            } => {
                write!(
                    f,
                    "Field '{}' out of range: {} not in [{}..{}]",
                    field, actual, min, max
                )
            }
            ValidationError::InvalidFormat { field, expected } => {
                write!(f, "Field '{}' invalid format, expected: {}", field, expected)
            }
            ValidationError::MissingField(field) => {
                write!(f, "Missing required field: {}", field)
            }
            ValidationError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Trait for models that can be validated
pub trait Validatable {
    /// Validate the model data
    ///
    /// Returns Ok(()) if all fields are valid, or Err with validation details
    fn validate(&self) -> Result<(), ValidationError>;
}

/// Trait for models with schema versioning
pub trait Versioned {
    /// Get the schema version of this model
    fn schema_version(&self) -> u32;

    /// Check if this model needs migration
    fn needs_migration(&self) -> bool {
        self.schema_version() < CURRENT_SCHEMA_VERSION
    }

    /// Migrate this model to the current schema version
    /// Returns a new instance with updated schema version
    fn migrate(&self) -> Self
    where
        Self: Sized + Clone;
}

/// Trait for models with timestamps
pub trait Timestamped {
    fn created_at(&self) -> DateTime<Utc>;
    fn updated_at(&self) -> DateTime<Utc>;
    fn touch(&mut self);
}

/// Default port for SSH connections
pub const DEFAULT_SSH_PORT: u16 = 22;

/// Default timeout for connections (in seconds)
pub const DEFAULT_CONNECTION_TIMEOUT: u64 = 30;

/// Default heartbeat interval (in seconds)
pub const DEFAULT_HEARTBEAT_INTERVAL: u64 = 30;

/// Maximum length for display names
pub const MAX_NAME_LENGTH: usize = 100;

/// Maximum length for usernames
pub const MAX_USERNAME_LENGTH: usize = 32;

/// Maximum length for hostnames
pub const MAX_HOSTNAME_LENGTH: usize = 253;

/// Validates a hostname according to RFC 1123
pub fn is_valid_hostname(hostname: &str) -> bool {
    if hostname.is_empty() || hostname.len() > MAX_HOSTNAME_LENGTH {
        return false;
    }

    // Check each label
    let labels: Vec<&str> = hostname.split('.').collect();
    for label in labels {
        if !is_valid_hostname_label(label) {
            return false;
        }
    }

    true
}

/// Validates a single hostname label
fn is_valid_hostname_label(label: &str) -> bool {
    if label.is_empty() || label.len() > 63 {
        return false;
    }

    // Must start and end with alphanumeric character
    let chars: Vec<char> = label.chars().collect();
    if !chars[0].is_alphanumeric() || !chars[chars.len() - 1].is_alphanumeric() {
        return false;
    }

    // Middle characters can be alphanumeric or hyphen
    for &c in &chars[1..chars.len() - 1] {
        if !c.is_alphanumeric() && c != '-' {
            return false;
        }
    }

    true
}

/// Validates an IP address (IPv4 or IPv6)
pub fn is_valid_ip(ip: &str) -> bool {
    ip.parse::<std::net::IpAddr>().is_ok()
}

/// Validates a host string (IP or hostname)
pub fn is_valid_host(host: &str) -> bool {
    let host = host.trim();
    is_valid_ip(host) || is_valid_hostname(host)
}

/// Validates a port number
pub fn is_valid_port(port: u16) -> bool {
    port > 0 && port <= 65535
}

/// Validates a hex color string
pub fn is_valid_hex_color(color: &str) -> bool {
    if !color.starts_with('#') {
        return false;
    }

    let hex_part = &color[1..];
    let len = hex_part.len();

    // Support both 3-digit and 6-digit hex colors
    if len != 3 && len != 6 {
        return false;
    }

    hex_part.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::InvalidField {
            field: "name".to_string(),
            message: "cannot be empty".to_string(),
        };
        assert!(err.to_string().contains("name"));
        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_versioned_trait() {
        struct TestModel {
            version: u32,
        }

        impl Versioned for TestModel {
            fn schema_version(&self) -> u32 {
                self.version
            }

            fn migrate(&self) -> Self
            where
                Self: Clone,
            {
                Self {
                    version: CURRENT_SCHEMA_VERSION,
                }
            }
        }

        let old_model = TestModel { version: 0 };
        assert!(old_model.needs_migration());
        assert_eq!(old_model.schema_version(), 0);

        let migrated = old_model.migrate();
        assert_eq!(migrated.schema_version(), CURRENT_SCHEMA_VERSION);
        assert!(!migrated.needs_migration());
    }

    #[test]
    fn test_is_valid_hostname() {
        assert!(is_valid_hostname("example.com"));
        assert!(is_valid_hostname("server.example.com"));
        assert!(is_valid_hostname("localhost"));
        assert!(is_valid_hostname("a-b-c.example"));

        assert!(!is_valid_hostname(""));
        assert!(!is_valid_hostname("-example.com"));
        assert!(!is_valid_hostname("example-.com"));
        assert!(!is_valid_hostname("example..com"));
    }

    #[test]
    fn test_is_valid_ip() {
        assert!(is_valid_ip("192.168.1.1"));
        assert!(is_valid_ip("::1"));
        assert!(is_valid_ip("0:0:0:0:0:0:0:1"));

        assert!(!is_valid_ip("999.999.999.999"));
        assert!(!is_valid_ip("not-an-ip"));
    }

    #[test]
    fn test_is_valid_host() {
        assert!(is_valid_host("example.com"));
        assert!(is_valid_host("192.168.1.1"));
        assert!(is_valid_host("::1"));

        assert!(!is_valid_host(""));
        assert!(!is_valid_host("   "));
    }

    #[test]
    fn test_is_valid_port() {
        assert!(is_valid_port(1));
        assert!(is_valid_port(22));
        assert!(is_valid_port(443));
        assert!(is_valid_port(65535));

        assert!(!is_valid_port(0));
        assert!(!is_valid_port(65536));
    }

    #[test]
    fn test_is_valid_hex_color() {
        assert!(is_valid_hex_color("#FFF"));
        assert!(is_valid_hex_color("#fff"));
        assert!(is_valid_hex_color("#4A90D9"));
        assert!(is_valid_hex_color("#4a90d9"));

        assert!(!is_valid_hex_color("4A90D9"));
        assert!(!is_valid_hex_color("#GGGGGG"));
        assert!(!is_valid_hex_color("#4A90D"));
        assert!(!is_valid_hex_color("#4A90D99"));
    }

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_SSH_PORT, 22);
        assert_eq!(CURRENT_SCHEMA_VERSION, 1);
        assert_eq!(MAX_NAME_LENGTH, 100);
        assert_eq!(MAX_USERNAME_LENGTH, 32);
    }
}
