//! 审计导出模块
//! 支持导出到JSON、CSV和SIEM格式

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

use super::{
    models::{AuditFilter, ExportStatus, ExportTask},
    storage::AuditStorage,
    AuditConfig, AuditRecord, AuditResult, AuditSummary,
};

/// 导出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Json,
    Csv,
    CEF,   // Common Event Format (ArcSight)
    LEEF,  // Log Event Extended Format (IBM QRadar)
    Syslog,
    Parquet,
}

impl ExportFormat {
    /// 获取文件扩展名
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::CEF => "cef",
            ExportFormat::LEEF => "leef",
            ExportFormat::Syslog => "log",
            ExportFormat::Parquet => "parquet",
        }
    }

    /// 获取MIME类型
    pub fn mime_type(&self) -> &'static str {
        match self {
            ExportFormat::Json => "application/json",
            ExportFormat::Csv => "text/csv",
            ExportFormat::CEF => "text/plain",
            ExportFormat::LEEF => "text/plain",
            ExportFormat::Syslog => "text/plain",
            ExportFormat::Parquet => "application/octet-stream",
        }
    }
}

/// SIEM配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SIEMConfig {
    pub enabled: bool,
    pub siem_type: SIEMType,
    pub host: String,
    pub port: u16,
    pub protocol: SIEMProtocol,
    pub api_key: Option<String>,
    pub tls_enabled: bool,
    pub batch_size: usize,
    pub flush_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SIEMType {
    Splunk,
    QRadar,
    ArcSight,
    Elastic,
    Datadog,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SIEMProtocol {
    Tcp,
    Udp,
    Https,
    Syslog,
}

/// 审计导出器
pub struct AuditExporter {
    storage: std::sync::Arc<dyn AuditStorage>,
    config: AuditConfig,
    export_dir: PathBuf,
    siem_config: Option<SIEMConfig>,
}

impl AuditExporter {
    /// 创建新的导出器
    pub fn new(
        storage: std::sync::Arc<dyn AuditStorage>,
        config: AuditConfig,
        export_dir: impl Into<PathBuf>,
        siem_config: Option<SIEMConfig>,
    ) -> Self {
        Self {
            storage,
            config,
            export_dir: export_dir.into(),
            siem_config,
        }
    }

    /// 创建导出任务
    pub async fn create_export_task(
        &self,
        created_by: impl Into<String>,
        filter: AuditFilter,
        format: ExportFormat,
    ) -> AuditResult<ExportTask> {
        let task = ExportTask::new(created_by, filter, format);

        // 保存任务到数据库
        // ...

        Ok(task)
    }

    /// 执行导出任务
    pub async fn execute_export(&self, task: &mut ExportTask) -> AuditResult<PathBuf> {
        task.status = ExportStatus::Processing;

        // 查询记录
        let records = self.storage.query(&task.filter).await?;
        let total = records.len() as f64;

        // 生成导出文件
        let filename = format!(
            "audit_export_{}_{}",
            task.task_id,
            Utc::now().format("%Y%m%d_%H%M%S")
        );
        let filepath = self.export_dir.join(format!("{}.{}.{}", filename, task.format.extension(), self.get_compression_ext()));

        // 确保目录存在
        tokio::fs::create_dir_all(&self.export_dir).await.map_err(super::AuditError::Io)?;

        // 根据格式导出
        match task.format {
            ExportFormat::Json => self.export_json(&records, &filepath).await?,
            ExportFormat::Csv => self.export_csv(&records, &filepath).await?,
            ExportFormat::CEF => self.export_cef(&records, &filepath).await?,
            ExportFormat::LEEF => self.export_leef(&records, &filepath).await?,
            ExportFormat::Syslog => self.export_syslog(&records, &filepath).await?,
            ExportFormat::Parquet => self.export_parquet(&records, &filepath).await?,
        }

        task.progress = 100.0;
        task.status = ExportStatus::Completed;
        task.completed_at = Some(Utc::now());
        task.file_path = Some(filepath.to_string_lossy().to_string());

        // 获取文件大小
        if let Ok(metadata) = tokio::fs::metadata(&filepath).await {
            task.file_size = Some(metadata.len());
        }

        // 设置过期时间 (7天后)
        task.expires_at = Some(Utc::now() + chrono::Duration::days(7));

        Ok(filepath)
    }

    /// 导出为JSON
    async fn export_json(
        &self,
        records: &[AuditRecord],
        filepath: &PathBuf,
    ) -> AuditResult<()> {
        use std::io::Write;

        let json = serde_json::to_vec_pretty(records)
            .map_err(super::AuditError::Serialization)?;

        let compressed = self.compress(&json)?;

        let mut file = std::fs::File::create(filepath)
            .map_err(super::AuditError::Io)?;
        file.write_all(&compressed)
            .map_err(super::AuditError::Io)?;

        Ok(())
    }

    /// 导出为CSV
    async fn export_csv(
        &self,
        records: &[AuditRecord],
        filepath: &PathBuf,
    ) -> AuditResult<()> {
        use std::io::Write;

        let mut csv_content = String::new();

        // 写入CSV头部
        csv_content.push_str("timestamp,id,event_type,category,severity,user_id,user_name,team_id,ip_address,resource_type,resource_id,action,result,session_id,country,city,details\n");

        // 写入记录
        for record in records {
            let details = record
                .details
                .as_ref()
                .map(|d| d.to_string())
                .unwrap_or_default()
                .replace('"', "\"\"");

            let line = format!(
                "{},{},{:?},{:?},{:?},{},{},{},{},{},{},{},{:?},{},{},{}\"{}\"\n",
                record.timestamp.to_rfc3339(),
                record.id,
                record.event_type,
                record.category,
                record.severity,
                record.user_id,
                record.user_name,
                record.team_id.as_deref().unwrap_or(""),
                record.ip_address,
                record.resource_type,
                record.resource_id.as_deref().unwrap_or(""),
                record.action,
                record.result,
                record.session_id.as_deref().unwrap_or(""),
                record.location.as_ref().and_then(|l| l.country.as_ref()).unwrap_or(""),
                record.location.as_ref().and_then(|l| l.city.as_ref()).unwrap_or(""),
                details
            );
            csv_content.push_str(&line);
        }

        let compressed = self.compress(csv_content.as_bytes())?;

        let mut file = std::fs::File::create(filepath)
            .map_err(super::AuditError::Io)?;
        file.write_all(&compressed)
            .map_err(super::AuditError::Io)?;

        Ok(())
    }

    /// 导出为CEF格式 (Common Event Format)
    async fn export_cef(
        &self,
        records: &[AuditRecord],
        filepath: &PathBuf,
    ) -> AuditResult<()> {
        use std::io::Write;

        let mut content = String::new();

        for record in records {
            // CEF:Version|Device Vendor|Device Product|Device Version|Signature ID|Name|Severity|Extensions
            let severity = match record.severity {
                super::Severity::Low => "1",
                super::Severity::Medium => "5",
                super::Severity::High => "8",
                super::Severity::Critical => "10",
            };

            let line = format!(
                "CEF:0|EasySSH|Pro|{}|{}|{}|{}|{}\n",
                self.config.postgres_url, // 使用版本信息
                format!("{:?}", record.event_type),
                record.action,
                severity,
                self.build_cef_extensions(record)
            );
            content.push_str(&line);
        }

        let compressed = self.compress(content.as_bytes())?;

        let mut file = std::fs::File::create(filepath)
            .map_err(super::AuditError::Io)?;
        file.write_all(&compressed)
            .map_err(super::AuditError::Io)?;

        Ok(())
    }

    /// 导出为LEEF格式 (Log Event Extended Format)
    async fn export_leef(
        &self,
        records: &[AuditRecord],
        filepath: &PathBuf,
    ) -> AuditResult<()> {
        use std::io::Write;

        let mut content = String::new();

        for record in records {
            // LEEF:Version|Vendor|Product|Version|EventID|devTime|usrName|src|...\n            let line = format!(
                "LEEF:2.0|EasySSH|Pro|1.0|{}|devTime={}|usrName={}|src={}|eventType={}|cat={}|sev={}\n",
                format!("{:?}", record.event_type),
                record.timestamp.to_rfc3339(),
                record.user_name,
                record.ip_address,
                format!("{:?}", record.event_type),
                format!("{:?}", record.category),
                format!("{:?}", record.severity),
            );
            content.push_str(&line);
        }

        let compressed = self.compress(content.as_bytes())?;

        let mut file = std::fs::File::create(filepath)
            .map_err(super::AuditError::Io)?;
        file.write_all(&compressed)
            .map_err(super::AuditError::Io)?;

        Ok(())
    }

    /// 导出为Syslog格式
    async fn export_syslog(
        &self,
        records: &[AuditRecord],
        filepath: &PathBuf,
    ) -> AuditResult<()> {
        use std::io::Write;

        let mut content = String::new();

        for record in records {
            // RFC 5424 Syslog格式
            let severity = match record.severity {
                super::Severity::Low => 6,    // Informational
                super::Severity::Medium => 4, // Warning
                super::Severity::High => 3,    // Error
                super::Severity::Critical => 2, // Critical
            };

            let line = format!(
                "<{}>{} {} {} - {}: {} user={} action={} resource={} result={}\n",
                (16 * 8 + severity), // facility 16 (local0) * 8 + severity
                record.timestamp.to_rfc3339(),
                "easyssh-pro",
                format!("{:?}", record.event_type),
                record.id,
                record.action,
                record.user_name,
                format!("{:?}", record.event_type),
                record.resource_type,
                format!("{:?}", record.result),
            );
            content.push_str(&line);
        }

        let compressed = self.compress(content.as_bytes())?;

        let mut file = std::fs::File::create(filepath)
            .map_err(super::AuditError::Io)?;
        file.write_all(&compressed)
            .map_err(super::AuditError::Io)?;

        Ok(())
    }

    /// 导出为Parquet格式 (用于大数据分析)
    async fn export_parquet(
        &self,
        _records: &[AuditRecord],
        _filepath: &PathBuf,
    ) -> AuditResult<()> {
        // Parquet导出需要arrow2或parquet crate
        // 这里提供框架，实际实现需要添加依赖
        Err(super::AuditError::Export(
            "Parquet export requires additional dependencies".to_string(),
        ))
    }

    /// 发送记录到SIEM
    pub async fn send_to_siem(&self, records: &[AuditRecord]) -> AuditResult<usize> {
        if let Some(ref config) = self.siem_config {
            if !config.enabled {
                return Ok(0);
            }

            let sent_count = match config.siem_type {
                SIEMType::Splunk => self.send_to_splunk(records, config).await?,
                SIEMType::QRadar => self.send_to_qradar(records, config).await?,
                SIEMType::ArcSight => self.send_to_arcsight(records, config).await?,
                SIEMType::Elastic => self.send_to_elastic(records, config).await?,
                SIEMType::Datadog => self.send_to_datadog(records, config).await?,
                SIEMType::Custom => self.send_to_custom(records, config).await?,
            };

            Ok(sent_count)
        } else {
            Ok(0)
        }
    }

    /// 发送到Splunk
    async fn send_to_splunk(
        &self,
        records: &[AuditRecord],
        config: &SIEMConfig,
    ) -> AuditResult<usize> {
        let client = reqwest::Client::new();
        let url = format!("{}:{}/services/collector/event", config.host, config.port);

        let mut sent = 0;

        for record in records {
            let payload = serde_json::json!({
                "time": record.timestamp.timestamp(),
                "event": record,
                "sourcetype": "easyssh:audit",
                "index": "security",
            });

            let response = client
                .post(&url)
                .header("Authorization", format!("Splunk {}", config.api_key.as_deref().unwrap_or("")))
                .json(&payload)
                .send()
                .await
                .map_err(|e| super::AuditError::Export(format!("Splunk send failed: {}", e)))?;

            if response.status().is_success() {
                sent += 1;
            }
        }

        Ok(sent)
    }

    /// 发送到QRadar
    async fn send_to_qradar(
        &self,
        _records: &[AuditRecord],
        _config: &SIEMConfig,
    ) -> AuditResult<usize> {
        // QRadar DSM集成实现
        Ok(0)
    }

    /// 发送到ArcSight
    async fn send_to_arcsight(
        &self,
        _records: &[AuditRecord],
        _config: &SIEMConfig,
    ) -> AuditResult<usize> {
        // ArcSight CEF集成实现
        Ok(0)
    }

    /// 发送到Elasticsearch
    async fn send_to_elastic(
        &self,
        records: &[AuditRecord],
        config: &SIEMConfig,
    ) -> AuditResult<usize> {
        let client = reqwest::Client::new();
        let url = format!("{}:{}/_bulk", config.host, config.port);

        let mut bulk_body = String::new();

        for record in records {
            let index_line = format!(
                r#"{{"index": {{"_index": "easyssh-audit-{}", "_id": "{}"}}}}"#,
                record.timestamp.format("%Y.%m.%d"),
                record.id
            );
            let doc = serde_json::to_string(record)
                .map_err(super::AuditError::Serialization)?;

            bulk_body.push_str(&index_line);
            bulk_body.push('\n');
            bulk_body.push_str(&doc);
            bulk_body.push('\n');
        }

        let response = client
            .post(&url)
            .header("Content-Type", "application/x-ndjson")
            .body(bulk_body)
            .send()
            .await
            .map_err(|e| super::AuditError::Export(format!("Elastic send failed: {}", e)))?;

        if response.status().is_success() {
            Ok(records.len())
        } else {
            Err(super::AuditError::Export(format!(
                "Elastic bulk insert failed: {}",
                response.status()
            )))
        }
    }

    /// 发送到Datadog
    async fn send_to_datadog(
        &self,
        _records: &[AuditRecord],
        _config: &SIEMConfig,
    ) -> AuditResult<usize> {
        // Datadog Logs API集成实现
        Ok(0)
    }

    /// 发送到自定义SIEM
    async fn send_to_custom(
        &self,
        _records: &[AuditRecord],
        _config: &SIEMConfig,
    ) -> AuditResult<usize> {
        // 自定义SIEM集成实现
        Ok(0)
    }

    /// 生成合规报告
    pub async fn generate_compliance_report(
        &self,
        framework: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> AuditResult<super::AuditReport> {
        let filter = AuditFilter {
            start_time: Some(start_time),
            end_time: Some(end_time),
            ..Default::default()
        };

        let records = self.storage.query(&filter).await?;

        let summary = self.generate_summary(&records);

        // 分析合规性
        let compliance_status = self.analyze_compliance(framework, &records);

        // 识别顶级事件
        let mut event_counts: std::collections::HashMap<super::AuditEventType, i64> = std::collections::HashMap::new();
        for record in &records {
            *event_counts.entry(record.event_type).or_insert(0) += 1;
        }
        let mut top_events: Vec<super::EventSummary> = event_counts
            .into_iter()
            .map(|(event_type, count)| super::EventSummary {
                event_type,
                count,
                trend: 0.0,
            })
            .collect();
        top_events.sort_by(|a, b| b.count.cmp(&a.count));
        top_events.truncate(10);

        // 识别顶级用户
        let mut user_counts: std::collections::HashMap<String, (String, i64, DateTime<Utc>)> = std::collections::HashMap::new();
        for record in &records {
            let entry = user_counts
                .entry(record.user_id.clone())
                .or_insert((record.user_name.clone(), 0, record.timestamp));
            entry.1 += 1;
            if record.timestamp > entry.2 {
                entry.2 = record.timestamp;
            }
        }
        let mut top_users: Vec<super::UserSummary> = user_counts
            .into_iter()
            .map(|(user_id, (user_name, count, last_activity))| super::UserSummary {
                user_id,
                user_name,
                action_count: count,
                last_activity,
            })
            .collect();
        top_users.sort_by(|a, b| b.action_count.cmp(&a.action_count));
        top_users.truncate(10);

        // 检测异常
        let anomalies = self.detect_anomalies(&records);

        Ok(super::AuditReport {
            report_id: uuid::Uuid::new_v4().to_string(),
            generated_at: Utc::now(),
            generated_by: "system".to_string(),
            period_start: start_time,
            period_end: end_time,
            summary,
            top_events,
            top_users,
            anomalies,
            compliance_status,
        })
    }

    /// 私有辅助方法

    fn compress(&self, data: &[u8]) -> AuditResult<Vec<u8>> {
        use std::io::Write;

        let mut encoder = zstd::Encoder::new(Vec::new(), 6)
            .map_err(|e| super::AuditError::Export(format!("Compression failed: {}", e)))?;

        encoder
            .write_all(data)
            .map_err(|e| super::AuditError::Export(format!("Compression write failed: {}", e)))?;

        encoder
            .finish()
            .map_err(|e| super::AuditError::Export(format!("Compression finish failed: {}", e)))
    }

    fn get_compression_ext(&self) -> &'static str {
        "zst"
    }

    fn build_cef_extensions(&self, record: &AuditRecord) -> String {
        let mut extensions = format!(
            "rt={} src={} duser={} suser={} cs1={} cs1Label=action cs2={} cs2Label=result",
            record.timestamp.to_rfc3339(),
            record.ip_address,
            record.user_id,
            record.user_name,
            record.action,
            format!("{:?}", record.result),
        );

        if let Some(ref team_id) = record.team_id {
            extensions.push_str(&format!(" cs3={} cs3Label=teamId", team_id));
        }

        if let Some(ref resource_id) = record.resource_id {
            extensions.push_str(&format!(" cs4={} cs4Label=resourceId", resource_id));
        }

        extensions
    }

    fn generate_summary(&self, records: &[AuditRecord]) -> AuditSummary {
        let total = records.len() as i64;
        let mut by_category: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        let mut by_severity: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        let mut failed_logins = 0i64;
        let mut unique_users = std::collections::HashSet::new();
        let mut unique_ips = std::collections::HashSet::new();
        let mut first_time: Option<DateTime<Utc>> = None;
        let mut last_time: Option<DateTime<Utc>> = None;

        for record in records {
            let category = format!("{:?}", record.category);
            *by_category.entry(category).or_insert(0) += 1;

            let severity = format!("{:?}", record.severity);
            *by_severity.entry(severity).or_insert(0) += 1;

            if record.event_type == super::AuditEventType::LoginFailure {
                failed_logins += 1;
            }

            unique_users.insert(record.user_id.clone());
            unique_ips.insert(record.ip_address.clone());

            if first_time.is_none() || record.timestamp < first_time.unwrap() {
                first_time = Some(record.timestamp);
            }
            if last_time.is_none() || record.timestamp > last_time.unwrap() {
                last_time = Some(record.timestamp);
            }
        }

        AuditSummary {
            total_records: total,
            records_by_category: by_category,
            records_by_severity: by_severity,
            failed_logins,
            unique_users: unique_users.len() as i64,
            unique_ips: unique_ips.len() as i64,
            time_range_start: first_time.unwrap_or_else(Utc::now),
            time_range_end: last_time.unwrap_or_else(Utc::now),
        }
    }

    fn analyze_compliance(
        &self,
        framework: &str,
        records: &[AuditRecord],
    ) -> super::ComplianceStatus {
        let mut violations = Vec::new();

        // SOC2 控制点检查
        if framework == "SOC2" || framework == "all" {
            // CC6.1: 逻辑和物理访问控制
            let permission_denied_count = records
                .iter()
                .filter(|r| r.event_type == super::AuditEventType::PermissionDenied)
                .count();

            if permission_denied_count > 100 {
                violations.push(super::ComplianceViolation {
                    framework: "SOC2".to_string(),
                    control_id: "CC6.1".to_string(),
                    description: format!("大量权限拒绝事件: {}", permission_denied_count),
                    severity: super::Severity::Medium,
                    remediation: "审查访问控制策略".to_string(),
                });
            }
        }

        // ISO27001 控制点检查
        if framework == "ISO27001" || framework == "all" {
            // A.9.4.5: 安全登录程序
            let failed_login_count = records
                .iter()
                .filter(|r| r.event_type == super::AuditEventType::LoginFailure)
                .count();

            if failed_login_count > 50 {
                violations.push(super::ComplianceViolation {
                    framework: "ISO27001".to_string(),
                    control_id: "A.9.4.5".to_string(),
                    description: format!("登录失败次数过多: {}", failed_login_count),
                    severity: super::Severity::High,
                    remediation: "实施账户锁定策略".to_string(),
                });
            }
        }

        // 等保2.0 控制点检查
        if framework == "dengbao" || framework == "all" {
            // 安全审计要求
            let unverified_count = records
                .iter()
                .filter(|r| !r.compliance.integrity_verified)
                .count();

            if unverified_count > 0 {
                violations.push(super::ComplianceViolation {
                    framework: "等保2.0".to_string(),
                    control_id: "8.1.4.3".to_string(),
                    description: format!("存在 {} 条未验证完整性的审计记录", unverified_count),
                    severity: super::Severity::High,
                    remediation: "启用审计日志完整性保护".to_string(),
                });
            }
        }

        super::ComplianceStatus {
            soc2_compliant: violations.iter().all(|v| v.framework != "SOC2"),
            iso27001_compliant: violations.iter().all(|v| v.framework != "ISO27001"),
            dengbao_compliant: violations.iter().all(|v| v.framework != "等保2.0"),
            violations,
        }
    }

    fn detect_anomalies(&self, _records: &[AuditRecord]) -> Vec<super::Anomaly> {
        // 异常检测逻辑
        Vec::new()
    }
}

use serde::{Deserialize, Serialize};

/// 导出任务管理器
pub struct ExportTaskManager {
    tasks: std::collections::HashMap<String, ExportTask>,
    exporter: Arc<AuditExporter>,
}

use std::sync::Arc;

impl ExportTaskManager {
    pub fn new(exporter: Arc<AuditExporter>) -> Self {
        Self {
            tasks: std::collections::HashMap::new(),
            exporter,
        }
    }

    /// 创建新任务
    pub fn create_task(
        &mut self,
        created_by: impl Into<String>,
        filter: AuditFilter,
        format: ExportFormat,
    ) -> ExportTask {
        let task = ExportTask::new(created_by, filter, format);
        self.tasks.insert(task.task_id.clone(), task.clone());
        task
    }

    /// 获取任务状态
    pub fn get_task(&self, task_id: &str) -> Option<&ExportTask> {
        self.tasks.get(task_id)
    }

    /// 执行任务
    pub async fn execute_task(&self, task_id: &str) -> AuditResult<PathBuf> {
        let mut task = self
            .tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| super::AuditError::Export("Task not found".to_string()))?;

        self.exporter.execute_export(&mut task).await
    }

    /// 清理过期任务
    pub fn cleanup_expired_tasks(&mut self) {
        let now = Utc::now();
        self.tasks
            .retain(|_, task| task.expires_at.map(|exp| exp > now).unwrap_or(true));
    }
}
