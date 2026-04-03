//! 文件传输模块

use crate::error::LiteError;
use crate::sftp::client::SftpClient;
use crate::sftp::progress::{ProgressCallback, ProgressTracker};
use crate::sftp::queue::TransferQueue;
use crate::sftp::types::{TransferDirection, TransferOptions, TransferResult, TransferTask};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// 文件传输器
#[derive(Debug, Clone)]
pub struct FileTransfer {
    client: Arc<RwLock<SftpClient>>,
    queue: Arc<RwLock<TransferQueue>>,
    progress: Arc<RwLock<ProgressTracker>>,
    default_options: TransferOptions,
}

impl FileTransfer {
    pub fn new(
        client: Arc<RwLock<SftpClient>>,
        queue: Arc<RwLock<TransferQueue>>,
        progress: Arc<RwLock<ProgressTracker>>,
    ) -> Self {
        Self {
            client,
            queue,
            progress,
            default_options: TransferOptions::default(),
        }
    }

    pub fn with_default_options(mut self, options: TransferOptions) -> Self {
        self.default_options = options;
        self
    }

    pub async fn download(
        &self,
        _remote_path: impl AsRef<Path>,
        _local_path: impl AsRef<Path>,
        _options: Option<TransferOptions>,
        _callback: Option<ProgressCallback>,
    ) -> Result<TransferResult, TransferError> {
        // 简化实现
        Ok(TransferResult::new("", 0, Duration::ZERO))
    }

    pub async fn upload(
        &self,
        _local_path: impl AsRef<Path>,
        _remote_path: impl AsRef<Path>,
        _options: Option<TransferOptions>,
        _callback: Option<ProgressCallback>,
    ) -> Result<TransferResult, TransferError> {
        // 简化实现
        Ok(TransferResult::new("", 0, Duration::ZERO))
    }

    pub async fn queue_download(
        &self,
        remote_path: impl Into<PathBuf>,
        local_path: impl Into<PathBuf>,
        options: Option<TransferOptions>,
    ) -> Result<String, LiteError> {
        let options = options.unwrap_or(self.default_options.clone());
        let client_id = self.client.read().await.id().to_string();

        let task = TransferTask::new(
            remote_path.into(),
            local_path.into(),
            TransferDirection::Download,
            &client_id,
        )
        .with_options(options);

        let queue = self.queue.write().await;
        let id = queue.add(task).await;
        Ok(id)
    }

    pub async fn queue_upload(
        &self,
        local_path: impl Into<PathBuf>,
        remote_path: impl Into<PathBuf>,
        options: Option<TransferOptions>,
    ) -> Result<String, LiteError> {
        let options = options.unwrap_or(self.default_options.clone());
        let client_id = self.client.read().await.id().to_string();

        let task = TransferTask::new(
            local_path.into(),
            remote_path.into(),
            TransferDirection::Upload,
            &client_id,
        )
        .with_options(options);

        let queue = self.queue.write().await;
        let id = queue.add(task).await;
        Ok(id)
    }
}

/// 传输错误
#[derive(Debug, Clone)]
pub enum TransferError {
    LocalIo(String),
    RemoteFileError(String),
    Cancelled,
    SpeedLimitTimeout,
    TaskFailed(String),
    VerificationFailed(String),
}

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferError::LocalIo(msg) => write!(f, "本地IO错误: {}", msg),
            TransferError::RemoteFileError(msg) => write!(f, "远程文件错误: {}", msg),
            TransferError::Cancelled => write!(f, "传输已取消"),
            TransferError::SpeedLimitTimeout => write!(f, "速度限制等待超时"),
            TransferError::TaskFailed(msg) => write!(f, "任务执行失败: {}", msg),
            TransferError::VerificationFailed(msg) => write!(f, "验证失败: {}", msg),
        }
    }
}

impl std::error::Error for TransferError {}

/// 传输句柄
pub struct TransferHandle {
    pub task_id: String,
}

/// 块配置
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    pub size: usize,
    pub parallel: usize,
    pub retries: u32,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            size: 64 * 1024,
            parallel: 3,
            retries: 3,
        }
    }
}
