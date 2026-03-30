pub mod ai_programming;
pub mod crypto;
pub mod db;
pub mod debug_ws;
pub mod edition;
pub mod error;
pub mod keychain;
#[cfg(feature = "pro")]
pub mod pro;
#[cfg(feature = "sftp")]
pub mod sftp;
pub mod ssh;
pub mod terminal;

pub use ai_programming::{
    ai_build, ai_check_rust, ai_health_check, ai_list_files, ai_read_code, ai_run_tests,
    ai_search_code, debug_test_all, debug_test_crypto, debug_test_db, debug_test_pro,
    debug_test_ssh, debug_test_terminal, DebugTestReport, DebugTestResult,
};
pub use db::{
    AuditEventRecord, GroupRecord, HostRecord, IdentityRecord, LayoutRecord, NewAuditEvent,
    NewGroup, NewHost, NewIdentity, NewLayout, NewServer, NewSession, NewSnippet, NewTag,
    NewSyncState, SessionRecord, ServerRecord, SnippetRecord, SyncStateRecord, TagRecord,
    UpdateGroup, UpdateHost, UpdateIdentity, UpdateLayout, UpdateServer, UpdateSession,
    UpdateSnippet, UpdateTag, UpdateSyncState,
};
pub use edition::{Edition, VersionInfo};
pub use error::LiteError;
#[cfg(feature = "sftp")]
pub use sftp::SftpSessionManager;
pub use ssh::{MuxStats, SshSessionManager};

use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex;

/// Application state for native platforms
pub struct AppState {
    pub db: StdMutex<Option<db::Database>>,
    pub ssh_manager: Mutex<SshSessionManager>,
    #[cfg(feature = "sftp")]
    pub sftp_manager: Mutex<SftpSessionManager>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            db: StdMutex::new(None),
            ssh_manager: Mutex::new(SshSessionManager::new()),
            #[cfg(feature = "sftp")]
            sftp_manager: Mutex::new(SftpSessionManager::new()),
        }
    }
}

/// Get database path
pub fn get_db_path() -> std::path::PathBuf {
    db::get_db_path()
}

/// Get all servers
pub fn get_servers(state: &AppState) -> Result<Vec<ServerRecord>, LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.get_servers()
}

/// Get single server
pub fn get_server(state: &AppState, id: &str) -> Result<ServerRecord, LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.get_server(id)
}

/// Add server
pub fn add_server(state: &AppState, server: &NewServer) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.add_server(server)
}

/// Update server
pub fn update_server(state: &AppState, server: &UpdateServer) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.update_server(server)
}

/// Delete server
pub fn delete_server(state: &AppState, id: &str) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.delete_server(id)
}

/// Get all groups
pub fn get_groups(state: &AppState) -> Result<Vec<GroupRecord>, LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.get_groups()
}

/// Add group
pub fn add_group(state: &AppState, group: &NewGroup) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.add_group(group)
}

/// Update group
pub fn update_group(state: &AppState, group: &UpdateGroup) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.update_group(group)
}

/// Delete group
pub fn delete_group(state: &AppState, id: &str) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.delete_group(id)
}

/// Initialize database
pub fn init_database(state: &AppState) -> Result<(), LiteError> {
    let db_path = get_db_path();
    let db = db::Database::new(db_path)?;
    db.init()?;

    let mut db_lock = state.db.lock().unwrap();
    *db_lock = Some(db);

    Ok(())
}

/// Open native terminal and connect (Lite mode)
pub fn connect_server(state: &AppState, id: &str) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;

    let server = db.get_server(id)?;
    terminal::open_native_terminal(
        &server.host,
        server.port as u16,
        &server.username,
        &server.auth_type,
    )
}

/// Connect to SSH and return session ID (Standard/Pro mode)
pub async fn ssh_connect(
    state: &AppState,
    id: &str,
    password: Option<&str>,
) -> Result<String, LiteError> {
    let (host, port, username): (String, u16, String) = {
        let db_lock = state.db.lock().unwrap();
        let db = db_lock
            .as_ref()
            .ok_or(LiteError::Config("Database not initialized".to_string()))?;
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

/// Execute SSH command
pub async fn ssh_execute(
    state: &AppState,
    session_id: &str,
    command: &str,
) -> Result<String, LiteError> {
    let ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.execute(session_id, command).await
}

/// Disconnect SSH session
pub async fn ssh_disconnect(state: &AppState, session_id: &str) -> Result<(), LiteError> {
    let mut ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.disconnect(session_id).await
}

/// List active SSH sessions
pub fn ssh_list_sessions(state: &AppState) -> Vec<String> {
    let ssh_manager = state.ssh_manager.blocking_lock();
    ssh_manager.list_sessions()
}

/// Get SSH MUX stats
pub fn ssh_get_mux_stats(state: &AppState) -> MuxStats {
    let ssh_manager = state.ssh_manager.blocking_lock();
    ssh_manager.get_mux_stats()
}
