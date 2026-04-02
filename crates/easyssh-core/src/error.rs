//! EasySSH Error Handling System
//!
//! This module provides unified error types for the EasySSH application.
//! It uses `thiserror` for ergonomic error definitions and provides:
//!
//! - Hierarchical error types (EasySSHErrors as the top-level enum)
//! - Automatic error conversion via #[from] attributes
//! - User-friendly error messages with Chinese localization
//! - Error codes for searching and debugging (e.g., E1000-E9900)
//! - Retry suggestions for transient errors
//! - Error recovery strategies for automatic error handling
//!
//! # Error Code Ranges
//!
//! | Range | Category |
//! |-------|----------|
//! | E1000-E1099 | Crypto errors |
//! | E1100-E1199 | Authentication errors |
//! | E1200-E1299 | Not found errors |
//! | E1300-E1399 | Permission errors |
//! | E1400-E1499 | Feature availability |
//! | E2000-E2999 | Database errors |
//! | E3000-E3999 | SSH errors |
//! | E4000-E4999 | I/O errors |
//! | E5000-E5999 | Configuration errors |
//! | E6000-E6999 | Validation errors |
//! | E7000-E7999 | User cancellation |
//! | E8000-E8999 | Serialization errors |
//! | E9000-E9999 | Network errors |
//! | E9900+ | Internal errors |
//!
//! # Example
//!
//! ```rust
//! use easyssh_core::error::{EasySSHErrors, Result, ErrorRecovery};
//!
//! fn may_fail() -> Result<String> {
//!     // Returns EasySSHErrors::Io on failure
//!     let content = std::fs::read_to_string("config.txt")?;
//!     Ok(content)
//! }
//!
//! fn handle_with_recovery() -> Result<String> {
//!     match may_fail() {
//!         Ok(v) => Ok(v),
//!         Err(e) => {
//!             // Try automatic recovery
//!             if let Some(recovery) = e.recovery_strategy() {
//!                 recovery.attempt_recover(&e)
//!             } else {
//!                 Err(e)
//!             }
//!         }
//!     }
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

/// Error recovery strategies for automatic error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Retry with exponential backoff
    RetryWithBackoff { max_attempts: u32, base_delay_ms: u64 },
    /// Refresh connection/session
    RefreshConnection,
    /// Re-initialize component
    Reinitialize,
    /// Clear cache and retry
    ClearCacheRetry,
    /// Fallback to offline mode
    FallbackOffline,
    /// No recovery possible
    NoRecovery,
}

impl RecoveryStrategy {
    /// Attempt to recover from the given error
    pub fn attempt_recover<T, F>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Result<T>,
    {
        match self {
            RecoveryStrategy::RetryWithBackoff { max_attempts, base_delay_ms } => {
                Self::retry_with_backoff(*max_attempts, *base_delay_ms, operation)
            }
            _ => {
                // For other strategies, caller needs to handle them specifically
                operation()
            }
        }
    }

    /// Retry operation with exponential backoff
    fn retry_with_backoff<T, F>(max_attempts: u32, base_delay_ms: u64, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Result<T>,
    {
        for attempt in 0..max_attempts {
            match operation() {
                Ok(result) => return Ok(result),
                Err(_e) if attempt < max_attempts - 1 => {
                    let delay = base_delay_ms * (2_u64.pow(attempt));
                    std::thread::sleep(std::time::Duration::from_millis(delay.min(30000)));
                }
                Err(e) => return Err(e),
            }
        }
        panic!("Retry loop exited unexpectedly")
    }

    /// Get human-readable description of the strategy
    pub fn description(&self) -> &'static str {
        match self {
            RecoveryStrategy::RetryWithBackoff { .. } => "稍后自动重试",
            RecoveryStrategy::RefreshConnection => "刷新连接",
            RecoveryStrategy::Reinitialize => "重新初始化",
            RecoveryStrategy::ClearCacheRetry => "清除缓存并重试",
            RecoveryStrategy::FallbackOffline => "切换至离线模式",
            RecoveryStrategy::NoRecovery => "需要手动处理",
        }
    }
}

/// Trait for types that can provide error recovery strategies
pub trait ErrorRecovery {
    /// Get the recommended recovery strategy for this error
    fn recovery_strategy(&self) -> Option<RecoveryStrategy>;
    /// Attempt automatic recovery
    fn attempt_recovery<T, F>(&self, operation: F) -> EasySSHResult<T>
    where
        F: FnMut() -> EasySSHResult<T>;
}

impl ErrorRecovery for EasySSHErrors {
    fn recovery_strategy(&self) -> Option<RecoveryStrategy> {
        match self {
            // Network errors - retry with backoff
            EasySSHErrors::Ssh(CoreSshError::ConnectionFailed { .. }) => {
                Some(RecoveryStrategy::RetryWithBackoff { max_attempts: 3, base_delay_ms: 1000 })
            }
            EasySSHErrors::Ssh(CoreSshError::ConnectionTimeout) => {
                Some(RecoveryStrategy::RetryWithBackoff { max_attempts: 3, base_delay_ms: 2000 })
            }
            EasySSHErrors::Ssh(CoreSshError::SessionDisconnected(_)) => {
                Some(RecoveryStrategy::RefreshConnection)
            }
            EasySSHErrors::Network(_) => {
                Some(RecoveryStrategy::RetryWithBackoff { max_attempts: 5, base_delay_ms: 1000 })
            }
            EasySSHErrors::Timeout => {
                Some(RecoveryStrategy::RetryWithBackoff { max_attempts: 3, base_delay_ms: 1000 })
            }

            // Database errors
            EasySSHErrors::Database(CoreDatabaseError::LockTimeout) => {
                Some(RecoveryStrategy::RetryWithBackoff { max_attempts: 5, base_delay_ms: 100 })
            }
            EasySSHErrors::Database(CoreDatabaseError::Connection(_)) => {
                Some(RecoveryStrategy::Reinitialize)
            }

            // I/O errors
            EasySSHErrors::Io(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                Some(RecoveryStrategy::RetryWithBackoff { max_attempts: 3, base_delay_ms: 100 })
            }
            EasySSHErrors::Io(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                Some(RecoveryStrategy::RetryWithBackoff { max_attempts: 3, base_delay_ms: 50 })
            }

            // No recovery for these
            EasySSHErrors::Crypto(CoreCryptoError::InvalidMasterPassword) => None,
            EasySSHErrors::Authentication(_) => None,
            EasySSHErrors::PermissionDenied(_) => None,
            EasySSHErrors::Validation(_) => None,
            EasySSHErrors::UserCancelled => None,
            EasySSHErrors::FeatureNotAvailable { .. } => None,
            _ => None,
        }
    }

    fn attempt_recovery<T, F>(&self, mut operation: F) -> EasySSHResult<T>
    where
        F: FnMut() -> EasySSHResult<T>,
    {
        if let Some(strategy) = self.recovery_strategy() {
            match strategy {
                RecoveryStrategy::RetryWithBackoff { max_attempts, base_delay_ms } => {
                    RecoveryStrategy::retry_with_backoff(max_attempts, base_delay_ms, &mut operation)
                }
                _ => operation(),
            }
        } else {
            Err(self.clone())
        }
    }
}

impl Clone for EasySSHErrors {
    fn clone(&self) -> Self {
        match self {
            EasySSHErrors::Crypto(e) => EasySSHErrors::Crypto(e.clone()),
            EasySSHErrors::Database(e) => EasySSHErrors::Database(e.clone()),
            EasySSHErrors::Ssh(e) => EasySSHErrors::Ssh(e.clone()),
            EasySSHErrors::Io(e) => EasySSHErrors::Io(std::io::Error::new(e.kind(), e.to_string())),
            EasySSHErrors::Config(s) => EasySSHErrors::Config(s.clone()),
            EasySSHErrors::Validation(s) => EasySSHErrors::Validation(s.clone()),
            EasySSHErrors::UserCancelled => EasySSHErrors::UserCancelled,
            EasySSHErrors::Serialization(e) => {
                // Can't clone serde_json::Error directly, convert to string
                EasySSHErrors::Serialization(serde_json::from_str::<serde_json::Value>("null").unwrap_err())
            }
            EasySSHErrors::Network(s) => EasySSHErrors::Network(s.clone()),
            EasySSHErrors::Authentication(s) => EasySSHErrors::Authentication(s.clone()),
            EasySSHErrors::Timeout => EasySSHErrors::Timeout,
            EasySSHErrors::NotFound(s) => EasySSHErrors::NotFound(s.clone()),
            EasySSHErrors::PermissionDenied(s) => EasySSHErrors::PermissionDenied(s.clone()),
            EasySSHErrors::FeatureNotAvailable { feature, edition } => {
                EasySSHErrors::FeatureNotAvailable {
                    feature: feature.clone(),
                    edition: edition.clone(),
                }
            }
            EasySSHErrors::Internal(s) => EasySSHErrors::Internal(s.clone()),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Critical,
    Error,
    Warning,
    Info,
}

/// Error display information for user-friendly messages
pub struct ErrorDisplay {
    /// Error code for searching documentation (e.g., "E1001")
    pub code: String,
    /// User-friendly message (translated to Chinese)
    pub message: String,
    /// Suggested action for the user
    pub suggestion: Option<String>,
    /// Whether the operation can be retried
    pub retryable: bool,
    /// Error severity
    pub severity: ErrorSeverity,
    /// Recovery strategy if available
    pub recovery: Option<RecoveryStrategy>,
}

impl EasySSHErrors {
    /// Create a configuration error
    pub fn configuration<T: Into<String>>(msg: T) -> Self {
        EasySSHErrors::Config(msg.into())
    }

    /// Get the detailed error code for documentation lookup
    ///
    /// Error codes follow this pattern:
    /// - E1000-E1099: Crypto errors
    /// - E1100-E1199: Authentication errors
    /// - E1200-E1299: Not found errors
    /// - E1300-E1399: Permission errors
    /// - E1400-E1499: Feature availability
    /// - E2000-E2999: Database errors
    /// - E3000-E3999: SSH errors
    /// - E4000-E4999: I/O errors
    /// - E5000-E5999: Configuration errors
    /// - E6000-E6999: Validation errors
    /// - E7000-E7999: User cancellation
    /// - E8000-E8999: Serialization errors
    /// - E9000-E9999: Network errors
    /// - E9900+: Internal errors
    pub fn error_code(&self) -> String {
        match self {
            // Crypto errors - E1000
            EasySSHErrors::Crypto(e) => match e {
                CoreCryptoError::KeyDerivation(_) => "E1001".to_string(),
                CoreCryptoError::Encryption(_) => "E1002".to_string(),
                CoreCryptoError::Decryption(_) => "E1003".to_string(),
                CoreCryptoError::InvalidMasterPassword => "E1004".to_string(),
                CoreCryptoError::Keychain(_) => "E1005".to_string(),
                CoreCryptoError::InvalidKeyFormat(_) => "E1006".to_string(),
                CoreCryptoError::RngError(_) => "E1007".to_string(),
            },
            // Database errors - E2000
            EasySSHErrors::Database(e) => match e {
                CoreDatabaseError::Connection(_) => "E2001".to_string(),
                CoreDatabaseError::Query(_) => "E2002".to_string(),
                CoreDatabaseError::Migration(_) => "E2003".to_string(),
                CoreDatabaseError::UniqueViolation(_) => "E2004".to_string(),
                CoreDatabaseError::ForeignKeyViolation(_) => "E2005".to_string(),
                CoreDatabaseError::RecordNotFound { .. } => "E2006".to_string(),
                CoreDatabaseError::Transaction(_) => "E2007".to_string(),
                CoreDatabaseError::LockTimeout => "E2008".to_string(),
            },
            // SSH errors - E3000
            EasySSHErrors::Ssh(e) => match e {
                CoreSshError::ConnectionFailed { .. } => "E3001".to_string(),
                CoreSshError::AuthFailed { .. } => "E3002".to_string(),
                CoreSshError::SessionNotFound(_) => "E3003".to_string(),
                CoreSshError::SessionDisconnected(_) => "E3004".to_string(),
                CoreSshError::ConnectionTimeout => "E3005".to_string(),
                CoreSshError::ChannelFailed(_) => "E3006".to_string(),
                CoreSshError::CommandFailed(_) => "E3007".to_string(),
                CoreSshError::Sftp(_) => "E3008".to_string(),
                CoreSshError::PortForward(_) => "E3009".to_string(),
                CoreSshError::ProxyJump(_) => "E3010".to_string(),
                CoreSshError::HostKeyVerification(_) => "E3011".to_string(),
                CoreSshError::SessionPoolFull => "E3012".to_string(),
            },
            // I/O errors - E4000
            EasySSHErrors::Io(e) => match e.kind() {
                std::io::ErrorKind::NotFound => "E4001".to_string(),
                std::io::ErrorKind::PermissionDenied => "E4002".to_string(),
                std::io::ErrorKind::ConnectionRefused => "E4003".to_string(),
                std::io::ErrorKind::ConnectionReset => "E4004".to_string(),
                std::io::ErrorKind::ConnectionAborted => "E4005".to_string(),
                std::io::ErrorKind::NotConnected => "E4006".to_string(),
                std::io::ErrorKind::AddrInUse => "E4007".to_string(),
                std::io::ErrorKind::AddrNotAvailable => "E4008".to_string(),
                std::io::ErrorKind::BrokenPipe => "E4009".to_string(),
                std::io::ErrorKind::AlreadyExists => "E4010".to_string(),
                std::io::ErrorKind::WouldBlock => "E4011".to_string(),
                std::io::ErrorKind::InvalidInput => "E4012".to_string(),
                std::io::ErrorKind::InvalidData => "E4013".to_string(),
                std::io::ErrorKind::TimedOut => "E4014".to_string(),
                std::io::ErrorKind::WriteZero => "E4015".to_string(),
                std::io::ErrorKind::Interrupted => "E4016".to_string(),
                std::io::ErrorKind::UnexpectedEof => "E4017".to_string(),
                std::io::ErrorKind::OutOfMemory => "E4018".to_string(),
                _ => "E4099".to_string(),
            },
            // Config errors - E5000
            EasySSHErrors::Config(_) => "E5001".to_string(),
            // Validation errors - E6000
            EasySSHErrors::Validation(_) => "E6001".to_string(),
            // User cancelled - E7000
            EasySSHErrors::UserCancelled => "E7001".to_string(),
            // Serialization errors - E8000
            EasySSHErrors::Serialization(_) => "E8001".to_string(),
            // Network errors - E9000
            EasySSHErrors::Network(_) => "E9001".to_string(),
            // Authentication errors - E1100
            EasySSHErrors::Authentication(_) => "E1101".to_string(),
            // Timeout - E1100
            EasySSHErrors::Timeout => "E1102".to_string(),
            // Not found - E1200
            EasySSHErrors::NotFound(_) => "E1201".to_string(),
            // Permission denied - E1300
            EasySSHErrors::PermissionDenied(_) => "E1301".to_string(),
            // Feature not available - E1400
            EasySSHErrors::FeatureNotAvailable { .. } => "E1401".to_string(),
            // Internal errors - E9900
            EasySSHErrors::Internal(_) => "E9901".to_string(),
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
            EasySSHErrors::Crypto(CoreCryptoError::InvalidMasterPassword) => {
                ErrorSeverity::Critical
            }
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

    /// Get detailed user-friendly error message in Chinese
    pub fn user_message(&self) -> String {
        let base_msg = self.to_string();
        let suggestion = self.suggestion_message();

        if let Some(sugg) = suggestion {
            format!("{}\n\n💡 建议: {}", base_msg, sugg)
        } else {
            base_msg
        }
    }

    /// Get user-friendly suggestion message
    fn suggestion_message(&self) -> Option<String> {
        match self {
            // Crypto errors
            EasySSHErrors::Crypto(CoreCryptoError::InvalidMasterPassword) => {
                Some("请确认主密码正确，或使用密码重置功能".to_string())
            }
            EasySSHErrors::Crypto(CoreCryptoError::KeyDerivation(_)) => {
                Some("请检查系统内存是否充足".to_string())
            }
            EasySSHErrors::Crypto(CoreCryptoError::InvalidKeyFormat(_)) => {
                Some("请检查密钥格式是否为OpenSSH或PEM格式".to_string())
            }

            // Database errors
            EasySSHErrors::Database(CoreDatabaseError::Connection(_)) => {
                Some("请检查数据库文件是否存在且未被其他程序占用".to_string())
            }
            EasySSHErrors::Database(CoreDatabaseError::UniqueViolation(_)) => {
                Some("该名称已存在，请使用其他名称".to_string())
            }
            EasySSHErrors::Database(CoreDatabaseError::RecordNotFound { table, id }) => {
                Some(format!("请在{}列表中检查ID '{}'是否存在", table, id))
            }

            // SSH errors
            EasySSHErrors::Ssh(CoreSshError::ConnectionFailed { host, port, message }) => {
                Some(format!(
                    "请检查:\n1. 服务器 {}:{} 是否在线\n2. 防火墙是否允许SSH连接\n3. 网络连接是否正常\n\n错误详情: {}",
                    host, port, message
                ))
            }
            EasySSHErrors::Ssh(CoreSshError::AuthFailed { host, username }) => {
                Some(format!(
                    "请检查:\n1. 用户名 '{}' 是否正确\n2. 密码或私钥是否正确\n3. 服务器 {} 是否允许该用户登录",
                    username, host
                ))
            }
            EasySSHErrors::Ssh(CoreSshError::HostKeyVerification(_)) => {
                Some("服务器身份验证失败。如果更换了服务器，请在设置中清除该主机的已知主机密钥".to_string())
            }
            EasySSHErrors::Ssh(CoreSshError::ConnectionTimeout) => {
                Some("连接超时，请检查:\n1. 服务器地址和端口是否正确\n2. 网络是否可达\n3. 防火墙设置".to_string())
            }

            // I/O errors
            EasySSHErrors::Io(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Some("找不到指定文件或目录，请检查路径是否正确".to_string())
            }
            EasySSHErrors::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                Some("权限不足，请检查文件权限或以管理员身份运行".to_string())
            }

            // Config errors
            EasySSHErrors::Config(_) => {
                Some("请检查配置文件格式或重置为默认配置".to_string())
            }

            // Validation errors
            EasySSHErrors::Validation(msg) => {
                Some(format!("请检查输入: {}", msg))
            }

            // Feature not available
            EasySSHErrors::FeatureNotAvailable { feature, edition } => {
                Some(format!("此功能 '{}' 仅在 {} 版本中可用，请升级您的版本", feature, edition))
            }

            // Not found
            EasySSHErrors::NotFound(item) => {
                Some(format!("'{}' 不存在，请检查名称拼写或重新创建", item))
            }

            // Permission denied
            EasySSHErrors::PermissionDenied(action) => {
                Some(format!("您没有权限执行 '{}'，请联系管理员", action))
            }

            // Network errors
            EasySSHErrors::Network(msg) => {
                Some(format!("网络问题: {}，请检查网络连接", msg))
            }

            // Authentication
            EasySSHErrors::Authentication(msg) => {
                Some(format!("认证失败: {}，请检查凭证", msg))
            }

            _ => None,
        }
    }

    /// Get the error display information
    pub fn display_info(&self) -> ErrorDisplay {
        ErrorDisplay {
            code: self.error_code(),
            message: self.user_message(),
            suggestion: self.suggestion_message(),
            retryable: self.is_retryable(),
            severity: self.severity(),
            recovery: self.recovery_strategy(),
        }
    }

    /// Get detailed context information for debugging
    ///
    /// Returns a vector of key-value pairs containing:
    /// - Error code
    /// - Error category
    /// - Detailed context from the specific error
    /// - Recovery strategy if available
    pub fn context(&self) -> Vec<(&'static str, String)> {
        let mut ctx = vec![
            ("error_code", self.error_code()),
            ("error_type", self.error_type_name()),
            ("retryable", self.is_retryable().to_string()),
            ("severity", format!("{:?}", self.severity())),
        ];

        if let Some(recovery) = self.recovery_strategy() {
            ctx.push(("recovery", recovery.description().to_string()));
        }

        // Add specific context based on error type
        match self {
            EasySSHErrors::Crypto(e) => {
                ctx.push(("crypto_error", e.to_string()));
                ctx.push(("error_category", "crypto".to_string()));
            }
            EasySSHErrors::Database(e) => {
                ctx.push(("database_error", e.to_string()));
                ctx.push(("error_category", "database".to_string()));
                if let CoreDatabaseError::RecordNotFound { table, id } = e {
                    ctx.push(("table", table.clone()));
                    ctx.push(("record_id", id.clone()));
                }
            }
            EasySSHErrors::Ssh(CoreSshError::ConnectionFailed {
                host,
                port,
                message,
            }) => {
                ctx.push(("host", host.clone()));
                ctx.push(("port", port.to_string()));
                ctx.push(("message", message.clone()));
                ctx.push(("error_category", "ssh_connection".to_string()));
            }
            EasySSHErrors::Ssh(CoreSshError::AuthFailed { host, username }) => {
                ctx.push(("host", host.clone()));
                ctx.push(("username", username.clone()));
                ctx.push(("error_category", "ssh_auth".to_string()));
            }
            EasySSHErrors::Ssh(CoreSshError::SessionNotFound(id))
            | EasySSHErrors::Ssh(CoreSshError::SessionDisconnected(id)) => {
                ctx.push(("session_id", id.clone()));
                ctx.push(("error_category", "ssh_session".to_string()));
            }
            EasySSHErrors::Ssh(e) => {
                ctx.push(("ssh_error", e.to_string()));
                ctx.push(("error_category", "ssh".to_string()));
            }
            EasySSHErrors::Io(e) => {
                ctx.push(("io_kind", format!("{:?}", e.kind())));
                ctx.push(("io_message", e.to_string()));
                ctx.push(("error_category", "io".to_string()));
            }
            EasySSHErrors::NotFound(item) => {
                ctx.push(("item", item.clone()));
                ctx.push(("error_category", "not_found".to_string()));
            }
            EasySSHErrors::Config(key) => {
                ctx.push(("config_key", key.clone()));
                ctx.push(("error_category", "config".to_string()));
            }
            EasySSHErrors::Validation(msg) => {
                ctx.push(("validation_msg", msg.clone()));
                ctx.push(("error_category", "validation".to_string()));
            }
            EasySSHErrors::Network(msg) => {
                ctx.push(("network_msg", msg.clone()));
                ctx.push(("error_category", "network".to_string()));
            }
            EasySSHErrors::Authentication(msg) => {
                ctx.push(("auth_msg", msg.clone()));
                ctx.push(("error_category", "authentication".to_string()));
            }
            EasySSHErrors::PermissionDenied(msg) => {
                ctx.push(("permission_msg", msg.clone()));
                ctx.push(("error_category", "permission".to_string()));
            }
            EasySSHErrors::Internal(msg) => {
                ctx.push(("internal_msg", msg.clone()));
                ctx.push(("error_category", "internal".to_string()));
            }
            EasySSHErrors::FeatureNotAvailable { feature, edition } => {
                ctx.push(("feature", feature.clone()));
                ctx.push(("current_edition", edition.clone()));
                ctx.push(("error_category", "feature_unavailable".to_string()));
            }
            EasySSHErrors::Serialization(e) => {
                ctx.push(("serialization_error", e.to_string()));
                ctx.push(("error_category", "serialization".to_string()));
            }
            EasySSHErrors::Timeout => {
                ctx.push(("error_category", "timeout".to_string()));
            }
            EasySSHErrors::UserCancelled => {
                ctx.push(("error_category", "user_cancelled".to_string()));
            }
        }

        ctx
    }

    /// Get the error type name for categorization
    fn error_type_name(&self) -> String {
        match self {
            EasySSHErrors::Crypto(_) => "crypto",
            EasySSHErrors::Database(_) => "database",
            EasySSHErrors::Ssh(_) => "ssh",
            EasySSHErrors::Io(_) => "io",
            EasySSHErrors::Config(_) => "config",
            EasySSHErrors::Validation(_) => "validation",
            EasySSHErrors::UserCancelled => "user_cancelled",
            EasySSHErrors::Serialization(_) => "serialization",
            EasySSHErrors::Network(_) => "network",
            EasySSHErrors::Authentication(_) => "authentication",
            EasySSHErrors::Timeout => "timeout",
            EasySSHErrors::NotFound(_) => "not_found",
            EasySSHErrors::PermissionDenied(_) => "permission_denied",
            EasySSHErrors::FeatureNotAvailable { .. } => "feature_unavailable",
            EasySSHErrors::Internal(_) => "internal",
        }
        .to_string()
    }

    /// Format error for log output with full context
    pub fn format_for_log(&self) -> String {
        let mut parts = vec![
            format!("[{}] {}", self.error_code(), self.error_type_name()),
            self.to_string(),
        ];

        let context = self.context();
        if !context.is_empty() {
            parts.push("Context:".to_string());
            for (k, v) in context {
                parts.push(format!("  {}: {}", k, v));
            }
        }

        parts.join("\n")
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
            LiteError::Io(msg) => EasySSHErrors::Io(std::io::Error::other(msg)),
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
            LiteError::SshSessionNotFound(id) => {
                EasySSHErrors::Ssh(CoreSshError::SessionNotFound(id))
            }
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
    fn custom<T: fmt::Display>(_msg: T) -> Self {
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
        assert_eq!(err.error_code(), "E5001");

        let err = EasySSHErrors::UserCancelled;
        assert_eq!(err.error_code(), "E7001");

        // Test detailed error codes
        let err = EasySSHErrors::Crypto(CoreCryptoError::InvalidMasterPassword);
        assert_eq!(err.error_code(), "E1004");

        let err = EasySSHErrors::Ssh(CoreSshError::ConnectionTimeout);
        assert_eq!(err.error_code(), "E3005");
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
        assert_eq!(info.code, "E3005");
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
        // Now includes: error_code, error_type, retryable, severity, host, port, message, error_category
        assert!(context.len() >= 7);
        assert!(context.iter().any(|(k, _)| *k == "host"));
        assert!(context.iter().any(|(k, _)| *k == "port"));
        assert!(context.iter().any(|(k, _)| *k == "error_code"));
    }

    #[test]
    fn test_recovery_strategy() {
        // Should have retry strategy
        let err = EasySSHErrors::Ssh(CoreSshError::ConnectionFailed {
            host: "test".to_string(),
            port: 22,
            message: "timeout".to_string(),
        });
        let recovery = err.recovery_strategy();
        assert!(recovery.is_some());
        assert!(matches!(recovery.unwrap(), RecoveryStrategy::RetryWithBackoff { .. }));

        // Should not have recovery strategy
        let err = EasySSHErrors::Crypto(CoreCryptoError::InvalidMasterPassword);
        assert!(err.recovery_strategy().is_none());

        let err = EasySSHErrors::UserCancelled;
        assert!(err.recovery_strategy().is_none());
    }

    #[test]
    fn test_user_message() {
        let err = EasySSHErrors::Ssh(CoreSshError::ConnectionFailed {
            host: "192.168.1.1".to_string(),
            port: 22,
            message: "timeout".to_string(),
        });
        let msg = err.user_message();
        assert!(msg.contains("192.168.1.1"));
        assert!(msg.contains("💡 建议"));
    }

    #[test]
    fn test_format_for_log() {
        let err = EasySSHErrors::Config("test key".to_string());
        let log_msg = err.format_for_log();
        assert!(log_msg.contains("E5001"));
        assert!(log_msg.contains("config"));
        assert!(log_msg.contains("Context"));
    }

    #[test]
    fn test_io_error_codes() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = EasySSHErrors::Io(io_err);
        assert_eq!(err.error_code(), "E4001");

        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = EasySSHErrors::Io(io_err);
        assert_eq!(err.error_code(), "E4002");
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
        assert_eq!(err.error_code(), "E1401");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", ErrorSeverity::Critical), "CRITICAL");
        assert_eq!(format!("{}", ErrorSeverity::Error), "ERROR");
        assert_eq!(format!("{}", ErrorSeverity::Warning), "WARNING");
        assert_eq!(format!("{}", ErrorSeverity::Info), "INFO");
    }

    #[test]
    fn test_recovery_strategy_descriptions() {
        assert_eq!(RecoveryStrategy::RetryWithBackoff { max_attempts: 3, base_delay_ms: 1000 }.description(), "稍后自动重试");
        assert_eq!(RecoveryStrategy::RefreshConnection.description(), "刷新连接");
        assert_eq!(RecoveryStrategy::NoRecovery.description(), "需要手动处理");
    }

    #[test]
    fn test_log_stats_format() {
        // This is for LogStats struct in logger.rs but we can test ErrorDisplay here
        let err = EasySSHErrors::Timeout;
        let info = err.display_info();
        assert!(info.recovery.is_some()); // Timeout has recovery strategy
    }
}
