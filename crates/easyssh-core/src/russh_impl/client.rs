//! SSH client implementation for russh backend
//!
//! Provides the main SSH client with connection management following
//! SYSTEM_INVARIANTS.md constraints.

use crate::russh_impl::config::{RusshAuthMethod, RusshConfig, RusshKnownHostsPolicy};
use crate::russh_impl::error::{RusshError, RusshResult};

use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(feature = "russh-backend")]
use async_trait::async_trait;

#[cfg(feature = "russh-backend")]
use russh_keys::ssh_key;

/// Connection test result.
#[derive(Debug, Clone)]
pub struct RusshConnectionTestResult {
    pub success: bool,
    pub error: Option<String>,
    pub server_version: Option<String>,
    pub connect_time_ms: u64,
    pub auth_method: String,
    pub host_key_fingerprint: Option<String>,
}

impl RusshConnectionTestResult {
    pub fn success(auth_method: impl Into<String>, connect_time_ms: u64) -> Self {
        Self {
            success: true,
            error: None,
            server_version: None,
            connect_time_ms,
            auth_method: auth_method.into(),
            host_key_fingerprint: None,
        }
    }

    pub fn failed(error: impl Into<String>, auth_method: impl Into<String>, connect_time_ms: u64) -> Self {
        Self {
            success: false,
            error: Some(error.into()),
            server_version: None,
            connect_time_ms,
            auth_method: auth_method.into(),
            host_key_fingerprint: None,
        }
    }
}

/// Client handler for russh
#[cfg(feature = "russh-backend")]
pub struct ClientHandler {
    known_hosts_policy: RusshKnownHostsPolicy,
}

#[cfg(feature = "russh-backend")]
impl russh::client::Handler for ClientHandler {
    type Error = RusshError;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        match self.known_hosts_policy {
            RusshKnownHostsPolicy::Ignore => Ok(true),
            RusshKnownHostsPolicy::Add | RusshKnownHostsPolicy::AcceptNew => Ok(true),
            RusshKnownHostsPolicy::Strict => Ok(true),
        }
    }
}

/// Russh-based SSH client.
pub struct RusshClient {
    config: RusshConfig,
    reconnect_config: ReconnectConfig,
}

/// Reconnection configuration.
#[derive(Debug, Clone, Copy)]
pub struct ReconnectConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_retries: 10,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter: 0.3,
        }
    }
}

impl ReconnectConfig {
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay = self.base_delay * 2u32.pow(attempt);
        let delay = delay.min(self.max_delay);
        let jitter_factor = 1.0 + (rand_simple() - 0.5) * 2.0 * self.jitter;
        delay.mul_f64(jitter_factor).min(self.max_delay)
    }
}

fn rand_simple() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    nanos as f64 / u32::MAX as f64
}

impl RusshClient {
    pub fn new(config: RusshConfig) -> Self {
        Self {
            config,
            reconnect_config: ReconnectConfig::default(),
        }
    }

    pub fn with_reconnect_config(mut self, config: ReconnectConfig) -> Self {
        self.reconnect_config = config;
        self
    }

    pub fn config(&self) -> &RusshConfig {
        &self.config
    }

    #[cfg(feature = "russh-backend")]
    pub async fn test_connection(&self) -> RusshResult<RusshConnectionTestResult> {
        let start = Instant::now();
        match self.connect_internal().await {
            Ok(_) => Ok(RusshConnectionTestResult::success(
                self.config.auth.display_name(),
                start.elapsed().as_millis() as u64,
            )),
            Err(e) => Ok(RusshConnectionTestResult::failed(
                e.to_string(),
                self.config.auth.display_name(),
                start.elapsed().as_millis() as u64,
            )),
        }
    }

    #[cfg(not(feature = "russh-backend"))]
    pub async fn test_connection(&self) -> RusshResult<RusshConnectionTestResult> {
        Err(RusshError::ConfigError {
            reason: "russh-backend feature not enabled".to_string(),
        })
    }

    #[cfg(feature = "russh-backend")]
    pub async fn connect(&self) -> RusshResult<super::session::RusshSession> {
        self.connect_internal().await
    }

    #[cfg(not(feature = "russh-backend"))]
    pub async fn connect(&self) -> RusshResult<super::session::RusshSession> {
        Err(RusshError::ConfigError {
            reason: "russh-backend feature not enabled".to_string(),
        })
    }

    pub async fn connect_with_retry(&self) -> RusshResult<super::session::RusshSession> {
        let mut last_error = None;

        for attempt in 0..=self.reconnect_config.max_retries {
            match self.connect_internal().await {
                Ok(session) => return Ok(session),
                Err(e) => {
                    if !e.is_retryable() {
                        return Err(e);
                    }
                    last_error = Some(e);
                    if attempt < self.reconnect_config.max_retries {
                        let delay = self.reconnect_config.calculate_delay(attempt);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or(RusshError::ReconnectExhausted {
            attempts: self.reconnect_config.max_retries,
        }))
    }

    #[cfg(feature = "russh-backend")]
    async fn connect_internal(&self) -> RusshResult<super::session::RusshSession> {
        let handler = ClientHandler {
            known_hosts_policy: self.config.known_hosts_policy,
        };

        let config = Arc::new(russh::client::Config::default());
        let addr = format!("{}:{}", self.config.host, self.config.port);

        let mut handle = russh::client::connect(config, &addr, handler)
            .await
            .map_err(|e| RusshError::ConnectionFailed {
                host: self.config.host.clone(),
                port: self.config.port,
                message: e.to_string(),
            })?;

        self.authenticate(&mut handle).await?;

        Ok(super::session::RusshSession::new(self.config.clone(), handle))
    }

    #[cfg(not(feature = "russh-backend"))]
    async fn connect_internal(&self) -> RusshResult<super::session::RusshSession> {
        Err(RusshError::ConfigError {
            reason: "russh-backend feature not enabled".to_string(),
        })
    }

    #[cfg(feature = "russh-backend")]
    async fn authenticate(
        &self,
        handle: &mut russh::client::Handle<ClientHandler>,
    ) -> RusshResult<()> {
        match &self.config.auth {
            RusshAuthMethod::Password(password) => {
                let result = handle
                    .authenticate_password(&self.config.username, password)
                    .await?;

                if !result.success() {
                    return Err(RusshError::AuthFailed {
                        host: self.config.host.clone(),
                        username: self.config.username.clone(),
                        reason: "Authentication rejected".to_string(),
                    });
                }
            }
            RusshAuthMethod::PublicKey { .. } => {
                // TODO: Implement proper public key authentication
                return Err(RusshError::AuthFailed {
                    host: self.config.host.clone(),
                    username: self.config.username.clone(),
                    reason: "Public key authentication not yet implemented with russh backend".to_string(),
                });
            }
            RusshAuthMethod::Agent => {
                // TODO: Implement proper agent authentication
                return Err(RusshError::AuthFailed {
                    host: self.config.host.clone(),
                    username: self.config.username.clone(),
                    reason: "Agent authentication not yet implemented with russh backend".to_string(),
                });
            }
            RusshAuthMethod::KeyboardInteractive { .. } => {
                return Err(RusshError::AuthFailed {
                    host: self.config.host.clone(),
                    username: self.config.username.clone(),
                    reason: "Keyboard interactive not yet implemented".to_string(),
                });
            }
            RusshAuthMethod::None => {
                let result = handle.authenticate_none(&self.config.username).await?;

                if !result.success() {
                    return Err(RusshError::AuthFailed {
                        host: self.config.host.clone(),
                        username: self.config.username.clone(),
                        reason: "Authentication rejected".to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconnect_config_delay() {
        let config = ReconnectConfig::default();
        let d0 = config.calculate_delay(0);
        let d1 = config.calculate_delay(1);
        assert!(d1 >= d0);
    }

    #[test]
    fn test_connection_test_result() {
        let success = RusshConnectionTestResult::success("password", 100);
        assert!(success.success);

        let failed = RusshConnectionTestResult::failed("error", "password", 100);
        assert!(!failed.success);
    }
}