//! SFTP文件管理器模块
//!
//! 提供安全的文件传输功能，支持:
//! - 远程文件系统浏览
//! - 文件上传/下载
//! - 断点续传
//! - 批量传输队列
//! - 实时进度追踪
//!
//! # 模块结构
//!
//! - `types`: 核心数据类型 (FileInfo, TransferTask, 等)
//! - `client`: SFTP客户端连接管理
//! - `transfer`: 文件传输实现
//! - `remote_fs`: 远程文件系统操作
//! - `queue`: 传输队列管理
//! - `progress`: 进度追踪

pub mod client;
pub mod progress;
pub mod queue;
pub mod remote_fs;
pub mod transfer;
pub mod types;

// 公共导出
pub use types::{
    FileInfo, FilePermission, FileType, SftpEntry, TransferDirection, TransferOptions,
    TransferResult, TransferStats, TransferStatus, TransferTask,
};

pub use client::{ConnectionState, SftpClient, SftpClientConfig};

pub use transfer::{ChunkConfig, FileTransfer, TransferError, TransferHandle};

pub use remote_fs::{ContentType, FileSystemWatcher, RemoteDir, RemoteFile, RemoteFs};

pub use queue::{QueueConfig, QueueEvent, QueueStats, TransferQueue};

pub use progress::{ProgressCallback, ProgressSnapshot, ProgressTracker, SpeedCalculator};

use crate::error::LiteError;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::sync::Mutex;

/// SFTP管理器 - 统一入口
///
/// 整合所有SFTP功能，提供高级文件管理能力
pub struct SftpManager {
    /// 客户端管理器 (使用Mutex以支持同步访问)
    clients: Arc<Mutex<client::ClientPool>>,
    /// 传输队列
    queue: Arc<RwLock<TransferQueue>>,
    /// 进度追踪器
    progress: Arc<RwLock<ProgressTracker>>,
}

impl SftpManager {
    /// 创建新的SFTP管理器
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(client::ClientPool::new())),
            queue: Arc::new(RwLock::new(TransferQueue::new())),
            progress: Arc::new(RwLock::new(ProgressTracker::new())),
        }
    }

    /// 添加SFTP客户端
    pub async fn add_client(&self, client: SftpClient) -> String {
        let mut clients = self.clients.lock().unwrap();
        clients.add(client).await
    }

    /// 获取传输队列
    pub fn queue(&self) -> Arc<RwLock<TransferQueue>> {
        self.queue.clone()
    }

    /// 获取进度追踪器
    pub fn progress_tracker(&self) -> Arc<RwLock<ProgressTracker>> {
        self.progress.clone()
    }

    /// 获取远程文件系统操作接口
    pub async fn remote_fs(&self, client_id: &str) -> Result<RemoteFs, LiteError> {
        let clients = self.clients.read().await;
        let client = clients.get(client_id).await?;
        Ok(RemoteFs::new(client))
    }

    /// 获取文件传输器
    pub async fn file_transfer(&self, client_id: &str) -> Result<FileTransfer, LiteError> {
        let clients = self.clients.read().await;
        let client = clients.get(client_id).await?;
        let queue = self.queue.clone();
        let progress = self.progress.clone();
        Ok(FileTransfer::new(client, queue, progress))
    }

    /// 关闭所有连接
    pub async fn shutdown(&self) -> Result<(), LiteError> {
        let mut clients = self.clients.write().await;
        clients.close_all().await;
        Ok(())
    }

    /// 重命名文件或目录
    pub async fn rename(
        &self,
        session_id: &str,
        old_path: &str,
        new_path: &str,
    ) -> Result<(), LiteError> {
        let remote_fs = self.remote_fs(session_id).await?;
        remote_fs.rename(old_path, new_path).await
    }

    /// 删除目录
    pub async fn rmdir(&self, session_id: &str, path: &str) -> Result<(), LiteError> {
        let remote_fs = self.remote_fs(session_id).await?;
        remote_fs.rmdir(path).await
    }

    /// 获取文件信息
    pub async fn stat(&self, session_id: &str, path: &str) -> Result<SftpEntry, LiteError> {
        let remote_fs = self.remote_fs(session_id).await?;
        let info = remote_fs.stat(path).await?;
        Ok(SftpEntry::from(info))
    }

    /// 下载文件
    pub async fn download(
        &self,
        session_id: &str,
        remote_path: &str,
        local_path: &str,
    ) -> Result<Vec<u8>, LiteError> {
        let file_transfer = self.file_transfer(session_id).await?;
        file_transfer
            .download(remote_path, local_path, None, None)
            .await
            .map_err(|e| LiteError::Io(format!("下载失败: {}", e)))?;
        // 读取下载的文件
        let data = tokio::fs::read(local_path)
            .await
            .map_err(|e| LiteError::Io(format!("读取下载文件失败: {}", e)))?;
        Ok(data)
    }

    /// 上传文件
    pub async fn upload(
        &self,
        session_id: &str,
        remote_path: &str,
        contents: &[u8],
    ) -> Result<(), LiteError> {
        let file_transfer = self.file_transfer(session_id).await?;
        // 先写入临时文件
        let temp_path =
            std::env::temp_dir().join(format!("sftp_upload_{}", uuid::Uuid::new_v4()));
        tokio::fs::write(&temp_path, contents)
            .await
            .map_err(|e| LiteError::Io(format!("写入临时文件失败: {}", e)))?;
        file_transfer
            .upload(temp_path.to_str().unwrap(), remote_path, None, None)
            .await
            .map_err(|e| LiteError::Io(format!("上传失败: {}", e)))?;
        // 删除临时文件
        let _ = tokio::fs::remove_file(&temp_path).await;
        Ok(())
    }

    /// 关闭会话
    pub async fn close_session(&self, session_id: &str) -> Result<(), LiteError> {
        let mut clients = self.clients.write().await;
        clients.remove(session_id).await;
        Ok(())
    }

    /// 创建SFTP会话 (添加客户端)
    pub async fn create_session(&self, session_id: &str, client: SftpClient) -> Result<(), LiteError> {
        let mut clients = self.clients.write().await;
        client::ClientPool::insert(&mut *clients, session_id, client);
        Ok(())
    }

    /// 列出所有会话ID
    pub async fn list_sessions(&self) -> Vec<String> {
        let clients = self.clients.read().await;
        clients.keys()
    }

    /// 列出目录
    pub async fn list_dir(&self, session_id: &str, path: &str) -> Result<Vec<SftpEntry>, LiteError> {
        let remote_fs = self.remote_fs(session_id).await?;
        let infos = remote_fs.list_dir(path).await?;
        Ok(infos.into_iter().map(SftpEntry::from).collect())
    }

    /// 创建目录
    pub async fn mkdir(&self, session_id: &str, path: &str, _mode: Option<i32>) -> Result<(), LiteError> {
        let remote_fs = self.remote_fs(session_id).await?;
        remote_fs.mkdir(path, _mode.unwrap_or(0o755) as u32).await
    }

    /// 删除文件
    pub async fn remove_file(&self, session_id: &str, path: &str) -> Result<(), LiteError> {
        let remote_fs = self.remote_fs(session_id).await?;
        remote_fs.remove_file(path).await
    }
}

impl Default for SftpManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 客户端连接池
mod client_pool {
    use super::*;
    use std::collections::HashMap;

    pub struct ClientPool {
        clients: HashMap<String, Arc<RwLock<SftpClient>>>,
    }

    impl ClientPool {
        pub fn new() -> Self {
            Self {
                clients: HashMap::new(),
            }
        }

        pub async fn add(&mut self, client: SftpClient) -> String {
            let id = uuid::Uuid::new_v4().to_string();
            self.clients
                .insert(id.clone(), Arc::new(RwLock::new(client)));
            id
        }

        pub fn insert(&mut self, id: &str, client: SftpClient) {
            self.clients.insert(
                id.to_string(),
                Arc::new(RwLock::new(client)),
            );
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
                let _ = self.remove(&id).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sftp_manager_new() {
        let manager = SftpManager::new();
        let queue_arc = manager.queue();
        let queue = queue_arc.read().await;
        let stats = queue.stats().await;
        assert_eq!(stats.pending, 0);
    }

    #[test]
    fn test_sftp_manager_default() {
        let _manager: SftpManager = Default::default();
        assert!(true);
    }
}
