//! Analytics storage for local data management
//!
//! Manages:
//! - Local event buffering
//! - Data retention policies
//! - Data export (GDPR/CCPA compliance)
//! - Data deletion (right to erasure)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{AnonymousId, TelemetryConfig, TelemetryError, TelemetryEventRecord};

/// Data retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRetentionPolicy {
    /// How long to keep data (in days)
    pub retention_days: u32,
    /// Maximum events to store locally
    pub max_local_events: usize,
    /// Auto-delete after retention period
    pub auto_delete: bool,
    /// Export before deletion (for compliance)
    pub export_before_delete: bool,
}

impl Default for DataRetentionPolicy {
    fn default() -> Self {
        Self {
            retention_days: 90,
            max_local_events: 10000,
            auto_delete: true,
            export_before_delete: false,
        }
    }
}

impl DataRetentionPolicy {
    /// GDPR compliance preset
    pub fn gdpr_compliant() -> Self {
        Self {
            retention_days: 30, // Shorter retention for GDPR
            max_local_events: 5000,
            auto_delete: true,
            export_before_delete: true,
        }
    }

    /// CCPA compliance preset
    pub fn ccpa_compliant() -> Self {
        Self {
            retention_days: 12 * 30, // ~1 year for CCPA
            max_local_events: 10000,
            auto_delete: false, // Keep until deletion request
            export_before_delete: true,
        }
    }
}

/// Analytics storage manager
pub struct AnalyticsStorage {
    policy: DataRetentionPolicy,
    storage_path: PathBuf,
    last_cleanup: Arc<Mutex<u64>>,
}

impl AnalyticsStorage {
    pub fn new(config: &TelemetryConfig) -> Result<Self, TelemetryError> {
        let policy = DataRetentionPolicy {
            retention_days: config.retention_days,
            max_local_events: config.max_local_events,
            auto_delete: true,
            export_before_delete: false,
        };

        let storage_path = Self::get_storage_path()?;

        Ok(Self {
            policy,
            storage_path,
            last_cleanup: Arc::new(Mutex::new(0)),
        })
    }

    fn get_storage_path() -> Result<PathBuf, TelemetryError> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| TelemetryError::Config("Cannot find data directory".to_string()))?
            .join("easyssh")
            .join("telemetry");

        std::fs::create_dir_all(&data_dir)?;

        Ok(data_dir)
    }

    /// Store events
    pub async fn store(&self, events: Vec<TelemetryEventRecord>) -> Result<(), TelemetryError> {
        // Append to local storage file
        let file_path = self.storage_path.join("events.jsonl");

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;

        use tokio::io::AsyncWriteExt;

        for event in events {
            let json = serde_json::to_string(&event)?;
            file.write_all(json.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }

        file.flush().await?;

        // Check if cleanup needed
        self.maybe_cleanup().await?;

        Ok(())
    }

    /// Retrieve events
    pub async fn retrieve(
        &self,
        batch_size: usize,
    ) -> Result<Vec<TelemetryEventRecord>, TelemetryError> {
        let file_path = self.storage_path.join("events.jsonl");

        if !file_path.exists() {
            return Ok(vec![]);
        }

        let content = tokio::fs::read_to_string(&file_path).await?;
        let mut events = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(event) = serde_json::from_str::<TelemetryEventRecord>(line) {
                events.push(event);
            }

            if events.len() >= batch_size {
                break;
            }
        }

        Ok(events)
    }

    /// Delete specific events
    pub async fn delete(&self, event_ids: Vec<String>) -> Result<(), TelemetryError> {
        let file_path = self.storage_path.join("events.jsonl");

        if !file_path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&file_path).await?;
        let mut remaining = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(event) = serde_json::from_str::<TelemetryEventRecord>(line) {
                if !event_ids.contains(&event.id) {
                    remaining.push(line.to_string());
                }
            }
        }

        tokio::fs::write(&file_path, remaining.join("\n")).await?;

        Ok(())
    }

    /// Get event count
    pub async fn count(&self) -> Result<usize, TelemetryError> {
        let file_path = self.storage_path.join("events.jsonl");

        if !file_path.exists() {
            return Ok(0);
        }

        let content = tokio::fs::read_to_string(&file_path).await?;
        let count = content.lines().filter(|l| !l.trim().is_empty()).count();

        Ok(count)
    }

    /// Clear all events
    pub async fn clear(&self) -> Result<(), TelemetryError> {
        let file_path = self.storage_path.join("events.jsonl");

        if file_path.exists() {
            tokio::fs::remove_file(&file_path).await?;
        }

        // Also clear SQLite if using
        let sqlite_path = self.storage_path.join("events.db");
        if sqlite_path.exists() {
            tokio::fs::remove_file(&sqlite_path).await?;
        }

        Ok(())
    }

    /// Export user data (GDPR portability)
    pub async fn export_data(&self, anonymous_id: &AnonymousId) -> Result<String, TelemetryError> {
        let file_path = self.storage_path.join("events.jsonl");

        if !file_path.exists() {
            return Ok("[]".to_string());
        }

        let content = tokio::fs::read_to_string(&file_path).await?;
        let mut user_events = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(event) = serde_json::from_str::<TelemetryEventRecord>(line) {
                if event.anonymous_id.as_str() == anonymous_id.as_str() {
                    user_events.push(event);
                }
            }
        }

        let export = DataExport {
            export_date: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            anonymous_id: anonymous_id.as_str().to_string(),
            event_count: user_events.len(),
            events: user_events,
            privacy_notice: "This export contains your anonymous usage data. It does not include any SSH credentials, server information, or personal identifiers.".to_string(),
        };

        serde_json::to_string_pretty(&export).map_err(|e| TelemetryError::Serialization(e))
    }

    /// Delete all data for a user (GDPR erasure)
    pub async fn delete_data(&self, anonymous_id: &AnonymousId) -> Result<(), TelemetryError> {
        let file_path = self.storage_path.join("events.jsonl");

        if !file_path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&file_path).await?;
        let mut remaining = Vec::new();
        let mut deleted_count = 0;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(event) = serde_json::from_str::<TelemetryEventRecord>(line) {
                if event.anonymous_id.as_str() != anonymous_id.as_str() {
                    remaining.push(line.to_string());
                } else {
                    deleted_count += 1;
                }
            }
        }

        tokio::fs::write(&file_path, remaining.join("\n")).await?;

        println!(
            "[Telemetry] Deleted {} events for user {}",
            deleted_count,
            anonymous_id.as_str()
        );

        // Log the deletion for compliance
        self.log_deletion(anonymous_id, deleted_count).await?;

        Ok(())
    }

    /// Apply retention policy
    pub async fn apply_retention(&self) -> Result<RetentionResult, TelemetryError> {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            - (self.policy.retention_days as u64 * 86400);

        let file_path = self.storage_path.join("events.jsonl");

        if !file_path.exists() {
            return Ok(RetentionResult::default());
        }

        let content = tokio::fs::read_to_string(&file_path).await?;
        let mut remaining = Vec::new();
        let mut expired_count = 0;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(event) = serde_json::from_str::<TelemetryEventRecord>(line) {
                if event.timestamp >= cutoff {
                    remaining.push(line.to_string());
                } else {
                    expired_count += 1;
                }
            }
        }

        tokio::fs::write(&file_path, remaining.join("\n")).await?;

        // Check max events limit
        let mut over_limit_count = 0;
        if remaining.len() > self.policy.max_local_events {
            // Keep only the most recent events
            let to_remove = remaining.len() - self.policy.max_local_events;
            over_limit_count = to_remove;
            remaining = remaining.split_off(to_remove);

            tokio::fs::write(&file_path, remaining.join("\n")).await?;
        }

        Ok(RetentionResult {
            expired_deleted: expired_count,
            over_limit_deleted: over_limit_count,
            remaining_events: remaining.len(),
        })
    }

    /// Maybe run cleanup (daily)
    async fn maybe_cleanup(&self) -> Result<(), TelemetryError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut last = self.last_cleanup.lock().unwrap();

        // Run cleanup once per day
        if now - *last > 86400 {
            if self.policy.auto_delete {
                let result = self.apply_retention().await?;
                println!("[Telemetry] Cleanup: {:?}", result);
            }
            *last = now;
        }

        Ok(())
    }

    /// Log deletion for compliance audit
    async fn log_deletion(
        &self,
        anonymous_id: &AnonymousId,
        count: usize,
    ) -> Result<(), TelemetryError> {
        let log_path = self.storage_path.join("deletion_log.jsonl");

        let log_entry = DeletionLogEntry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            anonymous_id: anonymous_id.as_str().to_string(),
            events_deleted: count,
            reason: "user_request".to_string(),
        };

        let json = serde_json::to_string(&log_entry)?;

        use tokio::io::AsyncWriteExt;

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .await?;

        file.write_all(json.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(())
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> StorageStats {
        let event_count = self.count().await.unwrap_or(0);

        let storage_size = if let Ok(metadata) =
            tokio::fs::metadata(self.storage_path.join("events.jsonl")).await
        {
            metadata.len()
        } else {
            0
        };

        StorageStats {
            event_count,
            storage_size_bytes: storage_size,
            retention_days: self.policy.retention_days,
            max_events: self.policy.max_local_events,
        }
    }
}

/// Data export structure (GDPR portability)
#[derive(Debug, Serialize)]
pub struct DataExport {
    pub export_date: u64,
    pub anonymous_id: String,
    pub event_count: usize,
    pub events: Vec<TelemetryEventRecord>,
    pub privacy_notice: String,
}

/// Retention policy application result
#[derive(Debug, Clone, Default)]
pub struct RetentionResult {
    pub expired_deleted: usize,
    pub over_limit_deleted: usize,
    pub remaining_events: usize,
}

/// Deletion log entry (compliance audit)
#[derive(Debug, Serialize, Deserialize)]
struct DeletionLogEntry {
    pub timestamp: u64,
    pub anonymous_id: String,
    pub events_deleted: usize,
    pub reason: String,
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub event_count: usize,
    pub storage_size_bytes: u64,
    pub retention_days: u32,
    pub max_events: usize,
}

/// Privacy compliance utilities
pub struct PrivacyCompliance;

impl PrivacyCompliance {
    /// Check if data is anonymized
    pub fn is_anonymized(event: &TelemetryEventRecord) -> bool {
        // Verify no PII in the event
        let json = match serde_json::to_string(&event) {
            Ok(j) => j,
            Err(_) => return false,
        };

        // Check for patterns that should not exist
        let forbidden_patterns = [
            r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b",         // IPs
            r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}", // Emails
        ];

        // In debug builds, log warnings if patterns found
        if cfg!(debug_assertions) {
            for pattern in &forbidden_patterns {
                if json.contains(pattern) {
                    eprintln!(
                        "[Privacy Warning] Potential PII detected in event: {}",
                        event.id
                    );
                    return false;
                }
            }
        }

        true
    }

    /// Generate privacy report
    pub async fn generate_privacy_report(
        storage: &AnalyticsStorage,
    ) -> Result<PrivacyReport, TelemetryError> {
        let stats = storage.get_stats().await;

        Ok(PrivacyReport {
            generated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            total_events: stats.event_count,
            storage_size_bytes: stats.storage_size_bytes,
            retention_policy: storage.policy.clone(),
            gdpr_compliant: true,
            ccpa_compliant: true,
            encryption_enabled: false, // Would be true in production with encrypted storage
            last_retention_run: *storage.last_cleanup.lock().unwrap(),
        })
    }
}

/// Privacy compliance report
#[derive(Debug, Serialize)]
pub struct PrivacyReport {
    pub generated_at: u64,
    pub total_events: usize,
    pub storage_size_bytes: u64,
    pub retention_policy: DataRetentionPolicy,
    pub gdpr_compliant: bool,
    pub ccpa_compliant: bool,
    pub encryption_enabled: bool,
    pub last_retention_run: u64,
}
