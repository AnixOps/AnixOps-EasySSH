//! Sync types - Data structures for synchronization

use crate::error::LiteError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Sync document type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SyncDocumentType {
    Host,
    Group,
    Identity,
    Tag,
    Snippet,
    Layout,
    Setting,
    KnownHost,
    VaultItem,
}

impl SyncDocumentType {
    /// Get priority for conflict resolution ordering
    pub fn priority(&self) -> u8 {
        match self {
            SyncDocumentType::Setting => 0,
            SyncDocumentType::Group => 1,
            SyncDocumentType::Tag => 2,
            SyncDocumentType::Identity => 3,
            SyncDocumentType::Host => 4,
            SyncDocumentType::Snippet => 5,
            SyncDocumentType::Layout => 6,
            SyncDocumentType::KnownHost => 7,
            SyncDocumentType::VaultItem => 8,
        }
    }

    /// Check if supports field-level merge
    pub fn supports_field_merge(&self) -> bool {
        matches!(
            self,
            SyncDocumentType::Host
                | SyncDocumentType::Group
                | SyncDocumentType::Identity
                | SyncDocumentType::Snippet
        )
    }
}

/// Sync operation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncOperation {
    Create,
    Update,
    Delete,
}

/// Sync document - encrypted configuration item
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncDocument {
    pub id: String,
    pub doc_type: SyncDocumentType,
    pub device_id: String,
    pub operation: SyncOperation,
    pub timestamp: i64,
    pub vector_clock: HashMap<String, u64>,
    pub encrypted_data: Vec<u8>,
    pub content_hash: String,
    pub parent_hashes: Vec<String>,
    pub deleted: bool,
    pub group_id: Option<String>,
    pub schema_version: u32,
}

impl SyncDocument {
    /// Create new sync document
    pub fn new(
        id: String,
        doc_type: SyncDocumentType,
        device_id: String,
        encrypted_data: Vec<u8>,
        content_hash: String,
    ) -> Self {
        Self {
            id,
            doc_type,
            device_id,
            operation: SyncOperation::Update,
            timestamp: chrono::Utc::now().timestamp_millis(),
            vector_clock: HashMap::new(),
            encrypted_data,
            content_hash,
            parent_hashes: Vec::new(),
            deleted: false,
            group_id: None,
            schema_version: 1,
        }
    }

    /// Update vector clock
    pub fn tick_clock(&mut self, device_id: &str) {
        let counter = self.vector_clock.entry(device_id.to_string()).or_insert(0);
        *counter += 1;
    }

    /// Merge vector clock (take max values)
    pub fn merge_clock(&mut self, other: &HashMap<String, u64>) {
        for (device, counter) in other {
            let entry = self.vector_clock.entry(device.clone()).or_insert(0);
            *entry = (*entry).max(*counter);
        }
    }

    /// Check if conflicts with another document
    pub fn has_conflict_with(&self, other: &SyncDocument) -> bool {
        !self.is_ancestor_of(other) && !other.is_ancestor_of(self)
    }

    /// Check if this document is ancestor of another (happens-before)
    pub fn is_ancestor_of(&self, other: &SyncDocument) -> bool {
        if self.content_hash == other.content_hash {
            return true;
        }

        let mut all_lte = true;
        let mut at_least_one_lt = false;

        for (device, self_counter) in &self.vector_clock {
            let other_counter = other.vector_clock.get(device).copied().unwrap_or(0);
            if *self_counter > other_counter {
                all_lte = false;
                break;
            }
            if *self_counter < other_counter {
                at_least_one_lt = true;
            }
        }

        for (device, other_counter) in &other.vector_clock {
            if !self.vector_clock.contains_key(device) && *other_counter > 0 {
                at_least_one_lt = true;
            }
        }

        all_lte && at_least_one_lt
    }
}

/// Sync bundle - batch sync data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncBundle {
    pub bundle_id: String,
    pub device_id: String,
    pub timestamp: i64,
    pub documents: Vec<SyncDocument>,
    pub checkpoint: String,
    pub compressed: bool,
    pub schema_version: u32,
}

impl SyncBundle {
    /// Create new sync bundle
    pub fn new(device_id: String, documents: Vec<SyncDocument>) -> Self {
        Self {
            bundle_id: Uuid::new_v4().to_string(),
            device_id,
            timestamp: chrono::Utc::now().timestamp_millis(),
            documents,
            checkpoint: String::new(),
            compressed: false,
            schema_version: 1,
        }
    }

    /// Calculate bundle size in bytes
    pub fn size_bytes(&self) -> usize {
        self.documents.iter().map(|d| d.encrypted_data.len()).sum()
    }
}

/// Sync metadata stored in cloud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    pub version: String,
    pub device_count: u32,
    pub last_modified: i64,
    pub encryption_key_hash: String,
    pub total_documents: u64,
    pub schema_version: u32,
}

impl Default for SyncMetadata {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            device_count: 0,
            last_modified: 0,
            encryption_key_hash: String::new(),
            total_documents: 0,
            schema_version: 1,
        }
    }
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,
    pub platform: String,
    pub last_seen: i64,
    pub capabilities: Vec<String>,
    pub app_version: String,
}

/// Sync history version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncVersion {
    pub version_id: String,
    pub timestamp: i64,
    pub device_id: String,
    pub description: Option<String>,
    pub document_count: u32,
    pub size_bytes: u64,
    pub tags: Vec<String>,
}

/// Sync scope configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncScope {
    pub include_all: bool,
    pub included_groups: Vec<String>,
    pub excluded_groups: Vec<String>,
    pub include_identities: bool,
    pub include_snippets: bool,
    pub include_layouts: bool,
    pub include_settings: bool,
    pub include_known_hosts: bool,
    pub include_vault_items: bool,
}

impl Default for SyncScope {
    fn default() -> Self {
        Self {
            include_all: true,
            included_groups: Vec::new(),
            excluded_groups: Vec::new(),
            include_identities: true,
            include_snippets: true,
            include_layouts: true,
            include_settings: true,
            include_known_hosts: true,
            include_vault_items: true,
        }
    }
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub enabled: bool,
    pub device_id: String,
    pub device_name: String,
    pub encryption_key: Option<String>,
    pub provider: SyncProvider,
    pub scope: SyncScope,
    pub auto_sync: bool,
    pub sync_interval_secs: u64,
    pub conflict_resolution: crate::sync::conflict::SyncConflictResolution,
    pub local_sync_enabled: bool,
    pub max_history_versions: u32,
    pub last_sync_at: Option<i64>,
    pub deduplication_enabled: bool,
    pub compression_enabled: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            device_id: Uuid::new_v4().to_string(),
            device_name: "EasySSH Device".to_string(),
            encryption_key: None,
            provider: SyncProvider::Disabled,
            scope: SyncScope::default(),
            auto_sync: true,
            sync_interval_secs: 300,
            conflict_resolution: crate::sync::conflict::SyncConflictResolution::default(),
            local_sync_enabled: false,
            max_history_versions: 10,
            last_sync_at: None,
            deduplication_enabled: true,
            compression_enabled: true,
        }
    }
}

/// Sync provider type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncProvider {
    Disabled,
    ICloud,
    GoogleDrive,
    OneDrive,
    DropBox,
    SelfHosted { url: String, token: String },
    LocalNetwork,
    CustomPath(PathBuf),
}

/// Sync status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Idle,
    Initializing,
    CheckingConnectivity,
    FetchingRemote,
    ResolvingConflicts,
    Uploading,
    Downloading,
    ApplyingChanges,
    CreatingVersion,
    Syncing,
    Error(String),
    Offline,
    Conflict(Vec<crate::sync::conflict::SyncConflict>),
}

/// Sync statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncStats {
    pub last_sync_at: Option<i64>,
    pub documents_synced: u64,
    pub documents_uploaded: u64,
    pub documents_downloaded: u64,
    pub conflicts_resolved: u32,
    pub conflicts_pending: u32,
    pub bytes_uploaded: u64,
    pub bytes_downloaded: u64,
    pub sync_duration_ms: u64,
    pub deduplicated_count: u32,
    pub compression_ratio: f64,
}

/// Local network sync beacon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSyncBeacon {
    pub device_id: String,
    pub device_name: String,
    pub port: u16,
    pub protocol_version: u32,
    pub timestamp: i64,
    pub signature: Vec<u8>,
    pub public_key_fingerprint: String,
}

/// Raw config data (before encryption)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawConfigData {
    pub id: String,
    pub doc_type: SyncDocumentType,
    pub data: serde_json::Value,
    pub updated_at: i64,
    pub deleted: bool,
}

/// Sync event
#[derive(Debug, Clone)]
pub enum SyncEvent {
    Started,
    Initializing,
    Progress {
        current: u32,
        total: u32,
        message: String,
    },
    DocumentSynced {
        id: String,
        doc_type: SyncDocumentType,
        operation: SyncOperation,
    },
    ConflictDetected {
        conflict: crate::sync::conflict::SyncConflict,
    },
    ConflictResolved {
        document_id: String,
        resolution: crate::sync::conflict::SyncConflictResolution,
    },
    VersionCreated {
        version: SyncVersion,
    },
    Completed {
        stats: SyncStats,
    },
    Error {
        error: String,
    },
    DeviceDiscovered {
        device: DeviceInfo,
    },
    ConnectivityChanged {
        online: bool,
    },
}

/// Compute BLAKE3 hash (faster than SHA-256, equally secure)
pub fn blake3_hash(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// Compute SHA-256 hash (for compatibility)
pub fn sha256_hash(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}
