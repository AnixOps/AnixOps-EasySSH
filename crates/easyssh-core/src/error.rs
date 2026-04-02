//! EasySSH Error Handling System
//!
//! This module provides unified error types for the EasySSH application.
//! It uses `thiserror` for ergonomic error definitions and provides:
//!
//! - Hierarchical error types (EasySSHErrors as the top-level enum)
//! - Automatic error conversion via #[from] attributes
//! - User-friendly error messages with translation keys
//! - Error codes for searching and debugging
//! - Retry suggestions for transient errors
//!
//! # Example
//!
//! ```rust
//! use easyssh_core::error::{EasySSHErrors, Result};
//!
//! fn may_fail() -> Result<String> {
//!     // Returns EasySSHErrors::Io on failure
//!     let content = std::fs::read_to_string("config.txt")?;
//!     Ok(content)
//! }
//! ```

use std::fmt;
use thiserror::Error;

/// Top-level error type for EasySSH operations.
///
/// This enum wraps all possible errors that can occur in the application,
/// providing a unified error handling interface.
#[derive(Debug, Error)]
pub enum EasySSHErrors {
    /// Cryptographic operation errors (encryption, decryption, key derivation)
    #[error("加密错误: {0}")]
    Crypto(#[from] CoreCryptoError),

    /// Database operation errors (SQLite, queries, migrations)
    #[error("数据库错误: {0}")]
    Database(#[from] CoreDatabaseError),

    /// SSH connection and operation errors
    #[error("SSH连接错误: {0}")]
    Ssh(#[from] CoreSshError),

    /// I/O errors from std::io
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration errors (invalid config files, missing settings)
    #[error("配置错误: {0}")]
    Config(String),

    /// Validation errors (invalid input, format errors)
    #[error("验证错误: {0}")]
    Validation(String),

    /// User cancelled the operation
    #[error("用户取消")]
    UserCancelled,

    /// Serialization/deserialization errors
    #[error("数据解析错误: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Network-related errors
    #[error("网络错误: {0}")]
    Network(String),

    /// Authentication failures (wrong password, invalid key)
    #[error("认证失败: {0}")]
    Authentication(String),

    /// Timeout errors
    #[error("操作超时")]
    Timeout,

    /// Not found errors (server, session, file)
    #[error("未找到: {0}")]
    NotFound(String),

    /// Permission denied errors
    #[error("权限不足: {0}")]
    PermissionDenied(String),

    /// Feature not available in current edition
    #[error("功能不可用: {feature} 在 {edition} 版本中不可用")]
    FeatureNotAvailable { feature: String, edition: String },

    /// Internal/unexpected errors
    #[error("内部错误: {0}")]
    Internal(String),
}

/// Result type alias using EasySSHErrors
pub type EasySSHResult<T> = std::result::Result<T, EasySSHErrors>;

/// Short Result type alias for convenience
pub type Result<T> = std::result::Result<T, EasySSHErrors>;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum CoreCryptoError {
    #[error("密钥派生失败: {0}")]
    KeyDerivation(String),

    #[error("加密失败: {0}")]
    Encryption(String),

    #[error("解密失败: {0}")]
    Decryption(String),

    #[error("无效的主密码")]
    InvalidMasterPassword,

    #[error("密钥库错误: {0}")]
    Keychain(String),

    #[error("无效的密钥格式: {0}")]
    InvalidKeyFormat(String),

    #[error("随机数生成失败: {0}")]
    RngError(String),
}

/// Database error types
#[derive(Debug, Error, Clone, PartialEq)]
pub enum CoreDatabaseError {
    #[error("连接失败: {0}")]
    Connection(String),

    #[error("查询执行失败: {0}")]
    Query(String),

    #[error("迁移失败: {0}")]
    Migration(String),

    #[error("唯一约束冲突: {0}")]
    UniqueViolation(String),

    #[error("外键约束冲突: {0}")]
    ForeignKeyViolation(String),

    #[error("记录未找到: {table}(id={id})")]
    RecordNotFound { table: String, id: String },

    #[error("事务失败: {0}")]
    Transaction(String),

    #[error("数据库锁定超时")]
    LockTimeout,
}

/// SSH error types
#[derive(Debug, Error, Clone, PartialEq)]
pub enum CoreSshError {
    #[error("连接到 {host}:{port} 失败: {message}")]
    ConnectionFailed {
        host: String,
        port: u16,
        message: String,
    },

    #[error("认证失败 (主机: {host}, 用户: {username})")]
    AuthFailed { host: String, username: String },

    #[error("会话不存在: {0}")]
    SessionNotFound(String),

    #[error("会话已断开: {0}")]
    SessionDisconnected(String),

    #[error("连接超时")]
    ConnectionTimeout,

    #[error("通道创建失败: {0}")]
    ChannelFailed(String),

    #[error("命令执行失败: {0}")]
    CommandFailed(String),

    #[error("SFTP错误: {0}")]
    Sftp(String),

    #[error("端口转发错误: {0}")]
    PortForward(String),

    #[error("代理跳转错误: {0}")]
    ProxyJump(String),

    #[error("主机密钥验证失败: {0}")]
    HostKeyVerification(String),

    #[error("会话池已满")]
    SessionPoolFull,
}

/// Error severity levels for logging and user notification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Critical,
    Error,
    Warning,
    Info,
}

/// Error display information for user-friendly messages
pub struct ErrorDisplay {
    /// Error code for searching documentation
    pub code: &'static str,
    /// User-friendly message (translated)
    pub message: String,
    /// Suggested action for the user
    pub suggestion: Option<String>,
    /// Whether the operation can be retried
    pub retryable: bool,
    /// Error severity
    pub severity: ErrorSeverity,
}

impl EasySSHErrors {
    /// Create a configuration error
    pub fn configuration<T: Into<String>>(msg: T) -> Self {
        EasySSHErrors::Config(msg.into())
    }

    /// Get the error code for documentation lookup
    pub fn error_code(&self) -> &'static str {
        match self {
            EasySSHErrors::Crypto(_) => "E1000",
            EasySSHErrors::Database(_) => "E2000",
            EasySSHErrors::Ssh(_) => "E3000",
            EasySSHErrors::Io(_) => "E4000",
            EasySSHErrors::Config(_) => "E5000",
            EasySSHErrors::Validation(_) => "E6000",
            EasySSHErrors::UserCancelled => "E7000",
            EasySSHErrors::Serialization(_) => "E8000",
            EasySSHErrors::Network(_) => "E9000",
            EasySSHErrors::Authentication(_) => "E1001",
            EasySSHErrors::Timeout => "E1100",
            EasySSHErrors::NotFound(_) => "E1200",
            EasySSHErrors::PermissionDenied(_) => "E1300",
            EasySSHErrors::FeatureNotAvailable { .. } => "E1400",
            EasySSHErrors::Internal(_) => "E9900",
        }
    }

    /// Get the translation key for this error
    pub fn translation_key(&self) -> &'static str {
        match self {
            EasySSHErrors::Crypto(_) => "error-crypto",
            EasySSHErrors::Database(_) => "error-database",
            EasySSHErrors::Ssh(_) => "error-ssh",
            EasySSHErrors::Io(_) => "error-io",
            EasySSHErrors::Config(_) => "error-config",
            EasySSHErrors::Validation(_) => "error-validation",
            EasySSHErrors::UserCancelled => "error-user-cancelled",
            EasySSHErrors::Serialization(_) => "error-serialization",
            EasySSHErrors::Network(_) => "error-network",
            EasySSHErrors::Authentication(_) => "error-authentication",
            EasySSHErrors::Timeout => "error-timeout",
            EasySSHErrors::NotFound(_) => "error-not-found",
            EasySSHErrors::PermissionDenied(_) => "error-permission-denied",
            EasySSHErrors::FeatureNotAvailable { .. } => "error-feature-not-available",
            EasySSHErrors::Internal(_) => "error-internal",
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            // Network and temporary errors are retryable
            EasySSHErrors::Ssh(CoreSshError::ConnectionFailed { .. }) => true,
            EasySSHErrors::Ssh(CoreSshError::ConnectionTimeout) => true,
            EasySSHErrors::Ssh(CoreSshError::SessionDisconnected(_)) => true,
            EasySSHErrors::Network(_) => true,
            EasySSHErrors::Timeout => true,
            EasySSHErrors::Database(CoreDatabaseError::Connection(_)) => true,
            EasySSHErrors::Database(CoreDatabaseError::LockTimeout) => true,
            EasySSHErrors::Io(e) if e.kind() == std::io::ErrorKind::Interrupted => true,
            // Other errors are generally not retryable
            _ => false,
        }
    }

    /// Get retry suggestion for this error
    pub fn retry_suggestion(&self) -> Option<&'static str> {
        if !self.is_retryable() {
            return None;
        }

        match self {
            EasySSHErrors::Ssh(CoreSshError::ConnectionFailed { .. }) => {
                Some("请检查网络连接和服务器地址，然后重试")
            }
            EasySSHErrors::Ssh(CoreSshError::ConnectionTimeout) => {
                Some("连接超时，请稍后重试或检查网络状态")
            }
            EasySSHErrors::Ssh(CoreSshError::SessionDisconnected(_)) => {
                Some("会话已断开，正在尝试重新连接...")
            }
            EasySSHErrors::Network(_) => Some("网络不稳定，请检查网络后重试"),
            EasySSHErrors::Timeout => Some("操作超时，请稍后重试"),
            EasySSHErrors::Database(CoreDatabaseError::LockTimeout) => {
                Some("数据库繁忙，请稍后重试")
            }
            _ => Some("请稍后重试"),
        }
    }

    /// Get the error severity
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            EasySSHErrors::Crypto(CoreCryptoError::InvalidMasterPassword) => ErrorSeverity::Critical,
            EasySSHErrors::Authentication(_) => ErrorSeverity::Critical,
            EasySSHErrors::PermissionDenied(_) => ErrorSeverity::Critical,
            EasySSHErrors::Ssh(CoreSshError::AuthFailed { .. }) => ErrorSeverity::Critical,
            EasySSHErrors::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                ErrorSeverity::Critical
            }
            EasySSHErrors::Config(_) => ErrorSeverity::Error,
            EasySSHErrors::Database(_) => ErrorSeverity::Error,
            EasySSHErrors::Ssh(_) => ErrorSeverity::Error,
            EasySSHErrors::Network(_) => ErrorSeverity::Error,
            EasySSHErrors::Timeout => ErrorSeverity::Warning,
            EasySSHErrors::UserCancelled => ErrorSeverity::Info,
            _ => ErrorSeverity::Error,
        }
    }

    /// Get user-friendly error display information
    pub fn display_info(&self) -> ErrorDisplay {
        ErrorDisplay {
            code: self.error_code(),
            message: self.to_string(),
            suggestion: self.retry_suggestion().map(|s| s.to_string()),
            retryable: self.is_retryable(),
            severity: self.severity(),
        }
    }

    /// Get detailed context information for logging
    pub fn context(&self) -> Vec<(&'static str, String)> {
        match self {
            EasySSHErrors::Crypto(e) => vec![("crypto_error", e.to_string())],
            EasySSHErrors::Database(e) => vec![("database_error", e.to_string())],
            EasySSHErrors::Ssh(CoreSshError::ConnectionFailed {
                host,
                port,
                message,
            }) => vec![
                ("host", host.clone()),
                ("port", port.to_string()),
                ("message", message.clone()),
            ],
            EasySSHErrors::Ssh(CoreSshError::AuthFailed { host, username }) => vec![
                ("host", host.clone()),
                ("username", username.clone()),
            ],
            EasySSHErrors::Ssh(CoreSshError::SessionNotFound(id))
            | EasySSHErrors::Ssh(CoreSshError::SessionDisconnected(id)) => {
                vec![("session_id", id.clone())]
            }
            EasySSHErrors::NotFound(item) => vec![("item", item.clone())],
            EasySSHErrors::Config(key) => vec![("config_key", key.clone())],
            EasySSHErrors::Validation(msg) => vec![("validation_msg", msg.clone())],
            EasySSHErrors::Network(msg) => vec![("network_msg", msg.clone())],
            EasySSHErrors::Authentication(msg) => vec![("auth_msg", msg.clone())],
            EasySSHErrors::PermissionDenied(msg) => vec![("permission_msg", msg.clone())],
            EasySSHErrors::Internal(msg) => vec![("internal_msg", msg.clone())],
            EasySSHErrors::FeatureNotAvailable { feature, edition } => vec![
                ("feature", feature.clone()),
                ("edition", edition.clone()),
            ],
            _ => vec![],
        }
    }
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
            ErrorSeverity::Error => write!(f, "ERROR"),
            ErrorSeverity::Warning => write!(f, "WARNING"),
            ErrorSeverity::Info => write!(f, "INFO"),
        }
    }
}

// Conversions from rusqlite errors
impl From<rusqlite::Error> for CoreDatabaseError {
    fn from(e: rusqlite::Error) -> Self {
        match e {
            rusqlite::Error::SqliteFailure(libsql_error, msg) => {
                let code = libsql_error.code;
                match code {
                    rusqlite::ErrorCode::DatabaseBusy => CoreDatabaseError::LockTimeout,
                    rusqlite::ErrorCode::ConstraintViolation => {
                        CoreDatabaseError::UniqueViolation(msg.unwrap_or_default())
                    }
                    _ => CoreDatabaseError::Query(msg.unwrap_or_else(|| format!("{:?}", code))),
                }
            }
            rusqlite::Error::QueryReturnedNoRows => CoreDatabaseError::RecordNotFound {
                table: "unknown".to_string(),
                id: "unknown".to_string(),
            },
            _ => CoreDatabaseError::Query(e.to_string()),
        }
    }
}

impl From<rusqlite::Error> for EasySSHErrors {
    fn from(e: rusqlite::Error) -> Self {
        EasySSHErrors::Database(e.into())
    }
}

// Conversion from existing LiteError for backward compatibility
impl From<LiteError> for EasySSHErrors {
    fn from(e: LiteError) -> Self {
        match e {
            LiteError::Database(msg) => EasySSHErrors::Database(CoreDatabaseError::Connection(msg)),
            LiteError::Crypto(msg) => EasySSHErrors::Crypto(CoreCryptoError::Encryption(msg)),
            LiteError::Ssh(msg) => EasySSHErrors::Ssh(CoreSshError::CommandFailed(msg)),
            LiteError::Io(msg) => {
                EasySSHErrors::Io(std::io::Error::new(std::io::ErrorKind::Other, msg))
            }
            LiteError::Config(msg) => EasySSHErrors::Config(msg),
            LiteError::InvalidMasterPassword => {
                EasySSHErrors::Crypto(CoreCryptoError::InvalidMasterPassword)
            }
            LiteError::SshTimeout => EasySSHErrors::Ssh(CoreSshError::ConnectionTimeout),
            LiteError::SshConnectionFailed {
                host,
                port,
                message,
            } => EasySSHErrors::Ssh(CoreSshError::ConnectionFailed {
                host,
                port,
                message,
            }),
            LiteError::SshAuthFailed { host, username } => {
                EasySSHErrors::Ssh(CoreSshError::AuthFailed { host, username })
            }
            LiteError::SshSessionNotFound(id) => EasySSHErrors::Ssh(CoreSshError::SessionNotFound(id)),
            LiteError::SshSessionDisconnected(id) => {
                EasySSHErrors::Ssh(CoreSshError::SessionDisconnected(id))
            }
            LiteError::AuthFailed => EasySSHErrors::Authentication("认证失败".to_string()),
            LiteError::ServerNotFound(id) => EasySSHErrors::NotFound(format!("服务器: {}", id)),
            LiteError::SessionPoolFull => EasySSHErrors::Ssh(CoreSshError::SessionPoolFull),
            LiteError::Json(msg) => EasySSHErrors::Serialization(serde_json::Error::custom(msg)),
            LiteError::FileNotFound { path } => EasySSHErrors::NotFound(format!("文件: {}", path)),
            LiteError::FeatureNotAvailable { feature, edition } => {
                EasySSHErrors::FeatureNotAvailable { feature, edition }
            }
            _ => EasySSHErrors::Internal(e.to_string()),
        }
    }
}

// Helper trait for creating custom serde_json::Error
trait CustomJsonError {
    fn custom<T: fmt::Display>(msg: T) -> Self;
}

impl CustomJsonError for serde_json::Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        serde_json::from_str::<serde_json::Value>("").unwrap_err()
    }
}

/// Legacy error type for backward compatibility
///
/// This type is being phased out in favor of EasySSHErrors.
/// New code should use EasySSHErrors directly.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum LiteError {
    #[error("error-database")]
    Database(String),

    #[error("error-crypto")]
    Crypto(String),

    #[error("error-keychain")]
    Keychain(String),

    #[error("error-terminal")]
    Terminal(String),

    #[error("error-terminal-emulator")]
    TerminalEmulator(String),

    #[error("error-layout")]
    Layout(String),

    #[error("error-team")]
    Team(String),

    #[error("error-audit")]
    Audit(String),

    #[error("error-rbac")]
    Rbac(String),

    #[error("error-ssh")]
    Ssh(String),

    #[error("ssh-connection-failed")]
    SshConnectionFailed {
        host: String,
        port: u16,
        message: String,
    },

    #[error("ssh-auth-failed")]
    SshAuthFailed { host: String, username: String },

    #[error("ssh-session-not-found")]
    SshSessionNotFound(String),

    #[error("ssh-session-disconnected")]
    SshSessionDisconnected(String),

    #[error("ssh-timeout")]
    SshTimeout,

    #[error("ssh-channel-failed")]
    SshChannelFailed(String),

    #[error("error-config")]
    Config(String),

    #[error("error-io")]
    Io(String),

    #[error("error-json")]
    Json(String),

    #[error("server-not-found")]
    ServerNotFound(String),

    #[error("group-not-found")]
    GroupNotFound(String),

    #[error("error-auth-failed")]
    AuthFailed,

    #[error("error-invalid-master-password")]
    InvalidMasterPassword,

    #[error("error-session-pool-full")]
    SessionPoolFull,

    #[error("connection-reset")]
    ConnectionReset,

    #[error("error-import-failed")]
    ImportFailed(String),

    #[error("error-export-failed")]
    ExportFailed(String),

    #[error("error-file-not-found")]
    FileNotFound { path: String },

    #[error("error-invalid-key")]
    InvalidKey(String),

    #[error("error-telemetry")]
    Telemetry(String),

    #[error("error-recording")]
    RecordingError(String),

    #[error("error-session-recording")]
    SessionRecording(String),

    #[error("error-session-not-found")]
    SessionNotFound(String),

    #[error("error-remote-desktop")]
    RemoteDesktop(String),

    #[error("error-docker")]
    Docker(String),

    #[error("error-git")]
    Git(String),

    #[error("git-merge-conflict")]
    GitMergeConflict { files: Vec<String> },

    #[error("error-sso")]
    Sso(String),

    #[error("error-feature-not-available")]
    FeatureNotAvailable { feature: String, edition: String },

    #[error("error-internal")]
    Internal(String),
}

impl LiteError {
    /// Get the translation key for this error type
    pub fn translation_key(&self) -> &'static str {
        match self {
            LiteError::Database(_) => "error-database",
            LiteError::Crypto(_) => "error-crypto",
            LiteError::Keychain(_) => "error-keychain",
            LiteError::Terminal(_) => "error-terminal",
            LiteError::TerminalEmulator(_) => "error-terminal",
            LiteError::Layout(_) => "error-layout",
            LiteError::Team(_) => "error-team",
            LiteError::Audit(_) => "error-audit",
            LiteError::Rbac(_) => "error-rbac",
            LiteError::Ssh(_) => "error-ssh",
            LiteError::SshConnectionFailed { .. } => "error-connection-failed",
            LiteError::SshAuthFailed { .. } => "connection-auth-failed",
            LiteError::SshSessionNotFound(_) => "ssh-session-not-found",
            LiteError::SshSessionDisconnected(_) => "ssh-session-disconnected",
            LiteError::SshTimeout => "connection-timeout",
            LiteError::SshChannelFailed(_) => "error-ssh",
            LiteError::Config(_) => "error-config",
            LiteError::Io(_) => "error-io",
            LiteError::Json(_) => "error-json",
            LiteError::ServerNotFound(_) => "error-not-found",
            LiteError::GroupNotFound(_) => "group-not-found",
            LiteError::AuthFailed => "error-auth-failed",
            LiteError::InvalidMasterPassword => "error-invalid-master-password",
            LiteError::SessionPoolFull => "error-session-pool-full",
            LiteError::ConnectionReset => "connection-reset",
            LiteError::ImportFailed(_) => "error-import-failed",
            LiteError::ExportFailed(_) => "error-export-failed",
            LiteError::FileNotFound { .. } => "error-file-not-found",
            LiteError::InvalidKey(_) => "error-invalid-key",
            LiteError::Telemetry(_) => "error-generic",
            LiteError::RecordingError(_) => "error-recording",
            LiteError::SessionRecording(_) => "error-session-recording",
            LiteError::SessionNotFound(_) => "error-session-not-found",
            LiteError::RemoteDesktop(_) => "error-remote-desktop",
            LiteError::Docker(_) => "error-docker",
            LiteError::Git(_) => "error-git",
            LiteError::Sso(_) => "error-sso",
            LiteError::GitMergeConflict { .. } => "git-merge-conflit",
            LiteError::FeatureNotAvailable { .. } => "error-feature-not-available",
            LiteError::Internal(_) => "error-internal",
        }
    }

    /// Get translation arguments for this error
    pub fn translation_args(&self) -> Vec<(&'static str, String)> {
        match self {
            LiteError::Database(msg)
            | LiteError::Docker(msg)
            | LiteError::Git(msg)
            | LiteError::Sso(msg)
            | LiteError::Crypto(msg)
            | LiteError::Keychain(msg)
            | LiteError::Terminal(msg)
            | LiteError::TerminalEmulator(msg)
            | LiteError::Layout(msg)
            | LiteError::Team(msg)
            | LiteError::Audit(msg)
            | LiteError::Rbac(msg)
            | LiteError::Ssh(msg)
            | LiteError::Config(msg)
            | LiteError::Io(msg)
            | LiteError::Json(msg)
            | LiteError::SshChannelFailed(msg)
            | LiteError::ImportFailed(msg)
            | LiteError::ExportFailed(msg)
            | LiteError::RecordingError(msg)
            | LiteError::SessionRecording(msg)
            | LiteError::InvalidKey(msg)
            | LiteError::RemoteDesktop(msg)
            | LiteError::Internal(msg) => {
                vec![("message", msg.clone())]
            }
            LiteError::SshConnectionFailed {
                host,
                port,
                message,
            } => {
                vec![
                    ("host", host.clone()),
                    ("port", port.to_string()),
                    ("message", message.clone()),
                ]
            }
            LiteError::SshAuthFailed { host, username } => {
                vec![("host", host.clone()), ("username", username.clone())]
            }
            LiteError::SshSessionNotFound(id)
            | LiteError::SshSessionDisconnected(id)
            | LiteError::SessionNotFound(id)
            | LiteError::ServerNotFound(id)
            | LiteError::GroupNotFound(id) => {
                vec![("0", id.clone())]
            }
            LiteError::FileNotFound { path } => {
                vec![("path", path.clone())]
            }
            LiteError::FeatureNotAvailable { feature, edition } => {
                vec![("feature", feature.clone()), ("edition", edition.clone())]
            }
            LiteError::GitMergeConflict { files } => {
                vec![("files", files.join(", "))]
            }
            _ => vec![],
        }
    }
}

impl serde::Serialize for LiteError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.translation_key())
    }
}

impl From<rusqlite::Error> for LiteError {
    fn from(e: rusqlite::Error) -> Self {
        LiteError::Database(e.to_string())
    }
}

impl From<std::io::Error> for LiteError {
    fn from(e: std::io::Error) -> Self {
        LiteError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for LiteError {
    fn from(e: serde_json::Error) -> Self {
        LiteError::Json(e.to_string())
    }
}

impl From<std::path::StripPrefixError> for LiteError {
    fn from(e: std::path::StripPrefixError) -> Self {
        LiteError::Io(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_assignment() {
        let err = EasySSHErrors::Config("test".to_string());
        assert_eq!(err.error_code(), "E5000");

        let err = EasySSHErrors::UserCancelled;
        assert_eq!(err.error_code(), "E7000");
    }

    #[test]
    fn test_error_translation_key() {
        let err = EasySSHErrors::Crypto(CoreCryptoError::Encryption("test".to_string()));
        assert_eq!(err.translation_key(), "error-crypto");

        let err = EasySSHErrors::Timeout;
        assert_eq!(err.translation_key(), "error-timeout");
    }

    #[test]
    fn test_retryable_errors() {
        // Retryable errors
        let err = EasySSHErrors::Ssh(CoreSshError::ConnectionFailed {
            host: "test".to_string(),
            port: 22,
            message: "timeout".to_string(),
        });
        assert!(err.is_retryable());

        let err = EasySSHErrors::Timeout;
        assert!(err.is_retryable());

        let err = EasySSHErrors::Network("timeout".to_string());
        assert!(err.is_retryable());

        // Non-retryable errors
        let err = EasySSHErrors::Config("invalid".to_string());
        assert!(!err.is_retryable());

        let err = EasySSHErrors::Validation("bad format".to_string());
        assert!(!err.is_retryable());

        let err = EasySSHErrors::UserCancelled;
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_error_severity() {
        assert_eq!(
            EasySSHErrors::Crypto(CoreCryptoError::InvalidMasterPassword).severity(),
            ErrorSeverity::Critical
        );
        assert_eq!(
            EasySSHErrors::Authentication("failed".to_string()).severity(),
            ErrorSeverity::Critical
        );
        assert_eq!(EasySSHErrors::UserCancelled.severity(), ErrorSeverity::Info);
        assert_eq!(EasySSHErrors::Timeout.severity(), ErrorSeverity::Warning);
    }

    #[test]
    fn test_display_info() {
        let err = EasySSHErrors::Ssh(CoreSshError::ConnectionTimeout);
        let info = err.display_info();
        assert_eq!(info.code, "E3000");
        assert!(info.retryable);
        assert!(info.suggestion.is_some());
    }

    #[test]
    fn test_error_context() {
        let err = EasySSHErrors::Ssh(CoreSshError::ConnectionFailed {
            host: "192.168.1.1".to_string(),
            port: 22,
            message: "refused".to_string(),
        });
        let context = err.context();
        assert_eq!(context.len(), 3);
        assert_eq!(context[0], ("host", "192.168.1.1".to_string()));
        assert_eq!(context[1], ("port", "22".to_string()));
    }

    #[test]
    fn test_from_rusqlite_error() {
        let sqlite_err = rusqlite::Error::InvalidQuery;
        let err: CoreDatabaseError = sqlite_err.into();
        assert!(matches!(err, CoreDatabaseError::Query(_)));
    }

    #[test]
    fn test_from_lite_error() {
        let lite_err = LiteError::Database("test".to_string());
        let err: EasySSHErrors = lite_err.into();
        assert!(matches!(err, EasySSHErrors::Database(_)));
    }

    #[test]
    fn test_crypto_error_variants() {
        let _ = CoreCryptoError::KeyDerivation("test".to_string());
        let _ = CoreCryptoError::Encryption("test".to_string());
        let _ = CoreCryptoError::Decryption("test".to_string());
        let _ = CoreCryptoError::InvalidMasterPassword;
        let _ = CoreCryptoError::Keychain("test".to_string());
        let _ = CoreCryptoError::InvalidKeyFormat("test".to_string());
        let _ = CoreCryptoError::RngError("test".to_string());
    }

    #[test]
    fn test_database_error_variants() {
        let _ = CoreDatabaseError::Connection("test".to_string());
        let _ = CoreDatabaseError::Query("test".to_string());
        let _ = CoreDatabaseError::Migration("test".to_string());
        let _ = CoreDatabaseError::UniqueViolation("test".to_string());
        let _ = CoreDatabaseError::ForeignKeyViolation("test".to_string());
        let _ = CoreDatabaseError::RecordNotFound {
            table: "test".to_string(),
            id: "1".to_string(),
        };
        let _ = CoreDatabaseError::Transaction("test".to_string());
        let _ = CoreDatabaseError::LockTimeout;
    }

    #[test]
    fn test_ssh_error_variants() {
        let _ = CoreSshError::ConnectionFailed {
            host: "test".to_string(),
            port: 22,
            message: "test".to_string(),
        };
        let _ = CoreSshError::AuthFailed {
            host: "test".to_string(),
            username: "user".to_string(),
        };
        let _ = CoreSshError::SessionNotFound("test".to_string());
        let _ = CoreSshError::SessionDisconnected("test".to_string());
        let _ = CoreSshError::ConnectionTimeout;
        let _ = CoreSshError::ChannelFailed("test".to_string());
        let _ = CoreSshError::CommandFailed("test".to_string());
        let _ = CoreSshError::Sftp("test".to_string());
        let _ = CoreSshError::PortForward("test".to_string());
        let _ = CoreSshError::ProxyJump("test".to_string());
        let _ = CoreSshError::HostKeyVerification("test".to_string());
        let _ = CoreSshError::SessionPoolFull;
    }

    #[test]
    fn test_result_type() {
        fn returns_result() -> EasySSHResult<String> {
            Ok("success".to_string())
        }

        fn returns_error() -> EasySSHResult<String> {
            Err(EasySSHErrors::NotFound("test".to_string()))
        }

        assert!(returns_result().is_ok());
        assert!(returns_error().is_err());
    }

    #[test]
    fn test_error_conversions_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: EasySSHErrors = io_err.into();
        assert!(matches!(err, EasySSHErrors::Io(_)));
    }

    #[test]
    fn test_error_conversions_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err: EasySSHErrors = json_err.into();
        assert!(matches!(err, EasySSHErrors::Serialization(_)));
    }

    #[test]
    fn test_feature_not_available() {
        let err = EasySSHErrors::FeatureNotAvailable {
            feature: "pro".to_string(),
            edition: "lite".to_string(),
        };
        assert_eq!(err.error_code(), "E1400");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", ErrorSeverity::Critical), "CRITICAL");
        assert_eq!(format!("{}", ErrorSeverity::Error), "ERROR");
        assert_eq!(format!("{}", ErrorSeverity::Warning), "WARNING");
        assert_eq!(format!("{}", ErrorSeverity::Info), "INFO");
    }
}
