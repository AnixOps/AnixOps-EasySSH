//! Backup and restore functionality

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use crate::database_client::{DatabaseConfig, DatabaseType, DatabaseError};

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub backup_type: BackupType,
    pub include_data: bool,
    pub include_schema: bool,
    pub compression_level: u32,
    pub encryption_enabled: bool,
    pub encryption_key: Option<String>,
    pub tables_to_include: Vec<String>,
    pub tables_to_exclude: Vec<String>,
    pub where_clauses: std::collections::HashMap<String, String>,
    pub create_before_date: Option<DateTime<Utc>>,
    pub verify_after_backup: bool,
    pub split_by_table: bool,
    pub max_file_size_mb: u64,
}

impl BackupConfig {
    pub fn full() -> Self {
        Self {
            backup_type: BackupType::Full,
            include_data: true,
            include_schema: true,
            compression_level: 6,
            encryption_enabled: false,
            encryption_key: None,
            tables_to_include: Vec::new(),
            tables_to_exclude: Vec::new(),
            where_clauses: std::collections::HashMap::new(),
            create_before_date: None,
            verify_after_backup: true,
            split_by_table: false,
            max_file_size_mb: 1024,
        }
    }

    pub fn schema_only() -> Self {
        let mut config = Self::full();
        config.include_data = false;
        config.backup_type = BackupType::Schema;
        config
    }

    pub fn data_only() -> Self {
        let mut config = Self::full();
        config.include_schema = false;
        config.backup_type = BackupType::Data;
        config
    }

    pub fn with_tables(mut self, tables: Vec<String>) -> Self {
        self.tables_to_include = tables;
        self
    }

    pub fn without_tables(mut self, tables: Vec<String>) -> Self {
        self.tables_to_exclude = tables;
        self
    }

    pub fn with_encryption(mut self, key: String) -> Self {
        self.encryption_enabled = true;
        self.encryption_key = Some(key);
        self
    }
}

/// Backup type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupType {
    Full,
    Schema,
    Data,
    Incremental,
    Differential,
}

/// Restore configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreConfig {
    pub target_database: String,
    pub create_database: bool,
    pub drop_existing: bool,
    pub partial_restore: bool,
    pub tables_to_restore: Vec<String>,
    pub data_only: bool,
    pub disable_foreign_keys: bool,
    pub disable_triggers: bool,
    pub verify_before_restore: bool,
    pub dry_run: bool,
}

impl RestoreConfig {
    pub fn new(target_database: String) -> Self {
        Self {
            target_database,
            create_database: true,
            drop_existing: false,
            partial_restore: false,
            tables_to_restore: Vec::new(),
            data_only: false,
            disable_foreign_keys: true,
            disable_triggers: false,
            verify_before_restore: true,
            dry_run: false,
        }
    }

    pub fn data_only(mut self) -> Self {
        self.data_only = true;
        self
    }

    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }
}

/// Backup result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    pub success: bool,
    pub file_path: PathBuf,
    pub file_size_bytes: u64,
    pub tables_backed_up: Vec<String>,
    pub rows_backed_up: u64,
    pub duration_ms: u64,
    pub compression_ratio: f64,
    pub checksum: String,
    pub warnings: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Restore result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    pub success: bool,
    pub tables_restored: Vec<String>,
    pub rows_restored: u64,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub duration_ms: u64,
    pub verify_passed: bool,
}

/// Backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub database_type: DatabaseType,
    pub database_name: String,
    pub server_version: String,
    pub backup_type: BackupType,
    pub tables: Vec<TableBackupInfo>,
    pub compression: CompressionInfo,
    pub encryption: Option<EncryptionInfo>,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableBackupInfo {
    pub name: String,
    pub row_count: u64,
    pub schema_size_bytes: u64,
    pub data_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionInfo {
    pub algorithm: String,
    pub level: u32,
    pub original_size: u64,
    pub compressed_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionInfo {
    pub algorithm: String,
    pub key_hash: String,
}

/// Backup manager
pub struct BackupManager;

impl BackupManager {
    pub fn new() -> Self {
        Self
    }

    /// Create a full database backup
    pub async fn backup(
        &self,
        config: &DatabaseConfig,
        backup_config: &BackupConfig,
        output_path: &Path,
    ) -> Result<BackupResult, DatabaseError> {
        let start = std::time::Instant::now();

        // Determine backup format based on database type
        match config.db_type {
            DatabaseType::SQLite => {
                self.backup_sqlite(config, backup_config, output_path).await
            }
            DatabaseType::MySQL => {
                self.backup_mysql(config, backup_config, output_path).await
            }
            DatabaseType::PostgreSQL => {
                self.backup_postgres(config, backup_config, output_path).await
            }
            _ => Err(DatabaseError::BackupError(
                format!("Backup not supported for {:?}", config.db_type)
            )),
        }?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Calculate file size and checksum
        let metadata = std::fs::metadata(output_path)
            .map_err(|e| DatabaseError::BackupError(e.to_string()))?;

        let file_size = metadata.len();
        let checksum = self.calculate_checksum(output_path).await?;

        Ok(BackupResult {
            success: true,
            file_path: output_path.to_path_buf(),
            file_size_bytes: file_size,
            tables_backed_up: Vec::new(), // Would be populated by actual backup
            rows_backed_up: 0,
            duration_ms,
            compression_ratio: 1.0, // Would be calculated
            checksum,
            warnings: Vec::new(),
            created_at: Utc::now(),
        })
    }

    async fn backup_sqlite(
        &self,
        config: &DatabaseConfig,
        _backup_config: &BackupConfig,
        output_path: &Path,
    ) -> Result<(), DatabaseError> {
        // SQLite backup is simple file copy or use backup API
        let source = std::path::Path::new(&config.database);

        if !source.exists() {
            return Err(DatabaseError::BackupError(
                format!("Source database not found: {}", config.database)
            ));
        }

        // For SQLite, we can just copy the file
        std::fs::copy(source, output_path)
            .map_err(|e| DatabaseError::BackupError(e.to_string()))?;

        Ok(())
    }

    async fn backup_mysql(
        &self,
        _config: &DatabaseConfig,
        _backup_config: &BackupConfig,
        _output_path: &Path,
    ) -> Result<(), DatabaseError> {
        // Would execute mysqldump command
        Err(DatabaseError::BackupError(
            "MySQL backup requires external mysqldump tool".to_string()
        ))
    }

    async fn backup_postgres(
        &self,
        _config: &DatabaseConfig,
        _backup_config: &BackupConfig,
        _output_path: &Path,
    ) -> Result<(), DatabaseError> {
        // Would execute pg_dump command
        Err(DatabaseError::BackupError(
            "PostgreSQL backup requires external pg_dump tool".to_string()
        ))
    }

    /// Restore from backup
    pub async fn restore(
        &self,
        config: &DatabaseConfig,
        restore_config: &RestoreConfig,
        backup_path: &Path,
    ) -> Result<RestoreResult, DatabaseError> {
        let start = std::time::Instant::now();

        // Verify backup file
        if !backup_path.exists() {
            return Err(DatabaseError::BackupError(
                "Backup file not found".to_string()
            ));
        }

        if restore_config.verify_before_restore {
            self.verify_backup(backup_path).await?;
        }

        // Perform restore based on database type
        let result = match config.db_type {
            DatabaseType::SQLite => {
                self.restore_sqlite(config, restore_config, backup_path).await
            }
            DatabaseType::MySQL => {
                self.restore_mysql(config, restore_config, backup_path).await
            }
            DatabaseType::PostgreSQL => {
                self.restore_postgres(config, restore_config, backup_path).await
            }
            _ => Err(DatabaseError::BackupError(
                format!("Restore not supported for {:?}", config.db_type)
            )),
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        result.map(|_| RestoreResult {
            success: true,
            tables_restored: Vec::new(),
            rows_restored: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
            duration_ms,
            verify_passed: true,
        })
    }

    async fn restore_sqlite(
        &self,
        config: &DatabaseConfig,
        restore_config: &RestoreConfig,
        backup_path: &Path,
    ) -> Result<(), DatabaseError> {
        if restore_config.dry_run {
            return Ok(());
        }

        let target = std::path::Path::new(&config.database);

        // Create target directory if needed
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DatabaseError::BackupError(e.to_string()))?;
        }

        // Copy backup to target
        std::fs::copy(backup_path, target)
            .map_err(|e| DatabaseError::BackupError(e.to_string()))?;

        Ok(())
    }

    async fn restore_mysql(
        &self,
        _config: &DatabaseConfig,
        _restore_config: &RestoreConfig,
        _backup_path: &Path,
    ) -> Result<(), DatabaseError> {
        Err(DatabaseError::BackupError(
            "MySQL restore requires external mysql tool".to_string()
        ))
    }

    async fn restore_postgres(
        &self,
        _config: &DatabaseConfig,
        _restore_config: &RestoreConfig,
        _backup_path: &Path,
    ) -> Result<(), DatabaseError> {
        Err(DatabaseError::BackupError(
            "PostgreSQL restore requires external psql/pg_restore tools".to_string()
        ))
    }

    /// Verify backup integrity
    pub async fn verify_backup(&self, backup_path: &Path) -> Result<bool, DatabaseError> {
        // Check if file exists and has content
        let metadata = std::fs::metadata(backup_path)
            .map_err(|e| DatabaseError::BackupError(e.to_string()))?;

        if metadata.len() == 0 {
            return Err(DatabaseError::BackupError(
                "Backup file is empty".to_string()
            ));
        }

        // Would also verify checksum, try to open file, etc.
        Ok(true)
    }

    /// Calculate file checksum
    async fn calculate_checksum(&self, path: &Path) -> Result<String, DatabaseError> {
        use sha2::{Sha256, Digest};

        let content = std::fs::read(path)
            .map_err(|e| DatabaseError::BackupError(e.to_string()))?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let result = hasher.finalize();

        Ok(format!("{:x}", result))
    }

    /// List available backups in directory
    pub fn list_backups(&self, backup_dir: &Path) -> Result<Vec<BackupInfo>, DatabaseError> {
        let mut backups = Vec::new();

        if let Ok(entries) = std::fs::read_dir(backup_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "backup" || e == "sql" || e == "db") {
                    if let Ok(metadata) = entry.metadata() {
                        backups.push(BackupInfo {
                            file_path: path.clone(),
                            file_name: path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default(),
                            size_bytes: metadata.len(),
                            created_at: metadata.created()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| DateTime::from_timestamp(d.as_secs() as i64, 0))
                                .flatten()
                                .unwrap_or_else(Utc::now),
                            database_type: None,
                        });
                    }
                }
            }
        }

        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(backups)
    }

    /// Read backup metadata
    pub async fn read_metadata(&self, backup_path: &Path) -> Result<BackupMetadata, DatabaseError> {
        // Would read metadata from backup file header
        Ok(BackupMetadata {
            version: "1.0".to_string(),
            created_at: Utc::now(),
            database_type: DatabaseType::SQLite,
            database_name: "unknown".to_string(),
            server_version: "unknown".to_string(),
            backup_type: BackupType::Full,
            tables: Vec::new(),
            compression: CompressionInfo {
                algorithm: "none".to_string(),
                level: 0,
                original_size: 0,
                compressed_size: 0,
            },
            encryption: None,
            checksum: String::new(),
        })
    }
}

impl Default for BackupManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Backup file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub file_path: PathBuf,
    pub file_name: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
    pub database_type: Option<DatabaseType>,
}

/// Scheduled backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledBackup {
    pub id: String,
    pub name: String,
    pub connection_id: String,
    pub backup_config: BackupConfig,
    pub schedule: BackupSchedule,
    pub retention_days: u32,
    pub backup_directory: PathBuf,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub run_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSchedule {
    pub frequency: BackupFrequency,
    pub day_of_week: Option<u8>,
    pub day_of_month: Option<u8>,
    pub hour: u8,
    pub minute: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupFrequency {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

/// Backup scheduler
pub struct BackupScheduler {
    schedules: Vec<ScheduledBackup>,
}

impl BackupScheduler {
    pub fn new() -> Self {
        Self {
            schedules: Vec::new(),
        }
    }

    pub fn add_schedule(&mut self, schedule: ScheduledBackup) {
        self.schedules.push(schedule);
    }

    pub fn remove_schedule(&mut self, id: &str) -> bool {
        if let Some(pos) = self.schedules.iter().position(|s| s.id == id) {
            self.schedules.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn get_due_backups(&self) -> Vec<&ScheduledBackup> {
        let now = Utc::now();
        self.schedules.iter()
            .filter(|s| s.enabled)
            .filter(|s| s.next_run.map_or(true, |next| next <= now))
            .collect()
    }

    pub fn calculate_next_run(&self, schedule: &BackupSchedule) -> DateTime<Utc> {
        let now = Utc::now();
        let today = now.date_naive();

        match schedule.frequency {
            BackupFrequency::Hourly => {
                now + chrono::Duration::hours(1)
            }
            BackupFrequency::Daily => {
                let next = today.succ_opt().unwrap_or(today);
                next.and_hms_opt(schedule.hour as u32, schedule.minute as u32, 0)
                    .map(|t| DateTime::from_naive_utc_and_offset(t, chrono::Utc))
                    .unwrap_or(now)
            }
            BackupFrequency::Weekly => {
                // Calculate next occurrence of day_of_week
                now + chrono::Duration::days(7)
            }
            BackupFrequency::Monthly => {
                now + chrono::Duration::days(30)
            }
        }
    }
}

impl Default for BackupScheduler {
    fn default() -> Self {
        Self::new()
    }
}
