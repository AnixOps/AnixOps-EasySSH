//! Backup engine - orchestrates the backup process

use super::*;
use compression::{compress_and_encrypt, compress_backup};
use incremental::{IncrementalBackupManager, IncrementalIndex};
use remote::{RemoteBackupConfig, RemoteFileBackup};
use scheduler::{BackupScheduler, ScheduleConfig};
use storage::{BackupStorage, LocalStorage};
use verification::BackupVerifier;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{error, info, warn};

/// Backup engine configuration
#[derive(Debug, Clone)]
pub struct BackupEngineConfig {
    /// Base directory for temporary files
    pub temp_dir: PathBuf,
    /// Default storage for backups
    pub default_storage_path: PathBuf,
    /// Index storage path
    pub index_dir: PathBuf,
    /// Maximum parallel jobs
    pub max_parallel_jobs: u32,
    /// Default bandwidth limit (bytes/sec, 0 = unlimited)
    pub default_bandwidth_limit: u64,
    /// Enable verification by default
    pub verify_by_default: bool,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Enable deduplication
    pub enable_deduplication: bool,
}

impl Default for BackupEngineConfig {
    fn default() -> Self {
        Self {
            temp_dir: std::env::temp_dir().join("easyssh-backups"),
            default_storage_path: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("backups"),
            index_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("easyssh-backup-indexes"),
            max_parallel_jobs: 4,
            default_bandwidth_limit: 0,
            verify_by_default: true,
            max_retries: 3,
            enable_deduplication: true,
        }
    }
}

/// Backup job definition
#[derive(Debug, Clone)]
pub struct BackupJob {
    pub id: BackupJobId,
    pub config: BackupConfig,
    pub storage: BackupStorage,
    pub status: BackupStatus,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub run_count: u32,
    pub last_error: Option<String>,
}

/// Backup job builder
pub struct BackupJobBuilder {
    id: BackupJobId,
    config: BackupConfig,
    storage_backends: Vec<Box<dyn StorageBackend>>,
}

impl BackupJobBuilder {
    /// Create a new job builder
    pub fn new(name: &str) -> Self {
        Self {
            id: BackupJobId::new(),
            config: BackupConfig {
                name: name.to_string(),
                ..Default::default()
            },
            storage_backends: vec![],
        }
    }

    /// Set source
    pub fn from_local(mut self, path: impl AsRef<Path>) -> Self {
        self.config.source = BackupSource::Local {
            path: path.as_ref().to_path_buf(),
        };
        self
    }

    /// Set remote source
    pub fn from_remote(mut self, host: &str, port: u16, username: &str, path: &str) -> Self {
        self.config.source = BackupSource::Remote {
            host: host.to_string(),
            port,
            username: username.to_string(),
            path: path.to_string(),
        };
        self
    }

    /// Set database source
    pub fn from_database(mut self, db_type: DatabaseType, connection_string: &str) -> Self {
        self.config.source = BackupSource::Database {
            db_type,
            connection_string: connection_string.to_string(),
        };
        self
    }

    /// Set local target
    pub fn to_local(mut self, path: impl AsRef<Path>) -> Self {
        self.config.targets.push(BackupTarget::Local {
            path: path.as_ref().to_path_buf(),
        });
        self
    }

    /// Set S3 target
    pub fn to_s3(mut self, bucket: &str, prefix: &str, region: &str) -> Self {
        self.config.targets.push(BackupTarget::S3 {
            bucket: bucket.to_string(),
            prefix: prefix.to_string(),
            region: region.to_string(),
        });
        self
    }

    /// Set backup type
    pub fn with_type(mut self, backup_type: BackupType) -> Self {
        self.config.backup_type = backup_type;
        self
    }

    /// Set schedule
    pub fn on_schedule(mut self, schedule: ScheduleConfig) -> Self {
        self.config.schedule = Some(schedule);
        self
    }

    /// Set compression
    pub fn with_compression(
        mut self,
        enabled: bool,
        format: CompressionFormat,
        level: u32,
    ) -> Self {
        self.config.compression = CompressionSettings {
            enabled,
            format,
            level,
        };
        self
    }

    /// Set encryption
    pub fn with_encryption(mut self, enabled: bool) -> Self {
        self.config.encryption.enabled = enabled;
        self
    }

    /// Add exclusion pattern
    pub fn exclude(mut self, pattern: &str) -> Self {
        self.config.exclusions.push(pattern.to_string());
        self
    }

    /// Set retention policy
    pub fn with_retention(mut self, policy: RetentionPolicy) -> Self {
        self.config.retention = policy;
        self
    }

    /// Build the job
    pub fn build(mut self) -> BackupResult<(BackupJob, BackupStorage)> {
        if self.config.targets.is_empty() {
            return Err(BackupError::InvalidConfiguration(
                "No backup target specified".to_string(),
            ));
        }

        // Create storage from first target
        let primary = self.create_storage(&self.config.targets[0])?;

        // Add mirrors from additional targets
        let mut storage = BackupStorage::new(primary);
        for target in self.config.targets.iter().skip(1) {
            let mirror = self.create_storage(target)?;
            storage = storage.add_mirror(mirror);
        }

        let job = BackupJob {
            id: self.id,
            config: self.config,
            storage: storage.clone(),
            status: BackupStatus::Queued,
            created_at: Utc::now(),
            last_run: None,
            next_run: None,
            run_count: 0,
            last_error: None,
        };

        Ok((job, storage))
    }

    fn create_storage(&self, target: &BackupTarget) -> BackupResult<Box<dyn StorageBackend>> {
        match target {
            BackupTarget::Local { path } => {
                let storage = LocalStorage::new(path)?;
                Ok(Box::new(storage))
            }
            _ => Err(BackupError::Storage(
                "Cloud storage requires feature flags".to_string(),
            )),
        }
    }
}

/// Backup engine
pub struct BackupEngine {
    config: BackupEngineConfig,
    jobs: Arc<RwLock<HashMap<BackupJobId, BackupJob>>>,
    scheduler: BackupScheduler,
    incremental_manager: IncrementalBackupManager,
    progress_tx: mpsc::Sender<BackupProgress>,
    progress_rx: Arc<Mutex<mpsc::Receiver<BackupProgress>>>,
    active_jobs:
        Arc<Mutex<HashMap<BackupJobId, tokio::task::JoinHandle<BackupResult<BackupSnapshot>>>>>,
}

impl BackupEngine {
    /// Create a new backup engine
    pub fn new(config: BackupEngineConfig) -> BackupResult<Self> {
        let scheduler = BackupScheduler::new();
        let incremental_manager = IncrementalBackupManager::new(&config.index_dir)?;
        let (progress_tx, progress_rx) = mpsc::channel(100);

        Ok(Self {
            config,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            scheduler,
            incremental_manager,
            progress_tx,
            progress_rx: Arc::new(Mutex::new(progress_rx)),
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Add a backup job
    pub async fn add_job(&self, job: BackupJob) -> BackupResult<()> {
        let mut jobs = self.jobs.write().await;
        let job_name = job.config.name.clone();
        let job_id = job.id;

        // Register with scheduler if scheduled
        if let Some(schedule) = &job.config.schedule {
            self.scheduler
                .add_job(job.id, job.config.clone(), schedule.clone())
                .await?;
        }

        jobs.insert(job.id, job);

        info!("Added backup job {}: {}", job_id.0, job_name);

        Ok(())
    }

    /// Remove a backup job
    pub async fn remove_job(&self, job_id: BackupJobId) -> BackupResult<()> {
        // Cancel if running
        self.cancel_job(job_id).await?;

        // Remove from scheduler
        self.scheduler.remove_job(job_id).await?;

        // Remove from jobs list
        let mut jobs = self.jobs.write().await;
        jobs.remove(&job_id);

        info!("Removed backup job {}", job_id.0);

        Ok(())
    }

    /// Get a job
    pub async fn get_job(&self, job_id: BackupJobId) -> Option<BackupJob> {
        let jobs = self.jobs.read().await;
        jobs.get(&job_id).cloned()
    }

    /// Get all jobs
    pub async fn get_all_jobs(&self) -> Vec<BackupJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Run a backup job immediately
    pub async fn run_job(
        &mut self,
        job_id: BackupJobId,
        force_full: bool,
    ) -> BackupResult<BackupSnapshot> {
        let job = self
            .get_job(job_id)
            .await
            .ok_or(BackupError::JobNotFound(job_id))?;

        info!("Running backup job {}: {}", job_id.0, job.config.name);

        let snapshot_id = SnapshotId::new();
        let backup_type = if force_full {
            BackupType::Full
        } else {
            job.config.backup_type
        };

        // Create snapshot record
        let mut snapshot = BackupSnapshot {
            id: snapshot_id,
            job_id,
            backup_type,
            status: BackupStatus::Running,
            source: job.config.source.clone(),
            target: job.config.targets[0].clone(),
            created_at: Utc::now(),
            completed_at: None,
            size_bytes: 0,
            compressed_size_bytes: 0,
            file_count: 0,
            checksum: String::new(),
            parent_snapshot: None,
            encryption_enabled: job.config.encryption.enabled,
            compression_enabled: job.config.compression.enabled,
            metadata: HashMap::new(),
            error_message: None,
        };

        // Execute backup
        let result = self.execute_backup(&job, &mut snapshot).await;

        // Update snapshot status
        match result {
            Ok(_) => {
                snapshot.status = BackupStatus::Completed;
                snapshot.completed_at = Some(Utc::now());
                info!(
                    "Backup job {} completed: {} files, {} bytes",
                    job_id.0, snapshot.file_count, snapshot.size_bytes
                );
            }
            Err(e) => {
                snapshot.status = BackupStatus::Failed;
                snapshot.error_message = Some(e.to_string());
                error!("Backup job {} failed: {}", job_id.0, e);
            }
        }

        // Store snapshot metadata
        let meta_json =
            serde_json::to_string(&snapshot).map_err(|e| BackupError::Config(e.to_string()))?;
        job.storage
            .store(
                &format!("snapshots/{}/metadata", snapshot_id.0),
                meta_json.as_bytes(),
                HashMap::new(),
            )
            .await?;

        // Update job stats
        let mut jobs = self.jobs.write().await;
        if let Some(j) = jobs.get_mut(&job_id) {
            j.last_run = Some(Utc::now());
            j.run_count += 1;
            j.status = snapshot.status;
            if snapshot.status == BackupStatus::Failed {
                j.last_error = snapshot.error_message.clone();
            }
        }

        Ok(snapshot)
    }

    /// Cancel a running job
    pub async fn cancel_job(&self, job_id: BackupJobId) -> BackupResult<()> {
        let mut active = self.active_jobs.lock().await;

        if let Some(handle) = active.remove(&job_id) {
            handle.abort();
            info!("Cancelled backup job {}", job_id.0);
        }

        Ok(())
    }

    /// Execute the backup process
    async fn execute_backup(
        &mut self,
        job: &BackupJob,
        snapshot: &mut BackupSnapshot,
    ) -> BackupResult<()> {
        let temp_dir = tempfile::TempDir::new().map_err(BackupError::Io)?;
        let work_dir = temp_dir.path().join("work");
        tokio::fs::create_dir_all(&work_dir)
            .await
            .map_err(BackupError::Io)?;

        // 1. Collect files
        info!("Collecting files from source...");
        let (files, total_size) = self.collect_files(&job.config.source, &job.config).await?;
        snapshot.size_bytes = total_size;
        snapshot.file_count = files.len() as u64;

        // 2. Build incremental index if needed
        let (incremental_index, diff) = if job.config.backup_type == BackupType::Incremental {
            info!("Building incremental index...");
            // Find parent snapshot
            let parent_id = self.find_parent_snapshot(job.id).await;
            snapshot.parent_snapshot = parent_id;

            if let Some(parent_id) = parent_id {
                let (index, diff) = self
                    .incremental_manager
                    .build_incremental_index(
                        &work_dir,
                        snapshot.id,
                        parent_id,
                        &BackupFilter::default(),
                    )
                    .await?;

                let changed_count = diff.added.len() + diff.modified.len();
                info!(
                    "Incremental backup: {} new, {} modified, {} deleted",
                    diff.added.len(),
                    diff.modified.len(),
                    diff.deleted.len()
                );

                if changed_count == 0 {
                    info!("No changes detected, skipping backup");
                    snapshot.status = BackupStatus::Completed;
                    snapshot.compressed_size_bytes = 0;
                    return Ok(());
                }

                (Some(index), Some(diff))
            } else {
                // No parent, do full backup
                let index = self
                    .incremental_manager
                    .build_index(&work_dir, snapshot.id, None, &BackupFilter::default())
                    .await?;
                (Some(index), None)
            }
        } else {
            (None, None)
        };

        // 3. Create archive
        info!("Creating archive...");
        let archive_path = temp_dir.path().join("backup.tar.gz");
        let filter = BackupFilter {
            exclude_patterns: job.config.exclusions.clone(),
            ..Default::default()
        };

        let archive_size = compression::compress_directory(
            &work_dir,
            &archive_path,
            job.config.compression.format,
            job.config.compression.level,
        )
        .await?;

        // 4. Encrypt if needed
        let final_path = if job.config.encryption.enabled {
            info!("Encrypting archive...");
            let encrypted_path = temp_dir.path().join("backup.tar.gz.enc");
            // Note: In production, get password from secure storage
            let password = "default_password"; // TODO: Get from keychain

            compression::encrypt_backup(
                &archive_path,
                &encrypted_path,
                password,
                &job.config.encryption,
            )
            .await?;

            encrypted_path
        } else {
            archive_path
        };

        // 5. Upload to storage
        info!("Uploading to storage...");
        snapshot.status = BackupStatus::Uploading;

        let final_data = tokio::fs::read(&final_path)
            .await
            .map_err(BackupError::Io)?;
        snapshot.checksum = blake3::hash(&final_data).to_hex().to_string();
        snapshot.compressed_size_bytes = final_data.len() as u64;

        let key = format!("snapshots/{}/data", snapshot.id.0);
        let metadata = HashMap::from([
            ("job_id".to_string(), job.id.0.to_string()),
            ("snapshot_id".to_string(), snapshot.id.0.to_string()),
            ("checksum".to_string(), snapshot.checksum.clone()),
            (
                "compression".to_string(),
                job.config.compression.enabled.to_string(),
            ),
            (
                "encryption".to_string(),
                job.config.encryption.enabled.to_string(),
            ),
        ]);

        job.storage.store(&key, &final_data, metadata).await?;

        // 6. Save index
        if let Some(index) = incremental_index {
            self.incremental_manager.save_index(&index).await?;
        }

        // 7. Verify if needed
        if job.config.verify_backup {
            info!("Verifying backup...");
            snapshot.status = BackupStatus::Verifying;
            let verifier = BackupVerifier::new(verification::VerificationOptions::default());
            let result = verifier
                .verify_file(&final_path, Some(&snapshot.checksum))
                .await;

            if !result.success {
                return Err(BackupError::Verification(format!(
                    "Verification failed: {} errors",
                    result.errors.len()
                )));
            }
        }

        // 8. Cleanup old snapshots based on retention policy
        self.apply_retention_policy(job, &job.config.retention)
            .await?;

        info!(
            "Backup completed successfully: {} bytes uploaded",
            final_data.len()
        );

        Ok(())
    }

    /// Collect files from source
    async fn collect_files(
        &self,
        source: &BackupSource,
        config: &BackupConfig,
    ) -> BackupResult<(Vec<PathBuf>, u64)> {
        match source {
            BackupSource::Local { path } => {
                let mut files = Vec::new();
                let mut total_size = 0u64;

                let filter = BackupFilter {
                    exclude_patterns: config.exclusions.clone(),
                    include_patterns: config.inclusions.clone(),
                    ..Default::default()
                };

                let entries = walkdir::WalkDir::new(path)
                    .into_iter()
                    .filter_map(|e| e.ok());

                for entry in entries {
                    let file_path = entry.path();
                    let relative_path = file_path.strip_prefix(path).unwrap_or(file_path);

                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_file() {
                            if filter.should_include(file_path, &metadata) {
                                files.push(relative_path.to_path_buf());
                                total_size += metadata.len();
                            }
                        }
                    }
                }

                Ok((files, total_size))
            }
            BackupSource::Remote {
                host,
                port,
                username,
                path,
            } => {
                // Remote backup - use SFTP
                let remote_config = RemoteBackupConfig {
                    host: host.clone(),
                    port: *port,
                    username: username.clone(),
                    auth_type: remote::RemoteAuthType::SshAgent,
                    source_path: PathBuf::from(path),
                    bandwidth_limit: config.bandwidth_limit,
                    timeout_seconds: 300,
                    pre_backup_script: None,
                    post_backup_script: None,
                };

                let remote_backup = RemoteFileBackup::new_ssh(remote_config).await?;
                let filter = BackupFilter {
                    exclude_patterns: config.exclusions.clone(),
                    ..Default::default()
                };

                // Download files
                let temp_dir = tempfile::TempDir::new().map_err(BackupError::Io)?;
                let result = remote_backup.backup(&filter, temp_dir.path(), None).await?;

                Ok((vec![], result.downloaded_size_bytes))
            }
            BackupSource::Database {
                db_type,
                connection_string,
            } => {
                // Database backup
                use database::{
                    DatabaseBackupConfig, DatabaseBackupEngine, DatabaseConfig, DatabaseType,
                };

                // Convert the DatabaseType from mod.rs to database::DatabaseType
                let db_type_converted = match db_type {
                    super::DatabaseType::MySQL => DatabaseType::MySQL,
                    super::DatabaseType::PostgreSQL => DatabaseType::PostgreSQL,
                    super::DatabaseType::MongoDB => DatabaseType::MongoDB,
                    super::DatabaseType::Redis => DatabaseType::Redis,
                    super::DatabaseType::SQLite => DatabaseType::SQLite,
                };

                let db_config = DatabaseConfig::new(db_type_converted, connection_string, "root");
                let backup_config = DatabaseBackupConfig {
                    connection: db_config,
                    options: database::DatabaseBackupOptions::default(),
                    output_path: self.config.temp_dir.clone(),
                    encrypt: config.encryption.enabled,
                    compression: config.compression.enabled,
                };

                if let Some(engine) = database::get_backup_engine(db_type_converted) {
                    let result = engine.backup(&backup_config).await?;
                    Ok((vec![result.output_path], result.size_bytes))
                } else {
                    Err(BackupError::Database(format!(
                        "Unsupported database type: {:?}",
                        db_type
                    )))
                }
            }
        }
    }

    /// Find the most recent parent snapshot for incremental backup
    async fn find_parent_snapshot(&self, job_id: BackupJobId) -> Option<SnapshotId> {
        // This would query the database or storage for the most recent completed snapshot
        // For now, return None (do full backup)
        None
    }

    /// Apply retention policy - cleanup old snapshots
    async fn apply_retention_policy(
        &self,
        job: &BackupJob,
        policy: &RetentionPolicy,
    ) -> BackupResult<()> {
        info!("Applying retention policy for job {}", job.id.0);

        // List all snapshots for this job
        let prefix = format!("snapshots/");
        let objects = job.storage.list(&prefix).await?;

        // Filter to this job's snapshots and sort by date
        let job_id_str = job.id.0.to_string();
        let mut snapshots: Vec<_> = objects
            .into_iter()
            .filter(|obj| {
                obj.metadata
                    .get("job_id")
                    .map(|s| s == &job_id_str)
                    .unwrap_or(false)
            })
            .collect();

        snapshots.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

        if snapshots.len() > policy.max_snapshots as usize {
            let to_delete = &snapshots[policy.max_snapshots as usize..];

            for obj in to_delete {
                info!("Deleting old snapshot: {}", obj.key);
                if let Err(e) = job.storage.delete(&obj.key).await {
                    warn!("Failed to delete old snapshot {}: {}", obj.key, e);
                }
            }
        }

        Ok(())
    }

    /// Start the scheduler
    pub async fn start_scheduler(&self) -> BackupResult<()> {
        self.scheduler.start().await?;

        // Listen for scheduler events
        let event_rx = self.scheduler.get_event_receiver().await;
        let jobs = self.jobs.clone();
        let scheduler = self.scheduler.clone();

        tokio::spawn(async move {
            let mut rx = event_rx.lock().await;

            while let Some(event) = rx.recv().await {
                match event {
                    scheduler::SchedulerEvent::JobTriggered(job_id, trigger) => {
                        info!("Job {} triggered by {:?}", job_id.0, trigger);

                        // Mark as running
                        if let Err(e) = scheduler.mark_job_running(job_id, true).await {
                            warn!("Failed to mark job {} as running: {}", job_id.0, e);
                        }

                        // Note: Actual job execution would be done here
                        // For now, just mark as completed
                        if let Err(e) = scheduler.mark_job_completed(job_id, true).await {
                            warn!("Failed to mark job {} as completed: {}", job_id.0, e);
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop_scheduler(&self) -> BackupResult<()> {
        self.scheduler.stop().await
    }

    /// Get progress receiver
    pub async fn get_progress_receiver(&self) -> Arc<Mutex<mpsc::Receiver<BackupProgress>>> {
        self.progress_rx.clone()
    }

    /// Get statistics
    pub async fn get_stats(&self) -> BackupResult<BackupStats> {
        let jobs = self.jobs.read().await;

        let mut stats = BackupStats::default();
        stats.total_jobs = jobs.len() as u32;

        // Get next scheduled backup
        for job in jobs.values() {
            if let Some(schedule) = &job.config.schedule {
                if schedule.enabled {
                    if let Some(next) = self.scheduler.get_next_execution(job.id).await {
                        if stats
                            .next_scheduled_backup
                            .map(|s| next < s)
                            .unwrap_or(true)
                        {
                            stats.next_scheduled_backup = Some(next);
                        }
                    }
                }
            }
        }

        Ok(stats)
    }
}

impl Clone for BackupEngine {
    fn clone(&self) -> Self {
        // Create new channels for the clone
        let (progress_tx, progress_rx) = mpsc::channel(100);

        Self {
            config: self.config.clone(),
            jobs: self.jobs.clone(),
            scheduler: BackupScheduler::new(),
            incremental_manager: IncrementalBackupManager::new(&self.config.index_dir).unwrap(),
            progress_tx,
            progress_rx: Arc::new(Mutex::new(progress_rx)),
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_backup_job_builder() {
        let result = BackupJobBuilder::new("Test Backup")
            .from_local("/home/user/data")
            .to_local("/backups")
            .with_type(BackupType::Incremental)
            .exclude("*.tmp")
            .exclude("*.log")
            .build();

        assert!(result.is_ok());

        let (job, _storage) = result.unwrap();
        assert_eq!(job.config.name, "Test Backup");
        assert_eq!(job.config.backup_type, BackupType::Incremental);
        assert_eq!(job.config.exclusions.len(), 2);
    }

    #[tokio::test]
    async fn test_backup_engine() {
        let temp_dir = TempDir::new().unwrap();

        let config = BackupEngineConfig {
            default_storage_path: temp_dir.path().join("backups"),
            index_dir: temp_dir.path().join("indexes"),
            ..Default::default()
        };

        let engine = BackupEngine::new(config).unwrap();

        // Create test source
        let source_dir = temp_dir.path().join("source");
        tokio::fs::create_dir_all(&source_dir).await.unwrap();
        tokio::fs::write(source_dir.join("test.txt"), b"Hello, World!")
            .await
            .unwrap();

        // Create and add job
        let (job, _storage) = BackupJobBuilder::new("Test Job")
            .from_local(&source_dir)
            .to_local(temp_dir.path().join("backups"))
            .build()
            .unwrap();

        engine.add_job(job.clone()).await.unwrap();

        // Get jobs
        let jobs = engine.get_all_jobs().await;
        assert_eq!(jobs.len(), 1);

        // Get specific job
        let retrieved = engine.get_job(job.id).await;
        assert!(retrieved.is_some());
    }
}
