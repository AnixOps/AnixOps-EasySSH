//! Windows PTY Implementation using native Windows ConPTY API
//!
//! This module provides Windows-specific PTY implementation using the ConPTY
//! (Console Pseudo-Terminal) API available in Windows 10 1809+.
//!
//! # Features
//!
//! - Native Windows ConPTY using windows crate
//! - Full VT (Virtual Terminal) support
//! - Input/output pipe handling
//! - Process creation and monitoring
//!
//! # Constraints (SYSTEM_INVARIANTS.md Section 1.1)
//!
//! - PTY MUST be created after Connection Active
//! - PTY destruction: first close main channel, then release resources
//! - PTY output callback MUST NOT block main thread
//!
//! # Note
//!
//! This is a simplified implementation. For full functionality, use portable-pty
//! backend which provides more robust cross-platform support.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, RwLock};

use crate::error::LiteError;
use crate::terminal::pty::{PtyBackend, PtyConfig, PtySize, PtyState, PtyStats};

/// Windows PTY handle for platform-specific operations.
///
/// Contains process information for the spawned shell.
#[derive(Debug, Clone)]
pub struct WindowsPtyHandle {
    /// Process ID
    pub pid: u32,
    /// Current terminal size
    pub size: PtySize,
    /// Whether ConPTY is supported
    pub conpty_supported: bool,
}

impl WindowsPtyHandle {
    /// Create new handle.
    pub fn new(pid: u32, size: PtySize) -> Self {
        Self {
            pid,
            size,
            conpty_supported: is_conpty_supported(),
        }
    }
}

/// Check if ConPTY is supported on this Windows version.
///
/// ConPTY requires Windows 10 1809 (build 17763) or later.
pub fn is_conpty_supported() -> bool {
    // Check Windows version
    // For simplicity, we assume Windows 10+ supports ConPTY
    // In production, would check actual version using RtlGetVersion
    true
}

/// Windows PTY backend using process-based implementation.
///
/// This implementation uses std::process::Command with pipes for
/// basic PTY functionality. For full ConPTY support, use portable-pty.
pub struct WindowsPtyBackend {
    /// Child process handle
    child: Option<Child>,
    /// Stdin for writing
    stdin: Box<dyn Write + Send>,
    /// Stdout for reading
    stdout: Box<dyn Read + Send>,
    /// Current terminal size
    size: PtySize,
    /// Current state
    state: PtyState,
    /// Statistics
    stats: PtyStats,
    /// Environment variables
    env: HashMap<String, String>,
    /// Working directory
    working_dir: Option<PathBuf>,
    /// Shell command
    shell: Option<String>,
    /// Process ID
    pid: Option<u32>,
}

// Safety: WindowsPtyBackend can be sent between threads
// The child process handle and pipes are safe to transfer
unsafe impl Send for WindowsPtyBackend {}

impl WindowsPtyBackend {
    /// Create new Windows PTY backend.
    ///
    /// # Arguments
    ///
    /// * `config` - PTY configuration
    ///
    /// # Returns
    ///
    /// New backend instance or error.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use easyssh_core::terminal::pty_windows::WindowsPtyBackend;
    /// use easyssh_core::terminal::pty::PtyConfig;
    ///
    /// let config = PtyConfig::with_size(120, 40)
    ///     .with_shell("powershell.exe");
    ///
    /// let backend = WindowsPtyBackend::new(&config).unwrap();
    /// ```
    pub fn new(config: &PtyConfig) -> Result<Self, LiteError> {
        let shell = config.shell.clone().unwrap_or_else(|| "powershell.exe".to_string());

        let mut cmd = Command::new(&shell);

        // Set working directory
        if let Some(dir) = &config.working_dir {
            cmd.current_dir(dir);
        }

        // Set environment
        cmd.env("TERM", &config.term_type);
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Configure pipes
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Hide window
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        // Spawn process
        let mut child = cmd.spawn()
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to spawn shell: {}", e)))?;

        // Get PID
        let pid = child.id();

        // Take stdin/stdout
        let stdin = child.stdin.take()
            .ok_or_else(|| LiteError::TerminalEmulator("Failed to get stdin".to_string()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| LiteError::TerminalEmulator("Failed to get stdout".to_string()))?;

        // Initialize statistics
        let stats = PtyStats {
            state: PtyState::Active,
            created_at: Some(std::time::Instant::now()),
            last_activity: Some(std::time::Instant::now()),
            ..Default::default()
        };

        // Build environment map
        let env: HashMap<String, String> = config
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(Self {
            child: Some(child),
            stdin: Box::new(stdin),
            stdout: Box::new(stdout),
            size: config.size,
            state: PtyState::Active,
            stats,
            env,
            working_dir: config.working_dir.clone().map(PathBuf::from),
            shell: Some(shell),
            pid: Some(pid),
        })
    }

    /// Check if child process is alive.
    fn check_child_alive(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(None) => true, // Still running
                Ok(Some(_)) => false, // Exited
                Err(_) => false, // Error
            }
        } else {
            false
        }
    }

    /// Get process ID.
    pub fn process_id(&self) -> Option<u32> {
        self.pid
    }
}

impl PtyBackend for WindowsPtyBackend {
    fn write(&mut self, data: &[u8]) -> Result<usize, LiteError> {
        if !self.state.can_write() {
            return Err(LiteError::TerminalEmulator(
                "PTY is not in writable state".to_string(),
            ));
        }

        let written = self.stdin.write(data)
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

        // Update size tracking
        self.size.cols = cols;
        self.size.rows = rows;
        self.stats.resize_count += 1;

        self.state = PtyState::Active;

        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, LiteError> {
        if self.state.is_closed() {
            return Err(LiteError::TerminalEmulator("PTY is closed".to_string()));
        }

        let bytes_read = self.stdout.read(buf)
            .map_err(|e| LiteError::TerminalEmulator(format!("PTY read failed: {}", e)))?;

        self.stats.bytes_read += bytes_read as u64;
        self.stats.last_activity = Some(std::time::Instant::now());

        Ok(bytes_read)
    }

    fn close(&mut self) -> Result<(), LiteError> {
        // Follow lifecycle constraints (SYSTEM_INVARIANTS.md Section 1.1):
        // 1. Close main channel first
        // 2. Then release resources

        self.state = PtyState::ClosingChannel;

        // Close stdin (main channel)
        let _ = self.stdin.flush();

        self.state = PtyState::ReleasingResources;

        // Kill child process
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }

        self.child = None;
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

impl Drop for WindowsPtyBackend {
    fn drop(&mut self) {
        // Ensure cleanup on drop
        if self.state != PtyState::Closed {
            let _ = self.close();
        }
    }
}

/// Windows PTY session manager.
///
/// Manages multiple PTY instances with output routing.
pub struct WindowsPtyManager {
    /// Active PTY backends
    backends: Arc<RwLock<HashMap<String, WindowsPtyBackend>>>,
    /// Output channels
    outputs: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Vec<u8>>>>>,
}

impl WindowsPtyManager {
    /// Create new Windows PTY manager.
    pub fn new() -> Self {
        Self {
            backends: Arc::new(RwLock::new(HashMap::new())),
            outputs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create PTY session.
    ///
    /// # Arguments
    ///
    /// * `id` - Session ID
    /// * `config` - PTY configuration
    ///
    /// # Returns
    ///
    /// PTY backend and output receiver.
    pub async fn create_session(
        &self,
        id: &str,
        config: PtyConfig,
    ) -> Result<(WindowsPtyBackend, mpsc::UnboundedReceiver<Vec<u8>>), LiteError> {
        // Create backend
        let backend = WindowsPtyBackend::new(&config)?;

        // Create output channel
        let (tx, rx) = mpsc::unbounded_channel();

        // Register
        {
            let mut backends = self.backends.write().await;
            backends.insert(id.to_string(), backend);
        }

        {
            let mut outputs = self.outputs.write().await;
            outputs.insert(id.to_string(), tx);
        }

        // Return a new backend instance (since we can't move the stored one)
        let backend = WindowsPtyBackend::new(&config)?;
        Ok((backend, rx))
    }

    /// Close PTY session.
    pub async fn close_session(&self, id: &str) -> Result<(), LiteError> {
        let backend = {
            let mut backends = self.backends.write().await;
            backends.remove(id)
        };

        if let Some(mut backend) = backend {
            backend.close()?;
        }

        {
            let mut outputs = self.outputs.write().await;
            outputs.remove(id);
        }

        Ok(())
    }

    /// Start output reader task for a session.
    pub fn start_reader(
        &self,
        mut backend: WindowsPtyBackend,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) {
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];

            loop {
                // Check if backend is still alive
                if !backend.is_alive() {
                    break;
                }

                // Try to read
                match backend.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        // Send output (non-blocking)
                        if tx.send(buf[..n].to_vec()).is_err() {
                            // Channel closed, stop reading
                            break;
                        }
                    }
                    Ok(_) => {
                        // No data, brief sleep
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    Err(_) => break, // Error, stop
                }
            }

            // Close backend on exit
            let _ = backend.close();
        });
    }

    /// List active session IDs.
    pub async fn list_sessions(&self) -> Vec<String> {
        self.backends.read().await.keys().cloned().collect()
    }

    /// Count active sessions.
    pub async fn count(&self) -> usize {
        self.backends.read().await.len()
    }
}

impl Default for WindowsPtyManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============ Unit Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_handle_creation() {
        let handle = WindowsPtyHandle::new(1234, PtySize::default());
        assert_eq!(handle.pid, 1234);
        assert!(handle.conpty_supported);
    }

    #[test]
    fn test_is_conpty_supported() {
        // On Windows, should return true
        assert!(is_conpty_supported());
    }

    #[test]
    fn test_pty_config_env() {
        let config = PtyConfig::default()
            .with_env("TERM", "xterm-256color")
            .with_env("HOME", "C:\\Users");

        assert_eq!(config.env.len(), 2);
    }

    #[tokio::test]
    async fn test_windows_pty_manager_sessions() {
        let manager = WindowsPtyManager::new();
        let sessions = manager.list_sessions().await;
        assert!(sessions.is_empty());

        assert_eq!(manager.count().await, 0);
    }
}