//! 审计日志模块 - Pro服务器端
//! 提供完整的审计追踪、实时告警、数据分析和归档功能
//!
//! 支持的存储后端:
//! - PostgreSQL (主存储)
//! - ClickHouse (大数据分析)
//! - S3 (长期归档)
//!
//! 合规性支持:
//! - SOC2 Type II
//! - ISO27001
//! - 等保2.0

pub mod alerting;
pub mod exporter;
pub mod logger;
pub mod models;
pub mod query;
pub mod storage;

pub use alerting::{AlertEngine, AlertRule, RealTimeAlert};
pub use exporter::{AuditExporter, ExportFormat, SIEMConfig};
pub use logger::AuditLogger;
pub use models::*;
pub use query::{AuditQuery, QueryBuilder, QueryResult};
pub use storage::{ArchiveConfig, AuditStorage, ClickHouseStorage, PostgresStorage, S3Archive};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// 审计模块错误类型
#[derive(Debug, Error)]
pub enum AuditError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Export error: {0}")]
    Export(String),

    #[error("Alert error: {0}")]
    Alert(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Compliance violation: {0}")]
    Compliance(String),
}

/// 审计结果类型
pub type AuditResult<T> = Result<T, AuditError>;

/// 审计事件类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "audit_event_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    // 认证事件
    LoginSuccess,
    LoginFailure,
    Logout,
    PasswordChange,
    MfaEnabled,
    MfaDisabled,
    TokenRefresh,
    SessionExpired,

    // 服务器事件
    ServerCreated,
    ServerUpdated,
    ServerDeleted,
    ServerConnected,
    ServerDisconnected,
    ServerImported,
    ServerExported,

    // 会话事件
    SessionStarted,
    SessionEnded,
    SessionRecorded,
    SessionShared,
    CommandExecuted,
    FileTransferred,

    // 团队事件
    MemberInvited,
    MemberJoined,
    MemberRemoved,
    RoleChanged,
    TeamCreated,
    TeamUpdated,
    TeamDeleted,

    // 权限事件
    PermissionGranted,
    PermissionRevoked,
    RoleCreated,
    RoleUpdated,
    RoleDeleted,

    // 配置事件
    ConfigChanged,
    SettingUpdated,
    SecretViewed,
    KeyUploaded,
    KeyDeleted,

    // 安全事件
    PermissionDenied,
    SuspiciousActivity,
    BruteForceAttempt,
    IpBlocked,
    IpUnblocked,
    ViolationDetected,

    // 合规事件
    AuditExported,
    DataRetrieved,
    PolicyViolation,
}

impl AuditEventType {
    /// 获取事件分类
    pub fn category(&self) -> AuditCategory {
        match self {
            AuditEventType::LoginSuccess
            | AuditEventType::LoginFailure
            | AuditEventType::Logout
            | AuditEventType::PasswordChange
            | AuditEventType::MfaEnabled
            | AuditEventType::MfaDisabled
            | AuditEventType::TokenRefresh
            | AuditEventType::SessionExpired => AuditCategory::Authentication,

            AuditEventType::ServerCreated
            | AuditEventType::ServerUpdated
            | AuditEventType::ServerDeleted
            | AuditEventType::ServerConnected
            | AuditEventType::ServerDisconnected
            | AuditEventType::ServerImported
            | AuditEventType::ServerExported => AuditCategory::Server,

            AuditEventType::SessionStarted
            | AuditEventType::SessionEnded
            | AuditEventType::SessionRecorded
            | AuditEventType::SessionShared
            | AuditEventType::CommandExecuted
            | AuditEventType::FileTransferred => AuditCategory::Session,

            AuditEventType::MemberInvited
            | AuditEventType::MemberJoined
            | AuditEventType::MemberRemoved
            | AuditEventType::RoleChanged
            | AuditEventType::TeamCreated
            | AuditEventType::TeamUpdated
            | AuditEventType::TeamDeleted => AuditCategory::Team,

            AuditEventType::PermissionGranted
            | AuditEventType::PermissionRevoked
            | AuditEventType::RoleCreated
            | AuditEventType::RoleUpdated
            | AuditEventType::RoleDeleted => AuditCategory::Permission,

            AuditEventType::ConfigChanged
            | AuditEventType::SettingUpdated
            | AuditEventType::SecretViewed
            | AuditEventType::KeyUploaded
            | AuditEventType::KeyDeleted => AuditCategory::Configuration,

            AuditEventType::PermissionDenied
            | AuditEventType::SuspiciousActivity
            | AuditEventType::BruteForceAttempt
            | AuditEventType::IpBlocked
            | AuditEventType::IpUnblocked
            | AuditEventType::ViolationDetected => AuditCategory::Security,

            AuditEventType::AuditExported
            | AuditEventType::DataRetrieved
            | AuditEventType::PolicyViolation => AuditCategory::Compliance,
        }
    }

    /// 获取事件严重级别
    pub fn severity(&self) -> Severity {
        match self {
            // 高危事件
            AuditEventType::LoginFailure
            | AuditEventType::SuspiciousActivity
            | AuditEventType::BruteForceAttempt
            | AuditEventType::PermissionDenied
            | AuditEventType::ViolationDetected
            | AuditEventType::PolicyViolation => Severity::High,

            // 中危事件
            AuditEventType::PasswordChange
            | AuditEventType::MfaDisabled
            | AuditEventType::ServerDeleted
            | AuditEventType::TeamDeleted
            | AuditEventType::MemberRemoved
            | AuditEventType::RoleDeleted
            | AuditEventType::KeyDeleted
            | AuditEventType::SecretViewed
            | AuditEventType::IpBlocked => Severity::Medium,

            // 低危事件
            _ => Severity::Low,
        }
    }

    /// 是否需要实时告警
    pub fn requires_alert(&self) -> bool {
        matches!(
            self,
            AuditEventType::LoginFailure
                | AuditEventType::SuspiciousActivity
                | AuditEventType::BruteForceAttempt
                | AuditEventType::PermissionDenied
                | AuditEventType::ViolationDetected
                | AuditEventType::PolicyViolation
                | AuditEventType::IpBlocked
        )
    }

    /// 获取事件描述
    pub fn description(&self) -> &'static str {
        match self {
            AuditEventType::LoginSuccess => "用户登录成功",
            AuditEventType::LoginFailure => "用户登录失败",
            AuditEventType::Logout => "用户登出",
            AuditEventType::PasswordChange => "密码修改",
            AuditEventType::MfaEnabled => "启用多因素认证",
            AuditEventType::MfaDisabled => "禁用多因素认证",
            AuditEventType::TokenRefresh => "令牌刷新",
            AuditEventType::SessionExpired => "会话过期",
            AuditEventType::ServerCreated => "创建服务器",
            AuditEventType::ServerUpdated => "更新服务器",
            AuditEventType::ServerDeleted => "删除服务器",
            AuditEventType::ServerConnected => "连接服务器",
            AuditEventType::ServerDisconnected => "断开服务器",
            AuditEventType::ServerImported => "导入服务器",
            AuditEventType::ServerExported => "导出服务器",
            AuditEventType::SessionStarted => "开始会话",
            AuditEventType::SessionEnded => "结束会话",
            AuditEventType::SessionRecorded => "录制会话",
            AuditEventType::SessionShared => "共享会话",
            AuditEventType::CommandExecuted => "执行命令",
            AuditEventType::FileTransferred => "文件传输",
            AuditEventType::MemberInvited => "邀请成员",
            AuditEventType::MemberJoined => "成员加入",
            AuditEventType::MemberRemoved => "移除成员",
            AuditEventType::RoleChanged => "变更角色",
            AuditEventType::TeamCreated => "创建团队",
            AuditEventType::TeamUpdated => "更新团队",
            AuditEventType::TeamDeleted => "删除团队",
            AuditEventType::PermissionGranted => "授予权限",
            AuditEventType::PermissionRevoked => "撤销权限",
            AuditEventType::RoleCreated => "创建角色",
            AuditEventType::RoleUpdated => "更新角色",
            AuditEventType::RoleDeleted => "删除角色",
            AuditEventType::ConfigChanged => "配置变更",
            AuditEventType::SettingUpdated => "设置更新",
            AuditEventType::SecretViewed => "查看密钥",
            AuditEventType::KeyUploaded => "上传密钥",
            AuditEventType::KeyDeleted => "删除密钥",
            AuditEventType::PermissionDenied => "权限拒绝",
            AuditEventType::SuspiciousActivity => "可疑活动",
            AuditEventType::BruteForceAttempt => "暴力破解尝试",
            AuditEventType::IpBlocked => "IP被阻止",
            AuditEventType::IpUnblocked => "IP解除阻止",
            AuditEventType::ViolationDetected => "违规检测",
            AuditEventType::AuditExported => "审计导出",
            AuditEventType::DataRetrieved => "数据检索",
            AuditEventType::PolicyViolation => "策略违规",
        }
    }
}

/// 审计分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "audit_category", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    Authentication,
    Server,
    Session,
    Team,
    Permission,
    Configuration,
    Security,
    Compliance,
}

/// 严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, PartialOrd, Ord)]
#[sqlx(type_name = "severity", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// 审计记录主结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub category: AuditCategory,
    pub severity: Severity,
    pub user_id: String,
    pub user_name: String,
    pub team_id: Option<String>,
    pub ip_address: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub action: String,
    pub result: ActionResult,
    pub details: Option<serde_json::Value>,
    pub session_id: Option<String>,
    pub user_agent: Option<String>,
    pub location: Option<GeoLocation>,
    pub compliance: ComplianceInfo,
    pub chain_hash: Option<String>,
    pub signature: Option<String>,
}

/// 操作结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "action_result", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActionResult {
    Success,
    Failure,
    Denied,
    Error,
    Timeout,
}

/// 地理位置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub country: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: Option<String>,
}

/// 合规信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComplianceInfo {
    /// 数据保留策略ID
    pub retention_policy_id: Option<String>,
    /// 合规框架 (SOC2, ISO27001, 等保2.0)
    pub frameworks: Vec<String>,
    /// 数据分类
    pub data_classification: DataClassification,
    /// 是否需要加密
    pub encryption_required: bool,
    /// 审计完整性验证
    pub integrity_verified: bool,
}

/// 数据分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DataClassification {
    #[default]
    Public,
    Internal,
    Confidential,
    Restricted,
}

impl AuditRecord {
    /// 创建新的审计记录
    pub fn new(
        event_type: AuditEventType,
        user_id: impl Into<String>,
        user_name: impl Into<String>,
        ip_address: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: now,
            event_type,
            category: event_type.category(),
            severity: event_type.severity(),
            user_id: user_id.into(),
            user_name: user_name.into(),
            team_id: None,
            ip_address: ip_address.into(),
            resource_type: String::new(),
            resource_id: None,
            action: event_type.description().to_string(),
            result: ActionResult::Success,
            details: None,
            session_id: None,
            user_agent: None,
            location: None,
            compliance: ComplianceInfo::default(),
            chain_hash: None,
            signature: None,
        }
    }

    /// 设置团队ID
    pub fn with_team_id(mut self, team_id: impl Into<String>) -> Self {
        self.team_id = Some(team_id.into());
        self
    }

    /// 设置资源信息
    pub fn with_resource(
        mut self,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
    ) -> Self {
        self.resource_type = resource_type.into();
        self.resource_id = Some(resource_id.into());
        self
    }

    /// 设置操作结果
    pub fn with_result(mut self, result: ActionResult) -> Self {
        self.result = result;
        self
    }

    /// 设置详情
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// 设置会话ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 设置User Agent
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// 设置地理位置
    pub fn with_location(mut self, location: GeoLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// 设置合规信息
    pub fn with_compliance(mut self, compliance: ComplianceInfo) -> Self {
        self.compliance = compliance;
        self
    }

    /// 计算记录哈希 (用于防篡改)
    pub fn compute_hash(&self) -> String {
        use blake3::Hasher;
        let mut hasher = Hasher::new();

        hasher.update(self.id.as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(format!("{:?}", self.event_type).as_bytes());
        hasher.update(self.user_id.as_bytes());
        hasher.update(self.ip_address.as_bytes());
        hasher.update(self.resource_type.as_bytes());
        if let Some(ref resource_id) = self.resource_id {
            hasher.update(resource_id.as_bytes());
        }
        hasher.update(format!("{:?}", self.result).as_bytes());
        if let Some(ref session_id) = self.session_id {
            hasher.update(session_id.as_bytes());
        }

        hasher.finalize().to_hex().to_string()
    }

    /// 密封记录 (添加哈希和签名)
    pub fn seal(mut self, previous_hash: Option<&str>, signing_key: Option<&[u8]>) -> Self {
        if let Some(prev) = previous_hash {
            self.chain_hash = Some(prev.to_string());
        }

        let hash = self.compute_hash();

        if let Some(key) = signing_key {
            self.signature = Some(Self::sign(&hash, key));
        }

        self
    }

    /// 签名数据
    fn sign(data: &str, key: &[u8]) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// 转换为CSV行
    pub fn to_csv(&self) -> String {
        let details = self
            .details
            .as_ref()
            .map(|d| d.to_string())
            .unwrap_or_default()
            .replace('"', "\"\"");

        format!(
            "{},{},{:?},{},{},{},{},{},{:?},{},{},\"{}\"",
            self.timestamp.to_rfc3339(),
            self.id,
            self.event_type,
            self.user_id,
            self.user_name,
            self.team_id.as_deref().unwrap_or(""),
            self.ip_address,
            self.resource_type,
            self.result,
            self.session_id.as_deref().unwrap_or(""),
            self.user_agent.as_deref().unwrap_or(""),
            details
        )
    }
}

/// 审计配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// PostgreSQL连接字符串
    pub postgres_url: String,
    /// ClickHouse连接字符串 (可选)
    pub clickhouse_url: Option<String>,
    /// S3归档配置 (可选)
    pub s3_config: Option<S3Config>,
    /// 实时告警配置
    pub alerting: AlertingConfig,
    /// 防篡改签名密钥
    pub signing_key: Option<String>,
    /// 默认数据保留天数
    pub retention_days: i32,
    /// 归档阈值天数 (超过此天数的记录自动归档到S3)
    pub archive_threshold_days: i32,
    /// 合规框架
    pub compliance_frameworks: Vec<String>,
}

/// S3配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub endpoint: Option<String>,
    pub prefix: Option<String>,
}

/// 告警配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// 是否启用实时告警
    pub enabled: bool,
    /// Webhook URL
    pub webhook_url: Option<String>,
    /// 邮件通知配置
    pub email_config: Option<EmailConfig>,
    /// 告警聚合窗口 (秒)
    pub aggregation_window_secs: u64,
    /// 告警抑制时间 (分钟)
    pub suppression_minutes: i64,
}

/// 邮件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub to_addresses: Vec<String>,
    pub use_tls: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            postgres_url: String::new(),
            clickhouse_url: None,
            s3_config: None,
            alerting: AlertingConfig::default(),
            signing_key: None,
            retention_days: 365,
            archive_threshold_days: 90,
            compliance_frameworks: vec!["SOC2".to_string(), "ISO27001".to_string()],
        }
    }
}

impl Default for AlertingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            webhook_url: None,
            email_config: None,
            aggregation_window_secs: 300,
            suppression_minutes: 60,
        }
    }
}

/// 审计统计摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_records: i64,
    pub records_by_category: std::collections::HashMap<String, i64>,
    pub records_by_severity: std::collections::HashMap<String, i64>,
    pub failed_logins: i64,
    pub unique_users: i64,
    pub unique_ips: i64,
    pub time_range_start: DateTime<Utc>,
    pub time_range_end: DateTime<Utc>,
}

/// 审计报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub report_id: String,
    pub generated_at: DateTime<Utc>,
    pub generated_by: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub summary: AuditSummary,
    pub top_events: Vec<EventSummary>,
    pub top_users: Vec<UserSummary>,
    pub anomalies: Vec<Anomaly>,
    pub compliance_status: ComplianceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    pub event_type: AuditEventType,
    pub count: i64,
    pub trend: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSummary {
    pub user_id: String,
    pub user_name: String,
    pub action_count: i64,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub anomaly_type: String,
    pub severity: Severity,
    pub description: String,
    pub affected_user: Option<String>,
    pub affected_ip: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub related_events: Vec<String>,
}

/// 合规状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub soc2_compliant: bool,
    pub iso27001_compliant: bool,
    pub dengbao_compliant: bool,
    pub violations: Vec<ComplianceViolation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceViolation {
    pub framework: String,
    pub control_id: String,
    pub description: String,
    pub severity: Severity,
    pub remediation: String,
}

/// 审计验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub valid: bool,
    pub total_records: usize,
    pub tampered_records: Vec<String>,
    pub broken_chain_at: Option<usize>,
    pub integrity_score: f64,
}

/// 批量审计记录请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAuditRequest {
    pub records: Vec<AuditRecord>,
    pub batch_id: Option<String>,
}

/// 批量审计响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAuditResponse {
    pub batch_id: String,
    pub processed: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}
