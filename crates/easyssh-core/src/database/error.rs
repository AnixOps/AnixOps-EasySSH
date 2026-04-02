//! Database error types
//!
//! This module defines error types specific to database operations,
//! providing detailed error information for debugging and user feedback.

use std::fmt;
use std::io;

/// Result type alias for database operations
pub type Result<T> = std::result::Result<T, DatabaseError>;

/// Error type for database operations
#[derive(Debug)]
pub enum DatabaseError {
    /// SQL error from sqlx
    SqlError(sqlx::Error),

    /// IO error during database operations
    Io(io::Error),

    /// Migration error
    Migration { version: i64, message: String },

    /// Validation error
    Validation(String),

    /// Record not found
    NotFound { entity: String, id: String },

    /// Constraint violation (unique constraint, foreign key, etc.)
    ConstraintViolation { constraint: String, message: String },

    /// Transaction error
    Transaction(String),

    /// Database connection error
    Connection(String),

    /// Configuration error
    Config(String),
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::SqlError(e) => write!(f, "Database SQL error: {}", e),
            DatabaseError::Io(e) => write!(f, "Database IO error: {}", e),
            DatabaseError::Migration { version, message } => {
                write!(f, "Migration error at version {}: {}", version, message)
            }
            DatabaseError::Validation(msg) => write!(f, "Validation error: {}", msg),
            DatabaseError::NotFound { entity, id } => {
                write!(f, "{} not found: {}", entity, id)
            }
            DatabaseError::ConstraintViolation {
                constraint,
                message,
            } => {
                write!(f, "Constraint violation ({}): {}", constraint, message)
            }
            DatabaseError::Transaction(msg) => write!(f, "Transaction error: {}", msg),
            DatabaseError::Connection(msg) => write!(f, "Connection error: {}", msg),
            DatabaseError::Config(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for DatabaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DatabaseError::SqlError(e) => Some(e),
            DatabaseError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        // Try to extract specific error information
        match &err {
            sqlx::Error::RowNotFound => DatabaseError::NotFound {
                entity: "Record".to_string(),
                id: "unknown".to_string(),
            },
            sqlx::Error::Database(db_err) => {
                let message = db_err.message().to_string();
                // Check for common SQLite error patterns
                if message.contains("UNIQUE constraint failed") {
                    DatabaseError::ConstraintViolation {
                        constraint: "UNIQUE".to_string(),
                        message,
                    }
                } else if message.contains("FOREIGN KEY constraint failed") {
                    DatabaseError::ConstraintViolation {
                        constraint: "FOREIGN KEY".to_string(),
                        message,
                    }
                } else if message.contains("NOT NULL constraint failed") {
                    DatabaseError::ConstraintViolation {
                        constraint: "NOT NULL".to_string(),
                        message,
                    }
                } else {
                    DatabaseError::SqlError(err)
                }
            }
            _ => DatabaseError::SqlError(err),
        }
    }
}

impl From<io::Error> for DatabaseError {
    fn from(err: io::Error) -> Self {
        DatabaseError::Io(err)
    }
}

impl From<String> for DatabaseError {
    fn from(err: String) -> Self {
        DatabaseError::Validation(err)
    }
}

impl From<&str> for DatabaseError {
    fn from(err: &str) -> Self {
        DatabaseError::Validation(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DatabaseError::Validation("Invalid input".to_string());
        assert_eq!(err.to_string(), "Validation error: Invalid input");

        let err = DatabaseError::NotFound {
            entity: "Server".to_string(),
            id: "123".to_string(),
        };
        assert_eq!(err.to_string(), "Server not found: 123");
    }

    #[test]
    fn test_from_sqlx_error() {
        let sqlx_err = sqlx::Error::RowNotFound;
        let db_err: DatabaseError = sqlx_err.into();

        match db_err {
            DatabaseError::NotFound { entity, .. } => {
                assert_eq!(entity, "Record");
            }
            _ => panic!("Expected NotFound error"),
        }
    }
}
