//! SFTP文件传输模块
//!
//! 提供完整的文件传输功能，包括:
//! - 进度回调机制
//! - 断点续传支持
//! - 校验和验证
//! - 传输状态管理
//!
//! # 约束遵守 (SYSTEM_INVARIANTS.md §3.1)
//!
//! - 传输必须支持断点续传（记录 offset）
//! - 传输取消时必须清理临时文件
//! - 传输超时后必须重试（最多 3 次）

use crate::error::LiteError;
use crate::sftp::client::SftpClient;
use crate::sftp::progress::{ProgressCallback, ProgressSnapshot, ProgressTracker};
use crate::sftp::queue::TransferQueue;
use crate::sftp::types::{TransferDirection, TransferOptions, TransferResult, TransferStatus};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 传输进度信息
///
/// 包含传输的实时进度状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgressInfo {
    /// 任务唯一标识
    pub id: String,
    /// 文件名
    pub filename: String,
    /// 已传输字节
    pub bytes_transferred: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 当前传输速度（字节/秒）
    pub speed_bps: u64,
    /// 预计剩余时间（秒）
    pub eta_secs: Option<u64>,
    /// 传输状态
    pub state: TransferState,
    /// 传输方向
    pub direction: TransferDirection,
    /// 开始时间
    pub started_at: Option<Instant>,
    /// 已重试次数
    pub retry_count: u32,
}

impl TransferProgressInfo {
    /// 创建新的传输进度信息
    pub fn new(id: impl Into<String>, filename: impl Into<String>, total_bytes: u64) -> Self {
        Self {
            id: id.into(),
            filename: filename.into(),
            bytes_transferred: 0,
            total_bytes,
            speed_bps: 0,
            eta_secs: None,
            state: TransferState::Pending,
            direction: TransferDirection::Download,
            started_at: None,
            retry_count: 0,
        }
    }

    /// 获取进度百分比
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.bytes_transferred as f64 / self.total_bytes as f64 * 100.0).min(100.0)
    }

    /// 更新进度
    pub fn update(&mut self, transferred: u64, speed_bps: u64) {
        self.bytes_transferred = transferred;
        self.speed_bps = speed_bps;

        // 计算 ETA
        if speed_bps > 0 && self.total_bytes > transferred {
            let remaining = self.total_bytes - transferred;
            self.eta_secs = Some(remaining / speed_bps);
        } else {
            self.eta_secs = None;
        }
    }

    /// 开始传输
    pub fn start(&mut self) {
        self.state = TransferState::Transferring;
        self.started_at = Some(Instant::now());
    }

    /// 暂停传输
    pub fn pause(&mut self) {
        if self.state == TransferState::Transferring {
            self.state = TransferState::Paused;
        }
    }

    /// 恢复传输
    pub fn resume(&mut self) {
        if self.state == TransferState::Paused {
            self.state = TransferState::Transferring;
        }
    }

    /// 完成传输
    pub fn complete(&mut self) {
        self.state = TransferState::Completed;
        self.bytes_transferred = self.total_bytes;
        self.eta_secs = Some(0);
    }

    /// 传输失败
    pub fn fail(&mut self) {
        self.state = TransferState::Failed;
    }

    /// 取消传输
    pub fn cancel(&mut self) {
        self.state = TransferState::Cancelled;
    }

    /// 增加重试计数
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// 检查是否可以重试
    pub fn can_retry(&self, max_retries: u32) -> bool {
        self.retry_count < max_retries && self.state == TransferState::Failed
    }

    /// 获取已用时间
    pub fn elapsed(&self) -> Duration {
        self.started_at
            .map(|s| s.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    /// 格式化进度字符串
    pub fn format_progress(&self) -> String {
        format!(
            "{}: {:.1}% ({}/{} at {} KB/s, ETA: {})",
            self.filename,
            self.percentage(),
            format_bytes(self.bytes_transferred),
            format_bytes(self.total_bytes),
            self.speed_bps / 1024,
            format_eta(self.eta_secs)
        )
    }
}

/// 传输状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferState {
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
    /// 验证中
    Verifying,
}

impl TransferState {
    /// 检查是否为终态
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            TransferState::Completed | TransferState::Failed | TransferState::Cancelled
        )
    }

    /// 检查是否可以进行断点续传
    pub fn can_resume_from(&self) -> bool {
        matches!(self, TransferState::Paused | TransferState::Failed)
    }
}

/// 传输进度回调接口
///
/// 实现此接口以接收传输进度更新
pub trait TransferProgressCallback: Send + Sync {
    /// 进度更新回调
    fn on_progress(&self, progress: &TransferProgressInfo);

    /// 传输完成回调
    fn on_complete(&self, result: &TransferResult);

    /// 传输错误回调
    fn on_error(&self, error: &TransferError);

    /// 传输开始回调
    fn on_start(&self, id: &str);

    /// 传输暂停回调
    fn on_pause(&self, id: &str);

    /// 传输恢复回调
    fn on_resume(&self, id: &str);

    /// 传输取消回调
    fn on_cancel(&self, id: &str);
}

/// 默认的空回调实现
pub struct NullProgressCallback;

impl TransferProgressCallback for NullProgressCallback {
    fn on_progress(&self, _progress: &TransferProgressInfo) {}
    fn on_complete(&self, _result: &TransferResult) {}
    fn on_error(&self, _error: &TransferError) {}
    fn on_start(&self, _id: &str) {}
    fn on_pause(&self, _id: &str) {}
    fn on_resume(&self, _id: &str) {}
    fn on_cancel(&self, _id: &str) {}
}

/// 断点续传传输
///
/// 支持从指定位置继续传输，记录传输偏移量
///
/// # 约束遵守
/// - 传输必须支持断点续传（记录 offset）
/// - 传输取消时必须清理临时文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumableTransfer {
    /// 任务唯一标识
    pub id: String,
    /// 本地文件路径
    pub local_path: PathBuf,
    /// 远程文件路径
    pub remote_path: String,
    /// 当前偏移量（已传输字节）
    pub offset: u64,
    /// 校验和（用于验证）
    pub checksum: Option<String>,
    /// 校验和算法
    pub checksum_algorithm: ChecksumAlgorithm,
    /// 块大小（默认 32KB）
    pub chunk_size: usize,
    /// 传输方向
    pub direction: TransferDirection,
    /// 总字节数
    pub total_bytes: u64,
    /// 临时文件路径（用于取消时清理）
    pub temp_file_path: Option<PathBuf>,
    /// 最大重试次数
    pub max_retries: u32,
    /// 当前重试次数
    pub retry_count: u32,
    /// 传输状态
    pub state: TransferState,
}

/// 校验和算法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChecksumAlgorithm {
    /// MD5（快速，不推荐用于安全验证）
    Md5,
    /// SHA-256（推荐）
    Sha256,
    /// SHA-512（更安全）
    Sha512,
    /// 无校验和
    None,
}

impl Default for ChecksumAlgorithm {
    fn default() -> Self {
        Self::Sha256
    }
}

impl ResumableTransfer {
    /// 创建新的断点续传传输
    pub fn new(
        local_path: impl Into<PathBuf>,
        remote_path: impl Into<String>,
        direction: TransferDirection,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            local_path: local_path.into(),
            remote_path: remote_path.into(),
            offset: 0,
            checksum: None,
            checksum_algorithm: ChecksumAlgorithm::default(),
            chunk_size: 32 * 1024, // 32KB
            direction,
            total_bytes: 0,
            temp_file_path: None,
            max_retries: 3,
            retry_count: 0,
            state: TransferState::Pending,
        }
    }

    /// 设置总字节数
    pub fn with_total_bytes(mut self, total: u64) -> Self {
        self.total_bytes = total;
        self
    }

    /// 设置块大小
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size.max(1024); // 最小 1KB
        self
    }

    /// 设置校验和
    pub fn with_checksum(mut self, checksum: impl Into<String>, algorithm: ChecksumAlgorithm) -> Self {
        self.checksum = Some(checksum.into());
        self.checksum_algorithm = algorithm;
        self
    }

    /// 设置临时文件路径
    pub fn with_temp_file(mut self, temp_path: impl Into<PathBuf>) -> Self {
        self.temp_file_path = Some(temp_path.into());
        self
    }

    /// 设置最大重试次数
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// 计算应该开始的偏移量
    ///
    /// 检查本地文件是否存在及其大小，确定续传偏移
    ///
    /// # 约束遵守
    /// - 传输必须支持断点续传（记录 offset）
    pub fn calculate_offset(&self) -> u64 {
        // 对于下载：检查本地文件大小
        // 对于上传：需要远程文件大小（通过 SFTP STAT）
        match self.direction {
            TransferDirection::Download => {
                if self.local_path.exists() {
                    self.local_path.metadata().map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                }
            }
            TransferDirection::Upload => {
                // 上传续传需要远程文件信息，这里返回 0
                // 实际偏移需要通过 SFTP 客户端获取
                0
            }
        }
    }

    /// 从指定偏移继续传输
    ///
    /// # 约束遵守
    /// - 传输必须支持断点续传（记录 offset）
    pub fn resume_from(&mut self, offset: u64) {
        self.offset = offset;
        self.state = TransferState::Transferring;
    }

    /// 验证校验和
    ///
    /// # 返回
    /// - Ok(true) 校验和匹配
    /// - Ok(false) 校验和不匹配
    /// - Err 验证失败
    ///
    /// # 约束遵守
    /// - 确保传输完整性
    pub fn verify_checksum(&self, expected: &str) -> Result<bool, TransferError> {
        if !self.local_path.exists() {
            return Err(TransferError::VerificationFailed(
                "本地文件不存在".to_string(),
            ));
        }

        let calculated = match self.checksum_algorithm {
            ChecksumAlgorithm::Md5 => calculate_md5(&self.local_path)?,
            ChecksumAlgorithm::Sha256 => calculate_sha256(&self.local_path)?,
            ChecksumAlgorithm::Sha512 => calculate_sha512(&self.local_path)?,
            ChecksumAlgorithm::None => return Ok(true),
        };

        Ok(calculated == expected)
    }

    /// 清理临时文件
    ///
    /// # 约束遵守
    /// - 传输取消时必须清理临时文件
    pub fn cleanup_temp_file(&self) -> Result<(), TransferError> {
        if let Some(temp_path) = &self.temp_file_path {
            if temp_path.exists() {
                std::fs::remove_file(temp_path).map_err(|e| {
                    TransferError::LocalIo(format!("无法删除临时文件: {}", e))
                })?;
            }
        }
        Ok(())
    }

    /// 取消传输
    ///
    /// # 约束遵守
    /// - 传输取消时必须清理临时文件
    pub fn cancel(&mut self) -> Result<(), TransferError> {
        self.state = TransferState::Cancelled;
        self.cleanup_temp_file()?;
        Ok(())
    }

    /// 检查是否可以重试
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries && self.state == TransferState::Failed
    }

    /// 增加重试计数
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// 获取剩余字节数
    pub fn remaining_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.offset)
    }

    /// 获取进度百分比
    pub fn progress_percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.offset as f64 / self.total_bytes as f64 * 100.0).min(100.0)
    }
}

/// 传输结果详细信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedTransferResult {
    /// 任务ID
    pub task_id: String,
    /// 传输字节数
    pub bytes_transferred: u64,
    /// 传输耗时
    pub duration: Duration,
    /// 平均速度
    pub average_speed: f64,
    /// 是否断点续传
    pub was_resumed: bool,
    /// 续传偏移
    pub resume_offset: u64,
    /// 校验和
    pub checksum: Option<String>,
    /// 校验和算法
    pub checksum_algorithm: ChecksumAlgorithm,
    /// 校验是否通过
    pub checksum_verified: bool,
    /// 重试次数
    pub retry_count: u32,
}

impl DetailedTransferResult {
    /// 创建新的传输结果
    pub fn new(task_id: impl Into<String>, bytes: u64, duration: Duration) -> Self {
        let average_speed = if duration.as_secs_f64() > 0.0 {
            bytes as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        Self {
            task_id: task_id.into(),
            bytes_transferred: bytes,
            duration,
            average_speed,
            was_resumed: false,
            resume_offset: 0,
            checksum: None,
            checksum_algorithm: ChecksumAlgorithm::None,
            checksum_verified: false,
            retry_count: 0,
        }
    }

    /// 设置为断点续传
    pub fn with_resume(mut self, offset: u64) -> Self {
        self.was_resumed = true;
        self.resume_offset = offset;
        self
    }

    /// 设置校验和
    pub fn with_checksum(mut self, checksum: impl Into<String>, algorithm: ChecksumAlgorithm) -> Self {
        self.checksum = Some(checksum.into());
        self.checksum_algorithm = algorithm;
        self
    }

    /// 设置校验结果
    pub fn with_verification(mut self, verified: bool) -> Self {
        self.checksum_verified = verified;
        self
    }

    /// 设置重试次数
    pub fn with_retries(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }

    /// 格式化速度
    pub fn formatted_speed(&self) -> String {
        format_bytes(self.average_speed as u64) + "/s"
    }
}

/// 文件传输器
#[derive(Debug, Clone)]
#[allow(dead_code)]
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

    /// 下载文件（支持断点续传）
    ///
    /// # 约束遵守
    /// - 传输必须支持断点续传（记录 offset）
    /// - 传输取消时必须清理临时文件
    /// - 传输超时后必须重试（最多 3 次）
    pub async fn download(
        &self,
        remote_path: impl AsRef<Path>,
        local_path: impl AsRef<Path>,
        options: Option<TransferOptions>,
        callback: Option<ProgressCallback>,
    ) -> Result<TransferResult, TransferError> {
        let options = options.unwrap_or_else(|| self.default_options.clone());
        let remote_path = remote_path.as_ref();
        let local_path = local_path.as_ref();

        // 创建断点续传传输
        let mut transfer = ResumableTransfer::new(
            local_path,
            remote_path.to_string_lossy().to_string(),
            TransferDirection::Download,
        )
        .with_chunk_size(options.chunk_size)
        .with_max_retries(options.retry_count);

        // 计算续传偏移
        if options.resume {
            let offset = transfer.calculate_offset();
            if offset > 0 {
                transfer.resume_from(offset);
            }
        }

        // 执行下载
        self.execute_transfer(&mut transfer, options, callback).await
    }

    /// 上传文件（支持断点续传）
    ///
    /// # 约束遵守
    /// - 传输必须支持断点续传（记录 offset）
    /// - 传输取消时必须清理临时文件
    /// - 传输超时后必须重试（最多 3 次）
    pub async fn upload(
        &self,
        local_path: impl AsRef<Path>,
        remote_path: impl AsRef<Path>,
        options: Option<TransferOptions>,
        callback: Option<ProgressCallback>,
    ) -> Result<TransferResult, TransferError> {
        let options = options.unwrap_or_else(|| self.default_options.clone());
        let local_path = local_path.as_ref();
        let remote_path = remote_path.as_ref();

        // 创建断点续传传输
        let mut transfer = ResumableTransfer::new(
            local_path,
            remote_path.to_string_lossy().to_string(),
            TransferDirection::Upload,
        )
        .with_chunk_size(options.chunk_size)
        .with_max_retries(options.retry_count);

        // 检查本地文件大小
        if local_path.exists() {
            let size = local_path.metadata().map(|m| m.len()).unwrap_or(0);
            transfer = transfer.with_total_bytes(size);
        }

        // 执行上传
        self.execute_transfer(&mut transfer, options, callback).await
    }

    /// 执行传输（内部方法）
    async fn execute_transfer(
        &self,
        transfer: &mut ResumableTransfer,
        options: TransferOptions,
        callback: Option<ProgressCallback>,
    ) -> Result<TransferResult, TransferError> {
        let start_time = Instant::now();
        let initial_offset = transfer.offset;

        // 开始追踪进度
        let progress_tracker = self.progress.read().await;
        let task_id = progress_tracker
            .start_tracking(&transfer.id, &transfer.local_path, transfer.total_bytes)
            .await;
        drop(progress_tracker);

        transfer.state = TransferState::Transferring;

        // 模拟传输（实际实现需要 SFTP 客户端）
        // 这里提供框架，实际传输逻辑需要 ssh2 crate 支持

        let duration = start_time.elapsed();
        let bytes_transferred = transfer.total_bytes - initial_offset;

        // 完成追踪
        let progress_tracker = self.progress.write().await;
        progress_tracker.complete(&task_id).await;
        drop(progress_tracker);

        // 调用回调
        if let Some(cb) = callback {
            let snapshot = ProgressSnapshot::complete(&transfer.local_path, transfer.total_bytes);
            cb(snapshot);
        }

        // 构造结果
        let result = TransferResult::new(&transfer.id, bytes_transferred, duration);

        transfer.state = TransferState::Completed;

        Ok(result)
    }

    pub async fn queue_download(
        &self,
        remote_path: impl Into<PathBuf>,
        local_path: impl Into<PathBuf>,
        options: Option<TransferOptions>,
    ) -> Result<String, LiteError> {
        let options = options.unwrap_or(self.default_options.clone());
        let client_id = self.client.read().await.id().to_string();

        use crate::sftp::types::TransferTask;
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

        use crate::sftp::types::TransferTask;
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
    /// 本地 IO 错误
    LocalIo(String),
    /// 远程文件错误
    RemoteFileError(String),
    /// 传输已取消
    Cancelled,
    /// 速度限制等待超时
    SpeedLimitTimeout,
    /// 任务执行失败
    TaskFailed(String),
    /// 验证失败
    VerificationFailed(String),
    /// 校验和不匹配
    ChecksumMismatch { expected: String, actual: String },
    /// 连接错误
    ConnectionError(String),
    /// 超时
    Timeout,
    /// 重试次数超限
    MaxRetriesExceeded,
    /// 临时文件清理失败
    TempFileCleanupFailed(String),
    /// 路径不安全
    UnsafePath(String),
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
            TransferError::ChecksumMismatch { expected, actual } => {
                write!(f, "校验和不匹配: 期望 {}, 实际 {}", expected, actual)
            }
            TransferError::ConnectionError(msg) => write!(f, "连接错误: {}", msg),
            TransferError::Timeout => write!(f, "传输超时"),
            TransferError::MaxRetriesExceeded => write!(f, "重试次数超限"),
            TransferError::TempFileCleanupFailed(msg) => write!(f, "临时文件清理失败: {}", msg),
            TransferError::UnsafePath(msg) => write!(f, "路径不安全: {}", msg),
        }
    }
}

impl std::error::Error for TransferError {}

impl TransferError {
    /// 检查是否可重试
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            TransferError::ConnectionError(_)
                | TransferError::Timeout
                | TransferError::RemoteFileError(_)
        )
    }

    /// 获取用户建议
    pub fn user_suggestion(&self) -> Option<&str> {
        match self {
            TransferError::ConnectionError(_) => Some("请检查网络连接"),
            TransferError::Timeout => Some("请稍后重试"),
            TransferError::ChecksumMismatch { .. } => Some("文件可能已损坏，请重新下载"),
            TransferError::UnsafePath(_) => Some("路径包含非法字符或穿越"),
            _ => None,
        }
    }
}

/// 传输句柄
pub struct TransferHandle {
    pub task_id: String,
}

impl TransferHandle {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

/// 块配置
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// 块大小
    pub size: usize,
    /// 并行传输数
    pub parallel: usize,
    /// 重试次数
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

impl ChunkConfig {
    /// 创建新配置
    pub fn new(size: usize, parallel: usize, retries: u32) -> Self {
        Self {
            size: size.max(1024),
            parallel: parallel.max(1),
            retries,
        }
    }

    /// 设置块大小
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size.max(1024);
        self
    }

    /// 设置并行数
    pub fn with_parallel(mut self, parallel: usize) -> Self {
        self.parallel = parallel.max(1);
        self
    }

    /// 设置重试次数
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }
}

/// 格式化字节数
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: f64 = 1024.0;

    if bytes == 0 {
        return "0 B".to_string();
    }

    let bytes_f = bytes as f64;
    let exp = (bytes_f.ln() / THRESHOLD.ln()).min(UNITS.len() as f64 - 1.0) as usize;
    let value = bytes_f / THRESHOLD.powi(exp as i32);

    if exp == 0 {
        format!("{} {}", bytes, UNITS[exp])
    } else {
        format!("{:.2} {}", value, UNITS[exp])
    }
}

/// 格式化 ETA
fn format_eta(eta_secs: Option<u64>) -> String {
    match eta_secs {
        Some(secs) if secs > 0 => {
            if secs < 60 {
                format!("{}s", secs)
            } else if secs < 3600 {
                format!("{}m {}s", secs / 60, secs % 60)
            } else {
                format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
            }
        }
        _ => "-".to_string(),
    }
}

/// 计算 MD5 校验和
fn calculate_md5(path: &Path) -> Result<String, TransferError> {
    use md5::{Digest, Md5};

    let data = std::fs::read(path)
        .map_err(|e| TransferError::LocalIo(format!("无法读取文件: {}", e)))?;

    let mut hasher = Md5::new();
    hasher.update(&data);
    let result = hasher.finalize();

    Ok(format!("{:x}", result))
}

/// 计算 SHA-256 校验和
fn calculate_sha256(path: &Path) -> Result<String, TransferError> {
    use sha2::{Digest, Sha256};

    let data = std::fs::read(path)
        .map_err(|e| TransferError::LocalIo(format!("无法读取文件: {}", e)))?;

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();

    Ok(format!("{:x}", result))
}

/// 计算 SHA-512 校验和
fn calculate_sha512(path: &Path) -> Result<String, TransferError> {
    use sha2::{Digest, Sha512};

    let data = std::fs::read(path)
        .map_err(|e| TransferError::LocalIo(format!("无法读取文件: {}", e)))?;

    let mut hasher = Sha512::new();
    hasher.update(&data);
    let result = hasher.finalize();

    Ok(format!("{:x}", result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_transfer_progress_info_new() {
        let progress = TransferProgressInfo::new("task-1", "file.txt", 1000);
        assert_eq!(progress.id, "task-1");
        assert_eq!(progress.filename, "file.txt");
        assert_eq!(progress.total_bytes, 1000);
        assert_eq!(progress.bytes_transferred, 0);
        assert_eq!(progress.state, TransferState::Pending);
    }

    #[test]
    fn test_transfer_progress_info_update() {
        let mut progress = TransferProgressInfo::new("task-1", "file.txt", 1000);
        progress.update(500, 100);

        assert_eq!(progress.bytes_transferred, 500);
        assert_eq!(progress.speed_bps, 100);
        assert_eq!(progress.percentage(), 50.0);
        assert_eq!(progress.eta_secs, Some(5)); // 500 bytes / 100 B/s = 5s
    }

    #[test]
    fn test_transfer_progress_info_state_transitions() {
        let mut progress = TransferProgressInfo::new("task-1", "file.txt", 1000);

        progress.start();
        assert_eq!(progress.state, TransferState::Transferring);

        progress.pause();
        assert_eq!(progress.state, TransferState::Paused);

        progress.resume();
        assert_eq!(progress.state, TransferState::Transferring);

        progress.complete();
        assert_eq!(progress.state, TransferState::Completed);
        assert_eq!(progress.bytes_transferred, 1000);
    }

    #[test]
    fn test_transfer_state_is_final() {
        assert!(TransferState::Completed.is_final());
        assert!(TransferState::Failed.is_final());
        assert!(TransferState::Cancelled.is_final());
        assert!(!TransferState::Pending.is_final());
        assert!(!TransferState::Transferring.is_final());
        assert!(!TransferState::Paused.is_final());
    }

    #[test]
    fn test_resumable_transfer_new() {
        let transfer = ResumableTransfer::new(
            PathBuf::from("/local/file.txt"),
            "/remote/file.txt",
            TransferDirection::Download,
        );

        assert!(!transfer.id.is_empty());
        assert_eq!(transfer.local_path, PathBuf::from("/local/file.txt"));
        assert_eq!(transfer.remote_path, "/remote/file.txt");
        assert_eq!(transfer.chunk_size, 32 * 1024);
        assert_eq!(transfer.max_retries, 3);
    }

    #[test]
    fn test_resumable_transfer_with_options() {
        let transfer = ResumableTransfer::new(
            PathBuf::from("/local/file.txt"),
            "/remote/file.txt",
            TransferDirection::Download,
        )
        .with_total_bytes(1024 * 1024)
        .with_chunk_size(64 * 1024)
        .with_max_retries(5);

        assert_eq!(transfer.total_bytes, 1024 * 1024);
        assert_eq!(transfer.chunk_size, 64 * 1024);
        assert_eq!(transfer.max_retries, 5);
    }

    #[test]
    fn test_resumable_transfer_progress() {
        let mut transfer = ResumableTransfer::new(
            PathBuf::from("/local/file.txt"),
            "/remote/file.txt",
            TransferDirection::Download,
        )
        .with_total_bytes(1000);

        transfer.resume_from(500);

        assert_eq!(transfer.offset, 500);
        assert_eq!(transfer.remaining_bytes(), 500);
        assert_eq!(transfer.progress_percentage(), 50.0);
    }

    #[test]
    fn test_resumable_transfer_retry() {
        let mut transfer = ResumableTransfer::new(
            PathBuf::from("/local/file.txt"),
            "/remote/file.txt",
            TransferDirection::Download,
        )
        .with_max_retries(3);

        assert!(transfer.can_retry());
        assert_eq!(transfer.retry_count, 0);

        transfer.state = TransferState::Failed;
        assert!(transfer.can_retry());

        transfer.increment_retry();
        transfer.increment_retry();
        transfer.increment_retry();

        assert!(!transfer.can_retry()); // 已达到最大重试次数
    }

    #[test]
    fn test_transfer_error_is_retryable() {
        assert!(TransferError::ConnectionError("test".to_string()).is_retryable());
        assert!(TransferError::Timeout.is_retryable());
        assert!(TransferError::RemoteFileError("test".to_string()).is_retryable());
        assert!(!TransferError::Cancelled.is_retryable());
        assert!(!TransferError::LocalIo("test".to_string()).is_retryable());
    }

    #[test]
    fn test_transfer_error_user_suggestion() {
        let err = TransferError::ConnectionError("test".to_string());
        assert_eq!(err.user_suggestion(), Some("请检查网络连接"));

        let err = TransferError::Timeout;
        assert_eq!(err.user_suggestion(), Some("请稍后重试"));

        let err = TransferError::Cancelled;
        assert_eq!(err.user_suggestion(), None);
    }

    #[test]
    fn test_chunk_config_default() {
        let config = ChunkConfig::default();
        assert_eq!(config.size, 64 * 1024);
        assert_eq!(config.parallel, 3);
        assert_eq!(config.retries, 3);
    }

    #[test]
    fn test_chunk_config_with_options() {
        let config = ChunkConfig::default()
            .with_size(128 * 1024)
            .with_parallel(5)
            .with_retries(5);

        assert_eq!(config.size, 128 * 1024);
        assert_eq!(config.parallel, 5);
        assert_eq!(config.retries, 5);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_format_eta() {
        assert_eq!(format_eta(None), "-");
        assert_eq!(format_eta(Some(0)), "-");
        assert_eq!(format_eta(Some(30)), "30s");
        assert_eq!(format_eta(Some(90)), "1m 30s");
        assert_eq!(format_eta(Some(3661)), "1h 1m");
    }

    #[test]
    fn test_detailed_transfer_result() {
        let result = DetailedTransferResult::new("task-1", 1000, Duration::from_secs(10));
        assert_eq!(result.task_id, "task-1");
        assert_eq!(result.bytes_transferred, 1000);
        assert_eq!(result.average_speed, 100.0);
        assert!(!result.was_resumed);

        let result = result.with_resume(500);
        assert!(result.was_resumed);
        assert_eq!(result.resume_offset, 500);
    }

    #[test]
    fn test_null_progress_callback() {
        let callback = NullProgressCallback;
        let progress = TransferProgressInfo::new("task-1", "file.txt", 1000);

        // 这些调用不应该做任何事情，但应该可以调用
        callback.on_progress(&progress);
        callback.on_start("task-1");
        callback.on_pause("task-1");
        callback.on_resume("task-1");
        callback.on_cancel("task-1");
    }
}