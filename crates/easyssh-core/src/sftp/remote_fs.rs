//! 远程文件系统模块

use crate::error::LiteError;
use crate::sftp::client::SftpClient;
use crate::sftp::types::FileInfo;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// 远程文件系统
#[derive(Debug, Clone)]
pub struct RemoteFs {
    client: Arc<RwLock<SftpClient>>,
    cache: Arc<RwLock<HashMap<PathBuf, (Vec<FileInfo>, tokio::time::Instant)>>>,
    cache_ttl: Duration,
}

impl RemoteFs {
    pub fn new(client: Arc<RwLock<SftpClient>>) -> Self {
        Self {
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(30),
        }
    }

    pub async fn list_dir(&self, path: impl AsRef<Path>) -> Result<Vec<FileInfo>, LiteError> {
        let client = self.client.read().await;
        client.list_dir(path).await
    }

    pub async fn stat(&self, path: impl AsRef<Path>) -> Result<FileInfo, LiteError> {
        let client = self.client.read().await;
        client.stat(path).await
    }

    pub async fn exists(&self, path: impl AsRef<Path>) -> Result<bool, LiteError> {
        let client = self.client.read().await;
        client.exists(path).await
    }

    pub async fn mkdir(&self, path: impl AsRef<Path>, mode: u32) -> Result<(), LiteError> {
        let client = self.client.read().await;
        client.mkdir(path, mode).await
    }

    pub async fn mkdir_p(&self, path: impl AsRef<Path>) -> Result<(), LiteError> {
        let client = self.client.read().await;
        client.mkdir_p(path).await
    }

    pub async fn remove_file(&self, path: impl AsRef<Path>) -> Result<(), LiteError> {
        let client = self.client.read().await;
        client.remove_file(path).await
    }

    pub async fn rmdir(&self, path: impl AsRef<Path>) -> Result<(), LiteError> {
        let client = self.client.read().await;
        client.rmdir(path).await
    }

    pub async fn rename(
        &self,
        old: impl AsRef<Path>,
        new: impl AsRef<Path>,
    ) -> Result<(), LiteError> {
        let client = self.client.read().await;
        client.rename(old, new).await
    }
}

/// 远程目录
#[derive(Debug, Clone)]
pub struct RemoteDir {
    pub info: FileInfo,
    pub children: Vec<RemoteEntry>,
}

/// 远程条目
#[derive(Debug, Clone)]
pub enum RemoteEntry {
    File(RemoteFile),
    Directory(RemoteDir),
}

/// 远程文件
#[derive(Debug, Clone)]
pub struct RemoteFile {
    pub info: FileInfo,
    pub content_type: ContentType,
}

/// 内容类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Text,
    Binary,
    Image,
    Video,
    Audio,
    Archive,
    Document,
    Code,
    Unknown,
}

/// 文件系统监视器
pub struct FileSystemWatcher {
    watched_paths: Arc<RwLock<Vec<PathBuf>>>,
    interval: Duration,
}

impl FileSystemWatcher {
    pub fn new(interval: Duration) -> Self {
        Self {
            watched_paths: Arc::new(RwLock::new(Vec::new())),
            interval,
        }
    }

    pub async fn watch(&self, path: impl AsRef<Path>) {
        let mut paths = self.watched_paths.write().await;
        let path = path.as_ref().to_path_buf();
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
}

/// 目录统计
#[derive(Debug, Clone)]
pub struct DirStats {
    pub total_entries: usize,
    pub files: usize,
    pub directories: usize,
    pub total_size: u64,
}

/// 文件变更
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub change_type: ChangeType,
    pub file_info: Option<FileInfo>,
}

/// 变更类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Renamed(PathBuf),
}
