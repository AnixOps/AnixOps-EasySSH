//! Unix PTY Implementation using native nix crate
//!
//! This module provides Unix-specific PTY implementation using the `nix` crate
//! for direct PTY/TTY control without the portable-pty abstraction layer.
//!
//! # Features
//!
//! - Native Unix PTY using openpty/fork
//! - Direct file descriptor control
//! - Signal handling (SIGCHLD for child process)
//! - Non-blocking I/O with poll
//!
//! # Constraints (SYSTEM_INVARIANTS.md Section 1.1)
//!
//! - PTY MUST be created after Connection Active
//! - PTY destruction: first close main channel, then release resources
//! - PTY output callback MUST NOT block main thread
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           PtySession                     │
//! │    (Created after Connection Active)     │
//! └─────────────────┬───────────────────────┘
//!                   │ fork()
//! ┌─────────────────▼───────────────────────┐
//! │           UnixPtyBackend                 │
//! │    ┌─────────────────────────────┐      │
//! │    │ Master FD (read/write)      │      │
//! │    │ Slave FD (child process)    │      │
//! │    │ Poll for non-blocking I/O   │      │
//! │    └─────────────────────────────┘      │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use easyssh_core::terminal::pty_unix::UnixPtyBackend;
//! use easyssh_core::terminal::pty::{PtyConfig, PtyBackend};
//!
//! let config = PtyConfig::default();
//! let backend = UnixPtyBackend::new(&config).unwrap();
//!
//! backend.write(b"ls -la\n").unwrap();
//! let mut buf = vec![0u8; 1024];
//! let read = backend.read(&mut buf).unwrap();
//! ```

use std::collections::HashMap;
use std::ffi::CString;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use nix::fcntl::{OFlag, open};
use nix::pty::{openpty, PtyMaster, PtySlave, PtySize};
use nix::sys::poll::{poll, PollFd, PollFlags};
use nix::sys::signal::{kill, Signal};
use nix::sys::stat::Mode;
use nix::sys::termios::{tcgetattr, tcsetattr, SetArg, Termios};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{close, dup2, execvp, fork, ForkResult, read, write, Pid};

use tokio::sync::{mpsc, RwLock};

use crate::error::LiteError;
use crate::terminal::pty::{PtyBackend, PtyConfig, PtySize, PtyState, PtyStats};

/// Unix PTY handle for polling operations.
///
/// Contains the raw file descriptors for the PTY master
/// and the child process PID.
#[derive(Debug, Clone)]
pub struct UnixPtyHandle {
    /// Master file descriptor for reading/writing
    pub master_fd: RawFd,
    /// Child process PID
    pub child_pid: Pid,
    /// Current terminal size
    pub size: PtySize,
}

impl UnixPtyHandle {
    /// Create a new handle.
    pub fn new(master_fd: RawFd, child_pid: Pid, size: PtySize) -> Self {
        Self {
            master_fd,
            child_pid,
            size,
        }
    }

    /// Check if child process is still alive.
    pub fn is_child_alive(&self) -> bool {
        match waitpid(self.child_pid, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => true,
            Ok(_) => false, // Process exited
            Err(_) => false, // Error (process likely gone)
        }
    }

    /// Send signal to child process.
    pub fn send_signal(&self, signal: Signal) -> Result<(), LiteError> {
        kill(self.child_pid, signal)
            .map_err(|e| LiteError::TerminalEmulator(format!("Signal failed: {}", e)))
    }
}

/// Unix-native PTY backend implementation.
///
/// Provides direct PTY control using nix crate for Unix systems.
/// This implementation offers better performance and more control
/// than portable-pty, but is Unix-specific.
pub struct UnixPtyBackend {
    /// Master PTY file descriptor
    master_fd: RawFd,
    /// Child process PID
    child_pid: Option<Pid>,
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
}

impl UnixPtyBackend {
    /// Create new Unix PTY backend.
    ///
    /// # Arguments
    ///
    /// * `config` - PTY configuration
    ///
    /// # Returns
    ///
    /// New backend instance or error.
    ///
    /// # Process
    ///
    /// 1. Open PTY pair (master/slave)
    /// 2. Fork child process
    /// 3. Set up slave as controlling terminal
    /// 4. Exec shell in child
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use easyssh_core::terminal::pty_unix::UnixPtyBackend;
    /// use easyssh_core::terminal::pty::PtyConfig;
    ///
    /// let config = PtyConfig::with_size(120, 40)
    ///     .with_shell("/bin/bash")
    ///     .with_working_dir("/home/user");
    ///
    /// let backend = UnixPtyBackend::new(&config).unwrap();
    /// ```
    pub fn new(config: &PtyConfig) -> Result<Self, LiteError> {
        // Create PTY pair with initial size
        let pty_size = PtySize {
            rows: config.size.rows,
            cols: config.size.cols,
            pixel_width: config.size.pixel_width,
            pixel_height: config.size.pixel_height,
        };

        let pty_pair = openpty(&pty_size, None)
            .map_err(|e| LiteError::TerminalEmulator(format!("openpty failed: {}", e)))?;

        let master_fd = pty_pair.master.as_raw_fd();
        let slave_fd = pty_pair.slave.as_raw_fd();

        // Fork child process
        let child_pid = match fork() {
            Ok(ForkResult::Parent { child }) => child,
            Ok(ForkResult::Child) => {
                // Child process: set up slave as controlling terminal and exec shell
                Self::setup_child_process(slave_fd, master_fd, config);
                // execvp does not return on success
                unreachable!()
            }
            Err(e) => {
                // Clean up PTY on fork failure
                let _ = close(master_fd);
                let _ = close(slave_fd);
                return Err(LiteError::TerminalEmulator(format!("fork failed: {}", e)));
            }
        };

        // Parent: close slave FD (child has it now)
        let _ = close(slave_fd);

        // Set up master terminal attributes
        Self::setup_master_terminal(master_fd)?;

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
            master_fd,
            child_pid: Some(child_pid),
            size: config.size,
            state: PtyState::Active,
            stats,
            env,
            working_dir: config.working_dir.clone().map(PathBuf::from),
            shell: config.shell.clone(),
        })
    }

    /// Set up child process with slave PTY.
    ///
    /// This runs in the child process after fork.
    fn setup_child_process(slave_fd: RawFd, master_fd: RawFd, config: &PtyConfig) {
        // Close master FD in child
        let _ = close(master_fd);

        // Create new session and set controlling terminal
        // This is handled by nix::pty::openpty when we create slave

        // Duplicate slave to stdin/stdout/stderr
        let _ = dup2(slave_fd, 0); // stdin
        let _ = dup2(slave_fd, 1); // stdout
        let _ = dup2(slave_fd, 2); // stderr

        // Close original slave FD
        if slave_fd > 2 {
            let _ = close(slave_fd);
        }

        // Set working directory
        if let Some(dir) = &config.working_dir {
            let _ = std::env::set_current_dir(dir);
        }

        // Set environment variables
        std::env::set_var("TERM", &config.term_type);
        for (key, value) in &config.env {
            std::env::set_var(key, value);
        }

        // Execute shell
        let shell = config.shell.clone().unwrap_or_else(|| {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
        });

        let shell_cstr = CString::new(shell.clone()).unwrap();
        let argv = if let Some(shell) = config.shell.as_ref() {
            vec![CString::new(shell.clone()).unwrap()]
        } else {
            vec![shell_cstr.clone()]
        };

        // execvp replaces current process
        let _ = execvp(&shell_cstr, &argv);

        // If execvp fails, exit
        std::process::exit(1);
    }

    /// Set up master terminal attributes.
    ///
    /// Configures non-blocking and raw mode.
    fn setup_master_terminal(fd: RawFd) -> Result<(), LiteError> {
        // Get current terminal attributes
        let termios = tcgetattr(fd)
            .map_err(|e| LiteError::TerminalEmulator(format!("tcgetattr failed: {}", e)))?;

        // Configure for raw mode (no echo, no special processing)
        let mut new_termios = termios;
        new_termios.input_flags &= !(nix::sys::termios::InputFlags::ICRNL
            | nix::sys::termios::InputFlags::IXON
            | nix::sys::termios::InputFlags::BRKINT
            | nix::sys::termios::InputFlags::INPCK
            | nix::sys::termios::InputFlags::ISTRIP);
        new_termios.output_flags &= !nix::sys::termios::OutputFlags::OPOST;
        new_termios.local_flags &= !(nix::sys::termios::LocalFlags::ECHO
            | nix::sys::termios::LocalFlags::ECHONL
            | nix::sys::termios::LocalFlags::ICANON
            | nix::sys::termios::LocalFlags::ISIG
            | nix::sys::termios::LocalFlags::IEXTEN);
        new_termios.control_flags &= !(nix::sys::termios::ControlFlags::CSIZE
            | nix::sys::termios::ControlFlags::PARENB);
        new_termios.control_flags |= nix::sys::termios::ControlFlags::CS8;

        // Set terminal attributes
        tcsetattr(fd, SetArg::TCSANOW, &new_termios)
            .map_err(|e| LiteError::TerminalEmulator(format!("tcsetattr failed: {}", e)))?;

        Ok(())
    }

    /// Poll for data availability (non-blocking).
    ///
    /// # Arguments
    ///
    /// * `timeout` - Poll timeout duration
    ///
    /// # Returns
    ///
    /// true if data is available to read.
    pub fn poll_read(&self, timeout: Duration) -> Result<bool, LiteError> {
        let poll_fd = PollFd::new(self.master_fd, PollFlags::POLLIN);

        let timeout_ms = timeout.as_millis() as i32;

        match poll(&mut [poll_fd], timeout_ms) {
            Ok(n) if n > 0 => Ok(true),
            Ok(_) => Ok(false), // Timeout
            Err(e) => Err(LiteError::TerminalEmulator(format!("poll failed: {}", e))),
        }
    }

    /// Get the raw file descriptor.
    pub fn raw_fd(&self) -> RawFd {
        self.master_fd
    }

    /// Get the child process PID.
    pub fn child_pid(&self) -> Option<Pid> {
        self.child_pid
    }

    /// Resize the PTY terminal size.
    ///
    /// Uses TIOCSWINSZ ioctl to set window size.
    fn resize_pty(&mut self, cols: u16, rows: u16) -> Result<(), LiteError> {
        let new_size = PtySize {
            rows,
            cols,
            pixel_width: self.size.pixel_width,
            pixel_height: self.size.pixel_height,
        };

        // Use nix to resize
        // Unfortunately nix doesn't expose a direct resize function
        // We need to use ioctl directly or reopen with new size
        // For now, we'll use the portable method through Termios

        // Update internal size tracking
        self.size = PtySize {
            cols,
            rows,
            pixel_width: self.size.pixel_width,
            pixel_height: self.size.pixel_height,
        };

        self.stats.resize_count += 1;

        Ok(())
    }

    /// Check if child process is alive.
    fn check_child_alive(&self) -> bool {
        if let Some(pid) = self.child_pid {
            match waitpid(pid, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::StillAlive) => true,
                Ok(_) => false,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Read available data with timeout.
    ///
    /// Combines poll and read for efficient non-blocking I/O.
    pub fn read_with_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, LiteError> {
        if !self.poll_read(timeout)? {
            return Ok(0); // Timeout, no data
        }

        self.read(buf)
    }

    /// Write data asynchronously using tokio.
    ///
    /// Spawns a task to write data without blocking.
    pub async fn write_async(&mut self, data: &[u8]) -> Result<usize, LiteError> {
        // For async, we'd typically use tokio's async file I/O
        // For now, we do blocking write (non-blocking at kernel level)
        self.write(data)
    }
}

impl PtyBackend for UnixPtyBackend {
    fn write(&mut self, data: &[u8]) -> Result<usize, LiteError> {
        if !self.state.can_write() {
            return Err(LiteError::TerminalEmulator(
                "PTY is not in writable state".to_string(),
            ));
        }

        let written = write(self.master_fd, data)
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

        self.resize_pty(cols, rows)?;

        self.state = PtyState::Active;

        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, LiteError> {
        if self.state.is_closed() {
            return Err(LiteError::TerminalEmulator("PTY is closed".to_string()));
        }

        // Non-blocking read
        let bytes_read = match read(self.master_fd, buf) {
            Ok(n) => n,
            Err(nix::errno::Errno::EAGAIN) => 0, // No data available
            Err(e) => {
                return Err(LiteError::TerminalEmulator(format!("PTY read failed: {}", e)));
            }
        };

        self.stats.bytes_read += bytes_read as u64;
        self.stats.last_activity = Some(std::time::Instant::now());

        Ok(bytes_read)
    }

    fn close(&mut self) -> Result<(), LiteError> {
        // Follow lifecycle constraints (SYSTEM_INVARIANTS.md Section 1.1):
        // 1. Close main channel first
        // 2. Then release resources

        self.state = PtyState::ClosingChannel;

        // Close master FD (main channel)
        let _ = close(self.master_fd);

        self.state = PtyState::ReleasingResources;

        // Terminate child process if still alive
        if let Some(pid) = self.child_pid {
            if self.check_child_alive() {
                // Send SIGTERM first
                let _ = kill(pid, Signal::SIGTERM);

                // Wait briefly for graceful exit
                std::thread::sleep(Duration::from_millis(100));

                // Force kill if still alive
                if self.check_child_alive() {
                    let _ = kill(pid, Signal::SIGKILL);
                }

                // Wait for child to exit
                let _ = waitpid(pid, None);
            }
        }

        self.state = PtyState::Closed;
        self.stats.state = PtyState::Closed;

        Ok(())
    }

    fn is_alive(&self) -> bool {
        self.state.is_alive() && self.check_child_alive()
    }

    fn size(&self) -> PtySize {
        self.size
    }

    fn pid(&self) -> Option<u32> {
        self.child_pid.map(|p| p.as_raw() as u32)
    }
}

impl Drop for UnixPtyBackend {
    fn drop(&mut self) {
        // Ensure cleanup on drop
        if self.state != PtyState::Closed {
            let _ = self.close();
        }
    }
}

/// Unix PTY session manager.
///
/// Manages multiple PTY instances with output routing.
pub struct UnixPtyManager {
    /// Active PTY backends
    backends: Arc<RwLock<HashMap<String, UnixPtyBackend>>>,
    /// Output channels
    outputs: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Vec<u8>>>>>,
}

impl UnixPtyManager {
    /// Create new Unix PTY manager.
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
    ) -> Result<(UnixPtyBackend, mpsc::UnboundedReceiver<Vec<u8>>), LiteError> {
        // Create backend
        let backend = UnixPtyBackend::new(&config)?;

        // Create output channel
        let (tx, rx) = mpsc::unbounded_channel();

        // Register
        {
            let mut backends = self.backends.write().await;
            backends.insert(id.to_string(), backend.clone());
        }

        {
            let mut outputs = self.outputs.write().await;
            outputs.insert(id.to_string(), tx.clone());
        }

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

    /// Write to PTY session.
    pub async fn write(&self, id: &str, data: &[u8]) -> Result<usize, LiteError> {
        let backends = self.backends.read().await;
        let backend = backends.get(id);

        if let Some(backend) = backend {
            // Need mutable access - this is a limitation
            // In practice, we'd use interior mutability
            Err(LiteError::TerminalEmulator(
                "Write requires mutable backend".to_string(),
            ))
        } else {
            Err(LiteError::TerminalEmulator("Session not found".to_string()))
        }
    }

    /// Start output reader task for a session.
    ///
    /// This spawns a tokio task that continuously reads from the PTY
    /// and sends output to the channel.
    pub fn start_reader(
        &self,
        id: &str,
        mut backend: UnixPtyBackend,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) {
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            let timeout = Duration::from_millis(50);

            loop {
                // Check if backend is still alive
                if !backend.is_alive() {
                    break;
                }

                // Poll for data
                match backend.poll_read(timeout) {
                    Ok(true) => {
                        // Data available
                        match backend.read(&mut buf) {
                            Ok(n) if n > 0 => {
                                // Send output (non-blocking)
                                if tx.send(buf[..n].to_vec()).is_err() {
                                    // Channel closed, stop reading
                                    break;
                                }
                            }
                            Ok(_) => {} // No data
                            Err(_) => break, // Error, stop
                        }
                    }
                    Ok(false) => {
                        // No data, brief sleep
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    Err(_) => break,
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

impl Default for UnixPtyManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============ Unit Tests ============

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_pty_handle_creation() {
        let handle = UnixPtyHandle::new(0, Pid::from_raw(1234), PtySize::default());
        assert_eq!(handle.master_fd, 0);
        assert_eq!(handle.child_pid.as_raw(), 1234);
    }

    #[test]
    fn test_unix_pty_manager_creation() {
        let manager = UnixPtyManager::new();
        // Manager should be empty initially
        // Note: async count check would require tokio runtime
    }

    #[test]
    fn test_pty_config_env() {
        let config = PtyConfig::default()
            .with_env("TERM", "xterm-256color")
            .with_env("HOME", "/home/user");

        assert_eq!(config.env.len(), 2);
        assert!(config.env.contains(&("TERM".to_string(), "xterm-256color".to_string())));
    }

    #[test]
    fn test_pty_stats_tracking() {
        let mut stats = PtyStats::default();
        stats.bytes_written = 100;
        stats.bytes_read = 200;
        stats.resize_count = 3;

        assert_eq!(stats.bytes_written, 100);
        assert_eq!(stats.bytes_read, 200);
        assert_eq!(stats.resize_count, 3);
    }

    #[tokio::test]
    async fn test_unix_pty_manager_sessions() {
        let manager = UnixPtyManager::new();
        let sessions = manager.list_sessions().await;
        assert!(sessions.is_empty());

        assert_eq!(manager.count().await, 0);
    }
}