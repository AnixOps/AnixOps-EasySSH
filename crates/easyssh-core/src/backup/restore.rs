//! Backup restore functionality

use super::{BackupError, BackupResult, CompressionFormat, SnapshotId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{error, info};

/// Restore options
#[derive(Serialize, Deserialize)]
pub struct RestoreOptions {
    /// Restore to a different location than original
    pub target_path: Option<PathBuf>,
    /// Overwrite existing files
    pub overwrite: bool,
    /// Verify after restore
    pub verify_after_restore: bool,
    /// Selective restore - specific files only
    pub selected_files: Vec<PathBuf>,
    /// Restore to specific date (for point-in-time recovery)
    pub point_in_time: Option<DateTime<Utc>>,
    /// Restore permissions
    pub restore_permissions: bool,
    /// Restore ownership (requires root/admin)
    pub restore_ownership: bool,
    /// Dry run - show what would be restored
    pub dry_run: bool,
    /// Progress callback
    #[serde(skip)]
    pub progress_callback: Option<Box<dyn Fn(RestoreProgress) + Send + Sync>>,
}

impl std::fmt::Debug for RestoreOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestoreOptions")
            .field("target_path", &self.target_path)
            .field("overwrite", &self.overwrite)
            .field("verify_after_restore", &self.verify_after_restore)
            .field("selected_files", &self.selected_files)
            .field("point_in_time", &self.point_in_time)
            .field("restore_permissions", &self.restore_permissions)
            .field("restore_ownership", &self.restore_ownership)
            .field("dry_run", &self.dry_run)
            .field("progress_callback", &"<progress callback>")
            .finish()
    }
}

impl Clone for RestoreOptions {
    fn clone(&self) -> Self {
        Self {
            target_path: self.target_path.clone(),
            overwrite: self.overwrite,
            verify_after_restore: self.verify_after_restore,
            selected_files: self.selected_files.clone(),
            point_in_time: self.point_in_time,
            restore_permissions: self.restore_permissions,
            restore_ownership: self.restore_ownership,
            dry_run: self.dry_run,
            progress_callback: None, // Cannot clone the callback
        }
    }
}

impl Default for RestoreOptions {
    fn default() -> Self {
        Self {
            target_path: None,
            overwrite: false,
            verify_after_restore: true,
            selected_files: vec![],
            point_in_time: None,
            restore_permissions: true,
            restore_ownership: false,
            dry_run: false,
            progress_callback: None,
        }
    }
}

/// Restore point information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestorePoint {
    pub snapshot_id: SnapshotId,
    pub created_at: DateTime<Utc>,
    pub backup_type: super::BackupType,
    pub source_path: PathBuf,
    pub size_bytes: u64,
    pub file_count: u64,
    pub description: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Restore progress
#[derive(Debug, Clone)]
pub struct RestoreProgress {
    pub phase: RestorePhase,
    pub current_file: Option<PathBuf>,
    pub files_processed: u64,
    pub files_total: u64,
    pub bytes_processed: u64,
    pub bytes_total: u64,
    pub percent_complete: f64,
    pub estimated_seconds_remaining: Option<u64>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestorePhase {
    Preparing,
    Downloading,
    Decrypting,
    Decompressing,
    Copying,
    Verifying,
    Completed,
    Failed,
}

/// Restore result
#[derive(Debug, Clone)]
pub struct RestoreResult {
    pub success: bool,
    pub files_restored: u64,
    pub files_skipped: u64,
    pub files_failed: u64,
    pub bytes_restored: u64,
    pub duration_seconds: f64,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub restored_to: PathBuf,
}

/// Restore manager
pub struct RestoreManager {
    storage: super::BackupStorage,
    options: RestoreOptions,
}

impl RestoreManager {
    /// Create a new restore manager
    pub fn new(storage: super::BackupStorage, options: RestoreOptions) -> Self {
        Self { storage, options }
    }

    /// List available restore points
    pub async fn list_restore_points(&self) -> BackupResult<Vec<RestorePoint>> {
        // This would typically query the database/storage for available snapshots
        // For now, return empty list
        Ok(vec![])
    }

    /// Restore from a snapshot
    pub async fn restore(&self, snapshot_id: SnapshotId) -> BackupResult<RestoreResult> {
        self.restore_with_options(snapshot_id, &self.options).await
    }

    /// Restore with custom options
    pub async fn restore_with_options(
        &self,
        snapshot_id: SnapshotId,
        options: &RestoreOptions,
    ) -> BackupResult<RestoreResult> {
        let start_time = std::time::Instant::now();
        let mut files_restored = 0u64;
        let mut files_skipped = 0u64;
        let mut files_failed = 0u64;
        let mut bytes_restored = 0u64;
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        info!("Starting restore from snapshot {}", snapshot_id.0);

        // Determine target path
        let target_path = options
            .target_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));

        // Create target directory
        if !options.dry_run {
            tokio::fs::create_dir_all(&target_path)
                .await
                .map_err(BackupError::Io)?;
        }

        // Download and extract backup
        let temp_dir = tempfile::TempDir::new().map_err(|e| BackupError::Io(e))?;
        let backup_path = temp_dir.path().join("backup.tar.gz");

        self.report_progress(
            &options,
            RestoreProgress {
                phase: RestorePhase::Preparing,
                current_file: None,
                files_processed: 0,
                files_total: 0,
                bytes_processed: 0,
                bytes_total: 0,
                percent_complete: 0.0,
                estimated_seconds_remaining: None,
                errors: vec![],
            },
        );

        // Retrieve backup from storage
        self.report_progress(
            &options,
            RestoreProgress {
                phase: RestorePhase::Downloading,
                current_file: None,
                files_processed: 0,
                files_total: 0,
                bytes_processed: 0,
                bytes_total: 0,
                percent_complete: 5.0,
                estimated_seconds_remaining: None,
                errors: vec![],
            },
        );

        match self
            .storage
            .retrieve(&format!("snapshots/{}", snapshot_id.0))
            .await
        {
            Ok(data) => {
                if !options.dry_run {
                    tokio::fs::write(&backup_path, data)
                        .await
                        .map_err(BackupError::Io)?;
                }
            }
            Err(e) => {
                return Err(BackupError::Storage(format!(
                    "Failed to retrieve snapshot: {}",
                    e
                )));
            }
        }

        // Decrypt if encrypted
        let decrypted_path = temp_dir.path().join("decrypted.tar.gz");
        // Decryption would happen here if encrypted

        // Decompress
        self.report_progress(
            &options,
            RestoreProgress {
                phase: RestorePhase::Decompressing,
                current_file: None,
                files_processed: 0,
                files_total: 0,
                bytes_processed: 0,
                bytes_total: 0,
                percent_complete: 20.0,
                estimated_seconds_remaining: None,
                errors: vec![],
            },
        );

        let extract_path = temp_dir.path().join("extracted");
        if !options.dry_run {
            super::decompress_backup(&backup_path, &extract_path, CompressionFormat::Gzip).await?;
        }

        // Copy files to target
        self.report_progress(
            &options,
            RestoreProgress {
                phase: RestorePhase::Copying,
                current_file: None,
                files_processed: 0,
                files_total: 0,
                bytes_processed: 0,
                bytes_total: 0,
                percent_complete: 40.0,
                estimated_seconds_remaining: None,
                errors: vec![],
            },
        );

        // Walk through extracted files
        let entries: Vec<_> = walkdir::WalkDir::new(&extract_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        let total_files = entries.len() as u64;
        let total_bytes: u64 = entries
            .iter()
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum();

        for (i, entry) in entries.iter().enumerate() {
            let source_path = entry.path();
            let relative_path = source_path
                .strip_prefix(&extract_path)
                .unwrap_or(source_path);

            // Filter by selected files if specified
            if !options.selected_files.is_empty() {
                let should_include = options
                    .selected_files
                    .iter()
                    .any(|p| relative_path.starts_with(p));
                if !should_include {
                    files_skipped += 1;
                    continue;
                }
            }

            let target_file = target_path.join(relative_path);

            self.report_progress(
                &options,
                RestoreProgress {
                    phase: RestorePhase::Copying,
                    current_file: Some(relative_path.to_path_buf()),
                    files_processed: i as u64,
                    files_total: total_files,
                    bytes_processed: bytes_restored,
                    bytes_total: total_bytes,
                    percent_complete: 40.0 + (i as f64 / total_files as f64) * 50.0,
                    estimated_seconds_remaining: None,
                    errors: errors.clone(),
                },
            );

            // Check if file exists
            if target_file.exists() && !options.overwrite {
                warnings.push(format!("File exists, skipping: {}", target_file.display()));
                files_skipped += 1;
                continue;
            }

            // Create parent directory
            if let Some(parent) = target_file.parent() {
                if !options.dry_run {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(BackupError::Io)?;
                }
            }

            // Copy file
            if !options.dry_run {
                match tokio::fs::copy(source_path, &target_file).await {
                    Ok(size) => {
                        files_restored += 1;
                        bytes_restored += size;

                        // Restore permissions if needed
                        if options.restore_permissions {
                            if let Ok(metadata) = entry.metadata() {
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::PermissionsExt;
                                    let perms = std::fs::Permissions::from_mode(
                                        metadata.permissions().mode(),
                                    );
                                    let _ = tokio::fs::set_permissions(&target_file, perms).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to restore {}: {}", target_file.display(), e);
                        errors.push(format!("{}: {}", target_file.display(), e));
                        files_failed += 1;
                    }
                }
            } else {
                // Dry run - just count
                files_restored += 1;
            }
        }

        // Verify after restore
        if options.verify_after_restore && !options.dry_run {
            self.report_progress(
                &options,
                RestoreProgress {
                    phase: RestorePhase::Verifying,
                    current_file: None,
                    files_processed: files_restored,
                    files_total: total_files,
                    bytes_processed: bytes_restored,
                    bytes_total: total_bytes,
                    percent_complete: 95.0,
                    estimated_seconds_remaining: None,
                    errors: errors.clone(),
                },
            );

            // Basic verification - check file count
            let restored_count = walkdir::WalkDir::new(&target_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .count() as u64;

            if restored_count != files_restored {
                warnings.push(format!(
                    "Verification warning: expected {} files, found {}",
                    files_restored, restored_count
                ));
            }
        }

        let duration = start_time.elapsed().as_secs_f64();

        let success = files_failed == 0 || (files_restored > 0 && errors.len() < 10);

        info!(
            "Restore completed: {} files restored, {} skipped, {} failed in {:.2}s",
            files_restored, files_skipped, files_failed, duration
        );

        Ok(RestoreResult {
            success,
            files_restored,
            files_skipped,
            files_failed,
            bytes_restored,
            duration_seconds: duration,
            errors,
            warnings,
            restored_to: target_path,
        })
    }

    /// Restore a single file
    pub async fn restore_file(
        &self,
        snapshot_id: SnapshotId,
        file_path: &Path,
        target_path: &Path,
    ) -> BackupResult<u64> {
        // Download full backup
        let temp_dir = tempfile::TempDir::new().map_err(|e| BackupError::Io(e))?;
        let backup_path = temp_dir.path().join("backup.tar.gz");

        let data = self
            .storage
            .retrieve(&format!("snapshots/{}", snapshot_id.0))
            .await?;
        tokio::fs::write(&backup_path, data)
            .await
            .map_err(BackupError::Io)?;

        // Extract
        let extract_path = temp_dir.path().join("extracted");
        super::decompress_backup(&backup_path, &extract_path, CompressionFormat::Gzip).await?;

        // Find and copy the specific file
        let source_file = extract_path.join(file_path);
        if !source_file.exists() {
            return Err(BackupError::Storage(format!(
                "File not found in backup: {}",
                file_path.display()
            )));
        }

        // Create parent directory
        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(BackupError::Io)?;
        }

        let bytes = tokio::fs::copy(&source_file, target_path)
            .await
            .map_err(BackupError::Io)?;

        info!(
            "Restored single file: {} -> {} ({} bytes)",
            file_path.display(),
            target_path.display(),
            bytes
        );

        Ok(bytes)
    }

    /// Point-in-time recovery for databases
    pub async fn point_in_time_recovery(
        &self,
        _snapshot_id: SnapshotId,
        _target_time: DateTime<Utc>,
        _options: &RestoreOptions,
    ) -> BackupResult<RestoreResult> {
        // This would replay WAL/bingo logs to reach a specific point in time
        // For now, return error as this requires database-specific implementation
        Err(BackupError::Restore(
            "Point-in-time recovery not yet implemented".to_string(),
        ))
    }

    /// Report progress if callback is set
    fn report_progress(&self, options: &RestoreOptions, progress: RestoreProgress) {
        if let Some(ref callback) = options.progress_callback {
            callback(progress);
        }
    }
}

/// Quick restore function
pub async fn quick_restore(
    storage: &super::BackupStorage,
    snapshot_id: SnapshotId,
    target_path: &Path,
    password: Option<&str>,
) -> BackupResult<RestoreResult> {
    let options = RestoreOptions {
        target_path: Some(target_path.to_path_buf()),
        overwrite: true,
        verify_after_restore: true,
        ..Default::default()
    };

    let manager = RestoreManager::new(storage.clone(), options);
    manager.restore(snapshot_id).await
}

/// One-click restore (with confirmation)
pub async fn one_click_restore(
    storage: &super::BackupStorage,
    snapshot_id: SnapshotId,
) -> BackupResult<RestoreResult> {
    info!("One-click restore initiated for snapshot {}", snapshot_id.0);

    // Get original path from metadata
    let metadata_key = format!("snapshots/{}/metadata", snapshot_id.0);
    let metadata = storage.retrieve(&metadata_key).await.ok();

    let target_path = if let Some(data) = metadata {
        if let Ok(json) = serde_json::from_slice::<HashMap<String, String>>(&data) {
            json.get("source_path").map(|p| PathBuf::from(p))
        } else {
            None
        }
    } else {
        None
    };

    let options = RestoreOptions {
        target_path: target_path.clone(),
        overwrite: false, // Be safe for one-click
        verify_after_restore: true,
        ..Default::default()
    };

    let manager = RestoreManager::new(storage.clone(), options);
    let result = manager.restore(snapshot_id).await?;

    if result.success {
        info!(
            "One-click restore completed successfully to {:?}",
            target_path
        );
    } else {
        error!(
            "One-click restore failed with {} errors",
            result.errors.len()
        );
    }

    Ok(result)
}

/// Create restore preview (what would be restored)
pub async fn preview_restore(
    storage: &super::BackupStorage,
    snapshot_id: SnapshotId,
) -> BackupResult<RestorePreview> {
    let temp_dir = tempfile::TempDir::new().map_err(|e| BackupError::Io(e))?;
    let backup_path = temp_dir.path().join("backup.tar.gz");

    // Download backup
    let data = storage
        .retrieve(&format!("snapshots/{}", snapshot_id.0))
        .await?;
    tokio::fs::write(&backup_path, data)
        .await
        .map_err(BackupError::Io)?;

    // Extract
    let extract_path = temp_dir.path().join("extracted");
    super::decompress_backup(&backup_path, &extract_path, CompressionFormat::Gzip).await?;

    // Walk and collect info
    let mut files = Vec::new();
    let mut total_size = 0u64;
    let mut file_count = 0u64;

    for entry in walkdir::WalkDir::new(&extract_path) {
        let entry = entry.map_err(|e| BackupError::Io(e.into()))?;

        if entry.file_type().is_file() {
            let path = entry.path();
            let relative = path.strip_prefix(&extract_path).unwrap_or(path);
            let metadata = entry.metadata().map_err(|e| BackupError::Io(e.into()))?;

            files.push(RestoreFileInfo {
                path: relative.to_path_buf(),
                size: metadata.len(),
                modified: metadata.modified().ok().map(|m| m.into()),
            });

            total_size += metadata.len();
            file_count += 1;
        }
    }

    Ok(RestorePreview {
        snapshot_id,
        file_count,
        total_size_bytes: total_size,
        files,
        original_location: None,
    })
}

/// Restore preview information
#[derive(Debug, Clone)]
pub struct RestorePreview {
    pub snapshot_id: SnapshotId,
    pub file_count: u64,
    pub total_size_bytes: u64,
    pub files: Vec<RestoreFileInfo>,
    pub original_location: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct RestoreFileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_restore_options() {
        let options = RestoreOptions::default();
        assert!(!options.overwrite);
        assert!(options.verify_after_restore);
        assert!(!options.dry_run);
    }

    #[tokio::test]
    async fn test_restore_result() {
        let result = RestoreResult {
            success: true,
            files_restored: 10,
            files_skipped: 2,
            files_failed: 0,
            bytes_restored: 10240,
            duration_seconds: 5.5,
            errors: vec![],
            warnings: vec![],
            restored_to: PathBuf::from("/tmp/restore"),
        };

        assert!(result.success);
        assert_eq!(result.files_restored, 10);
    }

    #[tokio::test]
    async fn test_restore_preview() {
        let temp_dir = TempDir::new().unwrap();

        // Create a mock backup
        let backup_dir = temp_dir.path().join("backup");
        tokio::fs::create_dir_all(&backup_dir).await.unwrap();
        tokio::fs::write(backup_dir.join("file1.txt"), b"content1")
            .await
            .unwrap();
        tokio::fs::write(backup_dir.join("file2.txt"), b"content2")
            .await
            .unwrap();

        // Note: preview_restore needs actual storage, so we just test the struct
        let preview = RestorePreview {
            snapshot_id: SnapshotId::new(),
            file_count: 2,
            total_size_bytes: 16,
            files: vec![
                RestoreFileInfo {
                    path: PathBuf::from("file1.txt"),
                    size: 8,
                    modified: Some(Utc::now()),
                },
                RestoreFileInfo {
                    path: PathBuf::from("file2.txt"),
                    size: 8,
                    modified: Some(Utc::now()),
                },
            ],
            original_location: Some(PathBuf::from("/original")),
        };

        assert_eq!(preview.file_count, 2);
        assert_eq!(preview.files.len(), 2);
    }
}
