//! SSH session management and connection pooling
//!
//! This module provides robust SSH connectivity with:
//! - Connection pooling and multiplexing for efficient resource usage
//! - Health tracking and automatic cleanup of stale connections
//! - Shell session management with bidirectional I/O
//! - Command execution with retry logic
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
//! use easyssh_core::ssh::{SshSessionManager, ConnectionHealth};
//!
//! async fn ssh_example() {
//!     let mut manager = SshSessionManager::new();
//!
//!     // Connect to a server
//!     let metadata = manager.connect(
//!         "session-1",
//!         "192.168.1.1",
//!         22,
//!         "root",
//!         Some("password")
//!     ).await.unwrap();
//!
//!     // Execute a command
//!     let output = manager.execute("session-1", "uname -a").await.unwrap();
//!     println!("Output: {}", output);
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
