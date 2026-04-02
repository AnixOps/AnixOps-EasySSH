//! Native Terminal Launcher
//!
//! Provides cross-platform native terminal launching for EasySSH Lite.
//! Automatically detects available terminals and supports user preferences.

use crate::error::LiteError;
use crate::models::server::{AuthMethod, Server};
use std::path::Path;
use std::process::Command;

/// Terminal preference settings
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalPreference {
    /// Auto-detect the best available terminal
    Auto,
    /// Specific terminal (platform-dependent)
    Specific(String),
}

impl Default for TerminalPreference {
    fn default() -> Self {
        TerminalPreference::Auto
    }
}

/// Supported terminal emulators by platform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalType {
    // Windows
    WindowsTerminal,
    PowerShell,
    Cmd,
    GitBash,
    // Linux
    GnomeTerminal,
    Konsole,
    Xfce4Terminal,
    Xterm,
    Alacritty,
    Kitty,
    // macOS
    TerminalApp,
    ITerm2,
    Warp,
}

impl TerminalType {
    /// Get the executable name for this terminal
    pub fn executable(&self) -> &'static str {
        match self {
            // Windows
            TerminalType::WindowsTerminal => "wt",
            TerminalType::PowerShell => "powershell",
            TerminalType::Cmd => "cmd",
            TerminalType::GitBash => "git-bash",
            // Linux
            TerminalType::GnomeTerminal => "gnome-terminal",
            TerminalType::Konsole => "konsole",
            TerminalType::Xfce4Terminal => "xfce4-terminal",
            TerminalType::Xterm => "xterm",
            TerminalType::Alacritty => "alacritty",
            TerminalType::Kitty => "kitty",
            // macOS
            TerminalType::TerminalApp => "Terminal",
            TerminalType::ITerm2 => "iTerm",
            TerminalType::Warp => "Warp",
        }
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            TerminalType::WindowsTerminal => "Windows Terminal",
            TerminalType::PowerShell => "PowerShell",
            TerminalType::Cmd => "Command Prompt",
            TerminalType::GitBash => "Git Bash",
            TerminalType::GnomeTerminal => "GNOME Terminal",
            TerminalType::Konsole => "Konsole",
            TerminalType::Xfce4Terminal => "XFCE4 Terminal",
            TerminalType::Xterm => "XTerm",
            TerminalType::Alacritty => "Alacritty",
            TerminalType::Kitty => "Kitty",
            TerminalType::TerminalApp => "Terminal.app",
            TerminalType::ITerm2 => "iTerm2",
            TerminalType::Warp => "Warp",
        }
    }

    /// Get all terminals for current platform
    pub fn platform_terminals() -> Vec<TerminalType> {
        #[cfg(target_os = "windows")]
        {
            vec![
                TerminalType::WindowsTerminal,
                TerminalType::PowerShell,
                TerminalType::Cmd,
                TerminalType::GitBash,
            ]
        }

        #[cfg(target_os = "linux")]
        {
            vec![
                TerminalType::GnomeTerminal,
                TerminalType::Konsole,
                TerminalType::Xfce4Terminal,
                TerminalType::Xterm,
                TerminalType::Alacritty,
                TerminalType::Kitty,
            ]
        }

        #[cfg(target_os = "macos")]
        {
            vec![
                TerminalType::TerminalApp,
                TerminalType::ITerm2,
                TerminalType::Warp,
            ]
        }
    }
}

/// Detected terminal information
#[derive(Debug, Clone)]
pub struct DetectedTerminal {
    pub terminal_type: TerminalType,
    pub executable_path: Option<String>,
    pub priority: u8, // Lower = higher priority
}

/// Terminal launcher configuration
#[derive(Debug, Clone)]
pub struct TerminalLauncher {
    preference: TerminalPreference,
    custom_args: Vec<String>,
}

impl Default for TerminalLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalLauncher {
    /// Create a new terminal launcher with default settings
    pub fn new() -> Self {
        Self {
            preference: TerminalPreference::default(),
            custom_args: Vec::new(),
        }
    }

    /// Set terminal preference
    pub fn with_preference(mut self, preference: TerminalPreference) -> Self {
        self.preference = preference;
        self
    }

    /// Set custom launch arguments
    pub fn with_custom_args(mut self, args: Vec<String>) -> Self {
        self.custom_args = args;
        self
    }

    /// Detect all available terminals on the system
    pub fn detect_available_terminals() -> Vec<DetectedTerminal> {
        let mut detected = Vec::new();

        for (priority, terminal_type) in TerminalType::platform_terminals().iter().enumerate() {
            if let Some(path) = find_executable(terminal_type.executable()) {
                detected.push(DetectedTerminal {
                    terminal_type: *terminal_type,
                    executable_path: Some(path),
                    priority: priority as u8,
                });
            }
        }

        // Sort by priority
        detected.sort_by_key(|t| t.priority);
        detected
    }

    /// Get the best available terminal
    pub fn get_best_terminal(&self) -> Result<TerminalType, LiteError> {
        match &self.preference {
            TerminalPreference::Auto => {
                let available = Self::detect_available_terminals();
                if available.is_empty() {
                    return Err(LiteError::Terminal("No terminal found".to_string()));
                }
                Ok(available[0].terminal_type)
            }
            TerminalPreference::Specific(name) => {
                // Try to find matching terminal by name
                let name_lower = name.to_lowercase();
                for terminal in TerminalType::platform_terminals() {
                    if terminal.executable().to_lowercase() == name_lower
                        || terminal.display_name().to_lowercase() == name_lower
                    {
                        if is_command_available(terminal.executable()) {
                            return Ok(terminal);
                        }
                    }
                }
                Err(LiteError::Terminal(format!(
                    "Preferred terminal '{}' not found",
                    name
                )))
            }
        }
    }

    /// Launch terminal and connect to server via SSH
    pub fn launch(&self, server: &Server) -> Result<(), LiteError> {
        let terminal = self.get_best_terminal()?;
        let ssh_cmd = generate_ssh_command(server);

        match terminal {
            // Windows terminals
            TerminalType::WindowsTerminal => {
                launch_windows_terminal(&ssh_cmd, server)
            }
            TerminalType::PowerShell => {
                launch_powershell(&ssh_cmd, server)
            }
            TerminalType::Cmd => {
                launch_cmd(&ssh_cmd, server)
            }
            TerminalType::GitBash => {
                launch_gitbash(&ssh_cmd, server)
            }
            // Linux terminals
            TerminalType::GnomeTerminal => {
                launch_gnome_terminal(&ssh_cmd, server)
            }
            TerminalType::Konsole => {
                launch_konsole(&ssh_cmd, server)
            }
            TerminalType::Xfce4Terminal => {
                launch_xfce4_terminal(&ssh_cmd, server)
            }
            TerminalType::Xterm => {
                launch_xterm(&ssh_cmd, server)
            }
            TerminalType::Alacritty => {
                launch_alacritty(&ssh_cmd, server)
            }
            TerminalType::Kitty => {
                launch_kitty(&ssh_cmd, server)
            }
            // macOS terminals
            TerminalType::TerminalApp => {
                launch_terminal_app(&ssh_cmd, server)
            }
            TerminalType::ITerm2 => {
                launch_iterm2(&ssh_cmd, server)
            }
            TerminalType::Warp => {
                launch_warp(&ssh_cmd, server)
            }
        }
    }

    /// Launch terminal with a custom command
    pub fn launch_with_command(&self, command: &str, title: Option<&str>) -> Result<(), LiteError> {
        let terminal = self.get_best_terminal()?;

        match terminal {
            TerminalType::WindowsTerminal => {
                launch_windows_terminal_raw(command, title)
            }
            TerminalType::PowerShell => {
                launch_powershell_raw(command, title)
            }
            TerminalType::Cmd => {
                launch_cmd_raw(command, title)
            }
            TerminalType::GitBash => {
                launch_gitbash_raw(command, title)
            }
            TerminalType::GnomeTerminal => {
                launch_gnome_terminal_raw(command, title)
            }
            TerminalType::Konsole => {
                launch_konsole_raw(command, title)
            }
            TerminalType::Xfce4Terminal => {
                launch_xfce4_terminal_raw(command, title)
            }
            TerminalType::Xterm => {
                launch_xterm_raw(command, title)
            }
            TerminalType::Alacritty => {
                launch_alacritty_raw(command, title)
            }
            TerminalType::Kitty => {
                launch_kitty_raw(command, title)
            }
            TerminalType::TerminalApp => {
                launch_terminal_app_raw(command, title)
            }
            TerminalType::ITerm2 => {
                launch_iterm2_raw(command, title)
            }
            TerminalType::Warp => {
                launch_warp_raw(command, title)
            }
        }
    }
}

/// Generate SSH command string based on server configuration
pub fn generate_ssh_command(server: &Server) -> String {
    match &server.auth_method {
        AuthMethod::Password { .. } => {
            format!(
                "ssh -o PreferredAuthentications=password -p {} {}@{}",
                server.port, server.username, server.host
            )
        }
        AuthMethod::PrivateKey { key_path, .. } => {
            format!(
                "ssh -i {} -p {} {}@{}",
                escape_shell_arg(key_path),
                server.port,
                server.username,
                server.host
            )
        }
        AuthMethod::Agent => {
            format!(
                "ssh -p {} {}@{}",
                server.port, server.username, server.host
            )
        }
    }
}

/// Escape shell argument for safe command execution
fn escape_shell_arg(arg: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        // Windows: handle spaces and special characters
        if arg.contains(' ') || arg.contains('\\') || arg.contains('"') {
            format!("\"{}\"", arg.replace('"', "\"\""))
        } else {
            arg.to_string()
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Unix: single quote and escape
        if arg.contains('\'') {
            format!("'{}'", arg.replace('\'', "'\"'\"'"))
        } else if arg.contains(' ') || arg.contains(';') || arg.contains('&') || arg.contains('|') {
            format!("'{}'", arg)
        } else {
            arg.to_string()
        }
    }
}

/// Find executable in system PATH
fn find_executable(name: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        // Try where command first
        if let Ok(output) = Command::new("where").arg(name).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return stdout.lines().next().map(|s| s.trim().to_string());
            }
        }

        // Common installation paths for Windows terminals
        let common_paths = match name {
            "wt" => vec![
                "%LOCALAPPDATA%\\Microsoft\\WindowsApps\\wt.exe",
                "%ProgramFiles%\\WindowsApps\\Microsoft.WindowsTerminal_*\\wt.exe",
            ],
            "powershell" => vec![
                "%SystemRoot%\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
                "%WINDIR%\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
            ],
            "git-bash" => vec![
                "%ProgramFiles%\\Git\\git-bash.exe",
                "%ProgramFiles(x86)%\\Git\\git-bash.exe",
                "%LOCALAPPDATA%\\Programs\\Git\\git-bash.exe",
            ],
            _ => vec![],
        };

        for path_pattern in common_paths {
            let expanded = expand_windows_env_vars(path_pattern);
            if Path::new(&expanded).exists() {
                return Some(expanded);
            }
        }

        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(output) = Command::new("which").arg(name).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return Some(stdout.trim().to_string());
            }
        }

        // Check common paths
        let common_paths = vec![
            format!("/usr/bin/{}" , name),
            format!("/usr/local/bin/{}" , name),
            format!("/opt/{}/bin/{}" , name, name),
            format!("/Applications/{}.app/Contents/MacOS/{}" , capitalize(name), name),
        ];

        for path in common_paths {
            if Path::new(&path).exists() {
                return Some(path);
            }
        }

        None
    }
}

/// Check if command is available
fn is_command_available(cmd: &str) -> bool {
    find_executable(cmd).is_some()
}

/// Expand Windows environment variables in a path string
#[cfg(target_os = "windows")]
fn expand_windows_env_vars(path: &str) -> String {
    let mut result = path.to_string();

    // Replace common environment variables
    let vars_to_expand = [
        ("%LOCALAPPDATA%", std::env::var("LOCALAPPDATA").unwrap_or_default()),
        ("%ProgramFiles%", std::env::var("ProgramFiles").unwrap_or_default()),
        ("%ProgramFiles(x86)%", std::env::var("ProgramFiles(x86)").unwrap_or_default()),
        ("%SystemRoot%", std::env::var("SystemRoot").unwrap_or_else(|_| std::env::var("WINDIR").unwrap_or_default())),
        ("%WINDIR%", std::env::var("WINDIR").unwrap_or_default()),
    ];

    for (var_name, var_value) in vars_to_expand {
        result = result.replace(var_name, &var_value);
    }

    result
}

#[cfg(not(target_os = "windows"))]
fn expand_windows_env_vars(_path: &str) -> String {
    String::new()
}

#[cfg(not(target_os = "windows"))]
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
    }
}

// ============================================================================
// Windows Terminal Launchers
// ============================================================================

#[cfg(target_os = "windows")]
fn launch_windows_terminal(ssh_cmd: &str, server: &Server) -> Result<(), LiteError> {
    let in_wt = std::env::var("WT_SESSION").is_ok();

    let mut cmd = Command::new("wt");

    if in_wt {
        // Already in Windows Terminal, open new tab
        cmd.arg("new-tab")
            .arg("--title")
            .arg(format!("SSH: {}", server.name));
    } else {
        // New window
        cmd.arg("--title").arg(format!("SSH: {}", server.name));
    }

    cmd.arg("powershell.exe")
        .arg("-NoExit")
        .arg("-Command")
        .arg(ssh_cmd);

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Windows Terminal: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_powershell(ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Command::new("powershell.exe")
        .arg("-NoExit")
        .arg("-Command")
        .arg(ssh_cmd)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("PowerShell: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_cmd(ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Command::new("cmd.exe")
        .arg("/K")
        .arg(ssh_cmd)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("CMD: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_gitbash(ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Command::new("git-bash.exe")
        .arg("-c")
        .arg(ssh_cmd)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Git Bash: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_windows_terminal_raw(command: &str, title: Option<&str>) -> Result<(), LiteError> {
    let mut cmd = Command::new("wt");

    if let Some(t) = title {
        cmd.arg("--title").arg(t);
    }

    cmd.arg("powershell.exe")
        .arg("-NoExit")
        .arg("-Command")
        .arg(command);

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Windows Terminal: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_powershell_raw(command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Command::new("powershell.exe")
        .arg("-NoExit")
        .arg("-Command")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("PowerShell: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_cmd_raw(command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Command::new("cmd.exe")
        .arg("/K")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("CMD: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_gitbash_raw(command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Command::new("git-bash.exe")
        .arg("-c")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Git Bash: {}", e)))?;

    Ok(())
}

// ============================================================================
// Linux Terminal Launchers
// ============================================================================

#[cfg(target_os = "linux")]
fn launch_gnome_terminal(ssh_cmd: &str, server: &Server) -> Result<(), LiteError> {
    let title = format!("SSH: {}", server.name);
    launch_gnome_terminal_raw(ssh_cmd, Some(&title))
}

#[cfg(target_os = "linux")]
fn launch_gnome_terminal_raw(command: &str, title: Option<&str>) -> Result<(), LiteError> {
    let mut cmd = Command::new("gnome-terminal");

    if let Some(t) = title {
        cmd.arg("--title").arg(t);
    }

    cmd.arg("--").arg("bash").arg("-c").arg(format!(
        "{}; read -p 'Press Enter to exit...'",
        command
    ));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("GNOME Terminal: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_konsole(ssh_cmd: &str, server: &Server) -> Result<(), LiteError> {
    let title = format!("SSH: {}", server.name);
    launch_konsole_raw(ssh_cmd, Some(&title))
}

#[cfg(target_os = "linux")]
fn launch_konsole_raw(command: &str, title: Option<&str>) -> Result<(), LiteError> {
    let mut cmd = Command::new("konsole");

    if let Some(t) = title {
        cmd.arg("--title").arg(t);
    }

    cmd.arg("-e").arg("bash").arg("-c").arg(format!(
        "{}; read -p 'Press Enter to exit...'",
        command
    ));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Konsole: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_xfce4_terminal(ssh_cmd: &str, server: &Server) -> Result<(), LiteError> {
    let title = format!("SSH: {}", server.name);
    launch_xfce4_terminal_raw(ssh_cmd, Some(&title))
}

#[cfg(target_os = "linux")]
fn launch_xfce4_terminal_raw(command: &str, title: Option<&str>) -> Result<(), LiteError> {
    let mut cmd = Command::new("xfce4-terminal");

    if let Some(t) = title {
        cmd.arg("--title").arg(t);
    }

    cmd.arg("-e").arg(format!(
        "bash -c \"{}; read -p 'Press Enter to exit...'\"",
        command.replace('"', "\\\"")
    ));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("XFCE4 Terminal: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_xterm(ssh_cmd: &str, server: &Server) -> Result<(), LiteError> {
    let title = format!("SSH: {}", server.name);
    launch_xterm_raw(ssh_cmd, Some(&title))
}

#[cfg(target_os = "linux")]
fn launch_xterm_raw(command: &str, title: Option<&str>) -> Result<(), LiteError> {
    let mut cmd = Command::new("xterm");

    if let Some(t) = title {
        cmd.arg("-title").arg(t);
    }

    cmd.arg("-e").arg("bash").arg("-c").arg(format!(
        "{}; read -p 'Press Enter to exit...'",
        command
    ));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("XTerm: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_alacritty(ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    launch_alacritty_raw(ssh_cmd, None)
}

#[cfg(target_os = "linux")]
fn launch_alacritty_raw(command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Command::new("alacritty")
        .arg("-e").arg("bash").arg("-c").arg(format!(
            "{}; read -p 'Press Enter to exit...'",
            command
        ))
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Alacritty: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_kitty(ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    launch_kitty_raw(ssh_cmd, None)
}

#[cfg(target_os = "linux")]
fn launch_kitty_raw(command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Command::new("kitty")
        .arg("-e").arg("bash").arg("-c").arg(format!(
            "{}; read -p 'Press Enter to exit...'",
            command
        ))
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Kitty: {}", e)))?;

    Ok(())
}

// ============================================================================
// macOS Terminal Launchers
// ============================================================================

#[cfg(target_os = "macos")]
fn launch_terminal_app(ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    launch_terminal_app_raw(ssh_cmd, None)
}

#[cfg(target_os = "macos")]
fn launch_terminal_app_raw(command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    let script = format!(
        r#"tell application "Terminal"
            if not running then launch
            activate
            do script "{}"
        end tell"#,
        command.replace('"', "\\\"")
    );

    Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Terminal.app: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_iterm2(ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    launch_iterm2_raw(ssh_cmd, None)
}

#[cfg(target_os = "macos")]
fn launch_iterm2_raw(command: &str, _title: Option<&str>) -> Result<(), LiteError> {
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
        command.replace('"', "\\\"")
    );

    Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("iTerm2: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_warp(ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    launch_warp_raw(ssh_cmd, None)
}

#[cfg(target_os = "macos")]
fn launch_warp_raw(command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    // Warp doesn't have a direct CLI, use open command with URL scheme or app path
    Command::new("open")
        .arg("-a")
        .arg("Warp")
        .arg("--args")
        .arg("-e")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Warp: {}", e)))?;

    Ok(())
}

// ============================================================================
// Stub implementations for non-target platforms (for compilation)
// ============================================================================

#[cfg(not(target_os = "windows"))]
fn launch_windows_terminal(_ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Windows Terminal not available on this platform".to_string()))
}

#[cfg(not(target_os = "windows"))]
fn launch_powershell(_ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Err(LiteError::Terminal("PowerShell not available on this platform".to_string()))
}

#[cfg(not(target_os = "windows"))]
fn launch_cmd(_ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Err(LiteError::Terminal("CMD not available on this platform".to_string()))
}

#[cfg(not(target_os = "windows"))]
fn launch_gitbash(_ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Git Bash not available on this platform".to_string()))
}

#[cfg(not(target_os = "linux"))]
fn launch_gnome_terminal(_ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Err(LiteError::Terminal("GNOME Terminal not available on this platform".to_string()))
}

#[cfg(not(target_os = "linux"))]
fn launch_konsole(_ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Konsole not available on this platform".to_string()))
}

#[cfg(not(target_os = "linux"))]
fn launch_xfce4_terminal(_ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Err(LiteError::Terminal("XFCE4 Terminal not available on this platform".to_string()))
}

#[cfg(not(target_os = "linux"))]
fn launch_xterm(_ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Err(LiteError::Terminal("XTerm not available on this platform".to_string()))
}

#[cfg(not(target_os = "linux"))]
fn launch_alacritty(_ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Alacritty not available on this platform".to_string()))
}

#[cfg(not(target_os = "linux"))]
fn launch_kitty(_ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Kitty not available on this platform".to_string()))
}

#[cfg(not(target_os = "macos"))]
fn launch_terminal_app(_ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Terminal.app not available on this platform".to_string()))
}

#[cfg(not(target_os = "macos"))]
fn launch_iterm2(_ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Err(LiteError::Terminal("iTerm2 not available on this platform".to_string()))
}

#[cfg(not(target_os = "macos"))]
fn launch_warp(_ssh_cmd: &str, _server: &Server) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Warp not available on this platform".to_string()))
}

// Raw command stubs
#[cfg(not(target_os = "windows"))]
fn launch_windows_terminal_raw(_command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Windows Terminal not available on this platform".to_string()))
}

#[cfg(not(target_os = "windows"))]
fn launch_powershell_raw(_command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Err(LiteError::Terminal("PowerShell not available on this platform".to_string()))
}

#[cfg(not(target_os = "windows"))]
fn launch_cmd_raw(_command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Err(LiteError::Terminal("CMD not available on this platform".to_string()))
}

#[cfg(not(target_os = "windows"))]
fn launch_gitbash_raw(_command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Git Bash not available on this platform".to_string()))
}

#[cfg(not(target_os = "linux"))]
fn launch_gnome_terminal_raw(_command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Err(LiteError::Terminal("GNOME Terminal not available on this platform".to_string()))
}

#[cfg(not(target_os = "linux"))]
fn launch_konsole_raw(_command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Konsole not available on this platform".to_string()))
}

#[cfg(not(target_os = "linux"))]
fn launch_xfce4_terminal_raw(_command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Err(LiteError::Terminal("XFCE4 Terminal not available on this platform".to_string()))
}

#[cfg(not(target_os = "linux"))]
fn launch_xterm_raw(_command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Err(LiteError::Terminal("XTerm not available on this platform".to_string()))
}

#[cfg(not(target_os = "linux"))]
fn launch_alacritty_raw(_command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Alacritty not available on this platform".to_string()))
}

#[cfg(not(target_os = "linux"))]
fn launch_kitty_raw(_command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Kitty not available on this platform".to_string()))
}

#[cfg(not(target_os = "macos"))]
fn launch_terminal_app_raw(_command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Terminal.app not available on this platform".to_string()))
}

#[cfg(not(target_os = "macos"))]
fn launch_iterm2_raw(_command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Err(LiteError::Terminal("iTerm2 not available on this platform".to_string()))
}

#[cfg(not(target_os = "macos"))]
fn launch_warp_raw(_command: &str, _title: Option<&str>) -> Result<(), LiteError> {
    Err(LiteError::Terminal("Warp not available on this platform".to_string()))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::server::{AuthMethod, Server};

    fn create_test_server(auth_method: AuthMethod) -> Server {
        Server::new(
            "Test Server".to_string(),
            "192.168.1.1".to_string(),
            22,
            "admin".to_string(),
            auth_method,
            None,
        )
    }

    #[test]
    fn test_generate_ssh_command_agent() {
        let server = create_test_server(AuthMethod::Agent);
        let cmd = generate_ssh_command(&server);
        assert_eq!(cmd, "ssh -p 22 admin@192.168.1.1");
    }

    #[test]
    fn test_generate_ssh_command_password() {
        let server = create_test_server(AuthMethod::Password {
            password: "secret".to_string(),
        });
        let cmd = generate_ssh_command(&server);
        assert_eq!(
            cmd,
            "ssh -o PreferredAuthentications=password -p 22 admin@192.168.1.1"
        );
    }

    #[test]
    fn test_generate_ssh_command_key() {
        let server = create_test_server(AuthMethod::PrivateKey {
            key_path: "/home/user/.ssh/id_rsa".to_string(),
            passphrase: None,
        });
        let cmd = generate_ssh_command(&server);
        assert!(cmd.contains("ssh"));
        assert!(cmd.contains("-i"));
        assert!(cmd.contains("/home/user/.ssh/id_rsa"));
        assert!(cmd.contains("-p 22"));
        assert!(cmd.contains("admin@192.168.1.1"));
    }

    #[test]
    fn test_terminal_type_display_name() {
        assert_eq!(TerminalType::WindowsTerminal.display_name(), "Windows Terminal");
        assert_eq!(TerminalType::PowerShell.display_name(), "PowerShell");
        assert_eq!(TerminalType::GnomeTerminal.display_name(), "GNOME Terminal");
        assert_eq!(TerminalType::ITerm2.display_name(), "iTerm2");
    }

    #[test]
    fn test_terminal_type_executable() {
        assert_eq!(TerminalType::WindowsTerminal.executable(), "wt");
        assert_eq!(TerminalType::PowerShell.executable(), "powershell");
        assert_eq!(TerminalType::GnomeTerminal.executable(), "gnome-terminal");
        assert_eq!(TerminalType::ITerm2.executable(), "iTerm");
    }

    #[test]
    fn test_terminal_preference_default() {
        let pref = TerminalPreference::default();
        assert!(matches!(pref, TerminalPreference::Auto));
    }

    #[test]
    fn test_terminal_launcher_builder() {
        let launcher = TerminalLauncher::new()
            .with_preference(TerminalPreference::Specific("alacritty".to_string()))
            .with_custom_args(vec!["--hold".to_string()]);

        assert!(matches!(
            launcher.preference,
            TerminalPreference::Specific(_)
        ));
        assert_eq!(launcher.custom_args.len(), 1);
    }

    #[test]
    fn test_escape_shell_arg_unix() {
        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(escape_shell_arg("simple"), "simple");
            assert_eq!(escape_shell_arg("with space"), "'with space'");
        }
    }

    #[test]
    fn test_terminal_platform_terminals() {
        let terminals = TerminalType::platform_terminals();
        assert!(!terminals.is_empty());

        #[cfg(target_os = "windows")]
        {
            assert!(terminals.contains(&TerminalType::WindowsTerminal));
            assert!(terminals.contains(&TerminalType::PowerShell));
        }

        #[cfg(target_os = "linux")]
        {
            assert!(terminals.contains(&TerminalType::GnomeTerminal));
            assert!(terminals.contains(&TerminalType::Xterm));
        }

        #[cfg(target_os = "macos")]
        {
            assert!(terminals.contains(&TerminalType::TerminalApp));
            assert!(terminals.contains(&TerminalType::ITerm2));
        }
    }

    #[test]
    fn test_detected_terminal_creation() {
        let detected = DetectedTerminal {
            terminal_type: TerminalType::GnomeTerminal,
            executable_path: Some("/usr/bin/gnome-terminal".to_string()),
            priority: 0,
        };

        assert_eq!(detected.terminal_type, TerminalType::GnomeTerminal);
        assert!(detected.executable_path.is_some());
        assert_eq!(detected.priority, 0);
    }
}
