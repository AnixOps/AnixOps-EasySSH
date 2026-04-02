//! 嵌入式终端实现
//! 提供完整的PTY终端仿真器，支持本地shell和SSH连接

use portable_pty::{CommandBuilder, NativePtySystem, PtyPair, PtySystem};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::error::LiteError;
use crate::terminal::{
    TerminalOutput, TerminalSession, TerminalSignal, TerminalSize, TerminalStats,
};

/// 终端仿真器抽象接口
#[async_trait::async_trait]
pub trait TerminalEmulator: Send + Sync {
    /// 获取终端ID
    fn id(&self) -> &str;

    /// 获取会话信息
    fn session_info(&self) -> TerminalSession;

    /// 调整终端大小
    async fn resize(&self, size: TerminalSize) -> Result<(), LiteError>;

    /// 发送输入数据
    async fn write(&self, data: &str) -> Result<(), LiteError>;

    /// 发送二进制数据
    async fn write_bytes(&self, data: &[u8]) -> Result<(), LiteError>;

    /// 发送控制信号
    async fn send_signal(&self, signal: TerminalSignal) -> Result<(), LiteError>;

    /// 检查是否存活
    fn is_alive(&self) -> bool;

    /// 获取性能统计
    fn stats(&self) -> TerminalStats;

    /// 关闭终端
    async fn close(&self) -> Result<(), LiteError>;

    /// 获取标题
    fn title(&self) -> String;

    /// 设置标题
    async fn set_title(&self, title: &str);
}

/// 基于PTY的终端实现
pub struct PtyTerminal {
    id: String,
    session: Arc<RwLock<TerminalSession>>,
    pty_pair: Arc<Mutex<PtyPair>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
    alive: Arc<std::sync::atomic::AtomicBool>,
    stats: Arc<RwLock<TerminalStats>>,
    title_tx: mpsc::UnboundedSender<String>,
}

unsafe impl Send for PtyTerminal {}
unsafe impl Sync for PtyTerminal {}

impl PtyTerminal {
    /// 创建新的本地shell终端
    pub fn new_local(
        id: &str,
        size: TerminalSize,
        shell: Option<&str>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<TerminalOutput>), LiteError> {
        let pty_system = NativePtySystem::default();

        let pty_pair = pty_system
            .openpty(size.to_pty_size())
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to open PTY: {}", e)))?;

        let cmd = match shell {
            Some(s) => CommandBuilder::new(s),
            None => Self::default_shell(),
        };

        let mut child = pty_pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to spawn shell: {}", e)))?;

        let reader = pty_pair
            .master
            .try_clone_reader()
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to clone reader: {}", e)))?;

        let writer = pty_pair
            .master
            .take_writer()
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to take writer: {}", e)))?;

        let (output_tx, output_rx) = mpsc::unbounded_channel();
        let (title_tx, mut title_rx) = mpsc::unbounded_channel::<String>();
        let stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_flag_worker = stop_flag.clone();
        let alive = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let alive_worker = alive.clone();

        let session = TerminalSession {
            id: id.to_string(),
            title: "Local Shell".to_string(),
            server_id: None,
            created_at: chrono::Utc::now(),
            size,
        };

        let stats = Arc::new(RwLock::new(TerminalStats::default()));
        let stats_worker = stats.clone();
        let session_arc = Arc::new(RwLock::new(session.clone()));
        let session_worker = session_arc.clone();

        // 启动读取线程
        let master_id = id.to_string();
        std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 8192];
            let mut frame_count = 0u64;
            let mut last_fps_update = Instant::now();

            loop {
                if stop_flag_worker.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                // 检查标题更新
                if let Ok(title) = title_rx.try_recv() {
                    let mut sess = session_worker.blocking_write();
                    sess.title = title.clone();
                }

                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = output_tx.send(TerminalOutput::Closed);
                        alive_worker.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).to_string();

                        // 更新统计
                        {
                            let mut s = stats_worker.blocking_write();
                            s.bytes_received += n as u64;
                            s.frames_rendered += 1;

                            // 每秒更新FPS
                            let now = Instant::now();
                            if now.duration_since(last_fps_update).as_secs() >= 1 {
                                s.avg_fps = frame_count as f32;
                                frame_count = 0;
                                last_fps_update = now;
                            }
                        }

                        if output_tx.send(TerminalOutput::Data(data)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = output_tx.send(TerminalOutput::Error(e.to_string()));
                        alive_worker.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                }
            }

            log::info!("Terminal {} reader thread exited", master_id);
        });

        // 等待子进程
        let id_worker = id.to_string();
        std::thread::spawn(move || {
            let _ = child.wait();
            log::info!("Terminal {} shell process exited", id_worker);
        });

        let terminal = Self {
            id: id.to_string(),
            session: session_arc,
            pty_pair: Arc::new(Mutex::new(pty_pair)),
            writer: Arc::new(Mutex::new(writer)),
            stop_flag,
            alive,
            stats,
            title_tx,
        };

        Ok((terminal, output_rx))
    }

    /// 创建SSH连接的终端
    pub fn new_ssh(
        id: &str,
        size: TerminalSize,
        host: &str,
        port: u16,
        username: &str,
        auth_method: SshAuthMethod,
    ) -> Result<(Self, mpsc::UnboundedReceiver<TerminalOutput>), LiteError> {
        let pty_system = NativePtySystem::default();

        let pty_pair = pty_system
            .openpty(size.to_pty_size())
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to open PTY: {}", e)))?;

        // 构建SSH命令
        let ssh_cmd = match &auth_method {
            SshAuthMethod::Password(pwd) => {
                format!(
                    "sshpass -p '{}' ssh -p {} {}@{}",
                    shell_escape(pwd),
                    port,
                    username,
                    host
                )
            }
            SshAuthMethod::Key(key_path) => {
                format!("ssh -p {} -i {} {}@{}", port, key_path, username, host)
            }
            SshAuthMethod::Agent => {
                format!("ssh -p {} -A {}@{}", port, username, host)
            }
            SshAuthMethod::None => {
                format!("ssh -p {} {}@{}", port, username, host)
            }
        };

        let mut cmd = CommandBuilder::new("sh");
        cmd.arg("-c");
        cmd.arg(&ssh_cmd);

        let mut child = pty_pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to spawn SSH: {}", e)))?;

        let reader = pty_pair
            .master
            .try_clone_reader()
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to clone reader: {}", e)))?;

        let writer = pty_pair
            .master
            .take_writer()
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to take writer: {}", e)))?;

        let (output_tx, output_rx) = mpsc::unbounded_channel();
        let (title_tx, mut title_rx) = mpsc::unbounded_channel::<String>();
        let stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_flag_worker = stop_flag.clone();
        let alive = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let alive_worker = alive.clone();

        let session = TerminalSession {
            id: id.to_string(),
            title: format!("{}@{}", username, host),
            server_id: Some(format!("{}@{}:{}", username, host, port)),
            created_at: chrono::Utc::now(),
            size,
        };

        let stats = Arc::new(RwLock::new(TerminalStats::default()));
        let stats_worker = stats.clone();
        let session_arc = Arc::new(RwLock::new(session.clone()));
        let session_worker = session_arc.clone();

        // 启动读取线程
        let master_id = id.to_string();
        std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 8192];
            let mut frame_count = 0u64;
            let mut last_fps_update = Instant::now();

            loop {
                if stop_flag_worker.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                // 检查标题更新
                if let Ok(title) = title_rx.try_recv() {
                    let mut sess = session_worker.blocking_write();
                    sess.title = title.clone();
                }

                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = output_tx.send(TerminalOutput::Closed);
                        alive_worker.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).to_string();

                        // 更新统计
                        {
                            let mut s = stats_worker.blocking_write();
                            s.bytes_received += n as u64;
                            s.frames_rendered += 1;

                            let now = Instant::now();
                            if now.duration_since(last_fps_update).as_secs() >= 1 {
                                s.avg_fps = frame_count as f32;
                                frame_count = 0;
                                last_fps_update = now;
                            }
                        }

                        if output_tx.send(TerminalOutput::Data(data)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = output_tx.send(TerminalOutput::Error(e.to_string()));
                        alive_worker.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                }
            }

            log::info!("SSH Terminal {} reader thread exited", master_id);
        });

        // 等待子进程
        let id_worker = id.to_string();
        std::thread::spawn(move || {
            let _ = child.wait();
            log::info!("SSH Terminal {} process exited", id_worker);
        });

        let terminal = Self {
            id: id.to_string(),
            session: session_arc,
            pty_pair: Arc::new(Mutex::new(pty_pair)),
            writer: Arc::new(Mutex::new(writer)),
            stop_flag,
            alive,
            stats,
            title_tx,
        };

        Ok((terminal, output_rx))
    }

    fn default_shell() -> CommandBuilder {
        #[cfg(target_os = "windows")]
        {
            CommandBuilder::new("powershell.exe")
        }
        #[cfg(not(target_os = "windows"))]
        {
            CommandBuilder::new(std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()))
        }
    }
}

#[async_trait::async_trait]
impl TerminalEmulator for PtyTerminal {
    fn id(&self) -> &str {
        &self.id
    }

    fn session_info(&self) -> TerminalSession {
        let session = self.session.blocking_read();
        session.clone()
    }

    async fn resize(&self, size: TerminalSize) -> Result<(), LiteError> {
        let pty_pair = self.pty_pair.lock().await;
        pty_pair
            .master
            .resize(size.to_pty_size())
            .map_err(|e| LiteError::TerminalEmulator(format!("Resize failed: {}", e)))?;

        let mut session = self.session.write().await;
        session.size = size;

        Ok(())
    }

    async fn write(&self, data: &str) -> Result<(), LiteError> {
        if !self.is_alive() {
            return Err(LiteError::TerminalEmulator(
                "Terminal is closed".to_string(),
            ));
        }

        let mut writer = self.writer.lock().await;
        writer
            .write_all(data.as_bytes())
            .map_err(|e| LiteError::TerminalEmulator(format!("Write failed: {}", e)))?;
        writer
            .flush()
            .map_err(|e| LiteError::TerminalEmulator(format!("Flush failed: {}", e)))?;

        // 更新发送字节数
        {
            let mut stats = self.stats.write().await;
            stats.bytes_sent += data.len() as u64;
        }

        Ok(())
    }

    async fn write_bytes(&self, data: &[u8]) -> Result<(), LiteError> {
        if !self.is_alive() {
            return Err(LiteError::TerminalEmulator(
                "Terminal is closed".to_string(),
            ));
        }

        let mut writer = self.writer.lock().await;
        writer
            .write_all(data)
            .map_err(|e| LiteError::TerminalEmulator(format!("Write failed: {}", e)))?;
        writer
            .flush()
            .map_err(|e| LiteError::TerminalEmulator(format!("Flush failed: {}", e)))?;

        {
            let mut stats = self.stats.write().await;
            stats.bytes_sent += data.len() as u64;
        }

        Ok(())
    }

    async fn send_signal(&self, signal: TerminalSignal) -> Result<(), LiteError> {
        let data = match signal {
            TerminalSignal::Interrupt => b"\x03", // ETX
            TerminalSignal::Eof => b"\x04",       // EOT
            TerminalSignal::Suspend => b"\x1a",   // SUB
            TerminalSignal::Quit => b"\x1c",      // FS
        };
        self.write_bytes(data).await
    }

    fn is_alive(&self) -> bool {
        self.alive.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn stats(&self) -> TerminalStats {
        self.stats.blocking_read().clone()
    }

    async fn close(&self) -> Result<(), LiteError> {
        self.stop_flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.alive
            .store(false, std::sync::atomic::Ordering::Relaxed);

        // 发送EOF信号给PTY
        let _ = self.send_signal(TerminalSignal::Eof).await;

        Ok(())
    }

    fn title(&self) -> String {
        let session = self.session.blocking_read();
        session.title.clone()
    }

    async fn set_title(&self, title: &str) {
        let _ = self.title_tx.send(title.to_string());
    }
}

/// SSH认证方法
#[derive(Debug, Clone)]
pub enum SshAuthMethod {
    None,
    Password(String),
    Key(String),
    Agent,
}

fn shell_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\'', "\\'")
        .replace('$', "\\$")
}

/// 终端管理器
pub struct TerminalManager {
    terminals: Arc<RwLock<HashMap<String, Arc<dyn TerminalEmulator>>>>,
    output_handlers: Arc<RwLock<HashMap<String, mpsc::UnboundedReceiver<TerminalOutput>>>>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            terminals: Arc::new(RwLock::new(HashMap::new())),
            output_handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建本地终端
    pub async fn create_local(
        &self,
        id: &str,
        size: TerminalSize,
        shell: Option<&str>,
    ) -> Result<mpsc::UnboundedReceiver<TerminalOutput>, LiteError> {
        let (terminal, receiver) = PtyTerminal::new_local(id, size, shell)?;
        let mut terminals = self.terminals.write().await;
        terminals.insert(id.to_string(), Arc::new(terminal));
        Ok(receiver)
    }

    /// 创建SSH终端
    pub async fn create_ssh(
        &self,
        id: &str,
        size: TerminalSize,
        host: &str,
        port: u16,
        username: &str,
        auth_method: SshAuthMethod,
    ) -> Result<mpsc::UnboundedReceiver<TerminalOutput>, LiteError> {
        let (terminal, receiver) =
            PtyTerminal::new_ssh(id, size, host, port, username, auth_method)?;
        let mut terminals = self.terminals.write().await;
        terminals.insert(id.to_string(), Arc::new(terminal));
        Ok(receiver)
    }

    /// 获取终端
    pub async fn get(&self, id: &str) -> Option<Arc<dyn TerminalEmulator>> {
        let terminals = self.terminals.read().await;
        terminals.get(id).cloned()
    }

    /// 写入终端
    pub async fn write(&self, id: &str, data: &str) -> Result<(), LiteError> {
        let terminal = self
            .get(id)
            .await
            .ok_or_else(|| LiteError::TerminalEmulator(format!("Terminal {} not found", id)))?;
        terminal.write(data).await
    }

    /// 写入二进制数据
    pub async fn write_bytes(&self, id: &str, data: &[u8]) -> Result<(), LiteError> {
        let terminal = self
            .get(id)
            .await
            .ok_or_else(|| LiteError::TerminalEmulator(format!("Terminal {} not found", id)))?;
        terminal.write_bytes(data).await
    }

    /// 调整大小
    pub async fn resize(&self, id: &str, size: TerminalSize) -> Result<(), LiteError> {
        let terminal = self
            .get(id)
            .await
            .ok_or_else(|| LiteError::TerminalEmulator(format!("Terminal {} not found", id)))?;
        terminal.resize(size).await
    }

    /// 关闭终端
    pub async fn close(&self, id: &str) -> Result<(), LiteError> {
        let terminal = self
            .get(id)
            .await
            .ok_or_else(|| LiteError::TerminalEmulator(format!("Terminal {} not found", id)))?;
        terminal.close().await?;

        let mut terminals = self.terminals.write().await;
        terminals.remove(id);

        Ok(())
    }

    /// 列出所有终端ID
    pub async fn list(&self) -> Vec<String> {
        let terminals = self.terminals.read().await;
        terminals.keys().cloned().collect()
    }

    /// 获取所有终端会话信息
    pub async fn list_sessions(&self) -> Vec<TerminalSession> {
        let terminals = self.terminals.read().await;
        let mut sessions = Vec::new();
        for (_, terminal) in terminals.iter() {
            sessions.push(terminal.session_info());
        }
        sessions
    }

    /// 发送信号
    pub async fn send_signal(&self, id: &str, signal: TerminalSignal) -> Result<(), LiteError> {
        let terminal = self
            .get(id)
            .await
            .ok_or_else(|| LiteError::TerminalEmulator(format!("Terminal {} not found", id)))?;
        terminal.send_signal(signal).await
    }

    /// 设置终端标题
    pub async fn set_title(&self, id: &str, title: &str) -> Result<(), LiteError> {
        let terminal = self
            .get(id)
            .await
            .ok_or_else(|| LiteError::TerminalEmulator(format!("Terminal {} not found", id)))?;
        terminal.set_title(title).await;
        Ok(())
    }

    /// 获取终端统计
    pub async fn get_stats(&self, id: &str) -> Result<TerminalStats, LiteError> {
        let terminal = self
            .get(id)
            .await
            .ok_or_else(|| LiteError::TerminalEmulator(format!("Terminal {} not found", id)))?;
        Ok(terminal.stats())
    }

    /// 检查终端是否存活
    pub async fn is_alive(&self, id: &str) -> bool {
        if let Some(terminal) = self.get(id).await {
            terminal.is_alive()
        } else {
            false
        }
    }

    /// 批量关闭所有终端
    pub async fn close_all(&self) {
        let ids = self.list().await;
        for id in ids {
            let _ = self.close(&id).await;
        }
    }

    /// 获取活跃终端数量
    pub async fn active_count(&self) -> usize {
        let terminals = self.terminals.read().await;
        terminals.values().filter(|t| t.is_alive()).count()
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 终端事件处理器
pub struct TerminalEventHandler {
    terminal_id: String,
    output_rx: mpsc::UnboundedReceiver<TerminalOutput>,
    on_data: Option<Box<dyn Fn(String) + Send>>,
    on_close: Option<Box<dyn Fn() + Send>>,
    on_error: Option<Box<dyn Fn(String) + Send>>,
}

impl TerminalEventHandler {
    pub fn new(terminal_id: &str, output_rx: mpsc::UnboundedReceiver<TerminalOutput>) -> Self {
        Self {
            terminal_id: terminal_id.to_string(),
            output_rx,
            on_data: None,
            on_close: None,
            on_error: None,
        }
    }

    pub fn on_data<F>(mut self, f: F) -> Self
    where
        F: Fn(String) + Send + 'static,
    {
        self.on_data = Some(Box::new(f));
        self
    }

    pub fn on_close<F>(mut self, f: F) -> Self
    where
        F: Fn() + Send + 'static,
    {
        self.on_close = Some(Box::new(f));
        self
    }

    pub fn on_error<F>(mut self, f: F) -> Self
    where
        F: Fn(String) + Send + 'static,
    {
        self.on_error = Some(Box::new(f));
        self
    }

    /// 启动事件处理循环
    pub async fn run(mut self) {
        while let Some(output) = self.output_rx.recv().await {
            match output {
                TerminalOutput::Data(data) => {
                    if let Some(ref callback) = self.on_data {
                        callback(data);
                    }
                }
                TerminalOutput::Closed => {
                    if let Some(ref callback) = self.on_close {
                        callback();
                    }
                    break;
                }
                TerminalOutput::Error(e) => {
                    if let Some(ref callback) = self.on_error {
                        callback(e);
                    }
                }
                TerminalOutput::Title(_) => {}
            }
        }

        log::info!("Terminal {} event handler exited", self.terminal_id);
    }
}
