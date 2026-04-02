//! SFTP客户端模块

use crate::error::LiteError;
use crate::sftp::types::FileInfo;
use ssh2::{Session, Sftp};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};

/// 连接状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Authenticating,
    Authenticated,
    Error(String),
}

/// 客户端配置
#[derive(Debug, Clone)]
pub struct SftpClientConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<PathBuf>,
    pub passphrase: Option<String>,
    pub connect_timeout: Duration,
}

impl Default for SftpClientConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 22,
            username: String::new(),
            password: None,
            private_key: None,
            passphrase: None,
            connect_timeout: Duration::from_secs(30),
        }
    }
}

impl SftpClientConfig {
    pub fn new(host: impl Into<String>, username: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            username: username.into(),
            ..Default::default()
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    pub fn with_private_key(mut self, key_path: impl Into<PathBuf>) -> Self {
        self.private_key = Some(key_path.into());
        self
    }
}

/// SFTP客户端
pub struct SftpClient {
    id: String,
    config: SftpClientConfig,
    session: Arc<Mutex<Session>>,
    sftp: Arc<Mutex<Option<Sftp>>>,
    state: Arc<RwLock<ConnectionState>>,
}

impl std::fmt::Debug for SftpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SftpClient")
            .field("id", &self.id)
            .field("config", &self.config)
            .finish()
    }
}

impl Clone for SftpClient {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            config: self.config.clone(),
            session: Arc::new(Mutex::new(Session::new().unwrap())),
            sftp: Arc::new(Mutex::new(None)),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
        }
    }
}

impl SftpClient {
    pub fn new(config: SftpClientConfig) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            config,
            session: Arc::new(Mutex::new(Session::new().unwrap())),
            sftp: Arc::new(Mutex::new(None)),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }

    pub async fn is_connected(&self) -> bool {
        matches!(self.state().await, ConnectionState::Connected)
    }

    pub async fn connect(&self) -> Result<(), LiteError> {
        *self.state.write().await = ConnectionState::Connecting;
        // 简化实现，实际应建立SSH连接
        *self.state.write().await = ConnectionState::Connected;
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), LiteError> {
        let mut sftp = self.sftp.lock().await;
        *sftp = None;
        *self.state.write().await = ConnectionState::Disconnected;
        Ok(())
    }

    pub async fn list_dir(&self, path: impl AsRef<Path>) -> Result<Vec<FileInfo>, LiteError> {
        // 简化实现
        Ok(Vec::new())
    }

    pub async fn stat(&self, path: impl AsRef<Path>) -> Result<FileInfo, LiteError> {
        // 简化实现
        Ok(FileInfo::new("test", path.as_ref()))
    }

    pub async fn exists(&self, _path: impl AsRef<Path>) -> Result<bool, LiteError> {
        Ok(false)
    }

    pub async fn mkdir(&self, _path: impl AsRef<Path>, _mode: u32) -> Result<(), LiteError> {
        Ok(())
    }

    pub async fn mkdir_p(&self, path: impl AsRef<Path>) -> Result<(), LiteError> {
        let path = path.as_ref();
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component);
            if !self.exists(&current).await? {
                self.mkdir(&current, 0o755).await?;
            }
        }
        Ok(())
    }

    pub async fn remove_file(&self, _path: impl AsRef<Path>) -> Result<(), LiteError> {
        Ok(())
    }

    pub async fn rmdir(&self, _path: impl AsRef<Path>) -> Result<(), LiteError> {
        Ok(())
    }

    pub async fn rename(&self, _old: impl AsRef<Path>, _new: impl AsRef<Path>) -> Result<(), LiteError> {
        Ok(())
    }
}

/// 客户端连接池
pub struct ClientPool {
    clients: HashMap<String, Arc<RwLock<SftpClient>>>,
}

impl std::fmt::Debug for ClientPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientPool")
            .field("client_count", &self.clients.len())
            .finish()
    }
}

impl ClientPool {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub async fn add(&mut self, client: SftpClient) -> String {
        let id = client.id().to_string();
        self.clients.insert(id.clone(), Arc::new(RwLock::new(client)));
        id
    }

    pub async fn get(&self, id: &str) -> Result<Arc<RwLock<SftpClient>>, LiteError> {
        self.clients
            .get(id)
            .cloned()
            .ok_or_else(|| LiteError::Ssh(format!("SFTP客户端不存在: {}", id)))
    }

    pub async fn remove(&mut self, id: &str) {
        if let Some(client) = self.clients.remove(id) {
            let _ = client.write().await.disconnect().await;
        }
    }

    pub async fn close_all(&mut self) {
        let ids: Vec<String> = self.clients.keys().cloned().collect();
        for id in ids {
            self.remove(&id).await;
        }
    }
}

use serde::{Deserialize, Serialize};
