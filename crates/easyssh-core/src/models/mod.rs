//! EasySSH Data Models
//!
//! This module provides the core data structures used throughout the EasySSH application.
//! Models are organized by domain and provide serialization support via serde.
//!
//! # Architecture
//!
//! The models follow a consistent pattern:
//! - Each model implements [`Validatable`] for data integrity
//! - Each model implements [`Versioned`] for schema migration support
//! - Each model implements [`Timestamped`] for audit trails
//! - DTOs are provided for create/update operations
//!
//! # Model Versions
//!
//! Each model includes a `schema_version` field for forward compatibility:
//! - Version 1: Initial schema (current)
//! - Version 2+: Future migrations
//!
//! # Example
//!
//! ```rust
//! use easyssh_core::models::{Server, AuthMethod, Validatable};
//!
//! let server = Server::new(
//!     "Production".to_string(),
//!     "192.168.1.1".to_string(),
//!     22,
//!     "root".to_string(),
//!     AuthMethod::Agent,
//!     None,
//! );
//!
//! assert!(server.validate().is_ok());
//! ```

pub mod connection;
pub mod group;
pub mod server;
pub mod settings;
pub mod user;
pub mod validation;

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
    AppearanceSettings, ApplicationSettings, BackupSettings, CreateSettingsDto, EncryptionSettings,
    NetworkSettings, SecuritySettings, Settings, SettingsBuilder, TerminalSettings,
    UpdateSettingsDto,
};

pub use user::{
    CreateUserDto, LocalUser, UpdateUserDto, User, UserPreferences, UserProfile, UserStatus,
};

pub use validation::{
    is_valid_email, is_valid_ssh_key_path, sanitize_filename,
    validate_password_strength, PasswordStrength, ValidationResult,
};

/// Current schema version for all models
///
/// This is incremented when making breaking changes to any model schema.
/// Use the [`Versioned`] trait to check and perform migrations.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Minimum schema version supported for migrations
pub const MIN_SCHEMA_VERSION: u32 = 1;

/// Maximum length for display names
pub const MAX_NAME_LENGTH: usize = 100;

/// Maximum length for usernames (SSH usernames typically limited to 32 chars)
pub const MAX_USERNAME_LENGTH: usize = 32;

/// Maximum length for hostnames (RFC 1123)
pub const MAX_HOSTNAME_LENGTH: usize = 253;

/// Maximum length for descriptions
pub const MAX_DESCRIPTION_LENGTH: usize = 500;

/// Default port for SSH connections
pub const DEFAULT_SSH_PORT: u16 = 22;

/// Default timeout for connections (in seconds)
pub const DEFAULT_CONNECTION_TIMEOUT: u64 = 30;

/// Default heartbeat interval (in seconds)
pub const DEFAULT_HEARTBEAT_INTERVAL: u64 = 30;

/// Maximum port number
pub const MAX_PORT: u16 = 65535;

/// Validation error for model data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationError {
    /// Field is empty or invalid
    InvalidField { field: String, message: String },
    /// Value is out of range
    OutOfRange {
        field: String,
        min: i64,
        max: i64,
        actual: i64,
    },
    /// Format is invalid
    InvalidFormat { field: String, expected: String },
    /// Required field is missing
    MissingField(String),
    /// Duplicate value
    Duplicate { field: String, value: String },
    /// Constraint violation
    ConstraintViolation { constraint: String, message: String },
    /// Multiple validation errors
    Multiple(Vec<ValidationError>),
    /// Custom validation error
    Custom(String),
}

impl ValidationError {
    /// Create a new invalid field error
    pub fn invalid_field(field: impl Into<String>, message: impl Into<String>) -> Self {
        ValidationError::InvalidField {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a new missing field error
    pub fn missing_field(field: impl Into<String>) -> Self {
        ValidationError::MissingField(field.into())
    }

    /// Create a new format error
    pub fn invalid_format(field: impl Into<String>, expected: impl Into<String>) -> Self {
        ValidationError::InvalidFormat {
            field: field.into(),
            expected: expected.into(),
        }
    }

    /// Create a new out of range error
    pub fn out_of_range(
        field: impl Into<String>,
        min: i64,
        max: i64,
        actual: i64,
    ) -> Self {
        ValidationError::OutOfRange {
            field: field.into(),
            min,
            max,
            actual,
        }
    }

    /// Combine multiple validation errors into one
    pub fn combine(errors: Vec<ValidationError>) -> Result<(), ValidationError> {
        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.into_iter().next().unwrap())
        } else {
            Err(ValidationError::Multiple(errors))
        }
    }

    /// Get the field name if this is a field-related error
    pub fn field(&self) -> Option<&str> {
        match self {
            ValidationError::InvalidField { field, .. } => Some(field),
            ValidationError::OutOfRange { field, .. } => Some(field),
            ValidationError::InvalidFormat { field, .. } => Some(field),
            ValidationError::MissingField(field) => Some(field),
            ValidationError::Duplicate { field, .. } => Some(field),
            _ => None,
        }
    }
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
                write!(
                    f,
                    "Field '{}' invalid format, expected: {}",
                    field, expected
                )
            }
            ValidationError::MissingField(field) => {
                write!(f, "Missing required field: {}", field)
            }
            ValidationError::Duplicate { field, value } => {
                write!(f, "Duplicate value for field '{}': {}", field, value)
            }
            ValidationError::ConstraintViolation { constraint, message } => {
                write!(f, "Constraint '{}' violated: {}", constraint, message)
            }
            ValidationError::Multiple(errors) => {
                write!(f, "Multiple validation errors ({}):", errors.len())?;
                for (i, err) in errors.iter().enumerate() {
                    write!(f, " [{}] {}", i + 1, err)?;
                }
                Ok(())
            }
            ValidationError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ValidationError::Multiple(errors) => errors.first().map(|e| e as _),
            _ => None,
        }
    }
}

/// Trait for models that can be validated
///
/// Implement this trait to provide validation logic for your data models.
/// Validation should check all business rules and constraints.
///
/// # Example
///
/// ```rust
/// use easyssh_core::models::{Validatable, ValidationError};
///
/// struct MyModel {
///     name: String,
/// }
///
/// impl Validatable for MyModel {
///     fn validate(&self) -> Result<(), ValidationError> {
///         if self.name.trim().is_empty() {
///             return Err(ValidationError::invalid_field("name", "cannot be empty"));
///         }
///         Ok(())
///     }
/// }
/// ```
pub trait Validatable {
    /// Validate the model data
    ///
    /// Returns `Ok(())` if all fields are valid, or `Err` with validation details.
    /// Implementations should check all validation rules and return the first error
    /// encountered, or use `ValidationError::combine` to collect all errors.
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` describing what validation failed and why.
    fn validate(&self) -> Result<(), ValidationError>;

    /// Validate with all errors collected
    ///
    /// This method collects all validation errors rather than returning on the first one.
    fn validate_all(&self) -> Result<(), ValidationError> {
        self.validate()
    }
}

/// Trait for models with schema versioning
///
/// This trait enables forward compatibility by allowing models to be migrated
/// from older schema versions to the current version.
///
/// # Example
///
/// ```rust
/// use easyssh_core::models::{Versioned, CURRENT_SCHEMA_VERSION};
///
/// struct MyModel {
///     schema_version: u32,
///     // ... other fields
/// }
///
/// impl Versioned for MyModel {
///     fn schema_version(&self) -> u32 {
///         self.schema_version
///     }
///
///     fn migrate(&self) -> Self where Self: Clone {
///         let mut migrated = self.clone();
///         migrated.schema_version = CURRENT_SCHEMA_VERSION;
///         migrated
///     }
/// }
/// ```
pub trait Versioned {
    /// Get the schema version of this model
    fn schema_version(&self) -> u32;

    /// Check if this model needs migration to current schema
    ///
    /// Returns `true` if the model's schema version is older than current.
    fn needs_migration(&self) -> bool {
        self.schema_version() < CURRENT_SCHEMA_VERSION
    }

    /// Check if this model version is supported
    ///
    /// Returns `true` if the model can be migrated to current schema.
    fn is_supported_version(&self) -> bool {
        let version = self.schema_version();
        version >= MIN_SCHEMA_VERSION && version <= CURRENT_SCHEMA_VERSION
    }

    /// Migrate this model to the current schema version
    ///
    /// Returns a new instance with updated schema version and any necessary
    /// data transformations.
    ///
    /// # Panics
    ///
    /// May panic if the model version is too old to be migrated.
    fn migrate(&self) -> Self
    where
        Self: Sized + Clone;
}

/// Trait for models with timestamps
///
/// Provides consistent timestamp handling across all models.
pub trait Timestamped {
    /// Get the creation timestamp
    fn created_at(&self) -> DateTime<Utc>;

    /// Get the last update timestamp
    fn updated_at(&self) -> DateTime<Utc>;

    /// Update the `updated_at` timestamp to current time
    fn touch(&mut self);
}

/// Trait for models that can be softly deleted
///
/// Soft deletion marks records as deleted without removing them from the database.
pub trait SoftDeletable {
    /// Check if the record is deleted
    fn is_deleted(&self) -> bool;

    /// Mark the record as deleted
    fn mark_deleted(&mut self);

    /// Restore a deleted record
    fn restore(&mut self);
}

/// Validates a hostname according to RFC 1123
///
/// # Examples
///
/// ```
/// use easyssh_core::models::is_valid_hostname;
///
/// assert!(is_valid_hostname("example.com"));
/// assert!(is_valid_hostname("server.example.com"));
/// assert!(is_valid_hostname("localhost"));
///
/// assert!(!is_valid_hostname(""));
/// assert!(!is_valid_hostname("-example.com"));
/// ```
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
///
/// # Examples
///
/// ```
/// use easyssh_core::models::is_valid_ip;
///
/// assert!(is_valid_ip("192.168.1.1"));
/// assert!(is_valid_ip("::1"));
/// assert!(is_valid_ip("2001:db8::1"));
///
/// assert!(!is_valid_ip("999.999.999.999"));
/// assert!(!is_valid_ip("not-an-ip"));
/// ```
pub fn is_valid_ip(ip: &str) -> bool {
    ip.parse::<std::net::IpAddr>().is_ok()
}

/// Validates a host string (IP or hostname)
///
/// # Examples
///
/// ```
/// use easyssh_core::models::is_valid_host;
///
/// assert!(is_valid_host("example.com"));
/// assert!(is_valid_host("192.168.1.1"));
/// assert!(is_valid_host("::1"));
///
/// assert!(!is_valid_host(""));
/// assert!(!is_valid_host("   "));
/// ```
pub fn is_valid_host(host: &str) -> bool {
    let host = host.trim();
    is_valid_ip(host) || is_valid_hostname(host)
}

/// Validates a port number
///
/// Valid ports are 1-65535 (0 is reserved/invalid for SSH)
///
/// # Examples
///
/// ```
/// use easyssh_core::models::is_valid_port;
///
/// assert!(is_valid_port(1));
/// assert!(is_valid_port(22));
/// assert!(is_valid_port(443));
/// assert!(is_valid_port(65535));
///
/// assert!(!is_valid_port(0));
/// assert!(!is_valid_port(65536));
/// ```
pub fn is_valid_port(port: u16) -> bool {
    port > 0 && port <= MAX_PORT
}

/// Validates a hex color string
///
/// Supports both 3-digit and 6-digit hex colors.
///
/// # Examples
///
/// ```
/// use easyssh_core::models::is_valid_hex_color;
///
/// assert!(is_valid_hex_color("#FFF"));
/// assert!(is_valid_hex_color("#4A90D9"));
/// assert!(is_valid_hex_color("#4a90d9"));
///
/// assert!(!is_valid_hex_color("4A90D9"));  // Missing #
/// assert!(!is_valid_hex_color("#GGGGGG")); // Invalid hex
/// ```
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

/// Validates that a string is a valid UUID
pub fn is_valid_uuid(uuid: &str) -> bool {
    uuid::Uuid::parse_str(uuid).is_ok()
}

/// Sanitize a string to be safe for use as a filename
pub fn sanitize_for_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect()
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
    fn test_validation_error_helpers() {
        let err = ValidationError::invalid_field("test", "message");
        assert!(matches!(err, ValidationError::InvalidField { .. }));

        let err = ValidationError::missing_field("test");
        assert!(matches!(err, ValidationError::MissingField(..)));

        let err = ValidationError::invalid_format("test", "expected");
        assert!(matches!(err, ValidationError::InvalidFormat { .. }));

        let err = ValidationError::out_of_range("test", 0, 100, 150);
        assert!(matches!(err, ValidationError::OutOfRange { .. }));
    }

    #[test]
    fn test_validation_error_field() {
        let err = ValidationError::invalid_field("name", "error");
        assert_eq!(err.field(), Some("name"));

        let err = ValidationError::Custom("test".to_string());
        assert_eq!(err.field(), None);
    }

    #[test]
    fn test_validation_error_combine() {
        let empty: Vec<ValidationError> = vec![];
        assert!(ValidationError::combine(empty).is_ok());

        let single = vec![ValidationError::invalid_field("test", "error")];
        assert!(ValidationError::combine(single).is_err());

        let multiple = vec![
            ValidationError::invalid_field("test1", "error1"),
            ValidationError::invalid_field("test2", "error2"),
        ];
        let result = ValidationError::combine(multiple);
        assert!(matches!(result, Err(ValidationError::Multiple(..))));
    }

    #[test]
    fn test_versioned_trait() {
        #[derive(Clone)]
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
        assert!(!old_model.is_supported_version());

        let current = TestModel {
            version: CURRENT_SCHEMA_VERSION,
        };
        assert!(!current.needs_migration());
        assert!(current.is_supported_version());

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
        assert!(is_valid_hostname("a.b.c.d.e.f.g"));

        assert!(!is_valid_hostname(""));
        assert!(!is_valid_hostname("-example.com"));
        assert!(!is_valid_hostname("example-.com"));
        assert!(!is_valid_hostname("example..com"));
        assert!(!is_valid_hostname(&"a".repeat(254)));
    }

    #[test]
    fn test_is_valid_hostname_label() {
        // Valid labels
        assert!(is_valid_hostname_label("example"));
        assert!(is_valid_hostname_label("a-b"));
        assert!(is_valid_hostname_label("a1"));

        // Invalid labels
        assert!(!is_valid_hostname_label(""));
        assert!(!is_valid_hostname_label(&"a".repeat(64)));
        assert!(!is_valid_hostname_label("-example"));
        assert!(!is_valid_hostname_label("example-"));
    }

    #[test]
    fn test_is_valid_ip() {
        assert!(is_valid_ip("192.168.1.1"));
        assert!(is_valid_ip("0.0.0.0"));
        assert!(is_valid_ip("255.255.255.255"));
        assert!(is_valid_ip("::1"));
        assert!(is_valid_ip("0:0:0:0:0:0:0:1"));
        assert!(is_valid_ip("2001:db8::1"));

        assert!(!is_valid_ip("999.999.999.999"));
        assert!(!is_valid_ip("192.168.1"));
        assert!(!is_valid_ip("not-an-ip"));
        assert!(!is_valid_ip(""));
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
    }

    #[test]
    fn test_is_valid_hex_color() {
        assert!(is_valid_hex_color("#FFF"));
        assert!(is_valid_hex_color("#fff"));
        assert!(is_valid_hex_color("#4A90D9"));
        assert!(is_valid_hex_color("#4a90d9"));
        assert!(is_valid_hex_color("#123"));

        assert!(!is_valid_hex_color("4A90D9")); // Missing #
        assert!(!is_valid_hex_color("#GGGGGG")); // Invalid hex chars
        assert!(!is_valid_hex_color("#4A90D")); // Wrong length
        assert!(!is_valid_hex_color("#4A90D99")); // Wrong length
        assert!(!is_valid_hex_color("")); // Empty
        assert!(!is_valid_hex_color("#")); // Just #
    }

    #[test]
    fn test_is_valid_uuid() {
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(is_valid_uuid("00000000-0000-0000-0000-000000000000"));

        assert!(!is_valid_uuid("not-a-uuid"));
        assert!(!is_valid_uuid(""));
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000")); // Too short
    }

    #[test]
    fn test_sanitize_for_filename() {
        assert_eq!(sanitize_for_filename("test.txt"), "test.txt");
        assert_eq!(sanitize_for_filename("test/file.txt"), "test_file.txt");
        assert_eq!(sanitize_for_filename("test:file"), "test_file");
        assert_eq!(sanitize_for_filename("test file"), "test_file");
        assert_eq!(sanitize_for_filename(""), "");
    }

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_SSH_PORT, 22);
        assert_eq!(CURRENT_SCHEMA_VERSION, 1);
        assert_eq!(MIN_SCHEMA_VERSION, 1);
        assert_eq!(MAX_NAME_LENGTH, 100);
        assert_eq!(MAX_USERNAME_LENGTH, 32);
        assert_eq!(MAX_HOSTNAME_LENGTH, 253);
        assert_eq!(MAX_DESCRIPTION_LENGTH, 500);
        assert_eq!(MAX_PORT, 65535);
    }
}
