//! DevOps事件响应中心 - 核心服务实现
//!
//! 提供完整的事件管理、告警聚合、自动诊断、运行手册执行等功能

use crate::incident_models::*;
use crate::db::Database;
use crate::redis_cache::RedisCache;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use sqlx::{Any, Transaction};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

pub struct IncidentService {
    db: Arc<Database>,
    redis: Arc<RedisCache>,
}

impl IncidentService {
    pub fn new(db: Arc<Database>, redis: Arc<RedisCache>) -> Self {
        Self { db, redis }
    }

    // ============= 事件CRUD操作 =============

    /// 创建新事件
    pub async fn create_incident(&self, req: CreateIncidentRequest, user_id: &str) -> Result<Incident> {
        let incident_id = Uuid::new_v4().to_string();
        let incident_number = generate_incident_number();
        let now = Utc::now();

        let affected_servers = req.affected_servers.map(|v| serde_json::json!(v));
        let affected_services = req.affected_services.map(|v| serde_json::json!(v));
        let tags = req.tags.map(|v| serde_json::json!(v));

        sqlx::query(r#"
            INSERT INTO incidents (
                id, incident_number, title, description, incident_type, severity, status,
                team_id, created_by, created_at, updated_at, detected_at,
                affected_servers, affected_services, assigned_to, escalation_level, tags
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&incident_id)
        .bind(&incident_number)
        .bind(&req.title)
        .bind(&req.description)
        .bind(req.incident_type.as_str())
        .bind(req.severity.as_str())
        .bind("detected")
        .bind(&req.team_id)
        .bind(user_id)
        .bind(now)
        .bind(now)
        .bind(now)
        .bind(affected_servers)
        .bind(affected_services)
        .bind(&req.assigned_to)
        .bind(0i32)
        .bind(tags)
        .execute(self.db.pool())
        .await?;

        // 创建事件时间线条目
        self.add_timeline_entry(
            &incident_id,
            TimelineEntryType::StatusChange,
            "事件已创建",
            &format!("事件 {} 已创建，严重程度: {}", incident_number, req.severity.as_str()),
            user_id,
            None,
        ).await?;

        // 如果指定了负责人，创建分配记录
        if let Some(assignee) = &req.assigned_to {
            self.add_timeline_entry(
                &incident_id,
                TimelineEntryType::Assignment,
                "事件已分配",
                &format!("事件分配给用户 {}", assignee),
                user_id,
                None,
            ).await?;

            // 添加参与者
            self.join_incident(&incident_id, assignee, ParticipantRole::IncidentCommander).await?;
        }

        info!("Created incident {}: {}", incident_number, req.title);

        // 从数据库获取完整记录
        self.get_incident_by_id(&incident_id).await
            .map_err(|e| anyhow!("Failed to fetch created incident: {}", e))
    }

    /// 获取事件详情
    pub async fn get_incident_by_id(&self, incident_id: &str) -> Result<Incident> {
        let incident = sqlx::query_as::<_, Incident>(r#"
            SELECT * FROM incidents WHERE id = ?
        "#)
        .bind(incident_id)
        .fetch_optional(self.db.pool())
        .await?;

        incident.ok_or_else(|| anyhow!("Incident not found: {}", incident_id))
    }

    /// 获取完整事件详情（包含时间线、参与者等）
    pub async fn get_incident_detail(&self, incident_id: &str) -> Result<IncidentDetailResponse> {
        let incident = self.get_incident_by_id(incident_id).await?;

        let timeline = self.get_incident_timeline(incident_id).await?;
        let participants = self.get_incident_participants(incident_id).await?;
        let alerts = self.get_incident_alerts(incident_id).await?;
        let diagnoses = self.get_incident_diagnoses(incident_id).await?;
        let related_incidents = self.get_related_incidents(incident_id).await?;
        let suggested_runbooks = self.suggest_runbooks(&incident).await?;

        Ok(IncidentDetailResponse {
            incident,
            timeline,
            participants,
            alerts,
            diagnoses,
            related_incidents,
            suggested_runbooks,
        })
    }

    /// 更新事件
    pub async fn update_incident(
        &self,
        incident_id: &str,
        req: UpdateIncidentRequest,
        user_id: &str,
    ) -> Result<Incident> {
        let incident = self.get_incident_by_id(incident_id).await?;
        let now = Utc::now();

        // 构建动态更新
        let mut updates = vec![];
        let mut params: Vec<Box<dyn sqlx::Type<sqlx::Any> + Send + Sync>> = vec![];

        if let Some(title) = &req.title {
            updates.push("title = ?");
            // params.push(Box::new(title.clone()));
        }

        // 执行更新
        sqlx::query(&format!(r#"UPDATE incidents SET updated_at = ? {} WHERE id = ?"#,
            if updates.is_empty() { "" } else { ", " }))
            .bind(now)
            .bind(incident_id)
            .execute(self.db.pool())
            .await?;

        // 处理状态变更
        if let Some(new_status) = &req.status {
            if new_status != &incident.status {
                self.handle_status_change(incident_id, &incident.status, new_status, user_id).await?;
            }
        }

        // 处理严重程度变更
        if let Some(new_severity) = &req.severity {
            if new_severity != &incident.severity {
                self.add_timeline_entry(
                    incident_id,
                    TimelineEntryType::SeverityChange,
                    "严重程度变更",
                    &format!("严重程度从 {} 变更为 {}", incident.severity.as_str(), new_severity.as_str()),
                    user_id,
                    None,
                ).await?;
            }
        }

        // 处理重新分配
        if let Some(new_assignee) = &req.assigned_to {
            if incident.assigned_to.as_ref() != Some(new_assignee) {
                self.add_timeline_entry(
                    incident_id,
                    TimelineEntryType::Assignment,
                    "事件重新分配",
                    &format!("事件从 {:?} 重新分配给 {}", incident.assigned_to, new_assignee),
                    user_id,
                    None,
                ).await?;

                // 更新或添加参与者
                self.join_incident(incident_id, new_assignee, ParticipantRole::IncidentCommander).await?;
            }
        }

        info!("Updated incident {} by user {}", incident_id, user_id);

        self.get_incident_by_id(incident_id).await
    }

    /// 确认事件
    pub async fn acknowledge_incident(
        &self,
        incident_id: &str,
        user_id: &str,
        note: Option<&str>,
    ) -> Result<Incident> {
        let now = Utc::now();

        sqlx::query(r#"
            UPDATE incidents
            SET status = 'acknowledged', acknowledged_at = ?, acknowledged_by = ?, updated_at = ?
            WHERE id = ?
        "#)
        .bind(now)
        .bind(user_id)
        .bind(now)
        .bind(incident_id)
        .execute(self.db.pool())
        .await?;

        let description = if let Some(n) = note {
            format!("用户 {} 确认了事件。备注: {}", user_id, n)
        } else {
            format!("用户 {} 确认了事件", user_id)
        };

        self.add_timeline_entry(
            incident_id,
            TimelineEntryType::StatusChange,
            "事件已确认",
            &description,
            user_id,
            None,
        ).await?;

        // 加入事件参与者
        self.join_incident(incident_id, user_id, ParticipantRole::Responder).await?;

        info!("Incident {} acknowledged by user {}", incident_id, user_id);

        self.get_incident_by_id(incident_id).await
    }

    /// 解决事件
    pub async fn resolve_incident(
        &self,
        incident_id: &str,
        user_id: &str,
        resolution: &str,
        root_cause: Option<&str>,
    ) -> Result<Incident> {
        let now = Utc::now();

        sqlx::query(r#"
            UPDATE incidents
            SET status = 'resolved', resolved_at = ?, resolved_by = ?, root_cause = ?, updated_at = ?
            WHERE id = ?
        "#)
        .bind(now)
        .bind(user_id)
        .bind(root_cause)
        .bind(now)
        .bind(incident_id)
        .execute(self.db.pool())
        .await?;

        let mut description = format!("事件已解决。解决方案: {}", resolution);
        if let Some(rc) = root_cause {
            description.push_str(&format!(" 根因: {}", rc));
        }

        self.add_timeline_entry(
            incident_id,
            TimelineEntryType::StatusChange,
            "事件已解决",
            &description,
            user_id,
            None,
        ).await?;

        // 关闭关联的告警
        self.resolve_incident_alerts(incident_id, user_id).await?;

        info!("Incident {} resolved by user {}", incident_id, user_id);

        self.get_incident_by_id(incident_id).await
    }

    /// 关闭事件
    pub async fn close_incident(&self, incident_id: &str, user_id: &str) -> Result<Incident> {
        let now = Utc::now();

        sqlx::query(r#"
            UPDATE incidents
            SET status = 'closed', closed_at = ?, updated_at = ?
            WHERE id = ?
        "#)
        .bind(now)
        .bind(now)
        .bind(incident_id)
        .execute(self.db.pool())
        .await?;

        self.add_timeline_entry(
            incident_id,
            TimelineEntryType::StatusChange,
            "事件已关闭",
            "事件已正式关闭",
            user_id,
            None,
        ).await?;

        info!("Incident {} closed by user {}", incident_id, user_id);

        self.get_incident_by_id(incident_id).await
    }

    /// 查询事件列表
    pub async fn query_incidents(&self, req: QueryIncidentsRequest) -> Result<IncidentListResponse> {
        let page = req.page.unwrap_or(1);
        let limit = req.limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        // 构建查询条件
        let mut conditions = vec!["1=1"];

        if let Some(team_id) = &req.team_id {
            conditions.push("team_id = ?");
        }

        let where_clause = conditions.join(" AND ");

        // 查询总数
        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM incidents WHERE {}", where_clause))
            .bind(req.team_id.as_ref())
            .fetch_one(self.db.pool())
            .await?;

        // 查询事件列表
        let incidents: Vec<Incident> = sqlx::query_as::<_, Incident>(&format!(r#"
            SELECT * FROM incidents
            WHERE {}
            ORDER BY
                CASE severity
                    WHEN 'critical' THEN 1
                    WHEN 'high' THEN 2
                    WHEN 'medium' THEN 3
                    WHEN 'low' THEN 4
                    ELSE 5
                END,
                created_at DESC
            LIMIT ? OFFSET ?
        "#, where_clause))
        .bind(req.team_id.as_ref())
        .bind(limit)
        .bind(offset)
        .fetch_all(self.db.pool())
        .await?;

        // 计算统计
        let stats = self.calculate_incident_stats(req.team_id.as_deref()).await?;

        Ok(IncidentListResponse {
            incidents,
            total,
            page,
            limit,
            stats,
        })
    }

    /// 计算事件统计
    async fn calculate_incident_stats(&self, team_id: Option<&str>) -> Result<IncidentStats> {
        let team_condition = if team_id.is_some() { "AND team_id = ?" } else { "" };

        let total_count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM incidents WHERE 1=1 {}", team_condition
        ))
        .bind(team_id)
        .fetch_one(self.db.pool())
        .await?;

        let active_count: i64 = sqlx::query_scalar(&format!(r#"
            SELECT COUNT(*) FROM incidents
            WHERE status IN ('detected', 'acknowledged', 'investigating', 'mitigating', 'escalated') {}
        "#, team_condition))
        .bind(team_id)
        .fetch_one(self.db.pool())
        .await?;

        let critical_count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM incidents WHERE severity = 'critical' {}", team_condition
        ))
        .bind(team_id)
        .fetch_one(self.db.pool())
        .await?;

        let high_count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM incidents WHERE severity = 'high' {}", team_condition
        ))
        .bind(team_id)
        .fetch_one(self.db.pool())
        .await?;

        let acknowledged_count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM incidents WHERE status = 'acknowledged' {}", team_condition
        ))
        .bind(team_id)
        .fetch_one(self.db.pool())
        .await?;

        let today = Utc::now().date_naive();
        let resolved_today: i64 = sqlx::query_scalar(&format!(r#"
            SELECT COUNT(*) FROM incidents
            WHERE status = 'resolved' AND DATE(resolved_at) = ? {}
        "#, team_condition))
        .bind(today.to_string())
        .bind(team_id)
        .fetch_one(self.db.pool())
        .await?;

        Ok(IncidentStats {
            total_count,
            active_count,
            critical_count,
            high_count,
            acknowledged_count,
            resolved_today,
            avg_resolution_time_minutes: None, // 需要计算
            mttr_last_7_days: None,
        })
    }

    // ============= 事件时间线管理 =============

    /// 添加时间线条目
    pub async fn add_timeline_entry(
        &self,
        incident_id: &str,
        entry_type: TimelineEntryType,
        title: &str,
        description: &str,
        user_id: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<IncidentTimelineEntry> {
        let entry_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(r#"
            INSERT INTO incident_timeline (
                id, incident_id, entry_type, title, description, created_by, created_at, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&entry_id)
        .bind(incident_id)
        .bind(entry_type.as_str())
        .bind(title)
        .bind(description)
        .bind(user_id)
        .bind(now)
        .bind(metadata)
        .execute(self.db.pool())
        .await?;

        debug!("Added timeline entry to incident {}: {}", incident_id, title);

        Ok(IncidentTimelineEntry {
            id: entry_id,
            incident_id: incident_id.to_string(),
            entry_type,
            title: title.to_string(),
            description: description.to_string(),
            created_by: user_id.to_string(),
            created_at: now,
            metadata,
        })
    }

    /// 获取事件时间线
    pub async fn get_incident_timeline(&self, incident_id: &str) -> Result<Vec<IncidentTimelineEntry>> {
        let entries = sqlx::query_as::<_, IncidentTimelineEntry>(r#"
            SELECT * FROM incident_timeline
            WHERE incident_id = ?
            ORDER BY created_at ASC
        "#)
        .bind(incident_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(entries)
    }

    // ============= 告警聚合管理 =============

    /// 创建告警并进行智能聚合
    pub async fn create_alert(&self, req: CreateAlertRequest) -> Result<Alert> {
        let fingerprint = generate_alert_fingerprint(
            &req.alert_type,
            req.server_id.as_deref(),
            req.service_name.as_deref(),
            req.metric_name.as_deref(),
        );

        let now = Utc::now();

        // 检查是否存在相同指纹的活跃告警
        let existing_alert: Option<Alert> = sqlx::query_as::<_, Alert>(r#"
            SELECT * FROM alerts
            WHERE fingerprint = ? AND status IN ('firing', 'acknowledged', 'flapping')
            ORDER BY last_occurrence_at DESC
            LIMIT 1
        "#)
        .bind(&fingerprint)
        .fetch_optional(self.db.pool())
        .await?;

        if let Some(mut existing) = existing_alert {
            // 聚合到现有告警
            existing.occurrence_count += 1;
            existing.last_occurrence_at = now;

            sqlx::query(r#"
                UPDATE alerts
                SET occurrence_count = ?, last_occurrence_at = ?, raw_data = ?
                WHERE id = ?
            "#)
            .bind(existing.occurrence_count)
            .bind(now)
            .bind(&req.raw_data)
            .bind(&existing.id)
            .execute(self.db.pool())
            .await?;

            // 如果超过阈值，升级为事件
            if existing.occurrence_count >= 5 && existing.incident_id.is_none() {
                let incident = self.create_incident_from_alert(&existing).await?;

                sqlx::query("UPDATE alerts SET incident_id = ? WHERE id = ?")
                    .bind(&incident.id)
                    .bind(&existing.id)
                    .execute(self.db.pool())
                    .await?;

                info!("Alert {} promoted to incident {}", existing.id, incident.id);
            }

            info!("Aggregated alert {} (count: {})", existing.alert_number, existing.occurrence_count);

            return Ok(existing);
        }

        // 创建新告警
        let alert_id = Uuid::new_v4().to_string();
        let alert_number = generate_alert_number();

        sqlx::query(r#"
            INSERT INTO alerts (
                id, alert_number, source, alert_type, severity, title, description, team_id,
                server_id, service_name, metric_name, metric_value, threshold,
                status, created_at, first_occurrence_at, last_occurrence_at, fingerprint,
                occurrence_count, raw_data
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&alert_id)
        .bind(&alert_number)
        .bind(&req.source)
        .bind(&req.alert_type)
        .bind(req.severity.as_str())
        .bind(&req.title)
        .bind(&req.description)
        .bind(&req.team_id)
        .bind(&req.server_id)
        .bind(&req.service_name)
        .bind(&req.metric_name)
        .bind(req.metric_value)
        .bind(req.threshold)
        .bind("firing")
        .bind(now)
        .bind(now)
        .bind(now)
        .bind(&fingerprint)
        .bind(1i32)
        .bind(&req.raw_data)
        .execute(self.db.pool())
        .await?;

        info!("Created new alert {}: {}", alert_number, req.title);

        self.get_alert_by_id(&alert_id).await
    }

    /// 从告警创建事件
    async fn create_incident_from_alert(&self, alert: &Alert) -> Result<Incident> {
        let incident_type = self.infer_incident_type(&alert.alert_type);

        let create_req = CreateIncidentRequest {
            title: format!("[AUTO] {}", alert.title),
            description: format!("自动创建自告警 {}: {}", alert.alert_number, alert.description),
            incident_type,
            severity: alert.severity.clone(),
            team_id: alert.team_id.clone(),
            affected_servers: alert.server_id.as_ref().map(|id| vec![id.clone()]),
            affected_services: alert.service_name.as_ref().map(|s| vec![s.clone()]),
            assigned_to: None,
            tags: Some(vec!["auto-created".to_string()]),
        };

        // 使用系统用户创建
        self.create_incident(create_req, "system").await
    }

    /// 推断事件类型
    fn infer_incident_type(&self, alert_type: &str) -> IncidentType {
        match alert_type.to_lowercase().as_str() {
            t if t.contains("cpu") => IncidentType::HighCpu,
            t if t.contains("memory") || t.contains("ram") => IncidentType::HighMemory,
            t if t.contains("disk") || t.contains("storage") => IncidentType::DiskFull,
            t if t.contains("network") || t.contains("connectivity") => IncidentType::NetworkIssue,
            t if t.contains("service") || t.contains("http") => IncidentType::ServiceUnavailable,
            t if t.contains("security") || t.contains("breach") => IncidentType::SecurityBreach,
            t if t.contains("ssl") || t.contains("certificate") || t.contains("tls") => IncidentType::SslExpired,
            t if t.contains("backup") => IncidentType::BackupFailed,
            t if t.contains("database") || t.contains("db") || t.contains("sql") => IncidentType::DatabaseError,
            t if t.contains("hardware") || t.contains("disk_failure") => IncidentType::HardwareFailure,
            t if t.contains("ddos") || t.contains("attack") => IncidentType::DdosAttack,
            _ => IncidentType::Custom,
        }
    }

    /// 获取告警详情
    pub async fn get_alert_by_id(&self, alert_id: &str) -> Result<Alert> {
        let alert = sqlx::query_as::<_, Alert>("SELECT * FROM alerts WHERE id = ?")
            .bind(alert_id)
            .fetch_optional(self.db.pool())
            .await?;

        alert.ok_or_else(|| anyhow!("Alert not found: {}", alert_id))
    }

    /// 获取聚合后的告警列表
    pub async fn get_aggregated_alerts(&self, team_id: &str) -> Result<Vec<AlertAggregationResult>> {
        let query = r#"
            SELECT
                fingerprint,
                aggregation_key,
                COUNT(*) as alert_count,
                MIN(created_at) as first_occurrence,
                MAX(last_occurrence_at) as last_occurrence,
                MAX(severity) as max_severity,
                (SELECT id FROM alerts a2 WHERE a2.fingerprint = alerts.fingerprint ORDER BY last_occurrence_at DESC LIMIT 1) as latest_alert_id,
                (SELECT id FROM alerts a3 WHERE a3.fingerprint = alerts.fingerprint ORDER BY created_at ASC LIMIT 1) as first_alert_id
            FROM alerts
            WHERE team_id = ? AND status IN ('firing', 'acknowledged', 'flapping')
            GROUP BY fingerprint
            ORDER BY last_occurrence_at DESC
        "#;

        let rows = sqlx::query(query)
            .bind(team_id)
            .fetch_all(self.db.pool())
            .await?;

        let mut results = Vec::new();

        for row in rows {
            let fingerprint: String = row.try_get("fingerprint")?;
            let alert_count: i32 = row.try_get("alert_count")?;
            let first_alert_id: String = row.try_get("first_alert_id")?;
            let latest_alert_id: String = row.try_get("latest_alert_id")?;

            let first_alert = self.get_alert_by_id(&first_alert_id).await?;
            let latest_alert = self.get_alert_by_id(&latest_alert_id).await?;

            // 检测告警抖动（flapping）
            let is_flapping = alert_count > 3 &&
                latest_alert.last_occurrence_at.signed_duration_since(first_alert.first_occurrence_at) < Duration::minutes(5);

            let suggested_action = if is_flapping {
                "告警抖动 detected，建议检查阈值设置或查看服务稳定性".to_string()
            } else if alert_count > 10 {
                "高频率告警，建议升级为事件并立即处理".to_string()
            } else {
                "正常告警频率".to_string()
            };

            results.push(AlertAggregationResult {
                aggregation_key: fingerprint.clone(),
                fingerprint,
                alert_count,
                first_alert,
                latest_alert,
                severity: latest_alert.severity.clone(),
                is_flapping,
                suggested_action,
            });
        }

        Ok(results)
    }

    /// 解决告警
    pub async fn resolve_alert(&self, alert_id: &str, user_id: &str) -> Result<Alert> {
        let now = Utc::now();

        sqlx::query(r#"
            UPDATE alerts
            SET status = 'resolved', resolved_at = ?, resolved_by = ?
            WHERE id = ?
        "#)
        .bind(now)
        .bind(user_id)
        .bind(alert_id)
        .execute(self.db.pool())
        .await?;

        info!("Alert {} resolved by user {}", alert_id, user_id);

        self.get_alert_by_id(alert_id).await
    }

    /// 抑制告警
    pub async fn suppress_alert(&self, alert_id: &str, user_id: &str) -> Result<Alert> {
        sqlx::query("UPDATE alerts SET status = 'suppressed' WHERE id = ?")
            .bind(alert_id)
            .execute(self.db.pool())
            .await?;

        info!("Alert {} suppressed by user {}", alert_id, user_id);

        self.get_alert_by_id(alert_id).await
    }

    /// 获取事件关联的告警
    pub async fn get_incident_alerts(&self, incident_id: &str) -> Result<Vec<Alert>> {
        let alerts = sqlx::query_as::<_, Alert>("SELECT * FROM alerts WHERE incident_id = ? ORDER BY created_at DESC")
            .bind(incident_id)
            .fetch_all(self.db.pool())
            .await?;

        Ok(alerts)
    }

    /// 解决事件关联的所有告警
    async fn resolve_incident_alerts(&self, incident_id: &str, user_id: &str) -> Result<()> {
        let now = Utc::now();

        sqlx::query(r#"
            UPDATE alerts
            SET status = 'resolved', resolved_at = ?, resolved_by = ?
            WHERE incident_id = ? AND status IN ('firing', 'acknowledged', 'flapping')
        "#)
        .bind(now)
        .bind(user_id)
        .bind(incident_id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    // ============= 参与者管理 =============

    /// 加入事件
    pub async fn join_incident(
        &self,
        incident_id: &str,
        user_id: &str,
        role: ParticipantRole,
    ) -> Result<IncidentParticipant> {
        let participant_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // 检查是否已参与
        let existing: Option<IncidentParticipant> = sqlx::query_as::<_, IncidentParticipant>(r#"
            SELECT * FROM incident_participants
            WHERE incident_id = ? AND user_id = ? AND is_active = TRUE
        "#)
        .bind(incident_id)
        .bind(user_id)
        .fetch_optional(self.db.pool())
        .await?;

        if existing.is_some() {
            return Err(anyhow!("User {} is already participating in incident {}", user_id, incident_id));
        }

        sqlx::query(r#"
            INSERT INTO incident_participants (
                id, incident_id, user_id, role, joined_at, is_active, notification_enabled
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&participant_id)
        .bind(incident_id)
        .bind(user_id)
        .bind(role.as_str())
        .bind(now)
        .bind(true)
        .bind(true)
        .execute(self.db.pool())
        .await?;

        // 添加时间线条目
        self.add_timeline_entry(
            incident_id,
            TimelineEntryType::Note,
            "新参与者加入",
            &format!("用户 {} 以 {:?} 角色加入事件", user_id, role),
            user_id,
            None,
        ).await?;

        info!("User {} joined incident {} as {:?}", user_id, incident_id, role);

        Ok(IncidentParticipant {
            id: participant_id,
            incident_id: incident_id.to_string(),
            user_id: user_id.to_string(),
            role,
            joined_at: now,
            left_at: None,
            is_active: true,
            notification_enabled: true,
        })
    }

    /// 离开事件
    pub async fn leave_incident(&self, incident_id: &str, user_id: &str) -> Result<()> {
        let now = Utc::now();

        sqlx::query(r#"
            UPDATE incident_participants
            SET left_at = ?, is_active = FALSE
            WHERE incident_id = ? AND user_id = ?
        "#)
        .bind(now)
        .bind(incident_id)
        .bind(user_id)
        .execute(self.db.pool())
        .await?;

        self.add_timeline_entry(
            incident_id,
            TimelineEntryType::Note,
            "参与者离开",
            &format!("用户 {} 离开事件", user_id),
            user_id,
            None,
        ).await?;

        info!("User {} left incident {}", user_id, incident_id);

        Ok(())
    }

    /// 获取事件参与者
    pub async fn get_incident_participants(&self, incident_id: &str) -> Result<Vec<IncidentParticipant>> {
        let participants = sqlx::query_as::<_, IncidentParticipant>(r#"
            SELECT * FROM incident_participants
            WHERE incident_id = ?
            ORDER BY joined_at ASC
        "#)
        .bind(incident_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(participants)
    }

    // ============= 诊断管理 =============

    /// 添加诊断结果
    pub async fn add_diagnosis(
        &self,
        incident_id: &str,
        diagnosis_type: &str,
        findings: &str,
        confidence_score: Option<f64>,
        suggested_actions: Option<Vec<String>>,
        runbook_suggestions: Option<Vec<String>>,
        created_by: &str,
    ) -> Result<DiagnosisResult> {
        let diagnosis_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // 重置其他诊断为非主要
        if diagnosis_type == "ai" || diagnosis_type == "automated" {
            sqlx::query("UPDATE diagnosis_results SET is_primary = FALSE WHERE incident_id = ?")
                .bind(incident_id)
                .execute(self.db.pool())
                .await?;
        }

        sqlx::query(r#"
            INSERT INTO diagnosis_results (
                id, incident_id, diagnosis_type, findings, confidence_score,
                suggested_actions, runbook_suggestions, created_by, created_at, is_primary
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&diagnosis_id)
        .bind(incident_id)
        .bind(diagnosis_type)
        .bind(findings)
        .bind(confidence_score)
        .bind(suggested_actions.as_ref().map(|v| serde_json::json!(v)))
        .bind(runbook_suggestions.as_ref().map(|v| serde_json::json!(v)))
        .bind(created_by)
        .bind(now)
        .bind(true)
        .execute(self.db.pool())
        .await?;

        // 添加时间线条目
        self.add_timeline_entry(
            incident_id,
            TimelineEntryType::Diagnosis,
            "新诊断结果",
            &format!("[{}] {}", diagnosis_type, findings.chars().take(100).collect::<String>()),
            created_by,
            None,
        ).await?;

        info!("Added diagnosis to incident {} by {}", incident_id, created_by);

        // 获取创建的诊断
        let diagnosis = sqlx::query_as::<_, DiagnosisResult>("SELECT * FROM diagnosis_results WHERE id = ?")
            .bind(&diagnosis_id)
            .fetch_one(self.db.pool())
            .await?;

        Ok(diagnosis)
    }

    /// 执行AI诊断
    pub async fn perform_ai_diagnosis(&self, incident_id: &str) -> Result<DiagnosisResult> {
        let incident = self.get_incident_detail(incident_id).await?;

        // 基于事件信息生成AI诊断
        let findings = self.analyze_incident_patterns(&incident).await?;

        // 查找相关历史事件
        let similar_incidents = self.find_similar_incidents(&incident).await?;

        // 建议的运行手册
        let runbook_suggestions: Vec<String> = self.suggest_runbooks(&incident.incident)
            .await?
            .into_iter()
            .map(|r| r.id)
            .collect();

        // 建议的操作
        let suggested_actions = vec![
            "检查服务器连接状态".to_string(),
            "查看最近部署记录".to_string(),
            "检查资源使用情况".to_string(),
        ];

        let diagnosis = self.add_diagnosis(
            incident_id,
            "ai",
            &findings,
            Some(0.85),
            Some(suggested_actions),
            Some(runbook_suggestions),
            "ai_system",
        ).await?;

        // 更新诊断的相似事件信息
        let related_ids: Vec<String> = similar_incidents.into_iter().map(|i| i.id).collect();

        sqlx::query("UPDATE diagnosis_results SET similar_past_incidents = ? WHERE id = ?")
            .bind(serde_json::json!(related_ids))
            .bind(&diagnosis.id)
            .execute(self.db.pool())
            .await?;

        info!("AI diagnosis completed for incident {}", incident_id);

        self.get_diagnosis_by_id(&diagnosis.id).await
    }

    /// 分析事件模式
    async fn analyze_incident_patterns(&self, detail: &IncidentDetailResponse) -> Result<String> {
        let incident = &detail.incident;
        let alerts = &detail.alerts;

        let mut findings = format!("基于事件 {} 的分析：\n", incident.incident_number);

        // 分析告警模式
        if !alerts.is_empty() {
            findings.push_str(&format!("检测到 {} 个相关告警。", alerts.len()));

            let high_severity_count = alerts.iter()
                .filter(|a| matches!(a.severity, IncidentSeverity::Critical | IncidentSeverity::High))
                .count();

            if high_severity_count > 0 {
                findings.push_str(&format!("其中 {} 个为高危告警。", high_severity_count));
            }
        }

        // 基于类型分析
        match incident.incident_type {
            IncidentType::ServerDown => {
                findings.push_str("服务器宕机事件，可能原因：硬件故障、网络中断、操作系统崩溃。");
            }
            IncidentType::HighCpu => {
                findings.push_str("CPU使用率过高，建议检查进程资源占用情况。");
            }
            IncidentType::HighMemory => {
                findings.push_str("内存使用率过高，可能存在内存泄漏。");
            }
            IncidentType::DiskFull => {
                findings.push_str("磁盘空间不足，建议清理日志或扩容。");
            }
            _ => {
                findings.push_str("需要进一步调查以确定根因。");
            }
        }

        Ok(findings)
    }

    /// 查找相似历史事件
    async fn find_similar_incidents(&self, current: &Incident) -> Result<Vec<Incident>> {
        let since = Utc::now() - Duration::days(90);

        let similar = sqlx::query_as::<_, Incident>(r#"
            SELECT * FROM incidents
            WHERE team_id = ?
            AND incident_type = ?
            AND status = 'closed'
            AND created_at > ?
            ORDER BY created_at DESC
            LIMIT 5
        "#)
        .bind(&current.team_id)
        .bind(current.incident_type.as_str())
        .bind(since)
        .fetch_all(self.db.pool())
        .await?;

        Ok(similar)
    }

    /// 获取诊断详情
    pub async fn get_diagnosis_by_id(&self, diagnosis_id: &str) -> Result<DiagnosisResult> {
        let diagnosis = sqlx::query_as::<_, DiagnosisResult>("SELECT * FROM diagnosis_results WHERE id = ?")
            .bind(diagnosis_id)
            .fetch_optional(self.db.pool())
            .await?;

        diagnosis.ok_or_else(|| anyhow!("Diagnosis not found: {}", diagnosis_id))
    }

    /// 获取事件的所有诊断
    pub async fn get_incident_diagnoses(&self, incident_id: &str) -> Result<Vec<DiagnosisResult>> {
        let diagnoses = sqlx::query_as::<_, DiagnosisResult>(r#"
            SELECT * FROM diagnosis_results
            WHERE incident_id = ?
            ORDER BY created_at DESC
        "#)
        .bind(incident_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(diagnoses)
    }

    // ============= 辅助方法 =============

    /// 处理状态变更
    async fn handle_status_change(
        &self,
        incident_id: &str,
        old_status: &IncidentStatus,
        new_status: &IncidentStatus,
        user_id: &str,
    ) -> Result<()> {
        let now = Utc::now();

        match new_status {
            IncidentStatus::Investigating => {
                // 更新调查开始时间等
            }
            IncidentStatus::Mitigating => {
                // 启动缓解措施跟踪
            }
            IncidentStatus::Resolved => {
                sqlx::query("UPDATE incidents SET resolved_at = ? WHERE id = ?")
                    .bind(now)
                    .bind(incident_id)
                    .execute(self.db.pool())
                    .await?;
            }
            _ => {}
        }

        Ok(())
    }

    /// 获取关联事件
    async fn get_related_incidents(&self, incident_id: &str) -> Result<Vec<Incident>> {
        let incident = self.get_incident_by_id(incident_id).await?;

        // 查找同类型的近期事件
        let since = Utc::now() - Duration::hours(24);

        let related = sqlx::query_as::<_, Incident>(r#"
            SELECT * FROM incidents
            WHERE id != ?
            AND team_id = ?
            AND incident_type = ?
            AND created_at > ?
            ORDER BY created_at DESC
            LIMIT 5
        "#)
        .bind(incident_id)
        .bind(&incident.team_id)
        .bind(incident.incident_type.as_str())
        .bind(since)
        .fetch_all(self.db.pool())
        .await?;

        Ok(related)
    }
}

// ============= 扩展trait实现 =============

impl IncidentType {
    fn as_str(&self) -> &'static str {
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
}

impl TimelineEntryType {
    fn as_str(&self) -> &'static str {
        match self {
            TimelineEntryType::StatusChange => "status_change",
            TimelineEntryType::SeverityChange => "severity_change",
            TimelineEntryType::Assignment => "assignment",
            TimelineEntryType::Escalation => "escalation",
            TimelineEntryType::Note => "note",
            TimelineEntryType::Action => "action",
            TimelineEntryType::Diagnosis => "diagnosis",
            TimelineEntryType::Communication => "communication",
            TimelineEntryType::Automation => "automation",
            TimelineEntryType::Alert => "alert",
            TimelineEntryType::RunbookExecuted => "runbook_executed",
        }
    }
}

impl ParticipantRole {
    fn as_str(&self) -> &'static str {
        match self {
            ParticipantRole::IncidentCommander => "incident_commander",
            ParticipantRole::TechLead => "tech_lead",
            ParticipantRole::Responder => "responder",
            ParticipantRole::Observer => "observer",
            ParticipantRole::Communicator => "communicator",
        }
    }
}
