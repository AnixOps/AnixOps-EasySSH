use std::process::Command;

use crate::error::LiteError;

/// SSH连接参数
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

    /// 构建SSH命令参数列表
    pub fn to_args(&self) -> Vec<String> {
        let mut args = vec!["-p".to_string(), self.port.to_string()];
        if self.forward_agent {
            args.push("-A".to_string());
        }
        args.push(format!("{}@{}", self.username, self.host));
        args
    }

    /// 构建单行命令字符串（用于需要字符串的场景）
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
            ssh_cmd.replace('"', "\\\"")
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

    // 按优先级尝试不同终端
    let terminals = [
        ("gnome-terminal", vec!["--", "bash", "-c"]),
        ("konsole", vec!["-e", "bash", "-c"]),
        ("xfce4-terminal", vec!["-e", "bash", "-c"]),
        ("xterm", vec!["-e", "bash", "-c"]),
    ];

    for (terminal, args) in terminals {
        if is_command_available(terminal) {
            let mut cmd = Command::new(terminal);
            for arg in &args {
                cmd.arg(arg);
            }
            cmd.arg(format!("{}; read -p 'Press Enter to exit...'", ssh_cmd));

            cmd.spawn()
                .map_err(|e| LiteError::Terminal(format!("{}: {}", terminal, e)))?;
            return Ok(());
        }
    }

    Err(LiteError::Terminal("未找到可用的终端程序".to_string()))
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
