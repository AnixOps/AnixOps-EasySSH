//! A non-sensitive session manager for the system OpenSSH tools.
//!
//! This crate never implements an SSH protocol, stores credentials, or talks
//! to a password manager. Authentication remains entirely with `ssh`, `scp`,
//! and the system SSH agent inherited by their child processes.

pub mod config;
pub mod domain;
pub mod openssh;
pub mod remote;
pub mod security;
pub mod ssh_config;
pub mod sync;
pub mod transfer;
pub mod workspace;

pub use config::{ConfigStore, MigrationReport};
pub use domain::{
    AppConfig, CommandSnippet, Connection, ConnectionTarget, DisplayDensity, ExperimentalFeatures,
    Group, Locale, SessionRecord, SidebarNavigation, SidebarPreferences, TerminalPreferences,
    Theme, Workspace,
};
pub use openssh::{AgentDiagnostics, ExternalTerminal, OpenSsh, OpenSshError, SshInvocation};
pub use remote::{
    PosixRemoteAdapter, RemoteCapabilities, RemoteEntry, RemoteEntryType, RemoteFileError,
    RemoteFileService, RemoteFileState, RemotePlatformAdapter, RemotePlatformKind,
    UnsupportedRemoteAdapter, WindowsRemoteAdapter,
};
pub use ssh_config::{scan_default_ssh_config, scan_ssh_config, SshConfigDiscovery};
pub use sync::{GitSync, SyncError, SyncFile, SyncStatus};
pub use transfer::{
    cancel, ScpInvocation, SftpListingInvocation, Transfer, TransferDirection, TransferStatus,
};
pub use workspace::{
    inspect_text, remote_state_changed, EditSession, LineEnding, LocalFileInfo,
    WorkspaceTempManager,
};
