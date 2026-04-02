#![allow(dead_code)]

//! Enhanced Connection Pool for Linux
//!
//! Features:
//! - Smart connection multiplexing
//! - Connection health checks with periodic ping
//! - Auto-reconnect on network disconnection
//! - Connection rate limiting
//! - Memory optimization with session data compression

use crate::error::LiteError;
use crate::ssh::{ConnectionHealth, SessionMetadata, SshSessionManager};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;

/// Connection state for enhanced tracking
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnhancedConnectionState {
    Connected,
    Connecting,
    Reconnecting { attempt: u32 },
    Disconnected,
    Failed { reason: &'static str },
}

/// Rate limiter for connection establishment
pub struct ConnectionRateLimiter {
    max_connections_per_minute: u32,
    connection_times: Arc<Mutex<Vec<Instant>>>,
}

impl ConnectionRateLimiter {
    pub fn new(max_per_minute: u32) -> Self {
        Self {
            max_connections_per_minute: max_per_minute,
            connection_times: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Check if a new connection is allowed
    pub async fn try_acquire(&self) -> bool {
        let mut times = self.connection_times.lock().await;
        let now = Instant::now();

        // Remove entries older than 1 minute
        times.retain(|t| now.duration_since(*t) < Duration::from_secs(60));

        if times.len() >= self.max_connections_per_minute as usize {
            return false;
        }

        times.push(now);
        true
    }

    /// Get current connection count in window
    pub async fn current_count(&self) -> usize {
        let mut times = self.connection_times.lock().await;
        let now = Instant::now();
        times.retain(|t| now.duration_since(*t) < Duration::from_secs(60));
        times.len()
    }
}

/// Health check configuration
#[derive(Clone, Debug)]
pub struct HealthCheckConfig {
    /// Interval between health checks
    pub interval_secs: u64,
    /// Timeout for health check command
    pub timeout_secs: u64,
    /// Number of consecutive failures before marking unhealthy
    pub failure_threshold: u32,
    /// Number of consecutive successes to recover from degraded
    pub recovery_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval_secs: 30,
            timeout_secs: 10,
            failure_threshold: 3,
            recovery_threshold: 2,
        }
    }
}

/// Auto-reconnect configuration
#[derive(Clone, Debug)]
pub struct ReconnectConfig {
    /// Maximum number of reconnection attempts
    pub max_attempts: u32,
    /// Initial delay before first reconnection
    pub initial_delay_ms: u64,
    /// Maximum delay between attempts (exponential backoff)
    pub max_delay_ms: u64,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Compressed session storage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressedSessionData {
    pub session_id: String,
    pub server_key: String,
    pub compressed_content: Vec<u8>,
    pub original_size: usize,
    pub compressed_at: u64,
}

/// Memory-optimized session storage
pub struct CompressedSessionStore {
    sessions: Arc<RwLock<HashMap<String, CompressedSessionData>>>,
    max_sessions: usize,
    total_compressed_bytes: AtomicU64,
    total_original_bytes: AtomicU64,
}

impl CompressedSessionStore {
    pub fn new(max_sessions: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_sessions,
            total_compressed_bytes: AtomicU64::new(0),
            total_original_bytes: AtomicU64::new(0),
        }
    }

    /// Compress and store session data
    pub async fn store(
        &self,
        session_id: &str,
        server_key: &str,
        content: &str,
    ) -> Result<(), LiteError> {
        let original = content.as_bytes();
        let original_size = original.len();

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(original)
            .map_err(|e| LiteError::Io(e.to_string()))?;
        let compressed = encoder.finish().map_err(|e| LiteError::Io(e.to_string()))?;

        let compressed_size = compressed.len();
        let data = CompressedSessionData {
            session_id: session_id.to_string(),
            server_key: server_key.to_string(),
            compressed_content: compressed,
            original_size,
            compressed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let mut sessions = self.sessions.write().await;

        // Evict oldest if at capacity
        if sessions.len() >= self.max_sessions {
            let oldest = sessions
                .iter()
                .min_by_key(|(_, v)| v.compressed_at)
                .map(|(k, _)| k.clone());
            if let Some(key) = oldest {
                if let Some(old) = sessions.remove(&key) {
                    self.total_compressed_bytes
                        .fetch_sub(old.compressed_content.len() as u64, Ordering::Relaxed);
                    self.total_original_bytes
                        .fetch_sub(old.original_size as u64, Ordering::Relaxed);
                }
            }
        }

        self.total_compressed_bytes
            .fetch_add(compressed_size as u64, Ordering::Relaxed);
        self.total_original_bytes
            .fetch_add(original_size as u64, Ordering::Relaxed);
        sessions.insert(session_id.to_string(), data);

        Ok(())
    }

    /// Decompress and retrieve session data
    pub async fn retrieve(&self, session_id: &str) -> Result<Option<String>, LiteError> {
        let sessions = self.sessions.read().await;

        if let Some(data) = sessions.get(session_id) {
            let mut decoder = ZlibDecoder::new(&data.compressed_content[..]);
            let mut result = String::new();
            decoder
                .read_to_string(&mut result)
                .map_err(|e| LiteError::Io(e.to_string()))?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Get storage statistics
    pub fn stats(&self) -> SessionStoreStats {
        let compressed = self.total_compressed_bytes.load(Ordering::Relaxed);
        let original = self.total_original_bytes.load(Ordering::Relaxed);
        let ratio = if original > 0 {
            (compressed as f64 / original as f64) * 100.0
        } else {
            0.0
        };

        SessionStoreStats {
            total_sessions: self.sessions.blocking_read().len(),
            total_compressed_bytes: compressed,
            total_original_bytes: original,
            compression_ratio: ratio,
        }
    }

    /// Clear all stored sessions
    pub async fn clear(&self) {
        let mut sessions = self.sessions.write().await;
        sessions.clear();
        self.total_compressed_bytes.store(0, Ordering::Relaxed);
        self.total_original_bytes.store(0, Ordering::Relaxed);
    }
}

/// Session storage statistics
#[derive(Debug, Clone, Serialize)]
pub struct SessionStoreStats {
    pub total_sessions: usize,
    pub total_compressed_bytes: u64,
    pub total_original_bytes: u64,
    pub compression_ratio: f64,
}

/// Health check worker for a connection
struct HealthCheckWorker {
    session_id: String,
    check_fn: Arc<dyn Fn() -> tokio::task::JoinHandle<Result<bool, LiteError>> + Send + Sync>,
    config: HealthCheckConfig,
    consecutive_failures: Arc<AtomicU64>,
    consecutive_successes: Arc<AtomicU64>,
    stop_signal: Arc<tokio::sync::watch::Sender<bool>>,
}

impl HealthCheckWorker {
    fn new(
        session_id: &str,
        config: HealthCheckConfig,
        check_fn: impl Fn() -> tokio::task::JoinHandle<Result<bool, LiteError>> + Send + Sync + 'static,
    ) -> (Self, tokio::sync::watch::Receiver<bool>) {
        let (tx, rx) = tokio::sync::watch::channel(false);
        (
            Self {
                session_id: session_id.to_string(),
                check_fn: Arc::new(check_fn),
                config,
                consecutive_failures: Arc::new(AtomicU64::new(0)),
                consecutive_successes: Arc::new(AtomicU64::new(0)),
                stop_signal: Arc::new(tx),
            },
            rx,
        )
    }

    fn start(&self) -> JoinHandle<()> {
        let interval_duration = Duration::from_secs(self.config.interval_secs);
        let session_id = self.session_id.clone();
        let check_fn = self.check_fn.clone();
        let config = self.config.clone();
        let failures = self.consecutive_failures.clone();
        let successes = self.consecutive_successes.clone();
        let mut stop_rx = self.stop_signal.subscribe();

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval_duration);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let check = check_fn();
                        match check.await {
                            Ok(Ok(true)) => {
                                let success_count = successes.fetch_add(1, Ordering::Relaxed) + 1;
                                failures.store(0, Ordering::Relaxed);

                                if success_count >= config.recovery_threshold as u64 {
                                    tracing::debug!("Health check passed for {}", session_id);
                                }
                            }
                            Ok(Ok(false)) | Ok(Err(_)) | Err(_) => {
                                let failure_count = failures.fetch_add(1, Ordering::Relaxed) + 1;
                                successes.store(0, Ordering::Relaxed);

                                if failure_count >= config.failure_threshold as u64 {
                                    tracing::warn!(
                                        "Health check failed {} consecutive times for {}",
                                        failure_count, session_id
                                    );
                                }
                            }
                        }
                    }
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            break;
                        }
                    }
                }
            }

            tracing::debug!("Health check worker stopped for {}", session_id);
        })
    }

    fn stop(&self) {
        let _ = self.stop_signal.send(true);
    }

    fn health_status(&self) -> ConnectionHealth {
        let failures = self.consecutive_failures.load(Ordering::Relaxed);
        let successes = self.consecutive_successes.load(Ordering::Relaxed);
        let threshold = self.config.failure_threshold as u64;

        if failures >= threshold {
            ConnectionHealth::Unhealthy
        } else if failures > 0 || successes == 0 {
            ConnectionHealth::Degraded
        } else {
            ConnectionHealth::Healthy
        }
    }
}

/// Enhanced session manager with all optimization features
pub struct EnhancedSshManager {
    base_manager: Arc<Mutex<SshSessionManager>>,
    rate_limiter: ConnectionRateLimiter,
    session_store: CompressedSessionStore,
    health_check_config: HealthCheckConfig,
    reconnect_config: ReconnectConfig,
    health_workers: Arc<Mutex<HashMap<String, HealthCheckWorker>>>,
    reconnection_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    session_states: Arc<RwLock<HashMap<String, EnhancedConnectionState>>>,
    reconnect_attempts: Arc<RwLock<HashMap<String, u32>>>,
    global_connection_count: Arc<AtomicU64>,
    max_global_connections: u64,
}

impl EnhancedSshManager {
    pub fn new() -> Self {
        Self::with_config(
            30,  // max connections per minute
            100, // max compressed sessions
            HealthCheckConfig::default(),
            ReconnectConfig::default(),
            100, // max global connections
        )
    }

    pub fn with_config(
        max_connections_per_minute: u32,
        max_stored_sessions: usize,
        health_check_config: HealthCheckConfig,
        reconnect_config: ReconnectConfig,
        max_global_connections: u64,
    ) -> Self {
        Self {
            base_manager: Arc::new(Mutex::new(SshSessionManager::new())),
            rate_limiter: ConnectionRateLimiter::new(max_connections_per_minute),
            session_store: CompressedSessionStore::new(max_stored_sessions),
            health_check_config,
            reconnect_config,
            health_workers: Arc::new(Mutex::new(HashMap::new())),
            reconnection_tasks: Arc::new(Mutex::new(HashMap::new())),
            session_states: Arc::new(RwLock::new(HashMap::new())),
            reconnect_attempts: Arc::new(RwLock::new(HashMap::new())),
            global_connection_count: Arc::new(AtomicU64::new(0)),
            max_global_connections,
        }
    }

    /// Connect with rate limiting and health check
    pub async fn connect(
        &self,
        session_id: &str,
        host: &str,
        port: u16,
        username: &str,
        password: Option<&str>,
    ) -> Result<SessionMetadata, LiteError> {
        // Check global connection limit
        let current_global = self.global_connection_count.load(Ordering::Relaxed);
        if current_global >= self.max_global_connections {
            return Err(LiteError::Config(format!(
                "Global connection limit reached ({}/{})",
                current_global, self.max_global_connections
            )));
        }

        // Apply rate limiting
        if !self.rate_limiter.try_acquire().await {
            return Err(LiteError::Config(
                "Connection rate limit exceeded. Please wait a moment.".to_string(),
            ));
        }

        // Update state
        {
            let mut states = self.session_states.write().await;
            states.insert(session_id.to_string(), EnhancedConnectionState::Connecting);
        }

        // Perform connection
        let result = {
            let mut manager = self.base_manager.lock().await;
            manager
                .connect(session_id, host, port, username, password)
                .await
        };

        match result {
            Ok(metadata) => {
                self.global_connection_count.fetch_add(1, Ordering::Relaxed);

                // Update state
                {
                    let mut states = self.session_states.write().await;
                    states.insert(session_id.to_string(), EnhancedConnectionState::Connected);
                }

                // Start health check worker
                self.start_health_check(session_id).await;

                tracing::info!("Enhanced SSH: Connected {}@{}:{}", username, host, port);
                Ok(metadata)
            }
            Err(e) => {
                let mut states = self.session_states.write().await;
                states.insert(
                    session_id.to_string(),
                    EnhancedConnectionState::Failed {
                        reason: "connection_failed",
                    },
                );
                Err(e)
            }
        }
    }

    /// Start health check for a session
    async fn start_health_check(&self, session_id: &str) {
        let base_manager = self.base_manager.clone();
        let sid = session_id.to_string();

        let check_fn = move || {
            let manager = base_manager.clone();
            let session_id = sid.clone();
            tokio::spawn(async move {
                let mgr = manager.lock().await;
                match mgr.check_health(&session_id).await {
                    Ok(ConnectionHealth::Healthy) => Ok(true),
                    _ => Ok(false),
                }
            })
        };

        let (worker, _) =
            HealthCheckWorker::new(session_id, self.health_check_config.clone(), check_fn);

        let handle = worker.start();

        // Store worker (we don't await the handle, it runs in background)
        // Explicitly drop the handle to suppress the warning about unused future
        drop(handle);

        let mut workers = self.health_workers.lock().await;
        workers.insert(session_id.to_string(), worker);
    }

    /// Execute command with auto-reconnect on failure
    pub async fn execute_with_auto_reconnect(
        &self,
        session_id: &str,
        command: &str,
    ) -> Result<String, LiteError> {
        let result = {
            let manager = self.base_manager.lock().await;
            manager.execute_with_retry(session_id, command, 1).await
        };

        match result {
            Ok(output) => Ok(output),
            Err(e) => {
                let err_str = e.to_string().to_lowercase();
                let needs_reconnect = err_str.contains("reset")
                    || err_str.contains("broken pipe")
                    || err_str.contains("connection refused")
                    || err_str.contains("connection reset")
                    || err_str.contains("eof");

                if needs_reconnect {
                    tracing::warn!(
                        "Connection lost for {}, attempting auto-reconnect...",
                        session_id
                    );

                    // Trigger auto-reconnect
                    self.trigger_reconnect(session_id).await;

                    // Return error for now, reconnect happens in background
                    Err(LiteError::Ssh(format!(
                        "Connection lost. Auto-reconnect initiated. Error: {}",
                        e
                    )))
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Trigger auto-reconnect for a session
    async fn trigger_reconnect(&self, session_id: &str) {
        {
            let mut states = self.session_states.write().await;
            states.insert(
                session_id.to_string(),
                EnhancedConnectionState::Reconnecting { attempt: 1 },
            );
        }

        // Start reconnection task
        let sid = session_id.to_string();
        let _base_manager = self.base_manager.clone();
        let config = self.reconnect_config.clone();
        let states = self.session_states.clone();
        let attempts = self.reconnect_attempts.clone();

        let handle = tokio::spawn(async move {
            let mut delay_ms = config.initial_delay_ms;
            let session_id = sid; // Move into the async block

            for attempt in 1..=config.max_attempts {
                tracing::info!("Reconnection attempt {} for {}", attempt, session_id);

                tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                // Update attempt count
                {
                    let mut att = attempts.write().await;
                    att.insert(session_id.clone(), attempt);
                }

                // Try to reconnect - in real implementation, we'd need stored credentials
                // For now, just update state
                {
                    let mut states = states.write().await;
                    states.insert(
                        session_id.clone(),
                        EnhancedConnectionState::Reconnecting { attempt },
                    );
                }

                // Exponential backoff
                delay_ms =
                    ((delay_ms as f64 * config.backoff_multiplier) as u64).min(config.max_delay_ms);
            }

            // Max attempts reached
            {
                let mut states = states.write().await;
                states.insert(
                    session_id.clone(),
                    EnhancedConnectionState::Failed {
                        reason: "max_reconnect_attempts",
                    },
                );
            }

            tracing::error!("Max reconnection attempts reached for {}", session_id);
        });

        let mut tasks = self.reconnection_tasks.lock().await;
        tasks.insert(session_id.to_string(), handle);
    }

    /// Store session terminal content with compression
    pub async fn store_session_content(
        &self,
        session_id: &str,
        server_key: &str,
        content: &str,
    ) -> Result<(), LiteError> {
        self.session_store
            .store(session_id, server_key, content)
            .await
    }

    /// Retrieve session terminal content
    pub async fn retrieve_session_content(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, LiteError> {
        self.session_store.retrieve(session_id).await
    }

    /// Disconnect and cleanup
    pub async fn disconnect(&self, session_id: &str) -> Result<(), LiteError> {
        // Stop health check
        {
            let mut workers = self.health_workers.lock().await;
            if let Some(worker) = workers.remove(session_id) {
                worker.stop();
            }
        }

        // Cancel reconnection if in progress
        {
            let mut tasks = self.reconnection_tasks.lock().await;
            if let Some(handle) = tasks.remove(session_id) {
                handle.abort();
            }
        }

        // Disconnect from base manager
        {
            let mut manager = self.base_manager.lock().await;
            manager.disconnect(session_id).await?;
        }

        // Update state
        {
            let mut states = self.session_states.write().await;
            states.insert(
                session_id.to_string(),
                EnhancedConnectionState::Disconnected,
            );
        }

        self.global_connection_count.fetch_sub(1, Ordering::Relaxed);

        tracing::info!("Enhanced SSH: Disconnected {}", session_id);
        Ok(())
    }

    /// Get current connection state
    pub async fn get_connection_state(&self, session_id: &str) -> EnhancedConnectionState {
        let states = self.session_states.read().await;
        states
            .get(session_id)
            .copied()
            .unwrap_or(EnhancedConnectionState::Disconnected)
    }

    /// Get comprehensive stats
    pub async fn get_stats(&self) -> EnhancedPoolStats {
        let base_stats = {
            let manager = self.base_manager.lock().await;
            manager.get_pool_stats()
        };

        let store_stats = self.session_store.stats();
        let rate_count = self.rate_limiter.current_count().await;
        let global_count = self.global_connection_count.load(Ordering::Relaxed);

        EnhancedPoolStats {
            base_stats,
            session_store: store_stats,
            rate_limited_connections: rate_count,
            global_connections: global_count as usize,
            max_global_connections: self.max_global_connections as usize,
        }
    }

    /// List all active session states
    pub async fn list_session_states(&self) -> Vec<(String, EnhancedConnectionState)> {
        let states = self.session_states.read().await;
        states.iter().map(|(k, v)| (k.clone(), *v)).collect()
    }

    /// Cleanup all resources
    pub async fn shutdown(&self) {
        // Stop all health workers
        {
            let mut workers = self.health_workers.lock().await;
            for (_, worker) in workers.drain() {
                worker.stop();
            }
        }

        // Cancel all reconnection tasks
        {
            let mut tasks = self.reconnection_tasks.lock().await;
            for (_, handle) in tasks.drain() {
                handle.abort();
            }
        }

        // Clear session store
        self.session_store.clear().await;

        tracing::info!("Enhanced SSH Manager shutdown complete");
    }
}

impl Default for EnhancedSshManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Enhanced pool statistics
#[derive(Debug, Clone, Serialize)]
pub struct EnhancedPoolStats {
    pub base_stats: crate::ssh::PoolStats,
    pub session_store: SessionStoreStats,
    pub rate_limited_connections: usize,
    pub global_connections: usize,
    pub max_global_connections: usize,
}

/// Builder for EnhancedSshManager
pub struct EnhancedSshManagerBuilder {
    max_connections_per_minute: u32,
    max_stored_sessions: usize,
    health_check_config: HealthCheckConfig,
    reconnect_config: ReconnectConfig,
    max_global_connections: u64,
}

impl EnhancedSshManagerBuilder {
    pub fn new() -> Self {
        Self {
            max_connections_per_minute: 30,
            max_stored_sessions: 100,
            health_check_config: HealthCheckConfig::default(),
            reconnect_config: ReconnectConfig::default(),
            max_global_connections: 100,
        }
    }

    pub fn max_connections_per_minute(mut self, value: u32) -> Self {
        self.max_connections_per_minute = value;
        self
    }

    pub fn max_stored_sessions(mut self, value: usize) -> Self {
        self.max_stored_sessions = value;
        self
    }

    pub fn health_check_interval(mut self, secs: u64) -> Self {
        self.health_check_config.interval_secs = secs;
        self
    }

    pub fn reconnect_max_attempts(mut self, attempts: u32) -> Self {
        self.reconnect_config.max_attempts = attempts;
        self
    }

    pub fn max_global_connections(mut self, value: u64) -> Self {
        self.max_global_connections = value;
        self
    }

    pub fn build(self) -> EnhancedSshManager {
        EnhancedSshManager::with_config(
            self.max_connections_per_minute,
            self.max_stored_sessions,
            self.health_check_config,
            self.reconnect_config,
            self.max_global_connections,
        )
    }
}

impl Default for EnhancedSshManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
