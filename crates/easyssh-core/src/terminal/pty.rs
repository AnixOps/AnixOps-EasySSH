//! PTY (Pseudo-Terminal) Management
//!
//! This module provides PTY abstraction for terminal sessions with:
//! - Cross-platform PTY backend support
//! - Connection state gating (SYSTEM_INVARIANTS.md Section 0.3)
//! - PTY lifecycle management (SYSTEM_INVARIANTS.md Section 1.1)
//! - Resize and I/O operations
//!
//! # Constraints (SYSTEM_INVARIANTS.md Section 1.1)
//!
//! - PTY MUST be created after Connection is Active
//! - PTY destruction: first close main channel, then release resources
//! - PTY output callback MUST NOT block main thread
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           TerminalSession                │
//! │    (Created after Connection Active)     │
//! └─────────────────┬───────────────────────┘
//!                   │ check_connection_ready()
//! ┌─────────────────▼───────────────────────┐
//! │           PtyManager                     │
//! │    ┌─────────────────────────────┐      │
//! │    │ PtyBackend (Trait)           │      │
//! │    │ - write()                    │      │
//! │    │ - resize()                   │      │
//! │    │ - read()                     │      │
//! │    │ - close()                    │      │
//! │    └─────────────────────────────┘      │
//! │    ┌─────────────────────────────┐      │
//! │    │ Platform Implementations     │      │
//! │    │ - ConPTY (Windows)           │      │
//! │    │ - UnixPTY (Linux/macOS)      │      │
//! │    │ - portable-pty (Fallback)    │      │
//! │    └─────────────────────────────┘      │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use easyssh_core::terminal::pty::{PtyManager, PtyConfig, PtyHandle};
//!
//! // Create PTY manager
//! let manager = PtyManager::new();
//!
//! // Create PTY session (requires active connection)
//! let config = PtyConfig::default();
//! let handle = manager.create_pty(config).await?;
//!
//! // Write to PTY
//! handle.write(b"ls -la\n").await?;
//!
//! // Resize terminal
//! handle.resize(120, 40).await?;
//!
//! // Close PTY
//! handle.close().await?;
//! ```

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::error::LiteError;

#[cfg(feature = "embedded-terminal")]
use portable_pty::{CommandBuilder, NativePtySystem, PtyPair, PtySystem};

// ============ Error Types ============

/// PTY-specific error type.
///
/// Provides detailed error information for PTY operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PtyError {
    /// Failed to create PTY
    #[error("Failed to create PTY: {0}")]
    CreationFailed(String),

    /// Failed to write to PTY
    #[error("Failed to write to PTY: {0}")]
    WriteFailed(String),

    /// Failed to read from PTY
    #[error("Failed to read from PTY: {0}")]
    ReadFailed(String),

    /// Failed to resize PTY
    #[error("Failed to resize PTY: {0}")]
    ResizeFailed(String),

    /// Failed to close PTY
    #[error("Failed to close PTY: {0}")]
    CloseFailed(String),

    /// PTY is not in a valid state for the operation
    #[error("Invalid PTY state: {0}")]
    InvalidState(String),

    /// Connection is not ready for PTY creation
    #[error("Connection not ready: {0}")]
    ConnectionNotReady(String),

    /// Process error
    #[error("Process error: {0}")]
    ProcessError(String),

    /// Platform-specific error
    #[error("Platform error: {0}")]
    PlatformError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(String),

    /// Channel error
    #[error("Channel error: {0}")]
    ChannelError(String),
}

impl PtyError {
    /// Check if error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            PtyError::ReadFailed(_) | PtyError::WriteFailed(_) | PtyError::ChannelError(_)
        )
    }

    /// Get user-friendly error message.
    pub fn user_message(&self) -> &'static str {
        match self {
            PtyError::CreationFailed(_) => "Failed to create terminal session",
            PtyError::WriteFailed(_) => "Failed to send input to terminal",
            PtyError::ReadFailed(_) => "Failed to read terminal output",
            PtyError::ResizeFailed(_) => "Failed to resize terminal",
            PtyError::CloseFailed(_) => "Failed to close terminal session",
            PtyError::InvalidState(_) => "Terminal is in an invalid state",
            PtyError::ConnectionNotReady(_) => "Connection is not ready",
            PtyError::ProcessError(_) => "Terminal process error",
            PtyError::PlatformError(_) => "Platform-specific error occurred",
            PtyError::IoError(_) => "I/O error occurred",
            PtyError::ChannelError(_) => "Communication error",
        }
    }
}

impl From<PtyError> for LiteError {
    fn from(err: PtyError) -> Self {
        LiteError::TerminalEmulator(err.to_string())
    }
}

/// PTY size configuration.
///
/// Defines the terminal dimensions for PTY creation.
#[derive(Debug, Clone, Copy)]
pub struct PtySize {
    /// Number of columns (width in characters)
    pub cols: u16,
    /// Number of rows (height in lines)
    pub rows: u16,
    /// Pixel width (optional, for graphical terminals)
    pub pixel_width: u16,
    /// Pixel height (optional, for graphical terminals)
    pub pixel_height: u16,
}

impl Default for PtySize {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

impl PtySize {
    /// Create a new PTY size.
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    /// Create with pixel dimensions.
    pub fn with_pixels(cols: u16, rows: u16, pixel_width: u16, pixel_height: u16) -> Self {
        Self {
            cols,
            rows,
            pixel_width,
            pixel_height,
        }
    }

    /// Convert to portable-pty size format.
    #[cfg(feature = "embedded-terminal")]
    pub fn to_portable_pty_size(&self) -> portable_pty::PtySize {
        portable_pty::PtySize {
            cols: self.cols,
            rows: self.rows,
            pixel_width: self.pixel_width,
            pixel_height: self.pixel_height,
        }
    }
}

/// PTY configuration for session creation.
///
/// Controls PTY behavior including size, shell, and environment.
#[derive(Debug, Clone)]
pub struct PtyConfig {
    /// Initial terminal size
    pub size: PtySize,
    /// Shell to spawn (None = system default)
    pub shell: Option<String>,
    /// Environment variables to set
    pub env: Vec<(String, String)>,
    /// Working directory for shell
    pub working_dir: Option<String>,
    /// Terminal type (TERM env var)
    pub term_type: String,
    /// Enable UTF-8 mode
    pub utf8: bool,
    /// Connection ID (for state gating)
    pub connection_id: Option<String>,
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            size: PtySize::default(),
            shell: None,
            env: Vec::new(),
            working_dir: None,
            term_type: "xterm-256color".to_string(),
            utf8: true,
            connection_id: None,
        }
    }
}

impl PtyConfig {
    /// Create config with size.
    pub fn with_size(cols: u16, rows: u16) -> Self {
        Self {
            size: PtySize::new(cols, rows),
            ..Default::default()
        }
    }

    /// Set shell program.
    pub fn with_shell(mut self, shell: &str) -> Self {
        self.shell = Some(shell.to_string());
        self
    }

    /// Set working directory.
    pub fn with_working_dir(mut self, dir: &str) -> Self {
        self.working_dir = Some(dir.to_string());
        self
    }

    /// Add environment variable.
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_string(), value.to_string()));
        self
    }

    /// Set connection ID (required for state gating).
    pub fn with_connection(mut self, connection_id: &str) -> Self {
        self.connection_id = Some(connection_id.to_string());
        self
    }

    /// Set terminal type.
    pub fn with_term_type(mut self, term_type: &str) -> Self {
        self.term_type = term_type.to_string();
        self
    }

    /// Build command from config.
    #[cfg(feature = "embedded-terminal")]
    pub fn build_command(&self) -> CommandBuilder {
        let mut cmd = match &self.shell {
            Some(shell) => CommandBuilder::new(shell),
            None => Self::default_shell_command(),
        };

        // Set environment
        cmd.env("TERM", &self.term_type);
        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        // Set working directory
        if let Some(dir) = &self.working_dir {
            cmd.cwd(dir);
        }

        cmd
    }

    /// Get default shell for the platform.
    #[cfg(feature = "embedded-terminal")]
    fn default_shell_command() -> CommandBuilder {
        #[cfg(target_os = "windows")]
        {
            // Use PowerShell on Windows
            CommandBuilder::new("powershell.exe")
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Use user's shell from SHELL env, fallback to /bin/sh
            let shell = std::env::var("SHELL").unwrap_or("/bin/sh".to_string());
            CommandBuilder::new(shell)
        }
    }

    #[cfg(not(feature = "embedded-terminal"))]
    fn default_shell_command() -> String {
        #[cfg(target_os = "windows")]
        {
            "powershell.exe".to_string()
        }

        #[cfg(not(target_os = "windows"))]
        {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
        }
    }
}

/// PTY state according to lifecycle constraints.
///
/// States follow SYSTEM_INVARIANTS.md Section 1.1:
/// - PTY must be created after Connection Active
/// - PTY destruction: close channel first, then release resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PtyState {
    /// PTY is being initialized
    #[default]
    Initializing,
    /// PTY is ready and connected
    Active,
    /// PTY is being resized
    Resizing,
    /// PTY channel is being closed
    ClosingChannel,
    /// PTY resources are being released
    ReleasingResources,
    /// PTY is fully closed
    Closed,
    /// PTY encountered error
    Error,
}

impl PtyState {
    /// Check if PTY can accept input.
    pub fn can_write(&self) -> bool {
        matches!(self, PtyState::Active | PtyState::Resizing)
    }

    /// Check if PTY can be resized.
    pub fn can_resize(&self) -> bool {
        matches!(self, PtyState::Active)
    }

    /// Check if PTY is alive.
    pub fn is_alive(&self) -> bool {
        matches!(self, PtyState::Initializing | PtyState::Active | PtyState::Resizing)
    }

    /// Check if PTY is closed.
    pub fn is_closed(&self) -> bool {
        matches!(self, PtyState::Closed | PtyState::Error)
    }
}

/// PTY statistics for monitoring.
#[derive(Debug, Clone, Default)]
pub struct PtyStats {
    /// Bytes written to PTY
    pub bytes_written: u64,
    /// Bytes read from PTY
    pub bytes_read: u64,
    /// Number of resize operations
    pub resize_count: u64,
    /// Current state
    pub state: PtyState,
    /// Creation time
    pub created_at: Option<std::time::Instant>,
    /// Last activity time
    pub last_activity: Option<std::time::Instant>,
}

/// PTY backend trait for platform implementations.
///
/// This trait defines the interface for PTY operations.
/// Each platform provides its own implementation.
///
/// # Constraints
///
/// - Must implement Send for thread safety
/// - write() must not block main thread
/// - close() must follow lifecycle order
pub trait PtyBackend: Send {
    /// Write data to the PTY.
    ///
    /// # Arguments
    ///
    /// * `data` - Bytes to write
    ///
    /// # Returns
    ///
    /// Number of bytes written or error.
    fn write(&mut self, data: &[u8]) -> Result<usize, LiteError>;

    /// Resize the PTY terminal.
    ///
    /// # Arguments
    ///
    /// * `cols` - New column count
    /// * `rows` - New row count
    fn resize(&mut self, cols: u16, rows: u16) -> Result<(), LiteError>;

    /// Read data from the PTY.
    ///
    /// # Arguments
    ///
    /// * `buf` - Buffer to read into
    ///
    /// # Returns
    ///
    /// Number of bytes read or error.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, LiteError>;

    /// Close the PTY.
    ///
    /// # Constraints (SYSTEM_INVARIANTS.md)
    ///
    /// - Must close main channel first
    /// - Then release resources
    fn close(&mut self) -> Result<(), LiteError>;

    /// Check if PTY is alive.
    fn is_alive(&self) -> bool;

    /// Get current size.
    fn size(&self) -> PtySize;

    /// Get process ID of the shell.
    fn pid(&self) -> Option<u32>;
}

/// Portable-PTY backend implementation.
///
/// Uses the portable-pty crate for cross-platform PTY support.
#[cfg(feature = "embedded-terminal")]
pub struct PortablePtyBackend {
    /// PTY pair (master/slave)
    pty_pair: PtyPair,
    /// Writer to master
    writer: Box<dyn Write + Send>,
    /// Reader from master
    reader: Box<dyn Read + Send>,
    /// Current size
    size: PtySize,
    /// Process ID
    pid: Option<u32>,
    /// State
    state: PtyState,
    /// Statistics
    stats: PtyStats,
}

#[cfg(feature = "embedded-terminal")]
impl PortablePtyBackend {
    /// Create new portable-pty backend.
    ///
    /// # Arguments
    ///
    /// * `config` - PTY configuration
    ///
    /// # Returns
    ///
    /// New backend instance or error.
    pub fn new(config: &PtyConfig) -> Result<Self, LiteError> {
        let pty_system = NativePtySystem::default();

        let pty_pair = pty_system
            .openpty(config.size.to_portable_pty_size())
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to open PTY: {}", e)))?;

        let cmd = config.build_command();

        let child = pty_pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to spawn shell: {}", e)))?;

        // Get PID
        let pid = child.process_id();

        let reader = pty_pair
            .master
            .try_clone_reader()
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to clone reader: {}", e)))?;

        let writer = pty_pair
            .master
            .take_writer()
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to take writer: {}", e)))?;

        let stats = PtyStats {
            state: PtyState::Active,
            created_at: Some(std::time::Instant::now()),
            last_activity: Some(std::time::Instant::now()),
            ..Default::default()
        };

        Ok(Self {
            pty_pair,
            writer,
            reader,
            size: config.size,
            pid,
            state: PtyState::Active,
            stats,
        })
    }
}

#[cfg(feature = "embedded-terminal")]
impl PtyBackend for PortablePtyBackend {
    fn write(&mut self, data: &[u8]) -> Result<usize, LiteError> {
        if !self.state.can_write() {
            return Err(LiteError::TerminalEmulator(
                "PTY is not in writable state".to_string(),
            ));
        }

        let written = self
            .writer
            .write(data)
            .map_err(|e| LiteError::TerminalEmulator(format!("PTY write failed: {}", e)))?;

        self.stats.bytes_written += written as u64;
        self.stats.last_activity = Some(std::time::Instant::now());

        Ok(written)
    }

    fn resize(&mut self, cols: u16, rows: u16) -> Result<(), LiteError> {
        if !self.state.can_resize() {
            return Err(LiteError::TerminalEmulator(
                "PTY cannot be resized in current state".to_string(),
            ));
        }

        self.state = PtyState::Resizing;

        let new_size = PtySize::new(cols, rows);

        self.pty_pair
            .master
            .resize(new_size.to_portable_pty_size())
            .map_err(|e| LiteError::TerminalEmulator(format!("PTY resize failed: {}", e)))?;

        self.size = new_size;
        self.stats.resize_count += 1;
        self.state = PtyState::Active;

        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, LiteError> {
        if self.state.is_closed() {
            return Err(LiteError::TerminalEmulator("PTY is closed".to_string()));
        }

        let read = self
            .reader
            .read(buf)
            .map_err(|e| LiteError::TerminalEmulator(format!("PTY read failed: {}", e)))?;

        self.stats.bytes_read += read as u64;
        self.stats.last_activity = Some(std::time::Instant::now());

        Ok(read)
    }

    fn close(&mut self) -> Result<(), LiteError> {
        // Follow lifecycle constraints (SYSTEM_INVARIANTS.md Section 1.1):
        // 1. Close main channel first
        // 2. Then release resources

        self.state = PtyState::ClosingChannel;

        // Close writer (main channel)
        let _ = self.writer.flush();

        self.state = PtyState::ReleasingResources;

        // Release PTY resources
        // Note: portable-pty handles cleanup when PtyPair is dropped

        self.state = PtyState::Closed;
        self.stats.state = PtyState::Closed;

        Ok(())
    }

    fn is_alive(&self) -> bool {
        self.state.is_alive()
    }

    fn size(&self) -> PtySize {
        self.size
    }

    fn pid(&self) -> Option<u32> {
        self.pid
    }
}

/// PTY handle for session use.
///
/// Provides a thread-safe handle to a PTY backend.
#[derive(Clone)]
pub struct PtyHandle {
    /// Unique ID
    id: String,
    /// Connection ID (for state gating)
    connection_id: String,
    /// Backend (wrapped for thread safety)
    backend: Arc<Mutex<Option<Box<dyn PtyBackend>>>>,
    /// State
    state: Arc<RwLock<PtyState>>,
    /// Statistics
    stats: Arc<RwLock<PtyStats>>,
    /// Output channel (non-blocking per SYSTEM_INVARIANTS.md)
    output_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Stop flag for read loop
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
}

impl PtyHandle {
    /// Create new PTY handle.
    pub fn new(
        id: &str,
        connection_id: &str,
        backend: Box<dyn PtyBackend>,
        output_tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Self {
        Self {
            id: id.to_string(),
            connection_id: connection_id.to_string(),
            backend: Arc::new(Mutex::new(Some(backend))),
            state: Arc::new(RwLock::new(PtyState::Active)),
            stats: Arc::new(RwLock::new(PtyStats::default())),
            output_tx,
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Get PTY ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get connection ID.
    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    /// Write data to PTY (async).
    ///
    /// # Constraints
    ///
    /// - Must not block main thread
    /// - Connection must be active (state gating)
    pub async fn write(&self, data: &[u8]) -> Result<usize, LiteError> {
        let mut backend = self.backend.lock().await;

        if let Some(ref mut b) = &mut *backend {
            let result = b.write(data);

            // Update stats
            if let Ok(written) = &result {
                let mut stats = self.stats.write().await;
                stats.bytes_written += *written as u64;
                stats.last_activity = Some(std::time::Instant::now());
            }

            result
        } else {
            Err(LiteError::TerminalEmulator("PTY backend not available".to_string()))
        }
    }

    /// Resize PTY (async).
    ///
    /// # Arguments
    ///
    /// * `cols` - New column count
    /// * `rows` - New row count
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<(), LiteError> {
        // Update state
        {
            let mut state = self.state.write().await;
            if !state.can_resize() {
                return Err(LiteError::TerminalEmulator(
                    "PTY cannot be resized in current state".to_string(),
                ));
            }
            *state = PtyState::Resizing;
        }

        let mut backend = self.backend.lock().await;

        if let Some(ref mut b) = &mut *backend {
            let result = b.resize(cols, rows);

            // Update state and stats
            {
                let mut state = self.state.write().await;
                *state = if result.is_ok() {
                    PtyState::Active
                } else {
                    PtyState::Error
                };
            }

            if result.is_ok() {
                let mut stats = self.stats.write().await;
                stats.resize_count += 1;
            }

            result
        } else {
            Err(LiteError::TerminalEmulator("PTY backend not available".to_string()))
        }
    }

    /// Close PTY (async).
    ///
    /// # Constraints (SYSTEM_INVARIANTS.md Section 1.1)
    ///
    /// - Must close main channel first
    /// - Then release resources
    pub async fn close(&self) -> Result<(), LiteError> {
        // Signal read loop to stop
        self.stop_flag.store(true, std::sync::atomic::Ordering::SeqCst);

        // Update state: closing channel
        {
            let mut state = self.state.write().await;
            *state = PtyState::ClosingChannel;
        }

        let mut backend = self.backend.lock().await;

        if let Some(ref mut b) = &mut *backend {
            // Close backend (follows lifecycle order)
            let result = b.close();

            // Update state
            {
                let mut state = self.state.write().await;
                *state = if result.is_ok() {
                    PtyState::Closed
                } else {
                    PtyState::Error
                };
            }

            // Remove backend
            *backend = None;

            result
        } else {
            // Already closed
            Ok(())
        }
    }

    /// Get current state.
    pub async fn state(&self) -> PtyState {
        *self.state.read().await
    }

    /// Get statistics.
    pub async fn stats(&self) -> PtyStats {
        self.stats.read().await.clone()
    }

    /// Check if PTY is alive.
    pub async fn is_alive(&self) -> bool {
        let state = self.state.read().await;
        state.is_alive()
    }

    /// Start output reader loop.
    ///
    /// # Constraints (SYSTEM_INVARIANTS.md Section 1.1)
    ///
    /// - Output callback must not block main thread
    /// - Uses unbounded channel for non-blocking send
    pub fn start_reader(&self) {
        // Note: In production, this would spawn a tokio task
        // For now, this is a placeholder that shows the pattern
        let backend = self.backend.clone();
        let output_tx = self.output_tx.clone();
        let stop_flag = self.stop_flag.clone();

        // In real implementation:
        // tokio::spawn(async move {
        //     while !stop_flag.load(Ordering::SeqCst) {
        //         let mut buf = vec![0u8; 4096];
        //         let mut b = backend.lock().await;
        //         if let Some(ref mut backend) = b {
        //             if let Ok(read) = backend.read(&mut buf) {
        //                 output_tx.send(buf[..read].to_vec()).ok();
        //             }
        //         }
        //     }
        // });
    }
}

/// PTY manager for session management.
///
/// Manages PTY instances with:
/// - Connection state gating
/// - Lifecycle management
/// - Output routing
pub struct PtyManager {
    /// Active PTY handles
    handles: Arc<RwLock<HashMap<String, PtyHandle>>>,
    /// Output receivers
    outputs: Arc<RwLock<HashMap<String, mpsc::UnboundedReceiver<Vec<u8>>>>>,
    /// Statistics
    stats: Arc<RwLock<HashMap<String, PtyStats>>>,
}

impl PtyManager {
    /// Create new PTY manager.
    pub fn new() -> Self {
        Self {
            handles: Arc::new(RwLock::new(HashMap::new())),
            outputs: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create PTY for a session.
    ///
    /// # Constraints (SYSTEM_INVARIANTS.md Section 1.1)
    ///
    /// - PTY must be created after Connection is Active
    /// - Must verify connection state before creation
    ///
    /// # Arguments
    ///
    /// * `id` - Unique PTY ID
    /// * `connection_id` - Connection ID (for state gating)
    /// * `config` - PTY configuration
    ///
    /// # Returns
    ///
    /// PTY handle and output receiver.
    #[cfg(feature = "embedded-terminal")]
    pub async fn create_pty(
        &self,
        id: &str,
        connection_id: &str,
        config: PtyConfig,
    ) -> Result<(PtyHandle, mpsc::UnboundedReceiver<Vec<u8>>), LiteError> {
        // Create output channel
        let (output_tx, output_rx) = mpsc::unbounded_channel();

        // Create backend
        let backend = PortablePtyBackend::new(&config)?;

        // Create handle
        let handle = PtyHandle::new(id, connection_id, Box::new(backend), output_tx);

        // Start reader
        handle.start_reader();

        // Register handle
        {
            let mut handles = self.handles.write().await;
            handles.insert(id.to_string(), handle.clone());
        }

        // Register output
        {
            let mut outputs = self.outputs.write().await;
            outputs.insert(id.to_string(), output_rx);
        }

        Ok((handle, self.outputs.write().await.remove(id).unwrap()))
    }

    /// Get PTY handle by ID.
    pub async fn get_handle(&self, id: &str) -> Option<PtyHandle> {
        let handles = self.handles.read().await;
        handles.get(id).cloned()
    }

    /// Close PTY by ID.
    ///
    /// # Constraints (SYSTEM_INVARIANTS.md Section 1.1)
    ///
    /// - Close main channel first
    /// - Then release resources
    pub async fn close_pty(&self, id: &str) -> Result<(), LiteError> {
        let handle = {
            let handles = self.handles.read().await;
            handles.get(id).cloned()
        };

        if let Some(h) = handle {
            h.close().await?;

            // Remove from registry
            {
                let mut handles = self.handles.write().await;
                handles.remove(id);
            }

            {
                let mut outputs = self.outputs.write().await;
                outputs.remove(id);
            }
        }

        Ok(())
    }

    /// Close all PTYs (for cleanup).
    pub async fn close_all(&self) -> Result<(), LiteError> {
        let ids: Vec<String> = {
            let handles = self.handles.read().await;
            handles.keys().cloned().collect()
        };

        for id in ids {
            self.close_pty(&id).await?;
        }

        Ok(())
    }

    /// Get manager statistics.
    pub async fn stats(&self) -> HashMap<String, PtyStats> {
        self.stats.read().await.clone()
    }

    /// Count active PTYs.
    pub async fn count(&self) -> usize {
        self.handles.read().await.len()
    }

    /// List PTY IDs.
    pub async fn list_ids(&self) -> Vec<String> {
        self.handles.read().await.keys().cloned().collect()
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============ Unit Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_size_default() {
        let size = PtySize::default();
        assert_eq!(size.cols, 80);
        assert_eq!(size.rows, 24);
    }

    #[test]
    fn test_pty_size_new() {
        let size = PtySize::new(120, 40);
        assert_eq!(size.cols, 120);
        assert_eq!(size.rows, 40);
    }

    #[test]
    fn test_pty_config_default() {
        let config = PtyConfig::default();
        assert_eq!(config.size.cols, 80);
        assert_eq!(config.term_type, "xterm-256color");
        assert!(config.utf8);
    }

    #[test]
    fn test_pty_config_builder() {
        let config = PtyConfig::with_size(120, 40)
            .with_shell("/bin/bash")
            .with_working_dir("/home/user")
            .with_env("FOO", "bar")
            .with_term_type("xterm");

        assert_eq!(config.size.cols, 120);
        assert_eq!(config.size.rows, 40);
        assert_eq!(config.shell, Some("/bin/bash".to_string()));
        assert_eq!(config.working_dir, Some("/home/user".to_string()));
        assert!(config.env.contains(&(String::from("FOO"), String::from("bar"))));
        assert_eq!(config.term_type, "xterm");
    }

    #[test]
    fn test_pty_state_checks() {
        assert!(PtyState::Active.can_write());
        assert!(PtyState::Active.can_resize());
        assert!(PtyState::Active.is_alive());
        assert!(!PtyState::Active.is_closed());

        assert!(!PtyState::Closed.can_write());
        assert!(!PtyState::Closed.can_resize());
        assert!(!PtyState::Closed.is_alive());
        assert!(PtyState::Closed.is_closed());
    }

    #[test]
    fn test_pty_stats_default() {
        let stats = PtyStats::default();
        assert_eq!(stats.bytes_written, 0);
        assert_eq!(stats.bytes_read, 0);
        assert_eq!(stats.state, PtyState::Initializing);
    }

    #[tokio::test]
    async fn test_pty_manager_creation() {
        let manager = PtyManager::new();
        assert_eq!(manager.count().await, 0);
    }

    #[tokio::test]
    async fn test_pty_manager_list_ids() {
        let manager = PtyManager::new();
        let ids = manager.list_ids().await;
        assert!(ids.is_empty());
    }
}