use gtk4::prelude::*;
use std::process::{Command, Stdio};

use crate::models::{AuthType, Server};

/// Terminal information structure
#[derive(Debug, Clone)]
pub struct TerminalInfo {
    pub name: &'static str,
    pub command: &'static str,
    pub args_builder: fn(&str) -> Vec<String>,
    pub priority: i32,
    pub desktop_env: Vec<&'static str>, // Preferred desktop environments
}

/// Priority order for terminal detection (higher = preferred)
fn get_terminal_list() -> Vec<TerminalInfo> {
    vec![
        // GNOME default terminal
        TerminalInfo {
            name: "GNOME Terminal",
            command: "gnome-terminal",
            args_builder: |cmd| {
                vec![
                    "--".to_string(),
                    "bash".to_string(),
                    "-c".to_string(),
                    cmd.to_string(),
                ]
            },
            priority: 100,
            desktop_env: vec!["GNOME", "ubuntu:GNOME"],
        },
        // GNOME new terminal
        TerminalInfo {
            name: "GNOME Console",
            command: "kgx",
            args_builder: |cmd| {
                vec![
                    "--".to_string(),
                    "bash".to_string(),
                    "-c".to_string(),
                    cmd.to_string(),
                ]
            },
            priority: 95,
            desktop_env: vec!["GNOME", "ubuntu:GNOME"],
        },
        // KDE default
        TerminalInfo {
            name: "Konsole",
            command: "konsole",
            args_builder: |cmd| {
                vec![
                    "-e".to_string(),
                    "bash".to_string(),
                    "-c".to_string(),
                    cmd.to_string(),
                ]
            },
            priority: 90,
            desktop_env: vec!["KDE", "plasma"],
        },
        // KDE new terminal
        TerminalInfo {
            name: "WezTerm",
            command: "wezterm",
            args_builder: |cmd| {
                vec![
                    "start".to_string(),
                    "--".to_string(),
                    "bash".to_string(),
                    "-c".to_string(),
                    cmd.to_string(),
                ]
            },
            priority: 85,
            desktop_env: vec![],
        },
        // Modern terminals
        TerminalInfo {
            name: "Alacritty",
            command: "alacritty",
            args_builder: |cmd| {
                vec![
                    "-e".to_string(),
                    "bash".to_string(),
                    "-c".to_string(),
                    cmd.to_string(),
                ]
            },
            priority: 80,
            desktop_env: vec![],
        },
        TerminalInfo {
            name: "Kitty",
            command: "kitty",
            args_builder: |cmd| vec!["bash".to_string(), "-c".to_string(), cmd.to_string()],
            priority: 80,
            desktop_env: vec![],
        },
        TerminalInfo {
            name: "Foot",
            command: "footclient",
            args_builder: |cmd| vec!["bash".to_string(), "-c".to_string(), cmd.to_string()],
            priority: 75,
            desktop_env: vec!["sway", "wayfire", "river"],
        },
        // XFCE
        TerminalInfo {
            name: "XFCE Terminal",
            command: "xfce4-terminal",
            args_builder: |cmd| vec!["-e".to_string(), cmd.to_string()],
            priority: 70,
            desktop_env: vec!["XFCE"],
        },
        // MATE
        TerminalInfo {
            name: "MATE Terminal",
            command: "mate-terminal",
            args_builder: |cmd| vec!["-e".to_string(), cmd.to_string()],
            priority: 70,
            desktop_env: vec!["MATE"],
        },
        // Cinnamon
        TerminalInfo {
            name: "GNOME Terminal (Cinnamon)",
            command: "gnome-terminal",
            args_builder: |cmd| {
                vec![
                    "--".to_string(),
                    "bash".to_string(),
                    "-c".to_string(),
                    cmd.to_string(),
                ]
            },
            priority: 70,
            desktop_env: vec!["X-Cinnamon", "Cinnamon"],
        },
        // LXQt
        TerminalInfo {
            name: "QTerminal",
            command: "qterminal",
            args_builder: |cmd| vec!["-e".to_string(), cmd.to_string()],
            priority: 65,
            desktop_env: vec!["LXQt"],
        },
        // Deepin
        TerminalInfo {
            name: "Deepin Terminal",
            command: "deepin-terminal",
            args_builder: |cmd| vec!["-e".to_string(), cmd.to_string()],
            priority: 65,
            desktop_env: vec!["Deepin"],
        },
        // Tilix (formerly Terminix)
        TerminalInfo {
            name: "Tilix",
            command: "tilix",
            args_builder: |cmd| vec!["-e".to_string(), cmd.to_string()],
            priority: 60,
            desktop_env: vec!["GNOME", "ubuntu:GNOME"],
        },
        // Terminator
        TerminalInfo {
            name: "Terminator",
            command: "terminator",
            args_builder: |cmd| vec!["-e".to_string(), cmd.to_string()],
            priority: 60,
            desktop_env: vec![],
        },
        // LXTerminal (LXDE)
        TerminalInfo {
            name: "LXTerminal",
            command: "lxterminal",
            args_builder: |cmd| vec!["-e".to_string(), cmd.to_string()],
            priority: 55,
            desktop_env: vec!["LXDE"],
        },
        // Guake (drop-down terminal)
        TerminalInfo {
            name: "Guake",
            command: "guake",
            args_builder: |cmd| vec!["--execute-command".to_string(), cmd.to_string()],
            priority: 50,
            desktop_env: vec![],
        },
        // Yakuake (KDE drop-down)
        TerminalInfo {
            name: "Yakuake",
            command: "yakuake",
            args_builder: |cmd| vec!["--execute".to_string(), cmd.to_string()],
            priority: 50,
            desktop_env: vec!["KDE", "plasma"],
        },
        // Tabby
        TerminalInfo {
            name: "Tabby",
            command: "tabby",
            args_builder: |cmd| vec!["run".to_string(), cmd.to_string()],
            priority: 45,
            desktop_env: vec![],
        },
        // Warp (modern terminal)
        TerminalInfo {
            name: "Warp",
            command: "warp",
            args_builder: |cmd| vec!["bash".to_string(), "-c".to_string(), cmd.to_string()],
            priority: 40,
            desktop_env: vec![],
        },
        // Rio
        TerminalInfo {
            name: "Rio",
            command: "rio",
            args_builder: |cmd| vec!["-e".to_string(), cmd.to_string()],
            priority: 40,
            desktop_env: vec![],
        },
        // Fallbacks
        TerminalInfo {
            name: "st",
            command: "st",
            args_builder: |cmd| {
                vec![
                    "-e".to_string(),
                    "bash".to_string(),
                    "-c".to_string(),
                    cmd.to_string(),
                ]
            },
            priority: 20,
            desktop_env: vec![],
        },
        TerminalInfo {
            name: "xterm",
            command: "xterm",
            args_builder: |cmd| vec!["-e".to_string(), cmd.to_string()],
            priority: 10,
            desktop_env: vec![],
        },
        TerminalInfo {
            name: "rxvt-unicode",
            command: "urxvt",
            args_builder: |cmd| vec!["-e".to_string(), cmd.to_string()],
            priority: 10,
            desktop_env: vec![],
        },
    ]
}

/// Launch native terminal with SSH connection
pub fn launch_terminal(server: &Server) {
    tracing::info!(
        "Launching terminal for {}@{}:{}",
        server.username,
        server.host,
        server.port
    );

    // Build SSH command
    let ssh_cmd = build_ssh_command(server);
    tracing::debug!("SSH command: {}", ssh_cmd);

    // Try terminals in order of preference
    let mut terminals = get_terminal_list();

    // Sort by priority (highest first)
    terminals.sort_by(|a, b| b.priority.cmp(&a.priority));

    // Boost desktop environment specific terminals
    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        let desktop = desktop.to_uppercase();
        for terminal in &mut terminals {
            for env in &terminal.desktop_env {
                if desktop.contains(&env.to_uppercase()) {
                    terminal.priority += 50; // Boost matching DE terminals
                    tracing::debug!("Boosted priority for {} (DE match)", terminal.name);
                    break;
                }
            }
        }
        // Re-sort after boosting
        terminals.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    // Try each terminal
    for terminal in &terminals {
        if is_terminal_available(terminal.command) {
            let args = (terminal.args_builder)(&ssh_cmd);

            tracing::debug!("Trying {} with args: {:?}", terminal.name, args);

            match spawn_terminal(terminal.command, &args) {
                Ok(_) => {
                    tracing::info!("Launched {} for SSH connection", terminal.name);
                    return;
                }
                Err(e) => {
                    tracing::warn!("Failed to launch {}: {}", terminal.name, e);
                }
            }
        } else {
            tracing::debug!("{} not available", terminal.name);
        }
    }

    // If no terminal found, show error dialog
    show_error_dialog(
        "No Terminal Found",
        "Could not find a supported terminal emulator.\n\nPlease install one of the following:\n\nModern:\n• GNOME Terminal (gnome-terminal)\n• GNOME Console (kgx)\n• Konsole (KDE)\n• Alacritty\n• Kitty\n• Foot\n• WezTerm\n\nOthers:\n• XFCE Terminal\n• MATE Terminal\n• Tilix\n• Terminator\n• xterm"
    );
}

fn build_ssh_command(server: &Server) -> String {
    let mut parts: Vec<String> = vec!["ssh".to_string()];

    // Add verbose flag for debugging (optional)
    // parts.push("-v".to_string());

    // Add port if not default
    if server.port != 22 {
        parts.push(format!("-p {}", server.port));
    }

    // Add identity file for key auth
    if server.auth_type == AuthType::Key {
        let key_path = server.identity_file.as_deref().unwrap_or("~/.ssh/id_rsa");
        let expanded_path = expand_home(key_path);
        parts.push(format!("-i {}", expanded_path));
    }

    // Add agent forwarding
    if server.auth_type == AuthType::Agent {
        parts.push("-A".to_string());
    }

    // Disable strict host key checking for better UX (with warning)
    // parts.push("-o".to_string());
    // parts.push(""StrictHostKeyChecking=no\".to_string());

    // Add user@host
    parts.push(format!("{}@{}", server.username, server.host));

    parts.join(" ")
}

fn is_terminal_available(terminal: &str) -> bool {
    // First try 'which'
    let which_result = Command::new("which")
        .arg(terminal)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if which_result {
        return true;
    }

    // Fallback: check if executable exists in PATH
    if let Ok(paths) = std::env::var("PATH") {
        for path in paths.split(':') {
            let full_path = std::path::Path::new(path).join(terminal);
            if full_path.exists() {
                return true;
            }
        }
    }

    false
}

fn spawn_terminal(terminal: &str, args: &[String]) -> Result<(), String> {
    let mut cmd = Command::new(terminal);
    cmd.args(args);

    // Detach from parent process
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    // Set working directory to home
    if let Ok(home) = std::env::var("HOME") {
        cmd.current_dir(&home);
    }

    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to spawn {}: {}", terminal, e))
}

fn expand_home(path: &str) -> String {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}{}", home, &path[1..]);
        }
    }
    path.to_string()
}

fn show_error_dialog(title: &str, message: &str) {
    gtk4::glib::spawn_future_local({
        let title = title.to_string();
        let message = message.to_string();

        async move {
            let dialog = adw::MessageDialog::builder()
                .heading(&title)
                .body(&message)
                .build();

            dialog.add_response("install", "Install Terminal");
            dialog.add_response("ok", "OK");
            dialog.set_response_appearance("install", adw::ResponseAppearance::Suggested);
            dialog.set_default_response(Some("ok"));

            dialog.connect_response(None, move |_, response| {
                if response == "install" {
                    // Try to open terminal installation guide or software center
                    let _ = open::that("https://github.com/alacritty/alacritty");
                }
            });

            dialog.present();
        }
    });
}

/// Launch terminal with specific working directory (for SFTP)
pub fn launch_terminal_in_dir(server: &Server, remote_path: &str) {
    tracing::info!(
        "Launching terminal for {}@{} in directory {}",
        server.username,
        server.host,
        remote_path
    );

    // Build SSH command with starting directory
    let ssh_cmd = if remote_path == "~" || remote_path == "/home/" {
        build_ssh_command(server)
    } else {
        format!(
            "{} 'cd {} && exec $SHELL'",
            build_ssh_command(server),
            remote_path
        )
    };

    let terminals = get_terminal_list();

    for terminal in &terminals {
        if is_terminal_available(terminal.command) {
            let args = (terminal.args_builder)(&ssh_cmd);

            if let Ok(_) = spawn_terminal(terminal.command, &args) {
                tracing::info!("Launched {} in remote directory", terminal.name);
                return;
            }
        }
    }

    // Fallback to regular terminal launch
    launch_terminal(server);
}

/// Check if a server is reachable (ping test)
pub fn check_server_reachable(host: &str) -> bool {
    // Try ping first
    let ping_result = Command::new("ping")
        .args(["-c", "1", "-W", "2", host])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if ping_result {
        return true;
    }

    // If ping fails (maybe blocked by firewall), try nc (netcat)
    let nc_result = Command::new("nc")
        .args(["-z", "-w", "2", host, "22"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if nc_result {
        return true;
    }

    // Try timeout with bash built-in
    Command::new("timeout")
        .args(["2", "bash", "-c", &format!("echo > /dev/tcp/{}/22", host)])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get list of available terminals on the system
pub fn get_available_terminals() -> Vec<String> {
    let all_terminals = get_terminal_list();
    all_terminals
        .into_iter()
        .filter(|t| is_terminal_available(t.command))
        .map(|t| t.name.to_string())
        .collect()
}

/// Detect the default terminal for the current desktop environment
pub fn detect_default_terminal() -> Option<String> {
    // Check desktop environment
    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        let desktop_upper = desktop.to_uppercase();

        // GNOME
        if desktop_upper.contains("GNOME") {
            if is_terminal_available("gnome-terminal") {
                return Some("gnome-terminal".to_string());
            }
            if is_terminal_available("kgx") {
                return Some("kgx".to_string());
            }
        }

        // KDE
        if desktop_upper.contains("KDE") {
            if is_terminal_available("konsole") {
                return Some("konsole".to_string());
            }
        }

        // XFCE
        if desktop_upper.contains("XFCE") {
            if is_terminal_available("xfce4-terminal") {
                return Some("xfce4-terminal".to_string());
            }
        }

        // MATE
        if desktop_upper.contains("MATE") {
            if is_terminal_available("mate-terminal") {
                return Some("mate-terminal".to_string());
            }
        }

        // LXQt
        if desktop_upper.contains("LXQT") {
            if is_terminal_available("qterminal") {
                return Some("qterminal".to_string());
            }
        }
    }

    // Try to read from x-terminal-emulator (Debian/Ubuntu)
    if is_terminal_available("x-terminal-emulator") {
        return Some("x-terminal-emulator".to_string());
    }

    // Fallback to first available terminal
    let terminals = get_terminal_list();
    for terminal in &terminals {
        if is_terminal_available(terminal.command) {
            return Some(terminal.command.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ssh_command() {
        let server = Server {
            id: "test".to_string(),
            name: "Test".to_string(),
            host: "example.com".to_string(),
            port: 22,
            username: "user".to_string(),
            auth_type: AuthType::Password,
            group_id: None,
            status: crate::models::ServerStatus::Disconnected,
            identity_file: None,
        };

        let cmd = build_ssh_command(&server);
        assert!(cmd.contains("ssh"));
        assert!(cmd.contains("user@example.com"));
        assert!(!cmd.contains("-p")); // No port flag for default port
    }

    #[test]
    fn test_build_ssh_command_with_port() {
        let server = Server {
            id: "test".to_string(),
            name: "Test".to_string(),
            host: "example.com".to_string(),
            port: 2222,
            username: "user".to_string(),
            auth_type: AuthType::Password,
            group_id: None,
            status: crate::models::ServerStatus::Disconnected,
            identity_file: None,
        };

        let cmd = build_ssh_command(&server);
        assert!(cmd.contains("-p 2222"));
    }

    #[test]
    fn test_expand_home() {
        let path = "~/.ssh/id_rsa";
        let expanded = expand_home(path);
        assert!(!expanded.starts_with("~/"));
    }

    #[test]
    fn test_terminal_list_not_empty() {
        let terminals = get_terminal_list();
        assert!(!terminals.is_empty());
    }
}
