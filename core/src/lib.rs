//! EasySSH Core Library
//!
//! This crate provides the core functionality for EasySSH, a multi-platform SSH client
//! with support for Lite, Standard, and Pro editions.
//!
//! # Overview
//!
//! The library is organized into modules based on functionality:
//!
//! - **SSH Management**: [`ssh`] module provides connection pooling and session management
//! - **Cryptography**: [`crypto`] module handles encryption using AES-256-GCM and Argon2id
//! - **Database**: [`db`] module for SQLite persistence
//! - **Workflow Automation**: [`workflow_engine`], [`workflow_executor`] for automation
//! - **Database Client**: `database_client` (feature: database-client) for connecting to MySQL, PostgreSQL, etc.
//! - **Internationalization**: [`i18n`] module for multi-language support
//! - **Keychain**: [`keychain`] for secure credential storage
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use easyssh_core::{AppState, init_database, get_servers};
//!
//! // Initialize app state
//! let state = AppState::new();
//!
//! // Initialize database
//! init_database(&state).expect("Failed to initialize database");
//!
//! // Get all servers
//! let servers = get_servers(&state).expect("Failed to get servers");
//! ```
//!
//! # Feature Flags
//!
//! The library uses feature flags to control compilation:
//!
//! - `standard`: Enables Standard edition features (embedded terminal, SFTP)
//! - `pro`: Enables Pro edition features (team management, RBAC, SSO)
//! - `sftp`: SFTP file transfer support
//! - `split-screen`: Terminal splitting capabilities
//! - `monitoring`: Server monitoring features
//! - `team`: Team collaboration features
//! - `audit`: Audit logging
//! - `sso`: Single Sign-On support
//! - `database-client`: Database client functionality
//!
//! # Architecture
//!
//! ```text
//!  Application Layer (Tauri/GTK4/WinUI bindings)
//!           │
//!           ▼
//!    Core Services (SSH, Crypto, Database)
//!           │
//!           ▼
//!   Platform Layer (Keychain, Terminal, OS)
//! ```

#[cfg(feature = "git")]
pub mod git_client;
#[cfg(feature = "git")]
pub mod git_ffi;
#[cfg(feature = "git")]
pub mod git_manager;
#[cfg(feature = "git")]
pub mod git_types;
#[cfg(feature = "git")]
pub mod git_workflow;
#[cfg(feature = "git")]
pub mod git_workflow_executor;

#[cfg(feature = "git")]
pub use git_client::GitClient;
#[cfg(feature = "git")]
pub use git_manager::GitManager;
#[cfg(feature = "git")]
pub use git_types::*;
#[cfg(feature = "git")]
pub use git_workflow::*;
#[cfg(feature = "git")]
pub use git_workflow_executor::*;

#[cfg(feature = "kubernetes")]
pub mod kubernetes;
#[cfg(feature = "kubernetes")]
pub mod kubernetes_client;
#[cfg(feature = "kubernetes")]
pub mod kubernetes_ffi;
#[cfg(all(feature = "kubernetes", feature = "tauri"))]
pub mod kubernetes_tauri;

#[cfg(debug_assertions)]
pub mod ai_programming;
#[cfg(not(debug_assertions))]
pub mod ai_programming {
    //! AI Programming interface - disabled in release builds

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct SearchResult {
        pub file: String,
        pub line_number: usize,
        pub line_content: String,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct CheckResult {
        pub success: bool,
        pub errors: String,
        pub warnings: String,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct TestResult {
        pub success: bool,
        pub output: String,
        pub errors: String,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct BuildResult {
        pub success: bool,
        pub output: String,
        pub errors: String,
    }

    #[derive(serde::Serialize)]
    pub struct HealthStatus {
        pub status: String,
        pub version: String,
        pub timestamp: String,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct DebugTestReport {
        pub total: usize,
        pub passed: usize,
        pub failed: usize,
        pub results: Vec<DebugTestResult>,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct DebugTestResult {
        pub name: String,
        pub category: String,
        pub passed: bool,
        pub message: String,
        pub details: Option<String>,
    }

    fn disabled_error<T>(msg: &str) -> Result<T, String> {
        Err(format!("AI programming interface is disabled in release builds: {}", msg))
    }

    // Sync functions
    pub fn ai_health_check() -> Result<HealthStatus, String> {
        disabled_error("health_check")
    }

    pub fn debug_test_db() -> Result<DebugTestReport, String> {
        disabled_error("test_db")
    }

    pub fn debug_test_crypto() -> Result<DebugTestReport, String> {
        disabled_error("test_crypto")
    }

    pub fn debug_test_ssh() -> Result<DebugTestReport, String> {
        disabled_error("test_ssh")
    }

    pub fn debug_test_terminal() -> Result<DebugTestReport, String> {
        disabled_error("test_terminal")
    }

    pub fn debug_test_pro() -> Result<DebugTestReport, String> {
        disabled_error("test_pro")
    }

    pub fn debug_test_all() -> Result<DebugTestReport, String> {
        disabled_error("test_all")
    }

    pub fn debug_quick_check() -> Result<DebugTestReport, String> {
        disabled_error("quick_check")
    }

    // Async functions - stubs that return immediate errors
    pub async fn ai_read_code(_path: String) -> Result<String, String> {
        disabled_error("read_code")
    }

    pub async fn ai_list_files(_dir: String, _pattern: Option<String>) -> Result<Vec<String>, String> {
        disabled_error("list_files")
    }

    pub async fn ai_search_code(_query: String, _path: Option<String>) -> Result<Vec<SearchResult>, String> {
        disabled_error("search_code")
    }

    pub async fn ai_check_rust() -> Result<CheckResult, String> {
        disabled_error("check_rust")
    }

    pub async fn ai_run_tests() -> Result<TestResult, String> {
        disabled_error("run_tests")
    }

    pub async fn ai_build() -> Result<BuildResult, String> {
        disabled_error("build")
    }

    // Async file functions
    pub async fn write_file(_path: String, _content: String) -> Result<(), String> {
        disabled_error("write_file")
    }

    pub async fn edit_file(_path: String, _old: String, _new: String) -> Result<(), String> {
        disabled_error("edit_file")
    }

    // Async git functions
    pub async fn git_status() -> Result<String, String> {
        disabled_error("git_status")
    }

    pub async fn git_diff(_path: Option<String>) -> Result<String, String> {
        disabled_error("git_diff")
    }

    pub async fn git_log(_count: usize) -> Result<String, String> {
        disabled_error("git_log")
    }

    pub async fn git_branch() -> Result<Vec<String>, String> {
        disabled_error("git_branch")
    }

    // Sync context functions
    pub fn set_context(_key: String, _value: String) -> Result<(), String> {
        disabled_error("set_context")
    }

    pub fn get_context(_key: String) -> Result<Option<String>, String> {
        disabled_error("get_context")
    }

    pub fn clear_context() -> Result<(), String> {
        disabled_error("clear_context")
    }
}
#[cfg(feature = "database-client")]
pub mod database_client;
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "sync")]
pub mod sync_ffi;

#[cfg(feature = "database-client")]
pub use database_client::*;
#[cfg(feature = "audit")]
pub mod audit;
#[cfg(feature = "pro")]
pub mod collaboration;
pub mod config_import_export;
pub mod connection_pool;
pub mod crypto;
pub mod db;
pub mod debug_ws;
pub mod edition;
pub mod error;
pub mod ffi;
pub mod i18n;
pub mod i18n_ffi;
pub mod keychain;
#[cfg(feature = "split-screen")]
pub mod layout;
#[cfg(all(feature = "standard", target_os = "linux"))]
pub mod linux_service;
#[cfg(feature = "log-monitor")]
pub mod log_monitor;
#[cfg(feature = "log-monitor")]
pub mod log_monitor_ffi;
#[cfg(feature = "monitoring")]
pub mod monitoring;
#[cfg(feature = "pro")]
pub mod pro;
#[cfg(feature = "pro")]
pub mod rbac;
#[cfg(feature = "tauri")]
pub mod recording_commands;
#[cfg(feature = "sftp")]
pub mod sftp;
pub mod ssh;
#[cfg(feature = "sso")]
pub mod sso;
#[cfg(feature = "team")]
pub mod team;
#[cfg(feature = "telemetry")]
pub mod telemetry;
pub use ssh::{ConnectionHealth, PoolStats, SessionMetadata, SshSessionManager};
#[cfg(feature = "embedded-terminal")]
pub use terminal::{
    ColorPalette, CursorStyle, EscapeSequence, PtyTerminal, RenderStats, TabInfo, TabManager,
    TabState, TerminalEmulator, TerminalManager, TerminalOutput, TerminalSize, TerminalTheme,
    ThemeManager, WebGlConfig, WebGlRenderer, XtermCompat, XtermMode,
};
pub mod terminal;
pub mod vault;
#[cfg(windows)]
pub mod windows_auth;

// Backup System
#[cfg(feature = "backup")]
pub mod backup;
#[cfg(feature = "backup")]
pub use backup::*;

// Workflow Automation System
#[cfg(feature = "workflow")]
pub mod macro_recorder;
#[cfg(feature = "workflow")]
pub mod script_library;
#[cfg(feature = "workflow")]
pub mod workflow_engine;
#[cfg(feature = "workflow")]
pub mod workflow_executor;
#[cfg(feature = "workflow")]
pub mod workflow_scheduler;
#[cfg(feature = "workflow")]
pub mod workflow_variables;

#[cfg(feature = "workflow")]
pub use macro_recorder::*;
#[cfg(feature = "workflow")]
pub use script_library::*;
#[cfg(feature = "workflow")]
pub use workflow_engine::*;
#[cfg(feature = "workflow")]
pub use workflow_executor::*;
#[cfg(feature = "workflow")]
pub use workflow_scheduler::*;
#[cfg(feature = "workflow")]
pub use workflow_variables::*;

pub mod security_tests;

// Docker Management
#[cfg(feature = "docker")]
pub mod docker;

#[cfg(feature = "remote-desktop")]
pub mod remote_desktop;

#[cfg(feature = "auto-update")]
pub mod auto_update;
pub mod port_forward;

#[cfg(debug_assertions)]
pub use ai_programming::{
    ai_build, ai_check_rust, ai_health_check, ai_list_files, ai_read_code, ai_run_tests,
    ai_search_code, debug_test_all, debug_test_crypto, debug_test_db, debug_test_pro,
    debug_test_ssh, debug_test_terminal, DebugTestReport, DebugTestResult,
};
pub use config_import_export::{
    ConfigExport, ConfigManager, ConflictResolution as ConfigConflictResolution, ExportFormat,
    GroupExport, HostExport, IdentityExport, ImportFormat, ImportResult, ServerCsvRecord,
    ServerExport, SnippetExport, TagExport,
};
pub use connection_pool::{
    CompressedSessionData, CompressedSessionStore, ConnectionRateLimiter, EnhancedConnectionState,
    EnhancedPoolStats, EnhancedSshManager, EnhancedSshManagerBuilder, HealthCheckConfig,
    ReconnectConfig, SessionStoreStats,
};
pub use db::{
    AuditEventRecord, GroupRecord, HostRecord, IdentityRecord, LayoutRecord, NewAuditEvent,
    NewGroup, NewHost, NewIdentity, NewLayout, NewServer, NewSession, NewSnippet, NewSyncState,
    NewTag, ServerRecord, SessionRecord, SnippetRecord, SyncStateRecord, TagRecord, UpdateGroup,
    UpdateHost, UpdateIdentity, UpdateLayout, UpdateServer, UpdateSession, UpdateSnippet,
    UpdateSyncState, UpdateTag,
};
pub use edition::{Edition, VersionInfo};
pub use error::LiteError;
pub use i18n::{
    format_date, format_datetime, format_number, get_current_language, get_language_display_name,
    get_rtl_class, get_supported_languages, get_text_direction, init as init_i18n, is_language_rtl,
    is_rtl, set_language, t, t_args, I18nError, TextDirection, DEFAULT_LANGUAGE, RTL_LANGUAGES,
    SUPPORTED_LANGUAGES,
};
#[cfg(feature = "split-screen")]
pub use layout::{Layout, LayoutManager, Panel, PanelContent, SplitDirection};
#[cfg(all(feature = "standard", target_os = "linux"))]
pub use linux_service::{
    generate_dbus_config, generate_systemd_service, install_systemd_service, DaemonConfig,
    LinuxServiceManager, ServiceInfo, ServiceState, SystemdNotifier,
};
#[cfg(feature = "log-monitor")]
pub use log_monitor::{
    Anomaly, ErrorPattern, ExportConfig, LogAlertAction, LogAlertCondition, LogAlertEvent,
    LogAlertRule, LogAnalysisResult, LogEntry, LogFilter, LogLevel, LogMonitorCenter,
    LogMonitorWebSocketServer, LogSource, LogStats, LogTrendDirection, LogType, ParserConfig,
    TimeSeriesPoint, Trend,
};
#[cfg(feature = "sftp")]
pub use sftp::SftpSessionManager;
#[cfg(feature = "sync")]
pub use sync::{
    DeviceInfo, LocalSyncBeacon, LocalSyncHandler, RawConfigData, SyncBundle, SyncConfig,
    SyncConflict, SyncConflictResolution, SyncDocument, SyncDocumentType, SyncEvent, SyncManager,
    SyncMetadata, SyncOperation, SyncProvider, SyncScope, SyncStats, SyncStatus, SyncVersion,
};

// Monitoring exports
#[cfg(feature = "audit")]
pub use audit::{AuditAction, AuditEntry, AuditLogger, AuditTarget, AuditVerificationResult};
#[cfg(feature = "pro")]
pub use collaboration::{
    Annotation, AnnotationType, ClipboardContentType, CollaborationActionType,
    CollaborationHistory, CollaborationManager, CollaborationMessage, CollaborationParticipant,
    CollaborationRecording, CollaborationRole, CollaborationSession, CollaborationSettings,
    CollaborationState, Comment, CommentReply, CursorPosition, RecordingSegment,
    SharedClipboardItem, WebRTCSignal, WebRTCSignalType,
};
#[cfg(feature = "monitoring")]
pub use monitoring::{
    Alert, AlertCondition, AlertEngine, AlertRule, AlertSeverity, AlertStatus, CapacityForecast,
    CapacityStatus, ChartSeries, ChartType, CustomDashboard, DashboardBuilder, DashboardFormatter,
    DashboardTemplates, DashboardViewModel, HealthSummary, LayoutAlgorithm, MetricCategory,
    MetricPoint, MetricType, MonitoringConfig, MonitoringError, MonitoringManager,
    NotificationChannel, NotificationChannelType, PerformanceComparison, ResourceType,
    ServerConnectionConfig, ServerHealthStatus, ServerMetrics, ServerOverview, ServerTopology,
    SlaStats, SlaStatus, TimeRange, TopologyEdge, TopologyEdgeType, TopologyLayout, TopologyNode,
    TopologyNodeType, TopologyStatus, TrendDirection, WidgetConfig, WidgetData, WidgetType,
    WidgetViewModel,
};
#[cfg(feature = "pro")]
pub use rbac::{Permission, RbacManager, Resource, RoleDefinition};
#[cfg(feature = "sso")]
pub use sso::{
    GroupToRoleMapping, LdapConfig, NameIdFormat, OidcAttributeMapping, OidcAuthRequest,
    OidcConfig, OidcTokenResponse, OidcUserInfo, SamlAttributeMapping, SamlAuthRequest,
    SamlAuthResponse, SamlConfig, SignatureAlgorithm, SsoManager, SsoMetadata, SsoProvider,
    SsoProviderConfig, SsoProviderType, SsoSession, SsoUserInfo, TeamSsoMapping,
};
#[cfg(feature = "team")]
pub use team::{Team, TeamInvite, TeamManager, TeamMember, TeamRole};

// Telemetry exports
#[cfg(feature = "telemetry")]
pub use telemetry::storage::{AnalyticsStorage, DataExport, RetentionResult, StorageStats};
#[cfg(feature = "telemetry")]
pub use telemetry::{track_event, track_feature};
#[cfg(feature = "telemetry")]
pub use telemetry::{
    AnonymousId, ConsentCategory, ConsentManager, ConsentStatus, DataRetentionPolicy,
    EventCollector, FeatureFlag, FeatureFlagManager, FeedbackCollector, FeedbackRating,
    HealthCheck, HealthMonitor, HealthStatus, MetricsRegistry, PlatformInfo, PrivacyCompliance,
    PrivacyRegion, PrivacyReport, ReporterStats, ServiceHealth, TelemetryConfig, TelemetryEdition,
    TelemetryError, TelemetryEvent, TelemetryEventRecord, TelemetryManager,
};

// Port forwarding exports
pub use port_forward::{
    builtin_templates, init_with_templates, ForwardRule, ForwardRuleTemplate, ForwardStatus,
    ForwardTopology, ForwardType, PortForwardManager, TopologyEdge as ForwardTopologyEdge,
    TopologyEdgeType as ForwardTopologyEdgeType, TopologyNode as ForwardTopologyNode,
    TopologyNodeType as ForwardTopologyNodeType, TrafficStats,
};

#[cfg(feature = "kubernetes")]
pub use kubernetes::{
    ContainerState, ExecOptions, HelmChart, HelmMaintainer, HelmRelease, HelmRepo, K8sCluster,
    K8sConfigMap, K8sContainer, K8sContainer as K8sContainerInfo, K8sContainerPort, K8sDeployment,
    K8sDeploymentCondition, K8sError, K8sEvent, K8sManager, K8sNamespace, K8sNode, K8sNodeAddress,
    K8sNodeCondition, K8sNodeResourceUsage, K8sObjectReference, K8sPod, K8sPodCondition,
    K8sPortForward, K8sResource, K8sResourceMetadata, K8sResourceRequirements, K8sResourceUsage,
    K8sSecret, K8sService, K8sServicePort, K8sVolume, LogOptions, PodStatus, Result as K8sResult,
};

// Vault exports
pub use vault::{
    ApiKeyEntry, AutofillConfig, CertificateEntry, EmergencyAccessLevel, EncryptedVaultItem,
    EnterpriseVault, HardwareAuthMethod, HardwareDeviceInfo, InvitationStatus, NoteFormat,
    PasswordEntry, PasswordGeneratorConfig, PasswordStrength, PasswordWeakness, SecureNoteEntry,
    SecurityAuditResult, SecurityLevel, SshKeyEntry, TOTPEntry, TrustedContact, UnlockOptions,
    VaultFolder, VaultItemMetadata, VaultItemType, VaultStats,
};

// Docker exports
#[cfg(feature = "docker")]
pub use docker::{
    Actor, ClusterInfo, ClusterSpec, ComposeProject, ComposeService, ContainerInfo,
    ContainerNetworkInfo, ContainerStats, ContainerStatus, CpuStats, DockerConnection, DockerEvent,
    DockerHostType, DockerManager, DockerSystemInfo, DockerTlsConfig, HostConfig, ImageInfo,
    IoEntry, IoStats, IpamConfig, IpamSubnetConfig, LogStream, MemoryStats, MountPoint,
    NetworkContainer, NetworkInfo, NetworkSettings, NetworkStats, PidsStats, PluginsInfo,
    PortMapping, RegistryConfig, RuntimeInfo, SwarmInfo, ThrottlingData, VolumeInfo,
    VolumeUsageData,
};

use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::sync::{mpsc, Mutex};

/// Application state container for native platforms.
///
/// `AppState` holds all the shared state for the EasySSH application,
/// including database connections, SSH session managers, and various
/// optional managers for features like SFTP, team collaboration, etc.
///
/// # Example
///
/// ```rust,no_run
/// use easyssh_core::AppState;
///
/// // Create default app state
/// let state = AppState::new();
///
/// // Create with custom SSH pool configuration
/// let state = AppState::with_ssh_pool_config(10, 600, 3600);
/// ```
pub struct AppState {
    pub db: StdMutex<Option<db::Database>>,
    pub ssh_manager: Arc<Mutex<SshSessionManager>>,
    #[cfg(feature = "sftp")]
    pub sftp_manager: Arc<Mutex<SftpSessionManager>>,
    #[cfg(feature = "split-screen")]
    pub layout_manager: Arc<Mutex<LayoutManager>>,
    #[cfg(feature = "team")]
    pub team_manager: Arc<Mutex<TeamManager>>,
    #[cfg(feature = "pro")]
    pub collaboration_manager: Arc<Mutex<CollaborationManager>>,
    #[cfg(feature = "audit")]
    pub audit_logger: Arc<Mutex<AuditLogger>>,
    #[cfg(feature = "pro")]
    pub rbac_manager: Arc<Mutex<RbacManager>>,
    #[cfg(feature = "auto-update")]
    pub auto_updater: Arc<RwLock<Option<crate::auto_update::AutoUpdater>>>,
    #[cfg(feature = "telemetry")]
    pub telemetry: Arc<tokio::sync::RwLock<Option<telemetry::TelemetryManager>>>,
    /// Port forwarding manager
    pub port_forward_manager: Arc<tokio::sync::RwLock<PortForwardManager>>,
    #[cfg(feature = "sync")]
    pub sync_manager: Arc<tokio::sync::RwLock<Option<sync::SyncManager>>>,
    #[cfg(feature = "kubernetes")]
    pub k8s_manager: Arc<tokio::sync::RwLock<K8sManager>>,
    #[cfg(feature = "log-monitor")]
    pub log_monitor: Arc<tokio::sync::RwLock<Option<log_monitor::LogMonitorCenter>>>,
    #[cfg(feature = "database-client")]
    pub db_client_manager: Arc<tokio::sync::RwLock<Option<DatabaseClientManager>>>,
    #[cfg(feature = "monitoring")]
    pub monitoring_manager: Arc<tokio::sync::RwLock<Option<MonitoringManager>>>,
    #[cfg(feature = "backup")]
    pub backup_engine: Arc<tokio::sync::RwLock<Option<backup::BackupEngine>>>,
    #[cfg(feature = "sso")]
    pub sso_manager: Arc<tokio::sync::RwLock<sso::SsoManager>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            db: StdMutex::new(None),
            ssh_manager: Arc::new(Mutex::new(SshSessionManager::new())),
            #[cfg(feature = "sftp")]
            sftp_manager: Arc::new(Mutex::new(SftpSessionManager::new())),
            #[cfg(feature = "split-screen")]
            layout_manager: Arc::new(Mutex::new(LayoutManager::new())),
            #[cfg(feature = "team")]
            team_manager: Arc::new(Mutex::new(TeamManager::new())),
            #[cfg(feature = "pro")]
            collaboration_manager: Arc::new(Mutex::new(CollaborationManager::new())),
            #[cfg(feature = "audit")]
            audit_logger: Arc::new(Mutex::new(AuditLogger::new())),
            #[cfg(feature = "pro")]
            rbac_manager: Arc::new(Mutex::new(RbacManager::new())),
            #[cfg(feature = "auto-update")]
            auto_updater: Arc::new(RwLock::new(None)),
            #[cfg(feature = "telemetry")]
            telemetry: Arc::new(tokio::sync::RwLock::new(None)),
            port_forward_manager: Arc::new(tokio::sync::RwLock::new(PortForwardManager::new())),
            #[cfg(feature = "sync")]
            sync_manager: Arc::new(tokio::sync::RwLock::new(None)),
            #[cfg(feature = "kubernetes")]
            k8s_manager: Arc::new(tokio::sync::RwLock::new(K8sManager::new())),
            #[cfg(feature = "log-monitor")]
            log_monitor: Arc::new(tokio::sync::RwLock::new(None)),
            #[cfg(feature = "database-client")]
            db_client_manager: Arc::new(tokio::sync::RwLock::new(None)),
            #[cfg(feature = "monitoring")]
            monitoring_manager: Arc::new(tokio::sync::RwLock::new(None)),
            #[cfg(feature = "backup")]
            backup_engine: Arc::new(tokio::sync::RwLock::new(None)),
            #[cfg(feature = "sso")]
            sso_manager: Arc::new(tokio::sync::RwLock::new(sso::SsoManager::new())),
        }
    }

    /// Create with custom SSH pool configuration
    pub fn with_ssh_pool_config(max_connections: usize, idle_timeout: u64, max_age: u64) -> Self {
        let ssh_manager =
            SshSessionManager::new().with_pool_config(max_connections, idle_timeout, max_age);

        Self {
            db: StdMutex::new(None),
            ssh_manager: Arc::new(Mutex::new(ssh_manager)),
            #[cfg(feature = "sftp")]
            sftp_manager: Arc::new(Mutex::new(SftpSessionManager::new())),
            #[cfg(feature = "split-screen")]
            layout_manager: Arc::new(Mutex::new(LayoutManager::new())),
            #[cfg(feature = "team")]
            team_manager: Arc::new(Mutex::new(TeamManager::new())),
            #[cfg(feature = "pro")]
            collaboration_manager: Arc::new(Mutex::new(CollaborationManager::new())),
            #[cfg(feature = "audit")]
            audit_logger: Arc::new(Mutex::new(AuditLogger::new())),
            #[cfg(feature = "pro")]
            rbac_manager: Arc::new(Mutex::new(RbacManager::new())),
            #[cfg(feature = "auto-update")]
            auto_updater: Arc::new(RwLock::new(None)),
            #[cfg(feature = "telemetry")]
            telemetry: Arc::new(tokio::sync::RwLock::new(None)),
            port_forward_manager: Arc::new(tokio::sync::RwLock::new(PortForwardManager::new())),
            #[cfg(feature = "sync")]
            sync_manager: Arc::new(tokio::sync::RwLock::new(None)),
            #[cfg(feature = "kubernetes")]
            k8s_manager: Arc::new(tokio::sync::RwLock::new(K8sManager::new())),
            #[cfg(feature = "log-monitor")]
            log_monitor: Arc::new(tokio::sync::RwLock::new(None)),
            #[cfg(feature = "database-client")]
            db_client_manager: Arc::new(tokio::sync::RwLock::new(None)),
            #[cfg(feature = "monitoring")]
            monitoring_manager: Arc::new(tokio::sync::RwLock::new(None)),
            #[cfg(feature = "backup")]
            backup_engine: Arc::new(tokio::sync::RwLock::new(None)),
            #[cfg(feature = "sso")]
            sso_manager: Arc::new(tokio::sync::RwLock::new(sso::SsoManager::new())),
        }
    }
}

/// Get the default database path for the application.
///
/// Returns the platform-appropriate path for the EasySSH database file.
/// On most systems, this will be in the user's data directory.
///
/// # Example
///
/// ```rust
/// use easyssh_core::get_db_path;
///
/// let path = get_db_path();
/// println!("Database path: {:?}", path);
/// ```
pub fn get_db_path() -> std::path::PathBuf {
    db::get_db_path()
}

/// Get all servers
pub fn get_servers(state: &AppState) -> Result<Vec<ServerRecord>, LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.get_servers()
}

/// Get single server
pub fn get_server(state: &AppState, id: &str) -> Result<ServerRecord, LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.get_server(id)
}

/// Add server
pub fn add_server(state: &AppState, server: &NewServer) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.add_server(server)
}

/// Update server
pub fn update_server(state: &AppState, server: &UpdateServer) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.update_server(server)
}

/// Delete server
pub fn delete_server(state: &AppState, id: &str) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.delete_server(id)
}

/// Get all groups
pub fn get_groups(state: &AppState) -> Result<Vec<GroupRecord>, LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.get_groups()
}

/// Add group
pub fn add_group(state: &AppState, group: &NewGroup) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.add_group(group)
}

/// Update group
pub fn update_group(state: &AppState, group: &UpdateGroup) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.update_group(group)
}

/// Delete group
pub fn delete_group(state: &AppState, id: &str) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;
    db.delete_group(id)
}

/// Initialize the application database.
///
/// Creates the database file and initializes all required tables.
/// Must be called before any database operations.
///
/// # Arguments
///
/// * `state` - The application state containing the database slot
///
/// # Errors
///
/// Returns `LiteError::Config` if the database is already initialized.
/// Returns `LiteError::Database` if there's an error creating the database.
///
/// # Example
///
/// ```rust,no_run
/// use easyssh_core::{AppState, init_database};
///
/// let state = AppState::new();
/// init_database(&state).expect("Failed to initialize database");
/// ```
pub fn init_database(state: &AppState) -> Result<(), LiteError> {
    let db_path = get_db_path();
    let db = db::Database::new(db_path)?;
    db.init()?;

    let mut db_lock = state.db.lock().unwrap();
    *db_lock = Some(db);

    Ok(())
}

/// Open native terminal and connect (Lite mode)
pub fn connect_server(state: &AppState, id: &str) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock
        .as_ref()
        .ok_or(LiteError::Config("Database not initialized".to_string()))?;

    let server = db.get_server(id)?;
    terminal::open_native_terminal(
        &server.host,
        server.port as u16,
        &server.username,
        &server.auth_type,
    )
}

/// Connect to an SSH server and return session metadata.
///
/// Establishes an SSH connection using the server configuration stored in the database.
/// The session ID is generated automatically and returned in the metadata.
///
/// # Arguments
///
/// * `state` - The application state containing the SSH manager
/// * `id` - The server ID from the database
/// * `password` - Optional password for authentication (uses SSH agent if None)
///
/// # Errors
///
/// Returns `LiteError::Config` if the database is not initialized.
/// Returns `LiteError::ServerNotFound` if the server ID doesn't exist.
/// Returns `LiteError::SshConnectionFailed` if the connection fails.
/// Returns `LiteError::SshAuthFailed` if authentication fails.
///
/// # Example
///
/// ```rust,no_run
/// use easyssh_core::{AppState, ssh_connect, init_database};
///
/// async fn connect_example() {
///     let state = AppState::new();
///     init_database(&state).unwrap();
///
///     // Connect to server with ID "server-1"
///     let metadata = ssh_connect(&state, "server-1", None).await.unwrap();
///     println!("Connected: {}", metadata.id);
/// }
/// ```
pub async fn ssh_connect(
    state: &AppState,
    id: &str,
    password: Option<&str>,
) -> Result<SessionMetadata, LiteError> {
    let (host, port, username): (String, u16, String) = {
        let db_lock = state.db.lock().unwrap();
        let db = db_lock
            .as_ref()
            .ok_or(LiteError::Config("Database not initialized".to_string()))?;
        let server = db.get_server(id)?;
        (
            server.host.clone(),
            server.port as u16,
            server.username.clone(),
        )
    };

    let session_id = uuid::Uuid::new_v4().to_string();
    let mut ssh_manager = state.ssh_manager.lock().await;
    let metadata = ssh_manager
        .connect(&session_id, &host, port, &username, password)
        .await?;

    Ok(metadata)
}

/// Execute SSH command with retry
pub async fn ssh_execute(
    state: &AppState,
    session_id: &str,
    command: &str,
) -> Result<String, LiteError> {
    let ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.execute_with_retry(session_id, command, 2).await
}

/// Execute SSH command without retry
pub async fn ssh_execute_once(
    state: &AppState,
    session_id: &str,
    command: &str,
) -> Result<String, LiteError> {
    let ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.execute(session_id, command).await
}

/// Disconnect SSH session
pub async fn ssh_disconnect(state: &AppState, session_id: &str) -> Result<(), LiteError> {
    let mut ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.disconnect(session_id).await
}

/// List active SSH sessions
pub fn ssh_list_sessions(state: &AppState) -> Vec<String> {
    let ssh_manager = state.ssh_manager.blocking_lock();
    ssh_manager.list_sessions()
}

/// Get SSH pool stats
pub fn ssh_get_pool_stats(state: &AppState) -> PoolStats {
    let ssh_manager = state.ssh_manager.blocking_lock();
    ssh_manager.get_pool_stats()
}

/// Get session metadata
pub async fn ssh_get_metadata(state: &AppState, session_id: &str) -> Option<SessionMetadata> {
    let ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.get_metadata(session_id)
}

/// Start streaming shell session
pub async fn ssh_execute_stream(
    state: &AppState,
    session_id: &str,
    command: &str,
) -> Result<mpsc::UnboundedReceiver<String>, LiteError> {
    let mut ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.execute_stream(session_id, command).await
}

/// Write to shell stdin
pub async fn ssh_write_shell_input(
    state: &AppState,
    session_id: &str,
    input: &[u8],
) -> Result<(), LiteError> {
    let ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.write_shell_input(session_id, input).await
}

/// Interrupt command (Ctrl+C)
pub async fn ssh_interrupt(state: &AppState, session_id: &str) -> Result<(), LiteError> {
    let ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.interrupt_command(session_id).await
}

/// Create SFTP session
#[cfg(feature = "sftp")]
pub async fn ssh_create_sftp(state: &AppState, session_id: &str) -> Result<ssh2::Sftp, LiteError> {
    let ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.create_sftp(session_id).await
}

// Docker Management Functions

#[cfg(feature = "docker")]
/// List containers on remote host via SSH
pub async fn docker_list_containers(
    state: &AppState,
    ssh_session_id: &str,
    all: bool,
) -> Result<Vec<ContainerInfo>, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .list_containers(&ssh_manager, ssh_session_id, all)
        .await
}

#[cfg(feature = "docker")]
/// Start container on remote host
pub async fn docker_start_container(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
) -> Result<(), LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .start_container(&ssh_manager, ssh_session_id, container_id)
        .await
}

#[cfg(feature = "docker")]
/// Stop container on remote host
pub async fn docker_stop_container(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
    timeout: Option<u32>,
) -> Result<(), LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .stop_container(&ssh_manager, ssh_session_id, container_id, timeout)
        .await
}

#[cfg(feature = "docker")]
/// Restart container on remote host
pub async fn docker_restart_container(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
    timeout: Option<u32>,
) -> Result<(), LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .restart_container(&ssh_manager, ssh_session_id, container_id, timeout)
        .await
}

#[cfg(feature = "docker")]
/// Remove container on remote host
pub async fn docker_remove_container(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
    force: bool,
    volumes: bool,
) -> Result<(), LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .remove_container(&ssh_manager, ssh_session_id, container_id, force, volumes)
        .await
}

#[cfg(feature = "docker")]
/// List images on remote host
pub async fn docker_list_images(
    state: &AppState,
    ssh_session_id: &str,
    all: bool,
) -> Result<Vec<ImageInfo>, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .list_images(&ssh_manager, ssh_session_id, all, false)
        .await
}

#[cfg(feature = "docker")]
/// Pull image on remote host
pub async fn docker_pull_image(
    state: &AppState,
    ssh_session_id: &str,
    image: &str,
    tag: Option<&str>,
) -> Result<(), LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .pull_image(&ssh_manager, ssh_session_id, image, tag, None)
        .await
}

#[cfg(feature = "docker")]
/// List networks on remote host
pub async fn docker_list_networks(
    state: &AppState,
    ssh_session_id: &str,
) -> Result<Vec<NetworkInfo>, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .list_networks(&ssh_manager, ssh_session_id)
        .await
}

#[cfg(feature = "docker")]
/// List volumes on remote host
pub async fn docker_list_volumes(
    state: &AppState,
    ssh_session_id: &str,
) -> Result<Vec<VolumeInfo>, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .list_volumes(&ssh_manager, ssh_session_id)
        .await
}

#[cfg(feature = "docker")]
/// Stream container logs
pub async fn docker_stream_logs(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
    follow: bool,
    tail: Option<i64>,
) -> Result<mpsc::UnboundedReceiver<String>, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .stream_logs(&ssh_manager, ssh_session_id, container_id, follow, tail)
        .await
}

#[cfg(feature = "docker")]
/// Execute command in container
pub async fn docker_exec(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
    command: &str,
) -> Result<String, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .exec_in_container(
            &ssh_manager,
            ssh_session_id,
            container_id,
            command,
            false,
            true,
            None,
            &[],
        )
        .await
}

#[cfg(feature = "docker")]
/// Get container stats
pub async fn docker_get_stats(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
) -> Result<ContainerStats, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .get_container_stats(&ssh_manager, ssh_session_id, container_id)
        .await
}

#[cfg(feature = "docker")]
/// List Compose projects
pub async fn docker_list_compose_projects(
    state: &AppState,
    ssh_session_id: &str,
) -> Result<Vec<ComposeProject>, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .list_compose_projects(&ssh_manager, ssh_session_id)
        .await
}

#[cfg(feature = "docker")]
/// Compose up
pub async fn docker_compose_up(
    state: &AppState,
    ssh_session_id: &str,
    project_dir: &str,
) -> Result<String, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .compose_up(&ssh_manager, ssh_session_id, project_dir, None, true, false)
        .await
}

#[cfg(feature = "docker")]
/// Compose down
pub async fn docker_compose_down(
    state: &AppState,
    ssh_session_id: &str,
    project_dir: &str,
) -> Result<String, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .compose_down(&ssh_manager, ssh_session_id, project_dir, false, false)
        .await
}

#[cfg(feature = "docker")]
/// Build Docker image
pub async fn docker_build_image(
    state: &AppState,
    ssh_session_id: &str,
    context_path: &str,
    dockerfile_path: Option<&str>,
    tag: Option<&str>,
    build_args: &[(&str, &str)],
    no_cache: bool,
) -> Result<String, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .build_image(
            &ssh_manager,
            ssh_session_id,
            context_path,
            dockerfile_path,
            tag,
            build_args,
            no_cache,
        )
        .await
}

#[cfg(feature = "docker")]
/// Build Docker image with streaming output
pub async fn docker_build_image_stream(
    state: &AppState,
    ssh_session_id: &str,
    context_path: &str,
    dockerfile_path: Option<&str>,
    tag: Option<&str>,
    build_args: &[(&str, &str)],
    no_cache: bool,
) -> Result<mpsc::UnboundedReceiver<String>, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .build_image_stream(
            &ssh_manager,
            ssh_session_id,
            context_path,
            dockerfile_path,
            tag,
            build_args,
            no_cache,
        )
        .await
}

#[cfg(feature = "docker")]
/// Stream container stats
pub async fn docker_stream_stats(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
) -> Result<mpsc::UnboundedReceiver<ContainerStats>, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .stream_stats(&ssh_manager, ssh_session_id, container_id)
        .await
}

#[cfg(feature = "docker")]
/// Stop stats stream
pub async fn docker_stop_stats_stream(
    _state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
) -> Result<(), LiteError> {
    let docker_manager = DockerManager::new();
    docker_manager
        .stop_stats_stream(ssh_session_id, container_id)
        .await
}

#[cfg(feature = "docker")]
/// Export container
pub async fn docker_export_container(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
    output_path: &str,
) -> Result<(), LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .export_container(&ssh_manager, ssh_session_id, container_id, output_path)
        .await
}

#[cfg(feature = "docker")]
/// Import image
pub async fn docker_import_image(
    state: &AppState,
    ssh_session_id: &str,
    input_path: &str,
    repository: Option<&str>,
    tag: Option<&str>,
) -> Result<String, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .import_image(&ssh_manager, ssh_session_id, input_path, repository, tag)
        .await
}

#[cfg(feature = "docker")]
/// Save image to tar
pub async fn docker_save_image(
    state: &AppState,
    ssh_session_id: &str,
    image: &str,
    output_path: &str,
) -> Result<(), LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .save_image(&ssh_manager, ssh_session_id, image, output_path)
        .await
}

#[cfg(feature = "docker")]
/// Load image from tar
pub async fn docker_load_image(
    state: &AppState,
    ssh_session_id: &str,
    input_path: &str,
) -> Result<String, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .load_image(&ssh_manager, ssh_session_id, input_path)
        .await
}

#[cfg(feature = "docker")]
/// Copy from container to host
pub async fn docker_copy_from_container(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
    container_path: &str,
    host_path: &str,
) -> Result<(), LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .copy_from_container(
            &ssh_manager,
            ssh_session_id,
            container_id,
            container_path,
            host_path,
        )
        .await
}

#[cfg(feature = "docker")]
/// Copy from host to container
pub async fn docker_copy_to_container(
    state: &AppState,
    ssh_session_id: &str,
    host_path: &str,
    container_id: &str,
    container_path: &str,
) -> Result<(), LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .copy_to_container(
            &ssh_manager,
            ssh_session_id,
            host_path,
            container_id,
            container_path,
        )
        .await
}

#[cfg(feature = "docker")]
/// Get Docker system info
pub async fn docker_get_system_info(
    state: &AppState,
    ssh_session_id: &str,
) -> Result<DockerSystemInfo, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .get_system_info(&ssh_manager, ssh_session_id)
        .await
}

#[cfg(feature = "docker")]
/// Stream Docker events
pub async fn docker_stream_events(
    state: &AppState,
    ssh_session_id: &str,
    since: Option<i64>,
    until: Option<i64>,
    filters: &[(&str, &str)],
) -> Result<mpsc::UnboundedReceiver<DockerEvent>, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .stream_events(&ssh_manager, ssh_session_id, since, until, filters)
        .await
}

#[cfg(feature = "docker")]
/// Stop events stream
pub async fn docker_stop_events_stream(
    _state: &AppState,
    ssh_session_id: &str,
) -> Result<(), LiteError> {
    let docker_manager = DockerManager::new();
    docker_manager.stop_events_stream(ssh_session_id).await
}

#[cfg(feature = "docker")]
/// Inspect container
pub async fn docker_inspect_container(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
) -> Result<ContainerInfo, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .inspect_container(&ssh_manager, ssh_session_id, container_id)
        .await
}

#[cfg(feature = "docker")]
/// Inspect image
pub async fn docker_inspect_image(
    state: &AppState,
    ssh_session_id: &str,
    image_id: &str,
) -> Result<ImageInfo, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .inspect_image(&ssh_manager, ssh_session_id, image_id)
        .await
}

#[cfg(feature = "docker")]
/// Get container top (processes)
pub async fn docker_top(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
) -> Result<String, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .top(&ssh_manager, ssh_session_id, container_id)
        .await
}

#[cfg(feature = "docker")]
/// Wait for container
pub async fn docker_wait(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
) -> Result<i32, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .wait(&ssh_manager, ssh_session_id, container_id)
        .await
}

#[cfg(feature = "docker")]
/// Rename container
pub async fn docker_rename_container(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
    new_name: &str,
) -> Result<(), LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .rename_container(&ssh_manager, ssh_session_id, container_id, new_name)
        .await
}

#[cfg(feature = "docker")]
/// Update container resources
pub async fn docker_update_container(
    state: &AppState,
    ssh_session_id: &str,
    container_id: &str,
    cpu_shares: Option<i64>,
    memory: Option<i64>,
    memory_swap: Option<i64>,
    cpu_period: Option<i64>,
    cpu_quota: Option<i64>,
    restart_policy: Option<&str>,
) -> Result<(), LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .update_container(
            &ssh_manager,
            ssh_session_id,
            container_id,
            cpu_shares,
            memory,
            memory_swap,
            cpu_period,
            cpu_quota,
            restart_policy,
        )
        .await
}

#[cfg(feature = "docker")]
/// Run container (create + start)
pub async fn docker_run_container(
    state: &AppState,
    ssh_session_id: &str,
    name: Option<&str>,
    image: &str,
    command: Option<&str>,
    ports: &[(u16, u16, &str)],
    volumes: &[(&str, &str)],
    env: &[(&str, &str)],
    network: Option<&str>,
    restart: Option<&str>,
    labels: &[(&str, &str)],
    detach: bool,
    auto_remove: bool,
) -> Result<String, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .run_container(
            &ssh_manager,
            ssh_session_id,
            name,
            image,
            command,
            ports,
            volumes,
            env,
            network,
            restart,
            labels,
            detach,
            auto_remove,
        )
        .await
}

#[cfg(feature = "docker")]
/// Get disk usage
pub async fn docker_get_disk_usage(
    state: &AppState,
    ssh_session_id: &str,
) -> Result<String, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .get_disk_usage(&ssh_manager, ssh_session_id)
        .await
}

#[cfg(feature = "docker")]
/// System prune
pub async fn docker_system_prune(
    state: &AppState,
    ssh_session_id: &str,
    all: bool,
    volumes: bool,
) -> Result<String, LiteError> {
    let docker_manager = DockerManager::new();
    let ssh_manager = state.ssh_manager.lock().await;
    docker_manager
        .system_prune(&ssh_manager, ssh_session_id, all, volumes)
        .await
}
