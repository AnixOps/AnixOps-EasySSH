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
    FileInfo, FilePermission, FileType, TransferDirection, TransferOptions, TransferResult,
    TransferStats, TransferStatus, TransferTask,
};

pub use client::{ConnectionState, SftpClient, SftpClientConfig};

pub use transfer::{ChunkConfig, FileTransfer, TransferError, TransferHandle};

pub use remote_fs::{ContentType, FileSystemWatcher, RemoteDir, RemoteFile, RemoteFs};

pub use queue::{QueueConfig, QueueEvent, QueueStats, TransferQueue};

pub use progress::{ProgressCallback, ProgressSnapshot, ProgressTracker, SpeedCalculator};

use crate::error::LiteError;
use std::sync::Arc;
use tokio::sync::RwLock;

/// SFTP管理器 - 统一入口
///
/// 整合所有SFTP功能，提供高级文件管理能力
pub struct SftpManager {
    /// 客户端管理器
    clients: Arc<RwLock<client::ClientPool>>,
    /// 传输队列
    queue: Arc<RwLock<TransferQueue>>,
    /// 进度追踪器
    progress: Arc<RwLock<ProgressTracker>>,
}

impl SftpManager {
    /// 创建新的SFTP管理器
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(client::ClientPool::new())),
            queue: Arc::new(RwLock::new(TransferQueue::new())),
            progress: Arc::new(RwLock::new(ProgressTracker::new())),
        }
    }

    /// 添加SFTP客户端
    pub async fn add_client(&self, client: SftpClient) -> String {
        let mut clients = self.clients.write().await;
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
