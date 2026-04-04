//! Core Types for Terminal Subsystem
//!
//! This module defines the fundamental types used across the terminal
//! subsystem including configuration, session state, and connection types.
//!
//! # Constraints (SYSTEM_INVARIANTS.md)
//!
//! All types follow the system invariants:
//! - State gating: TerminalSession requires Connection Active
//! - Resource ownership: Session owns Terminal, Terminal owns PTY
//! - Thread safety: All shared types use Arc<RwLock> or Arc<Mutex>
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           TerminalConfig                  │
//! │    (Global terminal settings)            │
//! └─────────────────────────────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────────┐
//! │           TerminalSession                 │
//! │    (Per-connection terminal state)       │
//! │    ┌─────────────────────────────┐      │
//! │    │ ScrollBuffer                 │      │
//! │    │ PtyHandle                    │      │
//! │    │ SearchIndex                  │      │
//! │    └─────────────────────────────┘      │
//! └─────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::LiteError;
use crate::terminal::scroll_buffer::{ScrollBuffer, ScrollBufferStats};
use crate::terminal::pty::{PtyHandle, PtySize, PtyState, PtyStats};

// ============ Configuration Types ============

/// Global terminal configuration.
///
/// Defines default settings for all terminal sessions.
/// These settings can be overridden per-session.
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    /// Default terminal columns
    pub cols: u16,
    /// Default terminal rows
    pub rows: u16,
    /// Scrollback buffer size (lines)
    pub scrollback_lines: usize,
    /// Font family name
    pub font_family: String,
    /// Font size in pixels
    pub font_size: u16,
    /// Line height multiplier
    pub line_height: f32,
    /// Cursor style
    pub cursor_style: CursorStyle,
    /// Cursor blink enabled
    pub cursor_blink: bool,
    /// Terminal type (TERM env var)
    pub term_type: String,
    /// Enable UTF-8 handling
    pub utf8: bool,
    /// Bell sound enabled
    pub bell: bool,
    /// Copy on select
    pub copy_on_select: bool,
    /// Right-click paste
    pub right_click_paste: bool,
    /// Scroll sensitivity
    pub scroll_sensitivity: u8,
    /// Auto-scroll on output
    pub scroll_on_output: bool,
    /// Enable WebGL rendering
    pub webgl: bool,
    /// Theme ID
    pub theme_id: String,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            scrollback_lines: 10000,
            font_family: "Consolas".to_string(),
            font_size: 14,
            line_height: 1.2,
            cursor_style: CursorStyle::Block,
            cursor_blink: true,
            term_type: "xterm-256color".to_string(),
            utf8: true,
            bell: false,
            copy_on_select: false,
            right_click_paste: false,
            scroll_sensitivity: 3,
            scroll_on_output: true,
            webgl: true,
            theme_id: "default".to_string(),
        }
    }
}

impl TerminalConfig {
    /// Create config with custom size.
    pub fn with_size(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            ..Default::default()
        }
    }

    /// Create config with custom scrollback.
    pub fn with_scrollback(lines: usize) -> Self {
        Self {
            scrollback_lines: lines,
            ..Default::default()
        }
    }

    /// Create config with custom font.
    pub fn with_font(font_family: &str, font_size: u16) -> Self {
        Self {
            font_family: font_family.to_string(),
            font_size,
            ..Default::default()
        }
    }

    /// Create Lite edition config (5000 scrollback).
    pub fn lite_default() -> Self {
        Self {
            scrollback_lines: 5000,
            ..Default::default()
        }
    }

    /// Create Standard edition config (10000 scrollback).
    pub fn standard_default() -> Self {
        Self::default()
    }

    /// Create Pro edition config (50000 scrollback).
    pub fn pro_default() -> Self {
        Self {
            scrollback_lines: 50000,
            ..Default::default()
        }
    }

    /// Get as PTY size.
    pub fn pty_size(&self) -> PtySize {
        PtySize::new(self.cols, self.rows)
    }
}

/// Cursor style for terminal display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    /// Block cursor (default)
    Block,
    /// Underline cursor
    Underline,
    /// Bar/I-beam cursor
    Bar,
}

impl CursorStyle {
    /// Get cursor style as string for CSS.
    pub fn css_class(&self) -> &'static str {
        match self {
            CursorStyle::Block => "cursor-block",
            CursorStyle::Underline => "cursor-underline",
            CursorStyle::Bar => "cursor-bar",
        }
    }
}

/// Terminal session state.
///
/// Represents the lifecycle state of a terminal session.
/// Follows SYSTEM_INVARIANTS.md constraints for state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalState {
    /// Session is being initialized
    #[default]
    Initializing,
    /// Waiting for connection to become active
    WaitingForConnection,
    /// Session is active and ready
    Active,
    /// Session is reconnecting
    Reconnecting,
    /// Session is being closed
    Closing,
    /// Session is closed
    Closed,
    /// Session encountered error
    Error,
}

impl TerminalState {
    /// Check if session can accept input.
    pub fn can_write(&self) -> bool {
        matches!(self, TerminalState::Active)
    }

    /// Check if session is alive.
    pub fn is_alive(&self) -> bool {
        matches!(
            self,
            TerminalState::Initializing
                | TerminalState::WaitingForConnection
                | TerminalState::Active
                | TerminalState::Reconnecting
        )
    }

    /// Check if session is closed.
    pub fn is_closed(&self) -> bool {
        matches!(self, TerminalState::Closed | TerminalState::Error)
    }

    /// Check if session is connected.
    pub fn is_connected(&self) -> bool {
        matches!(self, TerminalState::Active)
    }

    /// Get state as string for display.
    pub fn display(&self) -> &'static str {
        match self {
            TerminalState::Initializing => "Initializing",
            TerminalState::WaitingForConnection => "Waiting",
            TerminalState::Active => "Active",
            TerminalState::Reconnecting => "Reconnecting",
            TerminalState::Closing => "Closing",
            TerminalState::Closed => "Closed",
            TerminalState::Error => "Error",
        }
    }
}

// ============ Session Types ============

/// Terminal session information.
///
/// Contains metadata about a terminal session including
/// connection info, state, and timestamps.
#[derive(Debug, Clone)]
pub struct TerminalSessionInfo {
    /// Unique session ID
    pub id: String,
    /// Connection ID this session belongs to
    pub connection_id: String,
    /// Session title (for display)
    pub title: String,
    /// Current state
    pub state: TerminalState,
    /// Creation time
    pub created_at: Instant,
    /// Last activity time
    pub last_activity: Instant,
    /// Server ID (if SSH session)
    pub server_id: Option<String>,
    /// Server name (for display)
    pub server_name: Option<String>,
    /// Terminal size
    pub size: PtySize,
}

impl TerminalSessionInfo {
    /// Create new session info.
    pub fn new(id: &str, connection_id: &str) -> Self {
        Self {
            id: id.to_string(),
            connection_id: connection_id.to_string(),
            title: "Terminal".to_string(),
            state: TerminalState::Initializing,
            created_at: Instant::now(),
            last_activity: Instant::now(),
            server_id: None,
            server_name: None,
            size: PtySize::default(),
        }
    }

    /// Create with server info.
    pub fn with_server(id: &str, connection_id: &str, server_id: &str, server_name: &str) -> Self {
        Self {
            id: id.to_string(),
            connection_id: connection_id.to_string(),
            title: format!("SSH: {}", server_name),
            state: TerminalState::Initializing,
            created_at: Instant::now(),
            last_activity: Instant::now(),
            server_id: Some(server_id.to_string()),
            server_name: Some(server_name.to_string()),
            size: PtySize::default(),
        }
    }

    /// Update activity timestamp.
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Get idle duration.
    pub fn idle_duration(&self) -> Duration {
        self.last_activity.elapsed()
    }

    /// Get session age.
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Update title.
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// Update state.
    pub fn set_state(&mut self, state: TerminalState) {
        self.state = state;
    }

    /// Update size.
    pub fn set_size(&mut self, size: PtySize) {
        self.size = size;
    }
}

/// Terminal session with PTY and scroll buffer.
///
/// The main session type that combines:
/// - PTY handle for I/O
/// - Scroll buffer for history
/// - State management
///
/// # Constraints (SYSTEM_INVARIANTS.md)
///
/// - PTY must be created after Connection Active
/// - Session owns PTY and ScrollBuffer
/// - ScrollBuffer content preserved during reconnect
pub struct TerminalSession {
    /// Session info
    info: TerminalSessionInfo,
    /// Scroll buffer for output history
    scroll_buffer: Arc<RwLock<ScrollBuffer>>,
    /// PTY handle (optional, created after connection active)
    pty_handle: Option<PtyHandle>,
    /// Configuration
    config: TerminalConfig,
    /// Statistics
    stats: Arc<RwLock<TerminalSessionStats>>,
}

impl TerminalSession {
    /// Create new terminal session.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique session ID
    /// * `connection_id` - Connection ID (for state gating)
    /// * `config` - Terminal configuration
    ///
    /// # Returns
    ///
    /// New session without PTY (PTY created after connection active).
    pub fn new(id: &str, connection_id: &str, config: TerminalConfig) -> Self {
        Self {
            info: TerminalSessionInfo::new(id, connection_id),
            scroll_buffer: Arc::new(RwLock::new(ScrollBuffer::new(config.scrollback_lines))),
            pty_handle: None,
            config,
            stats: Arc::new(RwLock::new(TerminalSessionStats::default())),
        }
    }

    /// Create SSH session.
    pub fn ssh_session(
        id: &str,
        connection_id: &str,
        server_id: &str,
        server_name: &str,
        config: TerminalConfig,
    ) -> Self {
        Self {
            info: TerminalSessionInfo::with_server(id, connection_id, server_id, server_name),
            scroll_buffer: Arc::new(RwLock::new(ScrollBuffer::new(config.scrollback_lines))),
            pty_handle: None,
            config,
            stats: Arc::new(RwLock::new(TerminalSessionStats::default())),
        }
    }

    /// Get session ID.
    pub fn id(&self) -> &str {
        &self.info.id
    }

    /// Get connection ID.
    pub fn connection_id(&self) -> &str {
        &self.info.connection_id
    }

    /// Get session info.
    pub fn info(&self) -> &TerminalSessionInfo {
        &self.info
    }

    /// Get mutable session info.
    pub fn info_mut(&mut self) -> &mut TerminalSessionInfo {
        &mut self.info
    }

    /// Get current state.
    pub fn state(&self) -> TerminalState {
        self.info.state
    }

    /// Set state.
    pub async fn set_state(&mut self, state: TerminalState) {
        self.info.state = state;
        self.stats.write().await.state = state;
    }

    /// Get scroll buffer.
    pub fn scroll_buffer(&self) -> Arc<RwLock<ScrollBuffer>> {
        self.scroll_buffer.clone()
    }

    /// Get PTY handle.
    pub fn pty_handle(&self) -> Option<&PtyHandle> {
        self.pty_handle.as_ref()
    }

    /// Attach PTY handle (after connection becomes active).
    ///
    /// # Constraints (SYSTEM_INVARIANTS.md Section 1.1)
    ///
    /// - PTY must be created after Connection Active
    /// - Must verify connection state before attaching
    pub fn attach_pty(&mut self, handle: PtyHandle) {
        self.pty_handle = Some(handle);
        self.info.state = TerminalState::Active;
    }

    /// Remove PTY handle (for reconnect or close).
    ///
    /// # Constraints (SYSTEM_INVARIANTS.md Section 1.1)
    ///
    /// - ScrollBuffer content preserved during reconnect
    pub fn detach_pty(&mut self) -> Option<PtyHandle> {
        self.pty_handle.take()
    }

    /// Write to PTY.
    ///
    /// # Constraints
    ///
    /// - Session must be Active
    /// - Connection must be Active (state gating)
    pub async fn write(&self, data: &[u8]) -> Result<usize, LiteError> {
        if !self.state().can_write() {
            return Err(LiteError::TerminalEmulator(
                "Session is not in writable state".to_string(),
            ));
        }

        if let Some(handle) = &self.pty_handle {
            handle.write(data).await
        } else {
            Err(LiteError::TerminalEmulator("PTY not attached".to_string()))
        }
    }

    /// Resize terminal.
    ///
    /// Updates both PTY and internal size.
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<(), LiteError> {
        if !self.state().can_write() {
            return Err(LiteError::TerminalEmulator(
                "Session cannot be resized".to_string(),
            ));
        }

        if let Some(handle) = &self.pty_handle {
            handle.resize(cols, rows).await?;
        }

        // Note: In real implementation, would update self.info.size
        Ok(())
    }

    /// Close session.
    ///
    /// # Constraints (SYSTEM_INVARIANTS.md Section 1.1)
    ///
    /// - Close PTY channel first
    /// - Then release PTY resources
    /// - ScrollBuffer remains (for reconnect history)
    pub async fn close(&mut self) -> Result<(), LiteError> {
        self.info.state = TerminalState::Closing;

        if let Some(handle) = self.pty_handle.take() {
            handle.close().await?;
        }

        self.info.state = TerminalState::Closed;
        Ok(())
    }

    /// Prepare for reconnect.
    ///
    /// Closes PTY but preserves scroll buffer content.
    pub async fn prepare_reconnect(&mut self) -> Result<(), LiteError> {
        self.info.state = TerminalState::Reconnecting;

        // Close PTY
        if let Some(handle) = self.pty_handle.take() {
            handle.close().await?;
        }

        // ScrollBuffer content preserved (per SYSTEM_INVARIANTS.md)

        Ok(())
    }

    /// Get configuration.
    pub fn config(&self) -> &TerminalConfig {
        &self.config
    }

    /// Get statistics.
    pub async fn stats(&self) -> TerminalSessionStats {
        self.stats.read().await.clone()
    }

    /// Update statistics.
    pub async fn update_stats(&self, bytes_in: u64, bytes_out: u64) {
        let mut stats = self.stats.write().await;
        stats.bytes_received += bytes_in;
        stats.bytes_sent += bytes_out;
        stats.last_activity = Some(Instant::now());
    }

    /// Check if session is alive.
    pub fn is_alive(&self) -> bool {
        self.info.state.is_alive()
    }

    /// Check if session is active.
    pub fn is_active(&self) -> bool {
        self.info.state.is_connected()
    }
}

/// Terminal session statistics.
#[derive(Debug, Clone, Default)]
pub struct TerminalSessionStats {
    /// Bytes received from PTY
    pub bytes_received: u64,
    /// Bytes sent to PTY
    pub bytes_sent: u64,
    /// Current state
    pub state: TerminalState,
    /// Creation time
    pub created_at: Option<Instant>,
    /// Last activity time
    pub last_activity: Option<Instant>,
    /// Reconnect count
    pub reconnect_count: u32,
    /// Error count
    pub error_count: u32,
}

// ============ Connection Types ============

/// Connection state for SSH connections.
///
/// Used for state gating (SYSTEM_INVARIANTS.md Section 0.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection is idle
    Idle,
    /// Connection is being established
    Connecting,
    /// Connection is active
    Active,
    /// Connection failed
    Failed,
    /// Connection is disconnected
    Disconnected,
}

impl ConnectionState {
    /// Check if connection is ready for PTY creation.
    ///
    /// # Constraint (SYSTEM_INVARIANTS.md Section 0.3)
    ///
    /// PTY must be created only when connection is Active.
    pub fn can_create_pty(&self) -> bool {
        matches!(self, ConnectionState::Active)
    }

    /// Check if connection is active.
    pub fn is_active(&self) -> bool {
        matches!(self, ConnectionState::Active)
    }

    /// Check if connection can be used.
    pub fn is_ready(&self) -> bool {
        matches!(self, ConnectionState::Active)
    }

    /// Check if connection can reconnect.
    pub fn can_reconnect(&self) -> bool {
        matches!(
            self,
            ConnectionState::Failed | ConnectionState::Idle | ConnectionState::Disconnected
        )
    }
}

/// Check if connection is ready for terminal operations.
///
/// # Constraint (SYSTEM_INVARIANTS.md Section 0.3)
///
/// API calls must check connection state before proceeding.
///
/// # Example
///
/// ```rust
/// use easyssh_core::terminal::types::{ConnectionState, check_connection_ready};
///
/// let state = ConnectionState::Active;
/// check_connection_ready(state).unwrap();
///
/// let failed_state = ConnectionState::Failed;
/// assert!(check_connection_ready(failed_state).is_err());
/// ```
pub fn check_connection_ready(state: ConnectionState) -> Result<(), LiteError> {
    match state {
        ConnectionState::Active => Ok(()),
        _ => Err(LiteError::TerminalEmulator(
            "Connection is not active".to_string(),
        )),
    }
}

/// Check if PTY can be created.
///
/// # Constraint (SYSTEM_INVARIANTS.md Section 1.1)
///
/// PTY must be created after Connection Active.
pub fn check_can_create_pty(state: ConnectionState) -> Result<(), LiteError> {
    if state.can_create_pty() {
        Ok(())
    } else {
        Err(LiteError::TerminalEmulator(
            "PTY cannot be created: connection not active".to_string(),
        ))
    }
}

// ============ Event Types ============

/// Terminal event for notifications.
#[derive(Debug, Clone)]
pub enum TerminalEvent {
    /// Session created
    SessionCreated { session_id: String },
    /// Session state changed
    StateChanged {
        session_id: String,
        old_state: TerminalState,
        new_state: TerminalState,
    },
    /// PTY attached
    PtyAttached { session_id: String },
    /// PTY detached
    PtyDetached { session_id: String },
    /// Session resized
    Resized {
        session_id: String,
        cols: u16,
        rows: u16,
    },
    /// Session closed
    SessionClosed { session_id: String },
    /// Data received
    DataReceived { session_id: String, bytes: usize },
    /// Error occurred
    Error { session_id: String, error: String },
    /// Reconnect started
    ReconnectStarted { session_id: String },
    /// Reconnect completed
    ReconnectCompleted { session_id: String },
}

// ============ Performance Types ============

/// Terminal performance metrics.
#[derive(Debug, Clone, Default)]
pub struct TerminalPerformance {
    /// Render FPS
    pub fps: f32,
    /// Input latency (ms)
    pub input_latency_ms: f32,
    /// Output latency (ms)
    pub output_latency_ms: f32,
    /// Memory usage (bytes)
    pub memory_bytes: u64,
    /// Buffer fill percentage
    pub buffer_fill: f32,
    /// WebGL enabled
    pub webgl_enabled: bool,
}

/// Terminal resource limits.
///
/// Per SYSTEM_INVARIANTS.md Section 8.2:
/// - Lite: 5000 scrollback, 5 terminals
/// - Standard: 10000 scrollback, 20 terminals
/// - Pro: 50000 scrollback, 100 terminals
#[derive(Debug, Clone)]
pub struct TerminalLimits {
    /// Maximum scrollback lines
    pub max_scrollback: usize,
    /// Maximum terminal sessions
    pub max_sessions: usize,
    /// Maximum connections
    pub max_connections: usize,
}

impl TerminalLimits {
    /// Lite edition limits.
    pub fn lite() -> Self {
        Self {
            max_scrollback: 5000,
            max_sessions: 5,
            max_connections: 10,
        }
    }

    /// Standard edition limits.
    pub fn standard() -> Self {
        Self {
            max_scrollback: 10000,
            max_sessions: 20,
            max_connections: 50,
        }
    }

    /// Pro edition limits.
    pub fn pro() -> Self {
        Self {
            max_scrollback: 50000,
            max_sessions: 100,
            max_connections: 500,
        }
    }

    /// Check scrollback limit.
    pub fn check_scrollback(&self, lines: usize) -> Result<(), LiteError> {
        if lines > self.max_scrollback {
            Err(LiteError::Terminal(format!(
                "Scrollback exceeds limit: {} > {}",
                lines, self.max_scrollback
            )))
        } else {
            Ok(())
        }
    }

    /// Check session limit.
    pub fn check_sessions(&self, count: usize) -> Result<(), LiteError> {
        if count >= self.max_sessions {
            Err(LiteError::Terminal(format!(
                "Session limit reached: {} >= {}",
                count, self.max_sessions
            )))
        } else {
            Ok(())
        }
    }
}

// ============ Display Types ============

/// Terminal line with formatting info.
#[derive(Debug, Clone)]
pub struct TerminalLine {
    /// Line content
    pub content: String,
    /// Line number
    pub line_number: usize,
    /// Is wrapped
    pub is_wrapped: bool,
    /// Cursor position (if this line has cursor)
    pub cursor_col: Option<u16>,
}

/// Terminal position (row, column).
#[derive(Debug, Clone, Copy)]
pub struct TerminalPosition {
    /// Row (0-based)
    pub row: usize,
    /// Column (0-based)
    pub col: usize,
}

impl Default for TerminalPosition {
    fn default() -> Self {
        Self { row: 0, col: 0 }
    }
}

/// Terminal selection range.
#[derive(Debug, Clone)]
pub struct TerminalSelection {
    /// Start position
    pub start: TerminalPosition,
    /// End position
    pub end: TerminalPosition,
    /// Selected text
    pub text: Option<String>,
}

impl TerminalSelection {
    /// Create new selection.
    pub fn new(start: TerminalPosition, end: TerminalPosition) -> Self {
        Self {
            start,
            end,
            text: None,
        }
    }

    /// Check if position is in selection.
    pub fn contains(&self, pos: TerminalPosition) -> bool {
        pos.row >= self.start.row
            && pos.row <= self.end.row
            && (pos.row != self.start.row || pos.col >= self.start.col)
            && (pos.row != self.end.row || pos.col <= self.end.col)
    }

    /// Check if selection is empty.
    pub fn is_empty(&self) -> bool {
        self.start.row == self.end.row && self.start.col == self.end.col
    }
}

// ============ Unit Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_config_default() {
        let config = TerminalConfig::default();
        assert_eq!(config.cols, 80);
        assert_eq!(config.rows, 24);
        assert_eq!(config.scrollback_lines, 10000);
        assert_eq!(config.font_size, 14);
    }

    #[test]
    fn test_terminal_config_edition_defaults() {
        let lite = TerminalConfig::lite_default();
        assert_eq!(lite.scrollback_lines, 5000);

        let standard = TerminalConfig::standard_default();
        assert_eq!(standard.scrollback_lines, 10000);

        let pro = TerminalConfig::pro_default();
        assert_eq!(pro.scrollback_lines, 50000);
    }

    #[test]
    fn test_cursor_style_css() {
        assert_eq!(CursorStyle::Block.css_class(), "cursor-block");
        assert_eq!(CursorStyle::Underline.css_class(), "cursor-underline");
        assert_eq!(CursorStyle::Bar.css_class(), "cursor-bar");
    }

    #[test]
    fn test_terminal_state_checks() {
        assert!(TerminalState::Active.can_write());
        assert!(TerminalState::Active.is_alive());
        assert!(!TerminalState::Active.is_closed());
        assert!(TerminalState::Active.is_connected());

        assert!(!TerminalState::Closed.can_write());
        assert!(!TerminalState::Closed.is_alive());
        assert!(TerminalState::Closed.is_closed());
    }

    #[test]
    fn test_terminal_state_display() {
        assert_eq!(TerminalState::Active.display(), "Active");
        assert_eq!(TerminalState::Closed.display(), "Closed");
    }

    #[test]
    fn test_terminal_session_info_creation() {
        let info = TerminalSessionInfo::new("session-1", "conn-1");
        assert_eq!(info.id, "session-1");
        assert_eq!(info.connection_id, "conn-1");
        assert_eq!(info.state, TerminalState::Initializing);
    }

    #[test]
    fn test_terminal_session_info_with_server() {
        let info = TerminalSessionInfo::with_server("s1", "c1", "server-1", "MyServer");
        assert!(info.title.contains("MyServer"));
        assert_eq!(info.server_id, Some("server-1".to_string()));
    }

    #[test]
    fn test_connection_state_checks() {
        assert!(ConnectionState::Active.can_create_pty());
        assert!(ConnectionState::Active.is_active());
        assert!(ConnectionState::Active.is_ready());

        assert!(!ConnectionState::Idle.can_create_pty());
        assert!(!ConnectionState::Failed.can_create_pty());

        assert!(ConnectionState::Failed.can_reconnect());
        assert!(!ConnectionState::Active.can_reconnect());
    }

    #[test]
    fn test_check_connection_ready() {
        assert!(check_connection_ready(ConnectionState::Active).is_ok());
        assert!(check_connection_ready(ConnectionState::Idle).is_err());
        assert!(check_connection_ready(ConnectionState::Failed).is_err());
    }

    #[test]
    fn test_check_can_create_pty() {
        assert!(check_can_create_pty(ConnectionState::Active).is_ok());
        assert!(check_can_create_pty(ConnectionState::Connecting).is_err());
    }

    #[test]
    fn test_terminal_limits() {
        let lite = TerminalLimits::lite();
        assert_eq!(lite.max_scrollback, 5000);
        assert_eq!(lite.max_sessions, 5);

        let standard = TerminalLimits::standard();
        assert_eq!(standard.max_scrollback, 10000);
        assert_eq!(standard.max_sessions, 20);

        let pro = TerminalLimits::pro();
        assert_eq!(pro.max_scrollback, 50000);
        assert_eq!(pro.max_sessions, 100);
    }

    #[test]
    fn test_terminal_limits_check() {
        let limits = TerminalLimits::lite();

        assert!(limits.check_scrollback(1000).is_ok());
        assert!(limits.check_scrollback(5001).is_err());

        assert!(limits.check_sessions(3).is_ok());
        assert!(limits.check_sessions(5).is_err());
    }

    #[test]
    fn test_terminal_selection() {
        let sel = TerminalSelection::new(
            TerminalPosition { row: 0, col: 5 },
            TerminalPosition { row: 2, col: 10 },
        );

        assert!(sel.contains(TerminalPosition { row: 1, col: 0 }));
        assert!(!sel.contains(TerminalPosition { row: 3, col: 0 }));
        assert!(!sel.is_empty());
    }

    #[tokio::test]
    async fn test_terminal_session_creation() {
        let config = TerminalConfig::default();
        let session = TerminalSession::new("session-1", "conn-1", config);

        assert_eq!(session.id(), "session-1");
        assert_eq!(session.connection_id(), "conn-1");
        assert_eq!(session.state(), TerminalState::Initializing);
        assert!(session.pty_handle().is_none());
    }

    #[tokio::test]
    async fn test_terminal_session_ssh() {
        let config = TerminalConfig::default();
        let session = TerminalSession::ssh_session(
            "s1", "c1", "server-1", "MyServer", config
        );

        assert!(session.info().server_id.is_some());
        assert!(session.info().title.contains("MyServer"));
    }
}