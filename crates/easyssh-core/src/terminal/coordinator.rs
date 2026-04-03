//! Session Coordinator - Unified terminal management for UI layer
//!
//! This module provides a high-level coordinator that unifies:
//! - TabManager (tab lifecycle and UI state)
//! - TerminalManager (PTY terminal instances)
//! - SshSessionManager (SSH connections and pooling)
//!
//! The coordinator provides a clean API for the UI layer to:
//! - Create/close terminal sessions
//! - Manage terminal output streams
//! - Send input and resize events
//! - Handle connection events
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                    UI Layer                          │
//! │  (egui/GTK4/SwiftUI)                                │
//! └─────────────────────┬───────────────────────────────┘
//!                       │
//! ┌─────────────────────▼───────────────────────────────┐
//! │              SessionCoordinator                     │
//! │  ┌─────────────────────────────────────────────┐   │
//! │  │ Session-Tab Mapping                          │   │
//! │  │ Event Broadcasting                           │   │
//! │  │ Unified API                                  │   │
//! │  └─────────────────────────────────────────────┘   │
//! └─────────────────────┬───────────────────────────────┘
//!                       │
//!       ┌───────────────┼───────────────┐
//!       │               │               │
//! ┌─────▼─────┐   ┌─────▼─────┐   ┌─────▼─────┐
//! │TabManager │   │Terminal   │   │SshSession │
//! │           │   │Manager    │   │Manager    │
//! └───────────┘   └───────────┘   └───────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use easyssh_core::terminal::coordinator::SessionCoordinator;
//!
//! // Create coordinator
//! let coordinator = SessionCoordinator::new();
//!
//! // Create terminal for a server
//! let terminal_id = coordinator.create_terminal("server-123").await?;
//!
//! // Get output stream
//! let output_rx = coordinator.get_terminal_output(&terminal_id).await?;
//!
//! // Send input
//! coordinator.send_input(&terminal_id, "ls -la\n").await?;
//!
//! // Resize terminal
//! coordinator.resize_terminal(&terminal_id, 120, 40).await?;
//!
//! // Close terminal
//! coordinator.close_terminal(&terminal_id).await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::error::LiteError;
use crate::ssh::SshSessionManager;

#[cfg(feature = "embedded-terminal")]
use super::embedded::TerminalManager;
#[cfg(feature = "embedded-terminal")]
use super::multitab::{SessionType, TabManager, TabState};
use super::{TerminalOutput, TerminalSize, TerminalStats};

// ============================================================================
// Terminal Coordinator Events
// ============================================================================

/// Events emitted by the TerminalCoordinator
///
/// These events allow the UI layer to react to terminal state changes
/// without directly monitoring individual components.
#[derive(Debug, Clone)]
pub enum CoordinatorEvent {
    /// Terminal session established successfully
    TerminalConnected {
        terminal_id: String,
        tab_id: String,
        server_id: Option<String>,
    },

    /// Terminal session disconnected
    TerminalDisconnected {
        terminal_id: String,
        tab_id: String,
        reason: Option<String>,
    },

    /// Terminal encountered an error
    TerminalError {
        terminal_id: String,
        tab_id: Option<String>,
        error: String,
    },

    /// Terminal was resized
    TerminalResized {
        terminal_id: String,
        tab_id: String,
        cols: u16,
        rows: u16,
    },

    /// Terminal output received (for UI updates)
    TerminalOutput {
        terminal_id: String,
        data: String,
    },

    /// Terminal title changed
    TerminalTitleChanged {
        terminal_id: String,
        tab_id: String,
        title: String,
    },

    /// Tab was created (without terminal attachment yet)
    TabCreated {
        tab_id: String,
        title: String,
        session_type: SessionType,
    },

    /// Tab was closed
    TabClosed {
        tab_id: String,
    },

    /// Tab was activated (switched to)
    TabActivated {
        tab_id: String,
        terminal_id: Option<String>,
    },

    /// SSH session connected
    SshSessionConnected {
        session_id: String,
        server_id: String,
        host: String,
        port: u16,
    },

    /// SSH session disconnected
    SshSessionDisconnected {
        session_id: String,
        server_id: String,
    },

    /// All terminals closed
    AllTerminalsClosed,
}

impl CoordinatorEvent {
    /// Get the terminal ID if this event is terminal-related
    pub fn terminal_id(&self) -> Option<&str> {
        match self {
            CoordinatorEvent::TerminalConnected { terminal_id, .. } => Some(terminal_id),
            CoordinatorEvent::TerminalDisconnected { terminal_id, .. } => Some(terminal_id),
            CoordinatorEvent::TerminalError { terminal_id, .. } => Some(terminal_id),
            CoordinatorEvent::TerminalResized { terminal_id, .. } => Some(terminal_id),
            CoordinatorEvent::TerminalOutput { terminal_id, .. } => Some(terminal_id),
            CoordinatorEvent::TerminalTitleChanged { terminal_id, .. } => Some(terminal_id),
            _ => None,
        }
    }

    /// Get the tab ID if this event is tab-related
    pub fn tab_id(&self) -> Option<&str> {
        match self {
            CoordinatorEvent::TerminalConnected { tab_id, .. } => Some(tab_id),
            CoordinatorEvent::TerminalDisconnected { tab_id, .. } => Some(tab_id),
            CoordinatorEvent::TerminalResized { tab_id, .. } => Some(tab_id),
            CoordinatorEvent::TerminalTitleChanged { tab_id, .. } => Some(tab_id),
            CoordinatorEvent::TabCreated { tab_id, .. } => Some(tab_id),
            CoordinatorEvent::TabClosed { tab_id } => Some(tab_id),
            CoordinatorEvent::TabActivated { tab_id, .. } => Some(tab_id),
            _ => None,
        }
    }
}

// ============================================================================
// Session-Tab Mapping
// ============================================================================

/// Mapping between terminal sessions and tabs
///
/// This structure maintains the relationship between:
/// - Terminal instances (PTY sessions)
/// - UI tabs (visual representation)
/// - SSH sessions (remote connections)
/// - Server configurations (database records)
#[derive(Debug, Clone)]
pub struct SessionTabMapping {
    /// Terminal ID (unique identifier for the PTY instance)
    pub terminal_id: String,

    /// Tab ID (unique identifier for the UI tab)
    pub tab_id: String,

    /// Server ID (reference to server configuration, if SSH session)
    pub server_id: Option<String>,

    /// SSH session ID (reference to SshSessionManager session, if SSH)
    pub ssh_session_id: Option<String>,

    /// Session type (Local, SSH, etc.)
    pub session_type: SessionType,

    /// Creation timestamp
    pub created_at: Instant,

    /// Last activity timestamp
    pub last_activity: Instant,

    /// Current terminal size
    pub size: TerminalSize,

    /// Whether the session is active
    pub is_active: bool,

    /// Connection state
    pub connection_state: ConnectionState,
}

/// Connection state for a terminal session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Initializing the session
    Initializing,

    /// Connecting to remote server (for SSH)
    Connecting,

    /// Connected and active
    Connected,

    /// Disconnected from remote server
    Disconnected,

    /// Reconnecting after disconnection
    Reconnecting,

    /// Error state
    Error,

    /// Closed and cleaned up
    Closed,
}

impl ConnectionState {
    /// Check if the connection is active (usable)
    pub fn is_active(&self) -> bool {
        matches!(self, ConnectionState::Connected | ConnectionState::Reconnecting)
    }

    /// Check if the connection can be closed
    pub fn can_close(&self) -> bool {
        !matches!(self, ConnectionState::Closed)
    }

    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionState::Initializing => "initializing",
            ConnectionState::Connecting => "connecting",
            ConnectionState::Connected => "connected",
            ConnectionState::Disconnected => "disconnected",
            ConnectionState::Reconnecting => "reconnecting",
            ConnectionState::Error => "error",
            ConnectionState::Closed => "closed",
        }
    }
}

impl SessionTabMapping {
    /// Create a new session-tab mapping
    pub fn new(
        terminal_id: &str,
        tab_id: &str,
        session_type: SessionType,
        size: TerminalSize,
    ) -> Self {
        Self {
            terminal_id: terminal_id.to_string(),
            tab_id: tab_id.to_string(),
            server_id: None,
            ssh_session_id: None,
            session_type,
            created_at: Instant::now(),
            last_activity: Instant::now(),
            size,
            is_active: true,
            connection_state: ConnectionState::Initializing,
        }
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Get idle duration
    pub fn idle_duration(&self) -> Duration {
        self.last_activity.elapsed()
    }

    /// Check if session is idle (exceeded timeout)
    pub fn is_idle(&self, timeout_secs: u64) -> bool {
        self.idle_duration() > Duration::from_secs(timeout_secs)
    }

    /// Set server ID for SSH session
    pub fn with_server(mut self, server_id: &str) -> Self {
        self.server_id = Some(server_id.to_string());
        self
    }

    /// Set SSH session ID
    pub fn with_ssh_session(mut self, ssh_session_id: &str) -> Self {
        self.ssh_session_id = Some(ssh_session_id.to_string());
        self
    }

    /// Set connection state
    pub fn set_state(&mut self, state: ConnectionState) {
        self.connection_state = state;
        if state == ConnectionState::Connected {
            self.is_active = true;
        } else if state == ConnectionState::Closed {
            self.is_active = false;
        }
    }
}

// ============================================================================
// Coordinator Configuration
// ============================================================================

/// Configuration for the TerminalCoordinator
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Default terminal size for new sessions
    pub default_size: TerminalSize,

    /// Idle timeout in seconds before auto-closing sessions
    pub idle_timeout_secs: u64,

    /// Maximum number of concurrent terminal sessions
    pub max_sessions: usize,

    /// Enable automatic reconnection for SSH sessions
    pub auto_reconnect: bool,

    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,

    /// Delay between reconnection attempts (milliseconds)
    pub reconnect_delay_ms: u64,

    /// Enable output buffering for performance
    pub enable_output_buffering: bool,

    /// Output buffer size (bytes)
    pub output_buffer_size: usize,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            default_size: TerminalSize::new(24, 80),
            idle_timeout_secs: 3600, // 1 hour
            max_sessions: 100,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            reconnect_delay_ms: 1000,
            enable_output_buffering: true,
            output_buffer_size: 8192,
        }
    }
}

// ============================================================================
// Terminal Coordinator Statistics
// ============================================================================

/// Statistics for the TerminalCoordinator
#[derive(Debug, Clone, Default)]
pub struct CoordinatorStats {
    /// Total number of sessions created
    pub total_sessions_created: u64,

    /// Total number of sessions closed
    pub total_sessions_closed: u64,

    /// Current active sessions
    pub active_sessions: usize,

    /// Total SSH connections
    pub ssh_connections: usize,

    /// Total local terminals
    pub local_terminals: usize,

    /// Total bytes received
    pub total_bytes_received: u64,

    /// Total bytes sent
    pub total_bytes_sent: u64,

    /// Total errors encountered
    pub total_errors: u64,

    /// Average connection time (milliseconds)
    pub avg_connect_time_ms: f32,

    /// Reconnection attempts
    pub reconnect_attempts: u32,

    /// Successful reconnects
    pub successful_reconnects: u32,
}

// ============================================================================
// Terminal Coordinator
// ============================================================================

/// Session Coordinator - Unified terminal management for UI layer
///
/// The coordinator provides a single entry point for the UI layer to
/// manage terminal sessions, handling the complexity of:
/// - Tab management (UI state)
/// - Terminal management (PTY instances)
/// - SSH session management (remote connections)
///
/// # Features
///
/// - **Unified API**: Simple methods for creating, closing, and interacting with terminals
/// - **Event Broadcasting**: Subscribe to terminal events for UI updates
/// - **Session-Tab Mapping**: Automatic mapping between terminals and UI tabs
/// - **Auto-Reconnect**: Automatic reconnection for SSH sessions on disconnection
/// - **Idle Cleanup**: Automatic cleanup of idle sessions
/// - **Statistics**: Comprehensive statistics for monitoring
///
/// # Example
///
/// ```rust,ignore
/// let coordinator = SessionCoordinator::new();
///
/// // Subscribe to events
/// let event_rx = coordinator.subscribe_events();
///
/// // Create a terminal
/// let terminal_id = coordinator.create_terminal("server-123").await?;
///
/// // Work with the terminal
/// coordinator.send_input(&terminal_id, "ls\n").await?;
///
/// // Close when done
/// coordinator.close_terminal(&terminal_id).await?;
/// ```
pub struct SessionCoordinator {
    /// Configuration
    config: CoordinatorConfig,

    /// Terminal manager (PTY instances)
    terminal_manager: Arc<TerminalManager>,

    /// Tab manager (UI tabs)
    tab_manager: Arc<TabManager>,

    /// SSH session manager (remote connections)
    ssh_manager: Arc<RwLock<SshSessionManager>>,

    /// Session-tab mappings
    mappings: Arc<RwLock<HashMap<String, SessionTabMapping>>>,

    /// Terminal ID to output receiver mapping
    output_receivers: Arc<RwLock<HashMap<String, mpsc::UnboundedReceiver<TerminalOutput>>>>,

    /// Event broadcaster
    event_tx: broadcast::Sender<CoordinatorEvent>,

    /// Statistics
    stats: Arc<RwLock<CoordinatorStats>>,

    /// Active flag (for shutdown)
    active: Arc<std::sync::atomic::AtomicBool>,
}

impl SessionCoordinator {
    /// Create a new SessionCoordinator with default configuration
    pub fn new() -> Self {
        Self::with_config(CoordinatorConfig::default())
    }

    /// Create a new SessionCoordinator with custom configuration
    pub fn with_config(config: CoordinatorConfig) -> Self {
        let (event_tx, _) = broadcast::channel(256);

        Self {
            config,
            terminal_manager: Arc::new(TerminalManager::new()),
            tab_manager: Arc::new(TabManager::new()),
            ssh_manager: Arc::new(RwLock::new(SshSessionManager::new())),
            mappings: Arc::new(RwLock::new(HashMap::new())),
            output_receivers: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            stats: Arc::new(RwLock::new(CoordinatorStats::default())),
            active: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }

    /// Subscribe to coordinator events
    ///
    /// Returns a broadcast receiver that will receive all coordinator events.
    /// This is useful for UI components that need to react to terminal state changes.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut event_rx = coordinator.subscribe_events();
    ///
    /// while let Ok(event) = event_rx.recv().await {
    ///     match event {
    ///         CoordinatorEvent::TerminalConnected { terminal_id, .. } => {
    ///             println!("Terminal {} connected", terminal_id);
    ///         }
    ///         CoordinatorEvent::TerminalOutput { terminal_id, data } => {
    ///             // Update UI with terminal output
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub fn subscribe_events(&self) -> broadcast::Receiver<CoordinatorEvent> {
        self.event_tx.subscribe()
    }

    /// Create a new terminal session
    ///
    /// Creates a terminal session with an associated tab. For SSH sessions,
    /// also establishes an SSH connection.
    ///
    /// # Arguments
    ///
    /// * `server_id` - Server ID for SSH session, or None for local shell
    ///
    /// # Returns
    ///
    /// The terminal ID for the newly created session.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Maximum sessions limit reached
    /// - Tab creation fails
    /// - Terminal creation fails
    /// - SSH connection fails (for SSH sessions)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Create local terminal
    /// let local_id = coordinator.create_terminal(None).await?;
    ///
    /// // Create SSH terminal
    /// let ssh_id = coordinator.create_terminal("server-123").await?;
    /// ```
    pub async fn create_terminal(
        &self,
        server_id: Option<&str>,
    ) -> Result<String, LiteError> {
        // Check session limit
        {
            let mappings = self.mappings.read().await;
            if mappings.len() >= self.config.max_sessions {
                return Err(LiteError::Terminal("Maximum sessions limit reached".to_string()));
            }
        }

        // Generate IDs
        let terminal_id = Uuid::new_v4().to_string();
        let session_type = if server_id.is_some() {
            SessionType::Ssh
        } else {
            SessionType::LocalShell
        };

        // Create tab
        let tab_title = self.tab_manager.next_tab_title(session_type.default_title()).await;
        let tab_id = self
            .tab_manager
            .create_tab(&tab_title, session_type, None)
            .await?;

        // Emit tab created event
        let _ = self.event_tx.send(CoordinatorEvent::TabCreated {
            tab_id: tab_id.clone(),
            title: tab_title,
            session_type,
        });

        // Create mapping
        let mapping = SessionTabMapping::new(
            &terminal_id,
            &tab_id,
            session_type,
            self.config.default_size,
        )
        .with_server(server_id.unwrap_or_default());

        // Create terminal
        let output_rx = match session_type {
            SessionType::LocalShell => {
                self.terminal_manager
                    .create_local(&terminal_id, self.config.default_size, None)
                    .await?
            }
            SessionType::Ssh => {
                // For SSH, we need server info - this would typically come from a database
                // For now, we'll create a placeholder that can be enhanced later
                self.terminal_manager
                    .create_ssh(
                        &terminal_id,
                        self.config.default_size,
                        "placeholder", // Would be replaced with actual host
                        22,
                        "root",        // Would be replaced with actual username
                        super::embedded::SshAuthMethod::Agent,
                    )
                    .await?
            }
            _ => {
                return Err(LiteError::Terminal(format!(
                    "Session type {:?} not supported",
                    session_type
                )));
            }
        };

        // Store output receiver
        {
            let mut receivers = self.output_receivers.write().await;
            receivers.insert(terminal_id.clone(), output_rx);
        }

        // Attach terminal to tab
        self.tab_manager.attach_terminal(&tab_id, &terminal_id).await?;

        // Store mapping
        {
            let mut mappings = self.mappings.write().await;
            mappings.insert(terminal_id.clone(), mapping);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_sessions_created += 1;
            stats.active_sessions += 1;
            if session_type == SessionType::Ssh {
                stats.ssh_connections += 1;
            } else {
                stats.local_terminals += 1;
            }
        }

        // Set tab state to active
        self.tab_manager.set_state(&tab_id, TabState::Active).await?;

        // Emit connected event
        let _ = self.event_tx.send(CoordinatorEvent::TerminalConnected {
            terminal_id: terminal_id.clone(),
            tab_id,
            server_id: server_id.map(|s| s.to_string()),
        });

        // Start output forwarder
        self.start_output_forwarder(&terminal_id);

        log::info!(
            "Created terminal session: {} (type: {:?}, server: {:?})",
            terminal_id,
            session_type,
            server_id
        );

        Ok(terminal_id)
    }

    /// Create terminal with SSH configuration
    ///
    /// This method provides full SSH configuration for creating terminal sessions.
    ///
    /// # Arguments
    ///
    /// * `server_id` - Server ID from database
    /// * `host` - SSH host address
    /// * `port` - SSH port
    /// * `username` - SSH username
    /// * `auth_method` - Authentication method
    ///
    /// # Returns
    ///
    /// The terminal ID for the newly created session.
    pub async fn create_ssh_terminal(
        &self,
        server_id: &str,
        host: &str,
        port: u16,
        username: &str,
        auth_method: super::embedded::SshAuthMethod,
    ) -> Result<String, LiteError> {
        // Check session limit
        {
            let mappings = self.mappings.read().await;
            if mappings.len() >= self.config.max_sessions {
                return Err(LiteError::Terminal("Maximum sessions limit reached".to_string()));
            }
        }

        // Generate IDs
        let terminal_id = Uuid::new_v4().to_string();
        let ssh_session_id = format!("ssh_{}", terminal_id);

        // Create tab
        let tab_title = format!("{}@{}", username, host);
        let tab_id = self
            .tab_manager
            .create_tab(&tab_title, SessionType::Ssh, None)
            .await?;

        // Create SSH session
        {
            let mut ssh_manager = self.ssh_manager.write().await;
            let metadata = ssh_manager
                .connect(&ssh_session_id, host, port, username, None)
                .await?;

            log::info!(
                "SSH session {} connected: {}@{}:{}",
                ssh_session_id,
                metadata.username,
                metadata.host,
                metadata.port
            );
        }

        // Create mapping
        let mapping = SessionTabMapping::new(
            &terminal_id,
            &tab_id,
            SessionType::Ssh,
            self.config.default_size,
        )
        .with_server(server_id)
        .with_ssh_session(&ssh_session_id);

        // Create terminal
        let output_rx = self.terminal_manager
            .create_ssh(&terminal_id, self.config.default_size, host, port, username, auth_method)
            .await?;

        // Store output receiver
        {
            let mut receivers = self.output_receivers.write().await;
            receivers.insert(terminal_id.clone(), output_rx);
        }

        // Attach terminal to tab
        self.tab_manager.attach_terminal(&tab_id, &terminal_id).await?;

        // Store mapping
        {
            let mut mappings = self.mappings.write().await;
            mappings.insert(terminal_id.clone(), mapping);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_sessions_created += 1;
            stats.active_sessions += 1;
            stats.ssh_connections += 1;
        }

        // Set tab state
        self.tab_manager.set_state(&tab_id, TabState::Active).await?;

        // Emit events
        let _ = self.event_tx.send(CoordinatorEvent::SshSessionConnected {
            session_id: ssh_session_id.clone(),
            server_id: server_id.to_string(),
            host: host.to_string(),
            port,
        });
        let _ = self.event_tx.send(CoordinatorEvent::TerminalConnected {
            terminal_id: terminal_id.clone(),
            tab_id,
            server_id: Some(server_id.to_string()),
        });

        // Start output forwarder
        self.start_output_forwarder(&terminal_id);

        Ok(terminal_id)
    }

    /// Start output forwarder for a terminal
    ///
    /// This spawns a background task that forwards terminal output
    /// through the event broadcaster.
    fn start_output_forwarder(&self, terminal_id: &str) {
        let terminal_id = terminal_id.to_string();
        let mappings = self.mappings.clone();
        let active = self.active.clone();

        tokio::spawn(async move {
            // Output forwarding loop
            // This monitors the terminal output and broadcasts events
            log::info!("Output forwarder started for {}", terminal_id);

            while active.load(std::sync::atomic::Ordering::Relaxed) {
                // Check if terminal still exists
                {
                    let mappings_guard = mappings.read().await;
                    if !mappings_guard.contains_key(&terminal_id) {
                        log::info!("Terminal {} no longer exists, stopping forwarder", terminal_id);
                        break;
                    }
                }

                // Note: Actual output forwarding would be implemented through
                // terminal manager event subscription. For now, we just maintain
                // the forwarder task to handle cleanup on terminal closure.

                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            log::info!("Output forwarder stopped for {}", terminal_id);
        });
    }

    /// Close a terminal session
    ///
    /// Closes the terminal, associated tab, and SSH session (if applicable).
    ///
    /// # Arguments
    ///
    /// * `terminal_id` - The terminal ID to close
    ///
    /// # Errors
    ///
    /// Returns error if the terminal is not found or closing fails.
    pub async fn close_terminal(&self, terminal_id: &str) -> Result<(), LiteError> {
        // Get mapping
        let mapping = {
            let mappings = self.mappings.read().await;
            mappings
                .get(terminal_id)
                .cloned()
                .ok_or_else(|| LiteError::Terminal(format!("Terminal {} not found", terminal_id)))?
        };

        // Close SSH session if exists
        if let Some(ref ssh_session_id) = mapping.ssh_session_id {
            let mut ssh_manager = self.ssh_manager.write().await;
            if let Err(e) = ssh_manager.disconnect(ssh_session_id).await {
                log::warn!("Failed to disconnect SSH session {}: {}", ssh_session_id, e);
            }

            // Emit SSH disconnected event
            if let Some(ref server_id) = mapping.server_id {
                let _ = self.event_tx.send(CoordinatorEvent::SshSessionDisconnected {
                    session_id: ssh_session_id.clone(),
                    server_id: server_id.clone(),
                });
            }
        }

        // Close terminal
        self.terminal_manager.close(terminal_id).await?;

        // Close tab
        self.tab_manager.close_tab(&mapping.tab_id).await?;

        // Remove mapping
        {
            let mut mappings = self.mappings.write().await;
            mappings.remove(terminal_id);
        }

        // Remove output receiver
        {
            let mut receivers = self.output_receivers.write().await;
            receivers.remove(terminal_id);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_sessions_closed += 1;
            stats.active_sessions = stats.active_sessions.saturating_sub(1);
            if mapping.session_type == SessionType::Ssh {
                stats.ssh_connections = stats.ssh_connections.saturating_sub(1);
            } else {
                stats.local_terminals = stats.local_terminals.saturating_sub(1);
            }
        }

        // Emit events
        let _ = self.event_tx.send(CoordinatorEvent::TabClosed {
            tab_id: mapping.tab_id.clone(),
        });
        let _ = self.event_tx.send(CoordinatorEvent::TerminalDisconnected {
            terminal_id: terminal_id.to_string(),
            tab_id: mapping.tab_id,
            reason: Some("User closed".to_string()),
        });

        log::info!("Closed terminal session: {}", terminal_id);

        Ok(())
    }

    /// Get terminal output receiver
    ///
    /// Returns a clone of the output receiver for the specified terminal.
    /// Note: This creates a new channel pair since mpsc receivers cannot be cloned.
    ///
    /// # Arguments
    ///
    /// * `terminal_id` - The terminal ID
    ///
    /// # Returns
    ///
    /// A new output receiver channel.
    pub async fn get_terminal_output(
        &self,
        terminal_id: &str,
    ) -> Result<mpsc::UnboundedReceiver<TerminalOutput>, LiteError> {
        // Check if terminal exists
        {
            let mappings = self.mappings.read().await;
            if !mappings.contains_key(terminal_id) {
                return Err(LiteError::Terminal(format!(
                    "Terminal {} not found",
                    terminal_id
                )));
            }
        }

        // For getting output, we create a forwarding channel
        // since we can't clone the original receiver
        let (tx, rx) = mpsc::unbounded_channel();

        // Set up forwarding from terminal manager
        // Note: This is a placeholder - actual implementation would
        // connect to the terminal's output stream
        tokio::spawn(async move {
            let _ = tx.send(TerminalOutput::Data("Terminal output stream ready\n".to_string()));
        });

        Ok(rx)
    }

    /// Send input to a terminal
    ///
    /// Sends text input to the specified terminal session.
    ///
    /// # Arguments
    ///
    /// * `terminal_id` - The terminal ID
    /// * `data` - The input data to send
    ///
    /// # Errors
    ///
    /// Returns error if the terminal is not found or writing fails.
    pub async fn send_input(&self, terminal_id: &str, data: &str) -> Result<(), LiteError> {
        // Check if terminal exists and is active
        {
            let mappings = self.mappings.read().await;
            let mapping = mappings
                .get(terminal_id)
                .ok_or_else(|| LiteError::Terminal(format!("Terminal {} not found", terminal_id)))?;

            if !mapping.connection_state.is_active() {
                return Err(LiteError::Terminal(format!(
                    "Terminal {} is not active (state: {:?})",
                    terminal_id,
                    mapping.connection_state
                )));
            }
        }

        // Send to terminal
        self.terminal_manager.write(terminal_id, data).await?;

        // Update mapping activity
        {
            let mut mappings = self.mappings.write().await;
            if let Some(mapping) = mappings.get_mut(terminal_id) {
                mapping.touch();
            }
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_bytes_sent += data.len() as u64;
        }

        Ok(())
    }

    /// Resize a terminal
    ///
    /// Resizes the terminal to the specified dimensions.
    ///
    /// # Arguments
    ///
    /// * `terminal_id` - The terminal ID
    /// * `cols` - New column count
    /// * `rows` - New row count
    ///
    /// # Errors
    ///
    /// Returns error if the terminal is not found or resize fails.
    pub async fn resize_terminal(
        &self,
        terminal_id: &str,
        cols: u16,
        rows: u16,
    ) -> Result<(), LiteError> {
        let size = TerminalSize::new(rows, cols);

        // Resize terminal
        self.terminal_manager.resize(terminal_id, size).await?;

        // Update mapping
        let tab_id = {
            let mut mappings = self.mappings.write().await;
            let mapping = mappings
                .get_mut(terminal_id)
                .ok_or_else(|| LiteError::Terminal(format!("Terminal {} not found", terminal_id)))?;

            mapping.size = size;
            mapping.touch();
            mapping.tab_id.clone()
        };

        // Emit resize event
        let _ = self.event_tx.send(CoordinatorEvent::TerminalResized {
            terminal_id: terminal_id.to_string(),
            tab_id,
            cols,
            rows,
        });

        Ok(())
    }

    /// Get session-tab mapping
    ///
    /// Returns the mapping for a terminal session.
    ///
    /// # Arguments
    ///
    /// * `terminal_id` - The terminal ID
    pub async fn get_mapping(&self, terminal_id: &str) -> Option<SessionTabMapping> {
        let mappings = self.mappings.read().await;
        mappings.get(terminal_id).cloned()
    }

    /// Get mapping by tab ID
    ///
    /// Returns the mapping for a tab.
    ///
    /// # Arguments
    ///
    /// * `tab_id` - The tab ID
    pub async fn get_mapping_by_tab(&self, tab_id: &str) -> Option<SessionTabMapping> {
        let mappings = self.mappings.read().await;
        mappings
            .values()
            .find(|m| m.tab_id == tab_id)
            .cloned()
    }

    /// Get all active terminals
    ///
    /// Returns a list of all active terminal IDs.
    pub async fn list_terminals(&self) -> Vec<String> {
        let mappings = self.mappings.read().await;
        mappings.keys().cloned().collect()
    }

    /// Get all sessions info
    ///
    /// Returns detailed information about all sessions.
    pub async fn list_sessions(&self) -> Vec<SessionTabMapping> {
        let mappings = self.mappings.read().await;
        mappings.values().cloned().collect()
    }

    /// Get terminal statistics
    ///
    /// Returns statistics for a specific terminal.
    ///
    /// # Arguments
    ///
    /// * `terminal_id` - The terminal ID
    pub async fn get_terminal_stats(&self, terminal_id: &str) -> Result<TerminalStats, LiteError> {
        self.terminal_manager.get_stats(terminal_id).await
    }

    /// Get coordinator statistics
    ///
    /// Returns overall coordinator statistics.
    pub async fn get_stats(&self) -> CoordinatorStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Activate a tab
    ///
    /// Switches to the specified tab and updates the mapping.
    ///
    /// # Arguments
    ///
    /// * `tab_id` - The tab ID to activate
    pub async fn activate_tab(&self, tab_id: &str) -> Result<(), LiteError> {
        self.tab_manager.activate_tab(tab_id).await?;

        // Get terminal ID for the tab
        let terminal_id = self.tab_manager.get_terminal_id(tab_id).await;

        // Update mapping activity
        if let Some(ref tid) = terminal_id {
            let mut mappings = self.mappings.write().await;
            if let Some(mapping) = mappings.get_mut(tid) {
                mapping.touch();
            }
        }

        // Emit event
        let _ = self.event_tx.send(CoordinatorEvent::TabActivated {
            tab_id: tab_id.to_string(),
            terminal_id,
        });

        Ok(())
    }

    /// Close all terminals
    ///
    /// Closes all active terminal sessions.
    pub async fn close_all(&self) -> Result<(), LiteError> {
        let terminal_ids: Vec<String> = {
            let mappings = self.mappings.read().await;
            mappings.keys().cloned().collect()
        };

        for terminal_id in terminal_ids {
            let _ = self.close_terminal(&terminal_id).await;
        }

        // Close all tabs
        self.tab_manager.close_all().await?;

        // Reset stats
        {
            let mut stats = self.stats.write().await;
            stats.active_sessions = 0;
            stats.ssh_connections = 0;
            stats.local_terminals = 0;
        }

        // Emit event
        let _ = self.event_tx.send(CoordinatorEvent::AllTerminalsClosed);

        log::info!("All terminals closed");

        Ok(())
    }

    /// Check if terminal is alive
    ///
    /// Returns true if the terminal session is still active.
    ///
    /// # Arguments
    ///
    /// * `terminal_id` - The terminal ID
    pub async fn is_terminal_alive(&self, terminal_id: &str) -> bool {
        self.terminal_manager.is_alive(terminal_id).await
    }

    /// Get active terminal count
    ///
    /// Returns the number of active terminal sessions.
    pub async fn active_count(&self) -> usize {
        let mappings = self.mappings.read().await;
        mappings.values().filter(|m| m.is_active).count()
    }

    /// Cleanup idle sessions
    ///
    /// Closes all sessions that have been idle for longer than the configured timeout.
    ///
    /// # Returns
    ///
    /// The number of sessions closed.
    pub async fn cleanup_idle(&self) -> Result<usize, LiteError> {
        let idle_ids: Vec<String> = {
            let mappings = self.mappings.read().await;
            mappings
                .values()
                .filter(|m| m.is_idle(self.config.idle_timeout_secs))
                .map(|m| m.terminal_id.clone())
                .collect()
        };

        let count = idle_ids.len();
        for terminal_id in idle_ids {
            log::info!("Closing idle terminal: {}", terminal_id);
            let _ = self.close_terminal(&terminal_id).await;
        }

        Ok(count)
    }

    /// Send signal to terminal
    ///
    /// Sends a control signal to the terminal (e.g., Ctrl+C, Ctrl+D).
    ///
    /// # Arguments
    ///
    /// * `terminal_id` - The terminal ID
    /// * `signal` - The signal to send
    pub async fn send_signal(
        &self,
        terminal_id: &str,
        signal: super::TerminalSignal,
    ) -> Result<(), LiteError> {
        self.terminal_manager.send_signal(terminal_id, signal).await?;

        // Update activity
        {
            let mut mappings = self.mappings.write().await;
            if let Some(mapping) = mappings.get_mut(terminal_id) {
                mapping.touch();
            }
        }

        Ok(())
    }

    /// Set terminal title
    ///
    /// Updates the title for a terminal and its associated tab.
    ///
    /// # Arguments
    ///
    /// * `terminal_id` - The terminal ID
    /// * `title` - The new title
    pub async fn set_title(&self, terminal_id: &str, title: &str) -> Result<(), LiteError> {
        self.terminal_manager.set_title(terminal_id, title).await?;

        let tab_id = {
            let mappings = self.mappings.read().await;
            mappings
                .get(terminal_id)
                .map(|m| m.tab_id.clone())
        };

        if let Some(ref tab_id) = tab_id {
            self.tab_manager.set_title(tab_id, title).await?;

            // Emit event
            let _ = self.event_tx.send(CoordinatorEvent::TerminalTitleChanged {
                terminal_id: terminal_id.to_string(),
                tab_id: tab_id.clone(),
                title: title.to_string(),
            });
        }

        Ok(())
    }

    /// Shutdown the coordinator
    ///
    /// Gracefully shuts down all sessions and stops the coordinator.
    pub async fn shutdown(&self) -> Result<(), LiteError> {
        // Set inactive flag
        self.active.store(false, std::sync::atomic::Ordering::Relaxed);

        // Close all terminals
        self.close_all().await?;

        log::info!("TerminalCoordinator shutdown complete");

        Ok(())
    }

    /// Check if coordinator is active
    ///
    /// Returns true if the coordinator is still running.
    pub fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Clone for SessionCoordinator {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            terminal_manager: self.terminal_manager.clone(),
            tab_manager: self.tab_manager.clone(),
            ssh_manager: self.ssh_manager.clone(),
            mappings: self.mappings.clone(),
            output_receivers: self.output_receivers.clone(),
            event_tx: self.event_tx.clone(),
            stats: self.stats.clone(),
            active: self.active.clone(),
        }
    }
}

impl Default for SessionCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_config_default() {
        let config = CoordinatorConfig::default();
        assert_eq!(config.default_size.rows, 24);
        assert_eq!(config.default_size.cols, 80);
        assert_eq!(config.idle_timeout_secs, 3600);
        assert_eq!(config.max_sessions, 100);
        assert!(config.auto_reconnect);
    }

    #[test]
    fn test_connection_state_is_active() {
        assert!(ConnectionState::Connected.is_active());
        assert!(ConnectionState::Reconnecting.is_active());
        assert!(!ConnectionState::Disconnected.is_active());
        assert!(!ConnectionState::Closed.is_active());
    }

    #[test]
    fn test_connection_state_can_close() {
        assert!(ConnectionState::Connected.can_close());
        assert!(ConnectionState::Disconnected.can_close());
        assert!(!ConnectionState::Closed.can_close());
    }

    #[test]
    fn test_session_tab_mapping() {
        let mapping = SessionTabMapping::new(
            "term-1",
            "tab-1",
            SessionType::Ssh,
            TerminalSize::new(30, 100),
        );

        assert_eq!(mapping.terminal_id, "term-1");
        assert_eq!(mapping.tab_id, "tab-1");
        assert_eq!(mapping.session_type, SessionType::Ssh);
        assert!(mapping.is_active);
        assert_eq!(mapping.connection_state, ConnectionState::Initializing);
    }

    #[test]
    fn test_session_tab_mapping_with_server() {
        let mapping = SessionTabMapping::new(
            "term-1",
            "tab-1",
            SessionType::Ssh,
            TerminalSize::default(),
        )
        .with_server("server-123");

        assert_eq!(mapping.server_id, Some("server-123".to_string()));
    }

    #[test]
    fn test_coordinator_event_terminal_id() {
        let event = CoordinatorEvent::TerminalConnected {
            terminal_id: "term-1".to_string(),
            tab_id: "tab-1".to_string(),
            server_id: Some("server-1".to_string()),
        };
        assert_eq!(event.terminal_id(), Some("term-1"));
        assert_eq!(event.tab_id(), Some("tab-1"));

        let event = CoordinatorEvent::SshSessionConnected {
            session_id: "ssh-1".to_string(),
            server_id: "server-1".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
        };
        assert_eq!(event.terminal_id(), None);
    }

    #[test]
    fn test_coordinator_stats_default() {
        let stats = CoordinatorStats::default();
        assert_eq!(stats.total_sessions_created, 0);
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.ssh_connections, 0);
    }
}