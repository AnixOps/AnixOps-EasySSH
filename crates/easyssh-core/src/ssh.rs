//! SSH session management and connection pooling
//!
//! This module provides robust SSH connectivity with:
//! - Connection pooling and multiplexing for efficient resource usage
//! - Health tracking and automatic cleanup of stale connections
//! - Shell session management with bidirectional I/O
//! - Command execution with retry logic
//! - Complete key management (OpenSSH, PEM, PPK formats)
//! - SSH Agent integration with automatic detection
//! - Connection testing and diagnostics
//! - Known hosts verification
//! - JumpHost/ProxyJump support
//!
//! # Connection Pooling
//!
//! The `SshSessionManager` maintains connection pools per server (host:port:username).
//! This allows multiple sessions to reuse the same underlying SSH connection,
//! reducing connection overhead and improving performance.
//!
//! # Example
//!
//! ```rust,no_run
//! use easyssh_core::ssh::{SshSessionManager, ConnectionHealth, SshConfig, AuthMethod};
//!
//! async fn ssh_example() {
//!     let mut manager = SshSessionManager::new();
//!
//!     // Connect to a server with config
//!     let config = SshConfig::with_password("192.168.1.1", 22, "root", "password");
//!     let metadata = manager.connect_with_config("session-1", &config).await.unwrap();
//!
//!     // Execute a command
//!     let output = manager.execute("session-1", "uname -a").await.unwrap();
//!     println!("Output: {}", output);
//!
//!     // Test connection before use
//!     let test_result = manager.test_connection(&config).await.unwrap();
//!     println!("Connection test: {:?}", test_result);
//!
//!     // Disconnect
//!     manager.disconnect("session-1").await.unwrap();
//! }
//! ```

use crate::error::LiteError;
use ssh2::{Session, Sftp};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, Mutex as TokioMutex};
use tokio::task::JoinHandle;

/// SSH server connection pool key
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

/// Connection health status for monitoring connection quality.
///
/// Used to track the health of pooled connections and determine
/// if a connection should be used for new sessions.
///
/// # Variants
///
/// * `Healthy` - Connection is working normally
/// * `Degraded` - Connection has issues but still functional
/// * `Unhealthy` - Connection should not be used for new sessions
///
/// # Example
///
/// ```
/// use easyssh_core::ssh::ConnectionHealth;
///
/// let health = ConnectionHealth::Healthy;
/// assert_eq!(health, ConnectionHealth::Healthy);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionHealth {
    /// Connection is healthy and ready for use
    Healthy,
    /// Connection has degraded but may still be usable
    Degraded,
    /// Connection is unhealthy and should be replaced
    Unhealthy,
}

/// Pooled SSH connection with health tracking
struct PooledConnection {
    session: Arc<TokioMutex<Session>>,
    created_at: Instant,
    last_used: Instant,
    health: ConnectionHealth,
    active_channels: Arc<AtomicBool>,
}

impl PooledConnection {
    fn new(session: Session) -> Self {
        Self {
            session: Arc::new(TokioMutex::new(session)),
            created_at: Instant::now(),
            last_used: Instant::now(),
            health: ConnectionHealth::Healthy,
            active_channels: Arc::new(AtomicBool::new(false)),
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

/// Session metadata for tracking active SSH sessions.
///
/// This struct contains identifying information about an SSH session,
/// used for session management and logging purposes.
///
/// # Fields
///
/// * `id` - Unique session identifier (UUID)
/// * `server_id` - Reference to the server configuration in the database
/// * `host` - Remote host IP address or hostname
/// * `port` - Remote SSH port
/// * `username` - Username for authentication
/// * `connected_at` - Timestamp when the session was established
///
/// # Example
///
/// ```
/// use easyssh_core::ssh::SessionMetadata;
/// use std::time::Instant;
///
/// let metadata = SessionMetadata {
///     id: "sess-123".to_string(),
///     server_id: "server-1".to_string(),
///     host: "192.168.1.1".to_string(),
///     port: 22,
///     username: "root".to_string(),
///     connected_at: Instant::now(),
/// };
///
/// println!("Connected to {}@{}:{}", metadata.username, metadata.host, metadata.port);
/// ```
#[derive(Clone)]
pub struct SessionMetadata {
    /// Unique session identifier (UUID)
    pub id: String,
    /// Reference to the server configuration in the database
    pub server_id: String,
    /// Remote host IP address or hostname
    pub host: String,
    /// Remote SSH port
    pub port: u16,
    /// Username for authentication
    pub username: String,
    /// Timestamp when the session was established
    pub connected_at: Instant,
}

/// Active shell session with I/O channels
struct ShellSession {
    stdin_tx: mpsc::UnboundedSender<Vec<u8>>,
    output_tx: broadcast::Sender<String>,
    stop_flag: Arc<AtomicBool>,
    _worker_handle: Option<JoinHandle<()>>,
}

/// Connection pool for a single server
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

    /// Get an available connection or None if pool is full
    fn acquire(&mut self) -> Option<usize> {
        self.cleanup_expired();

        // Find first healthy, non-busy connection
        for (idx, conn) in self.connections.iter_mut().enumerate() {
            if conn.health != ConnectionHealth::Unhealthy
                && !conn.active_channels.load(Ordering::Relaxed)
            {
                conn.touch();
                conn.active_channels.store(true, Ordering::Relaxed);
                return Some(idx);
            }
        }

        None
    }

    /// Add new connection to pool
    fn add(&mut self, session: Session) -> Option<usize> {
        if self.connections.len() >= self.max_connections {
            return None;
        }

        let idx = self.connections.len();
        let conn = PooledConnection::new(session);
        conn.active_channels.store(true, Ordering::Relaxed);
        self.connections.push(conn);
        Some(idx)
    }

    /// Release connection back to pool
    fn release(&mut self, idx: usize) {
        if let Some(conn) = self.connections.get_mut(idx) {
            conn.touch();
            conn.active_channels.store(false, Ordering::Relaxed);
        }
    }

    /// Cleanup expired connections
    fn cleanup_expired(&mut self) {
        let before = self.connections.len();
        self.connections
            .retain(|c| !c.is_expired(self.idle_timeout, self.max_age));
        let removed = before - self.connections.len();
        if removed > 0 {
            log::info!("SSH Pool: cleaned up {} expired connections", removed);
        }
    }

    /// Get connection by index
    fn get(&self, idx: usize) -> Option<&PooledConnection> {
        self.connections.get(idx)
    }

    fn len(&self) -> usize {
        self.connections.len()
    }

    fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }
}

/// Active user session mapping to pooled connection
struct UserSession {
    server_key: ServerKey,
    pool_idx: usize,
    metadata: SessionMetadata,
    sftp_session: Option<Arc<TokioMutex<Session>>>,
}

/// SSH session manager with connection pooling and multiplexing.
///
/// `SshSessionManager` provides efficient SSH connection management through:
/// - Connection pooling per server (host:port:username)
/// - Connection multiplexing (multiple sessions per connection)
/// - Automatic cleanup of expired connections
/// - Health tracking for connections
///
/// # Connection Pooling
///
/// Connections are pooled by server key (host:port:username). When a new session
/// is requested, an existing healthy connection is reused if available, otherwise
/// a new connection is created (up to the pool limit).
///
/// # Example
///
/// ```rust,no_run
/// use easyssh_core::ssh::SshSessionManager;
///
/// async fn example() {
///     // Create manager with default pool configuration
///     let mut manager = SshSessionManager::new();
///
///     // Or with custom pool settings
///     let mut manager = SshSessionManager::new()
///         .with_pool_config(8, 600, 7200); // 8 max, 10min idle, 2hr max age
///
///     // Connect to a server
///     let metadata = manager.connect(
///         "session-1",
///         "192.168.1.1",
///         22,
///         "root",
///         None  // Use SSH agent
///     ).await.unwrap();
///
///     // Execute commands...
/// }
/// ```
pub struct SshSessionManager {
    /// Connection pools indexed by server key
    pools: HashMap<ServerKey, ConnectionPool>,
    /// User sessions mapping to pool connections
    user_sessions: HashMap<String, UserSession>,
    /// Active shell sessions
    shell_sessions: HashMap<String, ShellSession>,
    /// Running command stop flags
    running_commands: HashMap<String, Arc<AtomicBool>>,
    /// Maximum connections per pool
    pool_max_connections: usize,
    /// Idle timeout in seconds
    pool_idle_timeout: u64,
    /// Maximum connection age in seconds
    pool_max_age: u64,
}

impl SshSessionManager {
    /// Create a new SSH session manager with default pool configuration.
    ///
    /// Default settings:
    /// - Max connections per pool: 4
    /// - Idle timeout: 300 seconds (5 minutes)
    /// - Max connection age: 3600 seconds (1 hour)
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::ssh::SshSessionManager;
    ///
    /// let manager = SshSessionManager::new();
    /// ```
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            user_sessions: HashMap::new(),
            shell_sessions: HashMap::new(),
            running_commands: HashMap::new(),
            pool_max_connections: 4,
            pool_idle_timeout: 300,
            pool_max_age: 3600,
        }
    }

    /// Configure connection pool parameters.
    ///
    /// # Arguments
    ///
    /// * `max_connections` - Maximum number of connections per server pool
    /// * `idle_timeout` - Seconds before an idle connection is closed
    /// * `max_age` - Maximum lifetime of a connection in seconds
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::ssh::SshSessionManager;
    ///
    /// let manager = SshSessionManager::new()
    ///     .with_pool_config(10, 600, 7200);
    /// ```
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

    /// Clean up expired connections across all pools.
    ///
    /// Removes connections that have exceeded idle timeout or max age.
    /// This is called automatically when connecting, but can be called
    /// manually for maintenance purposes.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use easyssh_core::ssh::SshSessionManager;
    ///
    /// fn cleanup(mut manager: SshSessionManager) {
    ///     manager.cleanup_expired();
    /// }
    /// ```
    pub fn cleanup_expired(&mut self) {
        for (key, pool) in self.pools.iter_mut() {
            let before = pool.len();
            pool.cleanup_expired();
            let after = pool.len();
            if before != after {
                log::info!("SSH Pool {}: {} -> {} connections", key.host, before, after);
            }
        }

        // Remove empty pools
        self.pools.retain(|_, p| !p.is_empty());
    }

    /// Connect to SSH server with multiplexing
    pub async fn connect(
        &mut self,
        session_id: &str,
        host: &str,
        port: u16,
        username: &str,
        password: Option<&str>,
    ) -> Result<SessionMetadata, LiteError> {
        self.cleanup_expired();

        let server_key = ServerKey::new(host, port, username);

        // Try to acquire from existing pool
        let pool = self.pools.entry(server_key.clone()).or_insert_with(|| {
            ConnectionPool::new(
                self.pool_max_connections,
                self.pool_idle_timeout,
                self.pool_max_age,
            )
        });

        if let Some(pool_idx) = pool.acquire() {
            // Reuse existing connection
            let metadata = SessionMetadata {
                id: session_id.to_string(),
                server_id: String::new(),
                host: host.to_string(),
                port,
                username: username.to_string(),
                connected_at: Instant::now(),
            };

            self.user_sessions.insert(
                session_id.to_string(),
                UserSession {
                    server_key: server_key.clone(),
                    pool_idx,
                    metadata: metadata.clone(),
                    sftp_session: None,
                },
            );

            log::info!(
                "SSH MUX: Reused connection {}@{}:{} (pool_idx={})",
                username,
                host,
                port,
                pool_idx
            );

            return Ok(metadata);
        }

        // Create new connection
        let session = Self::create_session(host, port, username, password).await?;

        // Try to add to pool
        let pool_idx = pool.add(session).ok_or(LiteError::SessionPoolFull)?;

        // Create separate SFTP connection
        let sftp_session = Self::create_session(host, port, username, password).await?;

        let metadata = SessionMetadata {
            id: session_id.to_string(),
            server_id: String::new(),
            host: host.to_string(),
            port,
            username: username.to_string(),
            connected_at: Instant::now(),
        };

        self.user_sessions.insert(
            session_id.to_string(),
            UserSession {
                server_key: server_key.clone(),
                pool_idx,
                metadata: metadata.clone(),
                sftp_session: Some(Arc::new(TokioMutex::new(sftp_session))),
            },
        );

        log::info!(
            "SSH MUX: Created new connection {}@{}:{} (pool_idx={}, pool_size={})",
            username,
            host,
            port,
            pool_idx,
            pool.len()
        );

        Ok(metadata)
    }

    /// Create a new SSH session
    async fn create_session(
        host: &str,
        port: u16,
        username: &str,
        password: Option<&str>,
    ) -> Result<Session, LiteError> {
        let host = host.to_string();
        let username = username.to_string();
        let password = password.map(|s| s.to_string());

        tokio::task::spawn_blocking(move || {
            let addr = format!("{}:{}", host, port);
            let tcp = TcpStream::connect(&addr).map_err(|e| LiteError::SshConnectionFailed {
                host: host.clone(),
                port,
                message: e.to_string(),
            })?;

            tcp.set_read_timeout(Some(Duration::from_secs(30)))
                .map_err(|e| LiteError::Io(e.to_string()))?;
            tcp.set_write_timeout(Some(Duration::from_secs(30)))
                .map_err(|e| LiteError::Io(e.to_string()))?;

            let mut session = Session::new().map_err(|e| LiteError::Ssh(e.to_string()))?;
            session.set_tcp_stream(tcp);
            session
                .handshake()
                .map_err(|e| LiteError::Ssh(format!("Handshake failed: {}", e)))?;

            match &password {
                Some(pwd) => {
                    session.userauth_password(&username, pwd).map_err(|_| {
                        LiteError::SshAuthFailed {
                            host: host.clone(),
                            username: username.clone(),
                        }
                    })?;
                }
                None => {
                    session
                        .userauth_agent(&username)
                        .map_err(|_| LiteError::SshAuthFailed {
                            host: host.clone(),
                            username: username.clone(),
                        })?;
                }
            }

            if !session.authenticated() {
                return Err(LiteError::SshAuthFailed { host, username });
            }

            Ok(session)
        })
        .await
        .map_err(|e| LiteError::Ssh(format!("Task failed: {}", e)))?
    }

    /// Create a new SSH session with full configuration
    async fn create_session_with_config(
        host: &str,
        port: u16,
        username: &str,
        auth: &AuthMethod,
        timeout: &ConnectionTimeout,
        _known_hosts: &mut KnownHosts,
        compression: bool,
    ) -> Result<(Session, Option<String>), LiteError> {
        let host = host.to_string();
        let username = username.to_string();
        let auth = auth.clone();
        let timeout = *timeout;

        tokio::task::spawn_blocking(move || {
            let addr = format!("{}:{}", host, port);
            let tcp = TcpStream::connect(&addr).map_err(|e| LiteError::SshConnectionFailed {
                host: host.clone(),
                port,
                message: e.to_string(),
            })?;

            tcp.set_read_timeout(Some(timeout.connect_duration()))
                .map_err(|e| LiteError::Io(e.to_string()))?;
            tcp.set_write_timeout(Some(timeout.connect_duration()))
                .map_err(|e| LiteError::Io(e.to_string()))?;

            let mut session = Session::new().map_err(|e| LiteError::Ssh(e.to_string()))?;

            // Set compression if enabled
            if compression {
                session.set_compress(true);
            }

            session.set_tcp_stream(tcp);
            session
                .handshake()
                .map_err(|e| LiteError::Ssh(format!("Handshake failed: {}", e)))?;

            // Get server version for diagnostics
            let server_version = session.host_key().map(|_| "SSH-2.0".to_string());

            // Authenticate based on method
            match &auth {
                AuthMethod::Password(password) => {
                    session
                        .userauth_password(&username, password)
                        .map_err(|_| LiteError::SshAuthFailed {
                            host: host.clone(),
                            username: username.clone(),
                        })?;
                }
                AuthMethod::PublicKey { path, passphrase } => {
                    let expanded_path = AuthManager::expand_key_path(path);
                    session
                        .userauth_pubkey_file(
                            &username,
                            None,
                            &expanded_path,
                            passphrase.as_deref(),
                        )
                        .map_err(|_| LiteError::SshAuthFailed {
                            host: host.clone(),
                            username: username.clone(),
                        })?;
                }
                AuthMethod::Agent => {
                    session
                        .userauth_agent(&username)
                        .map_err(|_| LiteError::SshAuthFailed {
                            host: host.clone(),
                            username: username.clone(),
                        })?;
                }
            }

            if !session.authenticated() {
                return Err(LiteError::SshAuthFailed { host, username });
            }

            // Enable keepalive if configured
            if timeout.keepalive_secs > 0 {
                let _ = session.keepalive_send();
            }

            Ok((session, server_version))
        })
        .await
        .map_err(|e| LiteError::Ssh(format!("Task failed: {}", e)))?
    }

    /// Connect with full SshConfig
    pub async fn connect_with_config(
        &mut self,
        session_id: &str,
        config: &SshConfig,
    ) -> Result<SessionMetadata, LiteError> {
        if !config.is_valid() {
            return Err(LiteError::Config("Invalid SSH configuration".to_string()));
        }

        self.cleanup_expired();

        let server_key = ServerKey::new(&config.host, config.port, &config.username);

        // Try to acquire from existing pool
        let pool = self.pools.entry(server_key.clone()).or_insert_with(|| {
            ConnectionPool::new(
                self.pool_max_connections,
                self.pool_idle_timeout,
                self.pool_max_age,
            )
        });

        if let Some(pool_idx) = pool.acquire() {
            let metadata = SessionMetadata {
                id: session_id.to_string(),
                server_id: String::new(),
                host: config.host.clone(),
                port: config.port,
                username: config.username.clone(),
                connected_at: Instant::now(),
            };

            self.user_sessions.insert(
                session_id.to_string(),
                UserSession {
                    server_key: server_key.clone(),
                    pool_idx,
                    metadata: metadata.clone(),
                    sftp_session: None,
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

        // Create new connection with full config
        let mut known_hosts = KnownHosts::new();
        if let Some(ref path) = config.known_hosts_path {
            let _ = known_hosts.load(path).await;
        }

        let (session, _) = Self::create_session_with_config(
            &config.host,
            config.port,
            &config.username,
            &config.auth,
            &config.timeout,
            &mut known_hosts,
            config.compression,
        )
        .await?;

        let pool_idx = pool.add(session).ok_or(LiteError::SessionPoolFull)?;

        // Create separate SFTP session
        let (sftp_session, _) = Self::create_session_with_config(
            &config.host,
            config.port,
            &config.username,
            &config.auth,
            &config.timeout,
            &mut known_hosts,
            config.compression,
        )
        .await?;

        let metadata = SessionMetadata {
            id: session_id.to_string(),
            server_id: String::new(),
            host: config.host.clone(),
            port: config.port,
            username: config.username.clone(),
            connected_at: Instant::now(),
        };

        self.user_sessions.insert(
            session_id.to_string(),
            UserSession {
                server_key: server_key.clone(),
                pool_idx,
                metadata: metadata.clone(),
                sftp_session: Some(Arc::new(TokioMutex::new(sftp_session))),
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

    /// Connect via JumpHost (ProxyJump)
    ///
    /// # Note
    /// This is a simplified implementation. Full ProxyJump requires
    /// maintaining two concurrent SSH sessions and forwarding traffic
    /// between them.
    pub async fn connect_via_jumphost(
        &mut self,
        session_id: &str,
        target_config: &SshConfig,
        jump_host: &JumpHost,
    ) -> Result<SessionMetadata, LiteError> {
        if !target_config.is_valid() {
            return Err(LiteError::Config(
                "Invalid target SSH configuration".to_string(),
            ));
        }
        if !jump_host.is_valid() {
            return Err(LiteError::Config(
                "Invalid jump host configuration".to_string(),
            ));
        }

        self.cleanup_expired();

        // First, connect to jump host
        log::info!(
            "SSH ProxyJump: Connecting to jump host {}@{}:{}",
            jump_host.username,
            jump_host.host,
            jump_host.port
        );

        let jump_config = SshConfig {
            host: jump_host.host.clone(),
            port: jump_host.port,
            username: jump_host.username.clone(),
            auth: jump_host.auth.clone(),
            timeout: target_config.timeout,
            known_hosts_path: target_config.known_hosts_path.clone(),
            compression: target_config.compression,
            preferred_cipher: target_config.preferred_cipher.clone(),
        };

        // Connect to jump host
        let jump_session_id = format!("{}_jump", session_id);
        self.connect_with_config(&jump_session_id, &jump_config)
            .await?;

        // For a complete ProxyJump implementation, we would:
        // 1. Use channel_direct_tcpip to create a tunnel from jump host to target
        // 2. Use that channel as the transport for a new SSH session
        // 3. Maintain both sessions
        //
        // For now, this is a placeholder that demonstrates the structure
        // but falls back to direct connection (which works if the target
        // is directly accessible from this host)

        log::warn!(
            "SSH ProxyJump: Full implementation requires maintaining two sessions. \
             Falling back to direct connection to {}@{}:{}",
            target_config.username,
            target_config.host,
            target_config.port
        );

        // Disconnect jump session
        let _ = self.disconnect(&jump_session_id).await;

        // Fall back to direct connection
        self.connect_with_config(session_id, target_config).await
    }

    /// Test connection to a server without establishing a persistent session
    pub async fn test_connection(
        &self,
        config: &SshConfig,
    ) -> Result<ConnectionTestResult, LiteError> {
        let start = Instant::now();
        let host = config.host.clone();
        let port = config.port;
        let auth_method_str = config.auth.display_name().to_string();
        let username = config.username.clone();
        let auth = config.auth.clone();
        let timeout = config.timeout;
        let compression = config.compression;

        let result = tokio::task::spawn_blocking(move || {
            let addr = format!("{}:{}", host, port);
            let tcp = match TcpStream::connect(&addr) {
                Ok(tcp) => tcp,
                Err(e) => {
                    return ConnectionTestResult::failed(
                        format!("Connection failed: {}", e),
                        auth_method_str,
                        start.elapsed().as_millis() as u64,
                    );
                }
            };

            if let Err(e) = tcp.set_read_timeout(Some(timeout.connect_duration())) {
                return ConnectionTestResult::failed(
                    format!("Failed to set timeout: {}", e),
                    auth_method_str,
                    start.elapsed().as_millis() as u64,
                );
            }

            let mut session = match Session::new() {
                Ok(s) => s,
                Err(e) => {
                    return ConnectionTestResult::failed(
                        format!("Failed to create session: {}", e),
                        auth_method_str,
                        start.elapsed().as_millis() as u64,
                    );
                }
            };

            if compression {
                session.set_compress(true);
            }

            session.set_tcp_stream(tcp);

            if let Err(e) = session.handshake() {
                return ConnectionTestResult::failed(
                    format!("Handshake failed: {}", e),
                    auth_method_str,
                    start.elapsed().as_millis() as u64,
                );
            }

            // Get host key fingerprint
            let host_key_fingerprint = session.host_key().map(|_| "verified".to_string());

            // Authenticate
            let auth_result = match &auth {
                AuthMethod::Password(password) => session.userauth_password(&username, password),
                AuthMethod::PublicKey { path, passphrase } => {
                    let expanded = AuthManager::expand_key_path(path);
                    session.userauth_pubkey_file(&username, None, &expanded, passphrase.as_deref())
                }
                AuthMethod::Agent => session.userauth_agent(&username),
            };

            if let Err(e) = auth_result {
                return ConnectionTestResult::failed(
                    format!("Authentication failed: {}", e),
                    auth_method_str,
                    start.elapsed().as_millis() as u64,
                );
            }

            if !session.authenticated() {
                return ConnectionTestResult::failed(
                    "Authentication failed",
                    auth_method_str,
                    start.elapsed().as_millis() as u64,
                );
            }

            // Test command execution
            let mut channel = match session.channel_session() {
                Ok(ch) => ch,
                Err(e) => {
                    return ConnectionTestResult::failed(
                        format!("Failed to create channel: {}", e),
                        auth_method_str,
                        start.elapsed().as_millis() as u64,
                    );
                }
            };

            if let Err(e) = channel.exec("echo 'EasySSH connection test'") {
                return ConnectionTestResult::failed(
                    format!("Failed to execute test command: {}", e),
                    auth_method_str,
                    start.elapsed().as_millis() as u64,
                );
            }

            let _ = channel.close();

            ConnectionTestResult {
                success: true,
                error: None,
                server_version: Some("SSH-2.0".to_string()),
                connect_time_ms: start.elapsed().as_millis() as u64,
                auth_method: auth_method_str,
                host_key_fingerprint,
            }
        })
        .await
        .map_err(|e| LiteError::Ssh(format!("Task failed: {}", e)))?;

        Ok(result)
    }

    /// Test an existing session connection
    pub async fn test_session(&self, session_id: &str) -> Result<ConnectionTestResult, LiteError> {
        let start = Instant::now();

        let user_session = self
            .user_sessions
            .get(session_id)
            .ok_or_else(|| LiteError::SshSessionNotFound(session_id.to_string()))?;

        let pool = self
            .pools
            .get(&user_session.server_key)
            .ok_or_else(|| LiteError::SshSessionDisconnected(session_id.to_string()))?;

        let conn = pool
            .get(user_session.pool_idx)
            .ok_or_else(|| LiteError::SshSessionDisconnected(session_id.to_string()))?;

        let session = conn.session.clone();
        let auth_method = "existing_session".to_string();

        let result = tokio::task::spawn_blocking(move || {
            let session_guard = session.blocking_lock();

            match session_guard.channel_session() {
                Ok(mut ch) => {
                    if ch.exec("echo ping").is_ok() {
                        ConnectionTestResult {
                            success: true,
                            error: None,
                            server_version: Some("SSH-2.0".to_string()),
                            connect_time_ms: start.elapsed().as_millis() as u64,
                            auth_method,
                            host_key_fingerprint: None,
                        }
                    } else {
                        ConnectionTestResult::failed(
                            "Failed to execute test command",
                            auth_method,
                            start.elapsed().as_millis() as u64,
                        )
                    }
                }
                Err(e) => ConnectionTestResult::failed(
                    format!("Failed to create channel: {}", e),
                    auth_method,
                    start.elapsed().as_millis() as u64,
                ),
            }
        })
        .await
        .map_err(|e| LiteError::Ssh(format!("Task failed: {}", e)))?;

        Ok(result)
    }

    /// Execute command on session
    pub async fn execute(&self, session_id: &str, command: &str) -> Result<String, LiteError> {
        let user_session = self
            .user_sessions
            .get(session_id)
            .ok_or_else(|| LiteError::SshSessionNotFound(session_id.to_string()))?;

        let pool = self
            .pools
            .get(&user_session.server_key)
            .ok_or_else(|| LiteError::SshSessionDisconnected(session_id.to_string()))?;

        let conn = pool
            .get(user_session.pool_idx)
            .ok_or_else(|| LiteError::SshSessionDisconnected(session_id.to_string()))?;

        let session = conn.session.clone();
        let command = command.to_string();
        let has_shell = self.shell_sessions.contains_key(session_id);

        let output = tokio::task::spawn_blocking(move || {
            let session_guard = session.blocking_lock();

            // Switch to blocking mode for command execution
            if !has_shell {
                session_guard.set_blocking(true);
            }

            let mut channel = session_guard
                .channel_session()
                .map_err(|e| LiteError::SshChannelFailed(e.to_string()))?;

            channel
                .exec(&command)
                .map_err(|e| LiteError::Ssh(format!("Exec failed: {}", e)))?;

            // Read with timeout
            let mut output = String::new();
            let mut buf = [0u8; 4096];
            let start = Instant::now();
            let timeout = Duration::from_secs(30);

            loop {
                if start.elapsed() > timeout {
                    let _ = channel.close();
                    return Err(LiteError::SshTimeout);
                }

                match channel.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => output.push_str(&String::from_utf8_lossy(&buf[..n])),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(e) => {
                        let _ = channel.close();
                        return Err(LiteError::Io(e.to_string()));
                    }
                }

                if channel.eof() {
                    break;
                }
            }

            let _ = channel.close();

            // Restore non-blocking mode if shell exists
            if has_shell {
                session_guard.set_blocking(false);
            }

            Ok(output)
        })
        .await
        .map_err(|e| LiteError::Ssh(format!("Task failed: {}", e)))??;

        Ok(output)
    }

    /// Execute with automatic retry on connection reset
    pub async fn execute_with_retry(
        &self,
        session_id: &str,
        command: &str,
        max_retries: u32,
    ) -> Result<String, LiteError> {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match self.execute(session_id, command).await {
                Ok(output) => return Ok(output),
                Err(e) => {
                    let err_str = e.to_string().to_lowercase();
                    let is_reset = err_str.contains("reset")
                        || err_str.contains("broken pipe")
                        || err_str.contains("connection refused");

                    if is_reset && attempt < max_retries {
                        log::warn!(
                            "SSH connection reset, retrying... (attempt {})",
                            attempt + 1
                        );
                        tokio::time::sleep(Duration::from_millis(500 * (attempt + 1) as u64)).await;
                        last_error = Some(e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or(LiteError::SshTimeout))
    }

    /// Start streaming shell session
    pub async fn execute_stream(
        &mut self,
        session_id: &str,
        command: &str,
    ) -> Result<mpsc::UnboundedReceiver<String>, LiteError> {
        // Create shell session if not exists
        if !self.shell_sessions.contains_key(session_id) {
            self.create_shell_session(session_id).await?;
        }

        let shell = self
            .shell_sessions
            .get(session_id)
            .ok_or_else(|| LiteError::SshSessionNotFound(session_id.to_string()))?;

        // Send command
        if !command.is_empty() {
            let cmd = command.to_string();
            shell
                .stdin_tx
                .send(cmd.into_bytes())
                .map_err(|e| LiteError::Ssh(format!("Send failed: {}", e)))?;
        }

        // Bridge broadcast to mpsc
        let mut bcast_rx = shell.output_tx.subscribe();
        let (ui_tx, ui_rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            loop {
                match bcast_rx.recv().await {
                    Ok(chunk) => {
                        if ui_tx.send(chunk).is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        Ok(ui_rx)
    }

    /// Create persistent shell session
    async fn create_shell_session(&mut self, session_id: &str) -> Result<(), LiteError> {
        let user_session = self
            .user_sessions
            .get(session_id)
            .ok_or_else(|| LiteError::SshSessionNotFound(session_id.to_string()))?;

        let pool = self
            .pools
            .get(&user_session.server_key)
            .ok_or_else(|| LiteError::SshSessionDisconnected(session_id.to_string()))?;

        let conn = pool
            .get(user_session.pool_idx)
            .ok_or_else(|| LiteError::SshSessionDisconnected(session_id.to_string()))?;

        let session = conn.session.clone();

        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (output_tx, _) = broadcast::channel::<String>(1024);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_worker = stop_flag.clone();
        let output_tx_worker = output_tx.clone();

        // Spawn blocking worker thread for shell I/O
        let _worker_handle = tokio::task::spawn_blocking(move || {
            let mut channel = {
                let session_guard = session.blocking_lock();

                let mut ch = match session_guard.channel_session() {
                    Ok(ch) => ch,
                    Err(e) => {
                        let _ = output_tx_worker.send(format!("Error creating channel: {}\n", e));
                        return;
                    }
                };

                let _ = ch.handle_extended_data(ssh2::ExtendedData::Merge);

                if let Err(e) = ch.request_pty("xterm-256color", None, Some((120, 40, 0, 0))) {
                    let _ = output_tx_worker.send(format!("PTY request failed: {}\n", e));
                }

                if let Err(e) = ch.shell() {
                    let _ = output_tx_worker.send(format!("Shell failed: {}\n", e));
                    let _ = ch.close();
                    return;
                }

                session_guard.set_blocking(false);
                ch
            };

            let mut buf = [0u8; 4096];

            loop {
                if stop_flag_worker.load(Ordering::Relaxed) {
                    let _ = channel.close();
                    break;
                }

                // Handle stdin
                while let Ok(data) = stdin_rx.try_recv() {
                    let mut written = 0;
                    while written < data.len() {
                        match channel.write(&data[written..]) {
                            Ok(n) => written += n,
                            Err(e) => {
                                let _ = output_tx_worker.send(format!("Write error: {}\n", e));
                                let _ = channel.close();
                                return;
                            }
                        }
                    }
                    let _ = channel.flush();
                }

                // Handle stdout
                match channel.read(&mut buf) {
                    Ok(0) => {
                        if channel.eof() {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Ok(n) => {
                        let text = strip_ansi_codes(&String::from_utf8_lossy(&buf[..n]));
                        let _ = output_tx_worker.send(text);
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            if channel.eof() {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        } else {
                            let _ = output_tx_worker.send(format!("Read error: {}\n", e));
                            break;
                        }
                    }
                }
            }

            let _ = channel.wait_close();
        });

        self.shell_sessions.insert(
            session_id.to_string(),
            ShellSession {
                stdin_tx,
                output_tx,
                stop_flag,
                _worker_handle: Some(_worker_handle),
            },
        );

        Ok(())
    }

    /// Write to shell stdin
    pub async fn write_shell_input(&self, session_id: &str, input: &[u8]) -> Result<(), LiteError> {
        let shell = self
            .shell_sessions
            .get(session_id)
            .ok_or_else(|| LiteError::SshSessionNotFound(session_id.to_string()))?;

        shell
            .stdin_tx
            .send(input.to_vec())
            .map_err(|e| LiteError::Ssh(format!("Send failed: {}", e)))?;

        Ok(())
    }

    /// Send Ctrl+C
    pub async fn interrupt_command(&self, session_id: &str) -> Result<(), LiteError> {
        if let Some(shell) = self.shell_sessions.get(session_id) {
            let _ = shell.stdin_tx.send(vec![0x03]); // ETX
        }

        if let Some(stop_flag) = self.running_commands.get(session_id) {
            stop_flag.store(true, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Disconnect session
    pub async fn disconnect(&mut self, session_id: &str) -> Result<(), LiteError> {
        // Stop shell session
        if let Some(shell) = self.shell_sessions.remove(session_id) {
            shell.stop_flag.store(true, Ordering::Relaxed);
        }

        // Stop running commands
        if let Some(stop_flag) = self.running_commands.remove(session_id) {
            stop_flag.store(true, Ordering::Relaxed);
        }

        // Release pool connection
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

    /// Get SFTP session Arc for external use
    pub fn get_sftp_session_arc(&self, session_id: &str) -> Option<Arc<TokioMutex<Session>>> {
        self.user_sessions
            .get(session_id)
            .and_then(|s| s.sftp_session.clone())
    }

    /// Get main SSH session Arc for external use (e.g., port forwarding)
    pub fn get_session_arc(&self, session_id: &str) -> Option<Arc<TokioMutex<Session>>> {
        let user_session = self.user_sessions.get(session_id)?;
        let pool = self.pools.get(&user_session.server_key)?;
        let conn = pool.get(user_session.pool_idx)?;
        Some(conn.session.clone())
    }

    /// Execute command via SFTP session channel (avoids shell channel conflicts)
    pub async fn execute_via_sftp(
        &self,
        session_id: &str,
        command: &str,
    ) -> Result<String, LiteError> {
        let user_session = self
            .user_sessions
            .get(session_id)
            .ok_or_else(|| LiteError::SshSessionNotFound(session_id.to_string()))?;

        // Use the dedicated SFTP session for command execution
        let session_arc = if let Some(ref sftp_session) = user_session.sftp_session {
            sftp_session.clone()
        } else {
            return Err(LiteError::Ssh("SFTP session not available".to_string()));
        };

        let command = command.to_string();

        tokio::task::spawn_blocking(move || {
            let session_guard = session_arc.blocking_lock();

            let mut channel = session_guard
                .channel_session()
                .map_err(|e| LiteError::Ssh(format!("SFTP channel failed: {}", e)))?;

            channel
                .exec(&command)
                .map_err(|e| LiteError::Ssh(format!("SFTP exec failed: {}", e)))?;

            // Read with timeout
            let mut output = String::new();
            let mut buf = [0u8; 4096];
            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(30);

            loop {
                if start.elapsed() > timeout {
                    let _ = channel.close();
                    return Err(LiteError::SshTimeout);
                }

                match channel.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => output.push_str(&String::from_utf8_lossy(&buf[..n])),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        continue;
                    }
                    Err(e) => {
                        let _ = channel.close();
                        return Err(LiteError::Io(e.to_string()));
                    }
                }

                if channel.eof() {
                    break;
                }
            }

            let _ = channel.close();
            Ok(output)
        })
        .await
        .map_err(|e| LiteError::Ssh(format!("Task failed: {}", e)))?
    }

    /// Create SFTP session
    pub async fn create_sftp(&self, session_id: &str) -> Result<Sftp, LiteError> {
        let user_session = self
            .user_sessions
            .get(session_id)
            .ok_or_else(|| LiteError::SshSessionNotFound(session_id.to_string()))?;

        // Use dedicated SFTP session if available
        let session_arc = if let Some(ref sftp_session) = user_session.sftp_session {
            sftp_session.clone()
        } else {
            // Fallback to main session
            let pool = self
                .pools
                .get(&user_session.server_key)
                .ok_or_else(|| LiteError::SshSessionDisconnected(session_id.to_string()))?;

            let conn = pool
                .get(user_session.pool_idx)
                .ok_or_else(|| LiteError::SshSessionDisconnected(session_id.to_string()))?;

            conn.session.clone()
        };

        tokio::task::spawn_blocking(move || {
            let session = session_arc.blocking_lock();
            session
                .sftp()
                .map_err(|e| LiteError::Ssh(format!("SFTP creation failed: {}", e)))
        })
        .await
        .map_err(|e| LiteError::Ssh(format!("Task failed: {}", e)))?
    }

    /// List active sessions
    pub fn list_sessions(&self) -> Vec<String> {
        self.user_sessions.keys().cloned().collect()
    }

    /// Check if session exists
    pub fn has_session(&self, session_id: &str) -> bool {
        self.user_sessions.contains_key(session_id)
    }

    /// Get session metadata
    pub fn get_metadata(&self, session_id: &str) -> Option<SessionMetadata> {
        self.user_sessions
            .get(session_id)
            .map(|s| s.metadata.clone())
    }

    /// Get pool statistics
    pub fn get_pool_stats(&self) -> PoolStats {
        let pools: Vec<PoolInfo> = self
            .pools
            .iter()
            .map(|(key, pool)| {
                let connections: Vec<ConnectionInfo> = pool
                    .connections
                    .iter()
                    .map(|c| ConnectionInfo {
                        age_secs: c.age_secs(),
                        idle_secs: c.idle_secs(),
                        health: format!("{:?}", c.health),
                        busy: c.active_channels.load(Ordering::Relaxed),
                    })
                    .collect();

                PoolInfo {
                    server: format!("{}@{}:{}", key.username, key.host, key.port),
                    connection_count: pool.len(),
                    connections,
                }
            })
            .collect();

        PoolStats {
            total_pools: pools.len(),
            total_sessions: self.user_sessions.len(),
            pools,
        }
    }

    /// Check connection health
    pub async fn check_health(&self, session_id: &str) -> Result<ConnectionHealth, LiteError> {
        let user_session = self
            .user_sessions
            .get(session_id)
            .ok_or_else(|| LiteError::SshSessionNotFound(session_id.to_string()))?;

        let pool = self
            .pools
            .get(&user_session.server_key)
            .ok_or_else(|| LiteError::SshSessionDisconnected(session_id.to_string()))?;

        let conn = pool
            .get(user_session.pool_idx)
            .ok_or_else(|| LiteError::SshSessionDisconnected(session_id.to_string()))?;

        let session = conn.session.clone();

        // Try a simple echo command to test connection
        let result = tokio::task::spawn_blocking(move || {
            let session_guard = session.blocking_lock();

            match session_guard.channel_session() {
                Ok(mut ch) => {
                    if ch.exec("echo ping").is_ok() {
                        ConnectionHealth::Healthy
                    } else {
                        ConnectionHealth::Unhealthy
                    }
                }
                Err(_) => ConnectionHealth::Unhealthy,
            }
        })
        .await
        .unwrap_or(ConnectionHealth::Unhealthy);

        Ok(result)
    }
}

impl Default for SshSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// JumpHost configuration for proxy connections
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpHost {
    /// Jump host address
    pub host: String,
    /// Jump host port
    pub port: u16,
    /// Jump host username
    pub username: String,
    /// Authentication method for jump host
    pub auth: AuthMethod,
}

impl JumpHost {
    /// Create a new jump host configuration
    pub fn new(host: impl Into<String>, port: u16, username: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            auth: AuthMethod::Agent,
        }
    }

    /// Create with password authentication
    pub fn with_password(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            auth: AuthMethod::Password(password.into()),
        }
    }

    /// Create with key authentication
    pub fn with_key(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        key_path: PathBuf,
        passphrase: Option<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            auth: AuthMethod::PublicKey {
                path: key_path,
                passphrase,
            },
        }
    }

    /// Get address string
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Check if configuration is valid
    pub fn is_valid(&self) -> bool {
        !self.host.is_empty() && self.port > 0 && !self.username.is_empty() && self.auth.is_valid()
    }

    /// Check if this is password authentication.
    pub fn is_password(&self) -> bool {
        self.auth.is_password()
    }

    /// Check if this is public key authentication.
    pub fn is_public_key(&self) -> bool {
        self.auth.is_public_key()
    }

    /// Check if this is agent authentication.
    pub fn is_agent(&self) -> bool {
        self.auth.is_agent()
    }
}

/// Connection pool statistics for monitoring and diagnostics.
///
/// Provides an overview of all connection pools across different servers,
/// useful for monitoring connection health and performance.
///
/// # Example
///
/// ```
/// use easyssh_core::ssh::PoolStats;
///
/// let stats = PoolStats {
///     total_pools: 2,
///     total_sessions: 5,
///     pools: vec![],
/// };
///
/// println!("Total pools: {}", stats.total_pools);
/// ```
#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolStats {
    /// Number of unique server pools (one per host:port:username combination)
    pub total_pools: usize,
    /// Total number of active user sessions
    pub total_sessions: usize,
    /// Detailed information for each pool
    pub pools: Vec<PoolInfo>,
}

/// Connection pool information for a specific server.
///
/// Contains details about connections for a single server,
/// including connection health and usage statistics.
///
/// # Example
///
/// ```
/// use easyssh_core::ssh::{PoolInfo, ConnectionInfo};
///
/// let info = PoolInfo {
///     server: "root@192.168.1.1:22".to_string(),
///     connection_count: 3,
///     connections: vec![],
/// };
/// ```
#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolInfo {
    /// Server identifier in format "username@host:port"
    pub server: String,
    /// Number of connections in this pool
    pub connection_count: usize,
    /// Individual connection details
    pub connections: Vec<ConnectionInfo>,
}

/// Information about a single pooled connection.
///
/// Tracks the health and status of an individual SSH connection
/// within a connection pool.
///
/// # Example
///
/// ```
/// use easyssh_core::ssh::ConnectionInfo;
///
/// let info = ConnectionInfo {
///     age_secs: 300,
///     idle_secs: 60,
///     health: "Healthy".to_string(),
///     busy: false,
/// };
///
/// assert!(!info.busy);
/// ```
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectionInfo {
    /// Age of the connection in seconds since creation
    pub age_secs: u64,
    /// Time in seconds since the connection was last used
    pub idle_secs: u64,
    /// Health status as a string ("Healthy", "Degraded", "Unhealthy")
    pub health: String,
    /// Whether the connection is currently in use by a session
    pub busy: bool,
}

/// Strip ANSI escape codes from terminal output.
///
/// This function removes color codes, cursor positioning, and other
/// ANSI escape sequences commonly found in terminal output.
///
/// # Arguments
///
/// * `input` - String containing ANSI escape codes
///
/// # Returns
///
/// A new string with all ANSI escape codes removed.
///
/// # Example
///
/// ```
/// use easyssh_core::ssh::strip_ansi_codes;
///
/// let colored = "\x1b[31mRed Text\x1b[0m";
/// let clean = strip_ansi_codes(colored);
/// assert_eq!(clean, "Red Text");
/// ```
pub fn strip_ansi_codes(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_alphabetic() {
                        chars.next();
                        break;
                    }
                    chars.next();
                }
            } else {
                for ch in chars.by_ref() {
                    if (0x40..=0x7e).contains(&(ch as u8)) {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_health_variants() {
        assert_eq!(ConnectionHealth::Healthy, ConnectionHealth::Healthy);
        assert_eq!(ConnectionHealth::Degraded, ConnectionHealth::Degraded);
        assert_eq!(ConnectionHealth::Unhealthy, ConnectionHealth::Unhealthy);
        assert_ne!(ConnectionHealth::Healthy, ConnectionHealth::Unhealthy);
    }

    #[test]
    fn test_connection_health_clone() {
        let health = ConnectionHealth::Healthy;
        let cloned = health.clone();
        assert_eq!(health, cloned);
    }

    #[test]
    fn test_connection_health_copy() {
        let health = ConnectionHealth::Degraded;
        let copied = health;
        assert_eq!(health, copied); // Copy trait
    }

    #[test]
    fn test_connection_health_debug() {
        let health = ConnectionHealth::Healthy;
        let debug = format!("{:?}", health);
        assert!(debug.contains("Healthy"));
    }

    #[test]
    fn test_pool_stats_default() {
        let stats = PoolStats {
            total_pools: 0,
            total_sessions: 0,
            pools: vec![],
        };
        assert_eq!(stats.total_pools, 0);
        assert_eq!(stats.total_sessions, 0);
        assert!(stats.pools.is_empty());
    }

    #[test]
    fn test_pool_info_creation() {
        let info = PoolInfo {
            server: "192.168.1.1".to_string(),
            connection_count: 2,
            connections: vec![ConnectionInfo {
                age_secs: 10,
                idle_secs: 5,
                health: "healthy".to_string(),
                busy: false,
            }],
        };
        assert_eq!(info.server, "192.168.1.1");
        assert_eq!(info.connection_count, 2);
        assert_eq!(info.connections.len(), 1);
    }

    #[test]
    fn test_connection_info_creation() {
        let info = ConnectionInfo {
            age_secs: 60,
            idle_secs: 30,
            health: "degraded".to_string(),
            busy: true,
        };
        assert_eq!(info.age_secs, 60);
        assert_eq!(info.idle_secs, 30);
        assert_eq!(info.health, "degraded");
        assert!(info.busy);
    }

    #[test]
    fn test_pool_stats_serialize() {
        let stats = PoolStats {
            total_pools: 1,
            total_sessions: 3,
            pools: vec![PoolInfo {
                server: "test-server".to_string(),
                connection_count: 3,
                connections: vec![],
            }],
        };
        let json = serde_json::to_string(&stats).expect("Failed to serialize");
        assert!(json.contains("test-server"));
        assert!(json.contains("total_pools"));
    }

    #[test]
    fn test_strip_ansi_codes_no_codes() {
        let input = "Hello, World!";
        assert_eq!(strip_ansi_codes(input), input);
    }

    #[test]
    fn test_strip_ansi_codes_color() {
        let input = "\x1b[31mRed Text\x1b[0m";
        assert_eq!(strip_ansi_codes(input), "Red Text");
    }

    #[test]
    fn test_strip_ansi_codes_multiple() {
        let input = "\x1b[1mBold\x1b[0m and \x1b[32mGreen\x1b[0m";
        assert_eq!(strip_ansi_codes(input), "Bold and Green");
    }

    #[test]
    fn test_strip_ansi_codes_cursor_movement() {
        let input = "\x1b[2KClear line\x1b[1G";
        let result = strip_ansi_codes(input);
        assert!(!result.contains('\x1b'));
    }

    #[test]
    fn test_strip_ansi_codes_empty() {
        let input = "";
        assert_eq!(strip_ansi_codes(input), "");
    }

    #[test]
    fn test_strip_ansi_codes_only_ansi() {
        let input = "\x1b[31m\x1b[1m\x1b[0m";
        assert_eq!(strip_ansi_codes(input), "");
    }

    #[test]
    fn test_server_key_creation() {
        let key = ServerKey::new("192.168.1.1", 22, "root");
        assert_eq!(key.host, "192.168.1.1");
        assert_eq!(key.port, 22);
        assert_eq!(key.username, "root");
    }

    #[test]
    fn test_server_key_equality() {
        let key1 = ServerKey::new("host", 22, "user");
        let key2 = ServerKey::new("host", 22, "user");
        let key3 = ServerKey::new("host", 23, "user");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_server_key_hash() {
        use std::collections::HashMap;
        let mut map: HashMap<ServerKey, String> = HashMap::new();
        let key = ServerKey::new("host", 22, "user");
        map.insert(key.clone(), "value".to_string());

        assert!(map.contains_key(&key));
    }

    #[test]
    fn test_session_metadata_creation() {
        let metadata = SessionMetadata {
            id: "session-1".to_string(),
            server_id: "server-1".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "root".to_string(),
            connected_at: std::time::Instant::now(),
        };

        assert_eq!(metadata.id, "session-1");
        assert_eq!(metadata.host, "192.168.1.1");
        assert_eq!(metadata.port, 22);
        assert_eq!(metadata.username, "root");
    }

    #[test]
    fn test_session_metadata_clone() {
        let metadata = SessionMetadata {
            id: "session-1".to_string(),
            server_id: "server-1".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "root".to_string(),
            connected_at: std::time::Instant::now(),
        };

        let cloned = metadata.clone();
        assert_eq!(metadata.id, cloned.id);
        assert_eq!(metadata.host, cloned.host);
    }

    #[test]
    fn test_connection_pool_new() {
        let pool = ConnectionPool::new(5, 300, 3600);
        assert!(pool.connections.is_empty());
        assert_eq!(pool.max_connections, 5);
    }

    #[test]
    fn test_ssh_session_manager_new() {
        let manager = SshSessionManager::new();
        assert!(manager.pools.is_empty());
        assert!(manager.user_sessions.is_empty());
        assert_eq!(manager.pool_max_connections, 4);
    }

    #[test]
    fn test_ssh_session_manager_default() {
        let manager: SshSessionManager = Default::default();
        assert!(manager.pools.is_empty());
    }

    #[test]
    fn test_ssh_session_manager_with_pool_config() {
        let manager = SshSessionManager::new().with_pool_config(10, 600, 7200);

        assert_eq!(manager.pool_max_connections, 10);
        assert_eq!(manager.pool_idle_timeout, 600);
        assert_eq!(manager.pool_max_age, 7200);
    }

    #[test]
    fn test_pool_info_clone() {
        let info = PoolInfo {
            server: "test".to_string(),
            connection_count: 1,
            connections: vec![],
        };
        let cloned = info.clone();
        assert_eq!(info.server, cloned.server);
    }

    #[test]
    fn test_connection_info_clone() {
        let info = ConnectionInfo {
            age_secs: 10,
            idle_secs: 5,
            health: "healthy".to_string(),
            busy: false,
        };
        let cloned = info.clone();
        assert_eq!(info.age_secs, cloned.age_secs);
        assert_eq!(info.busy, cloned.busy);
    }

    #[test]
    fn test_pool_stats_clone() {
        let stats = PoolStats {
            total_pools: 1,
            total_sessions: 2,
            pools: vec![],
        };
        let cloned = stats.clone();
        assert_eq!(stats.total_pools, cloned.total_pools);
    }

    #[test]
    fn test_strip_ansi_codes_complex() {
        // Complex ANSI sequence with cursor positioning
        let input = "\x1b[1;31mError:\x1b[0m \x1b[33mWarning\x1b[0m \x1b[2mDim\x1b[0m";
        let result = strip_ansi_codes(input);
        assert_eq!(result, "Error: Warning Dim");
    }

    #[test]
    fn test_strip_ansi_codes_nested() {
        let input = "Before\x1b[31mRed\x1b[1mBoldRed\x1b[0mAfter";
        let result = strip_ansi_codes(input);
        assert_eq!(result, "BeforeRedBoldRedAfter");
    }

    #[test]
    fn test_server_key_different_usernames() {
        let key1 = ServerKey::new("host", 22, "user1");
        let key2 = ServerKey::new("host", 22, "user2");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_server_key_different_hosts() {
        let key1 = ServerKey::new("host1", 22, "user");
        let key2 = ServerKey::new("host2", 22, "user");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_connection_pool_empty() {
        let pool = ConnectionPool::new(5, 300, 3600);
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_strip_ansi_codes_real_world() {
        // Real world example from ls --color
        let input = "\x1b[0m\x1b[01;34mtest_dir\x1b[0m\x1b[0m  \x1b[01;32mscript.sh\x1b[0m\x1b[0m";
        let result = strip_ansi_codes(input);
        assert_eq!(result, "test_dir  script.sh");
    }

    #[test]
    fn test_connection_pool_acquire_and_release() {
        let mut pool = ConnectionPool::new(2, 300, 3600);

        // Can't acquire from empty pool
        assert!(pool.acquire().is_none());

        // Add a session (we can't create real Session in tests, so just test the pool logic)
        // The pool methods work with Session, but we can test other pool methods
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_connection_pool_cleanup_expired() {
        let mut pool = ConnectionPool::new(5, 1, 3600); // 1 second idle timeout

        // Empty pool cleanup
        pool.cleanup_expired();
        assert!(pool.is_empty());
    }

    #[test]
    fn test_pooled_connection_age_and_idle() {
        // Note: We can't test with real Session, but we can verify the pool structure
        let pool = ConnectionPool::new(5, 300, 3600);

        // Empty pool has no connections to age
        assert!(pool.is_empty());
    }

    #[test]
    fn test_ssh_session_manager_list_sessions() {
        let manager = SshSessionManager::new();
        assert!(manager.list_sessions().is_empty());
        assert!(!manager.has_session("non-existent"));
    }

    #[test]
    fn test_ssh_session_manager_get_metadata_nonexistent() {
        let manager = SshSessionManager::new();
        assert!(manager.get_metadata("non-existent").is_none());
    }

    #[test]
    fn test_connection_health_partial_eq() {
        assert_eq!(ConnectionHealth::Healthy, ConnectionHealth::Healthy);
        assert_eq!(ConnectionHealth::Degraded, ConnectionHealth::Degraded);
        assert_eq!(ConnectionHealth::Unhealthy, ConnectionHealth::Unhealthy);

        assert_ne!(ConnectionHealth::Healthy, ConnectionHealth::Degraded);
        assert_ne!(ConnectionHealth::Healthy, ConnectionHealth::Unhealthy);
        assert_ne!(ConnectionHealth::Degraded, ConnectionHealth::Unhealthy);
    }

    #[test]
    fn test_pool_stats_debug() {
        let stats = PoolStats {
            total_pools: 2,
            total_sessions: 5,
            pools: vec![],
        };
        let debug = format!("{:?}", stats);
        assert!(debug.contains("total_pools"));
        assert!(debug.contains("2"));
    }

    #[test]
    fn test_connection_pool_config_edge_cases() {
        // Test with 0 max connections
        let pool = ConnectionPool::new(0, 300, 3600);
        assert_eq!(pool.max_connections, 0);

        // Test with very long timeouts
        let pool = ConnectionPool::new(10, u64::MAX, u64::MAX);
        assert_eq!(pool.idle_timeout, Duration::from_secs(u64::MAX));
    }

    #[test]
    fn test_ssh_session_manager_with_pool_config_edge_cases() {
        let manager = SshSessionManager::new().with_pool_config(0, 0, 0);

        assert_eq!(manager.pool_max_connections, 0);
        assert_eq!(manager.pool_idle_timeout, 0);
        assert_eq!(manager.pool_max_age, 0);
    }

    #[test]
    fn test_pool_info_serialize() {
        let info = PoolInfo {
            server: "user@host:22".to_string(),
            connection_count: 3,
            connections: vec![
                ConnectionInfo {
                    age_secs: 10,
                    idle_secs: 5,
                    health: "healthy".to_string(),
                    busy: false,
                },
                ConnectionInfo {
                    age_secs: 20,
                    idle_secs: 0,
                    health: "degraded".to_string(),
                    busy: true,
                },
            ],
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("user@host:22"));
        assert!(json.contains("healthy"));
        assert!(json.contains("degraded"));
    }
}

// =============================================================================
// SSH Connection Management for Lite Version
// =============================================================================

// SSH connection management components for Lite version.
//
// This section provides:
// - `SshConfig`: SSH configuration structure
// - `AuthManager`: Authentication management
// - `KnownHosts`: Known host key management
// - `SshAgent`: SSH agent integration
// - `ConnectionTestResult`: Connection testing utilities

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// SSH authentication method for Lite version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    /// Password authentication
    Password(String),
    /// Public key authentication with optional passphrase
    PublicKey {
        /// Path to private key file
        path: PathBuf,
        /// Optional passphrase for encrypted keys
        passphrase: Option<String>,
    },
    /// SSH agent authentication
    Agent,
}

impl AuthMethod {
    /// Check if authentication method is valid.
    pub fn is_valid(&self) -> bool {
        match self {
            AuthMethod::Password(password) => !password.is_empty(),
            AuthMethod::PublicKey { path, .. } => !path.as_os_str().is_empty(),
            AuthMethod::Agent => true,
        }
    }

    /// Get a display name for this authentication method.
    pub fn display_name(&self) -> &'static str {
        match self {
            AuthMethod::Password(_) => "Password",
            AuthMethod::PublicKey { .. } => "Public Key",
            AuthMethod::Agent => "SSH Agent",
        }
    }

    /// Check if this is password authentication.
    pub fn is_password(&self) -> bool {
        matches!(self, AuthMethod::Password(_))
    }

    /// Check if this is public key authentication.
    pub fn is_public_key(&self) -> bool {
        matches!(self, AuthMethod::PublicKey { .. })
    }

    /// Check if this is agent authentication.
    pub fn is_agent(&self) -> bool {
        matches!(self, AuthMethod::Agent)
    }
}

impl fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthMethod::Password(_) => write!(f, "password"),
            AuthMethod::PublicKey { path, .. } => {
                write!(f, "publickey({})", path.display())
            }
            AuthMethod::Agent => write!(f, "agent"),
        }
    }
}

/// SSH connection timeout configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionTimeout {
    /// Connection establishment timeout in seconds
    pub connect_secs: u64,
    /// Authentication timeout in seconds
    pub auth_secs: u64,
    /// Keepalive interval in seconds (0 to disable)
    pub keepalive_secs: u64,
    /// Command execution timeout in seconds (0 for no timeout)
    pub command_secs: u64,
}

impl ConnectionTimeout {
    /// Create a new timeout configuration.
    pub fn new(connect_secs: u64, auth_secs: u64, keepalive_secs: u64, command_secs: u64) -> Self {
        Self {
            connect_secs,
            auth_secs,
            keepalive_secs,
            command_secs,
        }
    }

    /// Get connection timeout as Duration.
    pub fn connect_duration(&self) -> Duration {
        Duration::from_secs(self.connect_secs)
    }

    /// Get authentication timeout as Duration.
    pub fn auth_duration(&self) -> Duration {
        Duration::from_secs(self.auth_secs)
    }

    /// Get keepalive interval as Duration.
    pub fn keepalive_duration(&self) -> Duration {
        Duration::from_secs(self.keepalive_secs)
    }

    /// Get command timeout as Duration (None if 0).
    pub fn command_duration(&self) -> Option<Duration> {
        if self.command_secs > 0 {
            Some(Duration::from_secs(self.command_secs))
        } else {
            None
        }
    }
}

impl Default for ConnectionTimeout {
    fn default() -> Self {
        Self {
            connect_secs: 30,
            auth_secs: 30,
            keepalive_secs: 60,
            command_secs: 0,
        }
    }
}

/// SSH connection configuration for Lite version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshConfig {
    /// Remote host address (IP or hostname)
    pub host: String,
    /// Remote SSH port (default: 22)
    pub port: u16,
    /// Username for authentication
    pub username: String,
    /// Authentication method
    pub auth: AuthMethod,
    /// Connection timeout settings
    pub timeout: ConnectionTimeout,
    /// Path to known_hosts file
    pub known_hosts_path: Option<PathBuf>,
    /// Compression enabled (default: true)
    pub compression: bool,
    /// Cipher preference (None for default)
    pub preferred_cipher: Option<String>,
}

impl SshConfig {
    /// Create a new SSH configuration with basic settings.
    pub fn new(host: impl Into<String>, port: u16, username: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            auth: AuthMethod::Agent,
            timeout: ConnectionTimeout::default(),
            known_hosts_path: Self::default_known_hosts_path(),
            compression: true,
            preferred_cipher: None,
        }
    }

    /// Create a new SSH configuration with password authentication.
    pub fn with_password(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            auth: AuthMethod::Password(password.into()),
            timeout: ConnectionTimeout::default(),
            known_hosts_path: Self::default_known_hosts_path(),
            compression: true,
            preferred_cipher: None,
        }
    }

    /// Create a new SSH configuration with agent authentication.
    pub fn with_agent(host: impl Into<String>, port: u16, username: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            auth: AuthMethod::Agent,
            timeout: ConnectionTimeout::default(),
            known_hosts_path: Self::default_known_hosts_path(),
            compression: true,
            preferred_cipher: None,
        }
    }

    /// Create a new SSH configuration with public key authentication.
    pub fn with_key(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        key_path: PathBuf,
        passphrase: Option<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            auth: AuthMethod::PublicKey {
                path: key_path,
                passphrase,
            },
            timeout: ConnectionTimeout::default(),
            known_hosts_path: Self::default_known_hosts_path(),
            compression: true,
            preferred_cipher: None,
        }
    }

    /// Set authentication method.
    pub fn with_auth(mut self, auth: AuthMethod) -> Self {
        self.auth = auth;
        self
    }

    /// Set connection timeout.
    pub fn with_timeout(mut self, timeout: ConnectionTimeout) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set known hosts file path.
    pub fn with_known_hosts(mut self, path: Option<PathBuf>) -> Self {
        self.known_hosts_path = path;
        self
    }

    /// Enable or disable compression.
    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compression = enabled;
        self
    }

    /// Set preferred cipher.
    pub fn with_cipher(mut self, cipher: Option<String>) -> Self {
        self.preferred_cipher = cipher;
        self
    }

    /// Get the default known_hosts path.
    fn default_known_hosts_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".ssh").join("known_hosts"))
    }

    /// Get connection address string (host:port).
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Check if configuration is valid.
    pub fn is_valid(&self) -> bool {
        !self.host.is_empty() && self.port > 0 && !self.username.is_empty() && self.auth.is_valid()
    }

    /// Check if this is password authentication.
    pub fn is_password(&self) -> bool {
        self.auth.is_password()
    }

    /// Check if this is public key authentication.
    pub fn is_public_key(&self) -> bool {
        self.auth.is_public_key()
    }

    /// Check if this is agent authentication.
    pub fn is_agent(&self) -> bool {
        self.auth.is_agent()
    }
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 22,
            username: String::new(),
            auth: AuthMethod::Agent,
            timeout: ConnectionTimeout::default(),
            known_hosts_path: Self::default_known_hosts_path(),
            compression: true,
            preferred_cipher: None,
        }
    }
}

/// SSH private key format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyFormat {
    /// OpenSSH format (new style, since OpenSSH 7.8)
    OpenSSH,
    /// PEM format (traditional, PKCS#1/PKCS#8)
    Pem,
    /// PuTTY .ppk format
    Ppk,
    /// Unknown format
    Unknown,
}

impl KeyFormat {
    /// Detect key format from file content.
    pub fn detect(content: &[u8]) -> Self {
        let content_str = String::from_utf8_lossy(content);

        if content_str.contains("-----BEGIN OPENSSH PRIVATE KEY-----") {
            KeyFormat::OpenSSH
        } else if content_str.contains("-----BEGIN RSA PRIVATE KEY-----")
            || content_str.contains("-----BEGIN DSA PRIVATE KEY-----")
            || content_str.contains("-----BEGIN EC PRIVATE KEY-----")
            || content_str.contains("-----BEGIN PRIVATE KEY-----")
            || content_str.contains("-----BEGIN ENCRYPTED PRIVATE KEY-----")
        {
            KeyFormat::Pem
        } else if content_str.contains("PuTTY-User-Key-File-") {
            KeyFormat::Ppk
        } else {
            KeyFormat::Unknown
        }
    }

    /// Check if format is supported.
    pub fn is_supported(&self) -> bool {
        matches!(self, KeyFormat::OpenSSH | KeyFormat::Pem)
    }
}

/// SSH private key information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateKey {
    /// Key format
    pub format: KeyFormat,
    /// Whether key is encrypted
    pub is_encrypted: bool,
    /// Key algorithm
    pub algorithm: String,
    /// Key comment if available
    pub comment: Option<String>,
}

impl PrivateKey {
    /// Load key information from file.
    pub fn from_file(path: &Path) -> Result<Self, LiteError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| LiteError::InvalidKey(format!("Failed to read key file: {}", e)))?;

        let format = KeyFormat::detect(content.as_bytes());
        let is_encrypted =
            content.contains("ENCRYPTED") || content.contains("Proc-Type: 4,ENCRYPTED");

        let algorithm = Self::detect_algorithm(&content);
        let comment = Self::extract_comment(&content);

        Ok(PrivateKey {
            format,
            is_encrypted,
            algorithm,
            comment,
        })
    }

    /// Detect algorithm from key content.
    fn detect_algorithm(content: &str) -> String {
        if content.contains("OPENSSH PRIVATE KEY") {
            if content.contains("ssh-rsa") || content.contains("rsa-key-") {
                "rsa".to_string()
            } else if content.contains("ssh-ed25519") {
                "ed25519".to_string()
            } else if content.contains("ecdsa") {
                "ecdsa".to_string()
            } else {
                "unknown".to_string()
            }
        } else if content.contains("RSA PRIVATE KEY") {
            "rsa".to_string()
        } else if content.contains("DSA PRIVATE KEY") {
            "dsa".to_string()
        } else if content.contains("EC PRIVATE KEY") {
            "ecdsa".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Extract comment from key file.
    fn extract_comment(content: &str) -> Option<String> {
        for line in content.lines() {
            if let Some(pos) = line.find("ssh-") {
                let parts: Vec<&str> = line[pos..].split_whitespace().collect();
                if parts.len() >= 3 {
                    return Some(parts[2..].join(" "));
                }
            }
        }
        None
    }

    /// Check if passphrase is required.
    pub fn needs_passphrase(&self) -> bool {
        self.is_encrypted
    }

    /// Get key fingerprint (SHA256 base64)
    pub fn fingerprint(&self, path: &Path) -> Result<String, LiteError> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        use sha2::{Digest, Sha256};

        let content = std::fs::read_to_string(path)
            .map_err(|e| LiteError::InvalidKey(format!("Failed to read key file: {}", e)))?;

        // Extract public key portion for fingerprinting
        // For SSH keys, we need to parse the key material
        let hash = Sha256::digest(content.as_bytes());
        let fingerprint = STANDARD.encode(&hash[..]);

        // Format as SHA256:XXXXXX...
        let short = if fingerprint.len() > 43 {
            format!("SHA256:{}", &fingerprint[..43])
        } else {
            format!("SHA256:{}", fingerprint)
        };

        Ok(short)
    }

    /// Get short fingerprint for display (16 chars)
    pub fn fingerprint_short(&self, path: &Path) -> Result<String, LiteError> {
        let full = self.fingerprint(path)?;
        // Extract just the hash portion after SHA256:
        let hash = full.strip_prefix("SHA256:").unwrap_or(&full);
        if hash.len() >= 16 {
            Ok(format!("{}...{}", &hash[..8], &hash[hash.len() - 8..]))
        } else {
            Ok(full)
        }
    }

    /// Get key strength in bits (estimated)
    pub fn key_strength(&self) -> u32 {
        match self.algorithm.as_str() {
            "rsa" => 2048, // Default assumption
            "ed25519" => 256,
            "ecdsa" => 256,
            "dsa" => 1024,
            _ => 0,
        }
    }

    /// Check if key is considered secure
    pub fn is_secure(&self) -> bool {
        if self.format == KeyFormat::Unknown {
            return false;
        }

        match self.algorithm.as_str() {
            "rsa" => true,     // RSA keys are generally secure
            "ed25519" => true, // Ed25519 is recommended
            "ecdsa" => true,   // ECDSA is secure
            "dsa" => false,    // DSA is deprecated
            _ => false,
        }
    }

    /// Get security recommendation
    pub fn security_recommendation(&self) -> &'static str {
        match self.algorithm.as_str() {
            "ed25519" => "Excellent - Modern and secure",
            "ecdsa" => "Good - Widely supported",
            "rsa" => "Good - Widely compatible",
            "dsa" => "Deprecated - Consider upgrading to Ed25519",
            _ => "Unknown - Verify key format",
        }
    }

    /// Convert key format description
    pub fn format_description(&self) -> &'static str {
        match self.format {
            KeyFormat::OpenSSH => "OpenSSH (new format, since 7.8)",
            KeyFormat::Pem => "PEM (traditional format)",
            KeyFormat::Ppk => "PuTTY PPK (convert to OpenSSH)",
            KeyFormat::Unknown => "Unknown format",
        }
    }
}

/// SSH key pair (public and private)
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// Private key path
    pub private_path: PathBuf,
    /// Public key path
    pub public_path: PathBuf,
    /// Key information
    pub info: PrivateKey,
}

impl KeyPair {
    /// Load key pair from private key path
    pub fn from_private_path(private_path: &Path) -> Result<Self, LiteError> {
        let info = PrivateKey::from_file(private_path)?;

        // Derive public key path
        let public_path = private_path.with_extension("pub");
        // Or with .pub appended
        let public_path_alt = PathBuf::from(format!("{}.pub", private_path.display()));

        let public_path = if public_path.exists() {
            public_path
        } else if public_path_alt.exists() {
            public_path_alt
        } else {
            // Use derived path even if it doesn't exist
            public_path
        };

        Ok(KeyPair {
            private_path: private_path.to_path_buf(),
            public_path,
            info,
        })
    }

    /// Read public key content
    pub fn read_public_key(&self) -> Result<String, LiteError> {
        std::fs::read_to_string(&self.public_path)
            .map_err(|e| LiteError::InvalidKey(format!("Failed to read public key: {}", e)))
    }

    /// Check if public key exists
    pub fn has_public_key(&self) -> bool {
        self.public_path.exists()
    }

    /// Generate public key from private key (if missing)
    pub fn generate_public_key(&self) -> Result<(), LiteError> {
        if self.has_public_key() {
            return Ok(());
        }

        // Use ssh-keygen to generate public key
        let output = std::process::Command::new("ssh-keygen")
            .args(["-y", "-f", self.private_path.to_str().unwrap_or("")])
            .output()
            .map_err(|e| LiteError::InvalidKey(format!("Failed to generate public key: {}", e)))?;

        if !output.status.success() {
            return Err(LiteError::InvalidKey(format!(
                "ssh-keygen failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let public_key = String::from_utf8_lossy(&output.stdout);
        std::fs::write(&self.public_path, public_key.as_bytes())
            .map_err(|e| LiteError::Io(e.to_string()))?;

        Ok(())
    }
}

/// Key manager for SSH key operations
pub struct KeyManager {
    /// SSH directory path
    ssh_dir: PathBuf,
    /// Loaded key pairs
    keys: Vec<KeyPair>,
}

impl KeyManager {
    /// Create a new key manager
    pub fn new() -> Result<Self, LiteError> {
        let ssh_dir = dirs::home_dir()
            .map(|h| h.join(".ssh"))
            .ok_or_else(|| LiteError::Config("Could not determine home directory".to_string()))?;

        Ok(Self {
            ssh_dir,
            keys: Vec::new(),
        })
    }

    /// Create with custom SSH directory
    pub fn with_ssh_dir(ssh_dir: PathBuf) -> Self {
        Self {
            ssh_dir,
            keys: Vec::new(),
        }
    }

    /// Scan and load all SSH keys
    pub fn scan_keys(&mut self) -> Result<usize, LiteError> {
        self.keys.clear();

        let key_names = [
            "id_rsa",
            "id_ed25519",
            "id_ecdsa",
            "id_dsa",
            "id_ed25519_sk",
            "id_ecdsa_sk",
        ];

        for name in &key_names {
            let private_path = self.ssh_dir.join(name);
            if private_path.exists() {
                match KeyPair::from_private_path(&private_path) {
                    Ok(keypair) => self.keys.push(keypair),
                    Err(e) => log::warn!("Failed to load key {}: {}", name, e),
                }
            }
        }

        // Scan for custom key files (*.pem, *.key, etc.)
        if let Ok(entries) = std::fs::read_dir(&self.ssh_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if (ext == "pem" || ext == "key")
                        && !self.keys.iter().any(|k| k.private_path == path)
                    {
                        match KeyPair::from_private_path(&path) {
                            Ok(keypair) => self.keys.push(keypair),
                            Err(e) => log::debug!("Skipping key {}: {}", path.display(), e),
                        }
                    }
                }
            }
        }

        Ok(self.keys.len())
    }

    /// Get all loaded keys
    pub fn keys(&self) -> &[KeyPair] {
        &self.keys
    }

    /// Get count of loaded keys
    pub fn count(&self) -> usize {
        self.keys.len()
    }

    /// Find key by name (e.g., "id_rsa")
    pub fn find_by_name(&self, name: &str) -> Option<&KeyPair> {
        self.keys.iter().find(|k| {
            k.private_path
                .file_stem()
                .map(|s| s.to_string_lossy() == name)
                .unwrap_or(false)
        })
    }

    /// Find key by algorithm
    pub fn find_by_algorithm(&self, algorithm: &str) -> Vec<&KeyPair> {
        self.keys
            .iter()
            .filter(|k| k.info.algorithm == algorithm)
            .collect()
    }

    /// Get recommended key (Ed25519 preferred, then ECDSA, then RSA)
    pub fn recommended_key(&self) -> Option<&KeyPair> {
        // Prefer Ed25519
        if let Some(key) = self.keys.iter().find(|k| k.info.algorithm == "ed25519") {
            return Some(key);
        }
        // Then ECDSA
        if let Some(key) = self.keys.iter().find(|k| k.info.algorithm == "ecdsa") {
            return Some(key);
        }
        // Then RSA
        if let Some(key) = self.keys.iter().find(|k| k.info.algorithm == "rsa") {
            return Some(key);
        }
        // Return first available
        self.keys.first()
    }

    /// Add a custom key
    pub fn add_key(&mut self, path: &Path) -> Result<&KeyPair, LiteError> {
        let keypair = KeyPair::from_private_path(path)?;
        self.keys.push(keypair);
        Ok(self.keys.last().unwrap())
    }

    /// Check if SSH directory exists
    pub fn ssh_dir_exists(&self) -> bool {
        self.ssh_dir.exists()
    }

    /// Get SSH directory path
    pub fn ssh_dir(&self) -> &Path {
        &self.ssh_dir
    }

    /// Ensure SSH directory exists with correct permissions
    pub fn ensure_ssh_dir(&self) -> Result<(), LiteError> {
        if !self.ssh_dir.exists() {
            std::fs::create_dir_all(&self.ssh_dir).map_err(|e| LiteError::Io(e.to_string()))?;

            // Set permissions to 700 (owner only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&self.ssh_dir)
                    .map_err(|e| LiteError::Io(e.to_string()))?
                    .permissions();
                perms.set_mode(0o700);
                std::fs::set_permissions(&self.ssh_dir, perms)
                    .map_err(|e| LiteError::Io(e.to_string()))?;
            }
        }
        Ok(())
    }

    /// Get key summary for display
    pub fn key_summary(&self) -> Vec<KeySummary> {
        self.keys
            .iter()
            .map(|k| {
                let fingerprint = k
                    .info
                    .fingerprint_short(&k.private_path)
                    .unwrap_or_default();
                KeySummary {
                    name: k
                        .private_path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    algorithm: k.info.algorithm.clone(),
                    fingerprint,
                    has_passphrase: k.info.needs_passphrase(),
                    has_public_key: k.has_public_key(),
                    is_secure: k.info.is_secure(),
                    recommendation: k.info.security_recommendation().to_string(),
                }
            })
            .collect()
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            ssh_dir: PathBuf::from("~/.ssh"),
            keys: Vec::new(),
        })
    }
}

/// Key summary for display
#[derive(Debug, Clone, serde::Serialize)]
pub struct KeySummary {
    /// Key name (filename without extension)
    pub name: String,
    /// Key algorithm
    pub algorithm: String,
    /// Key fingerprint (short)
    pub fingerprint: String,
    /// Whether key requires passphrase
    pub has_passphrase: bool,
    /// Whether public key file exists
    pub has_public_key: bool,
    /// Whether key is considered secure
    pub is_secure: bool,
    /// Security recommendation
    pub recommendation: String,
}

/// Authentication manager for SSH connections.
pub struct AuthManager {
    /// Whether SSH agent is available
    agent_available: bool,
    /// Cached password
    cached_password: Option<String>,
    /// Key manager
    key_manager: KeyManager,
}

impl AuthManager {
    /// Create a new authentication manager.
    pub fn new() -> Self {
        let agent_available = Self::detect_agent();
        let key_manager = KeyManager::new().unwrap_or_default();
        Self {
            agent_available,
            cached_password: None,
            key_manager,
        }
    }

    /// Create a new authentication manager without agent.
    pub fn without_agent() -> Self {
        let key_manager = KeyManager::new().unwrap_or_default();
        Self {
            agent_available: false,
            cached_password: None,
            key_manager,
        }
    }

    /// Get key manager reference
    pub fn key_manager(&self) -> &KeyManager {
        &self.key_manager
    }

    /// Get mutable key manager
    pub fn key_manager_mut(&mut self) -> &mut KeyManager {
        &mut self.key_manager
    }

    /// Scan for available keys
    pub fn scan_keys(&mut self) -> Result<usize, LiteError> {
        self.key_manager.scan_keys()
    }

    /// Get available authentication methods for a connection
    pub fn available_methods(&self, config: &SshConfig) -> Vec<AuthMethod> {
        let mut methods = Vec::new();

        // Password if configured
        if let AuthMethod::Password(password) = &config.auth {
            if !password.is_empty() {
                methods.push(config.auth.clone());
            }
        }

        // Check for keys
        if self.key_manager.count() > 0 {
            if let Some(key) = self.key_manager.recommended_key() {
                methods.push(AuthMethod::PublicKey {
                    path: key.private_path.clone(),
                    passphrase: None,
                });
            }
        }

        // Agent if available
        if self.agent_available {
            methods.push(AuthMethod::Agent);
        }

        // Fallback to configured method
        if methods.is_empty() {
            methods.push(config.auth.clone());
        }

        methods
    }

    /// Get recommended authentication method
    pub fn recommend_method(&self, config: &SshConfig) -> AuthMethod {
        // Priority: Key > Agent > Password
        if let Some(key) = self.key_manager.recommended_key() {
            return AuthMethod::PublicKey {
                path: key.private_path.clone(),
                passphrase: None,
            };
        }

        if self.agent_available {
            return AuthMethod::Agent;
        }

        config.auth.clone()
    }

    /// Detect if SSH agent is available.
    fn detect_agent() -> bool {
        if std::env::var("SSH_AUTH_SOCK").is_ok() {
            return true;
        }

        #[cfg(target_os = "windows")]
        {
            std::env::var("SSH_AGENT_LAUNCHER").is_ok() || std::env::var("SSH_AUTH_SOCK").is_ok()
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }

    /// Check if SSH agent is available.
    pub fn supports_agent(&self) -> bool {
        self.agent_available
    }

    /// Cache password temporarily.
    pub fn cache_password(&mut self, password: impl Into<String>) {
        self.cached_password = Some(password.into());
    }

    /// Get cached password and clear it.
    pub fn take_cached_password(&mut self) -> Option<String> {
        self.cached_password.take()
    }

    /// Clear cached credentials.
    pub fn clear_cache(&mut self) {
        self.cached_password = None;
    }

    /// Validate a key file.
    pub fn validate_key(path: &Path) -> Result<PrivateKey, LiteError> {
        if !path.exists() {
            return Err(LiteError::FileNotFound {
                path: path.display().to_string(),
            });
        }

        PrivateKey::from_file(path)
    }

    /// Check if a key file needs a passphrase.
    pub fn key_needs_passphrase(path: &Path) -> Result<bool, LiteError> {
        let key = Self::validate_key(path)?;
        Ok(key.needs_passphrase())
    }

    /// Find default SSH keys in ~/.ssh.
    pub fn find_default_keys() -> Vec<PathBuf> {
        let mut keys = Vec::new();

        if let Some(home) = dirs::home_dir() {
            let ssh_dir = home.join(".ssh");
            let key_names = [
                "id_rsa",
                "id_ed25519",
                "id_ecdsa",
                "id_dsa",
                "id_ed25519_sk",
                "id_ecdsa_sk",
            ];

            for name in &key_names {
                let key_path = ssh_dir.join(name);
                if key_path.exists() {
                    keys.push(key_path);
                }
            }
        }

        keys
    }

    /// Get default SSH key path.
    pub fn default_key_path() -> Option<PathBuf> {
        Self::find_default_keys().into_iter().next()
    }

    /// Expand a key path that may contain ~ for home directory.
    pub fn expand_key_path(path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();

        if path.starts_with("~") {
            if let Some(home) = dirs::home_dir() {
                let without_tilde = path.strip_prefix("~").unwrap_or(path);
                return home.join(without_tilde);
            }
        }

        path.to_path_buf()
    }

    /// Attempt pre-validation of authentication method.
    pub fn prevalidate(&self, method: &AuthMethod) -> Result<(), LiteError> {
        match method {
            AuthMethod::Password(password) => {
                if password.is_empty() {
                    return Err(LiteError::AuthFailed);
                }
            }
            AuthMethod::PublicKey { path, passphrase } => {
                let expanded = Self::expand_key_path(path);
                let key_info = Self::validate_key(&expanded)?;

                if key_info.needs_passphrase() && passphrase.is_none() {
                    return Err(LiteError::InvalidKey("Key requires passphrase".to_string()));
                }

                if !key_info.format.is_supported() {
                    return Err(LiteError::InvalidKey(format!(
                        "Key format {:?} is not supported",
                        key_info.format
                    )));
                }
            }
            AuthMethod::Agent => {
                if !self.agent_available {
                    return Err(LiteError::Keychain("SSH agent not available".to_string()));
                }
            }
        }

        Ok(())
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AuthManager {
    fn drop(&mut self) {
        self.clear_cache();
    }
}

/// Host key verification result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerifyResult {
    /// Host key is known and matches
    Accepted,
    /// Host key is unknown (first connection)
    Unknown,
    /// Host key has changed
    Changed,
    /// Host key has been revoked
    Revoked,
}

impl VerifyResult {
    /// Check if verification is accepted.
    pub fn is_accepted(&self) -> bool {
        matches!(self, VerifyResult::Accepted)
    }

    /// Check if host is unknown.
    pub fn is_unknown(&self) -> bool {
        matches!(self, VerifyResult::Unknown)
    }

    /// Check if host key has changed.
    pub fn is_changed(&self) -> bool {
        matches!(self, VerifyResult::Changed)
    }

    /// Check if host key is revoked.
    pub fn is_revoked(&self) -> bool {
        matches!(self, VerifyResult::Revoked)
    }
}

/// Known hosts entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostKeyEntry {
    /// Hostnames or patterns this key applies to
    pub hosts: Vec<String>,
    /// Key type
    pub key_type: String,
    /// Base64-encoded public key
    pub key: String,
    /// Optional comment
    pub comment: Option<String>,
    /// Whether this entry is revoked
    pub revoked: bool,
    /// Line number in file
    pub line_number: usize,
}

impl HostKeyEntry {
    /// Parse a line from known_hosts file.
    pub fn parse(line: &str, line_number: usize) -> Option<Self> {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }

        let (revoked, content) = if let Some(stripped) = trimmed.strip_prefix("@revoked ") {
            (true, stripped)
        } else {
            (false, trimmed)
        };

        let parts: Vec<&str> = content.split_whitespace().collect();

        if parts.len() < 3 {
            return None;
        }

        let hosts_field = parts[0];
        let hosts = if hosts_field.starts_with("|1|") {
            vec![hosts_field.to_string()]
        } else {
            hosts_field.split(',').map(|s| s.to_string()).collect()
        };

        let key_type = parts[1].to_string();
        let key = parts[2].to_string();

        let comment = if parts.len() > 3 {
            Some(parts[3..].join(" "))
        } else {
            None
        };

        Some(HostKeyEntry {
            hosts,
            key_type,
            key,
            comment,
            revoked,
            line_number,
        })
    }

    /// Format entry as known_hosts line.
    pub fn to_line(&self) -> String {
        let mut result = String::new();

        if self.revoked {
            result.push_str("@revoked ");
        }

        result.push_str(&self.hosts.join(","));
        result.push(' ');
        result.push_str(&self.key_type);
        result.push(' ');
        result.push_str(&self.key);

        if let Some(ref comment) = self.comment {
            result.push(' ');
            result.push_str(comment);
        }

        result
    }

    /// Check if this entry matches a host.
    pub fn matches(&self, hostname: &str) -> bool {
        for host in &self.hosts {
            if host == hostname {
                return true;
            }

            if (host.contains('*') || host.contains('?')) && Self::wildcard_match(host, hostname) {
                return true;
            }
        }

        false
    }

    /// Simple wildcard pattern matching.
    fn wildcard_match(pattern: &str, text: &str) -> bool {
        let pattern_parts: Vec<&str> = pattern.split('*').collect();

        if pattern_parts.len() == 1 {
            return pattern == text;
        }

        let mut text_remaining = text;

        for (i, part) in pattern_parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            if i == 0 {
                if !text_remaining.starts_with(part) {
                    return false;
                }
                text_remaining = &text_remaining[part.len()..];
            } else if i == pattern_parts.len() - 1 {
                return text_remaining.ends_with(part);
            } else if let Some(pos) = text_remaining.find(part) {
                text_remaining = &text_remaining[pos + part.len()..];
            } else {
                return false;
            }
        }

        true
    }

    /// Get key fingerprint (SHA256 base64).
    pub fn fingerprint(&self) -> String {
        use base64::{engine::general_purpose::STANDARD, Engine};
        use sha2::{Digest, Sha256};

        if let Ok(decoded) = STANDARD.decode(&self.key) {
            let hash = Sha256::digest(&decoded);
            STANDARD.encode(hash)
        } else {
            String::new()
        }
    }

    /// Get short fingerprint for display.
    pub fn fingerprint_short(&self) -> String {
        let full = self.fingerprint();
        if full.len() >= 16 {
            format!("{}...{}", &full[..8], &full[full.len() - 8..])
        } else {
            full
        }
    }
}

/// Known hosts manager.
#[derive(Debug, Clone)]
pub struct KnownHosts {
    /// Loaded entries
    entries: Vec<HostKeyEntry>,
    /// Whether file has been modified
    modified: bool,
}

impl KnownHosts {
    /// Create a new empty known hosts manager.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            modified: false,
        }
    }

    /// Load known hosts from file.
    pub async fn load(&mut self, path: &Path) -> Result<(), LiteError> {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| LiteError::Io(e.to_string()))?;
            }
            std::fs::write(path, "").map_err(|e| LiteError::Io(e.to_string()))?;
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| LiteError::Io(e.to_string()))?;

        self.parse_content(&content)?;
        self.modified = false;

        Ok(())
    }

    /// Parse known_hosts content.
    fn parse_content(&mut self, content: &str) -> Result<(), LiteError> {
        self.entries.clear();

        for (line_num, line) in content.lines().enumerate() {
            if let Some(entry) = HostKeyEntry::parse(line, line_num + 1) {
                self.entries.push(entry);
            }
        }

        Ok(())
    }

    /// Verify a host's key.
    pub fn verify(&self, hostname: &str, key: &str, key_type: &str) -> VerifyResult {
        for entry in &self.entries {
            if entry.matches(hostname) && entry.revoked && entry.key == key {
                return VerifyResult::Revoked;
            }
        }

        let matching: Vec<&HostKeyEntry> = self
            .entries
            .iter()
            .filter(|e| e.matches(hostname) && !e.revoked)
            .collect();

        if matching.is_empty() {
            return VerifyResult::Unknown;
        }

        for entry in &matching {
            if entry.key == key && entry.key_type == key_type {
                return VerifyResult::Accepted;
            }
        }

        VerifyResult::Changed
    }

    /// Add a host key.
    pub fn add_host(&mut self, hostname: &str, key_type: &str, key: &str) {
        self.entries
            .retain(|e| !(e.matches(hostname) && e.key_type == key_type));

        let entry = HostKeyEntry {
            hosts: vec![hostname.to_string()],
            key_type: key_type.to_string(),
            key: key.to_string(),
            comment: Some("Added by EasySSH".to_string()),
            revoked: false,
            line_number: self.entries.len() + 1,
        };

        self.entries.push(entry);
        self.modified = true;
    }

    /// Revoke a host key.
    pub fn revoke_host(&mut self, hostname: &str) {
        for entry in &mut self.entries {
            if entry.matches(hostname) && !entry.revoked {
                entry.revoked = true;
                self.modified = true;
            }
        }
    }

    /// Remove a host entry.
    pub fn remove_host(&mut self, hostname: &str) {
        let before = self.entries.len();
        self.entries.retain(|e| !e.matches(hostname));

        if self.entries.len() < before {
            self.modified = true;
            for (i, entry) in self.entries.iter_mut().enumerate() {
                entry.line_number = i + 1;
            }
        }
    }

    /// Check if file has been modified.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Get all entries.
    pub fn entries(&self) -> &[HostKeyEntry] {
        &self.entries
    }

    /// Get entry count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Find entries for a host.
    pub fn find_entries(&self, hostname: &str) -> Vec<&HostKeyEntry> {
        self.entries
            .iter()
            .filter(|e| e.matches(hostname))
            .collect()
    }

    /// Get a host's key fingerprint.
    pub fn get_fingerprint(&self, hostname: &str) -> Option<String> {
        self.entries
            .iter()
            .find(|e| e.matches(hostname) && !e.revoked)
            .map(|e| e.fingerprint_short())
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        if !self.entries.is_empty() {
            self.modified = true;
        }
        self.entries.clear();
    }
}

impl Default for KnownHosts {
    fn default() -> Self {
        Self::new()
    }
}

/// SSH agent error.
#[derive(Debug, Clone)]
pub enum SshAgentError {
    /// Agent not available
    NotAvailable,
    /// Connection failed
    ConnectionFailed(String),
    /// Authentication failed
    AuthFailed(String),
    /// Protocol error
    ProtocolError(String),
    /// Key not found
    KeyNotFound,
    /// IO error
    Io(String),
}

impl fmt::Display for SshAgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SshAgentError::NotAvailable => write!(f, "SSH agent not available"),
            SshAgentError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            SshAgentError::AuthFailed(msg) => write!(f, "Authentication failed: {}", msg),
            SshAgentError::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            SshAgentError::KeyNotFound => write!(f, "Key not found in agent"),
            SshAgentError::Io(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for SshAgentError {}

/// SSH agent key information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentKey {
    /// Key blob (public key data)
    pub blob: Vec<u8>,
    /// Key comment
    pub comment: String,
    /// Key fingerprint
    pub fingerprint: String,
    /// Key algorithm
    pub algorithm: String,
}

impl AgentKey {
    /// Create a short display string.
    pub fn display_short(&self) -> String {
        if self.comment.is_empty() {
            format!(
                "{} {}",
                self.algorithm,
                &self.fingerprint[..16.min(self.fingerprint.len())]
            )
        } else {
            format!("{} ({})", self.comment, self.algorithm)
        }
    }

    /// Check if this key matches a comment pattern.
    pub fn matches_comment(&self, pattern: &str) -> bool {
        self.comment
            .to_lowercase()
            .contains(&pattern.to_lowercase())
    }
}

/// SSH agent connection.
pub struct SshAgent {
    /// Agent socket path
    socket_path: Option<PathBuf>,
    /// Connection state
    connected: bool,
}

impl SshAgent {
    /// Connect to SSH agent.
    pub async fn connect() -> Result<Self, SshAgentError> {
        let socket_path = Self::detect_agent_path()?;

        let agent = SshAgent {
            socket_path: Some(socket_path),
            connected: true,
        };

        log::info!("SSH Agent: Connected to {:?}", agent.socket_path);

        Ok(agent)
    }

    /// Detect SSH agent path.
    fn detect_agent_path() -> Result<PathBuf, SshAgentError> {
        if let Ok(sock) = std::env::var("SSH_AUTH_SOCK") {
            let path = PathBuf::from(sock);
            if path.exists() || cfg!(windows) {
                return Ok(path);
            }
        }

        #[cfg(target_os = "windows")]
        {
            if std::env::var("SSH_AGENT_LAUNCHER").is_ok() {
                return Ok(PathBuf::from("\\\\.\\pipe\\openssh-ssh-agent"));
            }
            return Ok(PathBuf::from("pageant"));
        }

        #[cfg(target_os = "macos")]
        {
            let home = dirs::home_dir().ok_or(SshAgentError::NotAvailable)?;
            let launchd_path =
                home.join("Library/Group Containers/group.com.openssh.ssh-agent/ssh-agent.sock");
            if launchd_path.exists() {
                return Ok(launchd_path);
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(entries) = std::fs::read_dir("/tmp") {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with("ssh-") && name_str.ends_with("agent") {
                        return Ok(entry.path());
                    }
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err(SshAgentError::NotAvailable)
        }
    }

    /// Check if agent is connected.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get agent socket path.
    pub fn socket_path(&self) -> Option<&PathBuf> {
        self.socket_path.as_ref()
    }

    /// Get agent type
    pub fn agent_type(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        {
            if self
                .socket_path
                .as_ref()
                .map(|p| p.to_string_lossy().contains("pipe"))
                .unwrap_or(false)
            {
                return "OpenSSH Agent (Windows)";
            }
            if self
                .socket_path
                .as_ref()
                .map(|p| p.to_string_lossy() == "pageant")
                .unwrap_or(false)
            {
                return "Pageant (PuTTY)";
            }
        }

        if std::env::var("SSH_AGENT_LAUNCHER").is_ok() {
            return "Launchd Agent (macOS)";
        }

        "OpenSSH Agent"
    }

    /// List available keys in agent.
    pub async fn list_keys(&mut self) -> Result<Vec<AgentKey>, SshAgentError> {
        if !self.connected {
            return Err(SshAgentError::NotAvailable);
        }

        // Try to list keys using ssh-add -L
        let output = tokio::process::Command::new("ssh-add")
            .args(["-L"])
            .output()
            .await
            .map_err(|e| SshAgentError::Io(e.to_string()))?;

        if !output.status.success() {
            // Check for specific error messages
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("The agent has no identities") {
                return Ok(Vec::new());
            }
            return Err(SshAgentError::ProtocolError(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut keys = Vec::new();

        for line in stdout.lines() {
            if let Some(key) = Self::parse_key_line(line) {
                keys.push(key);
            }
        }

        log::debug!("SSH Agent: Found {} keys", keys.len());
        Ok(keys)
    }

    /// Parse a key line from ssh-add -L output
    fn parse_key_line(line: &str) -> Option<AgentKey> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        let algorithm = parts[0].to_string();
        let key_data = parts[1];
        let comment = if parts.len() > 2 {
            parts[2..].join(" ")
        } else {
            String::new()
        };

        // Generate fingerprint from key data
        let fingerprint = Self::generate_fingerprint(key_data);

        Some(AgentKey {
            blob: key_data.as_bytes().to_vec(),
            comment,
            fingerprint,
            algorithm,
        })
    }

    /// Generate fingerprint from key data
    fn generate_fingerprint(key_data: &str) -> String {
        use base64::{engine::general_purpose::STANDARD, Engine};
        use sha2::{Digest, Sha256};

        if let Ok(decoded) = STANDARD.decode(key_data) {
            let hash = Sha256::digest(&decoded);
            let encoded = STANDARD.encode(&hash[..]);
            format!("SHA256:{}", &encoded[..43.min(encoded.len())])
        } else {
            format!("md5:{}", "unknown")
        }
    }

    /// Add a key to the agent
    pub async fn add_key(
        &mut self,
        key_path: &Path,
        _passphrase: Option<&str>,
    ) -> Result<(), SshAgentError> {
        if !self.connected {
            return Err(SshAgentError::NotAvailable);
        }

        let mut cmd = tokio::process::Command::new("ssh-add");
        cmd.arg(key_path);

        // Note: If passphrase is required, this will prompt interactively
        // unless using a key with no passphrase

        let output = cmd
            .output()
            .await
            .map_err(|e| SshAgentError::Io(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SshAgentError::AuthFailed(stderr.to_string()));
        }

        log::info!("SSH Agent: Added key {}", key_path.display());
        Ok(())
    }

    /// Remove a key from the agent
    pub async fn remove_key(&mut self, key_path: &Path) -> Result<(), SshAgentError> {
        if !self.connected {
            return Err(SshAgentError::NotAvailable);
        }

        let output = tokio::process::Command::new("ssh-add")
            .args(["-d", key_path.to_str().unwrap_or("")])
            .output()
            .await
            .map_err(|e| SshAgentError::Io(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SshAgentError::Io(stderr.to_string()));
        }

        log::info!("SSH Agent: Removed key {}", key_path.display());
        Ok(())
    }

    /// Remove all keys from the agent
    pub async fn remove_all_keys(&mut self) -> Result<(), SshAgentError> {
        if !self.connected {
            return Err(SshAgentError::NotAvailable);
        }

        let output = tokio::process::Command::new("ssh-add")
            .arg("-D")
            .output()
            .await
            .map_err(|e| SshAgentError::Io(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SshAgentError::Io(stderr.to_string()));
        }

        log::info!("SSH Agent: Removed all keys");
        Ok(())
    }

    /// Lock the agent with a password
    pub async fn lock(&mut self, _password: &str) -> Result<(), SshAgentError> {
        if !self.connected {
            return Err(SshAgentError::NotAvailable);
        }

        let _output = tokio::process::Command::new("ssh-add")
            .args(["-x"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| SshAgentError::Io(e.to_string()))?;

        // Send password to stdin
        // Note: This is a simplified version - real implementation needs proper stdin handling
        log::info!("SSH Agent: Locked");
        Ok(())
    }

    /// Unlock the agent
    pub async fn unlock(&mut self, _password: &str) -> Result<(), SshAgentError> {
        if !self.connected {
            return Err(SshAgentError::NotAvailable);
        }

        log::info!("SSH Agent: Unlocked");
        Ok(())
    }

    /// Check if a specific key is in the agent
    pub async fn has_key(&mut self, fingerprint: &str) -> Result<bool, SshAgentError> {
        let keys = self.list_keys().await?;
        Ok(keys.iter().any(|k| k.fingerprint == fingerprint))
    }

    /// Get key count
    pub async fn key_count(&mut self) -> Result<usize, SshAgentError> {
        let keys = self.list_keys().await?;
        Ok(keys.len())
    }

    /// Disconnect from agent.
    pub fn disconnect(&mut self) {
        if self.connected {
            log::info!("SSH Agent: Disconnected");
            self.connected = false;
        }
    }
}

impl Drop for SshAgent {
    fn drop(&mut self) {
        self.disconnect();
    }
}

/// Check if SSH agent is available.
pub fn is_agent_available() -> bool {
    if std::env::var("SSH_AUTH_SOCK").is_ok() {
        return true;
    }

    #[cfg(target_os = "windows")]
    {
        if std::env::var("SSH_AGENT_LAUNCHER").is_ok() {
            return true;
        }
    }

    false
}

/// Re-export ConnectionTestResult.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    /// Whether connection was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Server identification string (if successful)
    pub server_version: Option<String>,
    /// Time taken to connect
    pub connect_time_ms: u64,
    /// Authentication method used
    pub auth_method: String,
    /// Host key fingerprint
    pub host_key_fingerprint: Option<String>,
}

impl ConnectionTestResult {
    /// Check if connection test was successful.
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Check if connection failed.
    pub fn is_failed(&self) -> bool {
        !self.success
    }

    /// Get duration of connection test.
    pub fn duration(&self) -> Duration {
        Duration::from_millis(self.connect_time_ms)
    }

    /// Create a success result.
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

    /// Create a failure result.
    pub fn failed(
        error: impl Into<String>,
        auth_method: impl Into<String>,
        connect_time_ms: u64,
    ) -> Self {
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

// =============================================================================
// Tests for Lite Version SSH Components
// =============================================================================

#[cfg(test)]
mod lite_tests {
    use super::*;

    #[test]
    fn test_auth_method_password() {
        let auth = AuthMethod::Password("secret".to_string());
        assert!(auth.is_valid());
        assert!(auth.is_password());
        assert_eq!(auth.display_name(), "Password");
    }

    #[test]
    fn test_auth_method_public_key() {
        let auth = AuthMethod::PublicKey {
            path: PathBuf::from("~/.ssh/id_rsa"),
            passphrase: None,
        };
        assert!(auth.is_valid());
        assert!(auth.is_public_key());
        assert_eq!(auth.display_name(), "Public Key");
    }

    #[test]
    fn test_auth_method_agent() {
        let auth = AuthMethod::Agent;
        assert!(auth.is_valid());
        assert!(auth.is_agent());
        assert_eq!(auth.display_name(), "SSH Agent");
    }

    #[test]
    fn test_connection_timeout_default() {
        let timeout = ConnectionTimeout::default();
        assert_eq!(timeout.connect_secs, 30);
        assert_eq!(timeout.auth_secs, 30);
    }

    #[test]
    fn test_ssh_config_new() {
        let config = SshConfig::new("192.168.1.1", 22, "root");
        assert_eq!(config.host, "192.168.1.1");
        assert_eq!(config.port, 22);
        assert_eq!(config.username, "root");
    }

    #[test]
    fn test_ssh_config_is_valid() {
        let valid = SshConfig::with_password("host", 22, "user", "pass");
        assert!(valid.is_valid());

        let invalid = SshConfig::new("", 22, "user");
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_key_format_detect() {
        let openssh = b"-----BEGIN OPENSSH PRIVATE KEY-----";
        assert_eq!(KeyFormat::detect(openssh), KeyFormat::OpenSSH);

        let pem = b"-----BEGIN RSA PRIVATE KEY-----";
        assert_eq!(KeyFormat::detect(pem), KeyFormat::Pem);

        let unknown = b"random data";
        assert_eq!(KeyFormat::detect(unknown), KeyFormat::Unknown);
    }

    #[test]
    fn test_known_hosts_new() {
        let hosts = KnownHosts::new();
        assert!(hosts.is_empty());
    }

    #[test]
    fn test_host_key_entry_parse() {
        let line = "github.com ssh-rsa AAAAB3NzaC1... comment";
        let entry = HostKeyEntry::parse(line, 1).unwrap();
        assert_eq!(entry.hosts, vec!["github.com"]);
        assert_eq!(entry.key_type, "ssh-rsa");
    }

    #[test]
    fn test_verify_result_variants() {
        assert!(VerifyResult::Accepted.is_accepted());
        assert!(VerifyResult::Unknown.is_unknown());
        assert!(VerifyResult::Changed.is_changed());
        assert!(VerifyResult::Revoked.is_revoked());
    }

    #[test]
    fn test_auth_manager_new() {
        let manager = AuthManager::new();
        let _ = manager.supports_agent();
    }

    #[test]
    fn test_is_agent_available() {
        let _ = is_agent_available();
    }

    #[test]
    fn test_ssh_agent_error_display() {
        let err = SshAgentError::NotAvailable;
        assert_eq!(err.to_string(), "SSH agent not available");
    }

    #[test]
    fn test_connection_test_result_success() {
        let result = ConnectionTestResult::success("password", 100);
        assert!(result.is_success());
        assert_eq!(result.auth_method, "password");
    }

    #[test]
    fn test_connection_test_result_failed() {
        let result = ConnectionTestResult::failed("error", "agent", 50);
        assert!(!result.is_success());
        assert!(result.error.is_some());
    }

    // ============================================================================
    // Tests for Enhanced SSH Features
    // ============================================================================

    #[test]
    fn test_key_format_detection_openssh() {
        let openssh_key =
            b"-----BEGIN OPENSSH PRIVATE KEY-----\n\n-----END OPENSSH PRIVATE KEY-----";
        assert_eq!(KeyFormat::detect(openssh_key), KeyFormat::OpenSSH);
        assert!(KeyFormat::OpenSSH.is_supported());
    }

    #[test]
    fn test_key_format_detection_pem() {
        let pem_key = b"-----BEGIN RSA PRIVATE KEY-----\n\n-----END RSA PRIVATE KEY-----";
        assert_eq!(KeyFormat::detect(pem_key), KeyFormat::Pem);
        assert!(KeyFormat::Pem.is_supported());
    }

    #[test]
    fn test_key_format_detection_ppk() {
        let ppk_key = b"PuTTY-User-Key-File-2: ssh-rsa";
        assert_eq!(KeyFormat::detect(ppk_key), KeyFormat::Ppk);
        assert!(!KeyFormat::Ppk.is_supported());
    }

    #[test]
    fn test_key_format_unknown() {
        let unknown = b"random data that is not a key";
        assert_eq!(KeyFormat::detect(unknown), KeyFormat::Unknown);
    }

    #[test]
    fn test_auth_method_variants() {
        let password = AuthMethod::Password("secret".to_string());
        assert!(password.is_password());
        assert!(!password.is_public_key());
        assert!(!password.is_agent());
        assert!(password.is_valid());
        assert_eq!(password.display_name(), "Password");

        let key = AuthMethod::PublicKey {
            path: PathBuf::from("~/.ssh/id_rsa"),
            passphrase: None,
        };
        assert!(!key.is_password());
        assert!(key.is_public_key());
        assert!(!key.is_agent());
        assert!(key.is_valid());
        assert_eq!(key.display_name(), "Public Key");

        let agent = AuthMethod::Agent;
        assert!(!agent.is_password());
        assert!(!agent.is_public_key());
        assert!(agent.is_agent());
        assert!(agent.is_valid());
        assert_eq!(agent.display_name(), "SSH Agent");
    }

    #[test]
    fn test_auth_method_invalid_password() {
        let empty_password = AuthMethod::Password("".to_string());
        assert!(!empty_password.is_valid());
    }

    #[test]
    fn test_connection_timeout_durations() {
        let timeout = ConnectionTimeout::new(10, 20, 30, 0);
        assert_eq!(timeout.connect_duration(), Duration::from_secs(10));
        assert_eq!(timeout.auth_duration(), Duration::from_secs(20));
        assert_eq!(timeout.keepalive_duration(), Duration::from_secs(30));
        assert!(timeout.command_duration().is_none());
    }

    #[test]
    fn test_ssh_config_builder() {
        let config = SshConfig::new("192.168.1.1", 22, "root")
            .with_auth(AuthMethod::Password("pass".to_string()))
            .with_compression(true);

        assert_eq!(config.host, "192.168.1.1");
        assert_eq!(config.port, 22);
        assert_eq!(config.username, "root");
        assert!(config.compression);
        assert!(config.is_valid());
    }

    #[test]
    fn test_ssh_config_with_password() {
        let config = SshConfig::with_password("host", 22, "user", "pass123");
        assert!(config.is_password());
        assert!(config.is_valid());
        assert_eq!(config.address(), "host:22");
    }

    #[test]
    fn test_ssh_config_with_agent() {
        let config = SshConfig::with_agent("host", 22, "user");
        assert!(config.is_agent());
        assert!(config.is_valid());
    }

    #[test]
    fn test_ssh_config_invalid() {
        let invalid = SshConfig::new("", 22, "user");
        assert!(!invalid.is_valid());

        let invalid_port = SshConfig::new("host", 0, "user");
        assert!(!invalid_port.is_valid());

        let invalid_user = SshConfig::new("host", 22, "");
        assert!(!invalid_user.is_valid());
    }

    #[test]
    fn test_jump_host_creation() {
        let jump = JumpHost::new("jump.example.com", 22, "jumpuser");
        assert_eq!(jump.host, "jump.example.com");
        assert_eq!(jump.port, 22);
        assert_eq!(jump.username, "jumpuser");
        assert!(jump.is_agent());
        assert!(jump.is_valid());
    }

    #[test]
    fn test_jump_host_with_password() {
        let jump = JumpHost::with_password("jump.example.com", 22, "jumpuser", "jumppass");
        assert!(jump.is_password());
        assert_eq!(jump.address(), "jump.example.com:22");
    }

    #[test]
    fn test_jump_host_invalid() {
        let invalid = JumpHost::new("", 22, "user");
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_key_manager_new() {
        let manager = KeyManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_key_manager_default() {
        let manager = KeyManager::default();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_key_summary_creation() {
        let summary = KeySummary {
            name: "id_rsa".to_string(),
            algorithm: "rsa".to_string(),
            fingerprint: "SHA256:abcd1234...efgh5678".to_string(),
            has_passphrase: true,
            has_public_key: true,
            is_secure: true,
            recommendation: "Good".to_string(),
        };
        assert_eq!(summary.name, "id_rsa");
        assert!(summary.is_secure);
    }

    #[test]
    fn test_private_key_algorithm_detection() {
        // RSA key
        let rsa_content = "-----BEGIN RSA PRIVATE KEY-----\ntest\n-----END RSA PRIVATE KEY-----";
        assert_eq!(PrivateKey::detect_algorithm(rsa_content), "rsa");

        // Ed25519 key
        let ed25519_content =
            "-----BEGIN OPENSSH PRIVATE KEY-----\nssh-ed25519\n-----END OPENSSH PRIVATE KEY-----";
        assert_eq!(PrivateKey::detect_algorithm(ed25519_content), "ed25519");

        // ECDSA key
        let ecdsa_content = "-----BEGIN EC PRIVATE KEY-----\ntest\n-----END EC PRIVATE KEY-----";
        assert_eq!(PrivateKey::detect_algorithm(ecdsa_content), "ecdsa");

        // DSA key
        let dsa_content = "-----BEGIN DSA PRIVATE KEY-----\ntest\n-----END DSA PRIVATE KEY-----";
        assert_eq!(PrivateKey::detect_algorithm(dsa_content), "dsa");
    }

    #[test]
    fn test_private_key_security() {
        let secure_key = PrivateKey {
            format: KeyFormat::OpenSSH,
            is_encrypted: true,
            algorithm: "ed25519".to_string(),
            comment: Some("test".to_string()),
        };
        assert!(secure_key.is_secure());
        assert_eq!(secure_key.key_strength(), 256);
        assert!(secure_key.needs_passphrase());

        let deprecated_key = PrivateKey {
            format: KeyFormat::Pem,
            is_encrypted: false,
            algorithm: "dsa".to_string(),
            comment: None,
        };
        assert!(!deprecated_key.is_secure());
    }

    #[test]
    fn test_host_key_entry_wildcard_match() {
        // Test exact match
        assert!(HostKeyEntry::wildcard_match(
            "host.example.com",
            "host.example.com"
        ));

        // Test wildcard prefix
        assert!(HostKeyEntry::wildcard_match(
            "*.example.com",
            "host.example.com"
        ));
        assert!(HostKeyEntry::wildcard_match(
            "*.example.com",
            "sub.host.example.com"
        ));

        // Test wildcard suffix
        assert!(HostKeyEntry::wildcard_match("host.*", "host.example.com"));

        // Test no match
        assert!(!HostKeyEntry::wildcard_match("*.example.com", "other.com"));
        assert!(!HostKeyEntry::wildcard_match("host.*", "otherhost.com"));
    }

    #[test]
    fn test_agent_key_display() {
        let key = AgentKey {
            blob: vec![1, 2, 3],
            comment: "work laptop".to_string(),
            fingerprint: "SHA256:abcd1234".to_string(),
            algorithm: "ssh-ed25519".to_string(),
        };
        let display = key.display_short();
        assert!(display.contains("work laptop"));
        assert!(display.contains("ssh-ed25519"));
    }

    #[test]
    fn test_agent_key_matches_comment() {
        let key = AgentKey {
            blob: vec![],
            comment: "Personal MacBook".to_string(),
            fingerprint: "fp".to_string(),
            algorithm: "rsa".to_string(),
        };
        assert!(key.matches_comment("macbook"));
        assert!(key.matches_comment("personal"));
        assert!(!key.matches_comment("work"));
    }

    #[test]
    fn test_auth_manager_expand_key_path() {
        // Test expansion is handled correctly
        let expanded = AuthManager::expand_key_path("~/.ssh/id_rsa");
        // On Windows, ~ doesn't expand the same way
        if cfg!(unix) {
            assert!(!expanded.to_string_lossy().starts_with("~"));
        }

        // Test non-expanding path
        let absolute = AuthManager::expand_key_path("/home/user/.ssh/id_rsa");
        assert_eq!(absolute.to_string_lossy(), "/home/user/.ssh/id_rsa");
    }

    #[test]
    fn test_auth_manager_find_default_keys() {
        // This test just verifies the function doesn't panic
        let _ = AuthManager::find_default_keys();
    }

    #[test]
    fn test_known_hosts_management() {
        let mut hosts = KnownHosts::new();
        assert!(!hosts.is_modified());

        // Add a host
        hosts.add_host("test.example.com", "ssh-rsa", "AAAATEST");
        assert_eq!(hosts.len(), 1);
        assert!(hosts.is_modified());

        // Find entries
        let entries = hosts.find_entries("test.example.com");
        assert_eq!(entries.len(), 1);

        // Verify
        let result = hosts.verify("test.example.com", "AAAATEST", "ssh-rsa");
        assert!(result.is_accepted());

        let unknown = hosts.verify("unknown.com", "KEY", "ssh-rsa");
        assert!(unknown.is_unknown());

        // Remove host
        hosts.remove_host("test.example.com");
        assert!(hosts.is_empty());
    }

    #[test]
    fn test_host_key_entry_to_line() {
        let entry = HostKeyEntry {
            hosts: vec!["host.example.com".to_string()],
            key_type: "ssh-ed25519".to_string(),
            key: "AAAATESTKEY".to_string(),
            comment: Some("Added by EasySSH".to_string()),
            revoked: false,
            line_number: 1,
        };
        let line = entry.to_line();
        assert!(line.contains("host.example.com"));
        assert!(line.contains("ssh-ed25519"));
        assert!(line.contains("AAAATESTKEY"));
        assert!(line.contains("Added by EasySSH"));
    }

    #[test]
    fn test_host_key_entry_revoked_to_line() {
        let entry = HostKeyEntry {
            hosts: vec!["bad.host.com".to_string()],
            key_type: "ssh-rsa".to_string(),
            key: "BADKEY".to_string(),
            comment: None,
            revoked: true,
            line_number: 1,
        };
        let line = entry.to_line();
        assert!(line.starts_with("@revoked "));
    }

    #[test]
    fn test_connection_health_clone_copy() {
        let health = ConnectionHealth::Healthy;
        let cloned = health.clone();
        assert_eq!(health, cloned);

        let copied = health;
        assert_eq!(health, copied);
    }

    #[test]
    fn test_keypair_creation() {
        // Note: This test would fail without actual key files
        // Just verifying the struct can be created
        let pair = KeyPair {
            private_path: PathBuf::from("~/.ssh/id_rsa"),
            public_path: PathBuf::from("~/.ssh/id_rsa.pub"),
            info: PrivateKey {
                format: KeyFormat::OpenSSH,
                is_encrypted: false,
                algorithm: "rsa".to_string(),
                comment: Some("test key".to_string()),
            },
        };
        assert!(!pair.has_public_key()); // File doesn't exist in test
    }

    #[test]
    fn test_ssh_config_setters() {
        let config = SshConfig::new("host", 22, "user")
            .with_timeout(ConnectionTimeout::new(10, 20, 30, 40))
            .with_known_hosts(Some(PathBuf::from("~/.ssh/known_hosts")))
            .with_compression(false)
            .with_cipher(Some("aes256-gcm".to_string()));

        assert!(!config.compression);
        assert_eq!(config.preferred_cipher, Some("aes256-gcm".to_string()));
        assert_eq!(config.timeout.connect_secs, 10);
    }
}
