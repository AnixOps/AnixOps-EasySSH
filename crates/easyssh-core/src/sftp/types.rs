//! SFTP核心数据类型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// 格式化文件大小
pub fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    const THRESHOLD: f64 = 1024.0;

    if size == 0 {
        return "0 B".to_string();
    }

    let size_f = size as f64;
    let exp = (size_f.ln() / THRESHOLD.ln()).min(UNITS.len() as f64 - 1.0) as usize;
    let value = size_f / THRESHOLD.powi(exp as i32);

    if exp == 0 {
        format!("{} {}", size, UNITS[exp])
    } else {
        format!("{:.2} {}", value, UNITS[exp])
    }
}

/// 格式化持续时间
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// 文件信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileInfo {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub modified: DateTime<Utc>,
    pub permissions: u32,
    pub file_type: FileType,
}

impl FileInfo {
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            size: 0,
            is_dir: false,
            modified: Utc::now(),
            permissions: 0o644,
            file_type: FileType::File,
        }
    }

    pub fn formatted_size(&self) -> String {
        if self.is_dir {
            "-".to_string()
        } else {
            format_size(self.size)
        }
    }
}

/// 文件类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
    Symlink,
    Special,
    Unknown,
}

/// 传输任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferTask {
    pub id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub direction: TransferDirection,
    pub status: TransferStatus,
    pub progress: f64,
    pub transferred_bytes: u64,
    pub total_bytes: u64,
    pub created_at: DateTime<Utc>,
    pub options: TransferOptions,
    pub client_id: String,
}

impl TransferTask {
    pub fn new(
        source: impl Into<PathBuf>,
        destination: impl Into<PathBuf>,
        direction: TransferDirection,
        client_id: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source: source.into(),
            destination: destination.into(),
            direction,
            status: TransferStatus::Pending,
            progress: 0.0,
            transferred_bytes: 0,
            total_bytes: 0,
            created_at: Utc::now(),
            options: TransferOptions::default(),
            client_id: client_id.into(),
        }
    }

    pub fn with_options(mut self, options: TransferOptions) -> Self {
        self.options = options;
        self
    }

    pub fn with_total_bytes(mut self, total: u64) -> Self {
        self.total_bytes = total;
        self
    }

    pub fn start(&mut self) {
        self.status = TransferStatus::Transferring;
    }

    pub fn update_progress(&mut self, transferred: u64) {
        self.transferred_bytes = transferred;
        if self.total_bytes > 0 {
            self.progress = (transferred as f64 / self.total_bytes as f64 * 100.0).min(100.0);
        }
    }

    pub fn complete(&mut self) {
        self.status = TransferStatus::Completed;
        self.progress = 100.0;
        self.transferred_bytes = self.total_bytes;
    }

    pub fn fail(&mut self, _error: impl Into<String>) {
        self.status = TransferStatus::Failed;
    }

    pub fn pause(&mut self) {
        if self.status == TransferStatus::Transferring {
            self.status = TransferStatus::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.status == TransferStatus::Paused {
            self.status = TransferStatus::Transferring;
        }
    }

    pub fn cancel(&mut self) {
        self.status = TransferStatus::Cancelled;
    }

    pub fn can_pause(&self) -> bool {
        self.status == TransferStatus::Transferring
    }

    pub fn can_resume(&self) -> bool {
        self.status == TransferStatus::Paused
    }

    pub fn can_cancel(&self) -> bool {
        !self.status.is_done()
    }

    pub fn speed_bps(&self) -> f64 {
        0.0
    }
}

/// 传输方向
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferDirection {
    Upload,
    Download,
}

/// 传输状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferStatus {
    Pending,
    Transferring,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Verifying,
}

impl TransferStatus {
    pub fn is_done(&self) -> bool {
        matches!(
            self,
            TransferStatus::Completed | TransferStatus::Failed | TransferStatus::Cancelled
        )
    }
}

/// 传输选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferOptions {
    pub chunk_size: usize,
    pub resume: bool,
    pub max_concurrent: usize,
    pub speed_limit: u64,
    pub overwrite: bool,
    pub preserve_time: bool,
    pub preserve_permissions: bool,
    pub file_mode: u32,
    pub timeout: Duration,
    pub retry_count: u32,
    pub verify_checksum: bool,
}

impl Default for TransferOptions {
    fn default() -> Self {
        Self {
            chunk_size: 64 * 1024,
            resume: true,
            max_concurrent: 3,
            speed_limit: 0,
            overwrite: true,
            preserve_time: true,
            preserve_permissions: true,
            file_mode: 0o644,
            timeout: Duration::from_secs(30),
            retry_count: 3,
            verify_checksum: false,
        }
    }
}

/// 传输结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    pub task_id: String,
    pub bytes_transferred: u64,
    pub duration: Duration,
    pub average_speed: f64,
    pub was_resumed: bool,
    pub checksum: Option<String>,
}

impl TransferResult {
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
            checksum: None,
        }
    }

    pub fn formatted_speed(&self) -> String {
        format_size(self.average_speed as u64) + "/s"
    }
}

/// 传输统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransferStats {
    pub total_tasks: usize,
    pub completed: usize,
    pub failed: usize,
    pub active: usize,
    pub pending: usize,
    pub total_bytes: u64,
    pub current_speed: f64,
    pub eta_seconds: Option<f64>,
}

/// 文件权限
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePermission(pub u32);

impl FilePermission {
    pub fn new(mode: u32) -> Self {
        Self(mode & 0o777)
    }

    pub fn mode(&self) -> u32 {
        self.0
    }
}

/// SFTP条目信息 (用于兼容旧API)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SftpEntry {
    pub name: String,
    pub path: String,
    pub file_type: String,
    pub size: i64,
    pub mtime: i64,
    pub permissions: Option<u32>,
}

impl From<FileInfo> for SftpEntry {
    fn from(info: FileInfo) -> Self {
        let file_type = match info.file_type {
            FileType::File => "file",
            FileType::Directory => "directory",
            FileType::Symlink => "symlink",
            FileType::Special => "special",
            FileType::Unknown => "unknown",
        }
        .to_string();

        SftpEntry {
            name: info.name,
            path: info.path.to_string_lossy().to_string(),
            file_type,
            size: info.size as i64,
            mtime: info.modified.timestamp(),
            permissions: Some(info.permissions),
        }
    }
}

impl SftpEntry {
    /// 格式化文件大小显示
    pub fn size_display(&self) -> String {
        format_size(self.size as u64)
    }

    /// 格式化修改时间显示
    pub fn mtime_display(&self) -> String {
        use chrono::DateTime;
        let datetime = DateTime::from_timestamp(self.mtime, 0)
            .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());
        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}
