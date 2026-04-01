use thiserror::Error;

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
}

/// Get the translation key for an error
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
            LiteError::GitMergeConflict { .. } => "git-merge-conflict",
            LiteError::FeatureNotAvailable { .. } => "error-feature-not-available",
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
            | LiteError::RemoteDesktop(msg) => {
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
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize the translation key instead of the display string
        // This allows the frontend to translate the error
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

#[cfg(feature = "git")]
impl From<crate::git_types::GitError> for LiteError {
    fn from(e: crate::git_types::GitError) -> Self {
        match e {
            crate::git_types::GitError::MergeConflict(msg) => {
                LiteError::GitMergeConflict { files: vec![msg] }
            }
            _ => LiteError::Git(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_translation_key() {
        let err = LiteError::Database("connection failed".to_string());
        assert_eq!(err.translation_key(), "error-database");

        let err = LiteError::SshTimeout;
        assert_eq!(err.translation_key(), "connection-timeout");
    }

    #[test]
    fn test_error_translation_args() {
        let err = LiteError::Database("connection failed".to_string());
        let args = err.translation_args();
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].0, "message");
        assert_eq!(args[0].1, "connection failed");
    }

    #[test]
    fn test_error_serialize_translation_key() {
        let err = LiteError::AuthFailed;
        let json = serde_json::to_string(&err).expect("Failed to serialize");
        assert!(json.contains("error-auth-failed"));
    }

    #[test]
    fn test_error_display_keys() {
        // Errors should now display translation keys
        let err = LiteError::Database("test".to_string());
        assert_eq!(err.to_string(), "error-database");
    }

    #[test]
    fn test_from_rusqlite_error() {
        let sqlite_err = rusqlite::Error::InvalidQuery;
        let err: LiteError = sqlite_err.into();
        assert!(matches!(err, LiteError::Database(_)));
        assert_eq!(err.translation_key(), "error-database");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: LiteError = io_err.into();
        assert!(matches!(err, LiteError::Io(_)));
        assert_eq!(err.translation_key(), "error-io");
    }

    #[test]
    fn test_all_error_variants() {
        // Ensure all variants can be created and provide translation keys
        let _ = LiteError::Database("test".to_string());
        let _ = LiteError::Crypto("test".to_string());
        let _ = LiteError::Keychain("test".to_string());
        let _ = LiteError::Terminal("test".to_string());
        let _ = LiteError::TerminalEmulator("test".to_string());
        let _ = LiteError::Layout("test".to_string());
        let _ = LiteError::Team("test".to_string());
        let _ = LiteError::Audit("test".to_string());
        let _ = LiteError::Rbac("test".to_string());
        let _ = LiteError::Ssh("test".to_string());
        let _ = LiteError::Config("test".to_string());
        let _ = LiteError::Io("test".to_string());
        let _ = LiteError::Json("test".to_string());
        let _ = LiteError::ServerNotFound("test".to_string());
        let _ = LiteError::GroupNotFound("test".to_string());
        let _ = LiteError::AuthFailed;
        let _ = LiteError::InvalidMasterPassword;
        let _ = LiteError::SessionPoolFull;
        let _ = LiteError::ConnectionReset;
        let _ = LiteError::ImportFailed("test".to_string());
        let _ = LiteError::ExportFailed("test".to_string());
        let _ = LiteError::FileNotFound {
            path: "/test".to_string(),
        };
        let _ = LiteError::InvalidKey("test".to_string());
        let _ = LiteError::Telemetry("test".to_string());
        let _ = LiteError::Docker("test".to_string());
        let _ = LiteError::Git("test".to_string());
        let _ = LiteError::GitMergeConflict {
            files: vec!["file.txt".to_string()],
        };
        let _ = LiteError::FeatureNotAvailable {
            feature: "pro".to_string(),
            edition: "lite".to_string(),
        };
    }
}
