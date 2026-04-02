//! Terminal module
//! Provides native terminal launching (Lite) and embedded terminal emulator (Standard/Pro)

use crate::error::LiteError;
use std::process::Command;

// Submodules
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
pub use embedded::{PtyTerminal, TerminalEmulator, TerminalManager};
#[cfg(feature = "embedded-terminal")]
pub use multitab::{TabInfo, TabManager, TabState};
#[cfg(feature = "embedded-terminal")]
pub use theme::{ColorPalette, CursorStyle, TerminalTheme, ThemeManager};
#[cfg(feature = "embedded-terminal")]
pub use webgl::{RenderStats, WebGlConfig, WebGlRenderer};
#[cfg(feature = "embedded-terminal")]
pub use xterm_compat::{EscapeSequence, XtermCompat, XtermMode};

// ============ Native Terminal Launch (Lite Version) ============

/// SSH connection parameters
pub struct SshArgs {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub forward_agent: bool,
}

impl SshArgs {
    pub fn new(host: &str, port: u16, username: &str, auth_type: &str) -> Self {
        let forward_agent = matches!(auth_type, "key" | "agent");
        Self {
            host: host.to_string(),
            port,
            username: username.to_string(),
            forward_agent,
        }
    }

    /// Build SSH command arguments list
    pub fn to_args(&self) -> Vec<String> {
        let mut args = vec!["-p".to_string(), self.port.to_string()];
        if self.forward_agent {
            args.push("-A".to_string());
        }
        args.push(format!("{}@{}", self.username, self.host));
        args
    }

    /// Build single line command string
    pub fn to_command_string(&self) -> String {
        let args = self.to_args();
        format!("ssh {}", args.join(" "))
    }
}

#[cfg(target_os = "windows")]
pub fn open_native_terminal(
    host: &str,
    port: u16,
    username: &str,
    auth_type: &str,
) -> Result<(), LiteError> {
    let ssh_args = SshArgs::new(host, port, username, auth_type);
    let ssh_cmd = ssh_args.to_command_string();

    // 检查是否在 Windows Terminal 中运行
    let in_wt = std::env::var("WT_SESSION").is_ok();

    if in_wt && is_command_available("wt") {
        // 在 WT 中运行，使用 new-tab 在当前窗口开标签页
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

    // 不在 WT 中，直接在本地终端执行 SSH
    // 这会暂停 TUI，在当前终端运行 SSH，退出后恢复 TUI
    let mut child = Command::new("powershell.exe")
        .arg("-Command")
        .arg(&ssh_cmd)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("PowerShell: {}", e)))?;

    // 等待 SSH 会话结束
    child.wait().ok();

    Ok(())
}

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
