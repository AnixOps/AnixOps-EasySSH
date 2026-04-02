//! 嵌入式终端实现
//! 提供完整的PTY终端仿真器，支持本地shell和SSH连接
//!
//! # 架构
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           WebSocket (Frontend)          │
//! │              xterm.js                   │
//! └─────────────────┬───────────────────────┘
//!                   │ WebSocket Protocol
//! ┌─────────────────▼───────────────────────┐
//! │           WebSocketBridge               │
//! │    (Protocol Translation Layer)         │
//! └─────────────────┬───────────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────────┐
//! │           TerminalEmulator                │
//! │    (Escape Sequence Processing)         │
//! │         ┌─────────────┐                 │
//! │         │ XtermCompat │                 │
//! │         └─────────────┘                 │
//! └─────────────────┬───────────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────────┐
//! │            PtyManager                   │
//! │    (PTY Lifecycle & I/O Management)     │
//! └─────────────────┬───────────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────────┐
//! │           portable-pty                  │
//! │      (Native PTY Implementation)        │
//! └─────────────────────────────────────────┘
//! ```

use portable_pty::{CommandBuilder, NativePtySystem, PtyPair, PtySystem};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::tungstenite::Message as WsMessage;

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

    /// 获取关联的WebSocket会话ID
    fn websocket_session(&self) -> Option<String>;

    /// 设置WebSocket会话ID
    async fn set_websocket_session(&self, session_id: &str);
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
    websocket_session: Arc<RwLock<Option<String>>>,
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
            websocket_session: Arc::new(RwLock::new(None)),
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
            websocket_session: Arc::new(RwLock::new(None)),
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

    fn websocket_session(&self) -> Option<String> {
        self.websocket_session.blocking_read().clone()
    }

    async fn set_websocket_session(&self, session_id: &str) {
        let mut session = self.websocket_session.write().await;
        *session = Some(session_id.to_string());
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

// ============================================================================
// WebSocket Bridge - WebSocket与终端之间的协议桥接
// ============================================================================

/// WebSocket消息类型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum TerminalWsMessage {
    /// 输入数据 (从xterm.js到PTY)
    #[serde(rename = "input")]
    Input { data: String },
    /// 二进制输入
    #[serde(rename = "binary")]
    Binary { data: Vec<u8> },
    /// 调整大小
    #[serde(rename = "resize")]
    Resize {
        rows: u16,
        cols: u16,
        width: u16,
        height: u16,
    },
    /// 控制信号
    #[serde(rename = "signal")]
    Signal { signal: String },
    /// 请求终端信息
    #[serde(rename = "info")]
    Info,
    /// 终端就绪
    #[serde(rename = "ready")]
    Ready { client_info: ClientInfo },
    /// 心跳
    #[serde(rename = "ping")]
    Ping,
    /// 心跳响应
    #[serde(rename = "pong")]
    Pong,
    /// 剪贴板操作
    #[serde(rename = "clipboard")]
    Clipboard {
        action: String,
        data: Option<String>,
    },
    /// 鼠标事件
    #[serde(rename = "mouse")]
    Mouse {
        button: u8,
        x: u16,
        y: u16,
        action: String,
    },
    /// 焦点事件
    #[serde(rename = "focus")]
    Focus { focused: bool },
}

/// 客户端信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClientInfo {
    pub user_agent: String,
    pub platform: String,
    pub language: String,
    pub color_depth: u8,
    pub pixel_ratio: f64,
}

/// 终端输出消息 (从PTY到xterm.js)
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "data")]
pub enum TerminalWsOutput {
    /// 输出数据
    #[serde(rename = "output")]
    Output { data: String },
    /// 二进制输出
    #[serde(rename = "binary")]
    Binary { data: Vec<u8> },
    /// 终端信息响应
    #[serde(rename = "info")]
    Info {
        id: String,
        title: String,
        rows: u16,
        cols: u16,
    },
    /// 标题变更
    #[serde(rename = "title")]
    Title { title: String },
    /// 连接已关闭
    #[serde(rename = "closed")]
    Closed { reason: Option<String> },
    /// 错误
    #[serde(rename = "error")]
    Error { message: String },
    /// 心跳响应
    #[serde(rename = "pong")]
    Pong,
    /// 剪贴板数据
    #[serde(rename = "clipboard")]
    Clipboard { data: String },
    /// 响铃
    #[serde(rename = "bell")]
    Bell,
    /// 响铃权限请求
    #[serde(rename = "permission")]
    Permission { permission: String },
}

/// WebSocket桥接器
/// 处理WebSocket与终端之间的双向通信
#[derive(Clone)]
pub struct WebSocketBridge {
    terminal_id: String,
    terminal: Arc<dyn TerminalEmulator>,
    output_tx: mpsc::UnboundedSender<TerminalWsOutput>,
    output_rx: Arc<RwLock<mpsc::UnboundedReceiver<TerminalWsOutput>>>,
    input_tx: mpsc::UnboundedSender<TerminalWsMessage>,
    stats: Arc<RwLock<BridgeStats>>,
    config: BridgeConfig,
}

/// 桥接统计
#[derive(Debug, Clone, Default)]
pub struct BridgeStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub latency_ms: f32,
    pub reconnect_count: u32,
}

/// 桥接配置
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub enable_binary: bool,
    pub enable_mouse: bool,
    pub enable_clipboard: bool,
    pub ping_interval_secs: u64,
    pub max_message_size: usize,
    pub compression_threshold: usize,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            enable_binary: true,
            enable_mouse: true,
            enable_clipboard: true,
            ping_interval_secs: 30,
            max_message_size: 10 * 1024 * 1024, // 10MB
            compression_threshold: 1024,        // 1KB
        }
    }
}

impl WebSocketBridge {
    /// 创建新的WebSocket桥接器
    pub fn new(
        terminal_id: String,
        terminal: Arc<dyn TerminalEmulator>,
        config: Option<BridgeConfig>,
    ) -> (Self, mpsc::UnboundedReceiver<TerminalWsOutput>) {
        let (output_tx, output_rx) = mpsc::unbounded_channel();
        let (input_tx, _input_rx) = mpsc::unbounded_channel();

        let bridge = Self {
            terminal_id,
            terminal,
            output_tx,
            output_rx: Arc::new(RwLock::new(output_rx)),
            input_tx,
            stats: Arc::new(RwLock::new(BridgeStats::default())),
            config: config.unwrap_or_default(),
        };

        // 创建新的接收器给调用者
        let (tx, rx) = mpsc::unbounded_channel();

        // 转发输出
        let output_tx_clone = bridge.output_tx.clone();
        tokio::spawn(async move {
            // 这里可以添加额外的处理逻辑
        });

        (bridge, rx)
    }

    /// 处理传入的WebSocket消息
    pub async fn handle_message(&self, msg: TerminalWsMessage) -> Result<(), LiteError> {
        let start = Instant::now();

        match msg {
            TerminalWsMessage::Input { data } => {
                self.terminal.write(&data).await?;
            }
            TerminalWsMessage::Binary { data } => {
                if self.config.enable_binary {
                    self.terminal.write_bytes(&data).await?;
                }
            }
            TerminalWsMessage::Resize {
                rows,
                cols,
                width,
                height,
            } => {
                let size = TerminalSize::new(rows, cols).with_pixels(width, height);
                self.terminal.resize(size).await?;

                // 发送确认
                let _ = self.output_tx.send(TerminalWsOutput::Info {
                    id: self.terminal_id.clone(),
                    title: self.terminal.title(),
                    rows,
                    cols,
                });
            }
            TerminalWsMessage::Signal { signal } => {
                let term_signal = match signal.as_str() {
                    "interrupt" => TerminalSignal::Interrupt,
                    "eof" => TerminalSignal::Eof,
                    "suspend" => TerminalSignal::Suspend,
                    "quit" => TerminalSignal::Quit,
                    _ => {
                        return Err(LiteError::TerminalEmulator(format!(
                            "Unknown signal: {}",
                            signal
                        )))
                    }
                };
                self.terminal.send_signal(term_signal).await?;
            }
            TerminalWsMessage::Info => {
                let info = self.terminal.session_info();
                let _ = self.output_tx.send(TerminalWsOutput::Info {
                    id: info.id,
                    title: info.title,
                    rows: info.size.rows,
                    cols: info.size.cols,
                });
            }
            TerminalWsMessage::Ping => {
                let _ = self.output_tx.send(TerminalWsOutput::Pong);
            }
            TerminalWsMessage::Clipboard { action, data } => {
                if self.config.enable_clipboard {
                    self.handle_clipboard(&action, data).await?;
                }
            }
            TerminalWsMessage::Mouse {
                button,
                x,
                y,
                action,
            } => {
                if self.config.enable_mouse {
                    self.handle_mouse(button, x, y, &action).await?;
                }
            }
            TerminalWsMessage::Focus { focused } => {
                // 焦点变更事件，可用于暂停/恢复渲染
                log::debug!("Terminal {} focus changed: {}", self.terminal_id, focused);
            }
            _ => {}
        }

        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.messages_received += 1;
            stats.latency_ms = start.elapsed().as_secs_f32() * 1000.0;
        }

        Ok(())
    }

    /// 发送输出到WebSocket
    pub async fn send_output(&self, output: TerminalOutput) -> Result<(), LiteError> {
        let msg = match output {
            TerminalOutput::Data(data) => {
                // 更新统计
                {
                    let mut stats = self.stats.write().await;
                    stats.bytes_sent += data.len() as u64;
                    stats.messages_sent += 1;
                }
                TerminalWsOutput::Output { data }
            }
            TerminalOutput::Title(title) => TerminalWsOutput::Title { title },
            TerminalOutput::Closed => TerminalWsOutput::Closed { reason: None },
            TerminalOutput::Error(e) => TerminalWsOutput::Error { message: e },
        };

        self.output_tx
            .send(msg)
            .map_err(|e| LiteError::TerminalEmulator(format!("Failed to send output: {}", e)))?;

        Ok(())
    }

    /// 处理剪贴板操作
    async fn handle_clipboard(&self, action: &str, data: Option<String>) -> Result<(), LiteError> {
        match action {
            "copy" => {
                // 服务器请求复制到剪贴板
                if let Some(text) = data {
                    let _ = self
                        .output_tx
                        .send(TerminalWsOutput::Clipboard { data: text });
                }
            }
            "paste" => {
                // 客户端请求粘贴
                // 剪贴板内容应由前端处理，这里只是确认支持
            }
            "request" => {
                // 请求剪贴板权限
                let _ = self.output_tx.send(TerminalWsOutput::Permission {
                    permission: "clipboard".to_string(),
                });
            }
            _ => {}
        }
        Ok(())
    }

    /// 处理鼠标事件
    async fn handle_mouse(
        &self,
        button: u8,
        x: u16,
        y: u16,
        action: &str,
    ) -> Result<(), LiteError> {
        // 将鼠标事件转换为SGR格式
        let sgr = match action {
            "down" => 0,
            "up" => 3,
            "move" => 32,
            "wheel" => 64,
            _ => 0,
        };

        let btn = button + sgr;
        let data = format!("\x1b[<{};{};{}M", btn, x + 1, y + 1);
        self.terminal.write_bytes(data.as_bytes()).await?;

        Ok(())
    }

    /// 处理终端输出流
    pub async fn forward_output(&self, mut rx: mpsc::UnboundedReceiver<TerminalOutput>) {
        while let Some(output) = rx.recv().await {
            if let Err(e) = self.send_output(output.clone()).await {
                log::error!("Failed to forward output: {}", e);
                break;
            }

            if matches!(output, TerminalOutput::Closed) {
                break;
            }
        }

        log::info!(
            "WebSocketBridge {} output forwarder exited",
            self.terminal_id
        );
    }

    /// 获取桥接统计
    pub async fn stats(&self) -> BridgeStats {
        self.stats.read().await.clone()
    }

    /// 获取配置
    pub fn config(&self) -> &BridgeConfig {
        &self.config
    }

    /// 更新配置
    pub async fn update_config(&mut self, config: BridgeConfig) {
        self.config = config;
    }

    /// 序列化输出消息为WebSocket消息
    pub fn serialize_output(&self, msg: TerminalWsOutput) -> Result<WsMessage, String> {
        match serde_json::to_string(&msg) {
            Ok(json) => Ok(WsMessage::Text(json)),
            Err(e) => Err(format!("Failed to serialize: {}", e)),
        }
    }

    /// 解析WebSocket消息
    pub fn parse_message(&self, msg: &str) -> Result<TerminalWsMessage, String> {
        serde_json::from_str(msg).map_err(|e| format!("Failed to parse: {}", e))
    }
}

// ============================================================================
// PTY Manager - 伪终端生命周期管理
// ============================================================================

/// PTY实例信息
#[derive(Debug, Clone)]
pub struct PtyInstance {
    pub id: String,
    pub pty_type: PtyType,
    pub status: PtyStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub bytes_read: u64,
    pub bytes_written: u64,
}

/// PTY类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyType {
    LocalShell,
    Ssh,
    Docker,
    Wsl,
    Serial,
}

/// PTY状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyStatus {
    Initializing,
    Running,
    Suspended,
    Terminated,
    Error,
}

impl PtyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PtyStatus::Initializing => "initializing",
            PtyStatus::Running => "running",
            PtyStatus::Suspended => "suspended",
            PtyStatus::Terminated => "terminated",
            PtyStatus::Error => "error",
        }
    }
}

/// PTY管理器
/// 管理所有PTY实例的生命周期和资源
pub struct PtyManager {
    instances: Arc<RwLock<HashMap<String, PtyInstance>>>,
    terminals: Arc<RwLock<HashMap<String, Arc<dyn TerminalEmulator>>>>,
    bridges: Arc<RwLock<HashMap<String, WebSocketBridge>>>,
    max_instances: usize,
    idle_timeout_secs: u64,
}

impl PtyManager {
    /// 创建新的PTY管理器
    pub fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            terminals: Arc::new(RwLock::new(HashMap::new())),
            bridges: Arc::new(RwLock::new(HashMap::new())),
            max_instances: 100,
            idle_timeout_secs: 3600, // 1小时
        }
    }

    /// 创建本地shell PTY
    pub async fn create_local(
        &self,
        id: &str,
        size: TerminalSize,
        shell: Option<&str>,
    ) -> Result<
        (
            Arc<dyn TerminalEmulator>,
            mpsc::UnboundedReceiver<TerminalOutput>,
        ),
        LiteError,
    > {
        let (terminal, output_rx) = PtyTerminal::new_local(id, size, shell)?;
        let terminal_arc: Arc<dyn TerminalEmulator> = Arc::new(terminal);

        // 注册实例
        let instance = PtyInstance {
            id: id.to_string(),
            pty_type: PtyType::LocalShell,
            status: PtyStatus::Running,
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            bytes_read: 0,
            bytes_written: 0,
        };

        {
            let mut instances = self.instances.write().await;
            instances.insert(id.to_string(), instance);
        }

        {
            let mut terminals = self.terminals.write().await;
            terminals.insert(id.to_string(), terminal_arc.clone());
        }

        log::info!("Created local PTY: {}", id);
        Ok((terminal_arc, output_rx))
    }

    /// 创建SSH PTY
    pub async fn create_ssh(
        &self,
        id: &str,
        size: TerminalSize,
        host: &str,
        port: u16,
        username: &str,
        auth_method: SshAuthMethod,
    ) -> Result<
        (
            Arc<dyn TerminalEmulator>,
            mpsc::UnboundedReceiver<TerminalOutput>,
        ),
        LiteError,
    > {
        let (terminal, output_rx) =
            PtyTerminal::new_ssh(id, size, host, port, username, auth_method)?;
        let terminal_arc: Arc<dyn TerminalEmulator> = Arc::new(terminal);

        let instance = PtyInstance {
            id: id.to_string(),
            pty_type: PtyType::Ssh,
            status: PtyStatus::Running,
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            bytes_read: 0,
            bytes_written: 0,
        };

        {
            let mut instances = self.instances.write().await;
            instances.insert(id.to_string(), instance);
        }

        {
            let mut terminals = self.terminals.write().await;
            terminals.insert(id.to_string(), terminal_arc.clone());
        }

        log::info!("Created SSH PTY: {} -> {}@{}", id, username, host);
        Ok((terminal_arc, output_rx))
    }

    /// 获取PTY实例信息
    pub async fn get_instance(&self, id: &str) -> Option<PtyInstance> {
        let instances = self.instances.read().await;
        instances.get(id).cloned()
    }

    /// 获取终端
    pub async fn get_terminal(&self, id: &str) -> Option<Arc<dyn TerminalEmulator>> {
        let terminals = self.terminals.read().await;
        terminals.get(id).cloned()
    }

    /// 终止PTY实例
    pub async fn terminate(&self, id: &str) -> Result<(), LiteError> {
        // 关闭终端
        if let Some(terminal) = self.get_terminal(id).await {
            terminal.close().await?;
        }

        // 更新实例状态
        {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(id) {
                instance.status = PtyStatus::Terminated;
            }
        }

        // 清理
        {
            let mut terminals = self.terminals.write().await;
            terminals.remove(id);
        }

        {
            let mut bridges = self.bridges.write().await;
            bridges.remove(id);
        }

        log::info!("Terminated PTY: {}", id);
        Ok(())
    }

    /// 创建WebSocket桥接
    pub async fn create_bridge(
        &self,
        terminal_id: &str,
        config: Option<BridgeConfig>,
    ) -> Result<mpsc::UnboundedReceiver<TerminalWsOutput>, LiteError> {
        let terminal = self.get_terminal(terminal_id).await.ok_or_else(|| {
            LiteError::TerminalEmulator(format!("Terminal {} not found", terminal_id))
        })?;

        let (bridge, output_rx) = WebSocketBridge::new(terminal_id.to_string(), terminal, config);

        {
            let mut bridges = self.bridges.write().await;
            bridges.insert(terminal_id.to_string(), bridge);
        }

        Ok(output_rx)
    }

    /// 获取桥接器
    pub async fn get_bridge(&self, terminal_id: &str) -> Option<WebSocketBridge> {
        let bridges = self.bridges.read().await;
        bridges.get(terminal_id).cloned()
    }

    /// 更新实例活动
    pub async fn touch(&self, id: &str) {
        let mut instances = self.instances.write().await;
        if let Some(instance) = instances.get_mut(id) {
            instance.last_activity = chrono::Utc::now();
        }
    }

    /// 更新I/O统计
    pub async fn update_io_stats(&self, id: &str, bytes_read: u64, bytes_written: u64) {
        let mut instances = self.instances.write().await;
        if let Some(instance) = instances.get_mut(id) {
            instance.bytes_read += bytes_read;
            instance.bytes_written += bytes_written;
        }
    }

    /// 列出所有实例
    pub async fn list_instances(&self) -> Vec<PtyInstance> {
        let instances = self.instances.read().await;
        instances.values().cloned().collect()
    }

    /// 获取运行中的实例数
    pub async fn running_count(&self) -> usize {
        let instances = self.instances.read().await;
        instances
            .values()
            .filter(|i| i.status == PtyStatus::Running)
            .count()
    }

    /// 清理空闲实例
    pub async fn cleanup_idle(&self) -> Result<usize, LiteError> {
        let now = chrono::Utc::now();
        let idle_ids: Vec<String> = {
            let instances = self.instances.read().await;
            instances
                .values()
                .filter(|i| {
                    i.status == PtyStatus::Running
                        && (now - i.last_activity).num_seconds() > self.idle_timeout_secs as i64
                })
                .map(|i| i.id.clone())
                .collect()
        };

        let count = idle_ids.len();
        for id in idle_ids {
            let _ = self.terminate(&id).await;
        }

        Ok(count)
    }

    /// 终止所有实例
    pub async fn terminate_all(&self) {
        let ids: Vec<String> = {
            let instances = self.instances.read().await;
            instances.keys().cloned().collect()
        };

        for id in ids {
            let _ = self.terminate(&id).await;
        }
    }

    /// 获取管理器统计
    pub async fn stats(&self) -> PtyManagerStats {
        let instances = self.instances.read().await;
        let running = instances
            .values()
            .filter(|i| i.status == PtyStatus::Running)
            .count();
        let total_bytes_read: u64 = instances.values().map(|i| i.bytes_read).sum();
        let total_bytes_written: u64 = instances.values().map(|i| i.bytes_written).sum();

        PtyManagerStats {
            total_instances: instances.len(),
            running_instances: running,
            total_bytes_read,
            total_bytes_written,
        }
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// PTY管理器统计
#[derive(Debug, Clone)]
pub struct PtyManagerStats {
    pub total_instances: usize,
    pub running_instances: usize,
    pub total_bytes_read: u64,
    pub total_bytes_written: u64,
}

// ============================================================================
// Renderer Manager - 渲染器管理与协调
// ============================================================================

/// 渲染器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererType {
    /// WebGL GPU加速
    WebGl,
    /// Canvas 2D (降级)
    Canvas2D,
    /// DOM (最慢但最兼容)
    Dom,
}

/// 渲染器配置
#[derive(Debug, Clone)]
pub struct RendererConfig {
    pub renderer_type: RendererType,
    pub enable_gpu_acceleration: bool,
    pub target_fps: u32,
    pub font_size: f32,
    pub line_height: f32,
    pub cursor_blink: bool,
    pub scrollback_lines: usize,
    pub word_wrap: bool,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            renderer_type: RendererType::WebGl,
            enable_gpu_acceleration: true,
            target_fps: 60,
            font_size: 14.0,
            line_height: 1.2,
            cursor_blink: true,
            scrollback_lines: 10000,
            word_wrap: false,
        }
    }
}

/// 渲染器管理器
/// 管理终端渲染器的生命周期和配置
pub struct RendererManager {
    config: Arc<RwLock<RendererConfig>>,
    active_renderers: Arc<RwLock<HashMap<String, RendererType>>>,
    performance_stats: Arc<RwLock<HashMap<String, RenderPerformance>>>,
}

/// 渲染性能统计
#[derive(Debug, Clone, Default)]
pub struct RenderPerformance {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub dropped_frames: u64,
    pub gpu_memory_mb: f32,
}

impl RendererManager {
    /// 创建新的渲染器管理器
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(RendererConfig::default())),
            active_renderers: Arc::new(RwLock::new(HashMap::new())),
            performance_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册新的渲染器实例
    pub async fn register(&self, terminal_id: &str, renderer_type: RendererType) {
        let mut renderers = self.active_renderers.write().await;
        renderers.insert(terminal_id.to_string(), renderer_type);

        let mut stats = self.performance_stats.write().await;
        stats.insert(terminal_id.to_string(), RenderPerformance::default());

        log::info!(
            "Registered renderer for terminal {}: {:?}",
            terminal_id,
            renderer_type
        );
    }

    /// 注销渲染器
    pub async fn unregister(&self, terminal_id: &str) {
        let mut renderers = self.active_renderers.write().await;
        renderers.remove(terminal_id);

        let mut stats = self.performance_stats.write().await;
        stats.remove(terminal_id);

        log::info!("Unregistered renderer for terminal {}", terminal_id);
    }

    /// 获取当前配置
    pub async fn get_config(&self) -> RendererConfig {
        self.config.read().await.clone()
    }

    /// 更新配置
    pub async fn update_config(&self, config: RendererConfig) {
        let mut cfg = self.config.write().await;
        *cfg = config;
        log::info!("Renderer config updated");
    }

    /// 更新性能统计
    pub async fn update_performance(&self, terminal_id: &str, performance: RenderPerformance) {
        let mut stats = self.performance_stats.write().await;
        stats.insert(terminal_id.to_string(), performance);
    }

    /// 获取性能统计
    pub async fn get_performance(&self, terminal_id: &str) -> Option<RenderPerformance> {
        let stats = self.performance_stats.read().await;
        stats.get(terminal_id).cloned()
    }

    /// 检测最佳渲染器类型
    pub fn detect_optimal_renderer() -> RendererType {
        // 在Web环境中检测GPU支持
        // 这里返回默认的WebGL
        RendererType::WebGl
    }

    /// 获取推荐配置
    pub fn get_recommended_config(&self) -> RendererConfig {
        let renderer_type = Self::detect_optimal_renderer();
        RendererConfig {
            renderer_type,
            enable_gpu_acceleration: matches!(renderer_type, RendererType::WebGl),
            ..Default::default()
        }
    }

    /// 根据性能自动调整配置
    pub async fn auto_optimize(&self, terminal_id: &str) -> Option<RendererConfig> {
        if let Some(perf) = self.get_performance(terminal_id).await {
            let current_config = self.get_config().await;

            // 如果FPS持续低于目标的一半，降级渲染器
            if perf.fps < current_config.target_fps as f32 * 0.5 {
                let new_renderer = match current_config.renderer_type {
                    RendererType::WebGl => RendererType::Canvas2D,
                    RendererType::Canvas2D => RendererType::Dom,
                    RendererType::Dom => RendererType::Dom,
                };

                if new_renderer != current_config.renderer_type {
                    log::warn!(
                        "Low FPS detected ({}), downgrading renderer to {:?}",
                        perf.fps,
                        new_renderer
                    );

                    return Some(RendererConfig {
                        renderer_type: new_renderer,
                        target_fps: current_config.target_fps / 2,
                        ..current_config
                    });
                }
            }
        }

        None
    }

    /// 列出活跃渲染器
    pub async fn list_active(&self) -> Vec<(String, RendererType)> {
        let renderers = self.active_renderers.read().await;
        renderers.iter().map(|(k, v)| (k.clone(), *v)).collect()
    }

    /// 获取总渲染器数量
    pub async fn count(&self) -> usize {
        let renderers = self.active_renderers.read().await;
        renderers.len()
    }
}

impl Default for RendererManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Terminal Session Coordinator - 会话协调器
// ============================================================================

/// 终端会话协调器
/// 协调多个组件之间的交互
pub struct TerminalCoordinator {
    pty_manager: Arc<PtyManager>,
    renderer_manager: Arc<RendererManager>,
    tab_manager: Arc<super::multitab::TabManager>,
    theme_manager: Arc<super::theme::ThemeManager>,
}

impl TerminalCoordinator {
    /// 创建新的协调器
    pub fn new(
        pty_manager: Arc<PtyManager>,
        renderer_manager: Arc<RendererManager>,
        tab_manager: Arc<super::multitab::TabManager>,
        theme_manager: Arc<super::theme::ThemeManager>,
    ) -> Self {
        Self {
            pty_manager,
            renderer_manager,
            tab_manager,
            theme_manager,
        }
    }

    /// 创建新的终端会话
    pub async fn create_session(
        &self,
        session_type: super::multitab::SessionType,
        size: TerminalSize,
    ) -> Result<String, LiteError> {
        let id = uuid::Uuid::new_v4().to_string();

        // 创建标签页
        let tab_id = self
            .tab_manager
            .create_tab(&session_type.default_title(), session_type, None)
            .await?;

        // 创建PTY
        let (_terminal, output_rx) = match session_type {
            super::multitab::SessionType::LocalShell => {
                self.pty_manager.create_local(&id, size, None).await?
            }
            _ => {
                return Err(LiteError::TerminalEmulator(format!(
                    "Session type {:?} not supported via coordinator",
                    session_type
                )))
            }
        };

        // 关联标签页和终端
        self.tab_manager.attach_terminal(&tab_id, &id).await?;

        // 注册渲染器
        self.renderer_manager
            .register(&id, RendererType::WebGl)
            .await;

        // 启动输出转发
        let coordinator = self.clone();
        let id_for_task = id.clone();
        tokio::spawn(async move {
            coordinator
                .forward_terminal_output(&id_for_task, output_rx)
                .await;
        });

        log::info!("Created terminal session: {} (tab: {})", id, tab_id);
        Ok(id)
    }

    /// 转发终端输出
    async fn forward_terminal_output(
        &self,
        terminal_id: &str,
        mut rx: mpsc::UnboundedReceiver<TerminalOutput>,
    ) {
        while let Some(output) = rx.recv().await {
            // 更新统计
            if let TerminalOutput::Data(ref data) = output {
                self.pty_manager
                    .update_io_stats(terminal_id, data.len() as u64, 0)
                    .await;
            }

            if matches!(output, TerminalOutput::Closed) {
                break;
            }
        }

        // 清理
        self.renderer_manager.unregister(terminal_id).await;
        log::info!("Terminal output forwarder exited: {}", terminal_id);
    }

    /// 关闭会话
    pub async fn close_session(&self, terminal_id: &str) -> Result<(), LiteError> {
        // 获取关联的标签页
        let tabs = self.tab_manager.list_tabs().await;
        let tab_ids: Vec<String> = tabs
            .into_iter()
            .filter(|t| t.terminal_id.as_deref() == Some(terminal_id))
            .map(|t| t.id)
            .collect();

        // 关闭标签页
        for tab_id in tab_ids {
            let _ = self.tab_manager.close_tab(&tab_id).await;
        }

        // 终止PTY
        self.pty_manager.terminate(terminal_id).await?;

        log::info!("Closed terminal session: {}", terminal_id);
        Ok(())
    }

    /// 获取完整会话信息
    pub async fn get_session_info(&self, terminal_id: &str) -> Option<SessionInfo> {
        let terminal = self.pty_manager.get_terminal(terminal_id).await?;
        let instance = self.pty_manager.get_instance(terminal_id).await?;
        let performance = self.renderer_manager.get_performance(terminal_id).await;
        let session_info = terminal.session_info();

        Some(SessionInfo {
            id: terminal_id.to_string(),
            title: session_info.title,
            pty_type: instance.pty_type,
            status: instance.status,
            created_at: instance.created_at,
            last_activity: instance.last_activity,
            size: session_info.size,
            performance,
            bytes_read: instance.bytes_read,
            bytes_written: instance.bytes_written,
        })
    }

    /// 获取所有会话信息
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let mut sessions = Vec::new();
        let instances = self.pty_manager.list_instances().await;

        for instance in instances {
            if let Some(info) = self.get_session_info(&instance.id).await {
                sessions.push(info);
            }
        }

        sessions
    }

    /// 应用主题
    pub async fn apply_theme(&self, theme_name: &str) -> Result<(), String> {
        self.theme_manager.set_theme(theme_name).await
    }
}

impl Clone for TerminalCoordinator {
    fn clone(&self) -> Self {
        Self {
            pty_manager: self.pty_manager.clone(),
            renderer_manager: self.renderer_manager.clone(),
            tab_manager: self.tab_manager.clone(),
            theme_manager: self.theme_manager.clone(),
        }
    }
}

/// 完整会话信息
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub title: String,
    pub pty_type: PtyType,
    pub status: PtyStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub size: TerminalSize,
    pub performance: Option<RenderPerformance>,
    pub bytes_read: u64,
    pub bytes_written: u64,
}

// ============================================================================
// Embedded Terminal Server - WebSocket服务端
// ============================================================================

/// 嵌入式终端WebSocket服务端配置
#[derive(Debug, Clone)]
pub struct TerminalServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub auth_token: Option<String>,
    pub enable_cors: bool,
}

impl Default for TerminalServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8999,
            max_connections: 100,
            auth_token: None,
            enable_cors: true,
        }
    }
}

/// 嵌入式终端服务器
/// 提供WebSocket服务，允许前端连接
pub struct EmbeddedTerminalServer {
    config: TerminalServerConfig,
    coordinator: Arc<TerminalCoordinator>,
    connections: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<TerminalWsOutput>>>>,
}

impl EmbeddedTerminalServer {
    /// 创建新的服务器
    pub fn new(config: TerminalServerConfig, coordinator: Arc<TerminalCoordinator>) -> Self {
        Self {
            config,
            coordinator,
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 启动服务器
    pub async fn start(&self) -> Result<(), LiteError> {
        log::info!(
            "Starting embedded terminal server on {}:{}",
            self.config.host,
            self.config.port
        );

        // 实际的服务器启动需要集成到应用的主HTTP服务器
        // 这里提供配置和协调逻辑

        Ok(())
    }

    /// 处理新的WebSocket连接
    pub async fn handle_connection(
        &self,
        session_id: String,
        terminal_id: String,
        mut ws_tx: mpsc::UnboundedSender<WsMessage>,
        mut ws_rx: mpsc::UnboundedReceiver<String>,
    ) -> Result<(), LiteError> {
        log::info!(
            "New WebSocket connection: {} for terminal {}",
            session_id,
            terminal_id
        );

        // 设置WebSocket会话ID
        if let Some(terminal) = self
            .coordinator
            .pty_manager
            .get_terminal(&terminal_id)
            .await
        {
            terminal.set_websocket_session(&session_id).await;
        }

        // 创建桥接
        let (output_tx, mut output_rx) = mpsc::unbounded_channel::<TerminalWsOutput>();

        {
            let mut connections = self.connections.write().await;
            connections.insert(session_id.clone(), output_tx);
        }

        // 启动输出转发任务
        let session_id_clone = session_id.clone();
        let connections = self.connections.clone();
        tokio::spawn(async move {
            while let Some(msg) = output_rx.recv().await {
                let json = match serde_json::to_string(&msg) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Failed to serialize message: {}", e);
                        continue;
                    }
                };

                if ws_tx.send(WsMessage::Text(json)).is_err() {
                    break;
                }
            }

            // 连接断开，清理
            let mut conns = connections.write().await;
            conns.remove(&session_id_clone);
            log::info!("WebSocket connection closed: {}", session_id_clone);
        });

        // 处理输入消息
        while let Some(msg_str) = ws_rx.recv().await {
            let msg: TerminalWsMessage = match serde_json::from_str(&msg_str) {
                Ok(m) => m,
                Err(e) => {
                    log::warn!("Invalid message format: {}", e);
                    continue;
                }
            };

            // 转发到桥接器
            if let Some(terminal) = self
                .coordinator
                .pty_manager
                .get_terminal(&terminal_id)
                .await
            {
                match msg {
                    TerminalWsMessage::Input { data } => {
                        let _ = terminal.write(&data).await;
                    }
                    TerminalWsMessage::Binary { data } => {
                        let _ = terminal.write_bytes(&data).await;
                    }
                    TerminalWsMessage::Resize {
                        rows,
                        cols,
                        width,
                        height,
                    } => {
                        let size = TerminalSize::new(rows, cols).with_pixels(width, height);
                        let _ = terminal.resize(size).await;
                    }
                    TerminalWsMessage::Signal { signal } => {
                        let sig = match signal.as_str() {
                            "interrupt" => Some(TerminalSignal::Interrupt),
                            "eof" => Some(TerminalSignal::Eof),
                            "suspend" => Some(TerminalSignal::Suspend),
                            "quit" => Some(TerminalSignal::Quit),
                            _ => None,
                        };
                        if let Some(s) = sig {
                            let _ = terminal.send_signal(s).await;
                        }
                    }
                    _ => {}
                }

                // 更新活动
                self.coordinator.pty_manager.touch(&terminal_id).await;
            }
        }

        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&self, session_id: &str) {
        let mut connections = self.connections.write().await;
        connections.remove(session_id);
        log::info!("Disconnected session: {}", session_id);
    }

    /// 获取活跃连接数
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }
}

// ============================================================================
// 类型导出
// ============================================================================

pub use super::webgl::{RenderStats as WebGlRenderStats, WebGlConfig, WebGlRenderer};
pub use super::xterm_compat::{XtermCompat, XtermMode};
