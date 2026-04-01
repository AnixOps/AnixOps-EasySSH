//! DevOps事件响应中心 - 事后复盘与影响分析服务
//!
//! 提供事件报告生成、改进建议、影响分析等功能

use crate::incident_models::*;
use crate::db::Database;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

pub struct PostMortemService {
    db: Arc<Database>,
}

impl PostMortemService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    // ============= 事后复盘管理 =============

    /// 创建事后复盘
    pub async fn create_post_mortem(
        &self,
        incident_id: &str,
        title: &str,
        summary: &str,
        root_cause_analysis: &str,
        lessons_learned: &str,
        action_items: Vec<ActionItem>,
        user_id: &str,
    ) -> Result<PostMortem> {
        // 检查事件是否存在且已关闭/解决
        let incident: Incident = sqlx::query_as::<_, Incident>("SELECT * FROM incidents WHERE id = ?")
            .bind(incident_id)
            .fetch_optional(self.db.pool())
            .await?
            .ok_or_else(|| anyhow!("Incident not found: {}", incident_id))?;

        if !matches!(incident.status, IncidentStatus::Resolved | IncidentStatus::Closed) {
            return Err(anyhow!("Can only create post-mortem for resolved or closed incidents"));
        }

        // 检查是否已存在复盘
        let existing: Option<PostMortem> = sqlx::query_as::<_, PostMortem>(
            "SELECT * FROM post_mortems WHERE incident_id = ?"
        )
        .bind(incident_id)
        .fetch_optional(self.db.pool())
        .await?;

        if existing.is_some() {
            return Err(anyhow!("Post-mortem already exists for this incident"));
        }

        let post_mortem_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        // 收集贡献者
        let contributors = self.collect_contributors(incident_id).await?;

        // 生成时间线摘要
        let timeline_summary = self.generate_timeline_summary(incident_id).await?;

        // 解析行动项
        let action_items_json = if action_items.is_empty() {
            self.generate_suggested_action_items(&incident).await?
        } else {
            serde_json::json!(action_items)
        };

        sqlx::query(r#"
            INSERT INTO post_mortems (
                id, incident_id, title, summary, timeline_summary, root_cause_analysis,
                impact_analysis, resolution_steps, lessons_learned, action_items,
                contributors, started_at, status, created_by, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&post_mortem_id)
        .bind(incident_id)
        .bind(title)
        .bind(summary)
        .bind(&timeline_summary)
        .bind(root_cause_analysis)
        .bind("")  // impact_analysis 将在分析后更新
        .bind("")  // resolution_steps 将在分析后更新
        .bind(lessons_learned)
        .bind(&action_items_json)
        .bind(serde_json::json!(contributors))
        .bind(now)
        .bind("draft")
        .bind(user_id)
        .bind(now)
        .execute(self.db.pool())
        .await?;

        // 自动生成一些分析内容
        self.auto_analyze_post_mortem(&post_mortem_id, &incident).await?;

        info!("Created post-mortem {} for incident {}", post_mortem_id, incident_id);

        self.get_post_mortem_by_id(&post_mortem_id).await
    }

    /// 获取复盘详情
    pub async fn get_post_mortem_by_id(&self, post_mortem_id: &str) -> Result<PostMortem> {
        let post_mortem = sqlx::query_as::<_, PostMortem>("SELECT * FROM post_mortems WHERE id = ?")
            .bind(post_mortem_id)
            .fetch_optional(self.db.pool())
            .await?;

        post_mortem.ok_or_else(|| anyhow!("Post-mortem not found: {}", post_mortem_id))
    }

    /// 通过事件ID获取复盘
    pub async fn get_post_mortem_by_incident(&self, incident_id: &str) -> Result<Option<PostMortem>> {
        let post_mortem = sqlx::query_as::<_, PostMortem>("SELECT * FROM post_mortems WHERE incident_id = ?")
            .bind(incident_id)
            .fetch_optional(self.db.pool())
            .await?;

        Ok(post_mortem)
    }

    /// 更新复盘
    pub async fn update_post_mortem(
        &self,
        post_mortem_id: &str,
        summary: Option<&str>,
        root_cause_analysis: Option<&str>,
        impact_analysis: Option<&str>,
        resolution_steps: Option<&str>,
        lessons_learned: Option<&str>,
        action_items: Option<Vec<ActionItem>>,
        status: Option<PostMortemStatus>,
    ) -> Result<PostMortem> {
        let now = Utc::now();
        let current = self.get_post_mortem_by_id(post_mortem_id).await?;

        sqlx::query(r#"
            UPDATE post_mortems SET
                summary = ?,
                root_cause_analysis = ?,
                impact_analysis = ?,
                resolution_steps = ?,
                lessons_learned = ?,
                action_items = ?,
                status = ?,
                updated_at = ?
            WHERE id = ?
        "#)
        .bind(summary.unwrap_or(&current.summary))
        .bind(root_cause_analysis.unwrap_or(&current.root_cause_analysis))
        .bind(impact_analysis.unwrap_or(&current.impact_analysis))
        .bind(resolution_steps.unwrap_or(&current.resolution_steps))
        .bind(lessons_learned.unwrap_or(&current.lessons_learned))
        .bind(action_items.map(|a| serde_json::json!(a)))
        .bind(status.as_ref().map(|s| s.as_str()).unwrap_or(current.status.as_str()))
        .bind(now)
        .bind(post_mortem_id)
        .execute(self.db.pool())
        .await?;

        // 如果状态变为published，记录完成时间
        if let Some(PostMortemStatus::Published) = status {
            sqlx::query("UPDATE post_mortems SET completed_at = ? WHERE id = ?")
                .bind(now)
                .bind(post_mortem_id)
                .execute(self.db.pool())
                .await?;
        }

        info!("Updated post-mortem {}", post_mortem_id);

        self.get_post_mortem_by_id(post_mortem_id).await
    }

    /// 发布复盘
    pub async fn publish_post_mortem(&self, post_mortem_id: &str) -> Result<PostMortem> {
        let post_mortem = self.update_post_mortem(
            post_mortem_id,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(PostMortemStatus::Published),
        ).await?;

        info!("Published post-mortem {}", post_mortem_id);
        Ok(post_mortem)
    }

    /// 查询复盘列表
    pub async fn list_post_mortems(
        &self,
        team_id: Option<&str>,
        status: Option<PostMortemStatus>,
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
    ) -> Result<Vec<PostMortem>> {
        let mut query = r#"
            SELECT pm.* FROM post_mortems pm
            JOIN incidents i ON pm.incident_id = i.id
            WHERE 1=1
        "#.to_string();

        if team_id.is_some() {
            query.push_str(" AND i.team_id = ?");
        }

        if status.is_some() {
            query.push_str(" AND pm.status = ?");
        }

        if from_date.is_some() {
            query.push_str(" AND pm.started_at >= ?");
        }

        if to_date.is_some() {
            query.push_str(" AND pm.started_at <= ?");
        }

        query.push_str(" ORDER BY pm.started_at DESC");

        let mut q = sqlx::query_as::<_, PostMortem>(&query);

        if let Some(id) = team_id {
            q = q.bind(id);
        }

        if let Some(s) = status {
            q = q.bind(s.as_str());
        }

        if let Some(d) = from_date {
            q = q.bind(d);
        }

        if let Some(d) = to_date {
            q = q.bind(d);
        }

        let post_mortems = q.fetch_all(self.db.pool()).await?;

        Ok(post_mortems)
    }

    // ============= 自动分析功能 =============

    /// 收集贡献者
    async fn collect_contributors(&self, incident_id: &str) -> Result<Vec<String>> {
        let participants: Vec<(String,)> = sqlx::query_as(
            "SELECT user_id FROM incident_participants WHERE incident_id = ? AND is_active = TRUE"
        )
        .bind(incident_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(participants.into_iter().map(|p| p.0).collect())
    }

    /// 生成时间线摘要
    async fn generate_timeline_summary(&self, incident_id: &str) -> Result<String> {
        let entries: Vec<IncidentTimelineEntry> = sqlx::query_as::<_, IncidentTimelineEntry>(r#"
            SELECT * FROM incident_timeline
            WHERE incident_id = ?
            ORDER BY created_at ASC
        "#)
        .bind(incident_id)
        .fetch_all(self.db.pool())
        .await?;

        if entries.is_empty() {
            return Ok("无时间线记录".to_string());
        }

        let mut summary = String::from("## 事件时间线\n\n");

        for entry in &entries {
            let time = entry.created_at.format("%H:%M:%S");
            summary.push_str(&format!(
                "**{}** - [{}] {}\n\n",
                time,
                entry.entry_type.as_str(),
                entry.title
            ));
        }

        Ok(summary)
    }

    /// 生成建议的行动项
    async fn generate_suggested_action_items(&self, incident: &Incident) -> Result<serde_json::Value> {
        let mut items = Vec::new();

        // 基于事件类型生成建议
        match incident.incident_type {
            IncidentType::ServerDown => {
                items.push(ActionItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    description: "审查服务器高可用架构".to_string(),
                    assignee: "infra-team".to_string(),
                    priority: "high".to_string(),
                    due_date: Some(Utc::now() + Duration::days(7)),
                    status: "pending".to_string(),
                    created_at: Utc::now(),
                });
                items.push(ActionItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    description: "更新故障转移SOP".to_string(),
                    assignee: "sre-team".to_string(),
                    priority: "medium".to_string(),
                    due_date: Some(Utc::now() + Duration::days(14)),
                    status: "pending".to_string(),
                    created_at: Utc::now(),
                });
            }
            IncidentType::HighCpu | IncidentType::HighMemory => {
                items.push(ActionItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    description: "优化资源监控告警阈值".to_string(),
                    assignee: "monitoring-team".to_string(),
                    priority: "high".to_string(),
                    due_date: Some(Utc::now() + Duration::days(3)),
                    status: "pending".to_string(),
                    created_at: Utc::now(),
                });
            }
            IncidentType::DiskFull => {
                items.push(ActionItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    description: "实施自动日志清理策略".to_string(),
                    assignee: "sre-team".to_string(),
                    priority: "high".to_string(),
                    due_date: Some(Utc::now() + Duration::days(5)),
                    status: "pending".to_string(),
                    created_at: Utc::now(),
                });
            }
            _ => {
                items.push(ActionItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    description: "更新相关运行手册".to_string(),
                    assignee: "oncall-team".to_string(),
                    priority: "medium".to_string(),
                    due_date: Some(Utc::now() + Duration::days(7)),
                    status: "pending".to_string(),
                    created_at: Utc::now(),
                });
            }
        }

        // 通用的改进项
        items.push(ActionItem {
            id: uuid::Uuid::new_v4().to_string(),
            description: "审查和改进事件响应流程".to_string(),
            assignee: "incident-response-team".to_string(),
            priority: "medium".to_string(),
            due_date: Some(Utc::now() + Duration::days(14)),
            status: "pending".to_string(),
            created_at: Utc::now(),
        });

        Ok(serde_json::json!(items))
    }

    /// 自动分析复盘内容
    async fn auto_analyze_post_mortem(&self, post_mortem_id: &str, incident: &Incident) -> Result<()> {
        // 计算关键指标
        let metrics = self.calculate_incident_metrics(&incident.id).await?;

        // 生成解决方案步骤
        let resolution_steps = self.generate_resolution_steps(&incident.id).await?;

        // 生成影响分析摘要
        let impact_analysis = self.generate_impact_analysis(&incident.id).await?;

        // 更新复盘
        sqlx::query(r#"
            UPDATE post_mortems SET
                impact_analysis = ?,
                resolution_steps = ?
            WHERE id = ?
        "#)
        .bind(&impact_analysis)
        .bind(&resolution_steps)
        .bind(post_mortem_id)
        .execute(self.db.pool())
        .await?;

        info!("Auto-analyzed post-mortem {}", post_mortem_id);

        Ok(())
    }

    /// 计算事件指标
    async fn calculate_incident_metrics(&self, incident_id: &str) -> Result<IncidentMetrics> {
        let incident: Incident = sqlx::query_as::<_, Incident>("SELECT * FROM incidents WHERE id = ?")
            .bind(incident_id)
            .fetch_one(self.db.pool())
            .await?;

        // 计算MTTR (Mean Time To Resolution)
        let mttr_minutes = if let (Some(resolved_at), detected_at) = (incident.resolved_at, incident.detected_at) {
            let duration = resolved_at.signed_duration_since(detected_at);
            Some(duration.num_minutes() as f64)
        } else {
            None
        };

        // 计算确认时间
        let time_to_ack = if let (Some(acknowledged_at), detected_at) = (incident.acknowledged_at, incident.detected_at) {
            let duration = acknowledged_at.signed_duration_since(detected_at);
            Some(duration.num_minutes() as f64)
        } else {
            None
        };

        Ok(IncidentMetrics {
            period_start: incident.detected_at,
            period_end: incident.resolved_at.unwrap_or(Utc::now()),
            total_incidents: 1,
            incidents_by_severity: {
                let mut map = HashMap::new();
                map.insert(incident.severity.as_str().to_string(), 1);
                map
            },
            incidents_by_type: {
                let mut map = HashMap::new();
                map.insert(incident.incident_type.as_str().to_string(), 1);
                map
            },
            incidents_by_status: {
                let mut map = HashMap::new();
                map.insert(incident.status.as_str().to_string(), 1);
                map
            },
            avg_time_to_acknowledge_minutes: time_to_ack.unwrap_or(0.0),
            avg_time_to_resolve_minutes: mttr_minutes.unwrap_or(0.0),
            top_affected_services: vec![],  // 需要更多数据
            alert_storm_count: 0,
            false_positive_rate: 0.0,
        })
    }

    /// 生成解决方案步骤
    async fn generate_resolution_steps(&self, incident_id: &str) -> Result<String> {
        let entries: Vec<IncidentTimelineEntry> = sqlx::query_as::<_, IncidentTimelineEntry>(r#"
            SELECT * FROM incident_timeline
            WHERE incident_id = ? AND entry_type IN ('action', 'automation', 'runbook_executed')
            ORDER BY created_at ASC
        "#)
        .bind(incident_id)
        .fetch_all(self.db.pool())
        .await?;

        if entries.is_empty() {
            return Ok("未记录具体解决步骤".to_string());
        }

        let mut steps = String::from("## 解决步骤\n\n");

        for (i, entry) in entries.iter().enumerate() {
            steps.push_str(&format!(
                "{}. {}\n   - {}\n\n",
                i + 1,
                entry.title,
                entry.description
            ));
        }

        Ok(steps)
    }

    /// 生成影响分析
    async fn generate_impact_analysis(&self, incident_id: &str) -> Result<String> {
        let incident: Incident = sqlx::query_as::<_, Incident>("SELECT * FROM incidents WHERE id = ?")
            .bind(incident_id)
            .fetch_one(self.db.pool())
            .await?;

        let mut analysis = String::from("## 影响分析\n\n");

        // 持续时间
        let duration = if let Some(resolved_at) = incident.resolved_at {
            let d = resolved_at.signed_duration_since(incident.detected_at);
            format!("{}小时 {}分钟", d.num_hours(), d.num_minutes() % 60)
        } else {
            "进行中".to_string()
        };

        analysis.push_str(&format!("**事件持续时间:** {}\n\n", duration));

        // 严重程度
        analysis.push_str(&format!("**严重程度:** {}\n\n", incident.severity.as_str()));

        // 受影响服务器
        if let Some(servers) = &incident.affected_servers {
            if let Ok(server_list) = serde_json::from_value::<Vec<String>>(servers.clone()) {
                analysis.push_str(&format!("**受影响服务器:** {} 台\n\n", server_list.len()));
            }
        }

        // 受影响服务
        if let Some(services) = &incident.affected_services {
            if let Ok(service_list) = serde_json::from_value::<Vec<String>>(services.clone()) {
                analysis.push_str(&format!("**受影响服务:** {}\n", service_list.join(", ")));
            }
        }

        Ok(analysis)
    }

    // ============= 影响分析 =============

    /// 执行完整影响分析
    pub async fn analyze_impact(&self, incident_id: &str) -> Result<ImpactAnalysis> {
        let incident: Incident = sqlx::query_as::<_, Incident>("SELECT * FROM incidents WHERE id = ?")
            .bind(incident_id)
            .fetch_one(self.db.pool())
            .await?;

        // 分析受影响服务器
        let affected_servers = self.analyze_affected_servers(&incident).await?;

        // 分析受影响服务
        let affected_services = self.analyze_affected_services(&incident).await?;

        // 估算受影响用户
        let affected_users = self.estimate_affected_users(&incident, &affected_services).await?;

        // 业务影响评估
        let business_impact = self.assess_business_impact(&incident, &affected_services).await?;

        // 估算停机时间
        let estimated_downtime = incident.resolved_at.map(|resolved| {
            resolved.signed_duration_since(incident.detected_at).num_minutes()
        });

        // 财务影响估算
        let financial_impact = self.estimate_financial_impact(&incident, estimated_downtime).await?;

        Ok(ImpactAnalysis {
            incident_id: incident_id.to_string(),
            affected_servers,
            affected_services,
            affected_users,
            business_impact,
            estimated_downtime_minutes: estimated_downtime,
            financial_impact,
            analyzed_at: Utc::now(),
            analyzed_by: "system".to_string(),
        })
    }

    /// 分析受影响服务器
    async fn analyze_affected_servers(&self, incident: &Incident) -> Result<Vec<AffectedServer>> {
        let mut servers = Vec::new();

        if let Some(servers_json) = &incident.affected_servers {
            if let Ok(server_ids) = serde_json::from_value::<Vec<String>>(servers_json.clone()) {
                for server_id in server_ids {
                    // 在实际实现中，这里会从服务器数据库获取详细信息
                    servers.push(AffectedServer {
                        server_id: server_id.clone(),
                        server_name: format!("Server-{}", server_id[..8.min(server_id.len())].to_string()),
                        impact_level: ImpactLevel::Total,
                        services: vec![], // 需要从服务器配置获取
                    });
                }
            }
        }

        Ok(servers)
    }

    /// 分析受影响服务
    async fn analyze_affected_services(&self, incident: &Incident) -> Result<Vec<AffectedService>> {
        let mut services = Vec::new();

        if let Some(services_json) = &incident.affected_services {
            if let Ok(service_names) = serde_json::from_value::<Vec<String>>(services_json.clone()) {
                for service_name in service_names {
                    services.push(AffectedService {
                        service_name: service_name.clone(),
                        service_type: "application".to_string(), // 需要从服务注册表获取
                        status: "degraded".to_string(),
                        dependencies: vec![], // 需要从依赖图获取
                        impact_level: ImpactLevel::Total,
                    });
                }
            }
        }

        Ok(services)
    }

    /// 估算受影响用户
    async fn estimate_affected_users(
        &self,
        _incident: &Incident,
        affected_services: &[AffectedService],
    ) -> Result<Option<AffectedUsers>> {
        // 在实际实现中，这里会查询监控系统获取实际用户数
        let estimated_count = affected_services.len() as i64 * 1000; // 简化的估算

        Ok(Some(AffectedUsers {
            estimated_count,
            user_segments: vec!["general".to_string()],
            geographic_regions: vec!["global".to_string()],
        }))
    }

    /// 评估业务影响
    async fn assess_business_impact(
        &self,
        incident: &Incident,
        affected_services: &[AffectedService],
    ) -> Result<BusinessImpact> {
        let severity = match incident.severity {
            IncidentSeverity::Critical => "critical",
            IncidentSeverity::High => "high",
            IncidentSeverity::Medium => "medium",
            _ => "low",
        };

        let affected_functions: Vec<String> = affected_services
            .iter()
            .map(|s| s.service_name.clone())
            .collect();

        Ok(BusinessImpact {
            severity: severity.to_string(),
            description: format!("{} 导致业务功能受限", incident.title),
            affected_functions,
            workaround_available: false, // 需要根据实际情况判断
        })
    }

    /// 估算财务影响
    async fn estimate_financial_impact(
        &self,
        _incident: &Incident,
        downtime_minutes: Option<i64>,
    ) -> Result<Option<FinancialImpact>> {
        if let Some(minutes) = downtime_minutes {
            // 简化的估算：假设每分钟损失$100
            let revenue_loss = minutes as f64 * 100.0;

            Ok(Some(FinancialImpact {
                estimated_revenue_loss: Some(revenue_loss),
                estimated_recovery_cost: Some(revenue_loss * 0.3), // 恢复成本约为损失的30%
                currency: "USD".to_string(),
            }))
        } else {
            Ok(None)
        }
    }

    /// 生成AI改进建议
    pub async fn generate_improvement_suggestions(&self, post_mortem_id: &str) -> Result<Vec<String>> {
        let post_mortem = self.get_post_mortem_by_id(post_mortem_id).await?;
        let incident: Incident = sqlx::query_as::<_, Incident>("SELECT * FROM incidents WHERE id = ?")
            .bind(&post_mortem.incident_id)
            .fetch_one(self.db.pool())
            .await?;

        let mut suggestions = Vec::new();

        // 基于MTTR的建议
        if let Some(resolved_at) = incident.resolved_at {
            let mttr = resolved_at.signed_duration_since(incident.detected_at).num_minutes();

            if mttr > 60 {
                suggestions.push(format!(
                    "MTTR为{}分钟，超过1小时目标。建议优化事件响应流程，准备更多自动化工具。",
                    mttr
                ));
            }

            if mttr > 15 && matches!(incident.severity, IncidentSeverity::Critical) {
                suggestions.push("P1事件MTTR超过15分钟SLA，建议审查升级策略和响应团队可用性。".to_string());
            }
        }

        // 基于确认时间的建议
        if let Some(acknowledged_at) = incident.acknowledged_at {
            let time_to_ack = acknowledged_at.signed_duration_since(incident.detected_at).num_minutes();

            if time_to_ack > 5 {
                suggestions.push(format!(
                    "事件确认时间为{}分钟，建议优化告警路由，确保关键告警能及时到达响应人员。",
                    time_to_ack
                ));
            }
        } else {
            suggestions.push("事件未被及时确认，建议设置更严格的告警通知策略，包括短信和电话通知。".to_string());
        }

        // 基于事件类型的建议
        match incident.incident_type {
            IncidentType::ServerDown => {
                suggestions.push("建议实施多可用区部署策略，减少单点故障风险。".to_string());
            }
            IncidentType::HighCpu | IncidentType::HighMemory => {
                suggestions.push("建议优化自动扩缩容策略，提前预防资源瓶颈。".to_string());
            }
            IncidentType::DiskFull => {
                suggestions.push("建议实施自动存储管理策略，包括日志轮转和临时文件清理。".to_string());
            }
            _ => {}
        }

        // 通用建议
        suggestions.push("建议定期运行灾难恢复演练，验证响应流程的有效性。".to_string());
        suggestions.push("考虑引入更多可观测性工具，提前发现潜在问题。".to_string());

        Ok(suggestions)
    }

    /// 生成复盘报告
    pub async fn generate_post_mortem_report(&self, post_mortem_id: &str) -> Result<String> {
        let post_mortem = self.get_post_mortem_by_id(post_mortem_id).await?;
        let incident: Incident = sqlx::query_as::<_, Incident>("SELECT * FROM incidents WHERE id = ?")
            .bind(&post_mortem.incident_id)
            .fetch_one(self.db.pool())
            .await?;

        let impact = self.analyze_impact(&incident.id).await?;
        let suggestions = self.generate_improvement_suggestions(post_mortem_id).await?;

        let report = format!(r#"# {} 事后复盘报告

## 基本信息
- **事件编号:** {}
- **复盘标题:** {}
- **创建时间:** {}
- **状态:** {}

## 事件摘要
{}

## 时间线
{}

## 根因分析
{}

## 影响分析
{}

**受影响服务器:** {} 台
**受影响服务:** {}
**估算受影响用户:** {}

## 解决方案
{}

## 经验教训
{}

## 改进建议
{}

## 行动项
请查看系统中的行动项列表，确保所有改进措施按时完成。

---
*报告由 EasySSH 事件响应系统自动生成*
"#,
            incident.incident_number,
            incident.incident_number,
            post_mortem.title,
            post_mortem.started_at.format("%Y-%m-%d %H:%M:%S"),
            post_mortem.status.as_str(),
            post_mortem.summary,
            post_mortem.timeline_summary,
            post_mortem.root_cause_analysis,
            post_mortem.impact_analysis,
            impact.affected_servers.len(),
            impact.affected_services.iter().map(|s| s.service_name.clone()).collect::<Vec<_>>().join(", "),
            impact.affected_users.as_ref().map(|u| u.estimated_count.to_string()).unwrap_or_else(|| "未知".to_string()),
            post_mortem.resolution_steps,
            post_mortem.lessons_learned,
            suggestions.iter().enumerate().map(|(i, s)| format!("{}. {}", i + 1, s)).collect::<Vec<_>>().join("\n")
        );

        Ok(report)
    }
}

// ============= Trait扩展实现 =============

impl PostMortemStatus {
    fn as_str(&self) -> &'static str {
        match self {
            PostMortemStatus::Draft => "draft",
            PostMortemStatus::InReview => "in_review",
            PostMortemStatus::Approved => "approved",
            PostMortemStatus::Published => "published",
        }
    }
}

impl ImpactLevel {
    fn as_str(&self) -> &'static str {
        match self {
            ImpactLevel::Total => "total",
            ImpactLevel::Severe => "severe",
            ImpactLevel::Partial => "partial",
            ImpactLevel::Minimal => "minimal",
            ImpactLevel::None => "none",
        }
    }
}
