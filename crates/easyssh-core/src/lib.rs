//! A non-sensitive session manager for the system OpenSSH tools.
//!
//! This crate never implements an SSH protocol, stores credentials, or talks
//! to a password manager. Authentication remains entirely with `ssh`, `scp`,
//! and the system SSH agent inherited by their child processes.

pub mod config;
pub mod domain;
pub mod openssh;
pub mod pty;
pub mod security;
pub mod transfer;

pub use config::{ConfigStore, MigrationReport};
pub use domain::{AppConfig, Connection, ConnectionTarget, Group, TerminalPreferences, Theme};
pub use openssh::{AgentDiagnostics, OpenSsh, OpenSshError, SshInvocation};
pub use pty::TerminalSession;
pub use transfer::{ScpInvocation, Transfer, TransferDirection, TransferStatus};
