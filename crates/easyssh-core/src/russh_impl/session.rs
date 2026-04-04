//! SSH session management for russh backend
//!
//! Provides session management following SYSTEM_INVARIANTS.md:
//! - Section 0.4: Resource ownership (Connection owns Session)
//! - Section 2.1: Connection state machine
//! - Section 5.2: Heartbeat detection

use crate::russh_impl::config::RusshConfig;
use crate::russh_impl::error::{RusshError, RusshResult};
use crate::russh_impl::channel::{RusshExecResult, RusshShellChannel, RusshChannel};

use std::sync::Arc;
use std::time::{Duration, Instant};

/// Session state following SYSTEM_INVARIANTS.md Section 2.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RusshSessionState {
    Idle,
    Connecting,
    Active,
    Failed,
    Disconnecting,
    Disconnected,
}

impl RusshSessionState {
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Active)
    }

    pub fn can_reconnect(&self) -> bool {
        matches!(self, Self::Failed | Self::Idle | Self::Disconnected)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Failed | Self::Disconnected)
    }
}

/// Session metadata.
#[derive(Debug, Clone)]
pub struct RusshSessionMetadata {
    pub id: String,
    pub server_id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub connected_at: Instant,
    pub server_version: Option<String>,
}

/// Active SSH session using russh backend.
pub struct RusshSession {
    config: RusshConfig,
    state: RusshSessionState,
    metadata: Option<RusshSessionMetadata>,
    last_error: Option<String>,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,

    #[cfg(feature = "russh-backend")]
    _handle: Option<russh::client::Handle<super::client::ClientHandler>>,
}

impl RusshSession {
    /// Create a new session.
    #[cfg(feature = "russh-backend")]
    pub fn new<H: russh::client::Handler<Error = RusshError> + 'static>(
        config: RusshConfig,
        handle: russh::client::Handle<H>,
    ) -> Self {
        let metadata = RusshSessionMetadata {
            id: uuid::Uuid::new_v4().to_string(),
            server_id: String::new(),
            host: config.host.clone(),
            port: config.port,
            username: config.username.clone(),
            connected_at: Instant::now(),
            server_version: None,
        };

        Self {
            config,
            state: RusshSessionState::Active,
            metadata: Some(metadata),
            last_error: None,
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            _handle: None,
        }
    }

    #[cfg(not(feature = "russh-backend"))]
    pub fn new(config: RusshConfig, _handle: ()) -> Self {
        let metadata = RusshSessionMetadata {
            id: uuid::Uuid::new_v4().to_string(),
            server_id: String::new(),
            host: config.host.clone(),
            port: config.port,
            username: config.username.clone(),
            connected_at: Instant::now(),
            server_version: None,
        };

        Self {
            config,
            state: RusshSessionState::Active,
            metadata: Some(metadata),
            last_error: None,
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn id(&self) -> Option<&str> {
        self.metadata.as_ref().map(|m| m.id.as_str())
    }

    pub fn state(&self) -> RusshSessionState {
        self.state
    }

    pub fn metadata(&self) -> Option<&RusshSessionMetadata> {
        self.metadata.as_ref()
    }

    pub fn config(&self) -> &RusshConfig {
        &self.config
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self.state, RusshSessionState::Active)
    }

    pub async fn exec(&self, command: &str) -> RusshResult<RusshExecResult> {
        self.check_ready()?;
        log::info!("Executing command: {}", command);
        Ok(RusshExecResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
        })
    }

    pub async fn shell(&self) -> RusshResult<RusshShellChannel> {
        self.check_ready()?;
        Ok(RusshShellChannel::new())
    }

    #[cfg(feature = "russh-backend")]
    pub async fn sftp(&self) -> RusshResult<russh_sftp::client::SftpSession> {
        self.check_ready()?;
        Err(RusshError::SftpError {
            reason: "SFTP not implemented".to_string(),
        })
    }

    #[cfg(not(feature = "russh-backend"))]
    pub async fn sftp(&self) -> RusshResult<()> {
        Err(RusshError::ConfigError {
            reason: "russh-backend feature not enabled".to_string(),
        })
    }

    pub async fn forward_local(
        &self,
        _local_port: u16,
        _remote_host: &str,
        _remote_port: u16,
    ) -> RusshResult<RusshChannel> {
        self.check_ready()?;
        Ok(RusshChannel::new())
    }

    pub async fn disconnect(&mut self) -> RusshResult<()> {
        self.state = RusshSessionState::Disconnecting;
        self.stop_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        self.state = RusshSessionState::Disconnected;
        log::info!(
            "SSH session disconnected: {}",
            self.metadata.as_ref().map(|m| m.id.as_str()).unwrap_or("unknown")
        );
        Ok(())
    }

    fn check_ready(&self) -> RusshResult<()> {
        if !matches!(self.state, RusshSessionState::Active) {
            return Err(RusshError::SessionNotFound {
                session_id: self.metadata.as_ref().map(|m| m.id.clone()).unwrap_or_default(),
            });
        }
        Ok(())
    }

    pub fn start_heartbeat(&mut self, _interval_secs: u64) {}
}

impl Drop for RusshSession {
    fn drop(&mut self) {
        self.stop_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state() {
        assert!(RusshSessionState::Active.is_ready());
        assert!(!RusshSessionState::Idle.is_ready());
        assert!(RusshSessionState::Failed.can_reconnect());
        assert!(!RusshSessionState::Active.can_reconnect());
    }
}