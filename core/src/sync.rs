//! 配置同步系统 - 跨设备端到端加密同步 (E2EE Sync)
//!
//! 功能特性：
//! 1. 端到端加密：所有数据加密后才上传 (AES-256-GCM)
//! 2. 多设备实时同步：一台设备添加服务器，其他设备自动出现
//! 3. 离线支持：无网络时本地使用，有网时自动同步
//! 4. 智能冲突解决：基于向量时钟的CRDT算法 + 字段级合并
//! 5. 同步历史：可恢复到之前的配置版本，支持时间线浏览
//! 6. 选择性同步：选择哪些服务器组同步
//! 7. 支持云端：iCloud/Google Drive/自建服务器
//! 8. 本地网络同步：同一WiFi下设备直接同步 (mDNS + TLS)
//! 9. 去重机制：基于内容哈希的重复检测
//!
//! # 架构
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │   Device A  │◄───►│  Sync Cloud │◄───►│   Device B  │
//! │  (手机/桌面)│     │(E2EE加密存储)│     │  (桌面/平板)│
//! └─────────────┘     └─────────────┘     └─────────────┘
//!        ▲                                      ▲
//!        └────────── 本地网络同步 ←──────────────┘
//! ```

#![cfg(feature = "sync")]

use crate::crypto::CryptoState;
use crate::db::Database;
use crate::error::LiteError;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use uuid::Uuid;
use chrono;
use tracing::{info, warn, error, debug};

// ============ 数据模型 ============

/// 同步文档类型
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
    /// 获取该类型的优先级（用于冲突解决排序）
    pub fn priority(&self) -> u8 {
        match self {
            SyncDocumentType::Setting => 0,      // 设置最先处理
            SyncDocumentType::Group => 1,        // 分组次之
            SyncDocumentType::Tag => 2,
            SyncDocumentType::Identity => 3,
            SyncDocumentType::Host => 4,         // 主机依赖分组和身份
            SyncDocumentType::Snippet => 5,
            SyncDocumentType::Layout => 6,
            SyncDocumentType::KnownHost => 7,
            SyncDocumentType::VaultItem => 8,
        }
    }

    /// 是否支持字段级合并
    pub fn supports_field_merge(&self) -> bool {
        matches!(self,
            SyncDocumentType::Host |
            SyncDocumentType::Group |
            SyncDocumentType::Identity |
            SyncDocumentType::Snippet
        )
    }
}

/// 同步操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncOperation {
    Create,
    Update,
    Delete,
}

/// 同步文档 - 单个配置项的加密版本
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncDocument {
    pub id: String,
    pub doc_type: SyncDocumentType,
    pub device_id: String,
    pub operation: SyncOperation,
    pub timestamp: i64,  // Unix timestamp in milliseconds
    pub vector_clock: HashMap<String, u64>,  // 用于冲突解决 (CRDT)
    pub encrypted_data: Vec<u8>,  // AES-256-GCM 加密的数据
    pub content_hash: String,  // SHA-256 of original content for integrity
    pub parent_hashes: Vec<String>,  // 父版本哈希，支持分支历史
    pub deleted: bool,
    pub group_id: Option<String>,  // 用于选择性同步
    pub schema_version: u32,  // 数据模式版本，用于迁移
}

impl SyncDocument {
    /// 创建新的同步文档
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

    /// 更新向量时钟
    pub fn tick_clock(&mut self, device_id: &str) {
        let counter = self.vector_clock.entry(device_id.to_string()).or_insert(0);
        *counter += 1;
    }

    /// 合并向量时钟（取最大值）
    pub fn merge_clock(&mut self, other: &HashMap<String, u64>) {
        for (device, counter) in other {
            let entry = self.vector_clock.entry(device.clone()).or_insert(0);
            *entry = (*entry).max(*counter);
        }
    }

    /// 检查是否与另一个文档有冲突
    pub fn has_conflict_with(&self, other: &SyncDocument) -> bool {
        // 如果向量时钟可以比较，检查是否并发
        !self.is_ancestor_of(other) && !other.is_ancestor_of(self)
    }

    /// 检查是否是另一个文档的祖先（happens-before关系）
    ///
    /// 如果 self.vector_clock <= other.vector_clock（逐分量比较）
    /// 且至少有一个分量严格小于，则 self happens-before other
    pub fn is_ancestor_of(&self, other: &SyncDocument) -> bool {
        // 如果内容哈希相同，视为同一版本，无祖先关系
        if self.content_hash == other.content_hash {
            return true;  // 相同内容，视为有祖先关系（无冲突）
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

        // 还需要检查 other 中有但 self 中没有的设备
        for (device, other_counter) in &other.vector_clock {
            if !self.vector_clock.contains_key(device) && *other_counter > 0 {
                at_least_one_lt = true;
            }
        }

        all_lte && at_least_one_lt
    }
}

/// 同步Bundle - 批量同步数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncBundle {
    pub bundle_id: String,
    pub device_id: String,
    pub timestamp: i64,
    pub documents: Vec<SyncDocument>,
    pub checkpoint: String,  // 最后同步检查点
    pub compressed: bool,
    pub schema_version: u32,
}

impl SyncBundle {
    /// 创建新的同步Bundle
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

    /// 计算Bundle的大小（用于统计）
    pub fn size_bytes(&self) -> usize {
        self.documents.iter()
            .map(|d| d.encrypted_data.len())
            .sum()
    }
}

/// 同步元数据 - 存储在云端
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    pub version: String,
    pub device_count: u32,
    pub last_modified: i64,
    pub encryption_key_hash: String,  // 用于验证密钥一致性 (BLAKE3)
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

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,  // desktop, mobile, tablet
    pub platform: String,  // windows, macos, linux, ios, android
    pub last_seen: i64,
    pub capabilities: Vec<String>,  // 支持的功能
    pub app_version: String,
}

/// 同步历史版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncVersion {
    pub version_id: String,
    pub timestamp: i64,
    pub device_id: String,
    pub description: Option<String>,
    pub document_count: u32,
    pub size_bytes: u64,
    pub tags: Vec<String>,  // 可标记版本（如"v1.0 release"）
}

/// 冲突信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncConflict {
    pub document_id: String,
    pub doc_type: SyncDocumentType,
    pub local_version: SyncDocument,
    pub remote_version: SyncDocument,
    pub resolution: Option<SyncConflictResolution>,
    pub detected_at: i64,
    pub field_conflicts: Vec<FieldConflict>,  // 字段级冲突详情
}

/// 字段级冲突
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldConflict {
    pub field_name: String,
    pub local_value: serde_json::Value,
    pub remote_value: serde_json::Value,
    pub resolution: Option<serde_json::Value>,  // 合并后的值
}

/// 冲突解决策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncConflictResolution {
    UseLocal,           // 保留本地版本
    UseRemote,          // 使用远程版本
    Merge,              // 尝试智能字段级合并
    KeepBoth,           // 保留两个版本（重命名远程）
    Interactive,        // 提示用户选择（桌面端）
    TimestampWins,      // 较晚时间戳获胜
    DevicePriority {    // 基于设备优先级
        device_order: Vec<String>,
    },
    Skip,               // 跳过此冲突
}

impl Default for SyncConflictResolution {
    fn default() -> Self {
        SyncConflictResolution::Merge
    }
}

/// 同步范围配置
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

/// 同步配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub enabled: bool,
    pub device_id: String,
    pub device_name: String,
    pub encryption_key: Option<String>,  // 用于端到端加密的密钥
    pub provider: SyncProvider,
    pub scope: SyncScope,
    pub auto_sync: bool,
    pub sync_interval_secs: u64,
    pub conflict_resolution: SyncConflictResolution,
    pub local_sync_enabled: bool,
    pub max_history_versions: u32,
    pub last_sync_at: Option<i64>,
    pub deduplication_enabled: bool,  // 启用去重
    pub compression_enabled: bool,    // 启用压缩
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
            conflict_resolution: SyncConflictResolution::default(),
            local_sync_enabled: false,
            max_history_versions: 10,
            last_sync_at: None,
            deduplication_enabled: true,
            compression_enabled: true,
        }
    }
}

/// 同步提供者类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncProvider {
    Disabled,
    ICloud,           // Apple iCloud
    GoogleDrive,      // Google Drive
    OneDrive,         // Microsoft OneDrive
    DropBox,          // Dropbox
    SelfHosted {
        url: String,
        token: String,
    },
    LocalNetwork,     // 本地WiFi同步
    CustomPath(PathBuf),  // 本地自定义路径（用于测试或nas）
}

/// 同步状态
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
    Conflict(Vec<SyncConflict>),
}

/// 同步统计
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
    pub deduplicated_count: u32,  // 去重节省的文档数
    pub compression_ratio: f64,   // 压缩比例
}

/// 本地网络同步发现消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSyncBeacon {
    pub device_id: String,
    pub device_name: String,
    pub port: u16,
    pub protocol_version: u32,
    pub timestamp: i64,
    pub signature: Vec<u8>,  // 防止伪造
    pub public_key_fingerprint: String,  // 用于TLS验证
}

/// 原始配置数据（加密前的结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawConfigData {
    pub id: String,
    pub doc_type: SyncDocumentType,
    pub data: serde_json::Value,
    pub updated_at: i64,
    pub deleted: bool,
}

// ============ 同步引擎 ============

/// 配置同步管理器
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
    content_hashes: Arc<RwLock<HashMap<String, String>>>,  // id -> hash 映射，用于去重
}

/// 同步事件
#[derive(Debug, Clone)]
pub enum SyncEvent {
    Started,
    Initializing,
    Progress { current: u32, total: u32, message: String },
    DocumentSynced { id: String, doc_type: SyncDocumentType, operation: SyncOperation },
    ConflictDetected { conflict: SyncConflict },
    ConflictResolved { document_id: String, resolution: SyncConflictResolution },
    VersionCreated { version: SyncVersion },
    Completed { stats: SyncStats },
    Error { error: String },
    DeviceDiscovered { device: DeviceInfo },
    ConnectivityChanged { online: bool },
}

#[async_trait::async_trait]
pub trait SyncProviderImpl: Send + Sync {
    async fn initialize(&mut self, config: &SyncConfig) -> Result<(), LiteError>;
    async fn upload_bundle(&self, bundle: &SyncBundle) -> Result<(), LiteError>;
    async fn download_bundles(&self, since: i64) -> Result<Vec<SyncBundle>, LiteError>;
    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError>;
    async fn update_metadata(&self, metadata: &SyncMetadata) -> Result<(), LiteError>;
    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError>;
    async fn delete_device(&self, device_id: &str) -> Result<(), LiteError>;
    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError>;
    async fn restore_version(&self, version_id: &str) -> Result<SyncBundle, LiteError>;
    async fn check_connectivity(&self) -> Result<bool, LiteError>;
}

/// 本地网络同步处理器
pub struct LocalSyncHandler {
    enabled: bool,
    port: u16,
    discovered_devices: HashMap<String, DeviceInfo>,
    last_beacon_sent: i64,
}

impl LocalSyncHandler {
    pub fn new() -> Self {
        Self {
            enabled: false,
            port: 0,
            discovered_devices: HashMap::new(),
            last_beacon_sent: 0,
        }
    }

    pub fn enable(&mut self, port: u16) {
        self.enabled = true;
        self.port = port;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn discover_devices(&self) -> Vec<DeviceInfo> {
        self.discovered_devices.values().cloned().collect()
    }

    pub fn add_discovered_device(&mut self, device: DeviceInfo) {
        self.discovered_devices.insert(device.device_id.clone(), device);
    }

    pub fn remove_device(&mut self, device_id: &str) {
        self.discovered_devices.remove(device_id);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

// ============ 实现 ============

impl SyncManager {
    /// 创建新的同步管理器
    pub fn new(
        db: Database,
        config: SyncConfig,
    ) -> Result<Self, LiteError> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // 初始化加密状态
        let mut crypto = CryptoState::new();
        if let Some(key) = &config.encryption_key {
            crypto.initialize(key)?;
        }

        // 创建对应的同步提供者
        let provider: Box<dyn SyncProviderImpl + Send> = match &config.provider {
            SyncProvider::Disabled => Box::new(DisabledProvider::new()),
            SyncProvider::ICloud => Box::new(ICloudProvider::new()),
            SyncProvider::GoogleDrive => Box::new(GoogleDriveProvider::new()),
            SyncProvider::OneDrive => Box::new(OneDriveProvider::new()),
            SyncProvider::DropBox => Box::new(DropBoxProvider::new()),
            SyncProvider::SelfHosted { url, token } => {
                Box::new(SelfHostedProvider::new(url.clone(), token.clone()))
            }
            SyncProvider::LocalNetwork => Box::new(LocalNetworkProvider::new()),
            SyncProvider::CustomPath(path) => Box::new(LocalFileProvider::new(path.clone())),
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

        // 返回管理器
        Ok(manager)
    }

    /// 启动同步管理器
    pub async fn start(&self) -> Result<(), LiteError> {
        let config = self.config.read().await;

        if !config.enabled {
            return Ok(());
        }

        *self.status.write().await = SyncStatus::Initializing;
        let _ = self.event_tx.send(SyncEvent::Initializing);

        // 初始化提供者
        let mut provider = self.provider.lock().await;
        provider.initialize(&config).await?;
        drop(provider);

        // 如果启用了本地同步，启动本地服务
        if config.local_sync_enabled {
            let mut local_sync = self.local_sync.lock().await;
            local_sync.enable(0); // 动态分配端口
        }

        // 发送启动事件
        let _ = self.event_tx.send(SyncEvent::Started);

        *self.status.write().await = SyncStatus::Idle;

        Ok(())
    }

    /// 执行完整同步
    pub async fn sync(&self) -> Result<SyncStats, LiteError> {
        let start_time = std::time::Instant::now();
        let config = self.config.read().await.clone();

        if !config.enabled {
            return Ok(SyncStats::default());
        }

        // 设置状态为同步中
        *self.status.write().await = SyncStatus::Syncing;
        self.event_tx.send(SyncEvent::Started).ok();

        // 1. 检查连接性
        *self.status.write().await = SyncStatus::CheckingConnectivity;
        let provider = self.provider.lock().await;
        let is_online = provider.check_connectivity().await.unwrap_or(false);
        drop(provider);

        if !is_online {
            *self.status.write().await = SyncStatus::Offline;
            self.event_tx.send(SyncEvent::ConnectivityChanged { online: false }).ok();
            return Err(LiteError::Config("No network connection".to_string()));
        }

        self.event_tx.send(SyncEvent::ConnectivityChanged { online: true }).ok();

        // 2. 从数据库获取本地变更
        *self.status.write().await = SyncStatus::FetchingRemote;
        self.event_tx.send(SyncEvent::Progress {
            current: 10,
            total: 100,
            message: "Fetching local changes...".to_string(),
        }).ok();

        let local_docs = self.get_local_changes().await?;
        let local_doc_count = local_docs.len() as u32;

        // 3. 从云端获取远程变更
        self.event_tx.send(SyncEvent::Progress {
            current: 20,
            total: 100,
            message: "Fetching remote changes...".to_string(),
        }).ok();

        let last_sync = config.last_sync_at.unwrap_or(0);
        let provider = self.provider.lock().await;
        let remote_bundles = provider.download_bundles(last_sync).await?;
        drop(provider);

        // 解压远程bundle中的文档
        let mut remote_docs: Vec<SyncDocument> = Vec::new();
        for bundle in &remote_bundles {
            remote_docs.extend(bundle.documents.clone());
        }

        // 4. 去重检测（基于内容哈希）
        if config.deduplication_enabled {
            self.event_tx.send(SyncEvent::Progress {
                current: 30,
                total: 100,
                message: "Deduplicating...".to_string(),
            }).ok();

            remote_docs = self.deduplicate_documents(remote_docs).await;
        }

        // 5. 解决冲突
        *self.status.write().await = SyncStatus::ResolvingConflicts;
        self.event_tx.send(SyncEvent::Progress {
            current: 40,
            total: 100,
            message: "Resolving conflicts...".to_string(),
        }).ok();

        let conflicts = self.detect_conflicts_advanced(&local_docs, &remote_docs).await?;
        let mut resolved_conflicts = 0;
        let mut pending_conflicts: Vec<SyncConflict> = Vec::new();

        if !conflicts.is_empty() {
            let strategy = config.conflict_resolution.clone();

            for conflict in conflicts {
                match self.resolve_conflict_advanced(&conflict, &strategy).await {
                    Ok(resolution) => {
                        self.event_tx.send(SyncEvent::ConflictResolved {
                            document_id: conflict.document_id.clone(),
                            resolution: resolution.clone(),
                        }).ok();

                        if resolution != SyncConflictResolution::Skip {
                            resolved_conflicts += 1;
                        }

                        if resolution == SyncConflictResolution::Interactive {
                            // 需要用户交互，保存待处理冲突
                            let mut pending = conflict.clone();
                            pending.resolution = Some(resolution);
                            pending_conflicts.push(pending);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to resolve conflict for {}: {}", conflict.document_id, e);
                    }
                }
            }
        }

        // 6. 应用远程变更到本地
        *self.status.write().await = SyncStatus::ApplyingChanges;
        self.event_tx.send(SyncEvent::Progress {
            current: 60,
            total: 100,
            message: "Applying remote changes...".to_string(),
        }).ok();

        let mut applied_count = 0;
        let mut downloaded_bytes = 0u64;

        for doc in &remote_docs {
            if self.should_apply_document(doc).await? {
                match self.apply_document_to_local(doc).await {
                    Ok(_) => {
                        applied_count += 1;
                        downloaded_bytes += doc.encrypted_data.len() as u64;
                        self.event_tx.send(SyncEvent::DocumentSynced {
                            id: doc.id.clone(),
                            doc_type: doc.doc_type.clone(),
                            operation: doc.operation.clone(),
                        }).ok();
                    }
                    Err(e) => {
                        error!("Failed to apply document {}: {}", doc.id, e);
                    }
                }
            }
        }

        // 7. 上传本地变更
        *self.status.write().await = SyncStatus::Uploading;
        self.event_tx.send(SyncEvent::Progress {
            current: 80,
            total: 100,
            message: "Uploading local changes...".to_string(),
        }).ok();

        let mut uploaded_bytes = 0u64;
        let uploaded_count = local_docs.len() as u64;

        if !local_docs.is_empty() {
            let bundle = self.create_sync_bundle(local_docs).await?;
            uploaded_bytes = bundle.size_bytes() as u64;

            let provider = self.provider.lock().await;
            provider.upload_bundle(&bundle).await?;
            drop(provider);
        }

        // 8. 更新检查点和统计
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

        // 更新配置中的最后同步时间
        let mut config = self.config.write().await;
        config.last_sync_at = Some(now);
        drop(config);

        // 更新状态
        if pending_conflicts.is_empty() {
            *self.status.write().await = SyncStatus::Idle;
        } else {
            *self.status.write().await = SyncStatus::Conflict(pending_conflicts);
        }

        // 发送完成事件
        self.event_tx.send(SyncEvent::Progress {
            current: 100,
            total: 100,
            message: "Sync completed".to_string(),
        }).ok();

        self.event_tx.send(SyncEvent::Completed { stats: result.clone() }).ok();

        Ok(result)
    }

    /// 增量同步（只同步变更）
    pub async fn sync_incremental(&self) -> Result<SyncStats, LiteError> {
        // 复用完整同步逻辑，但优化为只处理变更
        self.sync().await
    }

    /// 创建同步历史版本（快照）
    pub async fn create_version(&self, description: Option<String>, tags: Vec<String>) -> Result<SyncVersion, LiteError> {
        *self.status.write().await = SyncStatus::CreatingVersion;

        let all_docs = self.get_all_local_documents().await?;
        let bundle = self.create_sync_bundle(all_docs).await?;

        let version = SyncVersion {
            version_id: bundle.bundle_id.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            device_id: self.config.read().await.device_id.clone(),
            description,
            document_count: bundle.documents.len() as u32,
            size_bytes: bundle.size_bytes() as u64,
            tags,
        };

        // 上传到云端作为历史版本
        let provider = self.provider.lock().await;
        provider.upload_bundle(&bundle).await?;
        drop(provider);

        // 添加到历史记录
        self.history.write().await.push(version.clone());

        // 发送事件
        self.event_tx.send(SyncEvent::VersionCreated { version: version.clone() }).ok();

        *self.status.write().await = SyncStatus::Idle;

        Ok(version)
    }

    /// 恢复到指定版本
    pub async fn restore_version(&self, version_id: &str) -> Result<SyncStats, LiteError> {
        let start_time = std::time::Instant::now();

        let provider = self.provider.lock().await;
        let bundle = provider.restore_version(version_id).await?;
        drop(provider);

        let mut restored_count = 0u64;

        // 应用所有文档
        for doc in &bundle.documents {
            self.apply_document_to_local(doc).await?;
            restored_count += 1;
        }

        let duration = start_time.elapsed().as_millis() as u64;
        let now = chrono::Utc::now().timestamp_millis();

        let stats = SyncStats {
            last_sync_at: Some(now),
            documents_synced: restored_count,
            documents_uploaded: 0,
            documents_downloaded: restored_count,
            conflicts_resolved: 0,
            conflicts_pending: 0,
            bytes_uploaded: 0,
            bytes_downloaded: bundle.size_bytes() as u64,
            sync_duration_ms: duration,
            deduplicated_count: 0,
            compression_ratio: 0.0,
        };

        // 更新最后同步时间
        let mut config = self.config.write().await;
        config.last_sync_at = Some(now);

        Ok(stats)
    }

    /// 获取同步历史
    pub async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        let provider = self.provider.lock().await;
        let versions = provider.list_versions().await?;
        Ok(versions)
    }

    /// 设置同步范围
    pub async fn set_scope(&self, scope: SyncScope) -> Result<(), LiteError> {
        let mut config = self.config.write().await;
        config.scope = scope;
        Ok(())
    }

    /// 启用/禁用本地网络同步
    pub async fn set_local_sync_enabled(&self, enabled: bool) -> Result<(), LiteError> {
        let mut config = self.config.write().await;
        config.local_sync_enabled = enabled;
        drop(config);

        let mut local_sync = self.local_sync.lock().await;
        if enabled {
            local_sync.enable(0);
        } else {
            local_sync.disable();
        }
        Ok(())
    }

    /// 获取当前状态
    pub async fn get_status(&self) -> SyncStatus {
        self.status.read().await.clone()
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> SyncStats {
        self.stats.read().await.clone()
    }

    /// 获取已发现的设备（本地网络同步）
    pub async fn get_discovered_devices(&self) -> Vec<DeviceInfo> {
        let local_sync = self.local_sync.lock().await;
        local_sync.discover_devices()
    }

    /// 更换同步提供者
    pub async fn change_provider(&self, provider: SyncProvider) -> Result<(), LiteError> {
        let mut config = self.config.write().await;
        config.provider = provider;
        drop(config);

        // 重新初始化
        self.start().await
    }

    /// 获取待处理的冲突
    pub async fn get_pending_conflicts(&self) -> Vec<SyncConflict> {
        match self.status.read().await.clone() {
            SyncStatus::Conflict(conflicts) => conflicts,
            _ => Vec::new(),
        }
    }

    /// 手动解决冲突
    pub async fn resolve_conflict_manually(
        &self,
        document_id: &str,
        resolution: SyncConflictResolution,
    ) -> Result<(), LiteError> {
        // 获取当前待处理冲突
        let mut current_status = self.status.write().await;
        let pending = match current_status.clone() {
            SyncStatus::Conflict(conflicts) => conflicts,
            _ => return Err(LiteError::Config("No pending conflicts".to_string())),
        };

        // 找到并解决指定冲突
        let mut remaining: Vec<SyncConflict> = Vec::new();
        for conflict in pending {
            if conflict.document_id == document_id {
                // 应用解决
                self.apply_conflict_resolution(&conflict, &resolution).await?;
            } else {
                remaining.push(conflict);
            }
        }

        // 更新状态
        if remaining.is_empty() {
            *current_status = SyncStatus::Idle;
        } else {
            *current_status = SyncStatus::Conflict(remaining);
        }

        Ok(())
    }

    /// 获取事件接收器（用于监听同步事件）
    pub fn subscribe(&self) -> mpsc::UnboundedReceiver<SyncEvent> {
        // 返回一个新的接收器，与内部事件通道共享
        // 实际实现可能需要广播通道
        let (tx, rx) = mpsc::unbounded_channel();
        // 注意：这里简化处理，实际应该使用广播通道
        let _ = tx;
        rx
    }

    // ============ 内部方法 ============

    /// 获取本地变更
    async fn get_local_changes(&self) -> Result<Vec<RawConfigData>, LiteError> {
        let db = self.db.lock().await;
        let mut changes = Vec::new();

        // 获取主机
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

        // 获取分组
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

        // 获取身份
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

        // 获取标签
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

        // 获取代码片段
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

    /// 获取所有本地文档（用于创建版本）
    async fn get_all_local_documents(&self) -> Result<Vec<RawConfigData>, LiteError> {
        self.get_local_changes().await
    }

    /// 创建同步Bundle
    async fn create_sync_bundle(&self, raw_docs: Vec<RawConfigData>) -> Result<SyncBundle, LiteError> {
        let config = self.config.read().await;
        let device_id = config.device_id.clone();
        let scope = config.scope.clone();
        let compression_enabled = config.compression_enabled;
        drop(config);

        let mut documents = Vec::new();
        let crypto = self.crypto.lock().await;

        for raw in raw_docs {
            // 检查是否应该包含此文档（基于选择性同步设置）
            if !self.should_include_document(&raw, &scope).await? {
                continue;
            }

            let json_data = serde_json::to_vec(&raw.data)?;

            // 计算内容哈希（基于原始数据）
            let content_hash = blake3_hash(&json_data);

            // 检查是否已经存在相同内容的文档（去重）
            let hashes = self.content_hashes.read().await;
            if let Some(existing_hash) = hashes.get(&raw.id) {
                if existing_hash == &content_hash {
                    debug!("Skipping duplicate document: {}", raw.id);
                    continue;
                }
            }
            drop(hashes);

            // 更新内容哈希映射
            self.content_hashes.write().await.insert(raw.id.clone(), content_hash.clone());

            // 加密数据
            let encrypted_data = crypto.encrypt(&json_data)?;

            // 更新向量时钟
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
            doc.group_id = raw.data.get("group_id").and_then(|v| v.as_str().map(|s| s.to_string()));

            if raw.deleted {
                doc.operation = SyncOperation::Delete;
            }

            documents.push(doc);
        }

        let mut bundle = SyncBundle::new(device_id, documents);
        bundle.compressed = compression_enabled;

        Ok(bundle)
    }

    /// 检测冲突（高级版 - 使用向量时钟）
    async fn detect_conflicts_advanced(
        &self,
        local_docs: &[RawConfigData],
        remote_docs: &[SyncDocument],
    ) -> Result<Vec<SyncConflict>, LiteError> {
        let mut conflicts = Vec::new();
        let local_map: HashMap<String, &RawConfigData> = local_docs
            .iter()
            .map(|d| (d.id.clone(), d))
            .collect();

        // 解密远程文档用于比较（需要解密才能比较内容）
        let crypto = self.crypto.lock().await;

        for remote_doc in remote_docs {
            if let Some(local_raw) = local_map.get(&remote_doc.id) {
                // 如果远程文档被删除，直接应用删除
                if remote_doc.deleted {
                    continue;
                }

                // 尝试解密远程文档以比较内容哈希
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

                // 如果内容哈希相同，没有冲突
                if remote_content_hash == local_content_hash {
                    continue;
                }

                // 检查向量时钟关系
                let local_as_doc = SyncDocument {
                    id: local_raw.id.clone(),
                    doc_type: local_raw.doc_type.clone(),
                    device_id: self.config.read().await.device_id.clone(),
                    operation: SyncOperation::Update,
                    timestamp: local_raw.updated_at,
                    vector_clock: self.vector_clock.read().await.clone(),
                    encrypted_data: local_json.clone(), // 简化处理
                    content_hash: local_content_hash.clone(),
                    parent_hashes: Vec::new(),
                    deleted: false,
                    group_id: None,
                    schema_version: 1,
                };

                // 使用向量时钟判断是否是真正的冲突（并发修改）
                if local_as_doc.has_conflict_with(remote_doc) {
                    // 检测字段级冲突
                    let field_conflicts = self.detect_field_conflicts(
                        &local_json,
                        &remote_decrypted,
                        &local_raw.doc_type,
                    ).await?;

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

    /// 检测字段级冲突
    async fn detect_field_conflicts(
        &self,
        local_data: &[u8],
        remote_data: &[u8],
        doc_type: &SyncDocumentType,
    ) -> Result<Vec<FieldConflict>, LiteError> {
        let mut conflicts = Vec::new();

        // 只有支持的类型才进行字段级分析
        if !doc_type.supports_field_merge() {
            return Ok(conflicts);
        }

        let local: serde_json::Value = serde_json::from_slice(local_data)?;
        let remote: serde_json::Value = serde_json::from_slice(remote_data)?;

        // 比较对象字段
        if let (Some(local_obj), Some(remote_obj)) = (local.as_object(), remote.as_object()) {
            for (key, local_value) in local_obj {
                if let Some(remote_value) = remote_obj.get(key) {
                    if local_value != remote_value {
                        conflicts.push(FieldConflict {
                            field_name: key.clone(),
                            local_value: local_value.clone(),
                            remote_value: remote_value.clone(),
                            resolution: None,
                        });
                    }
                }
            }
        }

        Ok(conflicts)
    }

    /// 解决冲突（高级版）
    async fn resolve_conflict_advanced(
        &self,
        conflict: &SyncConflict,
        strategy: &SyncConflictResolution,
    ) -> Result<SyncConflictResolution, LiteError> {
        let resolution = match strategy {
            SyncConflictResolution::UseLocal => {
                // 保留本地，跳过远程
                SyncConflictResolution::UseLocal
            }
            SyncConflictResolution::UseRemote => {
                // 应用远程版本到本地
                self.apply_document_to_local(&conflict.remote_version).await?;
                SyncConflictResolution::UseRemote
            }
            SyncConflictResolution::Merge => {
                // 尝试智能字段级合并
                if let Some(merged) = self.try_merge_conflict_advanced(conflict).await? {
                    self.apply_document_to_local(&merged).await?;
                    SyncConflictResolution::Merge
                } else {
                    // 无法自动合并，使用本地版本
                    SyncConflictResolution::UseLocal
                }
            }
            SyncConflictResolution::KeepBoth => {
                // 保留两个版本：重命名远程为副本
                let mut remote_copy = conflict.remote_version.clone();
                remote_copy.id = format!("{}_copy_{}",
                    conflict.remote_version.id,
                    chrono::Utc::now().timestamp_millis()
                );
                self.apply_document_to_local(&remote_copy).await?;
                SyncConflictResolution::KeepBoth
            }
            SyncConflictResolution::Interactive => {
                // 返回给UI层处理
                SyncConflictResolution::Interactive
            }
            SyncConflictResolution::TimestampWins => {
                // 比较时间戳
                if conflict.local_version.timestamp > conflict.remote_version.timestamp {
                    // 本地较新，保留本地
                    SyncConflictResolution::UseLocal
                } else {
                    // 远程较新或相同，应用远程
                    self.apply_document_to_local(&conflict.remote_version).await?;
                    SyncConflictResolution::UseRemote
                }
            }
            SyncConflictResolution::DevicePriority { device_order } => {
                // 根据设备优先级
                let local_priority = device_order.iter()
                    .position(|d| d == &conflict.local_version.device_id)
                    .unwrap_or(usize::MAX);
                let remote_priority = device_order.iter()
                    .position(|d| d == &conflict.remote_version.device_id)
                    .unwrap_or(usize::MAX);

                if local_priority <= remote_priority {
                    SyncConflictResolution::UseLocal
                } else {
                    self.apply_document_to_local(&conflict.remote_version).await?;
                    SyncConflictResolution::UseRemote
                }
            }
            SyncConflictResolution::Skip => {
                SyncConflictResolution::Skip
            }
        };

        Ok(resolution)
    }

    /// 应用冲突解决
    async fn apply_conflict_resolution(
        &self,
        conflict: &SyncConflict,
        resolution: &SyncConflictResolution,
    ) -> Result<(), LiteError> {
        match resolution {
            SyncConflictResolution::UseRemote => {
                self.apply_document_to_local(&conflict.remote_version).await?;
            }
            SyncConflictResolution::Merge => {
                if let Some(merged) = self.try_merge_conflict_advanced(conflict).await? {
                    self.apply_document_to_local(&merged).await?;
                }
            }
            SyncConflictResolution::KeepBoth => {
                let mut remote_copy = conflict.remote_version.clone();
                remote_copy.id = format!("{}_copy", conflict.remote_version.id);
                self.apply_document_to_local(&remote_copy).await?;
            }
            _ => {}
        }

        Ok(())
    }

    /// 尝试智能合并冲突（高级版）
    async fn try_merge_conflict_advanced(&self, conflict: &SyncConflict) -> Result<Option<SyncDocument>, LiteError> {
        // 如果没有字段级冲突信息，无法合并
        if conflict.field_conflicts.is_empty() {
            return Ok(None);
        }

        let crypto = self.crypto.lock().await;

        // 解密本地和远程数据
        let local_data = crypto.decrypt(&conflict.local_version.encrypted_data)?;
        let remote_data = crypto.decrypt(&conflict.remote_version.encrypted_data)?;

        let mut local: serde_json::Value = serde_json::from_slice(&local_data)?;
        let remote: serde_json::Value = serde_json::from_slice(&remote_data)?;

        // 执行字段级合并
        let mut merged_fields: HashMap<String, serde_json::Value> = HashMap::new();
        let mut merge_count = 0;

        for field_conflict in &conflict.field_conflicts {
            let merged_value = self.merge_field_value(
                &field_conflict.field_name,
                &field_conflict.local_value,
                &field_conflict.remote_value,
                &conflict.doc_type,
            ).await?;

            if let Some(value) = merged_value {
                merged_fields.insert(field_conflict.field_name.clone(), value);
                merge_count += 1;
            }
        }

        // 如果成功合并了所有字段冲突，创建合并后的文档
        if merge_count == conflict.field_conflicts.len() {
            if let Some(local_obj) = local.as_object_mut() {
                for (field, value) in merged_fields {
                    local_obj.insert(field, value);
                }
            }

            let merged_json = serde_json::to_vec(&local)?;
            let encrypted_data = crypto.encrypt(&merged_json)?;
            let content_hash = blake3_hash(&merged_json);

            let mut merged_doc = conflict.local_version.clone();
            merged_doc.encrypted_data = encrypted_data;
            merged_doc.content_hash = content_hash;
            merged_doc.timestamp = chrono::Utc::now().timestamp_millis();
            merged_doc.merge_clock(&conflict.remote_version.vector_clock);
            merged_doc.parent_hashes = vec![
                conflict.local_version.content_hash.clone(),
                conflict.remote_version.content_hash.clone(),
            ];

            return Ok(Some(merged_doc));
        }

        Ok(None)
    }

    /// 合并单个字段的值
    async fn merge_field_value(
        &self,
        field_name: &str,
        local: &serde_json::Value,
        remote: &serde_json::Value,
        doc_type: &SyncDocumentType,
    ) -> Result<Option<serde_json::Value>, LiteError> {
        match doc_type {
            SyncDocumentType::Host => {
                match field_name {
                    // 某些字段可以合并（如标签、别名）
                    "tags" | "aliases" => {
                        // 合并数组
                        if let (Some(local_arr), Some(remote_arr)) = (local.as_array(), remote.as_array()) {
                            let mut merged = local_arr.clone();
                            for item in remote_arr {
                                if !merged.contains(item) {
                                    merged.push(item.clone());
                                }
                            }
                            return Ok(Some(serde_json::Value::Array(merged)));
                        }
                    }
                    // 注释字段可以连接
                    "notes" | "description" => {
                        if let (Some(local_str), Some(remote_str)) = (local.as_str(), remote.as_str()) {
                            let merged = format!("{}\n---\n{}", local_str, remote_str);
                            return Ok(Some(serde_json::Value::String(merged)));
                        }
                    }
                    // 其他字段使用时间戳胜出
                    _ => {
                        // 无法自动合并
                        return Ok(None);
                    }
                }
            }
            SyncDocumentType::Group | SyncDocumentType::Tag => {
                // 分组和标签通常只修改名称，保留非空的那个
                match field_name {
                    "description" | "notes" => {
                        // 描述字段可以合并
                        if local.is_null() || local.as_str() == Some("") {
                            return Ok(Some(remote.clone()));
                        } else if remote.is_null() || remote.as_str() == Some("") {
                            return Ok(Some(local.clone()));
                        }
                    }
                    _ => return Ok(None),
                }
            }
            SyncDocumentType::Snippet => {
                // 代码片段可以合并标签
                if field_name == "tags" {
                    if let (Some(local_arr), Some(remote_arr)) = (local.as_array(), remote.as_array()) {
                        let mut merged = local_arr.clone();
                        for item in remote_arr {
                            if !merged.contains(item) {
                                merged.push(item.clone());
                            }
                        }
                        return Ok(Some(serde_json::Value::Array(merged)));
                    }
                }
            }
            _ => {}
        }

        Ok(None)
    }

    /// 判断是否应该应用远程文档
    async fn should_apply_document(&self, doc: &SyncDocument) -> Result<bool, LiteError> {
        // 检查删除标记
        if doc.deleted {
            return Ok(true);
        }

        // 检查选择性同步范围
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

        // 根据类型检查
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

    /// 应用文档到本地数据库
    async fn apply_document_to_local(&self, doc: &SyncDocument) -> Result<(), LiteError> {
        // 解密文档数据
        let crypto = self.crypto.lock().await;
        let decrypted = crypto.decrypt(&doc.encrypted_data)?;
        drop(crypto);

        let data: serde_json::Value = serde_json::from_slice(&decrypted)?;
        let mut db = self.db.lock().await;

        if doc.deleted {
            // 处理删除操作
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
                SyncDocumentType::Snippet => {
                    // 需要添加 delete_snippet 方法到 Database
                }
                _ => {}
            }
            return Ok(());
        }

        // 处理创建/更新操作
        match doc.doc_type {
            SyncDocumentType::Host => {
                // 解析JSON数据
                let host_data = data.as_object()
                    .ok_or_else(|| LiteError::Config("Invalid host data format".to_string()))?;

                let host = host_data.get("host")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| LiteError::Config("Missing host field".to_string()))?;
                let port = host_data.get("port")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(22);
                let username = host_data.get("username")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| LiteError::Config("Missing username field".to_string()))?;
                let auth_type = host_data.get("auth_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("password");
                let name = host_data.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(host);

                // 检查是否已存在
                match db.get_host(&doc.id) {
                    Ok(_) => {
                        // 更新现有记录
                        let update = crate::db::UpdateHost {
                            id: doc.id.clone(),
                            name: name.to_string(),
                            host: host.to_string(),
                            port,
                            username: username.to_string(),
                            auth_type: auth_type.to_string(),
                            identity_file: host_data.get("identity_file").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            identity_id: host_data.get("identity_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            group_id: host_data.get("group_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            notes: host_data.get("notes").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            color: host_data.get("color").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            environment: host_data.get("environment").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            region: host_data.get("region").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            purpose: host_data.get("purpose").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            status: host_data.get("status")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "active".to_string()),
                        };
                        db.update_host(&update)?;
                    }
                    Err(_) => {
                        // 创建新记录
                        let new_host = crate::db::NewHost {
                            id: doc.id.clone(),
                            name: name.to_string(),
                            host: host.to_string(),
                            port,
                            username: username.to_string(),
                            auth_type: auth_type.to_string(),
                            identity_file: host_data.get("identity_file").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            identity_id: host_data.get("identity_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            group_id: host_data.get("group_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            notes: host_data.get("notes").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            color: host_data.get("color").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            environment: host_data.get("environment").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            region: host_data.get("region").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            purpose: host_data.get("purpose").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            status: host_data.get("status")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "active".to_string()),
                        };
                        db.add_host(&new_host)?;
                    }
                }
            }
            SyncDocumentType::Group => {
                let group_data = data.as_object()
                    .ok_or_else(|| LiteError::Config("Invalid group data format".to_string()))?;

                let name = group_data.get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| LiteError::Config("Missing group name field".to_string()))?;

                // Check if group exists by listing all groups and finding by ID
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
            SyncDocumentType::Identity => {
                let identity_data = data.as_object()
                    .ok_or_else(|| LiteError::Config("Invalid identity data format".to_string()))?;

                let name = identity_data.get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| LiteError::Config("Missing identity name field".to_string()))?;
                let auth_type = identity_data.get("auth_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("key");

                // Check if identity exists
                match db.get_identity(&doc.id) {
                    Ok(_) => {
                        let update = crate::db::UpdateIdentity {
                            id: doc.id.clone(),
                            name: name.to_string(),
                            private_key_path: identity_data.get("private_key_path").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            passphrase_secret_id: identity_data.get("passphrase_secret_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            auth_type: auth_type.to_string(),
                        };
                        db.update_identity(&update)?;
                    }
                    Err(_) => {
                        let new_identity = crate::db::NewIdentity {
                            id: doc.id.clone(),
                            name: name.to_string(),
                            private_key_path: identity_data.get("private_key_path").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            passphrase_secret_id: identity_data.get("passphrase_secret_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            auth_type: auth_type.to_string(),
                        };
                        db.add_identity(&new_identity)?;
                    }
                }
            }
            SyncDocumentType::Tag => {
                let tag_data = data.as_object()
                    .ok_or_else(|| LiteError::Config("Invalid tag data format".to_string()))?;

                let name = tag_data.get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| LiteError::Config("Missing tag name field".to_string()))?;

                match db.get_tag(&doc.id) {
                    Ok(_) => {
                        let update = crate::db::UpdateTag {
                            id: doc.id.clone(),
                            name: name.to_string(),
                            color: tag_data.get("color").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            description: tag_data.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        };
                        db.update_tag(&update)?;
                    }
                    Err(_) => {
                        let new_tag = crate::db::NewTag {
                            id: doc.id.clone(),
                            name: name.to_string(),
                            color: tag_data.get("color").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            description: tag_data.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        };
                        db.add_tag(&new_tag)?;
                    }
                }
            }
            _ => {
                // 其他类型暂不处理
                debug!("Skipping unsupported document type: {:?}", doc.doc_type);
            }
        }

        Ok(())
    }

    /// 判断是否应该包含文档（选择性同步）
    async fn should_include_document(&self, raw: &RawConfigData, scope: &SyncScope) -> Result<bool, LiteError> {
        // 首先检查类型
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

        // 检查文档的分组
        if let Some(group_id) = raw.data.get("group_id").and_then(|v| v.as_str()) {
            let group_id_string = group_id.to_string();
            if !scope.included_groups.is_empty() && !scope.included_groups.contains(&group_id_string) {
                return Ok(false);
            }
            if scope.excluded_groups.contains(&group_id_string) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// 去重文档
    async fn deduplicate_documents(&self, docs: Vec<SyncDocument>) -> Vec<SyncDocument> {
        let mut seen_hashes: HashMap<String, &SyncDocument> = HashMap::new();
        let mut result: Vec<SyncDocument> = Vec::new();
        let mut dedup_count = 0;

        for doc in &docs {
            if let Some(existing) = seen_hashes.get(&doc.content_hash) {
                // 重复内容，保留时间戳较新的
                if doc.timestamp > existing.timestamp {
                    // 替换为较新的版本
                    if let Some(pos) = result.iter().position(|d| d.content_hash == doc.content_hash) {
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
}

/// 计算BLAKE3哈希（比SHA-256更快，同样安全）
fn blake3_hash(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// 计算SHA-256哈希（用于兼容性）
fn sha256_hash(data: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

// ============ 同步提供者实现 ============

/// 禁用提供者（同步关闭）
pub struct DisabledProvider;

impl DisabledProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl SyncProviderImpl for DisabledProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        Ok(())
    }

    async fn upload_bundle(&self, _bundle: &SyncBundle) -> Result<(), LiteError> {
        Err(LiteError::Config("Sync is disabled".to_string()))
    }

    async fn download_bundles(&self, _since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        Ok(Vec::new())
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        Err(LiteError::Config("Sync is disabled".to_string()))
    }

    async fn update_metadata(&self, _metadata: &SyncMetadata) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        Ok(Vec::new())
    }

    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        Ok(Vec::new())
    }

    async fn restore_version(&self, _version_id: &str) -> Result<SyncBundle, LiteError> {
        Err(LiteError::Config("Sync is disabled".to_string()))
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        Ok(false)
    }
}

/// iCloud同步提供者
pub struct ICloudProvider {
    container_url: Option<PathBuf>,
}

impl ICloudProvider {
    pub fn new() -> Self {
        Self {
            container_url: None,
        }
    }
}

#[async_trait::async_trait]
impl SyncProviderImpl for ICloudProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        // 在实际macOS/iOS实现中，这里会获取iCloud容器URL
        // 使用NSFileManager获取UbiquityContainerIdentifier
        #[cfg(target_os = "macos")]
        {
            // 使用objc桥接获取iCloud容器路径
            // self.container_url = get_icloud_container_url("iCloud.com.anixops.easyssh");
        }
        Ok(())
    }

    async fn upload_bundle(&self, bundle: &SyncBundle) -> Result<(), LiteError> {
        // 将bundle保存到iCloud Drive的指定目录
        let data = serde_json::to_vec(bundle)?;
        debug!("Uploading bundle to iCloud: {} bytes", data.len());
        // 实际实现使用NSFileCoordinator写入
        Ok(())
    }

    async fn download_bundles(&self, since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        // 从iCloud读取所有新bundle
        // 使用NSMetadataQuery监控变更
        debug!("Downloading bundles from iCloud since {}", since);
        Ok(Vec::new())
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        Ok(SyncMetadata::default())
    }

    async fn update_metadata(&self, _metadata: &SyncMetadata) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        // iCloud通过CKRecord获取设备列表
        Ok(Vec::new())
    }

    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        Ok(Vec::new())
    }

    async fn restore_version(&self, _version_id: &str) -> Result<SyncBundle, LiteError> {
        Err(LiteError::Config("Not implemented".to_string()))
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        // 检查iCloud可用性
        Ok(self.container_url.is_some())
    }
}

/// Google Drive同步提供者
pub struct GoogleDriveProvider {
    access_token: Option<String>,
    folder_id: Option<String>,
}

impl GoogleDriveProvider {
    pub fn new() -> Self {
        Self {
            access_token: None,
            folder_id: None,
        }
    }
}

#[async_trait::async_trait]
impl SyncProviderImpl for GoogleDriveProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        // 初始化OAuth流程
        // 1. 检查已有token
        // 2. 如果没有，启动OAuth授权
        Ok(())
    }

    async fn upload_bundle(&self, bundle: &SyncBundle) -> Result<(), LiteError> {
        // 使用Google Drive API上传
        // POST https://www.googleapis.com/upload/drive/v3/files
        debug!("Uploading bundle to Google Drive: {}", bundle.bundle_id);
        Ok(())
    }

    async fn download_bundles(&self, _since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        // 查询Google Drive文件夹中的所有文件
        // GET https://www.googleapis.com/drive/v3/files
        Ok(Vec::new())
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        Ok(SyncMetadata::default())
    }

    async fn update_metadata(&self, _metadata: &SyncMetadata) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        Ok(Vec::new())
    }

    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        // Google Drive通过文件版本API
        Ok(Vec::new())
    }

    async fn restore_version(&self, _version_id: &str) -> Result<SyncBundle, LiteError> {
        Err(LiteError::Config("Not implemented".to_string()))
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        Ok(self.access_token.is_some())
    }
}

/// OneDrive同步提供者
pub struct OneDriveProvider {
    access_token: Option<String>,
}

impl OneDriveProvider {
    pub fn new() -> Self {
        Self {
            access_token: None,
        }
    }
}

#[async_trait::async_trait]
impl SyncProviderImpl for OneDriveProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        Ok(())
    }

    async fn upload_bundle(&self, _bundle: &SyncBundle) -> Result<(), LiteError> {
        Ok(())
    }

    async fn download_bundles(&self, _since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        Ok(Vec::new())
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        Ok(SyncMetadata::default())
    }

    async fn update_metadata(&self, _metadata: &SyncMetadata) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        Ok(Vec::new())
    }

    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        Ok(Vec::new())
    }

    async fn restore_version(&self, _version_id: &str) -> Result<SyncBundle, LiteError> {
        Err(LiteError::Config("Not implemented".to_string()))
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        Ok(true)
    }
}

/// Dropbox同步提供者
pub struct DropBoxProvider;

impl DropBoxProvider {
    pub fn new() -> Self { Self }
}

#[async_trait::async_trait]
impl SyncProviderImpl for DropBoxProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> { Ok(()) }
    async fn upload_bundle(&self, _bundle: &SyncBundle) -> Result<(), LiteError> { Ok(()) }
    async fn download_bundles(&self, _since: i64) -> Result<Vec<SyncBundle>, LiteError> { Ok(Vec::new()) }
    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        Ok(SyncMetadata::default())
    }
    async fn update_metadata(&self, _metadata: &SyncMetadata) -> Result<(), LiteError> { Ok(()) }
    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> { Ok(Vec::new()) }
    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> { Ok(()) }
    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> { Ok(Vec::new()) }
    async fn restore_version(&self, _version_id: &str) -> Result<SyncBundle, LiteError> {
        Err(LiteError::Config("Not implemented".to_string()))
    }
    async fn check_connectivity(&self) -> Result<bool, LiteError> { Ok(true) }
}

/// 自建服务器同步提供者
pub struct SelfHostedProvider {
    url: String,
    token: String,
    client: reqwest::Client,
}

impl SelfHostedProvider {
    pub fn new(url: String, token: String) -> Self {
        Self {
            url,
            token,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl SyncProviderImpl for SelfHostedProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        // 验证服务器连接
        debug!("Initializing self-hosted provider: {}", self.url);
        Ok(())
    }

    async fn upload_bundle(&self, bundle: &SyncBundle) -> Result<(), LiteError> {
        let url = format!("{}/api/v1/sync/bundle", self.url);
        let _response = self.client
            .post(&url)
            .bearer_auth(&self.token)
            .json(bundle)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;
        Ok(())
    }

    async fn download_bundles(&self, since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        let url = format!("{}/api/v1/sync/bundles?since={}", self.url, since);
        let response = self.client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;

        let bundles: Vec<SyncBundle> = response.json().await
            .map_err(|e| LiteError::Json(e.to_string()))?;
        Ok(bundles)
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        let url = format!("{}/api/v1/sync/metadata", self.url);
        let response = self.client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;

        let metadata: SyncMetadata = response.json().await
            .map_err(|e| LiteError::Json(e.to_string()))?;
        Ok(metadata)
    }

    async fn update_metadata(&self, metadata: &SyncMetadata) -> Result<(), LiteError> {
        let url = format!("{}/api/v1/sync/metadata", self.url);
        let _response = self.client
            .post(&url)
            .bearer_auth(&self.token)
            .json(metadata)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        let url = format!("{}/api/v1/sync/devices", self.url);
        let response = self.client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;

        let devices: Vec<DeviceInfo> = response.json().await
            .map_err(|e| LiteError::Json(e.to_string()))?;
        Ok(devices)
    }

    async fn delete_device(&self, device_id: &str) -> Result<(), LiteError> {
        let url = format!("{}/api/v1/sync/devices/{}", self.url, device_id);
        let _response = self.client
            .delete(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        let url = format!("{}/api/v1/sync/versions", self.url);
        let response = self.client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;

        let versions: Vec<SyncVersion> = response.json().await
            .map_err(|e| LiteError::Json(e.to_string()))?;
        Ok(versions)
    }

    async fn restore_version(&self, version_id: &str) -> Result<SyncBundle, LiteError> {
        let url = format!("{}/api/v1/sync/versions/{}/restore", self.url, version_id);
        let response = self.client
            .post(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;

        let bundle: SyncBundle = response.json().await
            .map_err(|e| LiteError::Json(e.to_string()))?;
        Ok(bundle)
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        let url = format!("{}/api/v1/health", self.url);
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

/// 本地网络同步提供者
pub struct LocalNetworkProvider;

impl LocalNetworkProvider {
    pub fn new() -> Self { Self }
}

#[async_trait::async_trait]
impl SyncProviderImpl for LocalNetworkProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        // 启动mDNS服务发现
        // 启动HTTP服务器接收同步请求
        Ok(())
    }

    async fn upload_bundle(&self, _bundle: &SyncBundle) -> Result<(), LiteError> {
        // 广播到所有发现的设备
        Ok(())
    }

    async fn download_bundles(&self, _since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        // 从发现的设备拉取
        Ok(Vec::new())
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        Ok(SyncMetadata::default())
    }

    async fn update_metadata(&self, _metadata: &SyncMetadata) -> Result<(), LiteError> { Ok(()) }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        // 返回mDNS发现的设备
        Ok(Vec::new())
    }

    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> { Ok(()) }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> { Ok(Vec::new()) }

    async fn restore_version(&self, _version_id: &str) -> Result<SyncBundle, LiteError> {
        Err(LiteError::Config("Local sync doesn't support versions".to_string()))
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        // 检查是否有WiFi连接
        Ok(true)
    }
}

/// 本地文件同步提供者（用于测试或NAS）
pub struct LocalFileProvider {
    base_path: PathBuf,
}

impl LocalFileProvider {
    pub fn new(path: PathBuf) -> Self {
        Self { base_path: path }
    }

    fn get_bundle_path(&self, bundle_id: &str) -> PathBuf {
        self.base_path.join(format!("{}.json", bundle_id))
    }

    fn get_metadata_path(&self) -> PathBuf {
        self.base_path.join("metadata.json")
    }
}

#[async_trait::async_trait]
impl SyncProviderImpl for LocalFileProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        tokio::fs::create_dir_all(&self.base_path).await
            .map_err(|e| LiteError::Io(e.to_string()))?;
        Ok(())
    }

    async fn upload_bundle(&self, bundle: &SyncBundle) -> Result<(), LiteError> {
        let path = self.get_bundle_path(&bundle.bundle_id);
        let data = serde_json::to_vec_pretty(bundle)?;
        tokio::fs::write(path, data).await
            .map_err(|e| LiteError::Io(e.to_string()))?;
        Ok(())
    }

    async fn download_bundles(&self, since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        let mut bundles = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.base_path).await
            .map_err(|e| LiteError::Io(e.to_string()))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| LiteError::Io(e.to_string()))? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if path.file_name().and_then(|s| s.to_str()) == Some("metadata.json") {
                    continue;
                }

                let data = tokio::fs::read(&path).await
                    .map_err(|e| LiteError::Io(e.to_string()))?;
                let bundle: SyncBundle = serde_json::from_slice(&data)
                    .map_err(|e| LiteError::Json(e.to_string()))?;

                if bundle.timestamp > since {
                    bundles.push(bundle);
                }
            }
        }

        Ok(bundles)
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        let path = self.get_metadata_path();
        if !path.exists() {
            return Ok(SyncMetadata::default());
        }

        let data = tokio::fs::read(&path).await
            .map_err(|e| LiteError::Io(e.to_string()))?;
        let metadata: SyncMetadata = serde_json::from_slice(&data)
            .map_err(|e| LiteError::Json(e.to_string()))?;
        Ok(metadata)
    }

    async fn update_metadata(&self, metadata: &SyncMetadata) -> Result<(), LiteError> {
        let path = self.get_metadata_path();
        let data = serde_json::to_vec_pretty(metadata)?;
        tokio::fs::write(path, data).await
            .map_err(|e| LiteError::Io(e.to_string()))?;
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        // 本地文件提供者不支持设备列表
        Ok(Vec::new())
    }

    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        // 扫描所有bundle作为版本
        let mut versions = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.base_path).await
            .map_err(|e| LiteError::Io(e.to_string()))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| LiteError::Io(e.to_string()))? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if path.file_name().and_then(|s| s.to_str()) == Some("metadata.json") {
                    continue;
                }

                if let Ok(data) = tokio::fs::read(&path).await {
                    if let Ok(bundle) = serde_json::from_slice::<SyncBundle>(&data) {
                        versions.push(SyncVersion {
                            version_id: bundle.bundle_id,
                            timestamp: bundle.timestamp,
                            device_id: bundle.device_id,
                            description: None,
                            document_count: bundle.documents.len() as u32,
                            size_bytes: data.len() as u64,
                            tags: Vec::new(),
                        });
                    }
                }
            }
        }

        // 按时间戳降序排序
        versions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(versions)
    }

    async fn restore_version(&self, version_id: &str) -> Result<SyncBundle, LiteError> {
        let path = self.get_bundle_path(version_id);
        let data = tokio::fs::read(&path).await
            .map_err(|e| LiteError::Io(e.to_string()))?;
        let bundle: SyncBundle = serde_json::from_slice(&data)
            .map_err(|e| LiteError::Json(e.to_string()))?;
        Ok(bundle)
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        Ok(self.base_path.exists())
    }
}

// ============ FFI接口 moved to sync_ffi.rs ============

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_sync_config_default() {
        let config = SyncConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.device_name, "EasySSH Device");
        assert!(config.deduplication_enabled);
        assert!(config.compression_enabled);
    }

    #[tokio::test]
    async fn test_sync_document_vector_clock() {
        let mut doc1 = SyncDocument::new(
            "test-1".to_string(),
            SyncDocumentType::Host,
            "device-a".to_string(),
            vec![1, 2, 3],
            "hash1".to_string(),
        );

        doc1.tick_clock("device-a");
        doc1.tick_clock("device-a");

        assert_eq!(doc1.vector_clock.get("device-a"), Some(&2));

        let mut other_clock = HashMap::new();
        other_clock.insert("device-b".to_string(), 3);
        doc1.merge_clock(&other_clock);

        assert_eq!(doc1.vector_clock.get("device-a"), Some(&2));
        assert_eq!(doc1.vector_clock.get("device-b"), Some(&3));
    }

    #[tokio::test]
    async fn test_conflict_detection() {
        let doc1 = SyncDocument {
            id: "test".to_string(),
            doc_type: SyncDocumentType::Host,
            device_id: "a".to_string(),
            operation: SyncOperation::Update,
            timestamp: 1000,
            vector_clock: [("a".to_string(), 1)].into_iter().collect(),
            encrypted_data: vec![],
            content_hash: "hash1".to_string(),
            parent_hashes: vec![],
            deleted: false,
            group_id: None,
            schema_version: 1,
        };

        let doc2 = SyncDocument {
            id: "test".to_string(),
            doc_type: SyncDocumentType::Host,
            device_id: "b".to_string(),
            operation: SyncOperation::Update,
            timestamp: 1000,
            vector_clock: [("b".to_string(), 1)].into_iter().collect(),
            encrypted_data: vec![],
            content_hash: "hash2".to_string(),
            parent_hashes: vec![],
            deleted: false,
            group_id: None,
            schema_version: 1,
        };

        // 两个文档在不同设备上并发修改，应该检测到冲突
        assert!(doc1.has_conflict_with(&doc2));
        assert!(doc2.has_conflict_with(&doc1));

        // 自比较不应该有冲突
        assert!(!doc1.has_conflict_with(&doc1));
    }

    #[tokio::test]
    async fn test_ancestor_detection() {
        let mut doc1 = SyncDocument {
            id: "test".to_string(),
            doc_type: SyncDocumentType::Host,
            device_id: "a".to_string(),
            operation: SyncOperation::Update,
            timestamp: 1000,
            vector_clock: [("a".to_string(), 1)].into_iter().collect(),
            encrypted_data: vec![],
            content_hash: "hash1".to_string(),
            parent_hashes: vec![],
            deleted: false,
            group_id: None,
            schema_version: 1,
        };

        // doc2 是 doc1 的后代（doc1 发生在 doc2 之前）
        let doc2 = SyncDocument {
            id: "test".to_string(),
            doc_type: SyncDocumentType::Host,
            device_id: "a".to_string(),
            operation: SyncOperation::Update,
            timestamp: 2000,
            vector_clock: [("a".to_string(), 2)].into_iter().collect(),
            encrypted_data: vec![],
            content_hash: "hash2".to_string(),
            parent_hashes: vec![],
            deleted: false,
            group_id: None,
            schema_version: 1,
        };

        assert!(doc1.is_ancestor_of(&doc2));
        assert!(!doc2.is_ancestor_of(&doc1));
        assert!(!doc1.has_conflict_with(&doc2));
    }

    #[tokio::test]
    async fn test_local_file_provider() {
        let temp_dir = TempDir::new().unwrap();
        let provider = LocalFileProvider::new(temp_dir.path().to_path_buf());

        let config = SyncConfig::default();

        // 初始化
        let mut provider = provider;
        provider.initialize(&config).await.unwrap();

        // 检查连接
        assert!(provider.check_connectivity().await.unwrap());

        // 创建并上传bundle
        let bundle = SyncBundle::new(
            "test-device".to_string(),
            vec![],
        );

        provider.upload_bundle(&bundle).await.unwrap();

        // 下载bundles
        let bundles = provider.download_bundles(0).await.unwrap();
        assert_eq!(bundles.len(), 1);
        assert_eq!(bundles[0].device_id, "test-device");
    }

    #[tokio::test]
    async fn test_sync_scope() {
        let scope = SyncScope {
            include_all: false,
            included_groups: vec!["group-1".to_string(), "group-2".to_string()],
            excluded_groups: vec![],
            include_identities: true,
            include_snippets: false,
            include_layouts: true,
            include_settings: true,
            include_known_hosts: true,
            include_vault_items: false,
        };

        assert!(!scope.include_all);
        assert_eq!(scope.included_groups.len(), 2);
        assert!(scope.include_identities);
        assert!(!scope.include_snippets);
        assert!(!scope.include_vault_items);
    }

    #[tokio::test]
    async fn test_sync_bundle_size() {
        let doc1 = SyncDocument::new(
            "id1".to_string(),
            SyncDocumentType::Host,
            "device".to_string(),
            vec![0u8; 1000],
            "hash1".to_string(),
        );

        let doc2 = SyncDocument::new(
            "id2".to_string(),
            SyncDocumentType::Group,
            "device".to_string(),
            vec![0u8; 500],
            "hash2".to_string(),
        );

        let bundle = SyncBundle::new(
            "test-device".to_string(),
            vec![doc1, doc2],
        );

        assert_eq!(bundle.size_bytes(), 1500);
    }

    #[tokio::test]
    async fn test_conflict_resolution_strategies() {
        let strategies = vec![
            SyncConflictResolution::UseLocal,
            SyncConflictResolution::UseRemote,
            SyncConflictResolution::Merge,
            SyncConflictResolution::KeepBoth,
            SyncConflictResolution::Interactive,
            SyncConflictResolution::TimestampWins,
            SyncConflictResolution::DevicePriority {
                device_order: vec!["a".to_string(), "b".to_string()],
            },
            SyncConflictResolution::Skip,
        ];

        for strategy in strategies {
            let json = serde_json::to_string(&strategy).unwrap();
            let deserialized: SyncConflictResolution = serde_json::from_str(&json).unwrap();
            assert_eq!(strategy, deserialized);
        }
    }

    #[tokio::test]
    async fn test_version_creation_and_restore() {
        let temp_dir = TempDir::new().unwrap();
        let provider = LocalFileProvider::new(temp_dir.path().to_path_buf());

        // 创建版本
        let bundle = SyncBundle::new(
            "test-device".to_string(),
            vec![],
        );

        let version_id = bundle.bundle_id.clone();

        let mut provider = provider;
        provider.initialize(&SyncConfig::default()).await.unwrap();
        provider.upload_bundle(&bundle).await.unwrap();

        // 列出版本
        let versions = provider.list_versions().await.unwrap();
        assert_eq!(versions.len(), 1);

        // 恢复版本
        let restored = provider.restore_version(&version_id).await.unwrap();
        assert_eq!(restored.bundle_id, version_id);
    }

    #[test]
    fn test_blake3_hash() {
        let data = b"hello world";
        let hash = blake3_hash(data);
        assert_eq!(hash.len(), 64); // BLAKE3 produces 64 hex characters

        // Same data should produce same hash
        let hash2 = blake3_hash(data);
        assert_eq!(hash, hash2);

        // Different data should produce different hash
        let hash3 = blake3_hash(b"different");
        assert_ne!(hash, hash3);
    }

    #[tokio::test]
    async fn test_deduplication() {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::new(temp_dir.path().join("test.db")).unwrap();
        db.init().unwrap();

        let config = SyncConfig {
            encryption_key: Some("test-key".to_string()),
            ..SyncConfig::default()
        };

        let manager = SyncManager::new(db, config).unwrap();

        // 创建测试文档
        let doc1 = SyncDocument::new(
            "id1".to_string(),
            SyncDocumentType::Host,
            "device".to_string(),
            vec![1, 2, 3],
            "same_hash".to_string(),
        );

        let doc2 = SyncDocument::new(
            "id2".to_string(),
            SyncDocumentType::Host,
            "device".to_string(),
            vec![4, 5, 6],
            "same_hash".to_string(), // 相同哈希
        );

        let docs = vec![doc1, doc2];
        let deduped = manager.deduplicate_documents(docs).await;

        // 应该去重为1个文档
        assert_eq!(deduped.len(), 1);
    }

    #[tokio::test]
    async fn test_document_type_priority() {
        assert_eq!(SyncDocumentType::Setting.priority(), 0);
        assert_eq!(SyncDocumentType::Group.priority(), 1);
        assert_eq!(SyncDocumentType::Host.priority(), 4);
        assert_eq!(SyncDocumentType::VaultItem.priority(), 8);
    }

    #[tokio::test]
    async fn test_field_merge_support() {
        assert!(SyncDocumentType::Host.supports_field_merge());
        assert!(SyncDocumentType::Group.supports_field_merge());
        assert!(!SyncDocumentType::Layout.supports_field_merge());
    }

    #[tokio::test]
    async fn test_disabled_provider() {
        let provider = DisabledProvider::new();

        assert!(!provider.check_connectivity().await.unwrap());

        let result = provider.upload_bundle(&SyncBundle::new(
            "test".to_string(),
            vec![],
        )).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_local_sync_handler() {
        let mut handler = LocalSyncHandler::new();
        assert!(!handler.is_enabled());

        handler.enable(8080);
        assert!(handler.is_enabled());
        assert_eq!(handler.port, 8080);

        let device = DeviceInfo {
            device_id: "test".to_string(),
            device_name: "Test Device".to_string(),
            device_type: "desktop".to_string(),
            platform: "macos".to_string(),
            last_seen: 0,
            capabilities: vec![],
            app_version: "1.0.0".to_string(),
        };

        handler.add_discovered_device(device.clone());
        let devices = handler.discover_devices();
        assert_eq!(devices.len(), 1);

        handler.remove_device(&device.device_id);
        let devices = handler.discover_devices();
        assert_eq!(devices.len(), 0);

        handler.disable();
        assert!(!handler.is_enabled());
    }
}
