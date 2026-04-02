use gtk4::prelude::*;

use crate::models::{AuthType, Server};

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

    // Try different terminal emulators in order of preference
    let terminals = vec![
        ("gnome-terminal", vec!["--", "bash", "-c", &ssh_cmd]),
        ("konsole", vec!["-e", "bash", "-c", &ssh_cmd]),
        ("xfce4-terminal", vec!["-e", &ssh_cmd]),
        ("mate-terminal", vec!["-e", &ssh_cmd]),
        ("xterm", vec!["-e", &ssh_cmd]),
        ("alacritty", vec!["-e", "bash", "-c", &ssh_cmd]),
        ("kitty", vec!["bash", "-c", &ssh_cmd]),
        ("wezterm", vec!["start", "--", "bash", "-c", &ssh_cmd]),
    ];

    for (terminal, args) in &terminals {
        if is_terminal_available(terminal) {
            match spawn_terminal(terminal, args) {
                Ok(_) => {
                    tracing::info!("Launched {} for SSH connection", terminal);
                    return;
                }
                Err(e) => {
                    tracing::warn!("Failed to launch {}: {}", terminal, e);
                }
            }
        }
    }

    // If no terminal found, show error dialog
    show_error_dialog(
        "No Terminal Found",
        "Could not find a supported terminal emulator.\n\nPlease install one of the following:\n• gnome-terminal\n• konsole\n• xfce4-terminal\n• mate-terminal\n• xterm\n• alacritty\n• kitty\n• wezterm"
    );
}

fn build_ssh_command(server: &Server) -> String {
    let mut cmd = format!("ssh ");

    // Add port if not default
    if server.port != 22 {
        cmd.push_str(&format!("-p {} ", server.port));
    }

    // Add identity file for key auth
    if server.auth_type == AuthType::Key {
        let key_path = server.identity_file.as_deref().unwrap_or("~/.ssh/id_rsa");
        let expanded_path = expand_home(key_path);
        cmd.push_str(&format!("-i {} ", expanded_path));
    }

    // Add agent forwarding
    if server.auth_type == AuthType::Agent {
        cmd.push_str("-A ");
    }

    // Add user@host
    cmd.push_str(&format!("{}@{}", server.username, server.host));

    cmd
}

fn is_terminal_available(terminal: &str) -> bool {
    std::process::Command::new("which")
        .arg(terminal)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn spawn_terminal(terminal: &str, args: &[&str]) -> Result<(), String> {
    let mut cmd = std::process::Command::new(terminal);
    cmd.args(args);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

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
    // Create a simple GTK dialog for error
    gtk4::glib::spawn_future_local({
        let title = title.to_string();
        let message = message.to_string();

        async move {
            let dialog = adw::MessageDialog::builder()
                .heading(&title)
                .body(&message)
                .build();

            dialog.add_response("ok", "OK");
            dialog.set_default_response(Some("ok"));
            dialog.present();
        }
    });
}

/// Launch terminal with specific working directory (for SFTP)
pub fn launch_terminal_in_dir(server: &Server, _remote_path: &str) {
    // For now, just launch regular SSH
    // Future: could integrate with remote filesystem mounting
    launch_terminal(server);
}

/// Check if a server is reachable (ping test)
pub fn check_server_reachable(host: &str) -> bool {
    std::process::Command::new("ping")
        .args(["-c", "1", "-W", "2", host])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
