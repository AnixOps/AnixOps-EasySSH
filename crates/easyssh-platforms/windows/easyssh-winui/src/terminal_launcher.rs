//! Terminal Launcher Module for EasySSH Lite
//!
//! Provides native terminal launching functionality for Windows:
//! - Windows Terminal (preferred)
//! - PowerShell
//! - CMD

use std::process::Command;

/// Terminal preference options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalPreference {
    WindowsTerminal,
    PowerShell,
    Cmd,
    Auto, // Automatically detect best available
}

impl TerminalPreference {
    pub fn name(&self) -> &'static str {
        match self {
            TerminalPreference::WindowsTerminal => "Windows Terminal",
            TerminalPreference::PowerShell => "PowerShell",
            TerminalPreference::Cmd => "Command Prompt",
            TerminalPreference::Auto => "Auto-detect",
        }
    }

    pub fn all_options() -> Vec<Self> {
        vec![
            TerminalPreference::Auto,
            TerminalPreference::WindowsTerminal,
            TerminalPreference::PowerShell,
            TerminalPreference::Cmd,
        ]
    }
}

/// SSH connection parameters
#[derive(Debug, Clone)]
pub struct SshConnection {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String,
    pub identity_file: Option<String>,
}

impl SshConnection {
    pub fn new(
        host: String,
        port: u16,
        username: String,
        auth_type: String,
        identity_file: Option<String>,
    ) -> Self {
        Self {
            host,
            port,
            username,
            auth_type,
            identity_file,
        }
    }

    /// Build SSH command arguments
    fn build_ssh_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Port
        args.push("-p".to_string());
        args.push(self.port.to_string());

        // Agent forwarding for key/agent auth
        if self.auth_type == "agent" || self.auth_type == "key" {
            args.push("-A".to_string());
        }

        // Identity file for key auth
        if self.auth_type == "key" {
            if let Some(ref identity) = self.identity_file {
                args.push("-i".to_string());
                args.push(identity.clone());
            }
        }

        // Host specification
        args.push(format!("{}@{}", self.username, self.host));

        args
    }

    /// Build full SSH command string
    pub fn to_command_string(&self) -> String {
        let args = self.build_ssh_args();
        format!("ssh {}", args.join(" "))
    }
}

/// Check if a command is available in PATH
fn is_command_available(cmd: &str) -> bool {
    Command::new("where")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if running inside Windows Terminal
fn is_in_windows_terminal() -> bool {
    std::env::var("WT_SESSION").is_ok()
}

/// Launch SSH connection in preferred terminal
pub fn launch_ssh_terminal(
    connection: &SshConnection,
    preference: TerminalPreference,
) -> Result<(), TerminalError> {
    let preference = if preference == TerminalPreference::Auto {
        detect_best_terminal()
    } else {
        preference
    };

    match preference {
        TerminalPreference::WindowsTerminal => {
            launch_windows_terminal(connection)
        }
        TerminalPreference::PowerShell => {
            launch_powershell(connection)
        }
        TerminalPreference::Cmd => {
            launch_cmd(connection)
        }
        TerminalPreference::Auto => unreachable!(),
    }
}

/// Detect the best available terminal
fn detect_best_terminal() -> TerminalPreference {
    if is_command_available("wt") {
        TerminalPreference::WindowsTerminal
    } else if is_command_available("powershell") {
        TerminalPreference::PowerShell
    } else {
        TerminalPreference::Cmd
    }
}

/// Launch in Windows Terminal
fn launch_windows_terminal(connection: &SshConnection) -> Result<(), TerminalError> {
    let ssh_cmd = connection.to_command_string();
    let title = format!("SSH: {}", connection.host);

    // Check if already in Windows Terminal
    if is_in_windows_terminal() {
        // Open in new tab
        let mut cmd = Command::new("wt");
        cmd.arg("new-tab")
            .arg("--title")
            .arg(&title)
            .arg("powershell.exe")
            .arg("-NoExit")
            .arg("-Command")
            .arg(&ssh_cmd);

        cmd.spawn()
            .map_err(|e| TerminalError::LaunchFailed(format!("Windows Terminal: {}", e)))?;
    } else {
        // Open new Windows Terminal window
        let mut cmd = Command::new("wt");
        cmd.arg("--title")
            .arg(&title)
            .arg("powershell.exe")
            .arg("-NoExit")
            .arg("-Command")
            .arg(&ssh_cmd);

        cmd.spawn()
            .map_err(|e| TerminalError::LaunchFailed(format!("Windows Terminal: {}", e)))?;
    }

    Ok(())
}

/// Launch in PowerShell
fn launch_powershell(connection: &SshConnection) -> Result<(), TerminalError> {
    let ssh_cmd = connection.to_command_string();

    let mut cmd = Command::new("powershell.exe");
    cmd.arg("-NoExit")
        .arg("-Command")
        .arg(&ssh_cmd);

    cmd.spawn()
        .map_err(|e| TerminalError::LaunchFailed(format!("PowerShell: {}", e)))?;

    Ok(())
}

/// Launch in CMD
fn launch_cmd(connection: &SshConnection) -> Result<(), TerminalError> {
    let ssh_cmd = connection.to_command_string();

    let mut cmd = Command::new("cmd");
    cmd.arg("/K").arg(&ssh_cmd);

    cmd.spawn()
        .map_err(|e| TerminalError::LaunchFailed(format!("CMD: {}", e)))?;

    Ok(())
}

/// Terminal errors
#[derive(Debug, Clone)]
pub enum TerminalError {
    LaunchFailed(String),
    NoTerminalAvailable,
    InvalidConnection,
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminalError::LaunchFailed(msg) => write!(f, "Failed to launch terminal: {}", msg),
            TerminalError::NoTerminalAvailable => write!(f, "No suitable terminal available"),
            TerminalError::InvalidConnection => write!(f, "Invalid SSH connection parameters"),
        }
    }
}

impl std::error::Error for TerminalError {}

/// Get terminal preference from settings or use auto-detect
pub fn get_terminal_preference() -> TerminalPreference {
    // In a real implementation, this would read from settings
    // For now, return Auto
    TerminalPreference::Auto
}

/// Test if SSH is available
pub fn is_ssh_available() -> bool {
    is_command_available("ssh")
}

/// Get diagnostic information about terminal availability
pub fn get_terminal_diagnostics() -> TerminalDiagnostics {
    TerminalDiagnostics {
        windows_terminal_available: is_command_available("wt"),
        powershell_available: is_command_available("powershell"),
        cmd_available: is_command_available("cmd"),
        ssh_available: is_ssh_available(),
        in_windows_terminal: is_in_windows_terminal(),
    }
}

/// Terminal diagnostics information
#[derive(Debug, Clone)]
pub struct TerminalDiagnostics {
    pub windows_terminal_available: bool,
    pub powershell_available: bool,
    pub cmd_available: bool,
    pub ssh_available: bool,
    pub in_windows_terminal: bool,
}

impl TerminalDiagnostics {
    pub fn any_terminal_available(&self) -> bool {
        self.windows_terminal_available
            || self.powershell_available
            || self.cmd_available
    }

    pub fn get_best_terminal(&self) -> TerminalPreference {
        if self.windows_terminal_available {
            TerminalPreference::WindowsTerminal
        } else if self.powershell_available {
            TerminalPreference::PowerShell
        } else if self.cmd_available {
            TerminalPreference::Cmd
        } else {
            TerminalPreference::Auto
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_args_building() {
        let conn = SshConnection::new(
            "example.com".to_string(),
            2222,
            "user".to_string(),
            "key".to_string(),
            Some("/path/to/key".to_string()),
        );

        let args = conn.build_ssh_args();
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"2222".to_string()));
        assert!(args.contains(&"-A".to_string()));
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"/path/to/key".to_string()));
        assert!(args.contains(&"user@example.com".to_string()));
    }

    #[test]
    fn test_ssh_args_agent_auth() {
        let conn = SshConnection::new(
            "example.com".to_string(),
            22,
            "user".to_string(),
            "agent".to_string(),
            None,
        );

        let args = conn.build_ssh_args();
        assert!(args.contains(&"-A".to_string()));
        assert!(!args.contains(&"-i".to_string()));
    }

    #[test]
    fn test_ssh_args_password_auth() {
        let conn = SshConnection::new(
            "example.com".to_string(),
            22,
            "user".to_string(),
            "password".to_string(),
            None,
        );

        let args = conn.build_ssh_args();
        assert!(!args.contains(&"-A".to_string()));
    }

    #[test]
    fn test_terminal_preference_names() {
        assert_eq!(TerminalPreference::WindowsTerminal.name(), "Windows Terminal");
        assert_eq!(TerminalPreference::PowerShell.name(), "PowerShell");
        assert_eq!(TerminalPreference::Cmd.name(), "Command Prompt");
        assert_eq!(TerminalPreference::Auto.name(), "Auto-detect");
    }
}
