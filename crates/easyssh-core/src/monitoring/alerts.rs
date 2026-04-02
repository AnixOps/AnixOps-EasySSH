#![allow(dead_code)]

//! Alert management system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::monitoring::metrics::{MetricType, ServerMetrics};

use super::notifications::{NotificationChannel, NotificationChannelType};

pub use super::notifications::{
    ConfigValidationError, NotificationError, NotificationPayload, NotificationSender,
};

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

impl AlertSeverity {
    pub fn color(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "#3b82f6",
            AlertSeverity::Warning => "#f59e0b",
            AlertSeverity::Critical => "#ef4444",
            AlertSeverity::Emergency => "#7f1d1d",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "info",
            AlertSeverity::Warning => "alert-triangle",
            AlertSeverity::Critical => "x-circle",
            AlertSeverity::Emergency => "alert-octagon",
        }
    }

    pub fn notification_priority(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "low",
            AlertSeverity::Warning => "normal",
            AlertSeverity::Critical => "high",
            AlertSeverity::Emergency => "urgent",
        }
    }
}

/// Alert status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertStatus {
    Active,
    Acknowledged,
    Resolved,
    Silenced,
    Flapping,
}

/// An alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub server_id: String,
    pub server_name: String,
    pub severity: AlertSeverity,
    pub status: AlertStatus,
    pub title: String,
    pub message: String,
    pub metric_type: MetricType,
    pub metric_value: f64,
    pub threshold: f64,
    pub started_at: u64,
    pub acknowledged_at: Option<u64>,
    pub acknowledged_by: Option<String>,
    pub resolved_at: Option<u64>,
    pub flapping_count: u32,
    pub tags: Vec<String>,
    pub runbook_url: Option<String>,
    pub dashboard_url: Option<String>,
}

/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub severity: AlertSeverity,

    // Conditions
    pub metric_type: MetricType,
    pub condition: AlertCondition,
    pub threshold: f64,
    pub duration_secs: u64, // How long condition must persist
    pub cooldown_secs: u64, // Minimum time between alerts

    // Scope
    pub server_ids: Vec<String>, // Empty = all servers
    pub server_groups: Vec<String>,
    pub tags: Vec<String>,

    // Notifications
    pub notification_channels: Vec<String>,
    pub auto_resolve: bool,
    pub resolve_after_secs: u64,

    // Metadata
    pub runbook_url: Option<String>,
    pub dashboard_url: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Alert condition types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertCondition {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
    Between { min: f64, max: f64 },
    Outside { min: f64, max: f64 },
    Anomaly,
    NoData,
}

impl AlertCondition {
    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            AlertCondition::GreaterThan => value > threshold,
            AlertCondition::GreaterThanOrEqual => value >= threshold,
            AlertCondition::LessThan => value < threshold,
            AlertCondition::LessThanOrEqual => value <= threshold,
            AlertCondition::Equal => (value - threshold).abs() < f64::EPSILON,
            AlertCondition::NotEqual => (value - threshold).abs() >= f64::EPSILON,
            AlertCondition::Between { min, max } => value >= *min && value <= *max,
            AlertCondition::Outside { min, max } => value < *min || value > *max,
            AlertCondition::Anomaly => false, // Handled separately
            AlertCondition::NoData => false,  // Handled separately
        }
    }
}

/// Alert engine that evaluates rules and manages alert lifecycle
pub struct AlertEngine {
    storage: Arc<super::storage::MetricsStorage>,
    rules: Arc<RwLock<Vec<AlertRule>>>,
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    alert_history: Arc<RwLock<Vec<Alert>>>,
    notification_channels: Arc<RwLock<Vec<NotificationChannel>>>,
    notification_manager: Arc<tokio::sync::RwLock<super::notifications::NotificationManager>>,
    config: AlertEngineConfig,
    running: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone)]
pub struct AlertEngineConfig {
    pub check_interval_secs: u64,
    pub max_alert_history: usize,
    pub flapping_threshold: u32,
    pub flapping_window_secs: u64,
}

impl AlertEngine {
    pub async fn new(
        storage: Arc<super::storage::MetricsStorage>,
        config: &super::MonitoringConfig,
    ) -> Result<Self, super::MonitoringError> {
        Ok(Self {
            storage,
            rules: Arc::new(RwLock::new(Vec::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
            notification_channels: Arc::new(RwLock::new(Vec::new())),
            notification_manager: Arc::new(tokio::sync::RwLock::new(
                super::notifications::NotificationManager::new(),
            )),
            config: AlertEngineConfig {
                check_interval_secs: config.alert_check_interval_secs,
                max_alert_history: 10000,
                flapping_threshold: 3,
                flapping_window_secs: 300,
            },
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Add or update an alert rule
    pub async fn upsert_rule(&self, rule: AlertRule) -> Result<(), super::MonitoringError> {
        let mut rules = self.rules.write().await;

        if let Some(idx) = rules.iter().position(|r| r.id == rule.id) {
            rules[idx] = rule.clone();
        } else {
            rules.push(rule.clone());
        }

        self.storage.save_alert_rule(&rule).await?;
        Ok(())
    }

    /// Delete an alert rule
    pub async fn delete_rule(&self, rule_id: &str) -> Result<(), super::MonitoringError> {
        let mut rules = self.rules.write().await;
        rules.retain(|r| r.id != rule_id);
        self.storage.delete_alert_rule(rule_id).await?;
        Ok(())
    }

    /// Get all alert rules
    pub async fn get_rules(&self) -> Vec<AlertRule> {
        self.rules.read().await.clone()
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Result<Vec<Alert>, super::MonitoringError> {
        let alerts = self.active_alerts.read().await;
        Ok(alerts.values().cloned().collect())
    }

    /// Get alert history with filters
    pub async fn get_alert_history(
        &self,
        server_id: Option<&str>,
        severity: Option<AlertSeverity>,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<Alert>, super::MonitoringError> {
        self.storage
            .get_alert_history(server_id, severity, start_time, end_time)
            .await
    }

    /// Acknowledge an alert
    pub async fn acknowledge_alert(
        &self,
        alert_id: &str,
        user_id: &str,
    ) -> Result<(), super::MonitoringError> {
        let mut alerts = self.active_alerts.write().await;

        if let Some(alert) = alerts.get_mut(alert_id) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            alert.status = AlertStatus::Acknowledged;
            alert.acknowledged_at = Some(now);
            alert.acknowledged_by = Some(user_id.to_string());

            self.storage.update_alert(alert).await?;
        }

        Ok(())
    }

    /// Resolve an alert
    pub async fn resolve_alert(&self, alert_id: &str) -> Result<(), super::MonitoringError> {
        let mut active_alerts = self.active_alerts.write().await;
        let mut history = self.alert_history.write().await;

        if let Some(mut alert) = active_alerts.remove(alert_id) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            alert.status = AlertStatus::Resolved;
            alert.resolved_at = Some(now);

            history.push(alert.clone());
            self.storage.update_alert(&alert).await?;
        }

        Ok(())
    }

    /// Silence an alert (temporarily suppress notifications)
    pub async fn silence_alert(
        &self,
        alert_id: &str,
        _duration_minutes: u32,
    ) -> Result<(), super::MonitoringError> {
        let mut alerts = self.active_alerts.write().await;

        if let Some(alert) = alerts.get_mut(alert_id) {
            alert.status = AlertStatus::Silenced;
            // Schedule unsilence after duration
            // Implementation would use a background task
        }

        Ok(())
    }

    /// Start the alert engine
    pub async fn start(&self) -> Result<(), super::MonitoringError> {
        *self.running.write().await = true;

        // Load rules from storage
        let rules = self.storage.load_alert_rules().await?;
        *self.rules.write().await = rules;

        // Start background evaluation loop
        let rules = Arc::clone(&self.rules);
        let active_alerts = Arc::clone(&self.active_alerts);
        let storage = Arc::clone(&self.storage);
        let notification_manager = Arc::clone(&self.notification_manager);
        let running = Arc::clone(&self.running);
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(config.check_interval_secs));

            while *running.read().await {
                interval.tick().await;

                let rules_snapshot = rules.read().await.clone();

                for rule in rules_snapshot {
                    if !rule.enabled {
                        continue;
                    }

                    // Evaluate rule against metrics
                    if let Err(e) =
                        Self::evaluate_rule(&rule, &storage, &active_alerts, &notification_manager)
                            .await
                    {
                        log::error!("Failed to evaluate alert rule {}: {}", rule.id, e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the alert engine
    pub async fn stop(&self) {
        *self.running.write().await = false;
    }

    async fn evaluate_rule(
        rule: &AlertRule,
        storage: &Arc<super::storage::MetricsStorage>,
        active_alerts: &Arc<RwLock<HashMap<String, Alert>>>,
        notification_manager: &Arc<tokio::sync::RwLock<super::notifications::NotificationManager>>,
    ) -> Result<(), super::MonitoringError> {
        // Get servers to check
        let server_ids = if rule.server_ids.is_empty() {
            // Get all monitored servers
            storage.get_monitored_servers().await?
        } else {
            rule.server_ids.clone()
        };

        for server_id in server_ids {
            // Get latest metrics
            let metrics = storage.get_latest_metrics(&server_id).await?;

            // Extract metric value based on metric_type
            let value = Self::extract_metric_value(&metrics, &rule.metric_type);

            if let Some(value) = value {
                let condition_met = rule.condition.evaluate(value, rule.threshold);

                let alert_id = format!("{}:{}", rule.id, server_id);
                let mut alerts = active_alerts.write().await;

                if condition_met {
                    // Check if alert already exists
                    if !alerts.contains_key(&alert_id) {
                        // Create new alert
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        let alert = Alert {
                            id: alert_id.clone(),
                            rule_id: rule.id.clone(),
                            rule_name: rule.name.clone(),
                            server_id: server_id.clone(),
                            server_name: server_id.clone(), // Would fetch actual name
                            severity: rule.severity,
                            status: AlertStatus::Active,
                            title: format!("{} - {}", rule.name, server_id),
                            message: format!(
                                "{} is {} (current: {:.2}, threshold: {:.2})",
                                format!("{:?}", rule.metric_type),
                                Self::condition_description(&rule.condition),
                                value,
                                rule.threshold
                            ),
                            metric_type: rule.metric_type.clone(),
                            metric_value: value,
                            threshold: rule.threshold,
                            started_at: now,
                            acknowledged_at: None,
                            acknowledged_by: None,
                            resolved_at: None,
                            flapping_count: 0,
                            tags: rule.tags.clone(),
                            runbook_url: rule.runbook_url.clone(),
                            dashboard_url: rule.dashboard_url.clone(),
                        };

                        alerts.insert(alert_id, alert.clone());
                        storage.save_alert(&alert).await?;
                        drop(alerts); // Release lock before async notification

                        // Send notifications using the notification manager
                        let payload = super::notifications::NotificationPayload::from_alert(&alert);
                        let manager = notification_manager.read().await;
                        let results = manager
                            .send_to_channels(&rule.notification_channels, &payload, 60)
                            .await;
                        drop(manager);

                        for (channel_id, result) in results {
                            match result {
                                Ok(_) => {
                                    log::info!("Alert {} sent to channel {}", alert.id, channel_id)
                                }
                                Err(e) => log::error!(
                                    "Failed to send alert {} to channel {}: {}",
                                    alert.id,
                                    channel_id,
                                    e
                                ),
                            }
                        }
                    }
                } else if rule.auto_resolve {
                    // Check if alert exists and should be resolved
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();

                    if let Some(alert) = alerts.get(&alert_id) {
                        if now - alert.started_at >= rule.resolve_after_secs {
                            // Resolve the alert - just log for now
                            log::info!(
                                "Auto-resolving alert {} after {} seconds",
                                alert_id,
                                rule.resolve_after_secs
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn extract_metric_value(metrics: &ServerMetrics, metric_type: &MetricType) -> Option<f64> {
        match metric_type {
            MetricType::CpuUsage => Some(metrics.cpu_usage),
            MetricType::CpuUser => Some(metrics.cpu_user),
            MetricType::CpuSystem => Some(metrics.cpu_system),
            MetricType::CpuIowait => Some(metrics.cpu_iowait),
            MetricType::CpuSteal => Some(metrics.cpu_steal),
            MetricType::CpuLoad1 => Some(metrics.cpu_load1),
            MetricType::CpuLoad5 => Some(metrics.cpu_load5),
            MetricType::CpuLoad15 => Some(metrics.cpu_load15),
            MetricType::MemoryUsage => Some(metrics.memory_usage_percent()),
            MetricType::MemoryUsed => Some(metrics.memory_used as f64),
            MetricType::MemoryTotal => Some(metrics.memory_total as f64),
            MetricType::MemoryFree => Some(metrics.memory_free as f64),
            MetricType::SwapUsage => Some(metrics.swap_usage_percent()),
            MetricType::DiskUsage => Some(metrics.disk_usage_percent()),
            MetricType::DiskUsed => Some(metrics.disk_used as f64),
            MetricType::DiskFree => Some(metrics.disk_free as f64),
            MetricType::DiskIoUtil => Some(metrics.disk_io_util),
            MetricType::ProcessCount => Some(metrics.process_count as f64),
            MetricType::ProcessRunning => Some(metrics.process_running as f64),
            MetricType::ProcessZombie => Some(metrics.process_zombie as f64),
            MetricType::CpuTemp => metrics.cpu_temp,
            _ => None,
        }
    }

    fn condition_description(condition: &AlertCondition) -> &'static str {
        match condition {
            AlertCondition::GreaterThan => "above threshold",
            AlertCondition::GreaterThanOrEqual => "at or above threshold",
            AlertCondition::LessThan => "below threshold",
            AlertCondition::LessThanOrEqual => "at or below threshold",
            AlertCondition::Equal => "equal to threshold",
            AlertCondition::NotEqual => "not equal to threshold",
            AlertCondition::Between { .. } => "within range",
            AlertCondition::Outside { .. } => "outside range",
            AlertCondition::Anomaly => "anomalous",
            AlertCondition::NoData => "reporting no data",
        }
    }

    async fn send_notifications(&self, alert: &Alert, channels: &[String]) {
        let payload = super::notifications::NotificationPayload::from_alert(alert);
        let manager = self.notification_manager.read().await;

        let results = manager.send_to_channels(channels, &payload, 60).await;

        for (channel_id, result) in results {
            match result {
                Ok(_) => log::info!("Alert {} sent to channel {}", alert.id, channel_id),
                Err(e) => log::error!(
                    "Failed to send alert {} to channel {}: {}",
                    alert.id,
                    channel_id,
                    e
                ),
            }
        }
    }

    /// Add a notification channel to the alert engine
    pub async fn add_notification_channel(
        &self,
        channel: super::notifications::NotificationChannel,
    ) -> Result<(), super::MonitoringError> {
        let mut manager = self.notification_manager.write().await;
        manager
            .add_channel(&channel)
            .map_err(|e| super::MonitoringError::Config(e.to_string()))?;

        let mut channels = self.notification_channels.write().await;
        channels.push(NotificationChannel {
            id: channel.id,
            name: channel.name,
            channel_type: match channel.channel_type {
                super::notifications::NotificationChannelType::Email => {
                    NotificationChannelType::Email
                }
                super::notifications::NotificationChannelType::Slack => {
                    NotificationChannelType::Slack
                }
                super::notifications::NotificationChannelType::Discord => {
                    NotificationChannelType::Discord
                }
                super::notifications::NotificationChannelType::Webhook => {
                    NotificationChannelType::Webhook
                }
                super::notifications::NotificationChannelType::PagerDuty => {
                    NotificationChannelType::PagerDuty
                }
                super::notifications::NotificationChannelType::Opsgenie => {
                    NotificationChannelType::Opsgenie
                }
                super::notifications::NotificationChannelType::Telegram => {
                    NotificationChannelType::Telegram
                }
                super::notifications::NotificationChannelType::Sms => NotificationChannelType::Sms,
                super::notifications::NotificationChannelType::PushNotification => {
                    NotificationChannelType::PushNotification
                }
                super::notifications::NotificationChannelType::DesktopNotification => {
                    NotificationChannelType::DesktopNotification
                }
            },
            config: channel.config,
            enabled: channel.enabled,
            rate_limit_per_minute: channel.rate_limit_per_minute,
            created_at: channel.created_at,
            updated_at: channel.updated_at,
        });

        Ok(())
    }
}

/// Alert statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertStats {
    pub total_active: usize,
    pub by_severity: HashMap<AlertSeverity, usize>,
    pub by_status: HashMap<AlertStatus, usize>,
    pub today_count: usize,
    pub this_week_count: usize,
    pub avg_resolution_time_minutes: f64,
    pub top_servers: Vec<ServerAlertCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerAlertCount {
    pub server_id: String,
    pub server_name: String,
    pub alert_count: usize,
}

/// Alert group for organizing alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertGroup {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub rule_ids: Vec<String>,
    pub server_ids: Vec<String>,
    pub notification_channel_id: Option<String>,
    pub routing_key: Option<String>,
}

/// Predictive alert for proactive monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveAlert {
    pub id: String,
    pub server_id: String,
    pub predicted_issue: String,
    pub confidence: f64,
    pub predicted_time: u64,
    pub severity: AlertSeverity,
    pub recommended_action: String,
    pub related_metrics: Vec<MetricType>,
}

/// Get description for alert condition (public version)
pub fn condition_description(condition: &AlertCondition) -> &'static str {
    match condition {
        AlertCondition::GreaterThan => "above threshold",
        AlertCondition::GreaterThanOrEqual => "at or above threshold",
        AlertCondition::LessThan => "below threshold",
        AlertCondition::LessThanOrEqual => "at or below threshold",
        AlertCondition::Equal => "equal to threshold",
        AlertCondition::NotEqual => "not equal to threshold",
        AlertCondition::Between { .. } => "within range",
        AlertCondition::Outside { .. } => "outside range",
        AlertCondition::Anomaly => "anomalous",
        AlertCondition::NoData => "reporting no data",
    }
}
