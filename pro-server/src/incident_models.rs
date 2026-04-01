//! DevOps事件响应中心 - 事件检测、告警管理、协作处理
//!
//! 功能模块：
//! 1. 事件检测 - 自动检测服务器异常
//! 2. 告警聚合 - 相似告警合并，防止告警风暴
//! 3. 事件时间线 - 完整的事件时间线记录
//! 4. 自动诊断 - AI辅助问题诊断
//! 5. 运行手册 - 内置常见故障处理手册
//! 6. 协作处理 - 多人协作处理事件
//! 7. 影响分析 - 自动分析影响范围
//! 8. 升级策略 - 自动或手动事件升级
//! 9. 事后复盘 - 生成事件报告和改进建议
//! 10. 集成通知 - PagerDuty/OpsGenie/Slack集成

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

// ============= 事件严重程度 =============

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq, Eq, PartialOrd, Ord)]
#[sqlx(rename = "TEXT")]
pub enum IncidentSeverity {
    #[serde(rename = "critical")]
    Critical, // P1 - 系统完全不可用
    #[serde(rename = "high")]
    High, // P2 - 核心功能受损
    #[serde(rename = "medium")]
    Medium, // P3 - 部分功能受影响
    #[serde(rename = "low")]
    Low, // P4 - 轻微影响
    #[serde(rename = "info")]
    Info, // P5 - 信息性告警
}

impl IncidentSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            IncidentSeverity::Critical => "critical",
            IncidentSeverity::High => "high",
            IncidentSeverity::Medium => "medium",
            IncidentSeverity::Low => "low",
            IncidentSeverity::Info => "info",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "critical" => Some(IncidentSeverity::Critical),
            "high" => Some(IncidentSeverity::High),
            "medium" => Some(IncidentSeverity::Medium),
            "low" => Some(IncidentSeverity::Low),
            "info" => Some(IncidentSeverity::Info),
            _ => None,
        }
    }

    pub fn to_priority(&self) -> i32 {
        match self {
            IncidentSeverity::Critical => 1,
            IncidentSeverity::High => 2,
            IncidentSeverity::Medium => 3,
            IncidentSeverity::Low => 4,
            IncidentSeverity::Info => 5,
        }
    }

    pub fn sla_minutes(&self) -> i64 {
        match self {
            IncidentSeverity::Critical => 15, // 15分钟内必须响应
            IncidentSeverity::High => 60,     // 1小时内响应
            IncidentSeverity::Medium => 240,  // 4小时内响应
            IncidentSeverity::Low => 1440,    // 24小时内响应
            IncidentSeverity::Info => 10080,  // 1周内响应
        }
    }
}

// ============= 事件状态 =============

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq)]
#[sqlx(rename = "TEXT")]
pub enum IncidentStatus {
    #[serde(rename = "detected")]
    Detected, // 已检测
    #[serde(rename = "acknowledged")]
    Acknowledged, // 已确认
    #[serde(rename = "investigating")]
    Investigating, // 调查中
    #[serde(rename = "mitigating")]
    Mitigating, // 缓解中
    #[serde(rename = "resolved")]
    Resolved, // 已解决
    #[serde(rename = "closed")]
    Closed, // 已关闭
    #[serde(rename = "escalated")]
    Escalated, // 已升级
}

impl IncidentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            IncidentStatus::Detected => "detected",
            IncidentStatus::Acknowledged => "acknowledged",
            IncidentStatus::Investigating => "investigating",
            IncidentStatus::Mitigating => "mitigating",
            IncidentStatus::Resolved => "resolved",
            IncidentStatus::Closed => "closed",
            IncidentStatus::Escalated => "escalated",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self,
            IncidentStatus::Detected
                | IncidentStatus::Acknowledged
                | IncidentStatus::Investigating
                | IncidentStatus::Mitigating
                | IncidentStatus::Escalated
        )
    }
}

// ============= 事件类型 =============

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq)]
#[sqlx(rename = "TEXT")]
pub enum IncidentType {
    #[serde(rename = "server_down")]
    ServerDown,
    #[serde(rename = "high_cpu")]
    HighCpu,
    #[serde(rename = "high_memory")]
    HighMemory,
    #[serde(rename = "disk_full")]
    DiskFull,
    #[serde(rename = "network_issue")]
    NetworkIssue,
    #[serde(rename = "service_unavailable")]
    ServiceUnavailable,
    #[serde(rename = "security_breach")]
    SecurityBreach,
    #[serde(rename = "ssl_expired")]
    SslExpired,
    #[serde(rename = "backup_failed")]
    BackupFailed,
    #[serde(rename = "database_error")]
    DatabaseError,
    #[serde(rename = "application_error")]
    ApplicationError,
    #[serde(rename = "hardware_failure")]
    HardwareFailure,
    #[serde(rename = "ddos_attack")]
    DdosAttack,
    #[serde(rename = "configuration_error")]
    ConfigurationError,
    #[serde(rename = "custom")]
    Custom,
}

impl IncidentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            IncidentType::ServerDown => "server_down",
            IncidentType::HighCpu => "high_cpu",
            IncidentType::HighMemory => "high_memory",
            IncidentType::DiskFull => "disk_full",
            IncidentType::NetworkIssue => "network_issue",
            IncidentType::ServiceUnavailable => "service_unavailable",
            IncidentType::SecurityBreach => "security_breach",
            IncidentType::SslExpired => "ssl_expired",
            IncidentType::BackupFailed => "backup_failed",
            IncidentType::DatabaseError => "database_error",
            IncidentType::ApplicationError => "application_error",
            IncidentType::HardwareFailure => "hardware_failure",
            IncidentType::DdosAttack => "ddos_attack",
            IncidentType::ConfigurationError => "configuration_error",
            IncidentType::Custom => "custom",
        }
    }

    pub fn default_severity(&self) -> IncidentSeverity {
        match self {
            IncidentType::ServerDown => IncidentSeverity::Critical,
            IncidentType::SecurityBreach => IncidentSeverity::Critical,
            IncidentType::DdosAttack => IncidentSeverity::Critical,
            IncidentType::ServiceUnavailable => IncidentSeverity::High,
            IncidentType::DatabaseError => IncidentSeverity::High,
            IncidentType::HardwareFailure => IncidentSeverity::High,
            IncidentType::HighCpu => IncidentSeverity::Medium,
            IncidentType::HighMemory => IncidentSeverity::Medium,
            IncidentType::DiskFull => IncidentSeverity::Medium,
            IncidentType::NetworkIssue => IncidentSeverity::Medium,
            IncidentType::BackupFailed => IncidentSeverity::Medium,
            IncidentType::SslExpired => IncidentSeverity::Medium,
            IncidentType::ApplicationError => IncidentSeverity::Low,
            IncidentType::ConfigurationError => IncidentSeverity::Low,
            IncidentType::Custom => IncidentSeverity::Info,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            IncidentType::ServerDown => "服务器无法访问或已宕机",
            IncidentType::HighCpu => "CPU使用率过高",
            IncidentType::HighMemory => "内存使用率过高",
            IncidentType::DiskFull => "磁盘空间不足",
            IncidentType::NetworkIssue => "网络连接问题",
            IncidentType::ServiceUnavailable => "服务不可用",
            IncidentType::SecurityBreach => "安全入侵检测",
            IncidentType::SslExpired => "SSL证书过期",
            IncidentType::BackupFailed => "备份失败",
            IncidentType::DatabaseError => "数据库错误",
            IncidentType::ApplicationError => "应用程序错误",
            IncidentType::HardwareFailure => "硬件故障",
            IncidentType::DdosAttack => "DDoS攻击",
            IncidentType::ConfigurationError => "配置错误",
            IncidentType::Custom => "自定义事件",
        }
    }
}

// ============= 主事件模型 =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Incident {
    pub id: String,
    pub incident_number: String, // 格式: INC-YYYYMMDD-XXXX
    pub title: String,
    pub description: String,
    pub incident_type: IncidentType,
    pub severity: IncidentSeverity,
    pub status: IncidentStatus,
    pub team_id: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub detected_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub resolved_by: Option<String>,
    pub root_cause: Option<String>,
    pub impact_summary: Option<String>,
    pub affected_servers: Option<serde_json::Value>, // JSON array of server IDs
    pub affected_services: Option<serde_json::Value>, // JSON array of service names
    pub assigned_to: Option<String>,
    pub escalation_level: i32,                        // 0-5
    pub parent_incident_id: Option<String>,           // 用于关联到父事件
    pub related_incidents: Option<serde_json::Value>, // JSON array of incident IDs
    pub tags: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>, // 额外元数据
}

// ============= 告警模型 =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Alert {
    pub id: String,
    pub alert_number: String,        // 格式: ALERT-YYYYMMDD-XXXX
    pub incident_id: Option<String>, // 关联到事件
    pub source: String,              // 来源: monitoring, prometheus, zabbix, custom
    pub alert_type: String,
    pub severity: IncidentSeverity,
    pub title: String,
    pub description: String,
    pub team_id: String,
    pub server_id: Option<String>,
    pub service_name: Option<String>,
    pub metric_name: Option<String>,
    pub metric_value: Option<f64>,
    pub threshold: Option<f64>,
    pub status: AlertStatus,
    pub created_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub fingerprint: String, // 用于告警聚合
    pub aggregation_key: Option<String>,
    pub occurrence_count: i32, // 聚合计数
    pub first_occurrence_at: DateTime<Utc>,
    pub last_occurrence_at: DateTime<Utc>,
    pub raw_data: Option<serde_json::Value>, // 原始告警数据
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq)]
#[sqlx(rename = "TEXT")]
pub enum AlertStatus {
    #[serde(rename = "firing")]
    Firing, // 告警触发中
    #[serde(rename = "acknowledged")]
    Acknowledged, // 已确认
    #[serde(rename = "resolved")]
    Resolved, // 已解决
    #[serde(rename = "suppressed")]
    Suppressed, // 已抑制
    #[serde(rename = "flapping")]
    Flapping, // 抖动中
}

// ============= 事件时间线条目 =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct IncidentTimelineEntry {
    pub id: String,
    pub incident_id: String,
    pub entry_type: TimelineEntryType,
    pub title: String,
    pub description: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq)]
#[sqlx(rename = "TEXT")]
pub enum TimelineEntryType {
    #[serde(rename = "status_change")]
    StatusChange,
    #[serde(rename = "severity_change")]
    SeverityChange,
    #[serde(rename = "assignment")]
    Assignment,
    #[serde(rename = "escalation")]
    Escalation,
    #[serde(rename = "note")]
    Note,
    #[serde(rename = "action")]
    Action,
    #[serde(rename = "diagnosis")]
    Diagnosis,
    #[serde(rename = "communication")]
    Communication,
    #[serde(rename = "automation")]
    Automation,
    #[serde(rename = "alert")]
    Alert,
    #[serde(rename = "runbook_executed")]
    RunbookExecuted,
}

// ============= 诊断结果 =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DiagnosisResult {
    pub id: String,
    pub incident_id: String,
    pub diagnosis_type: String, // ai, manual, automated
    pub findings: String,
    pub confidence_score: Option<f64>, // AI置信度 0-1
    pub suggested_actions: Option<serde_json::Value>, // JSON array
    pub runbook_suggestions: Option<serde_json::Value>, // JSON array of runbook IDs
    pub related_incidents: Option<serde_json::Value>, // JSON array
    pub similar_past_incidents: Option<serde_json::Value>,
    pub created_by: String, // AI或用户ID
    pub created_at: DateTime<Utc>,
    pub is_primary: bool, // 是否是主要诊断
}

// ============= 运行手册 =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Runbook {
    pub id: String,
    pub title: String,
    pub description: String,
    pub incident_types: Option<serde_json::Value>, // 适用的告警类型
    pub severity_levels: Option<serde_json::Value>, // 适用的严重程度
    pub team_id: String,
    pub is_global: bool,                   // 是否是全局手册
    pub content: String,                   // Markdown格式内容
    pub steps: Option<serde_json::Value>,  // 结构化步骤 JSON
    pub automation_script: Option<String>, // 可选的自动化脚本
    pub estimated_duration_minutes: Option<i32>,
    pub success_rate: Option<f64>, // 历史成功率
    pub usage_count: i32,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
    pub tags: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RunbookStep {
    pub step_number: i32,
    pub title: String,
    pub description: String,
    pub command: Option<String>, // 可执行的命令
    pub expected_output: Option<String>,
    pub is_automated: bool,
    pub requires_approval: bool,
    pub timeout_seconds: Option<i32>,
}

// ============= 运行手册执行记录 =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct RunbookExecution {
    pub id: String,
    pub runbook_id: String,
    pub incident_id: String,
    pub executed_by: String,
    pub status: RunbookExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub current_step: i32,
    pub total_steps: i32,
    pub results: Option<serde_json::Value>, // 每步执行结果
    pub output_log: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq)]
#[sqlx(rename = "TEXT")]
pub enum RunbookExecutionStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "paused")]
    Paused,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

// ============= 事件协作 =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct IncidentParticipant {
    pub id: String,
    pub incident_id: String,
    pub user_id: String,
    pub role: ParticipantRole,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub notification_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq)]
#[sqlx(rename = "TEXT")]
pub enum ParticipantRole {
    #[serde(rename = "incident_commander")]
    IncidentCommander, // 事件指挥官
    #[serde(rename = "tech_lead")]
    TechLead, // 技术负责人
    #[serde(rename = "responder")]
    Responder, // 响应人员
    #[serde(rename = "observer")]
    Observer, // 观察员
    #[serde(rename = "communicator")]
    Communicator, // 沟通负责人
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct IncidentCommunication {
    pub id: String,
    pub incident_id: String,
    pub communication_type: CommunicationType,
    pub channel: String, // slack, email, sms, webhook
    pub recipient: String,
    pub content: String,
    pub sent_by: String,
    pub sent_at: DateTime<Utc>,
    pub status: CommunicationStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq)]
#[sqlx(rename = "TEXT")]
pub enum CommunicationType {
    #[serde(rename = "notification")]
    Notification,
    #[serde(rename = "status_update")]
    StatusUpdate,
    #[serde(rename = "escalation")]
    Escalation,
    #[serde(rename = "resolution")]
    Resolution,
    #[serde(rename = "stakeholder_update")]
    StakeholderUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq)]
#[sqlx(rename = "TEXT")]
pub enum CommunicationStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "sent")]
    Sent,
    #[serde(rename = "delivered")]
    Delivered,
    #[serde(rename = "failed")]
    Failed,
}

// ============= 升级策略 =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct EscalationPolicy {
    pub id: String,
    pub name: String,
    pub team_id: String,
    pub is_default: bool,
    pub rules: Option<serde_json::Value>, // JSON array of escalation rules
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EscalationRule {
    pub level: i32,
    pub condition: EscalationCondition,
    pub notify_users: Vec<String>,                // 用户ID列表
    pub notify_channels: Vec<String>,             // 通知渠道
    pub auto_assign: Option<String>,              // 自动分配给
    pub auto_escalate_after_minutes: Option<i32>, // 自动升级时间
    pub require_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EscalationCondition {
    pub condition_type: String, // time_based, severity_based, no_acknowledgment
    pub threshold_minutes: Option<i32>,
    pub severity_levels: Option<Vec<IncidentSeverity>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct EscalationHistory {
    pub id: String,
    pub incident_id: String,
    pub from_level: i32,
    pub to_level: i32,
    pub escalated_by: String, // 用户ID或"auto"
    pub reason: String,
    pub notified_users: Option<serde_json::Value>,
    pub escalated_at: DateTime<Utc>,
}

// ============= 事后复盘 =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PostMortem {
    pub id: String,
    pub incident_id: String,
    pub title: String,
    pub summary: String,
    pub timeline_summary: String,
    pub root_cause_analysis: String,
    pub impact_analysis: String,
    pub resolution_steps: String,
    pub lessons_learned: String,
    pub action_items: Option<serde_json::Value>, // JSON array
    pub contributors: Option<serde_json::Value>, // 参与人员
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: PostMortemStatus,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq)]
#[sqlx(rename = "TEXT")]
pub enum PostMortemStatus {
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "in_review")]
    InReview,
    #[serde(rename = "approved")]
    Approved,
    #[serde(rename = "published")]
    Published,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActionItem {
    pub id: String,
    pub description: String,
    pub assignee: String,
    pub priority: String,
    pub due_date: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

// ============= 外部集成 =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct IntegrationConfig {
    pub id: String,
    pub team_id: String,
    pub provider: IntegrationProvider,
    pub name: String,
    pub config: serde_json::Value, // 提供商特定配置
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_tested_at: Option<DateTime<Utc>>,
    pub last_test_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq)]
#[sqlx(rename = "TEXT")]
pub enum IntegrationProvider {
    #[serde(rename = "pagerduty")]
    PagerDuty,
    #[serde(rename = "opsgenie")]
    OpsGenie,
    #[serde(rename = "slack")]
    Slack,
    #[serde(rename = "teams")]
    Teams,
    #[serde(rename = "webhook")]
    Webhook,
    #[serde(rename = "email")]
    Email,
    #[serde(rename = "sms")]
    Sms,
    #[serde(rename = "discord")]
    Discord,
}

// ============= 影响分析 =============

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ImpactAnalysis {
    pub incident_id: String,
    pub affected_servers: Vec<AffectedServer>,
    pub affected_services: Vec<AffectedService>,
    pub affected_users: Option<AffectedUsers>,
    pub business_impact: BusinessImpact,
    pub estimated_downtime_minutes: Option<i64>,
    pub financial_impact: Option<FinancialImpact>,
    pub analyzed_at: DateTime<Utc>,
    pub analyzed_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AffectedServer {
    pub server_id: String,
    pub server_name: String,
    pub impact_level: ImpactLevel,
    pub services: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AffectedService {
    pub service_name: String,
    pub service_type: String,
    pub status: String,
    pub dependencies: Vec<String>,
    pub impact_level: ImpactLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AffectedUsers {
    pub estimated_count: i64,
    pub user_segments: Vec<String>,
    pub geographic_regions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq)]
#[sqlx(rename = "TEXT")]
pub enum ImpactLevel {
    #[serde(rename = "total")]
    Total, // 完全不可用
    #[serde(rename = "severe")]
    Severe, // 严重受限
    #[serde(rename = "partial")]
    Partial, // 部分受限
    #[serde(rename = "minimal")]
    Minimal, // 轻微影响
    #[serde(rename = "none")]
    None, // 无影响
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BusinessImpact {
    pub severity: String,
    pub description: String,
    pub affected_functions: Vec<String>,
    pub workaround_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FinancialImpact {
    pub estimated_revenue_loss: Option<f64>,
    pub estimated_recovery_cost: Option<f64>,
    pub currency: String,
}

// ============= 事件检测规则 =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DetectionRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rule_type: DetectionRuleType,
    pub team_id: String,
    pub conditions: serde_json::Value, // 检测条件 JSON
    pub severity: IncidentSeverity,
    pub auto_create_incident: bool,
    pub auto_assignee: Option<String>,
    pub notification_channels: Option<serde_json::Value>,
    pub runbook_id: Option<String>,
    pub is_active: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq)]
#[sqlx(rename = "TEXT")]
pub enum DetectionRuleType {
    #[serde(rename = "threshold")]
    Threshold, // 阈值检测
    #[serde(rename = "anomaly")]
    Anomaly, // 异常检测
    #[serde(rename = "pattern")]
    Pattern, // 模式匹配
    #[serde(rename = "composite")]
    Composite, // 复合条件
    #[serde(rename = "ml_based")]
    MlBased, // 机器学习
}

// ============= API请求/响应模型 =============

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateIncidentRequest {
    pub title: String,
    pub description: String,
    pub incident_type: IncidentType,
    pub severity: IncidentSeverity,
    pub team_id: String,
    pub affected_servers: Option<Vec<String>>,
    pub affected_services: Option<Vec<String>>,
    pub assigned_to: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateIncidentRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<IncidentStatus>,
    pub severity: Option<IncidentSeverity>,
    pub assigned_to: Option<String>,
    pub root_cause: Option<String>,
    pub impact_summary: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AcknowledgeIncidentRequest {
    pub user_id: String,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResolveIncidentRequest {
    pub user_id: String,
    pub resolution: String,
    pub root_cause: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddTimelineEntryRequest {
    pub entry_type: TimelineEntryType,
    pub title: String,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAlertRequest {
    pub source: String,
    pub alert_type: String,
    pub severity: IncidentSeverity,
    pub title: String,
    pub description: String,
    pub team_id: String,
    pub server_id: Option<String>,
    pub service_name: Option<String>,
    pub metric_name: Option<String>,
    pub metric_value: Option<f64>,
    pub threshold: Option<f64>,
    pub raw_data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRunbookRequest {
    pub title: String,
    pub description: String,
    pub incident_types: Option<Vec<IncidentType>>,
    pub severity_levels: Option<Vec<IncidentSeverity>>,
    pub team_id: String,
    pub is_global: bool,
    pub content: String,
    pub steps: Option<Vec<RunbookStep>>,
    pub automation_script: Option<String>,
    pub estimated_duration_minutes: Option<i32>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExecuteRunbookRequest {
    pub runbook_id: String,
    pub incident_id: String,
    pub executed_by: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePostMortemRequest {
    pub incident_id: String,
    pub title: String,
    pub summary: String,
    pub root_cause_analysis: String,
    pub lessons_learned: String,
    pub action_items: Option<Vec<ActionItem>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct JoinIncidentRequest {
    pub user_id: String,
    pub role: ParticipantRole,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct EscalateIncidentRequest {
    pub reason: String,
    pub target_level: Option<i32>,
    pub notify_users: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct QueryIncidentsRequest {
    pub team_id: Option<String>,
    pub status: Option<Vec<IncidentStatus>>,
    pub severity: Option<Vec<IncidentSeverity>>,
    pub incident_type: Option<Vec<IncidentType>>,
    pub assigned_to: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub tags: Option<Vec<String>>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IncidentListResponse {
    pub incidents: Vec<Incident>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub stats: IncidentStats,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IncidentStats {
    pub total_count: i64,
    pub active_count: i64,
    pub critical_count: i64,
    pub high_count: i64,
    pub acknowledged_count: i64,
    pub resolved_today: i64,
    pub avg_resolution_time_minutes: Option<f64>,
    pub mttr_last_7_days: Option<f64>, // Mean Time To Resolution
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IncidentDetailResponse {
    pub incident: Incident,
    pub timeline: Vec<IncidentTimelineEntry>,
    pub participants: Vec<IncidentParticipant>,
    pub alerts: Vec<Alert>,
    pub diagnoses: Vec<DiagnosisResult>,
    pub related_incidents: Vec<Incident>,
    pub suggested_runbooks: Vec<Runbook>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AlertAggregationResult {
    pub aggregation_key: String,
    pub fingerprint: String,
    pub alert_count: i32,
    pub first_alert: Alert,
    pub latest_alert: Alert,
    pub severity: IncidentSeverity,
    pub is_flapping: bool,
    pub suggested_action: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IncidentMetrics {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_incidents: i64,
    pub incidents_by_severity: HashMap<String, i64>,
    pub incidents_by_type: HashMap<String, i64>,
    pub incidents_by_status: HashMap<String, i64>,
    pub avg_time_to_acknowledge_minutes: f64,
    pub avg_time_to_resolve_minutes: f64,
    pub top_affected_services: Vec<ServiceMetric>,
    pub alert_storm_count: i64,
    pub false_positive_rate: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceMetric {
    pub service_name: String,
    pub incident_count: i64,
    pub total_downtime_minutes: i64,
}

// ============= 告警指纹生成 =============

pub fn generate_alert_fingerprint(
    alert_type: &str,
    server_id: Option<&str>,
    service_name: Option<&str>,
    metric_name: Option<&str>,
) -> String {
    use sha2::{Digest, Sha256};

    let content = format!(
        "{}:{}:{}:{}",
        alert_type,
        server_id.unwrap_or(""),
        service_name.unwrap_or(""),
        metric_name.unwrap_or("")
    );

    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();

    format!("{:x}", &result[..16]) // 取前16字节
}

// ============= 事件编号生成 =============

pub fn generate_incident_number() -> String {
    let now = Utc::now();
    let date_str = now.format("%Y%m%d").to_string();
    let random_suffix = format!("{:04}", now.timestamp_subsec_millis() % 10000);

    format!("INC-{}-{}", date_str, random_suffix)
}

pub fn generate_alert_number() -> String {
    let now = Utc::now();
    let date_str = now.format("%Y%m%d").to_string();
    let random_suffix = format!("{:04}", now.timestamp_subsec_millis() % 10000);

    format!("ALERT-{}-{}", date_str, random_suffix)
}
