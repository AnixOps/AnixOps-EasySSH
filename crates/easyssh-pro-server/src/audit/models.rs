//! 审计模型定义
//! 包含审计相关的所有数据结构定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    ActionResult, AuditCategory, AuditEventType, ComplianceInfo, DataClassification, GeoLocation,
    Severity,
};

/// 数据库审计记录模型 (PostgreSQL)
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct AuditRecordModel {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub category: String,
    pub severity: String,
    pub user_id: String,
    pub user_name: String,
    pub team_id: Option<String>,
    pub ip_address: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub action: String,
    pub result: String,
    pub details: Option<serde_json::Value>,
    pub session_id: Option<String>,
    pub user_agent: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub retention_policy_id: Option<String>,
    pub frameworks: Option<serde_json::Value>,
    pub data_classification: String,
    pub encryption_required: bool,
    pub integrity_verified: bool,
    pub chain_hash: Option<String>,
    pub signature: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl AuditRecordModel {
    pub fn from_audit_record(record: &super::AuditRecord) -> Self {
        Self {
            id: record.id.clone(),
            timestamp: record.timestamp,
            event_type: format!("{:?}", record.event_type),
            category: format!("{:?}", record.category),
            severity: format!("{:?}", record.severity),
            user_id: record.user_id.clone(),
            user_name: record.user_name.clone(),
            team_id: record.team_id.clone(),
            ip_address: record.ip_address.clone(),
            resource_type: record.resource_type.clone(),
            resource_id: record.resource_id.clone(),
            action: record.action.clone(),
            result: format!("{:?}", record.result),
            details: record.details.clone(),
            session_id: record.session_id.clone(),
            user_agent: record.user_agent.clone(),
            country: record.location.as_ref().map(|l| l.country.clone()).flatten(),
            city: record.location.as_ref().map(|l| l.city.clone()).flatten(),
            latitude: record.location.as_ref().and_then(|l| l.latitude),
            longitude: record.location.as_ref().and_then(|l| l.longitude),
            retention_policy_id: record.compliance.retention_policy_id.clone(),
            frameworks: Some(serde_json::to_value(&record.compliance.frameworks).unwrap_or_default()),
            data_classification: format!("{:?}", record.compliance.data_classification),
            encryption_required: record.compliance.encryption_required,
            integrity_verified: record.compliance.integrity_verified,
            chain_hash: record.chain_hash.clone(),
            signature: record.signature.clone(),
            created_at: Utc::now(),
        }
    }
}

/// ClickHouse审计记录模型 (列式存储优化)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickHouseAuditRecord {
    pub id: String,
    pub timestamp: i64,
    pub event_type: String,
    pub category: String,
    pub severity: u8,
    pub user_id: String,
    pub team_id: Option<String>,
    pub ip_address: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub action: String,
    pub result: u8,
    pub details: String,
    pub session_id: Option<String>,
    pub user_agent: Option<String>,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
}

impl ClickHouseAuditRecord {
    pub fn from_audit_record(record: &super::AuditRecord) -> Self {
        let timestamp = record.timestamp.timestamp_millis();
        let year = record.timestamp.year() as u16;
        let month = record.timestamp.month() as u8;
        let day = record.timestamp.day() as u8;
        let hour = record.timestamp.hour() as u8;

        Self {
            id: record.id.clone(),
            timestamp,
            event_type: format!("{:?}", record.event_type),
            category: format!("{:?}", record.category),
            severity: record.severity as u8,
            user_id: record.user_id.clone(),
            team_id: record.team_id.clone(),
            ip_address: record.ip_address.clone(),
            resource_type: record.resource_type.clone(),
            resource_id: record.resource_id.clone(),
            action: record.action.clone(),
            result: record.result as u8,
            details: record
                .details
                .as_ref()
                .map(|d| d.to_string())
                .unwrap_or_default(),
            session_id: record.session_id.clone(),
            user_agent: record.user_agent.clone(),
            year,
            month,
            day,
            hour,
        }
    }
}

/// S3归档记录格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3ArchiveRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub compressed_data: Vec<u8>,
    pub encryption_iv: Option<Vec<u8>>,
    pub original_size: u64,
    pub compression_ratio: f64,
}

/// 审计过滤条件
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditFilter {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub event_types: Option<Vec<AuditEventType>>,
    pub categories: Option<Vec<AuditCategory>>,
    pub severities: Option<Vec<Severity>>,
    pub user_ids: Option<Vec<String>>,
    pub team_id: Option<String>,
    pub resource_types: Option<Vec<String>>,
    pub resource_id: Option<String>,
    pub result: Option<ActionResult>,
    pub ip_addresses: Option<Vec<String>>,
    pub session_id: Option<String>,
    pub has_details: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 审计排序选项
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    Timestamp,
    EventType,
    Severity,
    UserId,
    ResourceType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// 审计查询选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOptions {
    pub filter: AuditFilter,
    pub sort_field: SortField,
    pub sort_order: SortOrder,
    pub include_details: bool,
    pub include_location: bool,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            filter: AuditFilter::default(),
            sort_field: SortField::Timestamp,
            sort_order: SortOrder::Desc,
            include_details: true,
            include_location: true,
        }
    }
}

/// 审计分页结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedAuditResult {
    pub records: Vec<super::AuditRecord>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub has_more: bool,
}

/// 时间聚合查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeAggregatedResult {
    pub bucket_start: DateTime<Utc>,
    pub bucket_end: DateTime<Utc>,
    pub count: i64,
    pub events_by_type: std::collections::HashMap<String, i64>,
    pub unique_users: i64,
}

/// 用户活动摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivitySummary {
    pub user_id: String,
    pub user_name: String,
    pub total_actions: i64,
    pub unique_sessions: i64,
    pub unique_ips: i64,
    pub first_activity: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub most_active_hour: u8,
    pub risk_score: f64,
}

/// IP活动摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpActivitySummary {
    pub ip_address: String,
    pub total_requests: i64,
    pub unique_users: Vec<String>,
    pub unique_teams: Vec<String>,
    pub failed_attempts: i64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub risk_score: f64,
    pub is_blocked: bool,
}

/// 审计导出任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportTask {
    pub task_id: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub filter: AuditFilter,
    pub format: super::ExportFormat,
    pub status: ExportStatus,
    pub progress: f64,
    pub file_path: Option<String>,
    pub file_size: Option<u64>,
    pub error_message: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "export_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Expired,
}

impl ExportTask {
    pub fn new(created_by: impl Into<String>, filter: AuditFilter, format: super::ExportFormat) -> Self {
        Self {
            task_id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            created_by: created_by.into(),
            filter,
            format,
            status: ExportStatus::Pending,
            progress: 0.0,
            file_path: None,
            file_size: None,
            error_message: None,
            completed_at: None,
            expires_at: None,
        }
    }
}

/// 告警规则模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRuleModel {
    pub rule_id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub event_types: Vec<String>,
    pub severity_threshold: Severity,
    pub conditions: Vec<AlertCondition>,
    pub actions: Vec<AlertAction>,
    pub aggregation_window_secs: u64,
    pub suppression_minutes: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
}

/// 告警条件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlertCondition {
    Threshold { field: String, operator: String, value: f64 },
    Pattern { pattern: String, matches: bool },
    Frequency { count: u32, window_secs: u64 },
    Anomaly { baseline: f64, deviation: f64 },
}

/// 告警动作
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlertAction {
    Webhook { url: String, headers: Option<serde_json::Value> },
    Email { to: Vec<String>, template: Option<String> },
    Slack { channel: String, webhook_url: String },
    Sms { phone_numbers: Vec<String> },
    BlockIp { duration_mins: i32 },
    DisableAccount { duration_mins: i32 },
}

/// 告警事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    pub alert_id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: Severity,
    pub triggered_at: DateTime<Utc>,
    pub description: String,
    pub affected_user: Option<String>,
    pub affected_ip: Option<String>,
    pub related_events: Vec<String>,
    pub status: AlertStatus,
    pub acknowledged_by: Option<String>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "alert_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AlertStatus {
    Triggered,
    Acknowledged,
    Resolved,
    Suppressed,
}

/// 归档任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveTask {
    pub task_id: String,
    pub created_at: DateTime<Utc>,
    pub date_range_start: DateTime<Utc>,
    pub date_range_end: DateTime<Utc>,
    pub status: ArchiveTaskStatus,
    pub source_records: i64,
    pub archived_records: i64,
    pub failed_records: i64,
    pub s3_location: Option<String>,
    pub compression_ratio: f64,
    pub error_message: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "archive_task_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ArchiveTaskStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// 审计保留策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub policy_id: String,
    pub name: String,
    pub description: String,
    pub category: AuditCategory,
    pub severity: Option<Severity>,
    pub retention_days: i32,
    pub archive_before_delete: bool,
    pub require_approval: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 审计数据库初始化SQL
pub const AUDIT_DB_SCHEMA: &str = r#"
-- 审计记录主表
CREATE TABLE IF NOT EXISTS audit_records (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    category VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    user_id UUID NOT NULL,
    user_name VARCHAR(255) NOT NULL,
    team_id UUID,
    ip_address INET NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    resource_id UUID,
    action VARCHAR(255) NOT NULL,
    result VARCHAR(20) NOT NULL,
    details JSONB,
    session_id UUID,
    user_agent TEXT,
    country VARCHAR(100),
    city VARCHAR(100),
    latitude DECIMAL(10, 8),
    longitude DECIMAL(11, 8),
    retention_policy_id UUID,
    frameworks JSONB,
    data_classification VARCHAR(50) DEFAULT 'internal',
    encryption_required BOOLEAN DEFAULT FALSE,
    integrity_verified BOOLEAN DEFAULT FALSE,
    chain_hash VARCHAR(128),
    signature VARCHAR(256),
    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT fk_team FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE SET NULL,
    CONSTRAINT fk_retention_policy FOREIGN KEY (retention_policy_id) REFERENCES retention_policies(policy_id)
);

-- 审计记录分区表 (按时间分区)
CREATE TABLE IF NOT EXISTS audit_records_partitioned (
    LIKE audit_records INCLUDING ALL
) PARTITION BY RANGE (timestamp);

-- 创建默认分区
CREATE TABLE IF NOT EXISTS audit_records_default PARTITION OF audit_records_partitioned
    DEFAULT;

-- 索引
CREATE INDEX IF NOT EXISTS idx_audit_records_timestamp ON audit_records(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_records_user_id ON audit_records(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_records_team_id ON audit_records(team_id);
CREATE INDEX IF NOT EXISTS idx_audit_records_event_type ON audit_records(event_type);
CREATE INDEX IF NOT EXISTS idx_audit_records_category ON audit_records(category);
CREATE INDEX IF NOT EXISTS idx_audit_records_severity ON audit_records(severity);
CREATE INDEX IF NOT EXISTS idx_audit_records_ip ON audit_records(ip_address);
CREATE INDEX IF NOT EXISTS idx_audit_records_session ON audit_records(session_id);
CREATE INDEX IF NOT EXISTS idx_audit_records_resource ON audit_records(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_records_composite ON audit_records(team_id, timestamp DESC, event_type);

-- 复合索引优化查询
CREATE INDEX IF NOT EXISTS idx_audit_records_user_time ON audit_records(user_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_records_team_user ON audit_records(team_id, user_id, timestamp DESC);

-- 导出任务表
CREATE TABLE IF NOT EXISTS export_tasks (
    task_id UUID PRIMARY KEY,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    created_by UUID NOT NULL,
    filter JSONB NOT NULL,
    format VARCHAR(20) NOT NULL,
    status VARCHAR(20) DEFAULT 'pending',
    progress DECIMAL(5, 2) DEFAULT 0,
    file_path TEXT,
    file_size BIGINT,
    error_message TEXT,
    completed_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,

    CONSTRAINT fk_created_by FOREIGN KEY (created_by) REFERENCES users(id)
);

-- 告警规则表
CREATE TABLE IF NOT EXISTS alert_rules (
    rule_id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    enabled BOOLEAN DEFAULT TRUE,
    event_types JSONB,
    severity_threshold VARCHAR(20),
    conditions JSONB NOT NULL,
    actions JSONB NOT NULL,
    aggregation_window_secs INTEGER DEFAULT 300,
    suppression_minutes INTEGER DEFAULT 60,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    created_by UUID NOT NULL,

    CONSTRAINT fk_created_by FOREIGN KEY (created_by) REFERENCES users(id)
);

-- 告警事件表
CREATE TABLE IF NOT EXISTS alert_events (
    alert_id UUID PRIMARY KEY,
    rule_id UUID NOT NULL,
    rule_name VARCHAR(255) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    triggered_at TIMESTAMPTZ DEFAULT NOW(),
    description TEXT NOT NULL,
    affected_user UUID,
    affected_ip INET,
    related_events JSONB,
    status VARCHAR(20) DEFAULT 'triggered',
    acknowledged_by UUID,
    acknowledged_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    resolution TEXT,

    CONSTRAINT fk_rule FOREIGN KEY (rule_id) REFERENCES alert_rules(rule_id),
    CONSTRAINT fk_affected_user FOREIGN KEY (affected_user) REFERENCES users(id),
    CONSTRAINT fk_acknowledged_by FOREIGN KEY (acknowledged_by) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_alert_events_triggered_at ON alert_events(triggered_at DESC);
CREATE INDEX IF NOT EXISTS idx_alert_events_status ON alert_events(status);
CREATE INDEX IF NOT EXISTS idx_alert_events_affected_user ON alert_events(affected_user);

-- 归档任务表
CREATE TABLE IF NOT EXISTS archive_tasks (
    task_id UUID PRIMARY KEY,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    date_range_start TIMESTAMPTZ NOT NULL,
    date_range_end TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) DEFAULT 'pending',
    source_records BIGINT DEFAULT 0,
    archived_records BIGINT DEFAULT 0,
    failed_records BIGINT DEFAULT 0,
    s3_location TEXT,
    compression_ratio DECIMAL(5, 2),
    error_message TEXT,
    completed_at TIMESTAMPTZ
);

-- 保留策略表
CREATE TABLE IF NOT EXISTS retention_policies (
    policy_id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    category VARCHAR(50) NOT NULL,
    severity VARCHAR(20),
    retention_days INTEGER NOT NULL,
    archive_before_delete BOOLEAN DEFAULT TRUE,
    require_approval BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 审计完整性验证记录
CREATE TABLE IF NOT EXISTS audit_integrity_checks (
    check_id UUID PRIMARY KEY,
    checked_at TIMESTAMPTZ DEFAULT NOW(),
    check_type VARCHAR(50) NOT NULL,
    records_checked INTEGER NOT NULL,
    valid_records INTEGER NOT NULL,
    tampered_records INTEGER DEFAULT 0,
    broken_chain_count INTEGER DEFAULT 0,
    integrity_score DECIMAL(5, 2),
    details JSONB,
    performed_by UUID,

    CONSTRAINT fk_performed_by FOREIGN KEY (performed_by) REFERENCES users(id)
);

-- 创建分区维护函数
CREATE OR REPLACE FUNCTION create_audit_partition(
    start_date DATE,
    end_date DATE
) RETURNS TEXT AS $$
DECLARE
    partition_name TEXT;
    start_ts TIMESTAMPTZ;
    end_ts TIMESTAMPTZ;
BEGIN
    partition_name := 'audit_records_' || TO_CHAR(start_date, 'YYYY_MM');
    start_ts := start_date::TIMESTAMPTZ;
    end_ts := end_date::TIMESTAMPTZ;

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF audit_records_partitioned
         FOR VALUES FROM (%L) TO (%L)',
        partition_name, start_ts, end_ts
    );

    RETURN partition_name;
END;
$$ LANGUAGE plpgsql;

-- 插入默认保留策略
INSERT INTO retention_policies (policy_id, name, description, category, retention_days) VALUES
    (gen_random_uuid(), 'Authentication Events', 'Login/logout events', 'Authentication', 365),
    (gen_random_uuid(), 'Server Operations', 'Server CRUD and connection events', 'Server', 730),
    (gen_random_uuid(), 'Security Events', 'Security critical events', 'Security', 2555),
    (gen_random_uuid(), 'Compliance Events', 'Compliance related events', 'Compliance', 2555),
    (gen_random_uuid(), 'Session Events', 'Session and command events', 'Session', 180),
    (gen_random_uuid(), 'Team Events', 'Team and member events', 'Team', 365)
ON CONFLICT DO NOTHING;
"#;

/// ClickHouse表结构
pub const CLICKHOUSE_SCHEMA: &str = r#"
-- 审计记录表 (MergeTree引擎用于时间序列数据)
CREATE TABLE IF NOT EXISTS audit_records (
    id UUID,
    timestamp Int64,
    event_type LowCardinality(String),
    category LowCardinality(String),
    severity UInt8,
    user_id UUID,
    team_id Nullable(UUID),
    ip_address IPv4,
    resource_type LowCardinality(String),
    resource_id Nullable(UUID),
    action String,
    result UInt8,
    details String,
    session_id Nullable(UUID),
    user_agent Nullable(String),
    year UInt16,
    month UInt8,
    day UInt8,
    hour UInt8
) ENGINE = MergeTree()
PARTITION BY (year, month)
ORDER BY (timestamp, user_id, event_type)
TTL timestamp + INTERVAL 2 YEAR;

-- 物化视图: 用户活动统计
CREATE MATERIALIZED VIEW IF NOT EXISTS user_activity_stats
ENGINE = SummingMergeTree()
PARTITION BY (year, month)
ORDER BY (user_id, year, month, day)
AS SELECT
    user_id,
    toYear(toDateTime(timestamp / 1000)) as year,
    toMonth(toDateTime(timestamp / 1000)) as month,
    toDayOfMonth(toDateTime(timestamp / 1000)) as day,
    count() as event_count,
    uniqExact(ip_address) as unique_ips,
    uniqExact(session_id) as unique_sessions
FROM audit_records
GROUP BY user_id, year, month, day;

-- 物化视图: 事件类型统计
CREATE MATERIALIZED VIEW IF NOT EXISTS event_type_stats
ENGINE = SummingMergeTree()
PARTITION BY (year, month)
ORDER BY (event_type, year, month, day)
AS SELECT
    event_type,
    toYear(toDateTime(timestamp / 1000)) as year,
    toMonth(toDateTime(timestamp / 1000)) as month,
    toDayOfMonth(toDateTime(timestamp / 1000)) as day,
    count() as event_count,
    uniqExact(user_id) as unique_users
FROM audit_records
GROUP BY event_type, year, month, day;

-- 物化视图: IP活动统计
CREATE MATERIALIZED VIEW IF NOT EXISTS ip_activity_stats
ENGINE = SummingMergeTree()
PARTITION BY (year, month)
ORDER BY (ip_address, year, month)
AS SELECT
    ip_address,
    toYear(toDateTime(timestamp / 1000)) as year,
    toMonth(toDateTime(timestamp / 1000)) as month,
    count() as request_count,
    uniqExact(user_id) as unique_users
FROM audit_records
GROUP BY ip_address, year, month;
"#;
