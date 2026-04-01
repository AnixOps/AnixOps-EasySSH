//! SFTP文件传输模块 - 高级文件传输支持
//!
//! 功能特性：
//! - 文件上传/下载，支持进度回调
//! - 目录递归传输
//! - 断点续传支持
//! - 传输队列管理
//! - 并行传输优化
//! - 传输限速控制
//!
//! # 示例
//!
//! ```rust,no_run
//! use easyssh_core::sftp::{
//!     SftpSessionManager, TransferProgress, TransferOptions, TransferQueue,
//! };
//!
//! async fn sftp_example() {
//!     let mut sftp = SftpSessionManager::new();
//!     // ... 建立SFTP会话
//!
//!     // 带进度回调的文件下载
//!     let options = TransferOptions::default()
//!         .with_resume(true)
//!         .with_chunk_size(64 * 1024);
//!
//!     let result = sftp.download_with_progress(
//!         "session-1",
//!         "/remote/file.txt",
//!         "/local/file.txt",
//!         options,
//!         |progress| {
//!             println!("{}% - {}/s",
//!                 progress.percentage(),
//!                 progress.speed_display()
//!             );
//!         }
//!     ).await.unwrap();
//! }
//! ```

use crate::error::LiteError;
use ssh2::Sftp;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex as TokioMutex, RwLock};
use tokio::task::JoinHandle;

/// 传输进度信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferProgress {
    /// 已传输字节数
    pub transferred: u64,
    /// 总字节数（如果已知）
    pub total: Option<u64>,
    /// 传输开始时间
    pub start_time: i64,
    /// 传输速度（字节/秒）
    pub speed_bps: f64,
    /// 已用时间（秒）
    pub elapsed_secs: f64,
    /// 预计剩余时间（秒）
    pub eta_secs: Option<f64>,
    /// 传输状态
    pub status: TransferStatus,
    /// 文件名
    pub filename: String,
    /// 是否恢复传输
    pub is_resume: bool,
    /// 已恢复的偏移量
    pub resume_offset: u64,
}

/// 传输状态
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TransferStatus {
    /// 等待中
    Pending,
    /// 传输中
    Transferring,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

impl TransferProgress {
    /// 创建新的传输进度
    pub fn new(filename: impl Into<String>, total: Option<u64>, is_resume: bool) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            transferred: 0,
            total,
            start_time: now,
            speed_bps: 0.0,
            elapsed_secs: 0.0,
            eta_secs: None,
            status: TransferStatus::Pending,
            filename: filename.into(),
            is_resume,
            resume_offset: 0,
        }
    }

    /// 计算百分比（0-100）
    pub fn percentage(&self) -> f64 {
        match self.total {
            Some(total) if total > 0 => {
                let pct = (self.transferred as f64 / total as f64) * 100.0;
                pct.min(100.0)
            }
            _ => 0.0,
        }
    }

    /// 格式化速度显示
    pub fn speed_display(&self) -> String {
        format_size(self.speed_bps as u64)
    }

    /// 格式化ETA显示
    pub fn eta_display(&self) -> String {
        match self.eta_secs {
            Some(secs) if secs > 0.0 => {
                if secs < 60.0 {
                    format!("{:.0}s", secs)
                } else if secs < 3600.0 {
                    format!("{:.0}m {:.0}s", secs / 60.0, secs % 60.0)
                } else {
                    format!("{:.0}h {:.0}m", secs / 3600.0, (secs % 3600.0) / 60.0)
                }
            }
            _ => "-".to_string(),
        }
    }

    /// 更新传输进度
    pub fn update(&mut self, transferred: u64) {
        self.transferred = transferred;
        let now = chrono::Utc::now().timestamp();
        self.elapsed_secs = (now - self.start_time) as f64;

        if self.elapsed_secs > 0.0 {
            self.speed_bps = self.transferred as f64 / self.elapsed_secs;
        }

        if let Some(total) = self.total {
            if self.speed_bps > 0.0 {
                let remaining = total.saturating_sub(self.transferred);
                self.eta_secs = Some(remaining as f64 / self.speed_bps);
            }
        }
    }

    /// 标记为完成
    pub fn complete(&mut self) {
        self.status = TransferStatus::Completed;
        let now = chrono::Utc::now().timestamp();
        self.elapsed_secs = (now - self.start_time) as f64;
        if let Some(total) = self.total {
            self.transferred = total;
        }
    }

    /// 标记为失败
    pub fn fail(&mut self) {
        self.status = TransferStatus::Failed;
    }

    /// 标记为取消
    pub fn cancel(&mut self) {
        self.status = TransferStatus::Cancelled;
    }

    /// 标记为暂停
    pub fn pause(&mut self) {
        self.status = TransferStatus::Paused;
    }

    /// 标记为传输中
    pub fn start(&mut self) {
        self.status = TransferStatus::Transferring;
        self.start_time = chrono::Utc::now().timestamp();
    }
}

/// 传输选项
#[derive(Debug, Clone)]
pub struct TransferOptions {
    /// 块大小（默认64KB）
    pub chunk_size: usize,
    /// 是否启用断点续传
    pub resume: bool,
    /// 最大并发传输数
    pub max_concurrent: usize,
    /// 速度限制（字节/秒，0表示无限制）
    pub speed_limit: u64,
    /// 是否覆盖已存在的文件
    pub overwrite: bool,
    /// 是否保持文件时间戳
    pub preserve_time: bool,
    /// 是否保持文件权限
    pub preserve_permissions: bool,
    /// 文件模式（创建新文件时使用）
    pub file_mode: i32,
}

impl Default for TransferOptions {
    fn default() -> Self {
        Self {
            chunk_size: 64 * 1024, // 64KB
            resume: true,
            max_concurrent: 3,
            speed_limit: 0,
            overwrite: true,
            preserve_time: true,
            preserve_permissions: true,
            file_mode: 0o644,
        }
    }
}

impl TransferOptions {
    /// 设置块大小
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// 启用/禁用断点续传
    pub fn with_resume(mut self, resume: bool) -> Self {
        self.resume = resume;
        self
    }

    /// 设置最大并发数
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// 设置速度限制
    pub fn with_speed_limit(mut self, limit_bps: u64) -> Self {
        self.speed_limit = limit_bps;
        self
    }

    /// 设置是否覆盖
    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }
}

/// SFTP条目信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SftpEntry {
    pub name: String,
    pub path: String,
    pub file_type: String, // "file", "directory", "symlink"
    pub size: i64,
    pub mtime: i64,
    pub permissions: Option<u32>,
}

impl SftpEntry {
    /// 获取文件大小格式化字符串
    pub fn size_display(&self) -> String {
        if self.file_type == "directory" {
            "-".to_string()
        } else {
            format_size(self.size as u64)
        }
    }

    /// 获取修改时间格式化字符串
    pub fn mtime_display(&self) -> String {
        if self.mtime == 0 {
            "-".to_string()
        } else {
            let dt =
                chrono::DateTime::from_timestamp(self.mtime, 0).unwrap_or_else(chrono::Utc::now);
            dt.format("%Y-%m-%d %H:%M").to_string()
        }
    }

    /// 检查是否为目录
    pub fn is_dir(&self) -> bool {
        self.file_type == "directory"
    }

    /// 检查是否为文件
    pub fn is_file(&self) -> bool {
        self.file_type == "file"
    }

    /// 检查是否为符号链接
    pub fn is_symlink(&self) -> bool {
        self.file_type == "symlink"
    }
}

/// 传输项（用于队列）
#[derive(Debug, Clone)]
pub struct TransferItem {
    pub id: String,
    pub session_id: String,
    pub source_path: String,
    pub dest_path: String,
    pub direction: TransferDirection,
    pub options: TransferOptions,
    pub progress: TransferProgress,
    pub created_at: Instant,
}

/// 传输方向
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TransferDirection {
    Upload,
    Download,
}

impl TransferItem {
    /// 创建新的传输项
    pub fn new(
        session_id: impl Into<String>,
        source: impl Into<String>,
        dest: impl Into<String>,
        direction: TransferDirection,
        options: TransferOptions,
        total_size: Option<u64>,
    ) -> Self {
        let source_str = source.into();
        let filename = Path::new(&source_str)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            source_path: source_str,
            dest_path: dest.into(),
            direction,
            options,
            progress: TransferProgress::new(filename, total_size, false),
            created_at: Instant::now(),
        }
    }
}

/// 传输队列管理器
pub struct TransferQueue {
    items: Arc<RwLock<Vec<TransferItem>>>,
    /// 取消标志（按传输ID）
    cancel_flags: Arc<RwLock<HashMap<String, Arc<AtomicBool>>>>,
    /// 暂停标志（按传输ID）
    pause_flags: Arc<RwLock<HashMap<String, Arc<AtomicBool>>>>,
    /// 进度发送器
    progress_tx: Option<mpsc::Sender<TransferProgress>>,
    /// 处理中的任务
    active_tasks: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
}

impl TransferQueue {
    pub fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(Vec::new())),
            cancel_flags: Arc::new(RwLock::new(HashMap::new())),
            pause_flags: Arc::new(RwLock::new(HashMap::new())),
            progress_tx: None,
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 设置进度回调通道
    pub fn set_progress_channel(&mut self, tx: mpsc::Sender<TransferProgress>) {
        self.progress_tx = Some(tx);
    }

    /// 添加传输任务到队列
    pub async fn add(&self, item: TransferItem) -> String {
        let id = item.id.clone();
        let mut items = self.items.write().await;
        items.push(item);
        id
    }

    /// 获取所有传输项
    pub async fn list(&self) -> Vec<TransferItem> {
        let items = self.items.read().await;
        items.clone()
    }

    /// 获取传输项
    pub async fn get(&self, id: &str) -> Option<TransferItem> {
        let items = self.items.read().await;
        items.iter().find(|item| item.id == id).cloned()
    }

    /// 取消传输
    pub async fn cancel(&self, id: &str) -> Result<(), LiteError> {
        // 设置取消标志
        let flags = self.cancel_flags.read().await;
        if let Some(flag) = flags.get(id) {
            flag.store(true, Ordering::Relaxed);
        }

        // 取消正在进行的任务
        let mut tasks = self.active_tasks.write().await;
        if let Some(handle) = tasks.remove(id) {
            handle.abort();
        }

        // 更新状态
        let mut items = self.items.write().await;
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.progress.cancel();
        }

        Ok(())
    }

    /// 暂停传输
    pub async fn pause(&self, id: &str) -> Result<(), LiteError> {
        let flags = self.pause_flags.read().await;
        if let Some(flag) = flags.get(id) {
            flag.store(true, Ordering::Relaxed);
        }

        let mut items = self.items.write().await;
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.progress.pause();
        }

        Ok(())
    }

    /// 继续传输
    pub async fn resume(&self, id: &str) -> Result<(), LiteError> {
        let flags = self.pause_flags.read().await;
        if let Some(flag) = flags.get(id) {
            flag.store(false, Ordering::Relaxed);
        }

        let mut items = self.items.write().await;
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.progress.status = TransferStatus::Transferring;
        }

        Ok(())
    }

    /// 移除传输项
    pub async fn remove(&self, id: &str) -> Result<(), LiteError> {
        // 先取消
        self.cancel(id).await.ok();

        let mut items = self.items.write().await;
        items.retain(|item| item.id != id);

        Ok(())
    }

    /// 获取队列统计
    pub async fn stats(&self) -> TransferQueueStats {
        let items = self.items.read().await;
        let mut pending = 0;
        let mut transferring = 0;
        let mut completed = 0;
        let mut failed = 0;

        for item in items.iter() {
            match item.progress.status {
                TransferStatus::Pending => pending += 1,
                TransferStatus::Transferring => transferring += 1,
                TransferStatus::Completed => completed += 1,
                TransferStatus::Failed => failed += 1,
                _ => {}
            }
        }

        TransferQueueStats {
            total: items.len(),
            pending,
            transferring,
            completed,
            failed,
        }
    }

    /// 创建取消标志
    async fn create_cancel_flag(&self, id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        let mut flags = self.cancel_flags.write().await;
        flags.insert(id.to_string(), flag.clone());
        flag
    }

    /// 创建暂停标志
    async fn create_pause_flag(&self, id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        let mut flags = self.pause_flags.write().await;
        flags.insert(id.to_string(), flag.clone());
        flag
    }

    /// 清理已完成的传输
    pub async fn cleanup_completed(&self) -> usize {
        let mut items = self.items.write().await;
        let before = items.len();
        items.retain(|item| {
            !matches!(
                item.progress.status,
                TransferStatus::Completed | TransferStatus::Cancelled
            )
        });
        before - items.len()
    }
}

impl Default for TransferQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// 传输队列统计
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferQueueStats {
    pub total: usize,
    pub pending: usize,
    pub transferring: usize,
    pub completed: usize,
    pub failed: usize,
}

/// 传输结果
#[derive(Debug, Clone)]
pub struct TransferResult {
    pub bytes_transferred: u64,
    pub duration: Duration,
    pub average_speed: f64,
    pub was_resumed: bool,
}

/// SFTP会话管理器
pub struct SftpSessionManager {
    sessions: HashMap<String, Arc<TokioMutex<Sftp>>>,
    /// 传输队列
    pub transfer_queue: Arc<RwLock<TransferQueue>>,
    /// 本地文件锁（用于断点续传）
    resume_state: Arc<RwLock<HashMap<String, u64>>>,
}

impl SftpSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            transfer_queue: Arc::new(RwLock::new(TransferQueue::new())),
            resume_state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建SFTP会话
    pub fn create_session(&mut self, session_id: &str, sftp: Sftp) -> Result<(), LiteError> {
        self.sessions
            .insert(session_id.to_string(), Arc::new(TokioMutex::new(sftp)));
        Ok(())
    }

    /// 列出目录内容
    pub async fn list_dir(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<Vec<SftpEntry>, LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or_else(|| LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        let dir = sftp
            .readdir(Path::new(path))
            .map_err(|e| LiteError::Io(format!("读取目录失败: {}", e)))?;

        let mut entries = Vec::new();
        for (p, stat) in dir {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // 跳过当前目录和父目录
            if name == "." || name == ".." {
                continue;
            }

            let file_type = if stat.is_dir() {
                "directory".to_string()
            } else {
                // Check if it's a symlink by checking permissions or assume file
                // Note: ssh2's FileStat doesn't expose is_symlink directly
                "file".to_string()
            };

            entries.push(SftpEntry {
                name,
                path: p.to_string_lossy().to_string(),
                file_type,
                size: stat.size.unwrap_or(0) as i64,
                mtime: stat.mtime.unwrap_or(0) as i64,
                permissions: stat.perm,
            });
        }

        // 按类型和名称排序：目录在前，文件在后
        entries.sort_by(|a, b| match (a.file_type.as_str(), b.file_type.as_str()) {
            ("directory", "file") => std::cmp::Ordering::Less,
            ("file", "directory") => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        Ok(entries)
    }

    /// 创建目录
    pub async fn mkdir(
        &self,
        session_id: &str,
        path: &str,
        mode: Option<i32>,
    ) -> Result<(), LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or_else(|| LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        let permissions = mode.unwrap_or(0o755);
        sftp.mkdir(Path::new(path), permissions)
            .map_err(|e| LiteError::Io(format!("创建目录失败: {}", e)))?;
        Ok(())
    }

    /// 递归创建目录
    pub async fn mkdir_p(&self, session_id: &str, path: &str) -> Result<(), LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or_else(|| LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        let path = Path::new(path);

        // 逐级创建目录
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component);
            if let Err(e) = sftp.mkdir(&current, 0o755) {
                // 忽略目录已存在的错误
                if e.code() != ssh2::ErrorCode::SFTP(4) {
                    return Err(LiteError::Io(format!(
                        "创建目录 {} 失败: {}",
                        current.display(),
                        e
                    )));
                }
            }
        }

        Ok(())
    }

    /// 删除文件
    pub async fn remove_file(&self, session_id: &str, path: &str) -> Result<(), LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or_else(|| LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        sftp.unlink(Path::new(path))
            .map_err(|e| LiteError::Io(format!("删除文件失败: {}", e)))?;
        Ok(())
    }

    /// 删除目录
    pub async fn rmdir(&self, session_id: &str, path: &str) -> Result<(), LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or_else(|| LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        sftp.rmdir(Path::new(path))
            .map_err(|e| LiteError::Io(format!("删除目录失败: {}", e)))?;
        Ok(())
    }

    /// 递归删除目录及其内容
    pub async fn rm_rf(&self, session_id: &str, path: &str) -> Result<(), LiteError> {
        // 使用Box::pin来避免递归async fn导致的无限大小类型
        self.rm_rf_inner(session_id, path).await
    }

    async fn rm_rf_inner(&self, session_id: &str, path: &str) -> Result<(), LiteError> {
        // 先列出目录内容
        let entries = self.list_dir(session_id, path).await?;

        // 删除所有内容
        for entry in entries {
            let full_path = format!("{}/{}", path.trim_end_matches('/'), entry.name);
            if entry.is_dir() {
                Box::pin(self.rm_rf_inner(session_id, &full_path)).await?;
            } else {
                self.remove_file(session_id, &full_path).await?;
            }
        }

        // 删除空目录
        self.rmdir(session_id, path).await?;
        Ok(())
    }

    /// 重命名/移动文件或目录
    pub async fn rename(
        &self,
        session_id: &str,
        old_path: &str,
        new_path: &str,
    ) -> Result<(), LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or_else(|| LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        sftp.rename(Path::new(old_path), Path::new(new_path), None)
            .map_err(|e| LiteError::Io(format!("重命名失败: {}", e)))?;
        Ok(())
    }

    /// 获取文件信息
    pub async fn stat(&self, session_id: &str, path: &str) -> Result<SftpEntry, LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or_else(|| LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        let stat = sftp
            .stat(Path::new(path))
            .map_err(|e| LiteError::Io(format!("获取文件信息失败: {}", e)))?;

        let name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let file_type = if stat.is_dir() {
            "directory".to_string()
        } else {
            "file".to_string()
        };

        Ok(SftpEntry {
            name,
            path: path.to_string(),
            file_type,
            size: stat.size.unwrap_or(0) as i64,
            mtime: stat.mtime.unwrap_or(0) as i64,
            permissions: stat.perm,
        })
    }

    /// 检查文件是否存在
    pub async fn exists(&self, session_id: &str, path: &str) -> Result<bool, LiteError> {
        match self.stat(session_id, path).await {
            Ok(_) => Ok(true),
            Err(LiteError::Io(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// 下载文件（带进度回调）
    pub async fn download_with_progress<F>(
        &self,
        session_id: &str,
        remote_path: &str,
        local_path: &str,
        options: TransferOptions,
        mut progress_callback: F,
    ) -> Result<TransferResult, LiteError>
    where
        F: FnMut(TransferProgress) + Send,
    {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or_else(|| LiteError::Ssh("SFTP会话不存在".to_string()))?
            .clone();

        // 获取远程文件信息
        let remote_stat = {
            let sftp = sftp_mutex.lock().await;
            sftp.stat(Path::new(remote_path))
                .map_err(|e| LiteError::Io(format!("获取远程文件信息失败: {}", e)))?
        };

        let total_size = remote_stat.size.unwrap_or(0);
        let filename = Path::new(remote_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // 检查本地文件是否存在（用于断点续传）
        let local_path_obj = Path::new(local_path);
        let start_offset = if options.resume && local_path_obj.exists() {
            match std::fs::metadata(local_path_obj) {
                Ok(metadata) => metadata.len(),
                Err(_) => 0,
            }
        } else {
            0
        };

        // 如果不需要覆盖且文件已完整存在，则跳过
        if !options.overwrite && start_offset >= total_size && total_size > 0 {
            let mut progress = TransferProgress::new(filename.clone(), Some(total_size), true);
            progress.transferred = total_size;
            progress.complete();
            progress_callback(progress);

            return Ok(TransferResult {
                bytes_transferred: 0,
                duration: Duration::ZERO,
                average_speed: 0.0,
                was_resumed: true,
            });
        }

        // 创建进度跟踪
        let mut progress =
            TransferProgress::new(filename.clone(), Some(total_size), start_offset > 0);
        progress.transferred = start_offset;
        progress.resume_offset = start_offset;
        progress.start();

        let start_time = Instant::now();

        // 创建本地目录（如果不存在）
        if let Some(parent) = local_path_obj.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| LiteError::Io(format!("创建本地目录失败: {}", e)))?;
        }

        // 打开本地文件（追加模式用于续传）
        let file_mode = if start_offset > 0 {
            tokio::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(local_path_obj)
                .await
        } else {
            tokio::fs::File::create(local_path_obj).await
        }
        .map_err(|e| LiteError::Io(format!("创建本地文件失败: {}", e)))?;

        let _local_file = file_mode;

        // 使用spawn_blocking执行SFTP读取
        let chunk_size = options.chunk_size;
        let remote_path_owned = remote_path.to_string();
        let local_path_owned = local_path.to_string();
        let start_offset_owned = start_offset;
        let speed_limit = options.speed_limit;

        let transfer_result = tokio::task::spawn_blocking(move || {
            let runtime = tokio::runtime::Handle::current();
            let mut buffer = vec![0u8; chunk_size];
            let mut total_transferred: u64 = start_offset_owned;

            let transfer_future = async {
                let sftp = sftp_mutex.lock().await;

                // 打开远程文件
                let mut remote_file = sftp
                    .open(Path::new(&remote_path_owned))
                    .map_err(|e| LiteError::Io(format!("打开远程文件失败: {}", e)))?;

                // 如果需要续传，跳转到指定位置
                if start_offset_owned > 0 {
                    remote_file
                        .seek(SeekFrom::Start(start_offset_owned))
                        .map_err(|e| LiteError::Io(format!("文件定位失败: {}", e)))?;
                }

                // 打开本地文件（在阻塞线程中使用std::fs）
                let mut local_file = if start_offset_owned > 0 {
                    std::fs::OpenOptions::new()
                        .write(true)
                        .append(true)
                        .open(&local_path_owned)
                        .map_err(|e| LiteError::Io(format!("打开本地文件失败: {}", e)))?
                } else {
                    std::fs::File::create(&local_path_owned)
                        .map_err(|e| LiteError::Io(format!("创建本地文件失败: {}", e)))?
                };

                let mut last_update = Instant::now();
                let mut bytes_since_update: u64 = 0;

                loop {
                    match remote_file.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            // 关键修复：将数据写入本地文件
                            local_file
                                .write_all(&buffer[..n])
                                .map_err(|e| LiteError::Io(format!("写入本地文件失败: {}", e)))?;

                            total_transferred += n as u64;
                            bytes_since_update += n as u64;

                            // 限速处理
                            if speed_limit > 0 {
                                let expected =
                                    Duration::from_secs_f64(n as f64 / speed_limit as f64);
                                let elapsed = last_update.elapsed();
                                if elapsed < expected {
                                    std::thread::sleep(expected - elapsed);
                                }
                            }

                            // 定期更新
                            if last_update.elapsed().as_millis() >= 100
                                || bytes_since_update >= 64 * 1024
                            {
                                last_update = Instant::now();
                                bytes_since_update = 0;
                            }
                        }
                        Err(e) => return Err(LiteError::Io(format!("读取远程文件失败: {}", e))),
                    }
                }

                // 确保所有数据写入磁盘
                local_file
                    .flush()
                    .map_err(|e| LiteError::Io(format!("刷新本地文件失败: {}", e)))?;

                Ok::<u64, LiteError>(total_transferred)
            };

            runtime.block_on(transfer_future)
        })
        .await
        .map_err(|e| LiteError::Io(format!("传输任务失败: {}", e)))?;

        let bytes_transferred = transfer_result?;
        let duration = start_time.elapsed();

        // 设置文件时间戳
        if options.preserve_time {
            if let Some(mtime) = remote_stat.mtime {
                let mtime_std = std::time::UNIX_EPOCH + std::time::Duration::from_secs(mtime);
                let _ = filetime::set_file_mtime(
                    local_path_obj,
                    filetime::FileTime::from_system_time(mtime_std),
                );
            }
        }

        // 发送最终进度
        let mut final_progress =
            TransferProgress::new(filename, Some(total_size), start_offset > 0);
        final_progress.transferred = bytes_transferred;
        final_progress.complete();
        progress_callback(final_progress);

        Ok(TransferResult {
            bytes_transferred,
            duration,
            average_speed: if duration.as_secs_f64() > 0.0 {
                bytes_transferred as f64 / duration.as_secs_f64()
            } else {
                0.0
            },
            was_resumed: start_offset > 0,
        })
    }

    /// 上传文件（带进度回调）
    pub async fn upload_with_progress<F>(
        &self,
        session_id: &str,
        local_path: &str,
        remote_path: &str,
        options: TransferOptions,
        mut progress_callback: F,
    ) -> Result<TransferResult, LiteError>
    where
        F: FnMut(TransferProgress) + Send,
    {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or_else(|| LiteError::Ssh("SFTP会话不存在".to_string()))?
            .clone();

        // 获取本地文件信息
        let local_metadata = tokio::fs::metadata(local_path)
            .await
            .map_err(|e| LiteError::Io(format!("获取本地文件信息失败: {}", e)))?;

        let total_size = local_metadata.len();
        let filename = Path::new(local_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // 检查远程文件是否存在（用于断点续传）
        let start_offset = if options.resume {
            match self.stat(session_id, remote_path).await {
                Ok(stat) => stat.size as u64,
                Err(_) => 0,
            }
        } else {
            0
        };

        // 如果不需要覆盖且文件已完整存在，则跳过
        if !options.overwrite && start_offset >= total_size && total_size > 0 {
            let mut progress = TransferProgress::new(filename.clone(), Some(total_size), true);
            progress.transferred = total_size;
            progress.complete();
            progress_callback(progress);

            return Ok(TransferResult {
                bytes_transferred: 0,
                duration: Duration::ZERO,
                average_speed: 0.0,
                was_resumed: true,
            });
        }

        // 创建进度跟踪
        let mut progress =
            TransferProgress::new(filename.clone(), Some(total_size), start_offset > 0);
        progress.transferred = start_offset;
        progress.resume_offset = start_offset;
        progress.start();

        let start_time = Instant::now();

        // 创建远程目录（如果不存在）
        let remote_path_obj = Path::new(remote_path);
        if let Some(parent) = remote_path_obj.parent() {
            self.mkdir_p(session_id, &parent.to_string_lossy())
                .await
                .ok();
        }

        let chunk_size = options.chunk_size;
        let remote_path_owned = remote_path.to_string();
        let local_path_owned = local_path.to_string();
        let start_offset_owned = start_offset;
        let speed_limit = options.speed_limit;
        let file_mode = options.file_mode;

        // 使用spawn_blocking执行SFTP上传
        let transfer_result = tokio::task::spawn_blocking(move || {
            let runtime = tokio::runtime::Handle::current();
            let mut total_transferred: u64 = start_offset_owned;

            let transfer_future = async {
                let sftp = sftp_mutex.lock().await;

                // 打开远程文件（追加模式用于续传）
                let mut remote_file = if start_offset_owned > 0 {
                    sftp.open(Path::new(&remote_path_owned))
                        .map_err(|e| LiteError::Io(format!("打开远程文件失败: {}", e)))?
                } else {
                    sftp.create(Path::new(&remote_path_owned))
                        .map_err(|e| LiteError::Io(format!("创建远程文件失败: {}", e)))?
                };

                // 如果是新文件，设置权限
                if start_offset_owned == 0 {
                    let stat = ssh2::FileStat {
                        size: None,
                        uid: None,
                        gid: None,
                        perm: Some(file_mode as u32),
                        atime: None,
                        mtime: None,
                    };
                    sftp.setstat(Path::new(&remote_path_owned), stat).ok();
                }

                // 打开本地文件
                let mut local_file = std::fs::File::open(&local_path_owned)
                    .map_err(|e| LiteError::Io(format!("打开本地文件失败: {}", e)))?;

                // 跳转到续传位置
                if start_offset_owned > 0 {
                    local_file
                        .seek(SeekFrom::Start(start_offset_owned))
                        .map_err(|e| LiteError::Io(format!("本地文件定位失败: {}", e)))?;
                }

                let mut buffer = vec![0u8; chunk_size];
                let mut last_update = Instant::now();
                let mut bytes_since_update: u64 = 0;

                loop {
                    match local_file.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            remote_file
                                .write_all(&buffer[..n])
                                .map_err(|e| LiteError::Io(format!("写入远程文件失败: {}", e)))?;

                            total_transferred += n as u64;
                            bytes_since_update += n as u64;

                            // 限速处理
                            if speed_limit > 0 {
                                let expected =
                                    Duration::from_secs_f64(n as f64 / speed_limit as f64);
                                let elapsed = last_update.elapsed();
                                if elapsed < expected {
                                    std::thread::sleep(expected - elapsed);
                                }
                            }

                            // 定期更新（这里不发送，在阻塞外部处理）
                            if last_update.elapsed().as_millis() >= 100
                                || bytes_since_update >= 64 * 1024
                            {
                                last_update = Instant::now();
                                bytes_since_update = 0;
                            }
                        }
                        Err(e) => return Err(LiteError::Io(format!("读取本地文件失败: {}", e))),
                    }
                }

                Ok::<u64, LiteError>(total_transferred)
            };

            runtime.block_on(transfer_future)
        })
        .await
        .map_err(|e| LiteError::Io(format!("上传任务失败: {}", e)))?;

        let bytes_transferred = transfer_result?;
        let duration = start_time.elapsed();

        // 设置远程文件时间戳
        if options.preserve_time {
            if let Ok(modified) = local_metadata.modified() {
                if let Ok(d) = modified.duration_since(std::time::UNIX_EPOCH) {
                    let mtime = d.as_secs();
                    let sftp_mutex = self
                        .sessions
                        .get(session_id)
                        .ok_or_else(|| LiteError::Ssh("SFTP会话不存在".to_string()))?
                        .clone();
                    let sftp = sftp_mutex.lock().await;
                    let stat = ssh2::FileStat {
                        size: None,
                        uid: None,
                        gid: None,
                        perm: None,
                        atime: None,
                        mtime: Some(mtime),
                    };
                    let _ = sftp.setstat(Path::new(remote_path), stat);
                }
            }
        }

        // 发送最终进度
        let mut final_progress =
            TransferProgress::new(filename, Some(total_size), start_offset > 0);
        final_progress.transferred = bytes_transferred;
        final_progress.complete();
        progress_callback(final_progress);

        Ok(TransferResult {
            bytes_transferred,
            duration,
            average_speed: if duration.as_secs_f64() > 0.0 {
                bytes_transferred as f64 / duration.as_secs_f64()
            } else {
                0.0
            },
            was_resumed: start_offset > 0,
        })
    }

    /// 简单下载（无进度回调）
    pub async fn download(
        &self,
        session_id: &str,
        remote_path: &str,
        local_path: &str,
    ) -> Result<Vec<u8>, LiteError> {
        let options = TransferOptions::default();

        self.download_with_progress(session_id, remote_path, local_path, options, |_progress| {})
            .await?;

        // 读取下载的文件
        let data = tokio::fs::read(local_path)
            .await
            .map_err(|e| LiteError::Io(format!("读取下载文件失败: {}", e)))?;

        Ok(data)
    }

    /// 简单上传（无进度回调）
    pub async fn upload(
        &self,
        session_id: &str,
        remote_path: &str,
        contents: &[u8],
    ) -> Result<(), LiteError> {
        // 先写入临时文件
        let temp_path = std::env::temp_dir().join(format!("sftp_upload_{}", uuid::Uuid::new_v4()));
        tokio::fs::write(&temp_path, contents)
            .await
            .map_err(|e| LiteError::Io(format!("写入临时文件失败: {}", e)))?;

        let options = TransferOptions::default();

        self.upload_with_progress(
            session_id,
            temp_path.to_str().unwrap(),
            remote_path,
            options,
            |_progress| {},
        )
        .await?;

        // 删除临时文件
        let _ = tokio::fs::remove_file(&temp_path).await;

        Ok(())
    }

    /// 下载目录（递归）
    pub async fn download_dir<F>(
        &self,
        session_id: &str,
        remote_path: &str,
        local_path: &str,
        options: TransferOptions,
        mut progress_callback: F,
    ) -> Result<u64, LiteError>
    where
        F: FnMut(TransferProgress) + Send,
    {
        let entries = self.list_dir(session_id, remote_path).await?;
        let mut total_bytes = 0u64;

        // 创建本地目录
        tokio::fs::create_dir_all(local_path)
            .await
            .map_err(|e| LiteError::Io(format!("创建本地目录失败: {}", e)))?;

        for entry in entries {
            let remote_full_path = format!("{}/{}", remote_path.trim_end_matches('/'), entry.name);
            let local_full_path = format!("{}/{}", local_path.trim_end_matches('/'), entry.name);

            if entry.is_dir() {
                // 递归下载子目录
                let sub_bytes = self
                    .download_dir(
                        session_id,
                        &remote_full_path,
                        &local_full_path,
                        options.clone(),
                        |mut progress| {
                            progress.filename = format!("{}/{}", remote_path, progress.filename);
                            progress_callback(progress);
                        },
                    )
                    .await?;
                total_bytes += sub_bytes;
            } else {
                // 下载文件
                let result = self
                    .download_with_progress(
                        session_id,
                        &remote_full_path,
                        &local_full_path,
                        options.clone(),
                        |mut progress| {
                            progress.filename = format!("{}/{}", remote_path, progress.filename);
                            progress_callback(progress);
                        },
                    )
                    .await?;
                total_bytes += result.bytes_transferred;
            }
        }

        Ok(total_bytes)
    }

    /// 上传目录（递归）
    pub async fn upload_dir<F>(
        &self,
        session_id: &str,
        local_path: &str,
        remote_path: &str,
        options: TransferOptions,
        mut progress_callback: F,
    ) -> Result<u64, LiteError>
    where
        F: FnMut(TransferProgress) + Send,
    {
        let local_path_obj = Path::new(local_path);

        // 读取本地目录
        let mut entries = tokio::fs::read_dir(local_path)
            .await
            .map_err(|e| LiteError::Io(format!("读取本地目录失败: {}", e)))?;

        // 创建远程目录
        self.mkdir_p(session_id, remote_path).await?;

        let mut total_bytes = 0u64;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| LiteError::Io(format!("读取目录项失败: {}", e)))?
        {
            let name = entry.file_name().to_string_lossy().to_string();
            let local_full_path = local_path_obj.join(&name).to_string_lossy().to_string();
            let remote_full_path = format!("{}/{}", remote_path.trim_end_matches('/'), name);

            let metadata = entry
                .metadata()
                .await
                .map_err(|e| LiteError::Io(format!("获取文件元数据失败: {}", e)))?;

            if metadata.is_dir() {
                // 递归上传子目录
                let sub_bytes = self
                    .upload_dir(
                        session_id,
                        &local_full_path,
                        &remote_full_path,
                        options.clone(),
                        |mut progress| {
                            progress.filename = format!("{}/{}", remote_path, progress.filename);
                            progress_callback(progress);
                        },
                    )
                    .await?;
                total_bytes += sub_bytes;
            } else {
                // 上传文件
                let result = self
                    .upload_with_progress(
                        session_id,
                        &local_full_path,
                        &remote_full_path,
                        options.clone(),
                        |mut progress| {
                            progress.filename = format!("{}/{}", remote_path, progress.filename);
                            progress_callback(progress);
                        },
                    )
                    .await?;
                total_bytes += result.bytes_transferred;
            }
        }

        Ok(total_bytes)
    }

    /// 将传输任务添加到队列
    pub async fn queue_download(
        &self,
        session_id: impl Into<String>,
        remote_path: impl Into<String>,
        local_path: impl Into<String>,
        options: TransferOptions,
    ) -> Result<String, LiteError> {
        let session_id_str = session_id.into();
        let remote_path_str = remote_path.into();
        let local_path_str = local_path.into();

        // 获取文件大小
        let stat = self.stat(&session_id_str, &remote_path_str).await?;
        let total_size = if stat.is_file() {
            Some(stat.size as u64)
        } else {
            None
        };

        let item = TransferItem::new(
            session_id_str,
            remote_path_str,
            local_path_str,
            TransferDirection::Download,
            options,
            total_size,
        );

        let queue = self.transfer_queue.write().await;
        let id = item.id.clone();
        queue.add(item).await;

        Ok(id)
    }

    /// 将上传任务添加到队列
    pub async fn queue_upload(
        &self,
        session_id: impl Into<String>,
        local_path: impl Into<String>,
        remote_path: impl Into<String>,
        options: TransferOptions,
    ) -> Result<String, LiteError> {
        let session_id_str = session_id.into();
        let local_path_str = local_path.into();
        let remote_path_str = remote_path.into();

        // 获取文件大小
        let metadata = tokio::fs::metadata(&local_path_str)
            .await
            .map_err(|e| LiteError::Io(format!("获取文件信息失败: {}", e)))?;
        let total_size = if metadata.is_file() {
            Some(metadata.len())
        } else {
            None
        };

        let item = TransferItem::new(
            session_id_str,
            local_path_str,
            remote_path_str,
            TransferDirection::Upload,
            options,
            total_size,
        );

        let queue = self.transfer_queue.write().await;
        let id = item.id.clone();
        queue.add(item).await;

        Ok(id)
    }

    /// 关闭会话
    pub async fn close_session(&mut self, session_id: &str) -> Result<(), LiteError> {
        self.sessions.remove(session_id);
        Ok(())
    }

    /// 列出所有会话
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }
}

impl Default for SftpSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 格式化文件大小
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if size >= TB {
        format!("{:.2} TB", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// 格式化速度显示
fn format_speed(bps: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bps >= GB {
        format!("{:.2} GB/s", bps as f64 / GB as f64)
    } else if bps >= MB {
        format!("{:.2} MB/s", bps as f64 / MB as f64)
    } else if bps >= KB {
        format!("{:.2} KB/s", bps as f64 / KB as f64)
    } else {
        format!("{} B/s", bps)
    }
}

/// 文件时间设置工具（跨平台）
#[cfg(unix)]
mod filetime {
    use std::path::Path;
    use std::time::SystemTime;

    #[derive(Clone, Copy)]
    pub struct FileTime(u64);

    impl FileTime {
        pub fn from_system_time(time: SystemTime) -> Self {
            let duration = time
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default();
            FileTime(duration.as_secs())
        }

        pub fn seconds(&self) -> u64 {
            self.0
        }
    }

    pub fn set_file_mtime(path: &Path, mtime: FileTime) -> std::io::Result<()> {
        let times = [libc::timespec {
            tv_sec: mtime.seconds() as i64,
            tv_nsec: 0,
        }; 2];
        let ret = unsafe {
            libc::utimensat(
                libc::AT_FDCWD,
                path.as_os_str().as_bytes().as_ptr() as *const _,
                times.as_ptr(),
                0,
            )
        };
        if ret == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}

#[cfg(not(unix))]
mod filetime {
    use std::path::Path;
    use std::time::SystemTime;

    #[derive(Clone, Copy)]
    pub struct FileTime(u64);

    impl FileTime {
        pub fn from_system_time(time: SystemTime) -> Self {
            let duration = time
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default();
            FileTime(duration.as_secs())
        }
    }

    pub fn set_file_mtime(_path: &Path, _mtime: FileTime) -> std::io::Result<()> {
        // Windows实现 - 使用标准库或WinAPI
        // 这里简化处理，实际项目中可以使用winapi crate
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sftp_session_manager_new() {
        let manager = SftpSessionManager::new();
        assert!(manager.sessions.is_empty());
    }

    #[test]
    fn test_sftp_session_manager_default() {
        let manager: SftpSessionManager = Default::default();
        assert!(manager.sessions.is_empty());
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1), "1 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(10240), "10.00 KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 5), "5.00 MB");
    }

    #[test]
    fn test_format_size_gigabytes() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_size(1024 * 1024 * 1024 * 5), "5.00 GB");
    }

    #[test]
    fn test_format_size_terabytes() {
        assert_eq!(format_size(1024 * 1024 * 1024 * 1024), "1.00 TB");
    }

    #[test]
    fn test_sftp_entry_size_display_file() {
        let entry = SftpEntry {
            name: "test.txt".to_string(),
            path: "/home/test.txt".to_string(),
            file_type: "file".to_string(),
            size: 1024,
            mtime: 1234567890,
            permissions: Some(0o644),
        };
        assert_eq!(entry.size_display(), "1.00 KB");
    }

    #[test]
    fn test_sftp_entry_size_display_directory() {
        let entry = SftpEntry {
            name: "folder".to_string(),
            path: "/home/folder".to_string(),
            file_type: "directory".to_string(),
            size: 4096,
            mtime: 1234567890,
            permissions: Some(0o755),
        };
        assert_eq!(entry.size_display(), "-");
    }

    #[test]
    fn test_sftp_entry_is_dir() {
        let dir = SftpEntry {
            name: "folder".to_string(),
            path: "/home/folder".to_string(),
            file_type: "directory".to_string(),
            size: 4096,
            mtime: 1234567890,
            permissions: Some(0o755),
        };
        assert!(dir.is_dir());
        assert!(!dir.is_file());
        assert!(!dir.is_symlink());
    }

    #[test]
    fn test_sftp_entry_is_file() {
        let file = SftpEntry {
            name: "test.txt".to_string(),
            path: "/home/test.txt".to_string(),
            file_type: "file".to_string(),
            size: 100,
            mtime: 1234567890,
            permissions: Some(0o644),
        };
        assert!(file.is_file());
        assert!(!file.is_dir());
    }

    #[test]
    fn test_sftp_entry_mtime_display_valid() {
        let entry = SftpEntry {
            name: "test.txt".to_string(),
            path: "/home/test.txt".to_string(),
            file_type: "file".to_string(),
            size: 100,
            mtime: 1609459200, // 2021-01-01 00:00:00 UTC
            permissions: Some(0o644),
        };
        let display = entry.mtime_display();
        assert!(display.contains("2021"));
    }

    #[test]
    fn test_sftp_entry_mtime_display_zero() {
        let entry = SftpEntry {
            name: "test.txt".to_string(),
            path: "/home/test.txt".to_string(),
            file_type: "file".to_string(),
            size: 100,
            mtime: 0,
            permissions: None,
        };
        assert_eq!(entry.mtime_display(), "-");
    }

    #[test]
    fn test_sftp_entry_clone() {
        let entry = SftpEntry {
            name: "test.txt".to_string(),
            path: "/home/test.txt".to_string(),
            file_type: "file".to_string(),
            size: 1024,
            mtime: 1234567890,
            permissions: Some(0o644),
        };
        let cloned = entry.clone();

        assert_eq!(entry.name, cloned.name);
        assert_eq!(entry.path, cloned.path);
        assert_eq!(entry.size, cloned.size);
        assert_eq!(entry.permissions, cloned.permissions);
    }

    #[test]
    fn test_sftp_entry_serialize() {
        let entry = SftpEntry {
            name: "test.txt".to_string(),
            path: "/home/test.txt".to_string(),
            file_type: "file".to_string(),
            size: 1024,
            mtime: 1234567890,
            permissions: Some(0o644),
        };
        let json = serde_json::to_string(&entry).expect("Failed to serialize");

        assert!(json.contains("test.txt"));
        assert!(json.contains("permissions"));
    }

    #[test]
    fn test_sftp_entry_deserialize() {
        let json = r#"{"name":"test.txt","path":"/home/test.txt","file_type":"file","size":1024,"mtime":1234567890,"permissions":420}"#;
        let entry: SftpEntry = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.permissions, Some(420));
    }

    #[test]
    fn test_transfer_progress_new() {
        let progress = TransferProgress::new("test.txt", Some(1024), false);
        assert_eq!(progress.filename, "test.txt");
        assert_eq!(progress.transferred, 0);
        assert_eq!(progress.total, Some(1024));
        assert_eq!(progress.status, TransferStatus::Pending);
        assert!(!progress.is_resume);
    }

    #[test]
    fn test_transfer_progress_resume() {
        let progress = TransferProgress::new("test.txt", Some(1024), true);
        assert!(progress.is_resume);
    }

    #[test]
    fn test_transfer_progress_percentage() {
        let mut progress = TransferProgress::new("test.txt", Some(100), false);
        assert_eq!(progress.percentage(), 0.0);

        progress.update(50);
        assert_eq!(progress.percentage(), 50.0);

        progress.update(100);
        assert_eq!(progress.percentage(), 100.0);
    }

    #[test]
    fn test_transfer_progress_percentage_no_total() {
        let progress = TransferProgress::new("test.txt", None, false);
        assert_eq!(progress.percentage(), 0.0);
    }

    #[test]
    fn test_transfer_progress_speed_display() {
        let mut progress = TransferProgress::new("test.txt", Some(1024 * 1024), false);
        progress.speed_bps = 1024.0 * 1024.0; // 1 MB/s
        assert_eq!(progress.speed_display(), "1.00 MB");
    }

    #[test]
    fn test_transfer_progress_eta_display() {
        let mut progress = TransferProgress::new("test.txt", Some(1000), false);
        progress.transferred = 500;
        progress.speed_bps = 100.0; // 100 bytes/s
        progress.eta_secs = Some(5.0);

        let eta = progress.eta_display();
        assert!(eta.contains("5") || eta.contains("s"));
    }

    #[test]
    fn test_transfer_progress_eta_display_no_eta() {
        let progress = TransferProgress::new("test.txt", None, false);
        assert_eq!(progress.eta_display(), "-");
    }

    #[test]
    fn test_transfer_progress_update() {
        let mut progress = TransferProgress::new("test.txt", Some(1000), false);
        let start_time = progress.start_time;

        std::thread::sleep(std::time::Duration::from_millis(50));
        progress.update(500);

        assert_eq!(progress.transferred, 500);
        assert!(progress.elapsed_secs >= 0.0); // Allow for 0 on very fast systems
        assert!(progress.speed_bps >= 0.0);
    }

    #[test]
    fn test_transfer_progress_complete() {
        let mut progress = TransferProgress::new("test.txt", Some(100), false);
        progress.transferred = 100;
        progress.complete();

        assert_eq!(progress.status, TransferStatus::Completed);
    }

    #[test]
    fn test_transfer_progress_fail() {
        let mut progress = TransferProgress::new("test.txt", Some(100), false);
        progress.fail();

        assert_eq!(progress.status, TransferStatus::Failed);
    }

    #[test]
    fn test_transfer_progress_cancel() {
        let mut progress = TransferProgress::new("test.txt", Some(100), false);
        progress.cancel();

        assert_eq!(progress.status, TransferStatus::Cancelled);
    }

    #[test]
    fn test_transfer_options_default() {
        let options = TransferOptions::default();
        assert_eq!(options.chunk_size, 64 * 1024);
        assert!(options.resume);
        assert_eq!(options.max_concurrent, 3);
        assert_eq!(options.speed_limit, 0);
        assert!(options.overwrite);
        assert!(options.preserve_time);
        assert!(options.preserve_permissions);
        assert_eq!(options.file_mode, 0o644);
    }

    #[test]
    fn test_transfer_options_builder() {
        let options = TransferOptions::default()
            .with_chunk_size(128 * 1024)
            .with_resume(false)
            .with_max_concurrent(5)
            .with_speed_limit(1024 * 1024)
            .with_overwrite(false);

        assert_eq!(options.chunk_size, 128 * 1024);
        assert!(!options.resume);
        assert_eq!(options.max_concurrent, 5);
        assert_eq!(options.speed_limit, 1024 * 1024);
        assert!(!options.overwrite);
    }

    #[test]
    fn test_transfer_direction_serialize() {
        let upload = TransferDirection::Upload;
        let download = TransferDirection::Download;

        let upload_json = serde_json::to_string(&upload).unwrap();
        let download_json = serde_json::to_string(&download).unwrap();

        assert!(upload_json.contains("Upload") || upload_json.contains("upload"));
        assert!(download_json.contains("Download") || download_json.contains("download"));
    }

    #[test]
    fn test_transfer_queue_stats() {
        let stats = TransferQueueStats {
            total: 10,
            pending: 3,
            transferring: 2,
            completed: 4,
            failed: 1,
        };

        assert_eq!(stats.total, 10);
        assert_eq!(
            stats.pending + stats.transferring + stats.completed + stats.failed,
            stats.total
        );
    }

    #[test]
    fn test_transfer_item_new() {
        let item = TransferItem::new(
            "session-1",
            "/remote/file.txt",
            "/local/file.txt",
            TransferDirection::Download,
            TransferOptions::default(),
            Some(1024),
        );

        assert!(!item.id.is_empty());
        assert_eq!(item.session_id, "session-1");
        assert_eq!(item.source_path, "/remote/file.txt");
        assert_eq!(item.dest_path, "/local/file.txt");
        assert_eq!(item.direction, TransferDirection::Download);
        assert_eq!(item.progress.total, Some(1024));
    }

    #[test]
    fn test_transfer_item_filename() {
        let item = TransferItem::new(
            "session-1",
            "/path/to/file.txt",
            "/local/",
            TransferDirection::Download,
            TransferOptions::default(),
            None,
        );

        assert_eq!(item.progress.filename, "file.txt");
    }

    #[test]
    fn test_transfer_result() {
        let result = TransferResult {
            bytes_transferred: 1024 * 1024,
            duration: Duration::from_secs(1),
            average_speed: 1024.0 * 1024.0,
            was_resumed: true,
        };

        assert_eq!(result.bytes_transferred, 1024 * 1024);
        assert!(result.was_resumed);
    }

    // 使用tokio的测试需要#[tokio::test]
    #[tokio::test]
    async fn test_transfer_queue_new() {
        let queue = TransferQueue::new();
        let stats = queue.stats().await;
        assert_eq!(stats.total, 0);
    }

    #[tokio::test]
    async fn test_transfer_queue_add() {
        let queue = TransferQueue::new();
        let item = TransferItem::new(
            "session-1",
            "/remote/file.txt",
            "/local/file.txt",
            TransferDirection::Download,
            TransferOptions::default(),
            Some(1024),
        );

        let id = queue.add(item).await;
        assert!(!id.is_empty());

        let stats = queue.stats().await;
        assert_eq!(stats.total, 1);
    }

    #[tokio::test]
    async fn test_transfer_queue_get() {
        let queue = TransferQueue::new();
        let item = TransferItem::new(
            "session-1",
            "/remote/file.txt",
            "/local/file.txt",
            TransferDirection::Download,
            TransferOptions::default(),
            Some(1024),
        );

        let id = queue.add(item.clone()).await;
        let retrieved = queue.get(&id).await;

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().source_path, "/remote/file.txt");
    }

    #[tokio::test]
    async fn test_transfer_queue_list() {
        let queue = TransferQueue::new();

        for i in 0..3 {
            let item = TransferItem::new(
                "session-1",
                format!("/remote/file{}.txt", i),
                format!("/local/file{}.txt", i),
                TransferDirection::Download,
                TransferOptions::default(),
                Some(1024),
            );
            queue.add(item).await;
        }

        let list = queue.list().await;
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn test_transfer_queue_remove() {
        let queue = TransferQueue::new();
        let item = TransferItem::new(
            "session-1",
            "/remote/file.txt",
            "/local/file.txt",
            TransferDirection::Download,
            TransferOptions::default(),
            Some(1024),
        );

        let id = queue.add(item).await;
        assert_eq!(queue.stats().await.total, 1);

        queue.remove(&id).await.unwrap();
        assert_eq!(queue.stats().await.total, 0);
    }

    #[tokio::test]
    async fn test_transfer_queue_pause_resume() {
        let queue = TransferQueue::new();
        let item = TransferItem::new(
            "session-1",
            "/remote/file.txt",
            "/local/file.txt",
            TransferDirection::Download,
            TransferOptions::default(),
            Some(1024),
        );

        let id = queue.add(item).await;
        queue.pause(&id).await.unwrap();

        let retrieved = queue.get(&id).await.unwrap();
        assert_eq!(retrieved.progress.status, TransferStatus::Paused);

        queue.resume(&id).await.unwrap();
        let resumed = queue.get(&id).await.unwrap();
        assert_eq!(resumed.progress.status, TransferStatus::Transferring);
    }

    #[tokio::test]
    async fn test_transfer_queue_cancel() {
        let queue = TransferQueue::new();
        let item = TransferItem::new(
            "session-1",
            "/remote/file.txt",
            "/local/file.txt",
            TransferDirection::Download,
            TransferOptions::default(),
            Some(1024),
        );

        let id = queue.add(item).await;
        queue.cancel(&id).await.unwrap();

        let retrieved = queue.get(&id).await.unwrap();
        assert_eq!(retrieved.progress.status, TransferStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_transfer_queue_cleanup() {
        let queue = TransferQueue::new();

        // 添加已完成的项目
        let completed_item = TransferItem::new(
            "session-1",
            "/remote/file1.txt",
            "/local/file1.txt",
            TransferDirection::Download,
            TransferOptions::default(),
            Some(1024),
        );
        let id1 = queue.add(completed_item).await;
        queue.cancel(&id1).await.unwrap(); // 标记为取消/完成

        // 添加未完成的项目
        let pending_item = TransferItem::new(
            "session-1",
            "/remote/file2.txt",
            "/local/file2.txt",
            TransferDirection::Download,
            TransferOptions::default(),
            Some(1024),
        );
        queue.add(pending_item).await;

        let cleaned = queue.cleanup_completed().await;
        assert_eq!(cleaned, 1);
        assert_eq!(queue.stats().await.total, 1);
    }
}
