#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, error, debug};
use futures_util::{SinkExt, StreamExt};

use crate::viewmodels::AppViewModel;

#[derive(Clone, Debug, Default, Serialize)]
pub struct UiDebugState {
    pub last_frame_ms: u128,
    pub is_connected: bool,
    pub current_session_id: Option<String>,
    pub terminal_buffer_len: usize,
    pub command_input_len: usize,
}

static UI_DEBUG_STATE: std::sync::OnceLock<std::sync::Mutex<UiDebugState>> = std::sync::OnceLock::new();

pub fn update_ui_debug(
    is_connected: bool,
    current_session_id: Option<String>,
    terminal_buffer_len: usize,
    command_input_len: usize,
) {
    let state = UiDebugState {
        last_frame_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        is_connected,
        current_session_id,
        terminal_buffer_len,
        command_input_len,
    };

    let lock = UI_DEBUG_STATE.get_or_init(|| std::sync::Mutex::new(UiDebugState::default()));
    if let Ok(mut guard) = lock.lock() {
        *guard = state;
    }
}

fn read_ui_debug() -> UiDebugState {
    UI_DEBUG_STATE
        .get_or_init(|| std::sync::Mutex::new(UiDebugState::default()))
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

fn process_snapshot() -> serde_json::Value {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let pid = std::process::id();
    let thread = std::thread::current();
    let thread_name = thread.name().unwrap_or("unknown").to_string();

    let ui = read_ui_debug();
    let ui_lag_ms = now_ms.saturating_sub(ui.last_frame_ms);

    serde_json::json!({
        "pid": pid,
        "thread": thread_name,
        "timestamp_ms": now_ms,
        "uptime_hint": "process_alive",
        "ui": ui,
        "ui_lag_ms": ui_lag_ms,
        "ui_stalled": ui_lag_ms > 1500
    })
}

/// WebSocket 控制服务器
pub struct WsControlServer {
    port: u16,
    view_model: Arc<Mutex<AppViewModel>>,
    clients: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Message>>>>,
}

impl WsControlServer {
    pub fn new(port: u16, view_model: Arc<Mutex<AppViewModel>>) -> Self {
        Self {
            port,
            view_model,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 启动 WebSocket 服务器
    pub async fn start(&self) -> anyhow::Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        info!("WebSocket control server started on ws://{}", addr);

        let clients = self.clients.clone();
        let view_model = self.view_model.clone();

        // High-priority background debug stream broadcaster (works even when UI loop stalls)
        let clients_for_debug = clients.clone();
        let view_model_for_debug = view_model.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));
            loop {
                interval.tick().await;

                let stats = match view_model_for_debug.try_lock() {
                    Ok(vm) => {
                        let s = vm.debug_stats();
                        serde_json::json!({
                            "type": "debug_stream",
                            "source": "viewmodel",
                            "stats": s,
                            "process": process_snapshot(),
                        })
                    }
                    Err(_) => {
                        serde_json::json!({
                            "type": "debug_stream",
                            "source": "process_only",
                            "warning": "viewmodel_lock_busy",
                            "process": process_snapshot(),
                        })
                    }
                };

                let msg = Message::text(stats.to_string());
                let clients_guard = clients_for_debug.read().await;
                for (_id, tx) in clients_guard.iter() {
                    let _ = tx.send(msg.clone());
                }
            }
        });

        loop {
            let (stream, addr) = listener.accept().await?;
            let clients = clients.clone();
            let view_model = view_model.clone();
            let client_id = uuid::Uuid::new_v4().to_string();

            info!("WebSocket client connected: {} ({})", client_id, addr);

            tokio::spawn(async move {
                let ws_stream = tokio_tungstenite::accept_async(stream).await;
                match ws_stream {
                    Ok(ws_stream) => {
                        handle_client(client_id, ws_stream, clients, view_model).await;
                    }
                    Err(e) => {
                        error!("WebSocket handshake failed: {}", e);
                    }
                }
            });
        }
    }

    /// 广播消息给所有客户端
    #[allow(dead_code)]
    pub async fn broadcast(&self, msg: &str) {
        let clients = self.clients.read().await;
        for (id, tx) in clients.iter() {
            if let Err(e) = tx.send(Message::text(msg)) {
                debug!("Failed to send to client {}: {}", id, e);
            }
        }
    }
}

async fn handle_client(
    client_id: String,
    stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    clients: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Message>>>>,
    view_model: Arc<Mutex<AppViewModel>>,
) {
    let (mut ws_tx, mut ws_rx) = stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // 注册客户端
    {
        let mut clients_guard = clients.write().await;
        clients_guard.insert(client_id.clone(), tx);
    }

    // 发送欢迎消息
    let welcome = serde_json::json!({
        "type": "welcome",
        "client_id": client_id,
        "message": "EasySSH WebSocket Control API v1.0",
        "commands": [
            "get_servers",
            "connect",
            "disconnect",
            "execute",
            "get_status",
            "add_server",
            "delete_server",
            "save_password",
            "get_password",
            "get_debug_snapshot",
            "interrupt",
            "automation_probe",
            "ping"
        ]
    });
    let _ = ws_tx.send(Message::text(welcome.to_string())).await;

    // 处理发送任务
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 处理接收到的命令
    while let Some(Ok(msg)) = ws_rx.next().await {
        if let Message::Text(text) = msg {
            debug!("Received from client {}: {}", client_id, text);

            match serde_json::from_str::<WsCommand>(&text) {
                Ok(cmd) => {
                    let response = handle_command(cmd, &view_model).await;
                    let response_text = serde_json::to_string(&response).unwrap_or_default();

                    let clients_guard = clients.read().await;
                    if let Some(tx) = clients_guard.get(&client_id) {
                        let _ = tx.send(Message::text(response_text));
                    }
                }
                Err(e) => {
                    let error = serde_json::json!({
                        "type": "error",
                        "error": format!("Invalid command format: {}", e)
                    });
                    let clients_guard = clients.read().await;
                    if let Some(tx) = clients_guard.get(&client_id) {
                        let _ = tx.send(Message::text(error.to_string()));
                    }
                }
            }
        }
    }

    // 清理
    send_task.abort();
    let mut clients_guard = clients.write().await;
    clients_guard.remove(&client_id);
    info!("WebSocket client disconnected: {}", client_id);
}

async fn handle_command(cmd: WsCommand, view_model: &Arc<Mutex<AppViewModel>>) -> WsResponse {
    match cmd.command.as_str() {
        "get_servers" => {
            let vm = view_model.lock().unwrap();
            let servers = vm.get_servers();
            WsResponse {
                success: true,
                data: Some(serde_json::json!({"servers": servers})),
                error: None,
            }
        }

        "connect" => {
            let server_id = cmd.params.get("server_id").and_then(|v| v.as_str());
            let password = cmd.params.get("password").and_then(|v| v.as_str());

            if let Some(id) = server_id {
                let vm = view_model.lock().unwrap();
                let servers = vm.get_servers();

                if let Some(server) = servers.iter().find(|s| s.id == id) {
                    let session_id = uuid::Uuid::new_v4().to_string();

                    match vm.connect(&session_id, &server.host, server.port, &server.username, password) {
                        Ok(_) => {
                            // initialize persistent shell stream
                            let _ = vm.execute_stream(&session_id, "");
                            let _ = vm.write_shell_input(&session_id, b"whoami\n");
                            WsResponse {
                                success: true,
                                data: Some(serde_json::json!({
                                    "session_id": session_id,
                                    "server": server
                                })),
                                error: None,
                            }
                        },
                        Err(e) => WsResponse {
                            success: false,
                            data: None,
                            error: Some(format!("Connection failed: {}", e)),
                        }
                    }
                } else {
                    WsResponse {
                        success: false,
                        data: None,
                        error: Some("Server not found".to_string()),
                    }
                }
            } else {
                WsResponse {
                    success: false,
                    data: None,
                    error: Some("Missing server_id parameter".to_string()),
                }
            }
        }

        "disconnect" => {
            let session_id = cmd.params.get("session_id").and_then(|v| v.as_str());

            if let Some(id) = session_id {
                let vm = view_model.lock().unwrap();
                match vm.disconnect(id) {
                    Ok(_) => WsResponse {
                        success: true,
                        data: Some(serde_json::json!({"disconnected": true})),
                        error: None,
                    },
                    Err(e) => WsResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Disconnect failed: {}", e)),
                    }
                }
            } else {
                // 断开所有会话
                WsResponse {
                    success: true,
                    data: Some(serde_json::json!({"message": "Disconnected all sessions"})),
                    error: None,
                }
            }
        }

        "execute" => {
            let session_id = cmd.params.get("session_id").and_then(|v| v.as_str());
            let command = cmd.params.get("command").and_then(|v| v.as_str());

            if let (Some(id), Some(cmd)) = (session_id, command) {
                let vm = view_model.lock().unwrap();
                // Execute via persistent shell stdin for true terminal semantics
                let line = format!("{}\n", cmd);
                match vm.write_shell_input(id, line.as_bytes()) {
                    Ok(_) => WsResponse {
                        success: true,
                        data: Some(serde_json::json!({
                            "queued": true,
                            "command": cmd
                        })),
                        error: None,
                    },
                    Err(e) => WsResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Command write failed: {}", e)),
                    }
                }
            } else {
                WsResponse {
                    success: false,
                    data: None,
                    error: Some("Missing session_id or command parameter".to_string()),
                }
            }
        }

        "interrupt" => {
            let session_id = cmd.params.get("session_id").and_then(|v| v.as_str());
            if let Some(id) = session_id {
                let vm = view_model.lock().unwrap();
                match vm.interrupt_command(id) {
                    Ok(_) => WsResponse {
                        success: true,
                        data: Some(serde_json::json!({"interrupted": true})),
                        error: None,
                    },
                    Err(e) => WsResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Interrupt failed: {}", e)),
                    },
                }
            } else {
                WsResponse {
                    success: false,
                    data: None,
                    error: Some("Missing session_id".to_string()),
                }
            }
        }

        "get_debug_snapshot" => {
            let stats = match view_model.try_lock() {
                Ok(vm) => serde_json::json!({
                    "source": "viewmodel",
                    "stats": vm.debug_stats(),
                    "process": process_snapshot(),
                }),
                Err(_) => serde_json::json!({
                    "source": "process_only",
                    "warning": "viewmodel_lock_busy",
                    "process": process_snapshot(),
                }),
            };

            WsResponse {
                success: true,
                data: Some(stats),
                error: None,
            }
        }

        "get_status" => {
            WsResponse {
                success: true,
                data: Some(serde_json::json!({
                    "status": "running",
                    "version": "0.3.0"
                })),
                error: None,
            }
        }

        "ping" => {
            WsResponse {
                success: true,
                data: Some(serde_json::json!({
                    "pong": true,
                    "timestamp_ms": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0)
                })),
                error: None,
            }
        }

        "add_server" => {
            let name = cmd.params.get("name").and_then(|v| v.as_str());
            let host = cmd.params.get("host").and_then(|v| v.as_str());
            let port = cmd.params.get("port").and_then(|v| v.as_i64()).unwrap_or(22);
            let username = cmd.params.get("username").and_then(|v| v.as_str());
            let auth_type = cmd.params.get("auth_type").and_then(|v| v.as_str()).unwrap_or("password");

            if let (Some(name), Some(host), Some(username)) = (name, host, username) {
                let vm = view_model.lock().unwrap();
                match vm.add_server(name, host, port, username, auth_type, None) {
                    Ok(_) => WsResponse {
                        success: true,
                        data: Some(serde_json::json!({"added": true, "name": name})),
                        error: None,
                    },
                    Err(e) => WsResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to add server: {}", e)),
                    }
                }
            } else {
                WsResponse {
                    success: false,
                    data: None,
                    error: Some("Missing required parameters: name, host, username".to_string()),
                }
            }
        }

        "delete_server" => {
            let server_id = cmd.params.get("server_id").and_then(|v| v.as_str());

            if let Some(id) = server_id {
                let vm = view_model.lock().unwrap();
                match vm.delete_server(id) {
                    Ok(_) => WsResponse {
                        success: true,
                        data: Some(serde_json::json!({"deleted": true, "server_id": id})),
                        error: None,
                    },
                    Err(e) => WsResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to delete server: {}", e)),
                    }
                }
            } else {
                WsResponse {
                    success: false,
                    data: None,
                    error: Some("Missing server_id".to_string()),
                }
            }
        }

        "save_password" => {
            let server_id = cmd.params.get("server_id").and_then(|v| v.as_str());
            let password = cmd.params.get("password").and_then(|v| v.as_str());

            if let (Some(id), Some(pwd)) = (server_id, password) {
                let vm = view_model.lock().unwrap();
                match vm.save_password(id, pwd) {
                    Ok(_) => WsResponse {
                        success: true,
                        data: Some(serde_json::json!({"saved": true})),
                        error: None,
                    },
                    Err(e) => WsResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to save password: {}", e)),
                    }
                }
            } else {
                WsResponse {
                    success: false,
                    data: None,
                    error: Some("Missing server_id or password".to_string()),
                }
            }
        }

        "get_password" => {
            let server_id = cmd.params.get("server_id").and_then(|v| v.as_str());

            if let Some(id) = server_id {
                let vm = view_model.lock().unwrap();
                match vm.get_saved_password(id) {
                    Some(pwd) => WsResponse {
                        success: true,
                        data: Some(serde_json::json!({
                            "has_password": true,
                            "password_length": pwd.len()
                        })),
                        error: None,
                    },
                    None => WsResponse {
                        success: true,
                        data: Some(serde_json::json!({"has_password": false})),
                        error: None,
                    }
                }
            } else {
                WsResponse {
                    success: false,
                    data: None,
                    error: Some("Missing server_id".to_string()),
                }
            }
        }

        "automation_probe" => {
            let server_id = cmd.params.get("server_id").and_then(|v| v.as_str());
            let password = cmd.params.get("password").and_then(|v| v.as_str()).unwrap_or("");

            let mut checks = vec![];

            // 1) process snapshot always available
            let snapshot = process_snapshot();
            checks.push(serde_json::json!({
                "name": "process_snapshot",
                "ok": true,
                "data": snapshot,
            }));

            // 2) viewmodel lock availability
            let vm_ok = view_model.try_lock().is_ok();
            checks.push(serde_json::json!({
                "name": "viewmodel_try_lock",
                "ok": vm_ok,
            }));

            // 3) optional password save/load probe for provided server_id
            if let Some(id) = server_id {
                let vm = view_model.lock().unwrap();

                let save_ok = vm.save_password(id, password).is_ok();
                checks.push(serde_json::json!({
                    "name": "password_save",
                    "ok": save_ok,
                }));

                let loaded = vm.get_saved_password(id);
                let load_ok = loaded.is_some();
                checks.push(serde_json::json!({
                    "name": "password_load",
                    "ok": load_ok,
                    "len": loaded.as_ref().map(|s| s.len()).unwrap_or(0),
                }));
            }

            let all_ok = checks.iter().all(|c| c.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));

            WsResponse {
                success: all_ok,
                data: Some(serde_json::json!({
                    "all_ok": all_ok,
                    "checks": checks,
                })),
                error: if all_ok { None } else { Some("One or more probes failed".to_string()) },
            }
        }

        _ => WsResponse {
            success: false,
            data: None,
            error: Some(format!("Unknown command: {}", cmd.command)),
        }
    }
}

#[derive(Debug, Deserialize)]
struct WsCommand {
    command: String,
    #[serde(default)]
    params: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct WsResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}
