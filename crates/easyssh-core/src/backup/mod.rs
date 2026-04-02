//! EasySSH Backup System
//!
//! Enterprise-grade backup solution with:
//! - File backup (remote to local/cloud)
//! - Database backup (MySQL/PostgreSQL)
//! - Incremental backup with deduplication
//! - Cron-style scheduled backups
//! - Compression and encryption
//! - Version management
//! - Backup verification
//! - Multi-location backup (local, S3, GCS, Azure)
//! - One-click restore
//! - Backup reports

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

pub mod compression;
pub mod database;
pub mod engine;
pub mod incremental;
pub mod remote;
pub mod report;
pub mod restore;
pub mod scheduler;
pub mod storage;
pub mod verification;

pub use compression::{compress_backup, decompress_backup, decrypt_backup, encrypt_backup};
// CompressionFormat is defined and exported from this module directly
pub use database::{backup_mysql, backup_postgresql, DatabaseBackupConfig};
pub use engine::{BackupEngine, BackupEngineConfig, BackupJob, BackupJobBuilder};
pub use incremental::{FileHash, IncrementalBackupManager, IncrementalIndex};
pub use remote::{RemoteBackupConfig, RemoteBackupSource, RemoteFileBackup};
pub use report::{BackupMetrics, BackupReport, BackupReportGenerator, NotificationChannel};
pub use restore::{RestoreManager, RestoreOptions, RestorePoint};
pub use scheduler::{BackupScheduler, CronSchedule, ScheduleConfig};
pub use storage::{BackupStorage, LocalStorage, StorageBackend, StorageCredentials};
pub use verification::{verify_backup_integrity, verify_backup_restorable, BackupVerifier};

/// Unique identifier for backup jobs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BackupJobId(pub Uuid);

impl BackupJobId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for BackupJobId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BackupJobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for backup snapshots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(pub Uuid);

impl SnapshotId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SnapshotId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Backup types supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupType {
    /// Full backup of all files
    Full,
    /// Incremental backup (only changed files)
    Incremental,
    /// Differential backup (changes since last full)
    Differential,
    /// Database backup only
    Database,
    /// System state backup
    SystemState,
}

/// Backup status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupStatus {
    /// Backup is queued
    Queued,
    /// Backup is running
    Running,
    /// Backup completed successfully
    Completed,
    /// Backup failed
    Failed,
    /// Backup is being verified
    Verifying,
    /// Backup is being uploaded
    Uploading,
    /// Backup is paused
    Paused,
    /// Backup was cancelled
    Cancelled,
}

/// Backup target types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupTarget {
    /// Local filesystem
    Local { path: PathBuf },
    /// Remote SSH server
    Remote {
        host: String,
        port: u16,
        username: String,
        path: String,
    },
    /// AWS S3
    S3 {
        bucket: String,
        prefix: String,
        region: String,
    },
    /// Google Cloud Storage
    Gcs { bucket: String, prefix: String },
    /// Azure Blob Storage
    Azure {
        account: String,
        container: String,
        prefix: String,
    },
}

/// Backup source types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupSource {
    /// Local directory
    Local { path: PathBuf },
    /// Remote directory via SSH
    Remote {
        host: String,
        port: u16,
        username: String,
        path: String,
    },
    /// Database connection
    Database {
        db_type: DatabaseType,
        connection_string: String,
    },
}

/// Database types supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseType {
    MySQL,
    PostgreSQL,
    MongoDB,
    Redis,
    SQLite,
}

/// Retention policy for backup versions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Keep daily backups for N days
    pub daily: u32,
    /// Keep weekly backups for N weeks
    pub weekly: u32,
    /// Keep monthly backups for N months
    pub monthly: u32,
    /// Keep yearly backups for N years
    pub yearly: u32,
    /// Maximum total snapshots to keep
    pub max_snapshots: u32,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            daily: 7,
            weekly: 4,
            monthly: 12,
            yearly: 3,
            max_snapshots: 100,
        }
    }
}

/// Encryption settings for backups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionSettings {
    /// Enable encryption
    pub enabled: bool,
    /// Encryption algorithm
    pub algorithm: EncryptionAlgorithm,
    /// Key derivation method
    pub key_derivation: KeyDerivationMethod,
    /// Key identifier (for key management)
    pub key_id: Option<String>,
}

impl Default for EncryptionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_derivation: KeyDerivationMethod::Argon2id,
            key_id: None,
        }
    }
}

/// Encryption algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
}

/// Key derivation methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyDerivationMethod {
    Argon2id,
    Pbkdf2,
}

/// Compression settings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionSettings {
    /// Enable compression
    pub enabled: bool,
    /// Compression format
    pub format: CompressionFormat,
    /// Compression level (1-9)
    pub level: u32,
}

impl Default for CompressionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            format: CompressionFormat::Zstd,
            level: 3,
        }
    }
}

/// Compression formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionFormat {
    Gzip,
    Bzip2,
    Zstd,
    Xz,
    Zip,
    Tar,
}

/// Backup snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSnapshot {
    pub id: SnapshotId,
    pub job_id: BackupJobId,
    pub backup_type: BackupType,
    pub status: BackupStatus,
    pub source: BackupSource,
    pub target: BackupTarget,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub size_bytes: u64,
    pub compressed_size_bytes: u64,
    pub file_count: u64,
    pub checksum: String,
    pub parent_snapshot: Option<SnapshotId>,
    pub encryption_enabled: bool,
    pub compression_enabled: bool,
    pub metadata: HashMap<String, String>,
    pub error_message: Option<String>,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Backup job name
    pub name: String,
    /// Backup description
    pub description: Option<String>,
    /// Source to backup
    pub source: BackupSource,
    /// Target location(s)
    pub targets: Vec<BackupTarget>,
    /// Backup type
    pub backup_type: BackupType,
    /// Schedule configuration
    pub schedule: Option<ScheduleConfig>,
    /// Retention policy
    pub retention: RetentionPolicy,
    /// Compression settings
    pub compression: CompressionSettings,
    /// Encryption settings
    pub encryption: EncryptionSettings,
    /// Pre-backup script
    pub pre_backup_script: Option<String>,
    /// Post-backup script
    pub post_backup_script: Option<String>,
    /// Exclusion patterns
    pub exclusions: Vec<String>,
    /// Include patterns (if specified, only these are backed up)
    pub inclusions: Vec<String>,
    /// Enable backup verification
    pub verify_backup: bool,
    /// Enable notification on failure
    pub notify_on_failure: bool,
    /// Enable notification on success
    pub notify_on_success: bool,
    /// Maximum bandwidth (bytes/sec, 0 = unlimited)
    pub bandwidth_limit: u64,
    /// Parallel upload threads
    pub parallel_uploads: u32,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            name: "Untitled Backup".to_string(),
            description: None,
            source: BackupSource::Local {
                path: PathBuf::from("/"),
            },
            targets: vec![BackupTarget::Local {
                path: PathBuf::from("/backups"),
            }],
            backup_type: BackupType::Incremental,
            schedule: None,
            retention: RetentionPolicy::default(),
            compression: CompressionSettings::default(),
            encryption: EncryptionSettings::default(),
            pre_backup_script: None,
            post_backup_script: None,
            exclusions: vec![],
            inclusions: vec![],
            verify_backup: true,
            notify_on_failure: true,
            notify_on_success: false,
            bandwidth_limit: 0,
            parallel_uploads: 4,
        }
    }
}

/// Backup errors
#[derive(Error, Debug)]
pub enum BackupError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("SSH error: {0}")]
    Ssh(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Verification error: {0}")]
    Verification(String),

    #[error("Schedule error: {0}")]
    Schedule(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Job not found: {0}")]
    JobNotFound(BackupJobId),

    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(SnapshotId),

    #[error("Restore error: {0}")]
    Restore(String),

    #[error("Cloud error: {0}")]
    Cloud(String),

    #[error("Backup cancelled")]
    Cancelled,

    #[error("Backup already running")]
    AlreadyRunning,

    #[error("Invalid backup configuration: {0}")]
    InvalidConfiguration(String),
}

/// Result type for backup operations
pub type BackupResult<T> = Result<T, BackupError>;

/// Backup progress callback
pub type ProgressCallback = Box<dyn Fn(BackupProgress) + Send + Sync>;

/// Backup progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupProgress {
    pub job_id: BackupJobId,
    pub snapshot_id: SnapshotId,
    pub status: BackupStatus,
    pub current_file: Option<String>,
    pub files_processed: u64,
    pub files_total: u64,
    pub bytes_processed: u64,
    pub bytes_total: u64,
    pub percent_complete: f64,
    pub estimated_seconds_remaining: Option<u64>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Backup statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupStats {
    pub total_jobs: u32,
    pub total_snapshots: u64,
    pub total_size_bytes: u64,
    pub compressed_size_bytes: u64,
    pub successful_backups: u64,
    pub failed_backups: u64,
    pub last_backup_time: Option<DateTime<Utc>>,
    pub next_scheduled_backup: Option<DateTime<Utc>>,
}

/// Cloud provider credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudCredentials {
    Aws {
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
    },
    Gcp {
        service_account_key: String,
    },
    Azure {
        account_name: String,
        account_key: String,
    },
}

/// Bandwidth limit settings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BandwidthLimit {
    /// Maximum bytes per second
    pub bytes_per_second: u64,
    /// Time window for rate limiting
    pub window_seconds: u64,
}

impl Default for BandwidthLimit {
    fn default() -> Self {
        Self {
            bytes_per_second: 0, // Unlimited
            window_seconds: 1,
        }
    }
}

/// Backup filter for file selection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupFilter {
    /// Include patterns (glob syntax)
    pub include_patterns: Vec<String>,
    /// Exclude patterns (glob syntax)
    pub exclude_patterns: Vec<String>,
    /// Minimum file size (bytes)
    pub min_size: Option<u64>,
    /// Maximum file size (bytes)
    pub max_size: Option<u64>,
    /// Only backup files modified after
    pub modified_after: Option<DateTime<Utc>>,
    /// Only backup files modified before
    pub modified_before: Option<DateTime<Utc>>,
}

impl BackupFilter {
    /// Check if a file should be included based on filters
    pub fn should_include(&self, path: &std::path::Path, metadata: &std::fs::Metadata) -> bool {
        // Check size filters
        if let Some(min_size) = self.min_size {
            if metadata.len() < min_size {
                return false;
            }
        }
        if let Some(max_size) = self.max_size {
            if metadata.len() > max_size {
                return false;
            }
        }

        // Check modification time filters
        if let Ok(modified) = metadata.modified() {
            let modified: DateTime<Utc> = modified.into();
            if let Some(after) = self.modified_after {
                if modified < after {
                    return false;
                }
            }
            if let Some(before) = self.modified_before {
                if modified > before {
                    return false;
                }
            }
        }

        // Check exclusion patterns
        let path_str = path.to_string_lossy();
        for pattern in &self.exclude_patterns {
            if Self::matches_glob(&path_str, pattern) {
                return false;
            }
        }

        // If include patterns are specified, file must match one
        if !self.include_patterns.is_empty() {
            let mut included = false;
            for pattern in &self.include_patterns {
                if Self::matches_glob(&path_str, pattern) {
                    included = true;
                    break;
                }
            }
            return included;
        }

        true
    }

    /// Simple glob matching
    fn matches_glob(path: &str, pattern: &str) -> bool {
        // Convert glob pattern to regex
        let pattern = pattern
            .replace("**", "|||DOUBLESTAR|||")
            .replace("*", "[^/]*")
            .replace("?", "[^/]")
            .replace("|||DOUBLESTAR|||", ".*");

        if let Ok(regex) = regex::Regex::new(&pattern) {
            regex.is_match(path)
        } else {
            path.contains(
                &pattern
                    .replace(".*", "")
                    .replace("[^/]*", "")
                    .replace("[^/]", ""),
            )
        }
    }
}

/// Helper function to format bytes
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Helper function to calculate duration in human-readable format
pub fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512.00 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(125), "2m 5s");
        assert_eq!(format_duration(3665), "1h 1m 5s");
    }

    #[test]
    fn test_backup_filter_glob() {
        let filter = BackupFilter {
            exclude_patterns: vec!["*.tmp".to_string(), "*.log".to_string()],
            ..Default::default()
        };

        assert!(BackupFilter::matches_glob("/path/file.tmp", "*.tmp"));
        assert!(BackupFilter::matches_glob("/path/file.log", "*.log"));
        assert!(!BackupFilter::matches_glob("/path/file.txt", "*.tmp"));
    }
}
