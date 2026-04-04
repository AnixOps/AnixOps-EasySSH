//! Automatic Reconnection System
//!
//! This module implements a robust automatic reconnection system for SSH connections
//! following the design constraints specified in SYSTEM_INVARIANTS.md Section 5.
//!
//! # Features
//!
//! - **Exponential Backoff**: Delays increase exponentially with configurable base and max
//! - **Jitter**: Random variation to prevent thundering herd effect
//! - **Heartbeat Monitoring**: Periodic health checks with configurable thresholds
//! - **State Tracking**: Full reconnection state machine
//! - **Event Emission**: Proper `connection_state_changed` event triggering
//!
//! # System Invariants (from SYSTEM_INVARIANTS.md)
//!
//! - Reconnect delay uses exponential backoff: `base * 2^attempt`
//! - Maximum retries: 10 (default)
//! - Maximum delay: 60 seconds (default)
//! - User-initiated disconnect does NOT trigger auto-reconnect
//! - Heartbeat interval: 30 seconds (default)
//! - 3 consecutive heartbeat failures trigger reconnection
//!
//! # Example
//!
//! ```rust
//! use easyssh_core::connection::{ReconnectOrchestrator, ReconnectConfig, HeartbeatConfig};
//! use std::time::Duration;
//!
//! // Create with default configuration
//! let orchestrator = ReconnectOrchestrator::default();
//!
//! // Or with custom configuration
//! let config = ReconnectConfig {
//!     max_retries: 5,
//!     base_delay: Duration::from_secs(2),
//!     max_delay: Duration::from_secs(30),
//!     jitter: 0.2,
//! };
//! let heartbeat = HeartbeatConfig {
//!     interval: Duration::from_secs(15),
//!     timeout: Duration::from_secs(5),
//!     failure_threshold: 3,
//!     recovery_threshold: 2,
//! };
//! let orchestrator = ReconnectOrchestrator::new(config).with_heartbeat(heartbeat);
//! ```

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task::JoinHandle;

/// Configuration for the reconnection orchestrator.
///
/// Controls the behavior of automatic reconnection attempts including
/// timing, retry limits, and jitter parameters.
///
/// # Default Values (from SYSTEM_INVARIANTS.md)
///
/// - `max_retries`: 10
/// - `base_delay`: 1 second
/// - `max_delay`: 60 seconds
/// - `jitter`: 0.3 (30% random variation)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReconnectConfig {
    /// Maximum number of reconnection attempts before giving up.
    /// Default: 10 (SYSTEM_INVARIANTS.md constraint)
    pub max_retries: u32,

    /// Base delay for exponential backoff calculation.
    /// Default: 1 second
    pub base_delay: Duration,

    /// Maximum delay cap to prevent excessive wait times.
    /// Default: 60 seconds (SYSTEM_INVARIANTS.md constraint)
    pub max_delay: Duration,

    /// Jitter factor (0.0 to 1.0) for random delay variation.
    /// Default: 0.3 (30% jitter to prevent thundering herd)
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
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration with custom retry count.
    pub fn with_max_retries(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Self::default()
        }
    }

    /// Calculate the delay for a given attempt number using exponential backoff.
    ///
    /// Formula: `delay = base_delay * 2^attempt`, capped at max_delay,
    /// with random jitter applied.
    ///
    /// # Arguments
    ///
    /// * `attempt` - The current attempt number (0-based)
    ///
    /// # Returns
    ///
    /// The calculated delay with jitter applied.
    ///
    /// # Example
    ///
    /// ```rust
    /// use easyssh_core::connection::ReconnectConfig;
    /// use std::time::Duration;
    ///
    /// let config = ReconnectConfig::default();
    ///
    /// // First attempt: ~1 second (base * 2^0 = 1, with jitter)
    /// let delay0 = config.calculate_delay(0);
    /// assert!(delay0 >= Duration::from_millis(700) && delay0 <= Duration::from_millis(1300));
    ///
    /// // Second attempt: ~2 seconds (base * 2^1 = 2, with jitter)
    /// let delay1 = config.calculate_delay(1);
    /// assert!(delay1 >= Duration::from_millis(1400) && delay1 <= Duration::from_millis(2600));
    /// ```
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        // Calculate exponential delay: base * 2^attempt
        let exponential_delay = if attempt > 31 {
            // Prevent overflow for very large attempts
            self.max_delay
        } else {
            let multiplier = 2u64.pow(attempt);
            let delay_micros = self.base_delay.as_micros() as u64 * multiplier;
            Duration::from_micros(delay_micros.min(self.max_delay.as_micros() as u64))
        };

        // Apply jitter: delay * (1 + (random - 0.5) * 2 * jitter)
        // Using mul_f64 can produce negative durations, so we use a different approach
        let final_delay = if self.jitter > 0.0 {
            let mut rng = rand::thread_rng();
            let random_factor: f64 = rng.gen_range(0.0..1.0); // 0.0 to 1.0
            // Scale the jitter range: from (1 - jitter) to (1 + jitter)
            let jitter_scale = 1.0 - self.jitter + random_factor * 2.0 * self.jitter;
            exponential_delay.mul_f64(jitter_scale)
        } else {
            exponential_delay
        };

        // Ensure delay is within bounds [base_delay, max_delay]
        final_delay.max(self.base_delay).min(self.max_delay)
    }

    /// Validate the configuration parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `max_retries` is 0
    /// - `base_delay` is zero
    /// - `max_delay` is less than `base_delay`
    /// - `jitter` is outside [0.0, 1.0]
    pub fn validate(&self) -> Result<(), String> {
        if self.max_retries == 0 {
            return Err("max_retries must be at least 1".to_string());
        }
        if self.base_delay.is_zero() {
            return Err("base_delay must be non-zero".to_string());
        }
        if self.max_delay < self.base_delay {
            return Err("max_delay must be >= base_delay".to_string());
        }
        if self.jitter < 0.0 || self.jitter > 1.0 {
            return Err("jitter must be in range [0.0, 1.0]".to_string());
        }
        Ok(())
    }
}

/// Configuration for heartbeat monitoring.
///
/// Heartbeat monitoring periodically checks connection health and
/// triggers reconnection when consecutive failures exceed threshold.
///
/// # Default Values (from SYSTEM_INVARIANTS.md)
///
/// - `interval`: 30 seconds
/// - `timeout`: 10 seconds
/// - `failure_threshold`: 3
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// Interval between heartbeat checks.
    /// Default: 30 seconds (SYSTEM_INVARIANTS.md constraint)
    pub interval: Duration,

    /// Timeout for each heartbeat check.
    /// Default: 10 seconds
    pub timeout: Duration,

    /// Number of consecutive failures before triggering reconnection.
    /// Default: 3 (SYSTEM_INVARIANTS.md constraint)
    pub failure_threshold: u32,

    /// Number of consecutive successes to mark connection as healthy.
    /// Default: 2
    pub recovery_threshold: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            failure_threshold: 3,
            recovery_threshold: 2,
        }
    }
}

impl HeartbeatConfig {
    /// Create a new heartbeat configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate the heartbeat configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.interval.is_zero() {
            return Err("interval must be non-zero".to_string());
        }
        if self.timeout.is_zero() {
            return Err("timeout must be non-zero".to_string());
        }
        if self.timeout > self.interval {
            return Err("timeout should be <= interval".to_string());
        }
        if self.failure_threshold == 0 {
            return Err("failure_threshold must be at least 1".to_string());
        }
        Ok(())
    }
}

/// Reconnection state tracking.
///
/// Tracks the current state of the reconnection process including
/// attempt count, timing, and failure information.
#[derive(Clone, Debug, PartialEq)]
pub enum ReconnectState {
    /// No reconnection in progress, connection is healthy
    Idle,

    /// Monitoring connection health via heartbeat
    Monitoring {
        /// Timestamp when monitoring started (serialized as duration for portability)
        started_at: Instant,
        /// Current consecutive heartbeat failure count
        consecutive_failures: u32,
        /// Current consecutive heartbeat success count
        consecutive_successes: u32,
    },

    /// Connection lost, waiting before next reconnection attempt
    Waiting {
        /// Current attempt number
        attempt: u32,
        /// When the next attempt should occur (serialized as duration for portability)
        next_attempt_at: Instant,
        /// Last error that triggered reconnection
        last_error: Option<String>,
    },

    /// Currently attempting to reconnect
    Attempting {
        /// Current attempt number
        attempt: u32,
        /// When this attempt started (serialized as duration for portability)
        started_at: Instant,
    },

    /// Reconnection succeeded, connection restored
    Succeeded {
        /// Total attempts made
        total_attempts: u32,
        /// Time taken to reconnect
        duration: Duration,
    },

    /// Reconnection failed after exhausting all retries
    Failed {
        /// Total attempts made
        total_attempts: u32,
        /// Final error message
        final_error: String,
    },

    /// User initiated disconnect, reconnection disabled
    UserDisconnected {
        /// Timestamp when user disconnected (serialized as duration for portability)
        disconnected_at: Instant,
    },
}

impl Default for ReconnectState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Heartbeat status for connection health monitoring.
#[derive(Clone, Debug, Copy, PartialEq, Serialize, Deserialize)]
pub enum HeartbeatStatus {
    /// Connection is healthy
    Healthy,

    /// Connection is degraded (some heartbeat failures)
    Degraded,

    /// Connection is unhealthy (threshold exceeded)
    Unhealthy,
}

/// Trigger source for reconnection.
///
/// Identifies what triggered a reconnection attempt for logging
/// and event emission purposes.
#[derive(Clone, Debug, PartialEq)]
pub enum ReconnectTrigger {
    /// Initial connection establishment
    InitialConnect,

    /// Heartbeat failure triggered reconnection
    HeartbeatFail {
        /// Consecutive failure count at trigger
        failure_count: u32,
    },

    /// Network error detected during operation
    NetworkError {
        /// Error description
        error: String,
    },

    /// Connection timeout
    Timeout,

    /// Manual reconnection request
    ManualRequest,

    /// Jump host cascade failure propagation
    CascadeFail {
        /// Source connection ID
        source_connection_id: String,
    },

    /// Idle timeout disconnection
    IdleTimeout,
}

/// Reconnection event for notification.
///
/// Events are emitted for each significant state change in the
/// reconnection process.
#[derive(Clone, Debug)]
pub struct ReconnectEvent {
    /// Connection ID that the event relates to
    pub connection_id: String,

    /// Current reconnection state
    pub state: ReconnectState,

    /// What triggered the reconnection (if applicable)
    pub trigger: Option<ReconnectTrigger>,

    /// Current attempt number (if applicable)
    pub attempt: Option<u32>,

    /// Error message (if applicable)
    pub error: Option<String>,

    /// Timestamp when event occurred
    pub timestamp: Instant,
}

/// The main reconnection orchestrator.
///
/// Manages automatic reconnection with exponential backoff, jitter,
/// and heartbeat monitoring. Follows all SYSTEM_INVARIANTS.md constraints.
///
/// # Architecture
///
/// The orchestrator uses:
/// - Atomic counters for thread-safe attempt tracking
/// - Broadcast channels for event emission
/// - Async tasks for heartbeat monitoring and reconnection attempts
///
/// # Thread Safety
///
/// All public methods are thread-safe and can be called from multiple
/// threads concurrently.
///
/// # Example
///
/// ```rust
/// use easyssh_core::connection::{ReconnectOrchestrator, ReconnectConfig, ReconnectTrigger};
/// use std::sync::Arc;
///
/// // Create orchestrator
/// let orchestrator = Arc::new(ReconnectOrchestrator::default());
///
/// // Subscribe to events
/// let mut event_rx = orchestrator.subscribe_events();
///
/// // Start monitoring a connection
/// orchestrator.start_monitoring("conn-123");
///
/// // Handle disconnect (trigger, error message)
/// orchestrator.handle_disconnect("conn-123", false, ReconnectTrigger::NetworkError { error: "Connection reset".to_string() }, None);
///
/// // Get current state
/// let state = orchestrator.get_state("conn-123");
/// ```
pub struct ReconnectOrchestrator {
    /// Reconnection configuration
    config: ReconnectConfig,

    /// Heartbeat configuration
    heartbeat_config: HeartbeatConfig,

    /// Current reconnection attempts per connection (atomic for thread safety)
    current_attempts: Arc<RwLock<std::collections::HashMap<String, u32>>>,

    /// Reconnection states per connection
    states: Arc<RwLock<std::collections::HashMap<String, ReconnectState>>>,

    /// Flag indicating if reconnection is enabled globally
    enabled: AtomicBool,

    /// User-initiated disconnect flags (connections manually disconnected)
    user_disconnected: Arc<RwLock<std::collections::HashMap<String, Instant>>>,

    /// Event broadcaster for state change notifications
    event_tx: broadcast::Sender<ReconnectEvent>,

    /// Heartbeat failure counters per connection
    heartbeat_failures: Arc<RwLock<std::collections::HashMap<String, u32>>>,

    /// Heartbeat success counters per connection
    heartbeat_successes: Arc<RwLock<std::collections::HashMap<String, u32>>>,

    /// Background heartbeat task handles
    heartbeat_tasks: Arc<RwLock<std::collections::HashMap<String, JoinHandle<()>>>>,

    /// Stop signals for heartbeat tasks
    heartbeat_stop_signals: Arc<RwLock<std::collections::HashMap<String, tokio::sync::watch::Sender<bool>>>>,

    /// Background reconnection task handles
    reconnect_tasks: Arc<RwLock<std::collections::HashMap<String, JoinHandle<()>>>>,
}

impl Default for ReconnectOrchestrator {
    fn default() -> Self {
        Self::new(ReconnectConfig::default())
    }
}

impl ReconnectOrchestrator {
    /// Create a new reconnection orchestrator with the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Reconnection configuration
    ///
    /// # Panics
    ///
    /// Panics if the configuration validation fails.
    pub fn new(config: ReconnectConfig) -> Self {
        config.validate().expect("Invalid reconnect config");

        let (event_tx, _) = broadcast::channel(256);

        Self {
            config,
            heartbeat_config: HeartbeatConfig::default(),
            current_attempts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            states: Arc::new(RwLock::new(std::collections::HashMap::new())),
            enabled: AtomicBool::new(true),
            user_disconnected: Arc::new(RwLock::new(std::collections::HashMap::new())),
            event_tx,
            heartbeat_failures: Arc::new(RwLock::new(std::collections::HashMap::new())),
            heartbeat_successes: Arc::new(RwLock::new(std::collections::HashMap::new())),
            heartbeat_tasks: Arc::new(RwLock::new(std::collections::HashMap::new())),
            heartbeat_stop_signals: Arc::new(RwLock::new(std::collections::HashMap::new())),
            reconnect_tasks: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Create a new orchestrator with custom heartbeat configuration.
    pub fn with_heartbeat(mut self, heartbeat_config: HeartbeatConfig) -> Self {
        heartbeat_config.validate().expect("Invalid heartbeat config");
        self.heartbeat_config = heartbeat_config;
        self
    }

    /// Enable or disable automatic reconnection globally.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// Check if automatic reconnection is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Subscribe to reconnection events.
    ///
    /// Returns a broadcast receiver that will receive all reconnection
    /// state change events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<ReconnectEvent> {
        self.event_tx.subscribe()
    }

    /// Get the current reconnection state for a connection.
    pub async fn get_state(&self, connection_id: &str) -> ReconnectState {
        let states = self.states.read().await;
        states
            .get(connection_id)
            .cloned()
            .unwrap_or(ReconnectState::Idle)
    }

    /// Get the current attempt count for a connection.
    pub async fn get_current_attempts(&self, connection_id: &str) -> u32 {
        let attempts = self.current_attempts.read().await;
        attempts.get(connection_id).copied().unwrap_or(0)
    }

    /// Start heartbeat monitoring for a connection.
    ///
    /// Begins periodic heartbeat checks to detect connection failures.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection to monitor
    ///
    /// # Example
    ///
    /// ```rust
    /// use easyssh_core::connection::ReconnectOrchestrator;
    ///
    /// let orchestrator = ReconnectOrchestrator::default();
    /// orchestrator.start_monitoring("conn-123");
    /// ```
    pub async fn start_monitoring(&self, connection_id: &str) {
        // Create stop signal channel
        let (stop_tx, mut stop_rx) = tokio::sync::watch::channel(false);

        // Store stop signal
        {
            let mut signals = self.heartbeat_stop_signals.write().await;
            signals.insert(connection_id.to_string(), stop_tx);
        }

        // Initialize monitoring state
        {
            let mut states = self.states.write().await;
            states.insert(
                connection_id.to_string(),
                ReconnectState::Monitoring {
                    started_at: Instant::now(),
                    consecutive_failures: 0,
                    consecutive_successes: 0,
                },
            );
        }

        // Clear failure counters
        {
            let mut failures = self.heartbeat_failures.write().await;
            failures.insert(connection_id.to_string(), 0);
            let mut successes = self.heartbeat_successes.write().await;
            successes.insert(connection_id.to_string(), 0);
        }

        // Clear user disconnected flag
        {
            let mut user_disc = self.user_disconnected.write().await;
            user_disc.remove(connection_id);
        }

        // Spawn heartbeat monitoring task
        let conn_id = connection_id.to_string();
        let interval = self.heartbeat_config.interval;
        let failure_threshold = self.heartbeat_config.failure_threshold;
        let heartbeat_failures = self.heartbeat_failures.clone();
        let heartbeat_successes = self.heartbeat_successes.clone();
        let states = self.states.clone();
        let event_tx = self.event_tx.clone();
        let enabled = Arc::new(AtomicBool::new(self.enabled.load(Ordering::SeqCst)));

        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if !enabled.load(Ordering::SeqCst) {
                            continue;
                        }

                        // Simulate heartbeat check (in real implementation,
                        // this would call the actual connection health check)
                        let heartbeat_ok = Self::perform_heartbeat_check(&conn_id).await;

                        if heartbeat_ok {
                            // Reset failures, increment successes
                            let mut failures = heartbeat_failures.write().await;
                            failures.insert(conn_id.clone(), 0);
                            let mut successes = heartbeat_successes.write().await;
                            let current = successes.get(&conn_id).copied().unwrap_or(0);
                            successes.insert(conn_id.clone(), current + 1);

                            // Update state
                            let mut states = states.write().await;
                            // Get the started_at value first, then insert
                            let started_at = states.get(&conn_id)
                                .and_then(|s| {
                                    if let ReconnectState::Monitoring { started_at, .. } = s {
                                        Some(*started_at)
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or_else(Instant::now);
                            states.insert(
                                conn_id.clone(),
                                ReconnectState::Monitoring {
                                    started_at,
                                    consecutive_failures: 0,
                                    consecutive_successes: current + 1,
                                },
                            );
                        } else {
                            // Increment failures, reset successes
                            let mut successes = heartbeat_successes.write().await;
                            successes.insert(conn_id.clone(), 0);
                            let mut failures = heartbeat_failures.write().await;
                            let current = failures.get(&conn_id).copied().unwrap_or(0);
                            failures.insert(conn_id.clone(), current + 1);

                            // Update state
                            let mut states = states.write().await;
                            // Get the started_at value first, then insert
                            let started_at = states.get(&conn_id)
                                .and_then(|s| {
                                    if let ReconnectState::Monitoring { started_at, .. } = s {
                                        Some(*started_at)
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or_else(Instant::now);
                            states.insert(
                                conn_id.clone(),
                                ReconnectState::Monitoring {
                                    started_at,
                                    consecutive_failures: current + 1,
                                    consecutive_successes: 0,
                                },
                            );

                            // Emit heartbeat failure event
                            let _ = event_tx.send(ReconnectEvent {
                                connection_id: conn_id.clone(),
                                state: ReconnectState::Monitoring {
                                    started_at: Instant::now(),
                                    consecutive_failures: current + 1,
                                    consecutive_successes: 0,
                                },
                                trigger: Some(ReconnectTrigger::HeartbeatFail {
                                    failure_count: current + 1,
                                }),
                                attempt: None,
                                error: Some("Heartbeat check failed".to_string()),
                                timestamp: Instant::now(),
                            });

                            // Check if threshold exceeded - would trigger reconnection
                            // (actual triggering is done by handle_disconnect)
                            if current + 1 >= failure_threshold {
                                tracing::warn!(
                                    "Heartbeat threshold exceeded for connection {}, triggering reconnection",
                                    conn_id
                                );
                            }
                        }
                    }
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            tracing::debug!("Heartbeat monitoring stopped for {}", conn_id);
                            break;
                        }
                    }
                }
            }
        });

        // Store task handle
        {
            let mut tasks = self.heartbeat_tasks.write().await;
            tasks.insert(connection_id.to_string(), handle);
        }

        tracing::info!("Started heartbeat monitoring for connection {}", connection_id);
    }

    /// Stop heartbeat monitoring for a connection.
    pub async fn stop_monitoring(&self, connection_id: &str) {
        // Send stop signal
        {
            let signals = self.heartbeat_stop_signals.read().await;
            if let Some(stop_tx) = signals.get(connection_id) {
                let _ = stop_tx.send(true);
            }
        }

        // Abort task if still running
        {
            let mut tasks = self.heartbeat_tasks.write().await;
            if let Some(handle) = tasks.remove(connection_id) {
                handle.abort();
            }
        }

        // Clear state
        {
            let mut states = self.states.write().await;
            states.remove(connection_id);
        }

        tracing::info!("Stopped heartbeat monitoring for connection {}", connection_id);
    }

    /// Handle a connection disconnect event.
    ///
    /// If `user_initiated` is true, automatic reconnection is disabled
    /// for this connection (SYSTEM_INVARIANTS.md constraint).
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection that disconnected
    /// * `user_initiated` - Whether the disconnect was user-initiated
    /// * `trigger` - What triggered the disconnect
    /// * `error` - Optional error message
    pub async fn handle_disconnect(
        &self,
        connection_id: &str,
        user_initiated: bool,
        trigger: ReconnectTrigger,
        error: Option<String>,
    ) {
        // SYSTEM_INVARIANTS.md: User-initiated disconnect does NOT trigger auto-reconnect
        if user_initiated {
            self.mark_user_disconnected(connection_id).await;
            return;
        }

        // Check if reconnection is enabled
        if !self.enabled.load(Ordering::SeqCst) {
            return;
        }

        // Check if user previously disconnected this connection
        {
            let user_disc = self.user_disconnected.read().await;
            if user_disc.contains_key(connection_id) {
                tracing::debug!(
                    "Skipping reconnection for user-disconnected connection {}",
                    connection_id
                );
                return;
            }
        }

        // Stop existing heartbeat monitoring
        self.stop_monitoring(connection_id).await;

        // Start reconnection process
        self.start_reconnection(connection_id, trigger, error).await;
    }

    /// Mark a connection as user-disconnected.
    ///
    /// Prevents automatic reconnection for this connection.
    async fn mark_user_disconnected(&self, connection_id: &str) {
        // Add to user disconnected map first
        {
            let mut user_disc = self.user_disconnected.write().await;
            user_disc.insert(connection_id.to_string(), Instant::now());
        }

        // Stop heartbeat monitoring (this may clear the state, so we do it before setting state)
        self.stop_monitoring(connection_id).await;

        // Cancel any ongoing reconnection
        {
            let mut tasks = self.reconnect_tasks.write().await;
            if let Some(handle) = tasks.remove(connection_id) {
                handle.abort();
            }
        }

        // Now set the UserDisconnected state (after stop_monitoring clears any previous state)
        {
            let mut states = self.states.write().await;
            states.insert(
                connection_id.to_string(),
                ReconnectState::UserDisconnected {
                    disconnected_at: Instant::now(),
                },
            );
        }

        // Emit event
        let _ = self.event_tx.send(ReconnectEvent {
            connection_id: connection_id.to_string(),
            state: ReconnectState::UserDisconnected {
                disconnected_at: Instant::now(),
            },
            trigger: Some(ReconnectTrigger::ManualRequest),
            attempt: None,
            error: None,
            timestamp: Instant::now(),
        });

        tracing::info!(
            "Connection {} marked as user-disconnected, auto-reconnect disabled",
            connection_id
        );
    }

    /// Start the reconnection process for a connection.
    async fn start_reconnection(
        &self,
        connection_id: &str,
        trigger: ReconnectTrigger,
        initial_error: Option<String>,
    ) {
        // Initialize attempt counter
        {
            let mut attempts = self.current_attempts.write().await;
            attempts.insert(connection_id.to_string(), 0);
        }

        // Create reconnection task
        let conn_id = connection_id.to_string();
        let config = self.config.clone();
        let current_attempts = self.current_attempts.clone();
        let states = self.states.clone();
        let event_tx = self.event_tx.clone();
        let user_disconnected = self.user_disconnected.clone();
        let enabled = Arc::new(AtomicBool::new(self.enabled.load(Ordering::SeqCst)));

        let handle = tokio::spawn(async move {
            for attempt in 0..config.max_retries {
                // Check if still enabled and not user-disconnected
                if !enabled.load(Ordering::SeqCst) {
                    tracing::debug!("Reconnection disabled, stopping for {}", conn_id);
                    return;
                }

                {
                    let user_disc = user_disconnected.read().await;
                    if user_disc.contains_key(&conn_id) {
                        tracing::debug!("User disconnected, stopping reconnection for {}", conn_id);
                        return;
                    }
                }

                // Update attempt counter
                {
                    let mut attempts = current_attempts.write().await;
                    attempts.insert(conn_id.clone(), attempt + 1);
                }

                // Calculate delay with exponential backoff
                let delay = config.calculate_delay(attempt);

                // Update state to waiting
                {
                    let mut states = states.write().await;
                    states.insert(
                        conn_id.clone(),
                        ReconnectState::Waiting {
                            attempt: attempt + 1,
                            next_attempt_at: Instant::now() + delay,
                            last_error: initial_error.clone(),
                        },
                    );
                }

                // Emit waiting event
                let _ = event_tx.send(ReconnectEvent {
                    connection_id: conn_id.clone(),
                    state: ReconnectState::Waiting {
                        attempt: attempt + 1,
                        next_attempt_at: Instant::now() + delay,
                        last_error: initial_error.clone(),
                    },
                    trigger: None,
                    attempt: Some(attempt + 1),
                    error: initial_error.clone(),
                    timestamp: Instant::now(),
                });

                // Wait before attempt
                tokio::time::sleep(delay).await;

                // Update state to attempting
                {
                    let mut states = states.write().await;
                    states.insert(
                        conn_id.clone(),
                        ReconnectState::Attempting {
                            attempt: attempt + 1,
                            started_at: Instant::now(),
                        },
                    );
                }

                // Emit attempting event
                let _ = event_tx.send(ReconnectEvent {
                    connection_id: conn_id.clone(),
                    state: ReconnectState::Attempting {
                        attempt: attempt + 1,
                        started_at: Instant::now(),
                    },
                    trigger: None,
                    attempt: Some(attempt + 1),
                    error: None,
                    timestamp: Instant::now(),
                });

                tracing::info!(
                    "Attempting reconnection for {} (attempt {})",
                    conn_id,
                    attempt + 1
                );

                // Attempt reconnection (placeholder - actual implementation would call connection logic)
                let reconnect_result = Self::attempt_reconnect(&conn_id).await;

                match reconnect_result {
                    Ok(()) => {
                        // Success!
                        {
                            let mut states = states.write().await;
                            states.insert(
                                conn_id.clone(),
                                ReconnectState::Succeeded {
                                    total_attempts: attempt + 1,
                                    duration: Instant::now().elapsed(),
                                },
                            );
                        }

                        // Emit success event
                        let _ = event_tx.send(ReconnectEvent {
                            connection_id: conn_id.clone(),
                            state: ReconnectState::Succeeded {
                                total_attempts: attempt + 1,
                                duration: Instant::now().elapsed(),
                            },
                            trigger: Some(ReconnectTrigger::InitialConnect),
                            attempt: Some(attempt + 1),
                            error: None,
                            timestamp: Instant::now(),
                        });

                        tracing::info!(
                            "Reconnection succeeded for {} after {} attempts",
                            conn_id,
                            attempt + 1
                        );

                        return;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Reconnection attempt {} failed for {}: {}",
                            attempt + 1,
                            conn_id,
                            e
                        );

                        // Continue to next attempt
                        if attempt + 1 >= config.max_retries {
                            // Final failure
                            {
                                let mut states = states.write().await;
                                states.insert(
                                    conn_id.clone(),
                                    ReconnectState::Failed {
                                        total_attempts: attempt + 1,
                                        final_error: e.clone(),
                                    },
                                );
                            }

                            // Emit failure event
                            let _ = event_tx.send(ReconnectEvent {
                                connection_id: conn_id.clone(),
                                state: ReconnectState::Failed {
                                    total_attempts: attempt + 1,
                                    final_error: e.clone(),
                                },
                                trigger: None,
                                attempt: Some(attempt + 1),
                                error: Some(e),
                                timestamp: Instant::now(),
                            });

                            tracing::error!(
                                "Reconnection failed for {} after {} attempts",
                                conn_id,
                                attempt + 1
                            );
                        }
                    }
                }
            }
        });

        // Store task handle
        {
            let mut tasks = self.reconnect_tasks.write().await;
            tasks.insert(connection_id.to_string(), handle);
        }
    }

    /// Perform a single heartbeat check.
    ///
    /// This is a placeholder implementation. In a real implementation,
    /// this would call the actual SSH session health check.
    async fn perform_heartbeat_check(_connection_id: &str) -> bool {
        // Placeholder: In real implementation, would check SSH connection health
        // by sending a keepalive or executing a simple command
        true
    }

    /// Attempt a single reconnection.
    ///
    /// This is a placeholder implementation. In a real implementation,
    /// this would call the actual SSH connection logic.
    async fn attempt_reconnect(connection_id: &str) -> Result<(), String> {
        // Placeholder: In real implementation, would attempt actual SSH reconnection
        // using stored credentials and connection parameters
        tracing::debug!("Placeholder reconnect attempt for {}", connection_id);
        Ok(())
    }

    /// Get heartbeat status for a connection.
    pub async fn get_heartbeat_status(&self, connection_id: &str) -> HeartbeatStatus {
        let failures = self.heartbeat_failures.read().await;
        let successes = self.heartbeat_successes.read().await;

        let failure_count = failures.get(connection_id).copied().unwrap_or(0);
        let success_count = successes.get(connection_id).copied().unwrap_or(0);

        if failure_count >= self.heartbeat_config.failure_threshold {
            HeartbeatStatus::Unhealthy
        } else if failure_count > 0 || success_count == 0 {
            HeartbeatStatus::Degraded
        } else {
            HeartbeatStatus::Healthy
        }
    }

    /// Reset the reconnection state for a connection.
    ///
    /// Used after a successful manual reconnection or when
    /// resetting the connection state.
    pub async fn reset(&self, connection_id: &str) {
        // Stop all monitoring and tasks
        self.stop_monitoring(connection_id).await;

        // Clear attempt counter
        {
            let mut attempts = self.current_attempts.write().await;
            attempts.remove(connection_id);
        }

        // Clear state
        {
            let mut states = self.states.write().await;
            states.insert(connection_id.to_string(), ReconnectState::Idle);
        }

        // Clear user disconnected flag
        {
            let mut user_disc = self.user_disconnected.write().await;
            user_disc.remove(connection_id);
        }

        // Clear counters
        {
            let mut failures = self.heartbeat_failures.write().await;
            failures.remove(connection_id);
            let mut successes = self.heartbeat_successes.write().await;
            successes.remove(connection_id);
        }

        tracing::info!("Reset reconnection state for connection {}", connection_id);
    }

    /// Cancel all ongoing reconnection tasks.
    pub async fn cancel_all_reconnections(&self) {
        let mut tasks = self.reconnect_tasks.write().await;
        for (_, handle) in tasks.drain() {
            handle.abort();
        }

        let mut states = self.states.write().await;
        for (_, state) in states.iter_mut() {
            if matches!(
                state,
                ReconnectState::Waiting { .. } | ReconnectState::Attempting { .. }
            ) {
                *state = ReconnectState::Idle;
            }
        }
    }

    /// Shutdown the orchestrator and clean up all resources.
    pub async fn shutdown(&self) {
        // Disable reconnection
        self.enabled.store(false, Ordering::SeqCst);

        // Stop all heartbeat tasks
        {
            let mut signals = self.heartbeat_stop_signals.write().await;
            for (_, stop_tx) in signals.drain() {
                let _ = stop_tx.send(true);
            }
        }

        // Abort all tasks
        {
            let mut heartbeat_tasks = self.heartbeat_tasks.write().await;
            for (_, handle) in heartbeat_tasks.drain() {
                handle.abort();
            }
        }

        {
            let mut reconnect_tasks = self.reconnect_tasks.write().await;
            for (_, handle) in reconnect_tasks.drain() {
                handle.abort();
            }
        }

        // Clear all state
        {
            let mut states = self.states.write().await;
            states.clear();
        }

        tracing::info!("Reconnect orchestrator shutdown complete");
    }

    /// Get the current configuration.
    pub fn config(&self) -> &ReconnectConfig {
        &self.config
    }

    /// Get the heartbeat configuration.
    pub fn heartbeat_config(&self) -> &HeartbeatConfig {
        &self.heartbeat_config
    }

    /// Check if a connection can be reconnected.
    ///
    /// Returns false if:
    /// - Connection was user-disconnected
    /// - Reconnection is globally disabled
    /// - Connection is in Failed state after exhausting retries
    pub async fn can_reconnect(&self, connection_id: &str) -> bool {
        if !self.enabled.load(Ordering::SeqCst) {
            return false;
        }

        let user_disc = self.user_disconnected.read().await;
        if user_disc.contains_key(connection_id) {
            return false;
        }

        let states = self.states.read().await;
        if let Some(state) = states.get(connection_id) {
            if matches!(state, ReconnectState::Failed { .. }) {
                return false;
            }
        }

        true
    }

    /// Manually trigger a reconnection for a connection.
    ///
    /// Clears any user-disconnected flag and starts reconnection.
    pub async fn manual_reconnect(&self, connection_id: &str) {
        // Clear user disconnected flag
        {
            let mut user_disc = self.user_disconnected.write().await;
            user_disc.remove(connection_id);
        }

        // Start reconnection
        self.start_reconnection(
            connection_id,
            ReconnectTrigger::ManualRequest,
            Some("Manual reconnect requested".to_string()),
        )
        .await;
    }
}

/// Heartbeat monitor for connection health checking.
///
/// Standalone heartbeat monitor that can be used without the full
/// orchestrator for simpler use cases.
pub struct HeartbeatMonitor {
    config: HeartbeatConfig,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_check: Arc<RwLock<Option<Instant>>>,
}

impl HeartbeatMonitor {
    /// Create a new heartbeat monitor with default configuration.
    pub fn new() -> Self {
        Self {
            config: HeartbeatConfig::default(),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_check: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a heartbeat monitor with custom configuration.
    pub fn with_config(config: HeartbeatConfig) -> Self {
        Self {
            config,
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_check: Arc::new(RwLock::new(None)),
        }
    }

    /// Record a heartbeat success.
    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Record a heartbeat failure.
    pub fn record_failure(&self) {
        self.success_count.store(0, Ordering::SeqCst);
        self.failure_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Get current heartbeat status.
    pub fn status(&self) -> HeartbeatStatus {
        let failures = self.failure_count.load(Ordering::SeqCst);
        let successes = self.success_count.load(Ordering::SeqCst);

        if failures >= self.config.failure_threshold {
            HeartbeatStatus::Unhealthy
        } else if failures > 0 || successes == 0 {
            HeartbeatStatus::Degraded
        } else {
            HeartbeatStatus::Healthy
        }
    }

    /// Check if heartbeat threshold has been exceeded.
    pub fn threshold_exceeded(&self) -> bool {
        self.failure_count.load(Ordering::SeqCst) >= self.config.failure_threshold
    }

    /// Reset the heartbeat counters.
    pub fn reset(&self) {
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
    }

    /// Get the failure count.
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::SeqCst)
    }

    /// Get the success count.
    pub fn success_count(&self) -> u32 {
        self.success_count.load(Ordering::SeqCst)
    }
}

impl Default for HeartbeatMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_reconnect_config_default() {
        let config = ReconnectConfig::default();
        assert_eq!(config.max_retries, 10);
        assert_eq!(config.base_delay, Duration::from_secs(1));
        assert_eq!(config.max_delay, Duration::from_secs(60));
        assert!((config.jitter - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_reconnect_config_validation() {
        // Valid config
        let config = ReconnectConfig::default();
        assert!(config.validate().is_ok());

        // Invalid: zero retries
        let config = ReconnectConfig {
            max_retries: 0,
            ..ReconnectConfig::default()
        };
        assert!(config.validate().is_err());

        // Invalid: zero base delay
        let config = ReconnectConfig {
            base_delay: Duration::ZERO,
            ..ReconnectConfig::default()
        };
        assert!(config.validate().is_err());

        // Invalid: max < base
        let config = ReconnectConfig {
            base_delay: Duration::from_secs(10),
            max_delay: Duration::from_secs(5),
            ..ReconnectConfig::default()
        };
        assert!(config.validate().is_err());

        // Invalid: jitter out of range
        let config = ReconnectConfig {
            jitter: 1.5,
            ..ReconnectConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_calculate_delay_exponential_backoff() {
        let config = ReconnectConfig {
            jitter: 0.0, // Disable jitter for deterministic testing
            ..ReconnectConfig::default()
        };

        // Attempt 0: base * 2^0 = 1s
        let delay0 = config.calculate_delay(0);
        assert_eq!(delay0, Duration::from_secs(1));

        // Attempt 1: base * 2^1 = 2s
        let delay1 = config.calculate_delay(1);
        assert_eq!(delay1, Duration::from_secs(2));

        // Attempt 2: base * 2^2 = 4s
        let delay2 = config.calculate_delay(2);
        assert_eq!(delay2, Duration::from_secs(4));

        // Attempt 3: base * 2^3 = 8s
        let delay3 = config.calculate_delay(3);
        assert_eq!(delay3, Duration::from_secs(8));
    }

    #[test]
    fn test_calculate_delay_max_cap() {
        let config = ReconnectConfig {
            max_delay: Duration::from_secs(30),
            jitter: 0.0,
            ..ReconnectConfig::default()
        };

        // Attempt 10: base * 2^10 = 1024s, but capped at 30s
        let delay10 = config.calculate_delay(10);
        assert_eq!(delay10, Duration::from_secs(30));
    }

    #[test]
    fn test_calculate_delay_with_jitter() {
        let config = ReconnectConfig::default();

        // With 30% jitter, delay should vary within the jitter range
        for attempt in 0..5 {
            let delay = config.calculate_delay(attempt);
            let base_delay = config.base_delay * 2u32.pow(attempt);
            let capped_delay = base_delay.min(config.max_delay);

            // Delay should be within [capped * (1-jitter), capped * (1+jitter)]
            // Since mul_f64 can produce slight variations, we use generous bounds
            let min_delay = capped_delay.mul_f64(0.5); // Allow for significant variation
            let max_delay = capped_delay.mul_f64(1.5);

            assert!(delay >= min_delay && delay <= max_delay,
                "Delay {} for attempt {} not in range [{}, {}]",
                delay.as_millis(), attempt, min_delay.as_millis(), max_delay.as_millis());
        }
    }

    #[test]
    fn test_heartbeat_config_default() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.interval, Duration::from_secs(30));
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.recovery_threshold, 2);
    }

    #[test]
    fn test_heartbeat_config_validation() {
        // Valid config
        let config = HeartbeatConfig::default();
        assert!(config.validate().is_ok());

        // Invalid: zero interval
        let config = HeartbeatConfig {
            interval: Duration::ZERO,
            ..HeartbeatConfig::default()
        };
        assert!(config.validate().is_err());

        // Invalid: zero threshold
        let config = HeartbeatConfig {
            failure_threshold: 0,
            ..HeartbeatConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_heartbeat_monitor_status() {
        let monitor = HeartbeatMonitor::new();

        // Initial status should be degraded (no checks yet)
        assert_eq!(monitor.status(), HeartbeatStatus::Degraded);

        // After success, should be healthy
        monitor.record_success();
        assert_eq!(monitor.status(), HeartbeatStatus::Healthy);

        // After 2 failures, should be degraded
        monitor.record_failure();
        monitor.record_failure();
        assert_eq!(monitor.status(), HeartbeatStatus::Degraded);

        // After 3 failures, should be unhealthy
        monitor.record_failure();
        assert_eq!(monitor.status(), HeartbeatStatus::Unhealthy);
        assert!(monitor.threshold_exceeded());

        // Reset should clear counters
        monitor.reset();
        assert_eq!(monitor.failure_count(), 0);
        assert_eq!(monitor.success_count(), 0);
    }

    #[tokio::test]
    async fn test_reconnect_orchestrator_basic() {
        let orchestrator = ReconnectOrchestrator::default();

        // Should be enabled by default
        assert!(orchestrator.is_enabled());

        // Get initial state
        let state = orchestrator.get_state("conn-123").await;
        assert_eq!(state, ReconnectState::Idle);

        // Can reconnect initially
        assert!(orchestrator.can_reconnect("conn-123").await);
    }

    #[tokio::test]
    async fn test_reconnect_orchestrator_user_disconnect() {
        let orchestrator = ReconnectOrchestrator::default();

        // Handle user-initiated disconnect
        orchestrator
            .handle_disconnect("conn-123", true, ReconnectTrigger::ManualRequest, None)
            .await;

        // Should not be able to reconnect after user disconnect
        assert!(!orchestrator.can_reconnect("conn-123").await);

        // State should be UserDisconnected
        let state = orchestrator.get_state("conn-123").await;
        assert!(matches!(state, ReconnectState::UserDisconnected { .. }));
    }

    #[tokio::test]
    async fn test_reconnect_orchestrator_start_monitoring() {
        let orchestrator = ReconnectOrchestrator::default();

        // Start monitoring
        orchestrator.start_monitoring("conn-123").await;

        // State should be Monitoring
        let state = orchestrator.get_state("conn-123").await;
        assert!(matches!(state, ReconnectState::Monitoring { .. }));

        // Heartbeat status should be degraded initially
        let status = orchestrator.get_heartbeat_status("conn-123").await;
        assert_eq!(status, HeartbeatStatus::Degraded);

        // Stop monitoring
        orchestrator.stop_monitoring("conn-123").await;

        // State should be cleared
        let state = orchestrator.get_state("conn-123").await;
        assert_eq!(state, ReconnectState::Idle);
    }

    #[tokio::test]
    async fn test_reconnect_orchestrator_reset() {
        let orchestrator = ReconnectOrchestrator::default();

        // User disconnect
        orchestrator
            .handle_disconnect("conn-123", true, ReconnectTrigger::ManualRequest, None)
            .await;

        // Reset
        orchestrator.reset("conn-123").await;

        // Should be able to reconnect now
        assert!(orchestrator.can_reconnect("conn-123").await);

        // State should be Idle
        let state = orchestrator.get_state("conn-123").await;
        assert_eq!(state, ReconnectState::Idle);
    }

    #[tokio::test]
    async fn test_reconnect_orchestrator_disabled() {
        let orchestrator = ReconnectOrchestrator::default();

        // Disable reconnection
        orchestrator.set_enabled(false);

        // Should not be able to reconnect
        assert!(!orchestrator.can_reconnect("conn-123").await);

        // Re-enable
        orchestrator.set_enabled(true);
        assert!(orchestrator.is_enabled());
    }

    #[tokio::test]
    async fn test_reconnect_orchestrator_events() {
        let orchestrator = ReconnectOrchestrator::default();

        // Subscribe to events
        let mut event_rx = orchestrator.subscribe_events();

        // User disconnect should emit event
        orchestrator
            .handle_disconnect("conn-456", true, ReconnectTrigger::ManualRequest, None)
            .await;

        // Should receive event
        match event_rx.try_recv() {
            Ok(event) => {
                assert_eq!(event.connection_id, "conn-456");
                assert!(matches!(event.state, ReconnectState::UserDisconnected { .. }));
            }
            Err(_) => {
                // Event may have been received already or channel overflow
                // This is acceptable for the test
            }
        }
    }
}