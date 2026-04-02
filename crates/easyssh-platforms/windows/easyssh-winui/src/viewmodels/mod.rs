pub mod port_forward;

use std::sync::Arc;
use std::sync::Mutex;

use crate::viewmodels::port_forward::PortForwardViewModel;
use easyssh_core::keychain::{delete_password, get_password, store_password};
use easyssh_core::sftp::{SftpEntry, SftpManager};
use easyssh_core::{AppState, GroupRecord, NewServer, ServerRecord, SshSessionManager};
use easyssh_core::{
    ConfigConflictResolution as ConflictResolution, ConfigManager, ExportFormat, ImportFormat,
    ImportResult,
};
use serde::Serialize;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Clone)]
pub struct AppViewModel {
    core_state: Arc<Mutex<AppState>>,
    ssh_manager: Arc<Mutex<SshSessionManager>>,
    sftp_manager: Arc<Mutex<SftpManager>>,
    runtime: Arc<Runtime>,
    port_forward_vm: Arc<Mutex<PortForwardViewModel>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DebugStats {
    pub servers_count: usize,
    pub active_sessions: usize,
    pub timestamp_ms: u128,
    pub mux_total_pools: usize,
}

impl AppViewModel {
    pub fn new() -> anyhow::Result<Self> {
        let core_state = Arc::new(Mutex::new(AppState::new()));
        let ssh_manager = Arc::new(Mutex::new(SshSessionManager::new()));
        let sftp_manager = Arc::new(Mutex::new(SftpManager::new()));
        let runtime = Arc::new(Runtime::new()?);

        Self::init_with_components(core_state, ssh_manager, sftp_manager, runtime)
    }

    /// Create with an existing Tokio runtime (for startup optimization)
    pub fn new_with_runtime(runtime: Arc<Runtime>) -> anyhow::Result<Self> {
        let core_state = Arc::new(Mutex::new(AppState::new()));
        let ssh_manager = Arc::new(Mutex::new(SshSessionManager::new()));
        let sftp_manager = Arc::new(Mutex::new(SftpManager::new()));

        Self::init_with_components(core_state, ssh_manager, sftp_manager, runtime)
    }

    /// Initialize database and port forward viewmodel
    fn init_with_components(
        core_state: Arc<Mutex<AppState>>,
        ssh_manager: Arc<Mutex<SshSessionManager>>,
        sftp_manager: Arc<Mutex<SftpManager>>,
        runtime: Arc<Runtime>,
    ) -> anyhow::Result<Self> {
        // Use fast path: check if DB is already initialized before full init
        let db_path = easyssh_core::get_db_path();
        let needs_full_init = match easyssh_core::db::Database::new(db_path) {
            Ok(db) => {
                match db.is_initialized() {
                    Ok(false) => true, // Tables don't exist, need full init
                    _ => false,        // Already initialized or error (assume initialized)
                }
            }
            Err(_) => true, // Can't open DB, try full init
        };

        if needs_full_init {
            let state = core_state.lock().unwrap();
            if let Err(e) = easyssh_core::init_database(&state) {
                error!("Failed to initialize database: {}", e);
            } else {
                info!("Database initialized (full)");
            }
        } else {
            // Fast path: just open the connection without running migrations
            let state = core_state.lock().unwrap();
            if let Ok(db) = easyssh_core::db::Database::new(easyssh_core::get_db_path()) {
                let mut db_lock = state.db.lock().unwrap();
                *db_lock = Some(db);
            }
            info!("Database initialized (fast path)");
        }

        // Defer PortForwardViewModel initialization (it's not needed immediately)
        let port_forward_vm = Arc::new(Mutex::new(
            PortForwardViewModel::new(core_state.clone())
                .map_err(|e| anyhow::anyhow!("Failed to create PortForwardViewModel: {}", e))?,
        ));

        Ok(Self {
            core_state,
            ssh_manager,
            sftp_manager,
            runtime,
            port_forward_vm,
        })
    }

    /// Get the port forward viewmodel
    pub fn get_port_forward_vm(&self) -> std::sync::MutexGuard<'_, PortForwardViewModel> {
        self.port_forward_vm.lock().unwrap()
    }

    /// Get the core state
    pub fn get_core_state(&self) -> Arc<Mutex<AppState>> {
        self.core_state.clone()
    }

    pub fn get_servers(&self) -> Vec<ServerViewModel> {
        let state = self.core_state.lock().unwrap();
        match easyssh_core::get_servers(&state) {
            Ok(servers) => servers.into_iter().map(ServerViewModel::from).collect(),
            Err(e) => {
                error!("Failed to get servers: {}", e);
                vec![]
            }
        }
    }

    pub fn get_groups(&self) -> Vec<GroupViewModel> {
        let state = self.core_state.lock().unwrap();
        match easyssh_core::get_groups(&state) {
            Ok(groups) => groups.into_iter().map(GroupViewModel::from).collect(),
            Err(e) => {
                error!("Failed to get groups: {}", e);
                vec![]
            }
        }
    }

    pub fn add_server(
        &self,
        name: &str,
        host: &str,
        port: i64,
        username: &str,
        auth_type: &str,
        group_id: Option<String>,
    ) -> anyhow::Result<()> {
        let state = self.core_state.lock().unwrap();

        let new_server = NewServer {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            host: host.to_string(),
            port,
            username: username.to_string(),
            auth_type: auth_type.to_string(),
            identity_file: None,
            group_id,
            status: "active".to_string(),
        };

        easyssh_core::add_server(&state, &new_server)?;
        info!("Added server: {}", name);
        Ok(())
    }

    pub fn update_server_group(
        &self,
        server_id: &str,
        group_id: Option<String>,
    ) -> anyhow::Result<()> {
        let state = self.core_state.lock().unwrap();
        let db_lock = state.db.lock().unwrap();
        let db = db_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        // Get current server data
        let server = db
            .get_server(server_id)
            .map_err(|e| anyhow::anyhow!("Failed to get server: {}", e))?;

        // Log before moving group_id
        info!("Updating server {} group to {:?}", server_id, group_id);

        // Update with new group_id
        let update = easyssh_core::UpdateServer {
            id: server_id.to_string(),
            name: Some(server.name),
            host: Some(server.host),
            port: Some(server.port),
            username: Some(server.username),
            auth_type: Some(server.auth_type),
            identity_file: server.identity_file,
            group_id,
            status: Some(server.status),
        };

        db.update_server(&update)
            .map_err(|e| anyhow::anyhow!("Failed to update server group: {}", e))?;

        Ok(())
    }

    pub fn add_group(&self, name: &str) -> anyhow::Result<String> {
        let state = self.core_state.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let new_group = easyssh_core::NewGroup {
            id: id.clone(),
            name: name.to_string(),
            color: "#4A90D9".to_string(),
        };
        easyssh_core::add_group(&state, &new_group)?;
        info!("Added group: {} (id: {})", name, id);
        Ok(id)
    }

    pub fn update_group(&self, id: &str, name: &str) -> anyhow::Result<()> {
        let state = self.core_state.lock().unwrap();
        let update = easyssh_core::UpdateGroup {
            id: id.to_string(),
            name: Some(name.to_string()),
            color: None,
        };
        easyssh_core::update_group(&state, &update)?;
        info!("Updated group {}: {}", id, name);
        Ok(())
    }

    pub fn delete_group(&self, id: &str) -> anyhow::Result<()> {
        let state = self.core_state.lock().unwrap();
        easyssh_core::delete_group(&state, id)?;
        info!("Deleted group: {}", id);
        Ok(())
    }

    pub fn delete_server(&self, server_id: &str) -> anyhow::Result<()> {
        let state = self.core_state.lock().unwrap();
        easyssh_core::delete_server(&state, server_id)
            .map_err(|e| anyhow::anyhow!("Failed to delete server: {}", e))?;

        // Best-effort cleanup of stored password for this server
        if let Err(e) = delete_password(server_id) {
            tracing::warn!(
                "Delete server password cleanup warning for {}: {}",
                server_id,
                e
            );
        }

        info!("Deleted server: {}", server_id);
        Ok(())
    }

    pub fn update_server(
        &self,
        server_id: &str,
        name: &str,
        host: &str,
        port: i64,
        username: &str,
        auth_type: &str,
    ) -> anyhow::Result<()> {
        let state = self.core_state.lock().unwrap();

        let update = easyssh_core::db::UpdateServer {
            id: server_id.to_string(),
            name: Some(name.to_string()),
            host: Some(host.to_string()),
            port: Some(port),
            username: Some(username.to_string()),
            auth_type: Some(auth_type.to_string()),
            identity_file: None,
            group_id: None,
            status: Some("active".to_string()),
        };

        easyssh_core::update_server(&state, &update)
            .map_err(|e| anyhow::anyhow!("Failed to update server: {}", e))?;

        info!("Updated server: {} ({})", name, server_id);
        Ok(())
    }

    pub fn get_saved_password(&self, server_id: &str) -> Option<String> {
        match get_password(server_id) {
            Ok(Some(pwd)) => {
                info!("Retrieved saved password for server {}", server_id);
                Some(pwd)
            }
            Ok(None) => {
                info!("No saved password found for server {}", server_id);
                None
            }
            Err(e) => {
                error!("Failed to get password from keychain/fallback: {}", e);
                None
            }
        }
    }

    pub fn save_password(&self, server_id: &str, password: &str) -> anyhow::Result<()> {
        store_password(server_id, password)
            .map_err(|e| anyhow::anyhow!("Failed to save password: {}", e))?;

        // immediate verify read-back (hard guarantee)
        match get_password(server_id) {
            Ok(Some(_)) => {
                info!("Password saved and verified for server {}", server_id);
                Ok(())
            }
            Ok(None) => Err(anyhow::anyhow!(
                "Password save verification failed: value not found after save"
            )),
            Err(e) => Err(anyhow::anyhow!("Password save verification failed: {}", e)),
        }
    }

    pub fn connect(
        &self,
        session_id: &str,
        host: &str,
        port: i64,
        username: &str,
        password: Option<&str>,
    ) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();

        rt.block_on(async {
            let mut mgr = manager.lock().unwrap();
            mgr.connect(session_id, host, port as u16, username, password)
                .await
                .map(|_| ())
                .map_err(|e| anyhow::anyhow!("SSH connection failed: {}", e))
        })
    }

    #[allow(dead_code)]
    pub fn is_connected(&self, session_id: &str) -> bool {
        let manager = self.ssh_manager.lock().unwrap();
        manager.has_session(session_id)
    }

    #[allow(dead_code)]
    pub fn execute_command(&self, session_id: &str, command: &str) -> anyhow::Result<String> {
        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();

        rt.block_on(async {
            let mgr = manager.lock().unwrap();
            mgr.execute(session_id, command)
                .await
                .map_err(|e| anyhow::anyhow!("Command execution failed: {}", e))
        })
    }

    /// Execute command with timeout (for monitor commands)
    pub fn execute_blocking_with_timeout(
        &self,
        session_id: &str,
        command: &str,
        timeout_secs: u64,
    ) -> anyhow::Result<String> {
        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();
        let sid = session_id.to_string();
        let cmd = command.to_string();

        rt.block_on(async move {
            match tokio::time::timeout(tokio::time::Duration::from_secs(timeout_secs), async {
                let mgr = manager.lock().unwrap();
                mgr.execute(&sid, &cmd)
                    .await
                    .map_err(|e| anyhow::anyhow!("SSH command failed: {}", e))
            })
            .await
            {
                Ok(result) => result,
                Err(_) => Err(anyhow::anyhow!("Command timed out after {}s", timeout_secs)),
            }
        })
    }

    /// Execute command via SFTP session (for monitor commands - avoids shell channel conflicts)
    pub fn execute_via_sftp(&self, session_id: &str, command: &str) -> anyhow::Result<String> {
        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();

        rt.block_on(async {
            let mgr = manager.lock().unwrap();
            mgr.execute_via_sftp(session_id, command)
                .await
                .map_err(|e| anyhow::anyhow!("SFTP command failed: {}", e))
        })
    }

    pub fn execute_stream(
        &self,
        session_id: &str,
        command: &str,
    ) -> anyhow::Result<mpsc::UnboundedReceiver<String>> {
        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();
        let sid = session_id.to_string();
        let cmd = command.to_string();

        rt.block_on(async {
            let mut mgr = manager.lock().unwrap();
            mgr.execute_stream(&sid, &cmd)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start streaming: {}", e))
        })
    }

    pub fn write_shell_input(&self, session_id: &str, input: &[u8]) -> anyhow::Result<()> {
        // Clone Arc OUTSIDE any lock
        let manager = self.ssh_manager.clone();
        let sid = session_id.to_string();
        let data = input.to_vec();

        // block_on but we're not holding any lock now
        self.runtime.block_on(async move {
            let mgr = manager.lock().unwrap();
            mgr.write_shell_input(&sid, &data)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to write shell input: {}", e))
        })
    }

    pub fn interrupt_command(&self, session_id: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();

        rt.block_on(async {
            let mgr = manager.lock().unwrap();
            mgr.interrupt_command(session_id)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to interrupt: {}", e))
        })
    }

    pub fn disconnect(&self, session_id: &str) -> anyhow::Result<()> {
        let _ = self.interrupt_command(session_id);

        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();

        rt.block_on(async {
            let mut mgr = manager.lock().unwrap();
            mgr.disconnect(session_id)
                .await
                .map_err(|e| anyhow::anyhow!("Disconnect failed: {}", e))
        })
    }

    /// 初始化SFTP会话（使用独立的SFTP连接）
    /// 注意：这个操作现在是异步的，不会阻塞
    pub fn init_sftp(&self, session_id: &str) {
        let ssh_mgr = self.ssh_manager.clone();
        let sftp_mgr = self.sftp_manager.clone();
        let sid = session_id.to_string();

        // Spawn完全独立的线程，不使用tokio runtime
        std::thread::spawn(move || {
            // 获取SFTP专用session Arc
            let sftp_session_arc = {
                let mgr = ssh_mgr.lock().unwrap();
                match mgr.get_sftp_session_arc(&sid) {
                    Some(s) => s.clone(),
                    None => {
                        error!("SFTP session {} not found", sid);
                        return;
                    }
                }
            };

            // 使用try_lock配合超时，因为blocking_lock在std线程中可能有问题
            let sftp = {
                let mut attempts = 0;
                let session_guard = loop {
                    match sftp_session_arc.try_lock() {
                        Ok(guard) => break guard,
                        Err(_) => {
                            attempts += 1;
                            if attempts > 100 {
                                // 1秒超时
                                error!("SFTP: failed to acquire lock after 1s");
                                return;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                    }
                };

                // 调用sftp()创建SFTP子系统
                // ssh2的Session::sftp()是阻塞调用
                session_guard
                    .sftp()
                    .map_err(|e| anyhow::anyhow!("SFTP subsystem failed: {}", e))
            };

            match sftp {
                Ok(sftp) => {
                    // 存储SFTP会话到manager
                    let mut mgr = sftp_mgr.lock().unwrap();
                    match mgr.create_session(&sid, sftp) {
                        Ok(()) => info!("SFTP session initialized for {}", sid),
                        Err(e) => error!("Failed to store SFTP: {}", e),
                    }
                }
                Err(e) => {
                    error!("SFTP init failed: {}", e);
                }
            }
        });
    }

    /// 检查SFTP是否已初始化
    pub fn is_sftp_initialized(&self, session_id: &str) -> bool {
        let mgr = self.sftp_manager.lock().unwrap();
        mgr.list_sessions().iter().any(|s| s == session_id)
    }

    /// SFTP列出目录
    pub fn sftp_list_dir(&self, session_id: &str, path: &str) -> anyhow::Result<Vec<SftpEntry>> {
        let rt = self.runtime.clone();
        let mgr = self.sftp_manager.clone();
        let sid = session_id.to_string();
        let path_str = path.to_string();

        // 检查SFTP是否已初始化（锁在作用域结束时自动释放）
        let is_initialized = {
            let m = mgr.lock().unwrap();
            m.list_sessions().contains(&sid)
        };

        if !is_initialized {
            return Err(anyhow::anyhow!("SFTP not initialized yet"));
        }

        // 现在锁已释放，可以安全地调用block_on
        rt.block_on(async {
            mgr.lock()
                .unwrap()
                .list_dir(&sid, &path_str)
                .await
                .map_err(|e| anyhow::anyhow!("List dir failed: {}", e))
        })
    }

    /// SFTP创建目录
    pub fn sftp_mkdir(&self, session_id: &str, path: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let mgr = self.sftp_manager.clone();

        rt.block_on(async {
            mgr.lock()
                .unwrap()
                .mkdir(session_id, path, Some(0o755))
                .await
                .map_err(|e| anyhow::anyhow!("Mkdir failed: {}", e))
        })
    }

    /// SFTP删除文件
    pub fn sftp_remove(&self, session_id: &str, path: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let mgr = self.sftp_manager.clone();

        rt.block_on(async {
            mgr.lock()
                .unwrap()
                .remove_file(session_id, path)
                .await
                .map_err(|e| anyhow::anyhow!("Remove file failed: {}", e))
        })
    }

    /// SFTP删除目录
    pub fn sftp_rmdir(&self, session_id: &str, path: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let mgr = self.sftp_manager.clone();

        rt.block_on(async {
            mgr.lock()
                .unwrap()
                .rmdir(session_id, path)
                .await
                .map_err(|e| anyhow::anyhow!("Rmdir failed: {}", e))
        })
    }

    /// SFTP重命名
    pub fn sftp_rename(
        &self,
        session_id: &str,
        old_path: &str,
        new_path: &str,
    ) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let mgr = self.sftp_manager.clone();

        rt.block_on(async {
            mgr.lock()
                .unwrap()
                .rename(session_id, old_path, new_path)
                .await
                .map_err(|e| anyhow::anyhow!("Rename failed: {}", e))
        })
    }

    /// SFTP下载文件
    #[allow(dead_code)]
    pub fn sftp_download(
        &self,
        session_id: &str,
        remote_path: &str,
        local_path: &str,
    ) -> anyhow::Result<Vec<u8>> {
        let rt = self.runtime.clone();
        let mgr = self.sftp_manager.clone();

        rt.block_on(async {
            mgr.lock()
                .unwrap()
                .download(session_id, remote_path, local_path)
                .await
                .map_err(|e| anyhow::anyhow!("Download failed: {}", e))
        })
    }

    /// SFTP上传文件
    #[allow(dead_code)]
    pub fn sftp_upload(
        &self,
        session_id: &str,
        remote_path: &str,
        contents: &[u8],
    ) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let mgr = self.sftp_manager.clone();

        rt.block_on(async {
            mgr.lock()
                .unwrap()
                .upload(session_id, remote_path, contents)
                .await
                .map_err(|e| anyhow::anyhow!("Upload failed: {}", e))
        })
    }

    /// SFTP获取文件信息
    #[allow(dead_code)]
    pub fn sftp_stat(&self, session_id: &str, path: &str) -> anyhow::Result<SftpEntry> {
        let rt = self.runtime.clone();
        let mgr = self.sftp_manager.clone();

        rt.block_on(async {
            mgr.lock()
                .unwrap()
                .stat(session_id, path)
                .await
                .map_err(|e| anyhow::anyhow!("Stat failed: {}", e))
        })
    }

    /// 关闭SFTP会话
    pub fn sftp_close(&self, session_id: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let mgr = self.sftp_manager.clone();

        rt.block_on(async {
            mgr.lock()
                .unwrap()
                .close_session(session_id)
                .await
                .map_err(|e| anyhow::anyhow!("Close SFTP session failed: {}", e))
        })
    }

    pub fn debug_stats(&self) -> DebugStats {
        let servers_count = self.get_servers().len();

        let (active_sessions, mux_stats) = {
            let manager = self.ssh_manager.lock().unwrap();
            let sessions = manager.list_sessions().len();
            let mux = manager.get_pool_stats();
            (sessions, mux)
        };

        DebugStats {
            servers_count,
            active_sessions,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            mux_total_pools: mux_stats.total_pools,
        }
    }

    /// Import configuration from various formats
    pub fn import_config(
        &self,
        content: &str,
        format: ImportFormat,
        conflict_resolution: ConflictResolution,
    ) -> anyhow::Result<ImportResult> {
        let state = self.core_state.lock().unwrap();
        let db_lock = state.db.lock().unwrap();
        let db = db_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        match format {
            ImportFormat::Json => ConfigManager::import_json(db, content, conflict_resolution)
                .map_err(|e| anyhow::anyhow!("Import failed: {}", e)),
            ImportFormat::JsonEncrypted => {
                // Need password for encrypted - this should be handled by the UI
                Err(anyhow::anyhow!(
                    "Encrypted import requires password. Use import_json_encrypted method."
                ))
            }
            ImportFormat::Csv => ConfigManager::import_csv(db, content, conflict_resolution)
                .map_err(|e| anyhow::anyhow!("Import failed: {}", e)),
            ImportFormat::SshConfig => {
                ConfigManager::import_ssh_config(db, content, conflict_resolution)
                    .map_err(|e| anyhow::anyhow!("Import failed: {}", e))
            }
            ImportFormat::AutoDetect => {
                // Try JSON first
                if content.trim().starts_with('{') {
                    ConfigManager::import_json(db, content, conflict_resolution)
                        .map_err(|e| anyhow::anyhow!("Import failed: {}", e))
                } else if content
                    .lines()
                    .next()
                    .map(|l| l.contains(','))
                    .unwrap_or(false)
                {
                    // Likely CSV
                    ConfigManager::import_csv(db, content, conflict_resolution)
                        .map_err(|e| anyhow::anyhow!("Import failed: {}", e))
                } else {
                    // Try SSH config
                    ConfigManager::import_ssh_config(db, content, conflict_resolution)
                        .map_err(|e| anyhow::anyhow!("Import failed: {}", e))
                }
            }
        }
    }

    /// Import from encrypted JSON with password
    pub fn import_config_encrypted(
        &self,
        content: &str,
        password: &str,
        conflict_resolution: ConflictResolution,
    ) -> anyhow::Result<ImportResult> {
        let state = self.core_state.lock().unwrap();
        let db_lock = state.db.lock().unwrap();
        let db = db_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        ConfigManager::import_json_encrypted(db, content, password, conflict_resolution)
            .map_err(|e| anyhow::anyhow!("Encrypted import failed: {}", e))
    }

    /// Export configuration to various formats
    pub fn export_config(
        &self,
        format: ExportFormat,
        password: &str,
        include_secrets: bool,
    ) -> anyhow::Result<String> {
        let state = self.core_state.lock().unwrap();
        let db_lock = state.db.lock().unwrap();
        let db = db_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        match format {
            ExportFormat::Json => ConfigManager::export_json(db, include_secrets)
                .map_err(|e| anyhow::anyhow!("Export failed: {}", e)),
            ExportFormat::JsonEncrypted => {
                if password.is_empty() {
                    return Err(anyhow::anyhow!("Password required for encrypted export"));
                }
                ConfigManager::export_json_encrypted(db, password, include_secrets)
                    .map_err(|e| anyhow::anyhow!("Encrypted export failed: {}", e))
            }
            ExportFormat::Csv => {
                ConfigManager::export_csv(db).map_err(|e| anyhow::anyhow!("Export failed: {}", e))
            }
            ExportFormat::SshConfig => ConfigManager::export_ssh_config(db)
                .map_err(|e| anyhow::anyhow!("Export failed: {}", e)),
        }
    }

    /// Save accessibility settings to database
    pub fn save_accessibility_settings(
        &self,
        high_contrast: bool,
        reduced_motion: bool,
        large_text: bool,
    ) -> anyhow::Result<()> {
        let state = self.core_state.lock().unwrap();
        let db_lock = state.db.lock().unwrap();
        let db = db_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        db.set_config("accessibility.high_contrast", &high_contrast.to_string())
            .map_err(|e| anyhow::anyhow!("Failed to save high_contrast: {}", e))?;
        db.set_config("accessibility.reduced_motion", &reduced_motion.to_string())
            .map_err(|e| anyhow::anyhow!("Failed to save reduced_motion: {}", e))?;
        db.set_config("accessibility.large_text", &large_text.to_string())
            .map_err(|e| anyhow::anyhow!("Failed to save large_text: {}", e))?;

        info!(
            "Accessibility settings saved: high_contrast={}, reduced_motion={}, large_text={}",
            high_contrast, reduced_motion, large_text
        );
        Ok(())
    }

    /// Load accessibility settings from database
    pub fn load_accessibility_settings(&self) -> anyhow::Result<(bool, bool, bool)> {
        let state = self.core_state.lock().unwrap();
        let db_lock = state.db.lock().unwrap();
        let db = db_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        let high_contrast = db
            .get_config("accessibility.high_contrast")?
            .map(|v| v.parse::<bool>().unwrap_or(false))
            .unwrap_or(false);
        let reduced_motion = db
            .get_config("accessibility.reduced_motion")?
            .map(|v| v.parse::<bool>().unwrap_or(false))
            .unwrap_or(false);
        let large_text = db
            .get_config("accessibility.large_text")?
            .map(|v| v.parse::<bool>().unwrap_or(false))
            .unwrap_or(false);

        info!(
            "Accessibility settings loaded: high_contrast={}, reduced_motion={}, large_text={}",
            high_contrast, reduced_motion, large_text
        );
        Ok((high_contrast, reduced_motion, large_text))
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ServerViewModel {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub group_id: Option<String>,
    pub auth_type: String,
}

impl From<ServerRecord> for ServerViewModel {
    fn from(s: ServerRecord) -> Self {
        Self {
            id: s.id,
            name: s.name,
            host: s.host,
            port: s.port,
            username: s.username,
            group_id: s.group_id,
            auth_type: s.auth_type,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct GroupViewModel {
    pub id: String,
    pub name: String,
}

impl From<GroupRecord> for GroupViewModel {
    fn from(g: GroupRecord) -> Self {
        Self {
            id: g.id,
            name: g.name,
        }
    }
}
