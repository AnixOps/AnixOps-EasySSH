//! Remote file backup over SSH

use super::{
    BackupError, BackupFilter, BackupResult, BackupSource, BackupTarget, CloudCredentials,
};
use crate::ssh::SshSessionManager;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info, warn};

/// Remote backup source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteBackupConfig {
    /// SSH host
    pub host: String,
    /// SSH port
    pub port: u16,
    /// SSH username
    pub username: String,
    /// SSH authentication type
    pub auth_type: RemoteAuthType,
    /// Source path on remote
    pub source_path: PathBuf,
    /// Bandwidth limit (bytes/sec)
    pub bandwidth_limit: u64,
    /// Connection timeout (seconds)
    pub timeout_seconds: u64,
    /// Pre-backup script on remote
    pub pre_backup_script: Option<String>,
    /// Post-backup script on remote
    pub post_backup_script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemoteAuthType {
    Password(String),
    PrivateKey {
        key_path: PathBuf,
        passphrase: Option<String>,
    },
    SshAgent,
}

/// Remote backup source trait
#[async_trait]
pub trait RemoteBackupSource: Send + Sync {
    /// Get file list from remote
    async fn list_files(
        &self,
        path: &Path,
        recursive: bool,
        filter: &BackupFilter,
    ) -> BackupResult<Vec<RemoteFileInfo>>;

    /// Download a file
    async fn download_file(
        &self,
        remote_path: &Path,
        local_path: &Path,
        progress_callback: Option<Box<dyn Fn(u64, u64) + Send + Sync>>,
    ) -> BackupResult<u64>;

    /// Execute command on remote
    async fn execute(&self, command: &str) -> BackupResult<String>;

    /// Test connection
    async fn test_connection(&self) -> BackupResult<()>;
}

/// Remote file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFileInfo {
    /// Relative path
    pub path: PathBuf,
    /// File size
    pub size: u64,
    /// Modified time
    pub modified_time: DateTime<Utc>,
    /// Is directory
    pub is_directory: bool,
    /// Permissions
    pub permissions: u32,
    /// Checksum (if available)
    pub checksum: Option<String>,
}

/// SSH-based remote backup
pub struct SshRemoteBackup {
    config: RemoteBackupConfig,
    session_manager: SshSessionManager,
    session_id: String,
}

impl SshRemoteBackup {
    /// Create a new SSH remote backup
    pub async fn new(config: RemoteBackupConfig) -> BackupResult<Self> {
        let mut session_manager = SshSessionManager::new();
        let session_id = uuid::Uuid::new_v4().to_string();

        // Connect via SSH
        let password = match &config.auth_type {
            RemoteAuthType::Password(pwd) => Some(pwd.as_str()),
            _ => None,
        };

        session_manager
            .connect(
                &session_id,
                &config.host,
                config.port,
                &config.username,
                password,
            )
            .await
            .map_err(|e| BackupError::Ssh(e.to_string()))?;

        Ok(Self {
            config,
            session_manager,
            session_id,
        })
    }

    /// Disconnect
    pub async fn disconnect(&mut self) -> BackupResult<()> {
        self.session_manager
            .disconnect(&self.session_id)
            .await
            .map_err(|e| BackupError::Ssh(e.to_string()))
    }

    /// Run pre-backup script
    async fn run_pre_backup(&self) -> BackupResult<()> {
        if let Some(script) = &self.config.pre_backup_script {
            info!("Running pre-backup script");
            let output = self.execute(script).await?;
            if !output.is_empty() {
                info!("Pre-backup output: {}", output);
            }
        }
        Ok(())
    }

    /// Run post-backup script
    async fn run_post_backup(&self) -> BackupResult<()> {
        if let Some(script) = &self.config.post_backup_script {
            info!("Running post-backup script");
            let output = self.execute(script).await?;
            if !output.is_empty() {
                info!("Post-backup output: {}", output);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl RemoteBackupSource for SshRemoteBackup {
    async fn list_files(
        &self,
        path: &Path,
        recursive: bool,
        _filter: &BackupFilter,
    ) -> BackupResult<Vec<RemoteFileInfo>> {
        let find_cmd = if recursive {
            format!(
                "find '{}' -type f -printf '%p|%s|%T@|%m\\n' 2>/dev/null",
                path.display()
            )
        } else {
            format!(
                "find '{}' -maxdepth 1 -type f -printf '%p|%s|%T@|%m\\n' 2>/dev/null",
                path.display()
            )
        };

        let output = self.execute(&find_cmd).await?;
        let mut files = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                let file_path = PathBuf::from(parts[0]);
                let size = parts[1].parse::<u64>().unwrap_or(0);
                let mtime = parts[2].parse::<f64>().unwrap_or(0.0) as i64;
                let perms = u32::from_str_radix(parts[3], 8).unwrap_or(0o644);

                let modified_time = DateTime::from_timestamp(mtime, 0).unwrap_or_else(Utc::now);

                files.push(RemoteFileInfo {
                    path: file_path,
                    size,
                    modified_time,
                    is_directory: false,
                    permissions: perms,
                    checksum: None,
                });
            }
        }

        Ok(files)
    }

    async fn download_file(
        &self,
        remote_path: &Path,
        local_path: &Path,
        _progress_callback: Option<Box<dyn Fn(u64, u64) + Send + Sync>>,
    ) -> BackupResult<u64> {
        // Create parent directory
        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(BackupError::Io)?;
        }

        // Use SCP or SFTP to download file
        // For now, use cat and stream
        let cmd = format!("cat '{}' 2>/dev/null", remote_path.display());
        let data = self.execute(&cmd).await?;

        tokio::fs::write(local_path, data.as_bytes())
            .await
            .map_err(BackupError::Io)?;

        Ok(data.len() as u64)
    }

    async fn execute(&self, command: &str) -> BackupResult<String> {
        self.session_manager
            .execute(&self.session_id, command)
            .await
            .map_err(|e| BackupError::Ssh(e.to_string()))
    }

    async fn test_connection(&self) -> BackupResult<()> {
        self.execute("echo 'Connection test'").await?;
        Ok(())
    }
}

/// SFTP-based remote backup (more efficient for file transfers)
pub struct SftpRemoteBackup {
    config: RemoteBackupConfig,
    session_manager: SshSessionManager,
    session_id: String,
}

impl SftpRemoteBackup {
    /// Create a new SFTP remote backup
    pub async fn new(config: RemoteBackupConfig) -> BackupResult<Self> {
        let mut session_manager = SshSessionManager::new();
        let session_id = uuid::Uuid::new_v4().to_string();

        let password = match &config.auth_type {
            RemoteAuthType::Password(pwd) => Some(pwd.as_str()),
            _ => None,
        };

        session_manager
            .connect(
                &session_id,
                &config.host,
                config.port,
                &config.username,
                password,
            )
            .await
            .map_err(|e| BackupError::Ssh(e.to_string()))?;

        Ok(Self {
            config,
            session_manager,
            session_id,
        })
    }

    /// Create SFTP session
    async fn create_sftp(&self) -> BackupResult<ssh2::Sftp> {
        self.session_manager
            .create_sftp(&self.session_id)
            .await
            .map_err(|e| BackupError::Ssh(e.to_string()))
    }
}

#[async_trait]
impl RemoteBackupSource for SftpRemoteBackup {
    async fn list_files(
        &self,
        path: &Path,
        recursive: bool,
        filter: &BackupFilter,
    ) -> BackupResult<Vec<RemoteFileInfo>> {
        let sftp = self.create_sftp().await?;
        let mut files = Vec::new();

        fn collect_files(
            sftp: &ssh2::Sftp,
            base_path: &Path,
            current_path: &Path,
            recursive: bool,
            filter: &BackupFilter,
            files: &mut Vec<RemoteFileInfo>,
        ) -> BackupResult<()> {
            let full_path = base_path.join(current_path);

            let readdir = sftp
                .readdir(&full_path)
                .map_err(|e| BackupError::Ssh(e.to_string()))?;

            for (entry_path, stat) in readdir {
                let relative_path = entry_path
                    .strip_prefix(base_path)
                    .unwrap_or(&entry_path)
                    .to_path_buf();

                let is_directory = stat.is_dir();

                if is_directory && recursive {
                    collect_files(sftp, base_path, &relative_path, recursive, filter, files)?;
                } else if !is_directory {
                    // Check filter
                    let modified: DateTime<Utc> =
                        DateTime::from_timestamp(stat.mtime.unwrap_or(0) as i64, 0)
                            .unwrap_or_else(Utc::now);

                    // Create metadata-like struct for filtering
                    let size = stat.size.unwrap_or(0);

                    // Simple size-based filtering
                    let mut include = true;
                    if filter.min_size.is_some() && size < filter.min_size.unwrap() {
                        include = false;
                    }
                    if filter.max_size.is_some() && size > filter.max_size.unwrap() {
                        include = false;
                    }

                    if include {
                        files.push(RemoteFileInfo {
                            path: relative_path,
                            size,
                            modified_time: modified,
                            is_directory: false,
                            permissions: stat.perm.unwrap_or(0o644) as u32,
                            checksum: None,
                        });
                    }
                }
            }

            Ok(())
        }

        // This is blocking, in production use spawn_blocking
        collect_files(&sftp, path, Path::new(""), recursive, filter, &mut files)?;

        Ok(files)
    }

    async fn download_file(
        &self,
        remote_path: &Path,
        local_path: &Path,
        progress_callback: Option<Box<dyn Fn(u64, u64) + Send + Sync>>,
    ) -> BackupResult<u64> {
        // Create parent directory
        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(BackupError::Io)?;
        }

        let sftp = self.create_sftp().await?;

        // Open remote file
        let mut remote_file = sftp
            .open(remote_path)
            .map_err(|e| BackupError::Ssh(e.to_string()))?;

        // Create local file
        let mut local_file = tokio::fs::File::create(local_path)
            .await
            .map_err(BackupError::Io)?;

        // Get file size for progress
        let metadata = remote_file
            .stat()
            .map_err(|e| BackupError::Ssh(e.to_string()))?;
        let total_size = metadata.size.unwrap_or(0);

        // Transfer with optional progress callback
        let mut buffer = vec![0u8; 65536];
        let mut downloaded = 0u64;

        // In production, use async SFTP implementation
        use std::io::Read;
        loop {
            match remote_file.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    local_file
                        .write_all(&buffer[..n])
                        .await
                        .map_err(BackupError::Io)?;
                    downloaded += n as u64;

                    if let Some(ref callback) = progress_callback {
                        callback(downloaded, total_size);
                    }

                    // Apply bandwidth limiting
                    if self.config.bandwidth_limit > 0 {
                        let delay_ms = (n as u64 * 1000) / self.config.bandwidth_limit;
                        if delay_ms > 0 {
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        }
                    }
                }
                Err(e) => return Err(BackupError::Io(e)),
            }
        }

        local_file.flush().await.map_err(BackupError::Io)?;

        Ok(downloaded)
    }

    async fn execute(&self, command: &str) -> BackupResult<String> {
        self.session_manager
            .execute(&self.session_id, command)
            .await
            .map_err(|e| BackupError::Ssh(e.to_string()))
    }

    async fn test_connection(&self) -> BackupResult<()> {
        self.execute("echo 'Connection test'").await?;
        Ok(())
    }
}

/// Remote backup orchestrator
pub struct RemoteFileBackup {
    source: Box<dyn RemoteBackupSource>,
    config: RemoteBackupConfig,
}

impl RemoteFileBackup {
    /// Create a new remote file backup
    pub async fn new_ssh(config: RemoteBackupConfig) -> BackupResult<Self> {
        let source = Box::new(SftpRemoteBackup::new(config.clone()).await?);
        Ok(Self { source, config })
    }

    /// Perform backup
    pub async fn backup(
        &self,
        filter: &BackupFilter,
        local_destination: &Path,
        progress_callback: Option<Box<dyn Fn(u64, u64, &str) + Send + Sync>>,
    ) -> BackupResult<RemoteBackupResult> {
        let start_time = std::time::Instant::now();

        // List files
        info!("Listing files from remote...");
        let files = self
            .source
            .list_files(&self.config.source_path, true, filter)
            .await?;
        let total_files = files.len() as u64;
        let total_size: u64 = files.iter().map(|f| f.size).sum();

        info!(
            "Found {} files, total size: {} bytes",
            total_files, total_size
        );

        // Download files
        let mut downloaded_size = 0u64;
        let mut downloaded_count = 0u64;
        let mut errors = Vec::new();

        for (i, file) in files.iter().enumerate() {
            let remote_path = self.config.source_path.join(&file.path);
            let local_path = local_destination.join(&file.path);

            if let Some(ref callback) = progress_callback {
                callback(
                    downloaded_count,
                    total_files,
                    &format!("Downloading {}...", file.path.display()),
                );
            }

            // Simplified download without progress callback in the closure
            // to avoid borrow checker issues
            match self
                .source
                .download_file(
                    &remote_path,
                    &local_path,
                    None::<Box<dyn Fn(u64, u64) + Send + Sync>>,
                )
                .await
            {
                Ok(size) => {
                    downloaded_size += size;
                    downloaded_count += 1;
                }
                Err(e) => {
                    warn!("Failed to download {}: {}", file.path.display(), e);
                    errors.push(format!("{}: {}", file.path.display(), e));
                }
            }
        }

        let duration = start_time.elapsed().as_secs_f64();

        info!(
            "Remote backup completed: {}/{} files, {} bytes in {:.2}s",
            downloaded_count, total_files, downloaded_size, duration
        );

        Ok(RemoteBackupResult {
            source_host: self.config.host.clone(),
            source_path: self.config.source_path.clone(),
            destination_path: local_destination.to_path_buf(),
            total_files,
            downloaded_files: downloaded_count,
            skipped_files: total_files - downloaded_count,
            total_size_bytes: total_size,
            downloaded_size_bytes: downloaded_size,
            duration_seconds: duration,
            errors,
        })
    }
}

/// Remote backup result
#[derive(Debug, Clone)]
pub struct RemoteBackupResult {
    pub source_host: String,
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub total_files: u64,
    pub downloaded_files: u64,
    pub skipped_files: u64,
    pub total_size_bytes: u64,
    pub downloaded_size_bytes: u64,
    pub duration_seconds: f64,
    pub errors: Vec<String>,
}

/// Rsync-style remote backup (uses rsync if available)
pub struct RsyncRemoteBackup;

impl RsyncRemoteBackup {
    /// Check if rsync is available
    pub async fn is_available() -> bool {
        let cmd = if cfg!(target_os = "windows") {
            Command::new("where").arg("rsync").output().await
        } else {
            Command::new("which").arg("rsync").output().await
        };

        matches!(cmd, Ok(output) if output.status.success())
    }

    /// Perform rsync backup
    pub async fn backup(
        config: &RemoteBackupConfig,
        destination: &Path,
        filter: &BackupFilter,
    ) -> BackupResult<RemoteBackupResult> {
        let start_time = std::time::Instant::now();

        let mut cmd = Command::new("rsync");

        // Archive mode, compress, partial transfers
        cmd.arg("-avzP");

        // Add bandwidth limit if specified
        if config.bandwidth_limit > 0 {
            let kbps = config.bandwidth_limit / 1024;
            cmd.arg("--bwlimit").arg(kbps.to_string());
        }

        // Add include/exclude patterns
        for pattern in &filter.exclude_patterns {
            cmd.arg("--exclude").arg(pattern);
        }

        for pattern in &filter.include_patterns {
            cmd.arg("--include").arg(pattern);
        }

        // Build source path
        let source = format!(
            "{}@{}:{}",
            config.username,
            config.host,
            config.source_path.display()
        );

        cmd.arg(source).arg(destination);

        let output = cmd.output().await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BackupError::Storage("rsync not found. Please install rsync.".to_string())
            } else {
                BackupError::Io(e)
            }
        })?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        let errors: Vec<String> = stderr
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with("sending incremental"))
            .map(|l| l.to_string())
            .collect();

        if !output.status.success() && errors.is_empty() {
            return Err(BackupError::Storage(format!(
                "rsync failed: {}",
                output.status
            )));
        }

        // Parse rsync output for statistics
        let (files_transferred, bytes_transferred) = Self::parse_rsync_output(&stdout);

        let duration = start_time.elapsed().as_secs_f64();

        Ok(RemoteBackupResult {
            source_host: config.host.clone(),
            source_path: config.source_path.clone(),
            destination_path: destination.to_path_buf(),
            total_files: files_transferred,
            downloaded_files: files_transferred,
            skipped_files: 0,
            total_size_bytes: bytes_transferred,
            downloaded_size_bytes: bytes_transferred,
            duration_seconds: duration,
            errors,
        })
    }

    fn parse_rsync_output(output: &str) -> (u64, u64) {
        // Parse rsync statistics
        let mut files = 0u64;
        let mut bytes = 0u64;

        for line in output.lines() {
            // Look for "sent X bytes  received Y bytes"
            if line.contains("sent") && line.contains("received") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if *part == "sent" && i + 1 < parts.len() {
                        if let Ok(n) = parts[i + 1].replace(",", "").parse::<u64>() {
                            bytes = n;
                        }
                    }
                }
            }

            // Look for number of files
            if line.contains("files transferred") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(first) = parts.first() {
                    if let Ok(n) = first.parse::<u64>() {
                        files = n;
                    }
                }
            }
        }

        (files, bytes)
    }
}

use tokio::process::Command;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_backup_config() {
        let config = RemoteBackupConfig {
            host: "example.com".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_type: RemoteAuthType::Password("secret".to_string()),
            source_path: PathBuf::from("/var/www"),
            bandwidth_limit: 1024 * 1024, // 1 MB/s
            timeout_seconds: 60,
            pre_backup_script: None,
            post_backup_script: None,
        };

        assert_eq!(config.host, "example.com");
        assert_eq!(config.port, 22);
        assert_eq!(config.source_path, PathBuf::from("/var/www"));
    }

    #[test]
    fn test_remote_file_info() {
        let info = RemoteFileInfo {
            path: PathBuf::from("test.txt"),
            size: 1024,
            modified_time: Utc::now(),
            is_directory: false,
            permissions: 0o644,
            checksum: None,
        };

        assert_eq!(info.size, 1024);
        assert!(!info.is_directory);
    }
}
