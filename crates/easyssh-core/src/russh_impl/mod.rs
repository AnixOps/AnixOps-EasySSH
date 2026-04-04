//! Russh-based SSH implementation for EasySSH
//!
//! This module provides a pure Rust SSH implementation using the `russh` crate,
//! which avoids OpenSSL dependencies and provides better security guarantees.
//!
//! # Architecture
//!
//! ```text
//! RusshClient
//!     ├── RusshConfig (connection configuration)
//!     ├── RusshSession (active SSH session)
//!     │     ├── ShellChannel (PTY shell)
//!     │     ├── ExecChannel (command execution)
//!     │     └── SftpSession (file transfer)
//!     └── RusshSessionManager (connection pooling)
//! ```
//!
//! # Feature Flags
//!
//! This module is only available when `russh-backend` feature is enabled:
//! ```toml
//! [features]
//! russh-backend = ["dep:russh", "dep:russh-sftp", "dep:async-trait"]
//! ```
//!
//! # Comparison with ssh2-backend
//!
//! | Aspect | ssh2-backend | russh-backend |
//! |--------|--------------|---------------|
//! | OpenSSL | Yes | No |
//! | Pure Rust | No | Yes |
//! | Security Audits | libssh2 audited | russh audited |
//! | Async Support | Blocking only | Native async |
//! | Platform Support | All | All |
//!
//! # Example
//!
//! ```rust,no_run
//! use easyssh_core::russh_impl::{RusshClient, RusshConfig, RusshAuthMethod};
//!
//! async fn connect_example() {
//!     let config = RusshConfig::with_password(
//!         "192.168.1.1",
//!         22,
//!         "root",
//!         "password"
//!     );
//!
//!     let client = RusshClient::new(config);
//!     let session = client.connect().await.unwrap();
//!
//!     // Execute a command
//!     let output = session.exec("uname -a").await.unwrap();
//!     println!("Output: {}", output);
//!
//!     // Disconnect
//!     session.disconnect().await.unwrap();
//! }
//! ```
//!
//! # System Invariants Compliance
//!
//! This implementation follows all constraints defined in `SYSTEM_INVARIANTS.md`:
//!
//! - **Section 2.1**: Connection state machine (Idle -> Connecting -> Active/Failed)
//! - **Section 2.2**: Connection pool limits (max 100 connections, 30min idle timeout)
//! - **Section 2.3**: Authentication security (keychain storage, memory clearing)
//! - **Section 5**: Automatic reconnect with exponential backoff
//!
//! # References
//!
//! - [russh crate](https://crates.io/crates/russh)
//! - [russh-sftp crate](https://crates.io/crates/russh-sftp)
//! - [OxideTerm SSH implementation](https://github.com/AnalyseDeCircuit/oxideterm)

pub mod config;
pub mod error;
pub mod client;
pub mod session;
pub mod channel;
pub mod manager;

#[cfg(test)]
mod tests;

// Re-export main types for convenience
pub use config::{RusshConfig, RusshAuthMethod, RusshKnownHostsPolicy, RusshTimeout};
pub use error::{RusshError, RusshResult};
pub use client::{RusshClient, RusshConnectionTestResult, ReconnectConfig as RusshReconnectConfig};
pub use session::{RusshSession, RusshSessionState, RusshSessionMetadata};
pub use channel::{RusshChannel, RusshShellChannel, RusshExecResult, ScrollBuffer};
pub use manager::{RusshSessionManager, RusshPoolStats};