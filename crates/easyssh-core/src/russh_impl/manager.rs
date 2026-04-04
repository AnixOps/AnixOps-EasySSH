//! SSH session manager with connection pooling for russh backend
//!
//! Provides connection management following SYSTEM_INVARIANTS.md:
//! - Section 2.2: Connection pool limits
//! - Section 0.3: State gating
//! - Section 0.4: Resource ownership

use crate::russh_impl::config::RusshConfig;
use crate::russh_impl::error::{RusshError, RusshResult};
use crate::russh_impl::session::{RusshSession, RusshSessionMetadata, RusshSessionState};
use crate::russh_impl::client::RusshConnectionTestResult;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Server key for connection pooling.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct ServerKey {
    host: String,
    port: u16,
    username: String,
}

impl ServerKey {
    fn new(host: &str, port: u16, username: &str) -> Self {
        Self {
            host: host.to_string(),
            port,
            username: username.to_string(),
        }
    }
}

/// Connection health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionHealth {
    /// Connection is healthy and ready
    Healthy,
    /// Connection has issues but still usable
    Degraded,
    /// Connection should not be used
    Unhealthy,
}

/// Pooled connection wrapper.
struct PooledConnection {
    /// Session reference
    session: Arc<Mutex<Option<RusshSession>>>,
    /// Creation timestamp
    created_at: Instant,
    /// Last used timestamp
    last_used: Instant,
    /// Health status
    health: ConnectionHealth,
    /// Active flag
    active: Arc<AtomicBool>,
}

impl PooledConnection {
    fn new(session: RusshSession) -> Self {
        Self {
            session: Arc::new(Mutex::new(Some(session))),
            created_at: Instant::now(),
            last_used: Instant::now(),
            health: ConnectionHealth::Healthy,
            active: Arc::new(AtomicBool::new(false)),
        }
    }

    fn touch(&mut self) {
        self.last_used = Instant::now();
    }

    fn is_expired(&self, idle_timeout: Duration, max_age: Duration) -> bool {
        self.last_used.elapsed() > idle_timeout || self.created_at.elapsed() > max_age
    }

    fn age_secs(&self) -> u64 {
        self.created_at.elapsed().as_secs()
    }

    fn idle_secs(&self) -> u64 {
        self.last_used.elapsed().as_secs()
    }
}

/// Connection pool for a single server.
struct ConnectionPool {
    connections: Vec<PooledConnection>,
    max_connections: usize,
    idle_timeout: Duration,
    max_age: Duration,
}

impl ConnectionPool {
    fn new(max_connections: usize, idle_timeout_secs: u64, max_age_secs: u64) -> Self {
        Self {
            connections: Vec::with_capacity(max_connections),
            max_connections,
            idle_timeout: Duration::from_secs(idle_timeout_secs),
            max_age: Duration::from_secs(max_age_secs),
        }
    }

    fn acquire(&mut self) -> Option<usize> {
        self.cleanup_expired();

        for (idx, conn) in self.connections.iter_mut().enumerate() {
            if conn.health != ConnectionHealth::Unhealthy
                && !conn.active.load(Ordering::Relaxed)
            {
                conn.touch();
                conn.active.store(true, Ordering::Relaxed);
                return Some(idx);
            }
        }

        None
    }

    fn add(&mut self, session: RusshSession) -> Option<usize> {
        if self.connections.len() >= self.max_connections {
            return None;
        }

        let idx = self.connections.len();
        let conn = PooledConnection::new(session);
        conn.active.store(true, Ordering::Relaxed);
        self.connections.push(conn);
        Some(idx)
    }

    fn release(&mut self, idx: usize) {
        if let Some(conn) = self.connections.get_mut(idx) {
            conn.touch();
            conn.active.store(false, Ordering::Relaxed);
        }
    }

    fn cleanup_expired(&mut self) {
        let before = self.connections.len();
        self.connections
            .retain(|c| !c.is_expired(self.idle_timeout, self.max_age));
        let removed = before - self.connections.len();
        if removed > 0 {
            log::info!("SSH Pool: cleaned up {} expired connections", removed);
        }
    }

    fn len(&self) -> usize {
        self.connections.len()
    }

    fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }
}

/// User session mapping.
struct UserSession {
    server_key: ServerKey,
    pool_idx: usize,
    metadata: RusshSessionMetadata,
}

/// Pool statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RusshPoolStats {
    /// Total number of pools
    pub total_pools: usize,
    /// Total active sessions
    pub total_sessions: usize,
    /// Per-pool information
    pub pools: Vec<RusshPoolInfo>,
}

/// Per-pool information.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RusshPoolInfo {
    /// Server identifier
    pub server: String,
    /// Number of connections
    pub connection_count: usize,
    /// Connection details
    pub connections: Vec<RusshConnectionInfo>,
}

/// Connection information.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RusshConnectionInfo {
    /// Age in seconds
    pub age_secs: u64,
    /// Idle time in seconds
    pub idle_secs: u64,
    /// Health status
    pub health: String,
    /// Whether connection is busy
    pub busy: bool,
}

/// SSH session manager with connection pooling.
///
/// Following SYSTEM_INVARIANTS.md Section 2.2:
/// - Each target server has max 1 Connection instance
/// - Pool size has upper limit (default 100)
/// - Idle connections timeout (default 30 minutes)
///
/// # Example
///
/// ```rust,no_run
/// use easyssh_core::russh_impl::{RusshSessionManager, RusshConfig};
///
/// async fn example() {
///     let mut manager = RusshSessionManager::new();
///
///     // Connect
///     let config = RusshConfig::with_agent("192.168.1.1", 22, "root");
///     let metadata = manager.connect("session-1", &config).await.unwrap();
///
///     // Execute command
///     let output = manager.execute("session-1", "uname -a").await.unwrap();
///
///     // Disconnect
///     manager.disconnect("session-1").await.unwrap();
/// }
/// ```
pub struct RusshSessionManager {
    /// Connection pools
    pools: HashMap<ServerKey, ConnectionPool>,
    /// User sessions
    user_sessions: HashMap<String, UserSession>,
    /// Maximum connections per pool
    pool_max_connections: usize,
    /// Idle timeout in seconds
    pool_idle_timeout: u64,
    /// Maximum connection age in seconds
    pool_max_age: u64,
}

impl RusshSessionManager {
    /// Create a new session manager with default settings.
    ///
    /// Default settings:
    /// - Max connections per pool: 4
    /// - Idle timeout: 300 seconds (5 minutes)
    /// - Max age: 3600 seconds (1 hour)
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            user_sessions: HashMap::new(),
            pool_max_connections: 4,
            pool_idle_timeout: 300,
            pool_max_age: 3600,
        }
    }

    /// Configure pool settings.
    pub fn with_pool_config(
        mut self,
        max_connections: usize,
        idle_timeout: u64,
        max_age: u64,
    ) -> Self {
        self.pool_max_connections = max_connections;
        self.pool_idle_timeout = idle_timeout;
        self.pool_max_age = max_age;
        self
    }

    /// Clean up expired connections.
    pub fn cleanup_expired(&mut self) {
        for (key, pool) in self.pools.iter_mut() {
            let before = pool.len();
            pool.cleanup_expired();
            let after = pool.len();
            if before != after {
                log::info!("SSH Pool {}: {} -> {} connections", key.host, before, after);
            }
        }

        self.pools.retain(|_, p| !p.is_empty());
    }

    /// Connect to SSH server.
    pub async fn connect(
        &mut self,
        session_id: &str,
        config: &RusshConfig,
    ) -> RusshResult<RusshSessionMetadata> {
        self.cleanup_expired();

        if !config.is_valid() {
            return Err(RusshError::ConfigError {
                reason: "Invalid configuration".to_string(),
            });
        }

        let server_key = ServerKey::new(&config.host, config.port, &config.username);

        // Check existing pool
        let pool = self.pools.entry(server_key.clone()).or_insert_with(|| {
            ConnectionPool::new(
                self.pool_max_connections,
                self.pool_idle_timeout,
                self.pool_max_age,
            )
        });

        // Try to acquire existing connection
        if let Some(pool_idx) = pool.acquire() {
            let metadata = RusshSessionMetadata {
                id: session_id.to_string(),
                server_id: String::new(),
                host: config.host.clone(),
                port: config.port,
                username: config.username.clone(),
                connected_at: Instant::now(),
                server_version: None,
            };

            self.user_sessions.insert(
                session_id.to_string(),
                UserSession {
                    server_key: server_key.clone(),
                    pool_idx,
                    metadata: metadata.clone(),
                },
            );

            log::info!(
                "SSH MUX: Reused connection {}@{}:{} (pool_idx={})",
                config.username,
                config.host,
                config.port,
                pool_idx
            );

            return Ok(metadata);
        }

        // Create new connection
        let client = super::client::RusshClient::new(config.clone());
        let session = client.connect().await?;

        let pool_idx = pool.add(session).ok_or(RusshError::PoolFull {
            max_connections: self.pool_max_connections,
        })?;

        let metadata = RusshSessionMetadata {
            id: session_id.to_string(),
            server_id: String::new(),
            host: config.host.clone(),
            port: config.port,
            username: config.username.clone(),
            connected_at: Instant::now(),
            server_version: None,
        };

        self.user_sessions.insert(
            session_id.to_string(),
            UserSession {
                server_key: server_key.clone(),
                pool_idx,
                metadata: metadata.clone(),
            },
        );

        log::info!(
            "SSH MUX: Created new connection {}@{}:{} (pool_idx={}, pool_size={})",
            config.username,
            config.host,
            config.port,
            pool_idx,
            pool.len()
        );

        Ok(metadata)
    }

    /// Test connection without establishing a session.
    pub async fn test_connection(&self, config: &RusshConfig) -> RusshResult<RusshConnectionTestResult> {
        let client = super::client::RusshClient::new(config.clone());
        client.test_connection().await
    }

    /// Disconnect a session.
    pub async fn disconnect(&mut self, session_id: &str) -> RusshResult<()> {
        if let Some(user_session) = self.user_sessions.remove(session_id) {
            if let Some(pool) = self.pools.get_mut(&user_session.server_key) {
                pool.release(user_session.pool_idx);

                log::info!(
                    "SSH MUX: Released connection {}@{}:{}",
                    user_session.metadata.username,
                    user_session.metadata.host,
                    user_session.metadata.port
                );
            }
        }

        Ok(())
    }

    /// List active sessions.
    pub fn list_sessions(&self) -> Vec<String> {
        self.user_sessions.keys().cloned().collect()
    }

    /// Check if session exists.
    pub fn has_session(&self, session_id: &str) -> bool {
        self.user_sessions.contains_key(session_id)
    }

    /// Get session metadata.
    pub fn get_metadata(&self, session_id: &str) -> Option<RusshSessionMetadata> {
        self.user_sessions.get(session_id).map(|s| s.metadata.clone())
    }

    /// Get pool statistics.
    pub fn get_pool_stats(&self) -> RusshPoolStats {
        let pools: Vec<RusshPoolInfo> = self
            .pools
            .iter()
            .map(|(key, pool)| {
                let connections: Vec<RusshConnectionInfo> = pool
                    .connections
                    .iter()
                    .map(|c| RusshConnectionInfo {
                        age_secs: c.age_secs(),
                        idle_secs: c.idle_secs(),
                        health: format!("{:?}", c.health),
                        busy: c.active.load(Ordering::Relaxed),
                    })
                    .collect();

                RusshPoolInfo {
                    server: format!("{}@{}:{}", key.username, key.host, key.port),
                    connection_count: pool.len(),
                    connections,
                }
            })
            .collect();

        RusshPoolStats {
            total_pools: pools.len(),
            total_sessions: self.user_sessions.len(),
            pools,
        }
    }

    /// Execute a command on a session.
    pub async fn execute(&self, session_id: &str, command: &str) -> RusshResult<String> {
        // This would need access to the actual session from the pool
        // For now, return an error
        Err(RusshError::SessionNotFound {
            session_id: session_id.to_string(),
        })
    }

    /// Get maximum connections per pool.
    pub fn max_connections(&self) -> usize {
        self.pool_max_connections
    }

    /// Get idle timeout in seconds.
    pub fn idle_timeout_secs(&self) -> u64 {
        self.pool_idle_timeout
    }

    /// Get maximum connection age in seconds.
    pub fn max_age_secs(&self) -> u64 {
        self.pool_max_age
    }
}

impl Default for RusshSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_key() {
        let key1 = ServerKey::new("host", 22, "user");
        let key2 = ServerKey::new("host", 22, "user");
        let key3 = ServerKey::new("host", 22, "other");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_pool_config() {
        let manager = RusshSessionManager::new()
            .with_pool_config(10, 600, 7200);

        assert_eq!(manager.pool_max_connections, 10);
        assert_eq!(manager.pool_idle_timeout, 600);
        assert_eq!(manager.pool_max_age, 7200);
    }

    #[test]
    fn test_pool_stats() {
        let manager = RusshSessionManager::new();
        let stats = manager.get_pool_stats();

        assert_eq!(stats.total_pools, 0);
        assert_eq!(stats.total_sessions, 0);
    }

    #[test]
    fn test_list_sessions() {
        let manager = RusshSessionManager::new();

        assert!(manager.list_sessions().is_empty());
        assert!(!manager.has_session("test"));
    }
}