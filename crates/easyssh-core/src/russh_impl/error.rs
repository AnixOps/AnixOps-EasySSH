//! Error types for russh-based SSH implementation
//!
//! Provides comprehensive error handling following SYSTEM_INVARIANTS.md:
//! - All errors contain context (operation, target, reason)
//! - Network errors distinguish transient vs permanent
//! - User errors provide actionable suggestions

use std::fmt;
use thiserror::Error;

/// Top-level error type for russh operations.
#[derive(Debug, Error)]
pub enum RusshError {
    /// Connection establishment failed
    #[error("Connection to {host}:{port} failed: {message}")]
    ConnectionFailed {
        host: String,
        port: u16,
        message: String,
    },

    /// Authentication failed
    #[error("Authentication failed for {username}@{host}: {reason}")]
    AuthFailed {
        host: String,
        username: String,
        reason: String,
    },

    /// Host key verification failed
    #[error("Host key verification failed for {host}: {reason}")]
    HostKeyVerification {
        host: String,
        reason: String,
    },

    /// Session not found or disconnected
    #[error("Session '{session_id}' not found or disconnected")]
    SessionNotFound {
        session_id: String,
    },

    /// Channel creation failed
    #[error("Failed to create channel: {reason}")]
    ChannelFailed {
        reason: String,
    },

    /// Command execution failed
    #[error("Command execution failed: {reason}")]
    ExecFailed {
        reason: String,
    },

    /// SFTP operation failed
    #[error("SFTP error: {reason}")]
    SftpError {
        reason: String,
    },

    /// Port forwarding failed
    #[error("Port forwarding error: {reason}")]
    PortForwardError {
        reason: String,
    },

    /// Connection timeout
    #[error("Connection timeout after {seconds}s")]
    Timeout {
        seconds: u64,
    },

    /// Configuration error
    #[error("Invalid configuration: {reason}")]
    ConfigError {
        reason: String,
    },

    /// Session pool is full
    #[error("Session pool is full (max: {max_connections})")]
    PoolFull {
        max_connections: usize,
    },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Russh protocol error
    #[error("SSH protocol error: {0}")]
    Protocol(String),

    /// Key format error
    #[error("Invalid key format: {reason}")]
    KeyFormat {
        reason: String,
    },

    /// Agent error
    #[error("SSH agent error: {reason}")]
    AgentError {
        reason: String,
    },

    /// Jump host error
    #[error("Jump host connection failed: {reason}")]
    JumpHostError {
        reason: String,
    },

    /// Reconnect exhausted
    #[error("Reconnect failed after {attempts} attempts")]
    ReconnectExhausted {
        attempts: u32,
    },

    /// Internal error
    #[error("Internal error: {reason}")]
    Internal {
        reason: String,
    },
}

impl RusshError {
    /// Check if this error is retryable (transient network issue).
    ///
    /// Following SYSTEM_INVARIANTS.md Section 7.1:
    /// "Network errors must distinguish temporary/permanent"
    pub fn is_retryable(&self) -> bool {
        match self {
            RusshError::ConnectionFailed { message, .. } => {
                // Transient errors: connection reset, timeout, network unreachable
                let lower = message.to_lowercase();
                lower.contains("reset")
                    || lower.contains("timeout")
                    || lower.contains("temporarily")
                    || lower.contains("network unreachable")
                    || lower.contains("connection refused")
            }
            RusshError::Timeout { .. } => true,
            RusshError::Protocol(msg) => {
                let lower = msg.to_lowercase();
                lower.contains("disconnect") && !lower.contains("protocol error")
            }
            RusshError::ReconnectExhausted { .. } => false,
            _ => false,
        }
    }

    /// Get user-friendly suggestion for resolving the error.
    ///
    /// Following SYSTEM_INVARIANTS.md Section 7.1:
    /// "User errors must provide actionable suggestions"
    pub fn user_suggestion(&self) -> Option<&'static str> {
        match self {
            RusshError::AuthFailed { .. } => {
                Some("Check username and password, or try using SSH key authentication")
            }
            RusshError::ConnectionFailed { .. } => {
                Some("Check network connectivity and verify the server address")
            }
            RusshError::HostKeyVerification { .. } => {
                Some("Verify the server's host key fingerprint with your administrator")
            }
            RusshError::Timeout { .. } => {
                Some("The server may be overloaded; try again later")
            }
            RusshError::PoolFull { .. } => {
                Some("Close unused sessions or increase the connection pool size")
            }
            RusshError::KeyFormat { .. } => {
                Some("Ensure the key is in OpenSSH or PEM format")
            }
            RusshError::AgentError { .. } => {
                Some("Ensure SSH agent is running and the key is loaded")
            }
            RusshError::JumpHostError { .. } => {
                Some("Verify jump host configuration and network access")
            }
            _ => None,
        }
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(self, RusshError::AuthFailed { .. })
    }

    /// Check if this is a host key verification error.
    pub fn is_host_key_error(&self) -> bool {
        matches!(self, RusshError::HostKeyVerification { .. })
    }

    /// Check if this is a timeout error.
    pub fn is_timeout(&self) -> bool {
        matches!(self, RusshError::Timeout { .. })
    }

    /// Get error code for debugging.
    pub fn error_code(&self) -> &'static str {
        match self {
            RusshError::ConnectionFailed { .. } => "R1001",
            RusshError::AuthFailed { .. } => "R1002",
            RusshError::HostKeyVerification { .. } => "R1003",
            RusshError::SessionNotFound { .. } => "R1004",
            RusshError::ChannelFailed { .. } => "R1005",
            RusshError::ExecFailed { .. } => "R1006",
            RusshError::SftpError { .. } => "R1007",
            RusshError::PortForwardError { .. } => "R1008",
            RusshError::Timeout { .. } => "R1009",
            RusshError::ConfigError { .. } => "R1010",
            RusshError::PoolFull { .. } => "R1011",
            RusshError::Io(_) => "R1012",
            RusshError::Protocol(_) => "R1013",
            RusshError::KeyFormat { .. } => "R1014",
            RusshError::AgentError { .. } => "R1015",
            RusshError::JumpHostError { .. } => "R1016",
            RusshError::ReconnectExhausted { .. } => "R1017",
            RusshError::Internal { .. } => "R1018",
        }
    }
}

/// Result type for russh operations.
pub type RusshResult<T> = std::result::Result<T, RusshError>;

#[cfg(feature = "russh-backend")]
impl From<russh::Error> for RusshError {
    fn from(e: russh::Error) -> Self {
        RusshError::Protocol(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_retryable() {
        let err = RusshError::ConnectionFailed {
            host: "test.com".into(),
            port: 22,
            message: "Connection reset by peer".into(),
        };
        assert!(err.is_retryable());

        let err = RusshError::AuthFailed {
            host: "test.com".into(),
            username: "root".into(),
            reason: "Invalid password".into(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_error_user_suggestion() {
        let err = RusshError::AuthFailed {
            host: "test.com".into(),
            username: "root".into(),
            reason: "Invalid password".into(),
        };
        assert!(err.user_suggestion().is_some());
    }

    #[test]
    fn test_error_codes() {
        let err = RusshError::Timeout { seconds: 30 };
        assert_eq!(err.error_code(), "R1009");
    }

    #[test]
    fn test_error_display() {
        let err = RusshError::ConnectionFailed {
            host: "192.168.1.1".into(),
            port: 22,
            message: "Network unreachable".into(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("192.168.1.1"));
        assert!(msg.contains("22"));
    }
}