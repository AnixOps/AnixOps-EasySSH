//! A non-sensitive session manager for the system OpenSSH tools.
//!
//! This crate never implements an SSH protocol, stores credentials, or talks
//! to a password manager. Authentication remains entirely with `ssh`, `scp`,
//! and the system SSH agent inherited by their child processes.

pub mod config;
pub mod domain;
pub mod openssh;
pub mod security;
pub mod sync;
pub mod transfer;

pub use config::{ConfigStore, MigrationReport};
pub use domain::{
    AppConfig, CommandSnippet, Connection, ConnectionTarget, DisplayDensity, Group, SessionRecord,
    SidebarNavigation, SidebarPreferences, TerminalPreferences, Theme, Workspace,
};
pub use openssh::{AgentDiagnostics, ExternalTerminal, OpenSsh, OpenSshError, SshInvocation};
pub use sync::{GitSync, SyncError, SyncFile, SyncStatus};
pub use transfer::{ScpInvocation, Transfer, TransferDirection, TransferStatus};
