pub mod ai_programming;
pub mod crypto;
pub mod db;
pub mod error;
pub mod keychain;
pub mod pro;
pub mod sftp;
pub mod ssh;
pub mod terminal;

use db::{GroupRecord, NewGroup, NewServer, ServerRecord, UpdateGroup, UpdateServer};
use error::LiteError;
use sftp::SftpSessionManager;
use ssh::{SshSessionManager, MuxStats};
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex;

/// 应用状态
pub struct AppState {
    pub db: StdMutex<Option<db::Database>>,
    pub ssh_manager: Mutex<SshSessionManager>,
    pub sftp_manager: Mutex<SftpSessionManager>,
}

/// 获取数据库路径
pub fn get_db_path() -> std::path::PathBuf {
    db::get_db_path()
}

/// 获取所有服务器
pub fn get_servers(state: &AppState) -> Result<Vec<ServerRecord>, LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("数据库未初始化".to_string()))?;
    db.get_servers()
}

/// 获取单个服务器
pub fn get_server(state: &AppState, id: &str) -> Result<ServerRecord, LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("数据库未初始化".to_string()))?;
    db.get_server(id)
}

/// 添加服务器
pub fn add_server(state: &AppState, server: &NewServer) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("数据库未初始化".to_string()))?;
    db.add_server(server)
}

/// 更新服务器
pub fn update_server(state: &AppState, server: &UpdateServer) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("数据库未初始化".to_string()))?;
    db.update_server(server)
}

/// 删除服务器
pub fn delete_server(state: &AppState, id: &str) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("数据库未初始化".to_string()))?;
    db.delete_server(id)
}

/// 获取所有分组
pub fn get_groups(state: &AppState) -> Result<Vec<GroupRecord>, LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("数据库未初始化".to_string()))?;
    db.get_groups()
}

/// 添加分组
pub fn add_group(state: &AppState, group: &NewGroup) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("数据库未初始化".to_string()))?;
    db.add_group(group)
}

/// 更新分组
pub fn update_group(state: &AppState, group: &UpdateGroup) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("数据库未初始化".to_string()))?;
    db.update_group(group)
}

/// 删除分组
pub fn delete_group(state: &AppState, id: &str) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("数据库未初始化".to_string()))?;
    db.delete_group(id)
}

/// 初始化数据库
pub fn init_database(state: &AppState) -> Result<(), LiteError> {
    let db_path = get_db_path();
    let db = db::Database::new(db_path)?;
    db.init()?;

    let mut db_lock = state.db.lock().unwrap();
    *db_lock = Some(db);

    Ok(())
}

/// 唤起原生终端连接服务器
pub fn connect_server(state: &AppState, id: &str) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("数据库未初始化".to_string()))?;

    let server = db.get_server(id)?;
    terminal::open_native_terminal(
        &server.host,
        server.port as u16,
        &server.username,
        &server.auth_type,
    )
}

/// 连接SSH服务器并返回会话ID (用于嵌入式终端)
pub async fn ssh_connect(
    state: &AppState,
    id: &str,
    password: Option<&str>,
) -> Result<String, LiteError> {
    // 获取服务器信息（同步持有锁）
    let (host, port, username): (String, u16, String) = {
        let db_lock = state.db.lock().unwrap();
        let db = db_lock
            .as_ref()
            .ok_or(LiteError::Config("数据库未初始化".to_string()))?;
        let server = db.get_server(id)?;
        (
            server.host.clone(),
            server.port as u16,
            server.username.clone(),
        )
    };

    let session_id = uuid::Uuid::new_v4().to_string();
    let mut ssh_manager = state.ssh_manager.lock().await;
    ssh_manager
        .connect(&session_id, &host, port, &username, password)
        .await?;

    Ok(session_id)
}

/// 执行SSH命令
pub async fn ssh_execute(
    state: &AppState,
    session_id: &str,
    command: &str,
) -> Result<String, LiteError> {
    let ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.execute(session_id, command).await
}

/// 断开SSH会话
pub async fn ssh_disconnect(state: &AppState, session_id: &str) -> Result<(), LiteError> {
    let mut ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.disconnect(session_id).await
}

/// 获取所有活跃SSH会话
pub fn ssh_list_sessions(state: &AppState) -> Vec<String> {
    let ssh_manager = state.ssh_manager.blocking_lock();
    ssh_manager.list_sessions()
}

/// 获取SSH MUX统计信息
pub fn ssh_get_mux_stats(state: &AppState) -> MuxStats {
    let ssh_manager = state.ssh_manager.blocking_lock();
    ssh_manager.get_mux_stats()
}
