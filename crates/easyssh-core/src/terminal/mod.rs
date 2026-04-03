//! Terminal Module
//!
//! This module provides terminal functionality for all EasySSH editions:
//! - **Lite Edition**: Native terminal launcher (uses system SSH client)
//! - **Standard/Pro Editions**: Embedded terminal emulator with WebGL rendering
//!
//! # Feature Flags
//!
//! The module behavior is controlled by feature flags:
//! - `embedded-terminal` - Enables embedded terminal emulator (Standard/Pro)
//! - Without the feature, only native terminal launching is available (Lite)
//!
//! # Architecture
//!
//! ## Native Terminal (Lite)
//! The native launcher (`launcher` module) detects the platform's available terminals
//! and spawns them with appropriate SSH command arguments.
//!
//! Supported terminals:
//! - Windows: Windows Terminal, PowerShell, CMD
//! - macOS: Terminal.app, iTerm2
//! - Linux: GNOME Terminal, Konsole, xterm, alacritty, etc.
//!
//! ## Embedded Terminal (Standard/Pro)
//! The embedded terminal provides a full terminal emulator with:
//! - PTY (pseudo-terminal) management
//! - WebSocket bridge for UI communication
//! - xterm.js compatibility layer
//! - WebGL-accelerated rendering
//! - Multi-tab support
//! - Theme management
//!
//! # Submodules
//!
//! | Module | Feature | Description |
//! |--------|---------|-------------|
//! | `launcher` | Always | Native terminal detection and launching |
//! | `embedded` | `embedded-terminal` | PTY and terminal emulator core |
//! | `multitab` | `embedded-terminal` | Tab management |
//! | `theme` | `embedded-terminal` | Color schemes and theming |
//! | `webgl` | `embedded-terminal` | WebGL renderer |
//! | `xterm_compat` | `embedded-terminal` | xterm.js compatibility |
//!
//! # Example
//!
//! ## Native Terminal (Lite)
//!
//! ```rust,no_run
//! use easyssh_core::terminal::{open_native_terminal, generate_ssh_command};
//!
//! // Open native terminal with SSH connection
//! open_native_terminal("192.168.1.1", 22, "root", "agent").unwrap();
//!
//! // Or generate SSH command for manual use
//! let cmd = generate_ssh_command("192.168.1.1", 22, "root", true);
//! println!("SSH command: {}", cmd);
//! ```
//!
//! ## Embedded Terminal (Standard/Pro)
//!
//! ```rust,ignore
//! use easyssh_core::terminal::{TerminalManager, TerminalServerConfig};
//!
//! // Create terminal manager
//! let config = TerminalServerConfig::default();
//! let manager = TerminalManager::new(config);
//!
//! // Start terminal session
//! let session = manager.create_session("session-1", "192.168.1.1", 22, "root").await.unwrap();
//! ```

use crate::error::LiteError;
use std::process::Command;

// Submodules
#[cfg(feature = "embedded-terminal")]
pub mod coordinator;
#[cfg(feature = "embedded-terminal")]
pub mod embedded;
#[cfg(feature = "embedded-terminal")]
pub mod multitab;
#[cfg(feature = "embedded-terminal")]
pub mod theme;
#[cfg(feature = "embedded-terminal")]
pub mod webgl;
#[cfg(feature = "embedded-terminal")]
pub mod xterm_compat;

// Native terminal launcher (available for all editions)
pub mod launcher;

// Export launcher types
pub use launcher::{
    generate_ssh_command, DetectedTerminal, TerminalLauncher, TerminalPreference, TerminalType,
};

// Export core types
#[cfg(feature = "embedded-terminal")]
pub use embedded::{
    BridgeConfig, BridgeStats, ClientInfo, EmbeddedTerminalServer, PtyInstance, PtyManager,
    PtyManagerStats, PtyStatus, PtyType, RenderPerformance, RendererConfig, RendererManager,
    RendererType, SessionInfo, TerminalCoordinator, TerminalEmulator, TerminalServerConfig,
    TerminalWsMessage, TerminalWsOutput, WebSocketBridge,
};
#[cfg(feature = "embedded-terminal")]
pub use embedded::{PtyTerminal, TerminalManager};
#[cfg(feature = "embedded-terminal")]
pub use multitab::{TabInfo, TabManager, TabState};
#[cfg(feature = "embedded-terminal")]
pub use theme::{ColorPalette, CursorStyle, TerminalTheme, ThemeManager};
#[cfg(feature = "embedded-terminal")]
pub use webgl::{RenderStats, WebGlConfig, WebGlRenderer};
#[cfg(feature = "embedded-terminal")]
pub use xterm_compat::{EscapeSequence, XtermCompat, XtermMode};

// Export coordinator types
#[cfg(feature = "embedded-terminal")]
pub use coordinator::{
    ConnectionState, CoordinatorConfig, CoordinatorEvent, CoordinatorStats, SessionCoordinator,
    SessionTabMapping,
};

// ============ Native Terminal Launch (Lite Version) ============

/// SSH connection parameters for native terminal launcher.
///
/// `SshArgs` encapsulates all parameters needed to construct an SSH command
/// for connecting to a remote server. It handles authentication method detection
/// and generates appropriate command-line arguments.
///
/// # Example
///
/// ```rust
/// use easyssh_core::terminal::SshArgs;
///
/// // Create with key/agent authentication (enables agent forwarding)
/// let args = SshArgs::new("192.168.1.1", 22, "root", "key");
/// assert!(args.forward_agent);
///
/// // Create with password authentication (no agent forwarding)
/// let args = SshArgs::new("192.168.1.1", 22, "root", "password");
/// assert!(!args.forward_agent);
///
/// // Generate command arguments
/// let cmd_args = args.to_args();
/// println!("SSH args: {:?}", cmd_args);
///
/// // Generate full command string
/// let cmd_string = args.to_command_string();
/// println!("{}", cmd_string); // ssh -p 22 -A root@192.168.1.1
/// ```
pub struct SshArgs {
    /// Target host address (IP or hostname)
    pub host: String,
    /// SSH port number (typically 22)
    pub port: u16,
    /// Username for authentication
    pub username: String,
    /// Whether to enable SSH agent forwarding (-A flag)
    pub forward_agent: bool,
}

impl SshArgs {
    /// Create new SSH arguments.
    ///
    /// Automatically detects whether to enable agent forwarding based on the
    /// authentication type. Key and agent authentication enable forwarding,
    /// while password authentication does not.
    ///
    /// # Arguments
    ///
    /// * `host` - Target host address (IP or hostname)
    /// * `port` - SSH port number
    /// * `username` - Username for authentication
    /// * `auth_type` - Authentication type: "key", "agent", or "password"
    ///
    /// # Returns
    ///
    /// A new `SshArgs` instance with appropriate settings.
    pub fn new(host: &str, port: u16, username: &str, auth_type: &str) -> Self {
        let forward_agent = matches!(auth_type, "key" | "agent");
        Self {
            host: host.to_string(),
            port,
            username: username.to_string(),
            forward_agent,
        }
    }

    /// Build SSH command arguments list.
    ///
    /// Generates the arguments portion of the SSH command (everything after `ssh`).
    /// Includes port specification and agent forwarding flag if enabled.
    ///
    /// # Returns
    ///
    /// A vector of argument strings suitable for passing to `std::process::Command`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use easyssh_core::terminal::SshArgs;
    ///
    /// let args = SshArgs::new("example.com", 2222, "user", "key");
    /// let cmd_args = args.to_args();
    /// assert_eq!(cmd_args, vec!["-p", "2222", "-A", "user@example.com"]);
    /// ```
    pub fn to_args(&self) -> Vec<String> {
        let mut args = vec!["-p".to_string(), self.port.to_string()];
        if self.forward_agent {
            args.push("-A".to_string());
        }
        args.push(format!("{}@{}", self.username, self.host));
        args
    }

    /// Build a complete SSH command string.
    ///
    /// Generates a single-line SSH command ready for execution or display.
    /// The command includes the `ssh` prefix followed by all arguments.
    ///
    /// # Returns
    ///
    /// A formatted SSH command string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use easyssh_core::terminal::SshArgs;
    ///
    /// let args = SshArgs::new("192.168.1.1", 22, "root", "key");
    /// let cmd = args.to_command_string();
    /// assert_eq!(cmd, "ssh -p 22 -A root@192.168.1.1");
    /// ```
    pub fn to_command_string(&self) -> String {
        let args = self.to_args();
        format!("ssh {}", args.join(" "))
    }
}

/// Open a native terminal and connect via SSH (Windows implementation).
///
/// On Windows, this function:
/// 1. Checks if running inside Windows Terminal (WT_SESSION env var)
/// 2. If in Windows Terminal, opens a new tab with the SSH command
/// 3. Falls back to PowerShell if Windows Terminal is not available
///
/// # Arguments
///
/// * `host` - Target host address (IP or hostname)
/// * `port` - SSH port number
/// * `username` - Username for SSH authentication
/// * `auth_type` - Authentication type: "key", "agent", or "password"
///
/// # Errors
///
/// Returns `LiteError::Terminal` if the terminal cannot be opened or
/// if the SSH command fails to execute.
///
/// # Example
///
/// ```rust,ignore
/// use easyssh_core::terminal::open_native_terminal;
///
/// // Connect to a server
/// open_native_terminal("192.168.1.100", 22, "admin", "key").unwrap();
/// ```
#[cfg(target_os = "windows")]
pub fn open_native_terminal(
    host: &str,
    port: u16,
    username: &str,
    auth_type: &str,
) -> Result<(), LiteError> {
    let ssh_args = SshArgs::new(host, port, username, auth_type);
    let ssh_cmd = ssh_args.to_command_string();

    // Check if running in Windows Terminal
    let in_wt = std::env::var("WT_SESSION").is_ok();

    if in_wt && is_command_available("wt") {
        // In Windows Terminal, open new tab
        let mut cmd = Command::new("wt");
        cmd.arg("new-tab")
            .arg("--title")
            .arg(format!("SSH: {}", host))
            .arg("powershell.exe")
            .arg("-NoExit")
            .arg("-Command")
            .arg(&ssh_cmd);

        match cmd.spawn() {
            Ok(_) => return Ok(()),
            Err(e) => {
                log::warn!("wt new-tab failed: {}", e);
            }
        }
    }

    // Not in Windows Terminal, use PowerShell directly
    let mut child = Command::new("powershell.exe")
        .arg("-Command")
        .arg(&ssh_cmd)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("PowerShell: {}", e)))?;

    child.wait().ok();
    Ok(())
}

/// Open a native terminal and connect via SSH (macOS implementation).
///
/// On macOS, this function tries multiple terminal emulators in order:
/// 1. **iTerm2** (if available) - Creates a new tab with the SSH command
/// 2. **Terminal.app** (built-in) - Opens a new window with the SSH command
///
/// Both use AppleScript (`osascript`) to control the terminals.
///
/// # Arguments
///
/// * `host` - Target host address (IP or hostname)
/// * `port` - SSH port number
/// * `username` - Username for SSH authentication
/// * `auth_type` - Authentication type: "key", "agent", or "password"
///
/// # Errors
///
/// Returns `LiteError::Terminal` if no suitable terminal is found or
/// if the AppleScript execution fails.
///
/// # Example
///
/// ```rust,ignore
/// use easyssh_core::terminal::open_native_terminal;
///
/// // Connect using key authentication (will use SSH agent)
/// open_native_terminal("server.example.com", 22, "user", "key").unwrap();
/// ```
#[cfg(target_os = "macos")]
pub fn open_native_terminal(
    host: &str,
    port: u16,
    username: &str,
    auth_type: &str,
) -> Result<(), LiteError> {
    let ssh_args = SshArgs::new(host, port, username, auth_type);
    let ssh_cmd = ssh_args.to_command_string();

    // 优先使用 iTerm2
    if is_command_available("osascript") {
        let script = format!(
            r#"tell application "iTerm"
                if not running then launch
                activate
                tell current window
                    create tab with default profile
                    tell current session
                        write text "{}"
                    end tell
                end tell
            end tell"#,
            ssh_cmd.replace('"', r#"\"#)
        );

        match Command::new("osascript").arg("-e").arg(&script).spawn() {
            Ok(_) => return Ok(()),
            Err(_) => {}
        }
    }

    // 回退到 Terminal.app
    let script = format!(
        r#"tell application "Terminal"
            if not running then launch
            activate
            do script "{}"
        end tell"#,
        ssh_cmd.replace('"', "\\\"")
    );

    Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Terminal.app: {}", e)))?;

    Ok(())
}

/// Open a native terminal and connect via SSH (Linux implementation).
///
/// On Linux, this function tries multiple terminal emulators in priority order:
/// 1. **GNOME Terminal** (`gnome-terminal`)
/// 2. **KDE Konsole** (`konsole`)
/// 3. **XFCE Terminal** (`xfce4-terminal`)
/// 4. **XTerm** (`xterm`) - fallback for minimal systems
///
/// The SSH command is wrapped with a prompt that waits for user input before
/// closing the terminal, ensuring the user can see any output or error messages.
///
/// # Arguments
///
/// * `host` - Target host address (IP or hostname)
/// * `port` - SSH port number
/// * `username` - Username for SSH authentication
/// * `auth_type` - Authentication type: "key", "agent", or "password"
///
/// # Errors
///
/// Returns `LiteError::Terminal` if no terminal emulator is found on the system
/// or if the terminal fails to launch.
///
/// # Example
///
/// ```rust,ignore
/// use easyssh_core::terminal::open_native_terminal;
///
/// // Will try gnome-terminal, konsole, xfce4-terminal, then xterm
/// open_native_terminal("192.168.1.1", 22, "root", "password").unwrap();
/// ```
#[cfg(target_os = "linux")]
pub fn open_native_terminal(
    host: &str,
    port: u16,
    username: &str,
    auth_type: &str,
) -> Result<(), LiteError> {
    let ssh_args = SshArgs::new(host, port, username, auth_type);
    let ssh_cmd = ssh_args.to_command_string();

    // Try terminals in priority order
    let terminals: [(&str, &[&str]); 4] = [
        ("gnome-terminal", &["--", "bash", "-c"]),
        ("konsole", &["-e", "bash", "-c"]),
        ("xfce4-terminal", &["-e", "bash", "-c"]),
        ("xterm", &["-e", "bash", "-c"]),
    ];

    for (terminal, args) in terminals {
        if is_command_available(terminal) {
            let mut cmd = Command::new(terminal);
            for arg in args {
                cmd.arg(arg);
            }
            cmd.arg(format!("{}; read -p \"Press Enter to exit...\"", ssh_cmd));

            cmd.spawn()
                .map_err(|e| LiteError::Terminal(format!("{}: {}", terminal, e)))?;
            return Ok(());
        }
    }

    Err(LiteError::Terminal(
        "No available terminal found".to_string(),
    ))
}

/// 检查命令是否可用
fn is_command_available(cmd: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("where")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

// ============ 共享类型定义 ============

/// 终端尺寸
#[derive(Debug, Clone, Copy)]
pub struct TerminalSize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

impl TerminalSize {
    /// 创建新的终端尺寸
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    /// 设置像素尺寸
    pub fn with_pixels(mut self, width: u16, height: u16) -> Self {
        self.pixel_width = width;
        self.pixel_height = height;
        self
    }
}

#[cfg(feature = "embedded-terminal")]
impl TerminalSize {
    pub fn to_pty_size(&self) -> portable_pty::PtySize {
        portable_pty::PtySize {
            rows: self.rows,
            cols: self.cols,
            pixel_width: self.pixel_width,
            pixel_height: self.pixel_height,
        }
    }
}

/// 终端输出数据
#[derive(Debug, Clone)]
pub enum TerminalOutput {
    Data(String),
    Title(String),
    Closed,
    Error(String),
}

/// 终端输入数据
#[derive(Debug, Clone)]
pub enum TerminalInput {
    Data(String),
    Resize(TerminalSize),
    Signal(TerminalSignal),
}

/// 终端信号
#[derive(Debug, Clone, Copy)]
pub enum TerminalSignal {
    Interrupt = 3, // Ctrl+C, ETX
    Eof = 4,       // Ctrl+D, EOT
    Suspend = 26,  // Ctrl+Z, SUB
    Quit = 28,     // Ctrl+\, FS
}

impl TerminalSignal {
    /// 获取信号的字节值
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

/// 终端会话信息
#[derive(Debug, Clone)]
pub struct TerminalSession {
    pub id: String,
    pub title: String,
    pub server_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub size: TerminalSize,
}

/// 终端性能统计
#[derive(Debug, Clone, Default)]
pub struct TerminalStats {
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub frames_rendered: u64,
    pub avg_fps: f32,
    pub latency_ms: f32,
}

// ============ 单元测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_args_new() {
        let args = SshArgs::new("192.168.1.1", 22, "admin", "password");
        assert_eq!(args.host, "192.168.1.1");
        assert_eq!(args.port, 22);
        assert_eq!(args.username, "admin");
        assert!(!args.forward_agent);
    }

    #[test]
    fn test_ssh_args_with_agent() {
        let args = SshArgs::new("192.168.1.1", 22, "admin", "key");
        assert!(args.forward_agent);
    }

    #[test]
    fn test_ssh_args_to_command() {
        let args = SshArgs::new("example.com", 2222, "user", "key");
        let cmd = args.to_command_string();
        assert!(cmd.contains("ssh"));
        assert!(cmd.contains("-p 2222"));
        assert!(cmd.contains("-A"));
        assert!(cmd.contains("user@example.com"));
    }

    #[test]
    fn test_terminal_size_default() {
        let size = TerminalSize::default();
        assert_eq!(size.rows, 24);
        assert_eq!(size.cols, 80);
    }

    #[test]
    fn test_terminal_size_new() {
        let size = TerminalSize::new(30, 100);
        assert_eq!(size.rows, 30);
        assert_eq!(size.cols, 100);
    }

    #[test]
    fn test_terminal_signal_as_str() {
        assert_eq!(TerminalSignal::Interrupt as u8, 3);
        assert_eq!(TerminalSignal::Eof as u8, 4);
    }
}
