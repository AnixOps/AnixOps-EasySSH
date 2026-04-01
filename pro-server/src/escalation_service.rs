//! DevOps事件响应中心 - 升级策略服务
//!
//! 提供事件自动升级、升级策略管理、通知集成等功能

use crate::incident_models::*;
use crate::db::Database;
use crate::redis_cache::RedisCache;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

pub struct EscalationService {
    db: Arc<Database>,
    redis: Arc<RedisCache>,
}

impl EscalationService {
    pub fn new(db: Arc<Database>, redis: Arc<RedisCache>) -> Self {
        Self { db, redis }
    }

    // ============= 升级策略管理 =============

    /// 创建升级策略
    pub async fn create_escalation_policy(
        &self,
        name: &str,
        team_id: &str,
        rules: Vec<EscalationRule>,
        is_default: bool,
    ) -> Result<EscalationPolicy> {
        let policy_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        // 如果设为默认，取消其他默认策略
        if is_default {
            sqlx::query("UPDATE escalation_policies SET is_default = FALSE WHERE team_id = ?")
                .bind(team_id)
                .execute(self.db.pool())
                .await?;
        }

        sqlx::query(r#"
            INSERT INTO escalation_policies (
                id, name, team_id, is_default, rules, created_at, updated_at, is_active
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&policy_id)
        .bind(name)
        .bind(team_id)
        .bind(is_default)
        .bind(serde_json::json!(rules))
        .bind(now)
        .bind(now)
        .bind(true)
        .execute(self.db.pool())
        .await?;

        info!("Created escalation policy {}: {}", policy_id, name);

        self.get_policy_by_id(&policy_id).await
    }

    /// 获取升级策略详情
    pub async fn get_policy_by_id(&self, policy_id: &str) -> Result<EscalationPolicy> {
        let policy = sqlx::query_as::<_, EscalationPolicy>("SELECT * FROM escalation_policies WHERE id = ?")
            .bind(policy_id)
            .fetch_optional(self.db.pool())
            .await?;

        policy.ok_or_else(|| anyhow!("Escalation policy not found: {}", policy_id))
    }

    /// 获取团队的升级策略列表
    pub async fn get_team_policies(&self, team_id: &str) -> Result<Vec<EscalationPolicy>> {
        let policies = sqlx::query_as::<_, EscalationPolicy>(r#"
            SELECT * FROM escalation_policies
            WHERE team_id = ? AND is_active = TRUE
            ORDER BY is_default DESC, created_at DESC
        "#)
        .bind(team_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(policies)
    }

    /// 获取团队默认升级策略
    pub async fn get_default_policy(&self, team_id: &str) -> Result<Option<EscalationPolicy>> {
        let policy = sqlx::query_as::<_, EscalationPolicy>(r#"
            SELECT * FROM escalation_policies
            WHERE team_id = ? AND is_default = TRUE AND is_active = TRUE
            LIMIT 1
        "#)
        .bind(team_id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(policy)
    }

    /// 更新升级策略
    pub async fn update_policy(
        &self,
        policy_id: &str,
        name: Option<&str>,
        rules: Option<Vec<EscalationRule>>,
        is_default: Option<bool>,
        is_active: Option<bool>,
    ) -> Result<EscalationPolicy> {
        let now = Utc::now();
        let current = self.get_policy_by_id(policy_id).await?;

        // 处理默认策略变更
        if let Some(true) = is_default {
            sqlx::query("UPDATE escalation_policies SET is_default = FALSE WHERE team_id = ?")
                .bind(&current.team_id)
                .execute(self.db.pool())
                .await?;
        }

        sqlx::query(r#"
            UPDATE escalation_policies
            SET name = ?, rules = ?, is_default = ?, is_active = ?, updated_at = ?
            WHERE id = ?
        "#)
        .bind(name.unwrap_or(&current.name))
        .bind(rules.map(|r| serde_json::json!(r)))
        .bind(is_default.unwrap_or(current.is_default))
        .bind(is_active.unwrap_or(current.is_active))
        .bind(now)
        .bind(policy_id)
        .execute(self.db.pool())
        .await?;

        info!("Updated escalation policy {}", policy_id);

        self.get_policy_by_id(policy_id).await
    }

    /// 删除升级策略
    pub async fn delete_policy(&self, policy_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM escalation_policies WHERE id = ?")
            .bind(policy_id)
            .execute(self.db.pool())
            .await?;

        info!("Deleted escalation policy {}", policy_id);
        Ok(())
    }

    // ============= 事件升级处理 =============

    /// 手动升级事件
    pub async fn escalate_incident(
        &self,
        incident_id: &str,
        user_id: &str,
        reason: &str,
        target_level: Option<i32>,
        notify_users: Option<Vec<String>>,
    ) -> Result<Incident> {
        let incident = sqlx::query_as::<_, Incident>("SELECT * FROM incidents WHERE id = ?")
            .bind(incident_id)
            .fetch_optional(self.db.pool())
            .await?
            .ok_or_else(|| anyhow!("Incident not found: {}", incident_id))?;

        let current_level = incident.escalation_level;
        let new_level = target_level.unwrap_or(current_level + 1);

        if new_level <= current_level {
            return Err(anyhow!("Target escalation level must be higher than current level"));
        }

        let now = Utc::now();

        // 更新事件升级级别
        sqlx::query(r#"
            UPDATE incidents
            SET escalation_level = ?, status = 'escalated', updated_at = ?
            WHERE id = ?
        "#)
        .bind(new_level)
        .bind(now)
        .bind(incident_id)
        .execute(self.db.pool())
        .await?;

        // 记录升级历史
        let escalation_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(r#"
            INSERT INTO escalation_history (
                id, incident_id, from_level, to_level, escalated_by, reason, notified_users, escalated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&escalation_id)
        .bind(incident_id)
        .bind(current_level)
        .bind(new_level)
        .bind(user_id)
        .bind(reason)
        .bind(notify_users.as_ref().map(|u| serde_json::json!(u)))
        .bind(now)
        .execute(self.db.pool())
        .await?;

        // 添加时间线条目
        let timeline_entry = format!("事件从级别 {} 升级到级别 {}。原因: {}", current_level, new_level, reason);

        sqlx::query(r#"
            INSERT INTO incident_timeline (id, incident_id, entry_type, title, description, created_by, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(incident_id)
        .bind("escalation")
        .bind("事件升级")
        .bind(&timeline_entry)
        .bind(user_id)
        .bind(now)
        .execute(self.db.pool())
        .await?;

        info!(
            "Escalated incident {} from level {} to {} by user {}",
            incident_id, current_level, new_level, user_id
        );

        // 发送升级通知
        if let Some(users) = notify_users {
            for user_id in users {
                self.send_escalation_notification(incident_id, &user_id, new_level).await?;
            }
        }

        // 获取更新后的事件
        let updated = sqlx::query_as::<_, Incident>("SELECT * FROM incidents WHERE id = ?")
            .bind(incident_id)
            .fetch_one(self.db.pool())
            .await?;

        Ok(updated)
    }

    /// 自动检查并执行升级
    pub async fn check_auto_escalation(&self, incident_id: &str) -> Result<Option<Incident>> {
        let incident = sqlx::query_as::<_, Incident>("SELECT * FROM incidents WHERE id = ?")
            .bind(incident_id)
            .fetch_optional(self.db.pool())
            .await?
            .ok_or_else(|| anyhow!("Incident not found: {}", incident_id))?;

        // 只处理活跃事件
        if !incident.status.is_active() {
            return Ok(None);
        }

        // 获取团队升级策略
        let policy = match self.get_default_policy(&incident.team_id).await? {
            Some(p) => p,
            None => return Ok(None),
        };

        let rules: Vec<EscalationRule> = serde_json::from_value(policy.rules.clone())
            .unwrap_or_default();

        // 找到适用的下一级规则
        let next_rule = rules.iter().find(|r| r.level == incident.escalation_level + 1);

        if let Some(rule) = next_rule {
            // 检查升级条件
            let should_escalate = self.evaluate_escalation_conditions(&incident, &rule.condition).await?;

            if should_escalate {
                info!("Auto-escalating incident {} to level {}", incident_id, rule.level);

                // 执行自动升级
                let escalated = self.escalate_incident(
                    incident_id,
                    "system",
                    "自动升级：达到升级策略条件",
                    Some(rule.level),
                    Some(rule.notify_users.clone()),
                ).await?;

                return Ok(Some(escalated));
            }
        }

        Ok(None)
    }

    /// 评估升级条件
    async fn evaluate_escalation_conditions(
        &self,
        incident: &Incident,
        condition: &EscalationCondition,
    ) -> Result<bool> {
        match condition.condition_type.as_str() {
            "time_based" => {
                // 基于时间的升级
                if let Some(threshold_minutes) = condition.threshold_minutes {
                    let elapsed = Utc::now().signed_duration_since(incident.created_at);
                    let elapsed_minutes = elapsed.num_minutes();

                    return Ok(elapsed_minutes >= threshold_minutes as i64);
                }
            }
            "no_acknowledgment" => {
                // 无人确认的升级
                if let Some(threshold_minutes) = condition.threshold_minutes {
                    if incident.status == IncidentStatus::Detected {
                        let elapsed = Utc::now().signed_duration_since(incident.created_at);
                        let elapsed_minutes = elapsed.num_minutes();

                        return Ok(elapsed_minutes >= threshold_minutes as i64);
                    }
                }
            }
            "severity_based" => {
                // 基于严重程度的升级
                if let Some(severities) = &condition.severity_levels {
                    return Ok(severities.contains(&incident.severity));
                }
            }
            _ => {
                warn!("Unknown escalation condition type: {}", condition.condition_type);
            }
        }

        Ok(false)
    }

    /// 发送升级通知
    async fn send_escalation_notification(
        &self,
        incident_id: &str,
        user_id: &str,
        escalation_level: i32,
    ) -> Result<()> {
        // 在实际实现中，这里会调用通知服务发送消息
        info!(
            "Sending escalation notification for incident {} to user {}, level {}",
            incident_id, user_id, escalation_level
        );
        Ok(())
    }

    /// 获取升级历史
    pub async fn get_escalation_history(&self, incident_id: &str) -> Result<Vec<EscalationHistory>> {
        let history = sqlx::query_as::<_, EscalationHistory>(r#"
            SELECT * FROM escalation_history
            WHERE incident_id = ?
            ORDER BY escalated_at ASC
        "#)
        .bind(incident_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(history)
    }

    // ============= 通知集成管理 =============

    /// 创建集成配置
    pub async fn create_integration(
        &self,
        team_id: &str,
        provider: IntegrationProvider,
        name: &str,
        config: serde_json::Value,
    ) -> Result<IntegrationConfig> {
        let integration_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(r#"
            INSERT INTO integration_configs (
                id, team_id, provider, name, config, is_active, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&integration_id)
        .bind(team_id)
        .bind(provider.as_str())
        .bind(name)
        .bind(&config)
        .bind(true)
        .bind(now)
        .bind(now)
        .execute(self.db.pool())
        .await?;

        info!("Created integration {}: {} ({:?})", integration_id, name, provider);

        self.get_integration_by_id(&integration_id).await
    }

    /// 获取集成配置详情
    pub async fn get_integration_by_id(&self, integration_id: &str) -> Result<IntegrationConfig> {
        let integration = sqlx::query_as::<_, IntegrationConfig>("SELECT * FROM integration_configs WHERE id = ?")
            .bind(integration_id)
            .fetch_optional(self.db.pool())
            .await?;

        integration.ok_or_else(|| anyhow!("Integration not found: {}", integration_id))
    }

    /// 获取团队的所有集成配置
    pub async fn get_team_integrations(&self, team_id: &str) -> Result<Vec<IntegrationConfig>> {
        let integrations = sqlx::query_as::<_, IntegrationConfig>(r#"
            SELECT * FROM integration_configs
            WHERE team_id = ?
            ORDER BY is_active DESC, created_at DESC
        "#)
        .bind(team_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(integrations)
    }

    /// 获取特定类型的集成配置
    pub async fn get_integrations_by_provider(
        &self,
        team_id: &str,
        provider: &IntegrationProvider,
    ) -> Result<Vec<IntegrationConfig>> {
        let integrations = sqlx::query_as::<_, IntegrationConfig>(r#"
            SELECT * FROM integration_configs
            WHERE team_id = ? AND provider = ? AND is_active = TRUE
        "#)
        .bind(team_id)
        .bind(provider.as_str())
        .fetch_all(self.db.pool())
        .await?;

        Ok(integrations)
    }

    /// 更新集成配置
    pub async fn update_integration(
        &self,
        integration_id: &str,
        name: Option<&str>,
        config: Option<serde_json::Value>,
        is_active: Option<bool>,
    ) -> Result<IntegrationConfig> {
        let now = Utc::now();
        let current = self.get_integration_by_id(integration_id).await?;

        sqlx::query(r#"
            UPDATE integration_configs
            SET name = ?, config = ?, is_active = ?, updated_at = ?
            WHERE id = ?
        "#)
        .bind(name.unwrap_or(&current.name))
        .bind(config.unwrap_or(current.config.clone()))
        .bind(is_active.unwrap_or(current.is_active))
        .bind(now)
        .bind(integration_id)
        .execute(self.db.pool())
        .await?;

        info!("Updated integration {}", integration_id);

        self.get_integration_by_id(integration_id).await
    }

    /// 测试集成连接
    pub async fn test_integration(&self, integration_id: &str) -> Result<bool> {
        let integration = self.get_integration_by_id(integration_id).await?;

        let test_result = match integration.provider {
            IntegrationProvider::PagerDuty => {
                self.test_pagerduty_integration(&integration.config).await?
            }
            IntegrationProvider::OpsGenie => {
                self.test_opsgenie_integration(&integration.config).await?
            }
            IntegrationProvider::Slack => {
                self.test_slack_integration(&integration.config).await?
            }
            IntegrationProvider::Teams => {
                self.test_teams_integration(&integration.config).await?
            }
            IntegrationProvider::Webhook => {
                self.test_webhook_integration(&integration.config).await?
            }
            _ => {
                info!("Test not implemented for provider {:?}", integration.provider);
                true
            }
        };

        // 更新测试状态
        let now = Utc::now();
        let status = if test_result { "success" } else { "failed" };

        sqlx::query(r#"
            UPDATE integration_configs
            SET last_tested_at = ?, last_test_status = ?
            WHERE id = ?
        "#)
        .bind(now)
        .bind(status)
        .bind(integration_id)
        .execute(self.db.pool())
        .await?;

        Ok(test_result)
    }

    /// 删除集成配置
    pub async fn delete_integration(&self, integration_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM integration_configs WHERE id = ?")
            .bind(integration_id)
            .execute(self.db.pool())
            .await?;

        info!("Deleted integration {}", integration_id);
        Ok(())
    }

    // ============= 集成提供商特定测试 =============

    async fn test_pagerduty_integration(&self, config: &serde_json::Value) -> Result<bool> {
        // 提取API密钥和路由密钥
        let api_key = config.get("api_key").and_then(|v| v.as_str());
        let routing_key = config.get("routing_key").and_then(|v| v.as_str());

        if api_key.is_none() || routing_key.is_none() {
            return Ok(false);
        }

        // 在实际实现中，这里会发送测试事件到PagerDuty
        info!("Testing PagerDuty integration");
        Ok(true)
    }

    async fn test_opsgenie_integration(&self, config: &serde_json::Value) -> Result<bool> {
        let api_key = config.get("api_key").and_then(|v| v.as_str());

        if api_key.is_none() {
            return Ok(false);
        }

        info!("Testing OpsGenie integration");
        Ok(true)
    }

    async fn test_slack_integration(&self, config: &serde_json::Value) -> Result<bool> {
        let webhook_url = config.get("webhook_url").and_then(|v| v.as_str());
        let bot_token = config.get("bot_token").and_then(|v| v.as_str());

        if webhook_url.is_none() && bot_token.is_none() {
            return Ok(false);
        }

        info!("Testing Slack integration");
        Ok(true)
    }

    async fn test_teams_integration(&self, config: &serde_json::Value) -> Result<bool> {
        let webhook_url = config.get("webhook_url").and_then(|v| v.as_str());

        if webhook_url.is_none() {
            return Ok(false);
        }

        info!("Testing Teams integration");
        Ok(true)
    }

    async fn test_webhook_integration(&self, config: &serde_json::Value) -> Result<bool> {
        let url = config.get("url").and_then(|v| v.as_str());

        if url.is_none() {
            return Ok(false);
        }

        info!("Testing webhook integration to {}", url.unwrap());
        Ok(true)
    }

    // ============= 发送通知 =============

    /// 发送事件通知到所有配置的集成
    pub async fn send_incident_notifications(
        &self,
        incident: &Incident,
        notification_type: CommunicationType,
    ) -> Result<Vec<CommunicationResult>> {
        let integrations = self.get_team_integrations(&incident.team_id).await?;

        let mut results = Vec::new();

        for integration in integrations {
            if !integration.is_active {
                continue;
            }

            let result = match integration.provider {
                IntegrationProvider::PagerDuty => {
                    self.send_pagerduty_notification(&integration.config, incident, &notification_type).await
                }
                IntegrationProvider::OpsGenie => {
                    self.send_opsgenie_notification(&integration.config, incident, &notification_type).await
                }
                IntegrationProvider::Slack => {
                    self.send_slack_notification(&integration.config, incident, &notification_type).await
                }
                IntegrationProvider::Teams => {
                    self.send_teams_notification(&integration.config, incident, &notification_type).await
                }
                IntegrationProvider::Email => {
                    self.send_email_notification(&integration.config, incident, &notification_type).await
                }
                IntegrationProvider::Webhook => {
                    self.send_webhook_notification(&integration.config, incident, &notification_type).await
                }
                _ => Ok(CommunicationResult {
                    success: false,
                    channel: integration.provider.as_str().to_string(),
                    error: Some("Provider not implemented".to_string()),
                }),
            };

            results.push(result?);
        }

        Ok(results)
    }

    /// PagerDuty通知
    async fn send_pagerduty_notification(
        &self,
        config: &serde_json::Value,
        incident: &Incident,
        notification_type: &CommunicationType,
    ) -> Result<CommunicationResult> {
        // 构建PagerDuty事件
        let routing_key = config.get("routing_key").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing routing_key in PagerDuty config"))?;

        let action = match notification_type {
            CommunicationType::Notification => "trigger",
            CommunicationType::Resolution => "resolve",
            _ => "trigger",
        };

        let payload = serde_json::json!({
            "routing_key": routing_key,
            "event_action": action,
            "dedup_key": format!("easyssh-{}", incident.id),
            "payload": {
                "summary": &incident.title,
                "severity": incident.severity.as_str(),
                "source": &incident.team_id,
                "custom_details": {
                    "incident_id": &incident.id,
                    "incident_number": &incident.incident_number,
                    "description": &incident.description,
                }
            }
        });

        // 在实际实现中，这里会发送HTTP请求到PagerDuty Events API
        info!("Would send PagerDuty notification: {}", payload.to_string());

        Ok(CommunicationResult {
            success: true,
            channel: "pagerduty".to_string(),
            error: None,
        })
    }

    /// OpsGenie通知
    async fn send_opsgenie_notification(
        &self,
        config: &serde_json::Value,
        incident: &Incident,
        notification_type: &CommunicationType,
    ) -> Result<CommunicationResult> {
        let api_key = config.get("api_key").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing api_key in OpsGenie config"))?;

        let payload = serde_json::json!({
            "message": &incident.title,
            "description": &incident.description,
            "priority": match incident.severity {
                IncidentSeverity::Critical => "P1",
                IncidentSeverity::High => "P2",
                IncidentSeverity::Medium => "P3",
                IncidentSeverity::Low => "P4",
                IncidentSeverity::Info => "P5",
            },
            "alias": format!("easyssh-{}", incident.incident_number),
            "details": {
                "incident_id": &incident.id,
                "incident_number": &incident.incident_number,
            }
        });

        info!("Would send OpsGenie notification: {}", payload.to_string());

        Ok(CommunicationResult {
            success: true,
            channel: "opsgenie".to_string(),
            error: None,
        })
    }

    /// Slack通知
    async fn send_slack_notification(
        &self,
        config: &serde_json::Value,
        incident: &Incident,
        notification_type: &CommunicationType,
    ) -> Result<CommunicationResult> {
        let webhook_url = config.get("webhook_url").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing webhook_url in Slack config"))?;

        let color = match incident.severity {
            IncidentSeverity::Critical => "#FF0000",
            IncidentSeverity::High => "#FF8C00",
            IncidentSeverity::Medium => "#FFD700",
            IncidentSeverity::Low => "#32CD32",
            IncidentSeverity::Info => "#1E90FF",
        };

        let payload = serde_json::json!({
            "attachments": [{
                "color": color,
                "title": format!("{} - {}", incident.incident_number, incident.title),
                "title_link": format!("https://easyssh.io/incidents/{}", incident.id),
                "fields": [
                    {
                        "title": "严重程度",
                        "value": incident.severity.as_str(),
                        "short": true
                    },
                    {
                        "title": "状态",
                        "value": incident.status.as_str(),
                        "short": true
                    },
                    {
                        "title": "描述",
                        "value": &incident.description,
                        "short": false
                    }
                ],
                "footer": "EasySSH Incident Response",
                "ts": Utc::now().timestamp()
            }]
        });

        info!("Would send Slack notification to {}", webhook_url);

        Ok(CommunicationResult {
            success: true,
            channel: "slack".to_string(),
            error: None,
        })
    }

    /// Teams通知
    async fn send_teams_notification(
        &self,
        config: &serde_json::Value,
        incident: &Incident,
        notification_type: &CommunicationType,
    ) -> Result<CommunicationResult> {
        let webhook_url = config.get("webhook_url").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing webhook_url in Teams config"))?;

        let payload = serde_json::json!({
            "@type": "MessageCard",
            "@context": "https://schema.org/extensions",
            "themeColor": match incident.severity {
                IncidentSeverity::Critical => "FF0000",
                IncidentSeverity::High => "FF8C00",
                IncidentSeverity::Medium => "FFD700",
                IncidentSeverity::Low => "32CD32",
                IncidentSeverity::Info => "1E90FF",
            },
            "summary": &incident.title,
            "sections": [{
                "activityTitle": &incident.title,
                "activitySubtitle": &incident.description,
                "facts": [
                    {
                        "name": "事件编号",
                        "value": &incident.incident_number
                    },
                    {
                        "name": "严重程度",
                        "value": incident.severity.as_str()
                    },
                    {
                        "name": "状态",
                        "value": incident.status.as_str()
                    }
                ]
            }]
        });

        info!("Would send Teams notification to {}", webhook_url);

        Ok(CommunicationResult {
            success: true,
            channel: "teams".to_string(),
            error: None,
        })
    }

    /// Email通知
    async fn send_email_notification(
        &self,
        config: &serde_json::Value,
        incident: &Incident,
        notification_type: &CommunicationType,
    ) -> Result<CommunicationResult> {
        let smtp_host = config.get("smtp_host").and_then(|v| v.as_str());
        let from_address = config.get("from_address").and_then(|v| v.as_str());

        if smtp_host.is_none() || from_address.is_none() {
            return Ok(CommunicationResult {
                success: false,
                channel: "email".to_string(),
                error: Some("Missing SMTP configuration".to_string()),
            });
        }

        info!("Would send email notification for incident {}", incident.id);

        Ok(CommunicationResult {
            success: true,
            channel: "email".to_string(),
            error: None,
        })
    }

    /// Webhook通知
    async fn send_webhook_notification(
        &self,
        config: &serde_json::Value,
        incident: &Incident,
        notification_type: &CommunicationType,
    ) -> Result<CommunicationResult> {
        let url = config.get("url").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing url in webhook config"))?;

        let payload = serde_json::json!({
            "event": "incident_update",
            "notification_type": notification_type.as_str(),
            "incident": incident,
            "timestamp": Utc::now().to_rfc3339(),
        });

        info!("Would send webhook notification to {}: {}", url, payload.to_string());

        Ok(CommunicationResult {
            success: true,
            channel: "webhook".to_string(),
            error: None,
        })
    }
}

// ============= 辅助结构 =============

pub struct CommunicationResult {
    pub success: bool,
    pub channel: String,
    pub error: Option<String>,
}

// ============= Trait扩展实现 =============

impl IntegrationProvider {
    fn as_str(&self) -> &'static str {
        match self {
            IntegrationProvider::PagerDuty => "pagerduty",
            IntegrationProvider::OpsGenie => "opsgenie",
            IntegrationProvider::Slack => "slack",
            IntegrationProvider::Teams => "teams",
            IntegrationProvider::Webhook => "webhook",
            IntegrationProvider::Email => "email",
            IntegrationProvider::Sms => "sms",
            IntegrationProvider::Discord => "discord",
        }
    }
}

impl CommunicationType {
    fn as_str(&self) -> &'static str {
        match self {
            CommunicationType::Notification => "notification",
            CommunicationType::StatusUpdate => "status_update",
            CommunicationType::Escalation => "escalation",
            CommunicationType::Resolution => "resolution",
            CommunicationType::StakeholderUpdate => "stakeholder_update",
        }
    }
}

impl IncidentStatus {
    fn as_str(&self) -> &'static str {
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
}
