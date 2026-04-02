//! Sync engine - Core synchronization logic

use crate::crypto::CryptoState;
use crate::db::Database;
use crate::error::LiteError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};

use super::conflict::{ConflictResolver, SyncConflict, SyncConflictResolution};
use super::providers::{LocalSyncHandler, SyncProviderImpl};
use super::types::{
    blake3_hash, DeviceInfo, RawConfigData, SyncBundle, SyncConfig, SyncDocument, SyncDocumentType,
    SyncEvent, SyncOperation, SyncProvider, SyncScope, SyncStats, SyncStatus, SyncVersion,
};

/// Sync manager
pub struct SyncManager {
    config: Arc<RwLock<SyncConfig>>,
    status: Arc<RwLock<SyncStatus>>,
    stats: Arc<RwLock<SyncStats>>,
    crypto: Arc<Mutex<CryptoState>>,
    db: Arc<Mutex<Database>>,
    provider: Arc<Mutex<Box<dyn SyncProviderImpl + Send>>>,
    local_sync: Arc<Mutex<LocalSyncHandler>>,
    event_tx: mpsc::UnboundedSender<SyncEvent>,
    event_rx: Arc<Mutex<mpsc::UnboundedReceiver<SyncEvent>>>,
    vector_clock: Arc<RwLock<HashMap<String, u64>>>,
    history: Arc<RwLock<Vec<SyncVersion>>>,
    content_hashes: Arc<RwLock<HashMap<String, String>>>,
}

impl SyncManager {
    /// Create new sync manager
    pub fn new(db: Database, config: SyncConfig) -> Result<Self, LiteError> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let mut crypto = CryptoState::new();
        if let Some(key) = &config.encryption_key {
            crypto.initialize(key)?;
        }

        let provider: Box<dyn SyncProviderImpl + Send> = match &config.provider {
            SyncProvider::Disabled => Box::new(super::providers::DisabledProvider::new()),
            SyncProvider::ICloud => Box::new(super::providers::ICloudProvider::new()),
            SyncProvider::GoogleDrive => Box::new(super::providers::GoogleDriveProvider::new()),
            SyncProvider::OneDrive => Box::new(super::providers::OneDriveProvider::new()),
            SyncProvider::DropBox => Box::new(super::providers::DropBoxProvider::new()),
            SyncProvider::SelfHosted { url, token } => Box::new(
                super::providers::SelfHostedProvider::new(url.clone(), token.clone()),
            ),
            SyncProvider::LocalNetwork => Box::new(super::providers::LocalNetworkProvider::new()),
            SyncProvider::CustomPath(path) => {
                Box::new(super::providers::LocalFileProvider::new(path.clone()))
            }
        };

        let manager = Self {
            config: Arc::new(RwLock::new(config)),
            status: Arc::new(RwLock::new(SyncStatus::Idle)),
            stats: Arc::new(RwLock::new(SyncStats::default())),
            crypto: Arc::new(Mutex::new(crypto)),
            db: Arc::new(Mutex::new(db)),
            provider: Arc::new(Mutex::new(provider)),
            local_sync: Arc::new(Mutex::new(LocalSyncHandler::new())),
            event_tx: event_tx.clone(),
            event_rx: Arc::new(Mutex::new(event_rx)),
            vector_clock: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            content_hashes: Arc::new(RwLock::new(HashMap::new())),
        };

        Ok(manager)
    }

    /// Start sync manager
    pub async fn start(&self) -> Result<(), LiteError> {
        let config = self.config.read().await;

        if !config.enabled {
            return Ok(());
        }

        *self.status.write().await = SyncStatus::Initializing;
        let _ = self.event_tx.send(SyncEvent::Initializing);

        let mut provider = self.provider.lock().await;
        provider.initialize(&config).await?;
        drop(provider);

        if config.local_sync_enabled {
            let mut local_sync = self.local_sync.lock().await;
            local_sync.enable(0);
        }

        let _ = self.event_tx.send(SyncEvent::Started);
        *self.status.write().await = SyncStatus::Idle;

        Ok(())
    }

    /// Perform full sync
    pub async fn sync(&self) -> Result<SyncStats, LiteError> {
        let start_time = std::time::Instant::now();
        let config = self.config.read().await.clone();

        if !config.enabled {
            return Ok(SyncStats::default());
        }

        *self.status.write().await = SyncStatus::Syncing;
        self.event_tx.send(SyncEvent::Started).ok();

        // 1. Check connectivity
        *self.status.write().await = SyncStatus::CheckingConnectivity;
        let provider = self.provider.lock().await;
        let is_online = provider.check_connectivity().await.unwrap_or(false);
        drop(provider);

        if !is_online {
            *self.status.write().await = SyncStatus::Offline;
            self.event_tx
                .send(SyncEvent::ConnectivityChanged { online: false })
                .ok();
            return Err(LiteError::Config("No network connection".to_string()));
        }

        self.event_tx
            .send(SyncEvent::ConnectivityChanged { online: true })
            .ok();

        // 2. Get local changes
        self.event_tx
            .send(SyncEvent::Progress {
                current: 10,
                total: 100,
                message: "Fetching local changes...".to_string(),
            })
            .ok();

        let local_docs = self.get_local_changes().await?;
        let local_doc_count = local_docs.len() as u32;

        // 3. Get remote changes
        self.event_tx
            .send(SyncEvent::Progress {
                current: 20,
                total: 100,
                message: "Fetching remote changes...".to_string(),
            })
            .ok();

        let last_sync = config.last_sync_at.unwrap_or(0);
        let provider = self.provider.lock().await;
        let remote_bundles = provider.download_bundles(last_sync).await?;
        drop(provider);

        let mut remote_docs: Vec<SyncDocument> = Vec::new();
        for bundle in &remote_bundles {
            remote_docs.extend(bundle.documents.clone());
        }

        // 4. Deduplicate
        if config.deduplication_enabled {
            self.event_tx
                .send(SyncEvent::Progress {
                    current: 30,
                    total: 100,
                    message: "Deduplicating...".to_string(),
                })
                .ok();
            remote_docs = self.deduplicate_documents(remote_docs).await;
        }

        // 5. Resolve conflicts
        *self.status.write().await = SyncStatus::ResolvingConflicts;
        self.event_tx
            .send(SyncEvent::Progress {
                current: 40,
                total: 100,
                message: "Resolving conflicts...".to_string(),
            })
            .ok();

        let conflicts = self.detect_conflicts(&local_docs, &remote_docs).await?;
        let mut resolved_conflicts = 0;
        let mut pending_conflicts: Vec<SyncConflict> = Vec::new();

        if !conflicts.is_empty() {
            let strategy = config.conflict_resolution.clone();

            for conflict in conflicts {
                match self.resolve_conflict(&conflict, &strategy).await {
                    Ok(resolution) => {
                        self.event_tx
                            .send(SyncEvent::ConflictResolved {
                                document_id: conflict.document_id.clone(),
                                resolution: resolution.clone(),
                            })
                            .ok();

                        if resolution != SyncConflictResolution::Skip {
                            resolved_conflicts += 1;
                        }

                        if resolution == SyncConflictResolution::Interactive {
                            let mut pending = conflict.clone();
                            pending.resolution = Some(resolution);
                            pending_conflicts.push(pending);
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to resolve conflict for {}: {}",
                            conflict.document_id, e
                        );
                    }
                }
            }
        }

        // 6. Apply remote changes
        *self.status.write().await = SyncStatus::ApplyingChanges;
        self.event_tx
            .send(SyncEvent::Progress {
                current: 60,
                total: 100,
                message: "Applying remote changes...".to_string(),
            })
            .ok();

        let mut applied_count = 0;
        let mut downloaded_bytes = 0u64;

        for doc in &remote_docs {
            if self.should_apply_document(doc).await? {
                match self.apply_document_to_local(doc).await {
                    Ok(_) => {
                        applied_count += 1;
                        downloaded_bytes += doc.encrypted_data.len() as u64;
                        self.event_tx
                            .send(SyncEvent::DocumentSynced {
                                id: doc.id.clone(),
                                doc_type: doc.doc_type.clone(),
                                operation: doc.operation.clone(),
                            })
                            .ok();
                    }
                    Err(e) => {
                        error!("Failed to apply document {}: {}", doc.id, e);
                    }
                }
            }
        }

        // 7. Upload local changes
        *self.status.write().await = SyncStatus::Uploading;
        self.event_tx
            .send(SyncEvent::Progress {
                current: 80,
                total: 100,
                message: "Uploading local changes...".to_string(),
            })
            .ok();

        let mut uploaded_bytes = 0u64;
        let uploaded_count = local_docs.len() as u64;

        if !local_docs.is_empty() {
            let bundle = self.create_sync_bundle(local_docs).await?;
            uploaded_bytes = bundle.size_bytes() as u64;

            let provider = self.provider.lock().await;
            provider.upload_bundle(&bundle).await?;
            drop(provider);
        }

        // 8. Update stats
        let duration = start_time.elapsed().as_millis() as u64;
        let now = chrono::Utc::now().timestamp_millis();

        let mut stats = self.stats.write().await;
        stats.last_sync_at = Some(now);
        stats.documents_synced += applied_count as u64;
        stats.documents_uploaded += uploaded_count;
        stats.documents_downloaded += applied_count as u64;
        stats.conflicts_resolved += resolved_conflicts;
        stats.conflicts_pending = pending_conflicts.len() as u32;
        stats.bytes_uploaded += uploaded_bytes;
        stats.bytes_downloaded += downloaded_bytes;
        stats.sync_duration_ms = duration;

        let result = stats.clone();
        drop(stats);

        let mut config = self.config.write().await;
        config.last_sync_at = Some(now);
        drop(config);

        if pending_conflicts.is_empty() {
            *self.status.write().await = SyncStatus::Idle;
        } else {
            *self.status.write().await = SyncStatus::Conflict(pending_conflicts);
        }

        self.event_tx
            .send(SyncEvent::Progress {
                current: 100,
                total: 100,
                message: "Sync completed".to_string(),
            })
            .ok();

        self.event_tx
            .send(SyncEvent::Completed {
                stats: result.clone(),
            })
            .ok();

        Ok(result)
    }

    /// Get local changes from database
    async fn get_local_changes(&self) -> Result<Vec<RawConfigData>, LiteError> {
        let db = self.db.lock().await;
        let mut changes = Vec::new();

        let hosts = db.get_hosts()?;
        for host in hosts {
            changes.push(RawConfigData {
                id: host.id.clone(),
                doc_type: SyncDocumentType::Host,
                data: serde_json::to_value(&host)?,
                updated_at: host.updated_at.parse::<i64>().unwrap_or(0),
                deleted: false,
            });
        }

        let groups = db.get_groups()?;
        for group in groups {
            changes.push(RawConfigData {
                id: group.id.clone(),
                doc_type: SyncDocumentType::Group,
                data: serde_json::to_value(&group)?,
                updated_at: group.updated_at.parse::<i64>().unwrap_or(0),
                deleted: false,
            });
        }

        let identities = db.get_identities()?;
        for identity in identities {
            changes.push(RawConfigData {
                id: identity.id.clone(),
                doc_type: SyncDocumentType::Identity,
                data: serde_json::to_value(&identity)?,
                updated_at: identity.updated_at.parse::<i64>().unwrap_or(0),
                deleted: false,
            });
        }

        let tags = db.get_tags()?;
        for tag in tags {
            changes.push(RawConfigData {
                id: tag.id.clone(),
                doc_type: SyncDocumentType::Tag,
                data: serde_json::to_value(&tag)?,
                updated_at: tag.updated_at.parse::<i64>().unwrap_or(0),
                deleted: false,
            });
        }

        let snippets = db.get_snippets()?;
        for snippet in snippets {
            changes.push(RawConfigData {
                id: snippet.id.clone(),
                doc_type: SyncDocumentType::Snippet,
                data: serde_json::to_value(&snippet)?,
                updated_at: snippet.updated_at.parse::<i64>().unwrap_or(0),
                deleted: false,
            });
        }

        Ok(changes)
    }

    /// Create sync bundle from raw documents
    async fn create_sync_bundle(
        &self,
        raw_docs: Vec<RawConfigData>,
    ) -> Result<SyncBundle, LiteError> {
        let config = self.config.read().await;
        let device_id = config.device_id.clone();
        let scope = config.scope.clone();
        let compression_enabled = config.compression_enabled;
        drop(config);

        let mut documents = Vec::new();
        let crypto = self.crypto.lock().await;

        for raw in raw_docs {
            if !self.should_include_document_sync(&raw, &scope).await? {
                continue;
            }

            let json_data = serde_json::to_vec(&raw.data)?;
            let content_hash = blake3_hash(&json_data);

            let hashes = self.content_hashes.read().await;
            if let Some(existing_hash) = hashes.get(&raw.id) {
                if existing_hash == &content_hash {
                    debug!("Skipping duplicate document: {}", raw.id);
                    continue;
                }
            }
            drop(hashes);

            self.content_hashes
                .write()
                .await
                .insert(raw.id.clone(), content_hash.clone());

            let encrypted_data = crypto.encrypt(&json_data)?;

            let mut vector_clock = self.vector_clock.write().await;
            let counter = vector_clock.entry(device_id.clone()).or_insert(0);
            *counter += 1;

            let mut doc = SyncDocument::new(
                raw.id.clone(),
                raw.doc_type.clone(),
                device_id.clone(),
                encrypted_data,
                content_hash,
            );
            doc.doc_type = raw.doc_type;
            doc.vector_clock = vector_clock.clone();
            doc.group_id = raw
                .data
                .get("group_id")
                .and_then(|v| v.as_str().map(|s| s.to_string()));

            if raw.deleted {
                doc.operation = SyncOperation::Delete;
            }

            documents.push(doc);
        }

        let mut bundle = SyncBundle::new(device_id, documents);
        bundle.compressed = compression_enabled;

        Ok(bundle)
    }

    /// Detect conflicts between local and remote documents
    async fn detect_conflicts(
        &self,
        local_docs: &[RawConfigData],
        remote_docs: &[SyncDocument],
    ) -> Result<Vec<SyncConflict>, LiteError> {
        let mut conflicts = Vec::new();
        let local_map: HashMap<String, &RawConfigData> =
            local_docs.iter().map(|d| (d.id.clone(), d)).collect();

        let crypto = self.crypto.lock().await;

        for remote_doc in remote_docs {
            if let Some(local_raw) = local_map.get(&remote_doc.id) {
                if remote_doc.deleted {
                    continue;
                }

                let remote_decrypted = match crypto.decrypt(&remote_doc.encrypted_data) {
                    Ok(data) => data,
                    Err(_) => {
                        warn!("Failed to decrypt remote document: {}", remote_doc.id);
                        continue;
                    }
                };

                let remote_content_hash = blake3_hash(&remote_decrypted);
                let local_json = serde_json::to_vec(&local_raw.data)?;
                let local_content_hash = blake3_hash(&local_json);

                if remote_content_hash == local_content_hash {
                    continue;
                }

                let local_as_doc = SyncDocument {
                    id: local_raw.id.clone(),
                    doc_type: local_raw.doc_type.clone(),
                    device_id: self.config.read().await.device_id.clone(),
                    operation: SyncOperation::Update,
                    timestamp: local_raw.updated_at,
                    vector_clock: self.vector_clock.read().await.clone(),
                    encrypted_data: local_json,
                    content_hash: local_content_hash,
                    parent_hashes: Vec::new(),
                    deleted: false,
                    group_id: None,
                    schema_version: 1,
                };

                if local_as_doc.has_conflict_with(remote_doc) {
                    let field_conflicts = ConflictResolver::detect_field_conflicts(
                        &local_json,
                        &remote_decrypted,
                        &local_raw.doc_type,
                    )?;

                    conflicts.push(SyncConflict {
                        document_id: remote_doc.id.clone(),
                        doc_type: remote_doc.doc_type.clone(),
                        local_version: local_as_doc,
                        remote_version: remote_doc.clone(),
                        resolution: None,
                        detected_at: chrono::Utc::now().timestamp_millis(),
                        field_conflicts,
                    });
                }
            }
        }

        Ok(conflicts)
    }

    /// Resolve conflict using strategy
    async fn resolve_conflict(
        &self,
        conflict: &SyncConflict,
        strategy: &SyncConflictResolution,
    ) -> Result<SyncConflictResolution, LiteError> {
        let resolution = ConflictResolver::resolve_with_strategy(conflict, strategy);

        match &resolution {
            SyncConflictResolution::UseRemote => {
                self.apply_document_to_local(&conflict.remote_version)
                    .await?;
            }
            SyncConflictResolution::Merge => {
                // Try field-level merge
                let crypto = self.crypto.lock().await;
                let local_data = crypto.decrypt(&conflict.local_version.encrypted_data)?;
                let remote_data = crypto.decrypt(&conflict.remote_version.encrypted_data)?;

                let local: serde_json::Value = serde_json::from_slice(&local_data)?;
                let remote: serde_json::Value = serde_json::from_slice(&remote_data)?;

                if let Some(merged) = ConflictResolver::try_merge_fields(
                    &local,
                    &remote,
                    &conflict.field_conflicts,
                    &conflict.doc_type,
                )? {
                    let merged_json = serde_json::to_vec(&merged)?;
                    let encrypted_data = crypto.encrypt(&merged_json)?;
                    let content_hash = blake3_hash(&merged_json);

                    let mut merged_doc = conflict.local_version.clone();
                    merged_doc.encrypted_data = encrypted_data;
                    merged_doc.content_hash = content_hash;
                    merged_doc.timestamp = chrono::Utc::now().timestamp_millis();
                    merged_doc.merge_clock(&conflict.remote_version.vector_clock);

                    self.apply_document_to_local(&merged_doc).await?;
                }
            }
            _ => {}
        }

        Ok(resolution)
    }

    /// Apply document to local database
    async fn apply_document_to_local(&self, doc: &SyncDocument) -> Result<(), LiteError> {
        let crypto = self.crypto.lock().await;
        let decrypted = crypto.decrypt(&doc.encrypted_data)?;
        drop(crypto);

        let data: serde_json::Value = serde_json::from_slice(&decrypted)?;
        let mut db = self.db.lock().await;

        if doc.deleted {
            match doc.doc_type {
                SyncDocumentType::Host => {
                    let _ = db.delete_host(&doc.id);
                }
                SyncDocumentType::Group => {
                    let _ = db.delete_group(&doc.id);
                }
                SyncDocumentType::Identity => {
                    let _ = db.delete_identity(&doc.id);
                }
                SyncDocumentType::Tag => {
                    let _ = db.delete_tag(&doc.id);
                }
                _ => {}
            }
            return Ok(());
        }

        // Handle create/update operations
        match doc.doc_type {
            SyncDocumentType::Host => {
                if let Some(obj) = data.as_object() {
                    let host = obj.get("host").and_then(|v| v.as_str()).unwrap_or("");
                    let port = obj.get("port").and_then(|v| v.as_i64()).unwrap_or(22);
                    let username = obj.get("username").and_then(|v| v.as_str()).unwrap_or("");
                    let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or(host);

                    // Check if host exists
                    match db.get_host(&doc.id) {
                        Ok(_) => {
                            // Update
                            let update = crate::db::UpdateHost {
                                id: doc.id.clone(),
                                name: name.to_string(),
                                host: host.to_string(),
                                port,
                                username: username.to_string(),
                                auth_type: obj
                                    .get("auth_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("password")
                                    .to_string(),
                                identity_file: obj
                                    .get("identity_file")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                identity_id: obj
                                    .get("identity_id")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                group_id: obj
                                    .get("group_id")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                notes: obj
                                    .get("notes")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                color: obj
                                    .get("color")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                environment: obj
                                    .get("environment")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                region: obj
                                    .get("region")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                purpose: obj
                                    .get("purpose")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                status: obj
                                    .get("status")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("active")
                                    .to_string(),
                            };
                            db.update_host(&update)?;
                        }
                        Err(_) => {
                            // Create new
                            let new_host = crate::db::NewHost {
                                id: doc.id.clone(),
                                name: name.to_string(),
                                host: host.to_string(),
                                port,
                                username: username.to_string(),
                                auth_type: obj
                                    .get("auth_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("password")
                                    .to_string(),
                                identity_file: obj
                                    .get("identity_file")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                identity_id: obj
                                    .get("identity_id")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                group_id: obj
                                    .get("group_id")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                notes: obj
                                    .get("notes")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                color: obj
                                    .get("color")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                environment: obj
                                    .get("environment")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                region: obj
                                    .get("region")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                purpose: obj
                                    .get("purpose")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                status: obj
                                    .get("status")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("active")
                                    .to_string(),
                            };
                            db.add_host(&new_host)?;
                        }
                    }
                }
            }
            SyncDocumentType::Group => {
                if let Some(obj) = data.as_object() {
                    let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let groups = db.get_groups()?;
                    let exists = groups.iter().any(|g| g.id == doc.id);

                    if exists {
                        let update = crate::db::UpdateGroup {
                            id: doc.id.clone(),
                            name: name.to_string(),
                        };
                        db.update_group(&update)?;
                    } else {
                        let new_group = crate::db::NewGroup {
                            id: doc.id.clone(),
                            name: name.to_string(),
                        };
                        db.add_group(&new_group)?;
                    }
                }
            }
            SyncDocumentType::Identity => {
                if let Some(obj) = data.as_object() {
                    let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let auth_type = obj
                        .get("auth_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("key");

                    match db.get_identity(&doc.id) {
                        Ok(_) => {
                            let update = crate::db::UpdateIdentity {
                                id: doc.id.clone(),
                                name: name.to_string(),
                                private_key_path: obj
                                    .get("private_key_path")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                passphrase_secret_id: obj
                                    .get("passphrase_secret_id")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                auth_type: auth_type.to_string(),
                            };
                            db.update_identity(&update)?;
                        }
                        Err(_) => {
                            let new_identity = crate::db::NewIdentity {
                                id: doc.id.clone(),
                                name: name.to_string(),
                                private_key_path: obj
                                    .get("private_key_path")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                passphrase_secret_id: obj
                                    .get("passphrase_secret_id")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                auth_type: auth_type.to_string(),
                            };
                            db.add_identity(&new_identity)?;
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Check if document should be applied based on sync scope
    async fn should_apply_document(&self, doc: &SyncDocument) -> Result<bool, LiteError> {
        if doc.deleted {
            return Ok(true);
        }

        let config = self.config.read().await;
        let scope = config.scope.clone();
        drop(config);

        if let Some(group_id) = &doc.group_id {
            if !scope.include_all {
                if !scope.included_groups.is_empty() && !scope.included_groups.contains(group_id) {
                    return Ok(false);
                }
            }
            if scope.excluded_groups.contains(group_id) {
                return Ok(false);
            }
        }

        match doc.doc_type {
            SyncDocumentType::Identity if !scope.include_identities => return Ok(false),
            SyncDocumentType::Snippet if !scope.include_snippets => return Ok(false),
            SyncDocumentType::Layout if !scope.include_layouts => return Ok(false),
            SyncDocumentType::Setting if !scope.include_settings => return Ok(false),
            SyncDocumentType::KnownHost if !scope.include_known_hosts => return Ok(false),
            SyncDocumentType::VaultItem if !scope.include_vault_items => return Ok(false),
            _ => {}
        }

        Ok(true)
    }

    /// Check if document should be included in sync bundle
    async fn should_include_document_sync(
        &self,
        raw: &RawConfigData,
        scope: &SyncScope,
    ) -> Result<bool, LiteError> {
        match raw.doc_type {
            SyncDocumentType::Identity if !scope.include_identities => return Ok(false),
            SyncDocumentType::Snippet if !scope.include_snippets => return Ok(false),
            SyncDocumentType::Layout if !scope.include_layouts => return Ok(false),
            SyncDocumentType::Setting if !scope.include_settings => return Ok(false),
            SyncDocumentType::KnownHost if !scope.include_known_hosts => return Ok(false),
            SyncDocumentType::VaultItem if !scope.include_vault_items => return Ok(false),
            _ => {}
        }

        if scope.include_all {
            return Ok(true);
        }

        if let Some(group_id) = raw.data.get("group_id").and_then(|v| v.as_str()) {
            let group_id_string = group_id.to_string();
            if !scope.included_groups.is_empty()
                && !scope.included_groups.contains(&group_id_string)
            {
                return Ok(false);
            }
            if scope.excluded_groups.contains(&group_id_string) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Deduplicate documents based on content hash
    async fn deduplicate_documents(&self, docs: Vec<SyncDocument>) -> Vec<SyncDocument> {
        let mut seen_hashes: HashMap<String, &SyncDocument> = HashMap::new();
        let mut result: Vec<SyncDocument> = Vec::new();
        let mut dedup_count = 0;

        for doc in &docs {
            if let Some(existing) = seen_hashes.get(&doc.content_hash) {
                if doc.timestamp > existing.timestamp {
                    if let Some(pos) = result
                        .iter()
                        .position(|d| d.content_hash == doc.content_hash)
                    {
                        result[pos] = doc.clone();
                        seen_hashes.insert(doc.content_hash.clone(), doc);
                    }
                }
                dedup_count += 1;
            } else {
                seen_hashes.insert(doc.content_hash.clone(), doc);
                result.push(doc.clone());
            }
        }

        if dedup_count > 0 {
            info!("Deduplicated {} documents", dedup_count);
            self.stats.write().await.deduplicated_count += dedup_count;
        }

        result
    }

    /// Get current status
    pub async fn get_status(&self) -> SyncStatus {
        self.status.read().await.clone()
    }

    /// Get sync stats
    pub async fn get_stats(&self) -> SyncStats {
        self.stats.read().await.clone()
    }

    /// Get discovered devices
    pub async fn get_discovered_devices(&self) -> Vec<DeviceInfo> {
        let local_sync = self.local_sync.lock().await;
        local_sync.discover_devices()
    }

    /// Get pending conflicts
    pub async fn get_pending_conflicts(&self) -> Vec<SyncConflict> {
        match self.status.read().await.clone() {
            SyncStatus::Conflict(conflicts) => conflicts,
            _ => Vec::new(),
        }
    }

    /// Subscribe to sync events
    pub fn subscribe(&self) -> mpsc::UnboundedReceiver<SyncEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        let _ = tx;
        rx
    }
}
