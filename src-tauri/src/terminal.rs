use std::process::Command;

use crate::error::LiteError;

#[cfg(target_os = "windows")]
pub fn open_native_terminal(
    host: &str,
    port: u16,
    username: &str,
    auth_type: &str,
) -> Result<(), LiteError> {
    let ssh_cmd = build_ssh_command(host, port, username, auth_type);

    // 优先使用Windows Terminal
    let terminals = [
        ("wt", vec!["--title", "EasySSH", "-e"]),
        ("WindowsTerminal.exe", vec!["--title", "EasySSH", "-e"]),
        ("powershell.exe", vec!["-NoExit", "-Command"]),
        ("cmd.exe", vec![]),
    ];

    for (terminal, args) in terminals {
        if is_command_available(terminal) {
            let mut cmd = Command::new(terminal);
            for arg in args {
                cmd.arg(arg);
            }
            if terminal == "wt" || terminal == "WindowsTerminal.exe" {
                cmd.arg("cmd");
                cmd.arg("/c");
                cmd.arg(&ssh_cmd);
            } else if terminal == "powershell.exe" {
                cmd.arg(format!("ssh {}", ssh_cmd));
            } else {
                cmd.arg("/c");
                cmd.arg(&ssh_cmd);
            }

            cmd.spawn()
                .map_err(|e| LiteError::Terminal(e.to_string()))?;
            return Ok(());
        }
    }

    Err(LiteError::Terminal("未找到可用的终端程序".to_string()))
}

#[cfg(target_os = "macos")]
pub fn open_native_terminal(
    host: &str,
    port: u16,
    username: &str,
    auth_type: &str,
) -> Result<(), LiteError> {
    let ssh_cmd = build_ssh_command(host, port, username, auth_type);

    // 优先使用iTerm2
    if is_command_available("iterm") || is_command_available("iTerm") {
        let script = format!(
            r#"tell application "iTerm"
                activate
                create window with default profile
                tell current session of current window
                    write text "ssh {}"
                end tell
            end tell"#,
            ssh_cmd
        );
        Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .spawn()
            .map_err(|e| LiteError::Terminal(e.to_string()))?;
        return Ok(());
    }

    // 回退到Terminal.app
    if is_command_available("terminal") || is_command_available("Terminal") {
        Command::new("osascript")
            .arg("-e")
            .arg(&format!(
                r#"tell application "Terminal"
                    activate
                    do script "ssh {}"
                end tell"#,
                ssh_cmd
            ))
            .spawn()
            .map_err(|e| LiteError::Terminal(e.to_string()))?;
        return Ok(());
    }

    Err(LiteError::Terminal("未找到可用的终端程序".to_string()))
}

#[cfg(target_os = "linux")]
pub fn open_native_terminal(
    host: &str,
    port: u16,
    username: &str,
    auth_type: &str,
) -> Result<(), LiteError> {
    let ssh_cmd = build_ssh_command(host, port, username, auth_type);

    // 按优先级尝试不同终端
    let terminals = [
        ("gnome-terminal", vec!["--"]),
        ("konsole", vec!["-e"]),
        ("xfce4-terminal", vec!["-e"]),
        ("xterm", vec!["-e"]),
    ];

    for (terminal, args) in terminals {
        if is_command_available(terminal) {
            let mut cmd = Command::new(terminal);
            for arg in args {
                cmd.arg(arg);
            }
            cmd.arg("bash")
                .arg("-c")
                .arg(format!("ssh {}; read -p '按回车键退出...'", ssh_cmd));

            cmd.spawn()
                .map_err(|e| LiteError::Terminal(e.to_string()))?;
            return Ok(());
        }
    }

    Err(LiteError::Terminal("未找到可用的终端程序".to_string()))
}

pub fn build_ssh_command(host: &str, port: u16, username: &str, auth_type: &str) -> String {
    let mut cmd = format!("{}@{} -p {}", username, host, port);

    match auth_type {
        "key" => {
            // 使用SSH Agent或默认密钥
            cmd.push_str(" -A");
        }
        "agent" => {
            // SSH Agent转发
            cmd.push_str(" -A");
        }
        _ => {
            // 密码认证 - SSH会提示输入
        }
    }

    cmd
}

fn is_command_available(cmd: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        // On Windows, use `where` instead of `which`
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
