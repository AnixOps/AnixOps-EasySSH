//! Rollback mechanism for failed updates

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs;

#[derive(Debug)]
pub struct RollbackManager {
    backup_dir: PathBuf,
    max_backups: u32,
    backups: VecDeque<BackupEntry>,
}

#[derive(Debug, Clone)]
struct BackupEntry {
    pub version: String,
    pub path: PathBuf,
    pub timestamp: u64,
    pub metadata: BackupMetadata,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BackupMetadata {
    pub original_path: PathBuf,
    pub version: String,
    pub created_at: String,
    pub platform: String,
    pub arch: String,
}

impl RollbackManager {
    pub async fn new(temp_dir: Option<PathBuf>, max_backups: u32) -> anyhow::Result<Self> {
        let backup_dir = temp_dir.map(|d| d.join("backups")).unwrap_or_else(|| {
            dirs::data_local_dir()
                .unwrap_or_else(|| std::env::temp_dir())
                .join("easyssh")
                .join("backups")
        });

        fs::create_dir_all(&backup_dir).await?;

        let backups = Self::load_existing_backups(&backup_dir).await?;

        Ok(Self {
            backup_dir,
            max_backups,
            backups,
        })
    }

    async fn load_existing_backups(dir: &Path) -> anyhow::Result<VecDeque<BackupEntry>> {
        let mut backups = VecDeque::new();

        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read(&path).await {
                    if let Ok(metadata) = serde_json::from_slice::<BackupMetadata>(&content) {
                        let binary_path = path.with_extension("");
                        if binary_path.exists() {
                            backups.push_back(BackupEntry {
                                version: metadata.version.clone(),
                                path: binary_path,
                                timestamp: SystemTime::now()
                                    .duration_since(SystemTime::UNIX_EPOCH)?
                                    .as_secs(),
                                metadata,
                            });
                        }
                    }
                }
            }
        }

        // Sort by timestamp (oldest first)
        backups
            .make_contiguous()
            .sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(backups)
    }

    /// Create backup before update
    pub async fn create_backup(
        &self,
        current_exe: &Path,
        new_version: &str,
    ) -> anyhow::Result<PathBuf> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        let backup_name = format!("easyssh-backup-{}-{}", super::CURRENT_VERSION, timestamp);

        let backup_path = self.backup_dir.join(&backup_name);
        let metadata_path = backup_path.with_extension("json");

        // Copy current executable
        fs::copy(current_exe, &backup_path).await?;

        // Write metadata
        let metadata = BackupMetadata {
            original_path: current_exe.to_path_buf(),
            version: super::CURRENT_VERSION.to_string(),
            created_at: chrono::Local::now().to_rfc3339(),
            platform: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        };

        fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?).await?;

        // Cleanup old backups
        self.cleanup_old_backups().await?;

        Ok(backup_path)
    }

    /// Perform rollback to previous version
    pub async fn rollback(&self) -> anyhow::Result<String> {
        // Get most recent backup
        let backup = self
            .backups
            .back()
            .ok_or_else(|| anyhow::anyhow!("No backup available for rollback"))?;

        let current_exe = std::env::current_exe()?;

        // Verify backup integrity
        if !self.verify_backup(&backup.path).await? {
            return Err(anyhow::anyhow!("Backup verification failed"));
        }

        // On Windows, we need special handling for running executable
        #[cfg(target_os = "windows")]
        {
            super::platform::windows::WindowsUpdater::schedule_replace_on_reboot(
                &backup.path,
                &current_exe,
            )
            .await?;

            // Schedule restart
            tokio::spawn(async {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                std::process::exit(0);
            });
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On Unix, we can atomically replace
            fs::rename(&backup.path, &current_exe).await?;

            // Restart
            tokio::spawn(async {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                std::process::exit(0);
            });
        }

        Ok(backup.version.clone())
    }

    /// Verify backup integrity
    async fn verify_backup(&self, backup_path: &Path) -> anyhow::Result<bool> {
        // Check if file exists and is readable
        match fs::metadata(backup_path).await {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Ok(false);
                }

                // Verify file size is reasonable
                if metadata.len() == 0 {
                    return Ok(false);
                }

                // Try to read first few bytes
                let mut file = fs::File::open(backup_path).await?;
                let mut header = [0u8; 4];
                if let Err(_) = tokio::io::AsyncReadExt::read_exact(&mut file, &mut header).await {
                    return Ok(false);
                }

                // Check for valid executable magic
                #[cfg(target_os = "windows")]
                {
                    // Windows PE: MZ
                    Ok(header[0] == 0x4D && header[1] == 0x5A)
                }

                #[cfg(target_os = "macos")]
                {
                    // macOS Mach-O: 0xFEEDFACE or 0xFEEDFACF or 0xCAFEBABE (fat binary)
                    let magic = u32::from_be_bytes(header);
                    Ok(magic == 0xFEEDFACE || magic == 0xFEEDFACF || magic == 0xCAFEBABE)
                }

                #[cfg(target_os = "linux")]
                {
                    // ELF: 0x7F ELF
                    Ok(header[0] == 0x7F && &header[1..4] == b"ELF")
                }

                #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
                {
                    Ok(true)
                }
            }
            Err(_) => Ok(false),
        }
    }

    /// Cleanup old backups
    async fn cleanup_old_backups(&self) -> anyhow::Result<()> {
        let mut backups = self.backups.clone();

        while backups.len() > self.max_backups as usize {
            if let Some(old_backup) = backups.pop_front() {
                let _ = fs::remove_file(&old_backup.path).await;
                let _ = fs::remove_file(&old_backup.path.with_extension("json")).await;
            }
        }

        Ok(())
    }

    /// Get available backups
    pub fn get_available_backups(&self) -> Vec<(String, u64)> {
        self.backups
            .iter()
            .map(|b| (b.version.clone(), b.timestamp))
            .collect()
    }

    /// Check if rollback is possible
    pub fn can_rollback(&self) -> bool {
        !self.backups.is_empty()
    }

    /// Get backup directory size
    pub async fn get_backup_size(&self) -> anyhow::Result<u64> {
        let mut total_size = 0u64;

        let mut entries = fs::read_dir(&self.backup_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }

    /// Clear all backups
    pub async fn clear_all_backups(&self) -> anyhow::Result<()> {
        let mut entries = fs::read_dir(&self.backup_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            fs::remove_file(entry.path()).await?;
        }

        Ok(())
    }

    /// Mark current installation as stable (prevents rollback)
    pub async fn mark_stable(&self) -> anyhow::Result<()> {
        let stable_marker = self.backup_dir.join(".stable");
        let content = format!(
            "version={}\ntimestamp={}",
            super::CURRENT_VERSION,
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs()
        );

        fs::write(&stable_marker, content).await?;
        Ok(())
    }

    /// Check if current installation is stable
    pub async fn is_stable(&self) -> anyhow::Result<bool> {
        let stable_marker = self.backup_dir.join(".stable");

        if let Ok(content) = fs::read_to_string(&stable_marker).await {
            for line in content.lines() {
                if let Some(version) = line.strip_prefix("version=") {
                    return Ok(version == super::CURRENT_VERSION);
                }
            }
        }

        Ok(false)
    }
}

/// Health check for rollback system
pub async fn health_check(rollback_manager: &RollbackManager) -> RollbackHealth {
    let can_rollback = rollback_manager.can_rollback();
    let available_backups = rollback_manager.get_available_backups().len();

    let backup_size = rollback_manager.get_backup_size().await.unwrap_or(0);
    let is_stable = rollback_manager.is_stable().await.unwrap_or(false);

    RollbackHealth {
        can_rollback,
        available_backups,
        backup_size,
        is_stable,
        healthy: can_rollback || is_stable,
    }
}

#[derive(Debug)]
pub struct RollbackHealth {
    pub can_rollback: bool,
    pub available_backups: usize,
    pub backup_size: u64,
    pub is_stable: bool,
    pub healthy: bool,
}

/// Emergency recovery - restore from backup even if current state unknown
pub async fn emergency_recovery(backup_path: &Path) -> anyhow::Result<()> {
    let current_exe = std::env::current_exe()?;

    // Verify backup
    let metadata = fs::metadata(backup_path).await?;
    if metadata.len() == 0 {
        return Err(anyhow::anyhow!("Backup file is empty"));
    }

    // Replace current executable
    fs::rename(backup_path, &current_exe).await?;

    log::info!("Emergency recovery completed");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backup_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = RollbackManager::new(Some(temp_dir.path().to_path_buf()), 3)
            .await
            .unwrap();

        // Create a test file to backup
        let test_file = temp_dir.path().join("test.exe");
        fs::write(&test_file, b"test content").await.unwrap();

        // Create backup
        let backup = manager.create_backup(&test_file, "1.0.0").await.unwrap();

        assert!(backup.exists());
        assert!(backup.with_extension("json").exists());
    }

    #[tokio::test]
    async fn test_backup_cleanup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = RollbackManager::new(Some(temp_dir.path().to_path_buf()), 2)
            .await
            .unwrap();

        // Create multiple backups
        let test_file = temp_dir.path().join("test.exe");
        fs::write(&test_file, b"test").await.unwrap();

        for i in 0..5 {
            let manager = RollbackManager::new(Some(temp_dir.path().to_path_buf()), 2)
                .await
                .unwrap();
            manager
                .create_backup(&test_file, &format!("1.0.{}", i))
                .await
                .unwrap();
        }

        // Check that old backups are cleaned up
        let final_manager = RollbackManager::new(Some(temp_dir.path().to_path_buf()), 2)
            .await
            .unwrap();

        assert_eq!(final_manager.get_available_backups().len(), 2);
    }
}
