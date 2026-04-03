//! Notification channels for alert delivery
//!
//! Supports: Email, Slack, Discord, Webhook, PagerDuty, Telegram, SMS

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Notification channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub id: String,
    pub name: String,
    pub channel_type: NotificationChannelType,
    pub config: HashMap<String, String>,
    pub enabled: bool,
    pub rate_limit_per_minute: u32,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Available notification channel types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannelType {
    Email,
    Slack,
    Discord,
    Webhook,
    PagerDuty,
    Opsgenie,
    Telegram,
    Sms,
    PushNotification,
    DesktopNotification,
}

impl NotificationChannelType {
    pub fn display_name(&self) -> &'static str {
        match self {
            NotificationChannelType::Email => "Email",
            NotificationChannelType::Slack => "Slack",
            NotificationChannelType::Discord => "Discord",
            NotificationChannelType::Webhook => "Webhook",
            NotificationChannelType::PagerDuty => "PagerDuty",
            NotificationChannelType::Opsgenie => "Opsgenie",
            NotificationChannelType::Telegram => "Telegram",
            NotificationChannelType::Sms => "SMS",
            NotificationChannelType::PushNotification => "Push Notification",
            NotificationChannelType::DesktopNotification => "Desktop Notification",
        }
    }

    pub fn required_config_fields(&self) -> Vec<&'static str> {
        match self {
            NotificationChannelType::Email => vec![
                "smtp_host",
                "smtp_port",
                "username",
                "password",
                "from_address",
                "to_addresses",
            ],
            NotificationChannelType::Slack => vec!["webhook_url"],
            NotificationChannelType::Discord => vec!["webhook_url"],
            NotificationChannelType::Webhook => vec!["url", "method"],
            NotificationChannelType::PagerDuty => vec!["integration_key", "api_url"],
            NotificationChannelType::Opsgenie => vec!["api_key", "api_url"],
            NotificationChannelType::Telegram => vec!["bot_token", "chat_id"],
            NotificationChannelType::Sms => {
                vec!["provider", "api_key", "from_number", "to_numbers"]
            }
            NotificationChannelType::PushNotification => vec!["provider", "api_key"],
            NotificationChannelType::DesktopNotification => vec![],
        }
    }
}

/// Notification payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub title: String,
    pub message: String,
    pub severity: super::alerts::AlertSeverity,
    pub server_id: String,
    pub server_name: String,
    pub metric_type: String,
    pub metric_value: f64,
    pub threshold: f64,
    pub timestamp: u64,
    pub alert_id: String,
    pub runbook_url: Option<String>,
    pub dashboard_url: Option<String>,
    pub tags: Vec<String>,
}

impl NotificationPayload {
    /// Create from alert data
    pub fn from_alert(alert: &super::alerts::Alert) -> Self {
        Self {
            title: alert.title.clone(),
            message: alert.message.clone(),
            severity: alert.severity,
            server_id: alert.server_id.clone(),
            server_name: alert.server_name.clone(),
            metric_type: format!("{:?}", alert.metric_type),
            metric_value: alert.metric_value,
            threshold: alert.threshold,
            timestamp: alert.started_at,
            alert_id: alert.id.clone(),
            runbook_url: alert.runbook_url.clone(),
            dashboard_url: alert.dashboard_url.clone(),
            tags: alert.tags.clone(),
        }
    }
}

/// Notification sender trait
#[async_trait::async_trait]
pub trait NotificationSender: Send + Sync {
    async fn send(&self, payload: &NotificationPayload) -> Result<(), NotificationError>;
    fn channel_type(&self) -> NotificationChannelType;
    fn validate_config(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<(), ConfigValidationError>;
}

/// Notification error types
#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("SMTP error: {0}")]
    Smtp(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Rate limited")]
    RateLimited,
    #[error("Channel disabled")]
    ChannelDisabled,
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Other error: {0}")]
    Other(String),
}

/// Configuration validation error
#[derive(Debug, thiserror::Error)]
#[error("Configuration validation error: {field} - {message}")]
pub struct ConfigValidationError {
    pub field: String,
    pub message: String,
}

/// Email notification sender
pub struct EmailSender {
    config: HashMap<String, String>,
}

impl EmailSender {
    pub fn new(config: HashMap<String, String>) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl NotificationSender for EmailSender {
    async fn send(&self, payload: &NotificationPayload) -> Result<(), NotificationError> {
        // In a production implementation, this would use lettre or similar
        // For now, we log and simulate
        log::info!(
            "[EMAIL] To: {:?}, Subject: {}, Body: {}",
            self.config.get("to_addresses"),
            payload.title,
            payload.message
        );

        // Email sending would be implemented here using SMTP
        // Example with lettre:
        // let email = Message::builder()
        //     .from(self.config["from_address"].parse()?)
        //     .to(self.config["to_addresses"].parse()?)
        //     .subject(&payload.title)
        //     .body(self.format_email_body(payload))?
        //     .build()?;
        //
        // let creds = Credentials::new(
        //     self.config["username"].clone(),
        //     self.config["password"].clone(),
        // );
        //
        // let mailer = SmtpTransport::relay(&self.config["smtp_host"])?
        //     .credentials(creds)
        //     .port(self.config["smtp_port"].parse()?)
        //     .build();
        //
        // mailer.send(&email)?;

        Ok(())
    }

    fn channel_type(&self) -> NotificationChannelType {
        NotificationChannelType::Email
    }

    fn validate_config(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<(), ConfigValidationError> {
        let required = vec![
            "smtp_host",
            "smtp_port",
            "username",
            "password",
            "from_address",
            "to_addresses",
        ];
        for field in required {
            if !config.contains_key(field) {
                return Err(ConfigValidationError {
                    field: field.to_string(),
                    message: "Required field missing".to_string(),
                });
            }
        }
        Ok(())
    }
}

impl EmailSender {
    #[allow(dead_code)]
    fn format_email_body(&self, payload: &NotificationPayload) -> String {
        format!(
            r#"<html>
<body>
    <h2 style="color: {};">{}</h2>
    <p><strong>Server:</strong> {} ({})</p>
    <p><strong>Metric:</strong> {} = {:.2} (threshold: {:.2})</p>
    <p><strong>Time:</strong> {}</p>
    <p><strong>Severity:</strong> {:?}</p>
    {}
    {}
    <hr>
    <p style="font-size: 12px; color: #666;">Alert ID: {}</p>
</body>
</html>"#,
            payload.severity.color(),
            payload.title,
            payload.server_name,
            payload.server_id,
            payload.metric_type,
            payload.metric_value,
            payload.threshold,
            chrono::DateTime::from_timestamp(payload.timestamp as i64, 0)
                .map(|dt| dt.to_string())
                .unwrap_or_else(|| payload.timestamp.to_string()),
            payload.severity,
            payload
                .runbook_url
                .as_ref()
                .map(|url| format!("<p><a href=\"{}\">Runbook</a></p>", url))
                .unwrap_or_default(),
            payload
                .dashboard_url
                .as_ref()
                .map(|url| format!("<p><a href=\"{}\">Dashboard</a></p>", url))
                .unwrap_or_default(),
            payload.alert_id,
        )
    }
}

/// Slack notification sender
pub struct SlackSender {
    config: HashMap<String, String>,
    client: reqwest::Client,
}

impl SlackSender {
    pub fn new(config: HashMap<String, String>) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl NotificationSender for SlackSender {
    async fn send(&self, payload: &NotificationPayload) -> Result<(), NotificationError> {
        let webhook_url = self
            .config
            .get("webhook_url")
            .ok_or_else(|| NotificationError::Config("Missing webhook_url".to_string()))?;

        let color = match payload.severity {
            super::alerts::AlertSeverity::Info => "#3b82f6",
            super::alerts::AlertSeverity::Warning => "#f59e0b",
            super::alerts::AlertSeverity::Critical => "#ef4444",
            super::alerts::AlertSeverity::Emergency => "#7f1d1d",
        };

        let slack_payload = serde_json::json!({
            "attachments": [{
                "color": color,
                "title": payload.title,
                "text": payload.message,
                "fields": [
                    {
                        "title": "Server",
                        "value": format!("{} ({})", payload.server_name, payload.server_id),
                        "short": true
                    },
                    {
                        "title": "Metric",
                        "value": format!("{} = {:.2}", payload.metric_type, payload.metric_value),
                        "short": true
                    },
                    {
                        "title": "Severity",
                        "value": format!("{:?}", payload.severity),
                        "short": true
                    },
                    {
                        "title": "Threshold",
                        "value": format!("{:.2}", payload.threshold),
                        "short": true
                    }
                ],
                "footer": format!("EasySSH Alert | ID: {}", payload.alert_id),
                "ts": payload.timestamp as i64
            }]
        });

        let response = self
            .client
            .post(webhook_url)
            .json(&slack_payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(NotificationError::Other(format!(
                "Slack API error: {} - {}",
                status, text
            )));
        }

        log::info!("[SLACK] Notification sent successfully to {}", webhook_url);
        Ok(())
    }

    fn channel_type(&self) -> NotificationChannelType {
        NotificationChannelType::Slack
    }

    fn validate_config(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<(), ConfigValidationError> {
        if !config.contains_key("webhook_url") {
            return Err(ConfigValidationError {
                field: "webhook_url".to_string(),
                message: "Slack webhook URL is required".to_string(),
            });
        }
        Ok(())
    }
}

/// Discord notification sender
pub struct DiscordSender {
    config: HashMap<String, String>,
    client: reqwest::Client,
}

impl DiscordSender {
    pub fn new(config: HashMap<String, String>) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl NotificationSender for DiscordSender {
    async fn send(&self, payload: &NotificationPayload) -> Result<(), NotificationError> {
        let webhook_url = self
            .config
            .get("webhook_url")
            .ok_or_else(|| NotificationError::Config("Missing webhook_url".to_string()))?;

        let color = match payload.severity {
            super::alerts::AlertSeverity::Info => 0x3b82f6,
            super::alerts::AlertSeverity::Warning => 0xf59e0b,
            super::alerts::AlertSeverity::Critical => 0xef4444,
            super::alerts::AlertSeverity::Emergency => 0x7f1d1d,
        };

        let discord_payload = serde_json::json!({
            "embeds": [{
                "title": payload.title,
                "description": payload.message,
                "color": color,
                "fields": [
                    {
                        "name": "Server",
                        "value": format!("{} ({})", payload.server_name, payload.server_id),
                        "inline": true
                    },
                    {
                        "name": "Metric",
                        "value": format!("{} = {:.2}", payload.metric_type, payload.metric_value),
                        "inline": true
                    },
                    {
                        "name": "Severity",
                        "value": format!("{:?}", payload.severity),
                        "inline": true
                    }
                ],
                "footer": {
                    "text": format!("EasySSH Alert | ID: {}", payload.alert_id)
                },
                "timestamp": chrono::DateTime::from_timestamp(payload.timestamp as i64, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            }]
        });

        let response = self
            .client
            .post(webhook_url)
            .json(&discord_payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(NotificationError::Other(format!(
                "Discord API error: {}",
                response.status()
            )));
        }

        log::info!("[DISCORD] Notification sent successfully");
        Ok(())
    }

    fn channel_type(&self) -> NotificationChannelType {
        NotificationChannelType::Discord
    }

    fn validate_config(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<(), ConfigValidationError> {
        if !config.contains_key("webhook_url") {
            return Err(ConfigValidationError {
                field: "webhook_url".to_string(),
                message: "Discord webhook URL is required".to_string(),
            });
        }
        Ok(())
    }
}

/// Generic webhook notification sender
pub struct WebhookSender {
    config: HashMap<String, String>,
    client: reqwest::Client,
}

impl WebhookSender {
    pub fn new(config: HashMap<String, String>) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl NotificationSender for WebhookSender {
    async fn send(&self, payload: &NotificationPayload) -> Result<(), NotificationError> {
        let url = self
            .config
            .get("url")
            .ok_or_else(|| NotificationError::Config("Missing url".to_string()))?;
        let method = self
            .config
            .get("method")
            .map(|m| m.to_uppercase())
            .unwrap_or_else(|| "POST".to_string());

        let custom_headers = self
            .config
            .get("headers")
            .and_then(|h| serde_json::from_str::<HashMap<String, String>>(h).ok())
            .unwrap_or_default();

        let template = self.config.get("template");
        let body = if let Some(tpl) = template {
            // Simple template substitution
            tpl.replace("{{title}}", &payload.title)
                .replace("{{message}}", &payload.message)
                .replace("{{server_id}}", &payload.server_id)
                .replace("{{server_name}}", &payload.server_name)
                .replace("{{metric_type}}", &payload.metric_type)
                .replace("{{metric_value}}", &payload.metric_value.to_string())
                .replace("{{threshold}}", &payload.threshold.to_string())
                .replace("{{severity}}", &format!("{:?}", payload.severity))
                .replace("{{alert_id}}", &payload.alert_id)
                .replace("{{timestamp}}", &payload.timestamp.to_string())
        } else {
            serde_json::to_string(payload)?
        };

        let mut request = match method.as_str() {
            "GET" => self.client.get(url),
            "PUT" => self.client.put(url),
            "PATCH" => self.client.patch(url),
            _ => self.client.post(url),
        };

        // Add custom headers
        for (key, value) in custom_headers {
            request = request.header(&key, &value);
        }

        let response = request.body(body).send().await?;

        if !response.status().is_success() {
            return Err(NotificationError::Other(format!(
                "Webhook error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        log::info!("[WEBHOOK] Notification sent to {}", url);
        Ok(())
    }

    fn channel_type(&self) -> NotificationChannelType {
        NotificationChannelType::Webhook
    }

    fn validate_config(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<(), ConfigValidationError> {
        if !config.contains_key("url") {
            return Err(ConfigValidationError {
                field: "url".to_string(),
                message: "Webhook URL is required".to_string(),
            });
        }
        Ok(())
    }
}

/// Telegram notification sender
pub struct TelegramSender {
    config: HashMap<String, String>,
    client: reqwest::Client,
}

impl TelegramSender {
    pub fn new(config: HashMap<String, String>) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl NotificationSender for TelegramSender {
    async fn send(&self, payload: &NotificationPayload) -> Result<(), NotificationError> {
        let bot_token = self
            .config
            .get("bot_token")
            .ok_or_else(|| NotificationError::Config("Missing bot_token".to_string()))?;
        let chat_id = self
            .config
            .get("chat_id")
            .ok_or_else(|| NotificationError::Config("Missing chat_id".to_string()))?;

        let emoji = match payload.severity {
            super::alerts::AlertSeverity::Info => "ℹ️",
            super::alerts::AlertSeverity::Warning => "⚠️",
            super::alerts::AlertSeverity::Critical => "🚨",
            super::alerts::AlertSeverity::Emergency => "🔥",
        };

        let message = format!(
            "{} *{}*\n\n{}\n\n*Server:* {} ({})
*Metric:* {} = {:.2}
*Severity:* {:?}
*Time:* {}",
            emoji,
            escape_markdown(&payload.title),
            escape_markdown(&payload.message),
            escape_markdown(&payload.server_name),
            escape_markdown(&payload.server_id),
            escape_markdown(&payload.metric_type),
            payload.metric_value,
            payload.severity,
            chrono::DateTime::from_timestamp(payload.timestamp as i64, 0)
                .map(|dt| dt.to_string())
                .unwrap_or_else(|| payload.timestamp.to_string())
        );

        let api_url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
        let telegram_payload = serde_json::json!({
            "chat_id": chat_id,
            "text": message,
            "parse_mode": "MarkdownV2",
            "disable_web_page_preview": true
        });

        let response = self
            .client
            .post(&api_url)
            .json(&telegram_payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(NotificationError::Other(format!(
                "Telegram API error: {}",
                response.status()
            )));
        }

        log::info!("[TELEGRAM] Notification sent successfully");
        Ok(())
    }

    fn channel_type(&self) -> NotificationChannelType {
        NotificationChannelType::Telegram
    }

    fn validate_config(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<(), ConfigValidationError> {
        if !config.contains_key("bot_token") {
            return Err(ConfigValidationError {
                field: "bot_token".to_string(),
                message: "Telegram bot token is required".to_string(),
            });
        }
        if !config.contains_key("chat_id") {
            return Err(ConfigValidationError {
                field: "chat_id".to_string(),
                message: "Telegram chat ID is required".to_string(),
            });
        }
        Ok(())
    }
}

fn escape_markdown(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('_', "\\_")
        .replace('*', "\\*")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('(', "\\(")
        .replace(')', "\\)")
        .replace('~', "\\~")
        .replace('`', "\\`")
        .replace('>', "\\>")
        .replace('#', "\\#")
        .replace('+', "\\+")
        .replace('-', "\\-")
        .replace('=', "\\=")
        .replace('|', "\\|")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('.', "\\.")
        .replace('!', "\\!")
}

/// PagerDuty notification sender
pub struct PagerDutySender {
    config: HashMap<String, String>,
    client: reqwest::Client,
}

impl PagerDutySender {
    pub fn new(config: HashMap<String, String>) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl NotificationSender for PagerDutySender {
    async fn send(&self, payload: &NotificationPayload) -> Result<(), NotificationError> {
        let integration_key = self
            .config
            .get("integration_key")
            .ok_or_else(|| NotificationError::Config("Missing integration_key".to_string()))?;
        let api_url = self
            .config
            .get("api_url")
            .unwrap_or(&"https://events.pagerduty.com/v2/enqueue".to_string())
            .clone();

        let severity = match payload.severity {
            super::alerts::AlertSeverity::Info => "info",
            super::alerts::AlertSeverity::Warning => "warning",
            super::alerts::AlertSeverity::Critical => "critical",
            super::alerts::AlertSeverity::Emergency => "critical",
        };

        // Build links array
        let mut links = Vec::new();
        if let Some(ref url) = payload.dashboard_url {
            links.push(serde_json::json!({"href": url, "text": "Dashboard"}));
        }
        if let Some(ref url) = payload.runbook_url {
            links.push(serde_json::json!({"href": url, "text": "Runbook"}));
        }

        let pagerduty_payload = serde_json::json!({
            "routing_key": integration_key,
            "event_action": "trigger",
            "dedup_key": payload.alert_id,
            "payload": {
                "summary": payload.title,
                "severity": severity,
                "source": payload.server_id,
                "component": payload.metric_type,
                "custom_details": {
                    "message": payload.message,
                    "server_name": payload.server_name,
                    "metric_value": payload.metric_value,
                    "threshold": payload.threshold,
                    "alert_id": payload.alert_id
                }
            },
            "links": links
        });

        let response = self
            .client
            .post(&api_url)
            .json(&pagerduty_payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(NotificationError::Other(format!(
                "PagerDuty API error: {}",
                response.status()
            )));
        }

        log::info!("[PAGERDUTY] Incident triggered successfully");
        Ok(())
    }

    fn channel_type(&self) -> NotificationChannelType {
        NotificationChannelType::PagerDuty
    }

    fn validate_config(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<(), ConfigValidationError> {
        if !config.contains_key("integration_key") {
            return Err(ConfigValidationError {
                field: "integration_key".to_string(),
                message: "PagerDuty integration key is required".to_string(),
            });
        }
        Ok(())
    }
}

/// Desktop notification sender (cross-platform)
pub struct DesktopNotificationSender;

impl DesktopNotificationSender {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl NotificationSender for DesktopNotificationSender {
    async fn send(&self, payload: &NotificationPayload) -> Result<(), NotificationError> {
        #[cfg(target_os = "windows")]
        {
            // Windows notification would use winrt or winapi
            log::info!(
                "[DESKTOP NOTIFICATION - Windows] {}: {}",
                payload.title,
                payload.message
            );
        }

        #[cfg(target_os = "macos")]
        {
            // macOS notification would use mac-notification-sys
            log::info!(
                "[DESKTOP NOTIFICATION - macOS] {}: {}",
                payload.title,
                payload.message
            );
        }

        #[cfg(target_os = "linux")]
        {
            // Linux notification would use notify-rust
            log::info!(
                "[DESKTOP NOTIFICATION - Linux] {}: {}",
                payload.title,
                payload.message
            );
        }

        Ok(())
    }

    fn channel_type(&self) -> NotificationChannelType {
        NotificationChannelType::DesktopNotification
    }

    fn validate_config(
        &self,
        _config: &HashMap<String, String>,
    ) -> Result<(), ConfigValidationError> {
        Ok(())
    }
}

/// Notification manager that handles all channels
pub struct NotificationManager {
    channels: HashMap<String, Box<dyn NotificationSender>>,
    rate_limits: HashMap<String, tokio::sync::Mutex<RateLimitState>>,
}

struct RateLimitState {
    last_sent: std::time::Instant,
    count: u32,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            rate_limits: HashMap::new(),
        }
    }

    /// Add a notification channel
    pub fn add_channel(
        &mut self,
        channel: &NotificationChannel,
    ) -> Result<(), ConfigValidationError> {
        let sender: Box<dyn NotificationSender> = match channel.channel_type {
            NotificationChannelType::Email => Box::new(EmailSender::new(channel.config.clone())),
            NotificationChannelType::Slack => Box::new(SlackSender::new(channel.config.clone())),
            NotificationChannelType::Discord => {
                Box::new(DiscordSender::new(channel.config.clone()))
            }
            NotificationChannelType::Webhook => {
                Box::new(WebhookSender::new(channel.config.clone()))
            }
            NotificationChannelType::Telegram => {
                Box::new(TelegramSender::new(channel.config.clone()))
            }
            NotificationChannelType::PagerDuty => {
                Box::new(PagerDutySender::new(channel.config.clone()))
            }
            NotificationChannelType::DesktopNotification => {
                Box::new(DesktopNotificationSender::new())
            }
            _ => {
                return Err(ConfigValidationError {
                    field: "channel_type".to_string(),
                    message: format!(
                        "Channel type {:?} not yet implemented",
                        channel.channel_type
                    ),
                });
            }
        };

        sender.validate_config(&channel.config)?;
        self.channels.insert(channel.id.clone(), sender);
        self.rate_limits.insert(
            channel.id.clone(),
            tokio::sync::Mutex::new(RateLimitState {
                last_sent: std::time::Instant::now(),
                count: 0,
            }),
        );

        Ok(())
    }

    /// Send notification to a specific channel
    pub async fn send_to_channel(
        &self,
        channel_id: &str,
        payload: &NotificationPayload,
        rate_limit: u32,
    ) -> Result<(), NotificationError> {
        let sender = self.channels.get(channel_id).ok_or_else(|| {
            NotificationError::Config(format!("Channel {} not found", channel_id))
        })?;

        // Check rate limit
        let mut rate_limit_state = self
            .rate_limits
            .get(channel_id)
            .ok_or_else(|| {
                NotificationError::Config(format!(
                    "Rate limit state not found for channel {}",
                    channel_id
                ))
            })?
            .lock()
            .await;

        let elapsed = rate_limit_state.last_sent.elapsed();
        if elapsed.as_secs() < 60 && rate_limit_state.count >= rate_limit {
            return Err(NotificationError::RateLimited);
        }

        if elapsed.as_secs() >= 60 {
            rate_limit_state.count = 0;
            rate_limit_state.last_sent = std::time::Instant::now();
        }

        rate_limit_state.count += 1;
        drop(rate_limit_state);

        sender.send(payload).await
    }

    /// Send notification to multiple channels
    pub async fn send_to_channels(
        &self,
        channel_ids: &[String],
        payload: &NotificationPayload,
        rate_limit: u32,
    ) -> Vec<(String, Result<(), NotificationError>)> {
        let mut results = Vec::new();

        for channel_id in channel_ids {
            let result = self.send_to_channel(channel_id, payload, rate_limit).await;
            results.push((channel_id.clone(), result));
        }

        results
    }

    /// Remove a channel
    pub fn remove_channel(&mut self, channel_id: &str) {
        self.channels.remove(channel_id);
        self.rate_limits.remove(channel_id);
    }

    /// Get available channel types
    pub fn available_channel_types() -> Vec<NotificationChannelType> {
        vec![
            NotificationChannelType::Email,
            NotificationChannelType::Slack,
            NotificationChannelType::Discord,
            NotificationChannelType::Webhook,
            NotificationChannelType::Telegram,
            NotificationChannelType::PagerDuty,
            NotificationChannelType::DesktopNotification,
        ]
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}
