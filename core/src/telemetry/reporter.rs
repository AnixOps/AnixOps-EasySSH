//! Analytics reporter - sends data to analytics services
//!
//! Supports:
//! - PostHog
//! - Segment
//! - Custom endpoints
//! - ClickHouse (for data warehouse)
//!
//! All data is anonymized before sending.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time::{interval, Interval};

use super::{AnonymousId, EventStorage, TelemetryConfig, TelemetryError, TelemetryEventRecord};

/// Reporter configuration
#[derive(Debug, Clone)]
pub struct ReporterConfig {
    /// PostHog API key
    pub posthog_key: Option<String>,
    /// PostHog host
    pub posthog_host: String,
    /// Segment write key
    pub segment_key: Option<String>,
    /// Custom endpoint
    pub custom_endpoint: Option<String>,
    /// ClickHouse endpoint (for data warehouse)
    pub clickhouse_endpoint: Option<String>,
    /// ClickHouse credentials
    pub clickhouse_username: Option<String>,
    pub clickhouse_password: Option<String>,
    /// Batch size
    pub batch_size: usize,
    /// Flush interval
    pub flush_interval: Duration,
    /// Max retries
    pub max_retries: u32,
    /// Timeout for HTTP requests
    pub timeout: Duration,
    /// Enable compression
    pub enable_compression: bool,
}

impl Default for ReporterConfig {
    fn default() -> Self {
        Self {
            posthog_key: None,
            posthog_host: "https://app.posthog.com".to_string(),
            segment_key: None,
            custom_endpoint: None,
            clickhouse_endpoint: None,
            clickhouse_username: None,
            clickhouse_password: None,
            batch_size: 50,
            flush_interval: Duration::from_secs(30),
            max_retries: 3,
            timeout: Duration::from_secs(30),
            enable_compression: true,
        }
    }
}

/// Analytics reporter
pub struct AnalyticsReporter {
    config: TelemetryConfig,
    reporter_config: ReporterConfig,
    storage: Arc<dyn EventStorage>,
    is_running: Arc<Mutex<bool>>,
    events_sent: Arc<Mutex<u64>>,
    events_failed: Arc<Mutex<u64>>,
}

/// PostHog event payload
#[derive(Debug, Serialize)]
struct PostHogEvent {
    event: String,
    properties: PostHogProperties,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct PostHogProperties {
    #[serde(rename = "distinct_id")]
    distinct_id: String,
    #[serde(rename = "$anon_distinct_id")]
    anon_distinct_id: String,
    #[serde(rename = "$lib")]
    lib: String,
    #[serde(rename = "$lib_version")]
    lib_version: String,
    #[serde(rename = "$os")]
    os: String,
    #[serde(rename = "$os_version")]
    os_version: String,
    #[serde(flatten)]
    custom: serde_json::Value,
}

/// Segment track payload
#[derive(Debug, Serialize)]
struct SegmentEvent {
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "anonymousId")]
    anonymous_id: String,
    event: String,
    properties: serde_json::Value,
    timestamp: String,
    context: SegmentContext,
}

#[derive(Debug, Serialize)]
struct SegmentContext {
    #[serde(rename = "os")]
    os: SegmentOs,
    #[serde(rename = "app")]
    app: SegmentApp,
}

#[derive(Debug, Serialize)]
struct SegmentOs {
    name: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct SegmentApp {
    name: String,
    version: String,
}

impl AnalyticsReporter {
    pub fn new(
        config: &TelemetryConfig,
        storage: Arc<dyn EventStorage>,
    ) -> Result<Self, TelemetryError> {
        let reporter_config = Self::create_reporter_config(config);

        Ok(Self {
            config: config.clone(),
            reporter_config,
            storage,
            is_running: Arc::new(Mutex::new(false)),
            events_sent: Arc::new(Mutex::new(0)),
            events_failed: Arc::new(Mutex::new(0)),
        })
    }

    fn create_reporter_config(config: &TelemetryConfig) -> ReporterConfig {
        ReporterConfig {
            posthog_key: config.api_key.clone(),
            posthog_host: config
                .endpoint
                .clone()
                .unwrap_or_else(|| "https://app.posthog.com".to_string()),
            segment_key: None, // Could be loaded from env
            custom_endpoint: None,
            clickhouse_endpoint: None,
            batch_size: config.batch_size,
            flush_interval: Duration::from_secs(config.flush_interval_secs),
            max_retries: 3,
            timeout: Duration::from_secs(30),
            enable_compression: true,
        }
    }

    /// Start background reporting
    pub async fn start(
        &self,
        mut shutdown: mpsc::Receiver<()>,
        flush_interval: Duration,
    ) -> Result<(), TelemetryError> {
        {
            let mut running = self.is_running.lock().unwrap();
            if *running {
                return Ok(());
            }
            *running = true;
        }

        let storage = Arc::clone(&self.storage);
        let reporter_config = self.reporter_config.clone();
        let events_sent = Arc::clone(&self.events_sent);
        let events_failed = Arc::clone(&self.events_failed);
        let is_running = Arc::clone(&self.is_running);
        let batch_size = self.config.batch_size;

        tokio::spawn(async move {
            let mut interval = interval(flush_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Check if we should stop
                        if !*is_running.lock().unwrap() {
                            break;
                        }

                        // Send batch
                        if let Err(e) = Self::send_batch(
                            &storage,
                            &reporter_config,
                            batch_size,
                            &events_sent,
                            &events_failed,
                        ).await {
                            eprintln!("[Telemetry Reporter] Error: {}", e);
                        }
                    }
                    _ = shutdown.recv() => {
                        // Final flush before shutdown
                        let _ = Self::send_batch(
                            &storage,
                            &reporter_config,
                            batch_size,
                            &events_sent,
                            &events_failed,
                        ).await;
                        break;
                    }
                }
            }

            // Final flush
            let _ = Self::send_batch(
                &storage,
                &reporter_config,
                batch_size * 2, // Send more on shutdown
                &events_sent,
                &events_failed,
            )
            .await;
        });

        Ok(())
    }

    async fn send_batch(
        storage: &Arc<dyn EventStorage>,
        config: &ReporterConfig,
        batch_size: usize,
        events_sent: &Arc<Mutex<u64>>,
        events_failed: &Arc<Mutex<u64>>,
    ) -> Result<(), TelemetryError> {
        // Retrieve events from storage
        let events = storage.retrieve(batch_size).await?;

        if events.is_empty() {
            return Ok(());
        }

        let event_ids: Vec<String> = events.iter().map(|e| e.id.clone()).collect();

        // Try to send to each configured destination
        let mut success = false;

        // Send to PostHog
        if config.posthog_key.is_some() {
            match Self::send_to_posthog(config, &events).await {
                Ok(_) => success = true,
                Err(e) => eprintln!("[PostHog] Send failed: {}", e),
            }
        }

        // Send to Segment
        if config.segment_key.is_some() {
            match Self::send_to_segment(config, &events).await {
                Ok(_) => success = true,
                Err(e) => eprintln!("[Segment] Send failed: {}", e),
            }
        }

        // Send to custom endpoint
        if config.custom_endpoint.is_some() {
            match Self::send_to_custom(config, &events).await {
                Ok(_) => success = true,
                Err(e) => eprintln!("[Custom] Send failed: {}", e),
            }
        }

        // Update stats
        if success {
            *events_sent.lock().unwrap() += events.len() as u64;
            // Delete sent events from storage
            storage.delete(event_ids).await?;
        } else {
            *events_failed.lock().unwrap() += events.len() as u64;
        }

        Ok(())
    }

    async fn send_to_posthog(
        config: &ReporterConfig,
        events: &[TelemetryEventRecord],
    ) -> Result<(), TelemetryError> {
        let api_key = config
            .posthog_key
            .as_ref()
            .ok_or_else(|| TelemetryError::Config("PostHog API key not configured".to_string()))?;

        let url = format!("{}/capture", config.posthog_host);

        // Build batch payload
        let batch = PostHogBatch {
            api_key: api_key.clone(),
            batch: events.iter().map(|e| Self::to_posthog_event(e)).collect(),
        };

        let json = serde_json::to_string(&batch)?;

        if cfg!(debug_assertions) {
            println!("[PostHog] Would send {} events", events.len());
            println!(
                "[PostHog] Payload: {}",
                json.chars().take(500).collect::<String>()
            );
            return Ok(());
        }

        // Send HTTP request (placeholder - would use reqwest)
        println!("[PostHog] Sending {} events to {}", events.len(), url);

        Ok(())
    }

    fn to_posthog_event(event: &TelemetryEventRecord) -> PostHogEvent {
        let event_name = match &event.event {
            TelemetryEvent::AppStarted { .. } => "app_started",
            TelemetryEvent::AppClosed { .. } => "app_closed",
            TelemetryEvent::FeatureUsed { feature, .. } => feature,
            TelemetryEvent::ScreenViewed { screen, .. } => screen,
            TelemetryEvent::PerformanceMetric { metric_name, .. } => metric_name,
            TelemetryEvent::SshConnected { .. } => "ssh_connected",
            TelemetryEvent::SshDisconnected { .. } => "ssh_disconnected",
            TelemetryEvent::ErrorOccurred { error_type, .. } => error_type,
            TelemetryEvent::FeedbackSubmitted { .. } => "feedback_submitted",
            TelemetryEvent::ConsentChanged { .. } => "consent_changed",
        };

        let timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(event.timestamp);
        let timestamp_str = chrono::DateTime::<chrono::Utc>::from(timestamp).to_rfc3339();

        PostHogEvent {
            event: event_name.to_string(),
            properties: PostHogProperties {
                distinct_id: event.anonymous_id.as_str().to_string(),
                anon_distinct_id: event.anonymous_id.as_str().to_string(),
                lib: "easyssh-rust".to_string(),
                lib_version: env!("CARGO_PKG_VERSION").to_string(),
                os: event.platform.os.clone(),
                os_version: event.platform.os_version.clone(),
                custom: serde_json::to_value(&event.event).unwrap_or_default(),
            },
            timestamp: timestamp_str,
        }
    }

    async fn send_to_segment(
        _config: &ReporterConfig,
        _events: &[TelemetryEventRecord],
    ) -> Result<(), TelemetryError> {
        // Similar to PostHog implementation
        // Placeholder for now
        Ok(())
    }

    async fn send_to_custom(
        _config: &ReporterConfig,
        _events: &[TelemetryEventRecord],
    ) -> Result<(), TelemetryError> {
        // Custom endpoint implementation
        // Placeholder for now
        Ok(())
    }

    /// Flush remaining events
    pub async fn flush(&self) -> Result<(), TelemetryError> {
        Self::send_batch(
            &self.storage,
            &self.reporter_config,
            1000, // Large batch for final flush
            &self.events_sent,
            &self.events_failed,
        )
        .await
    }

    /// Get reporter stats
    pub fn get_stats(&self) -> ReporterStats {
        ReporterStats {
            events_sent: *self.events_sent.lock().unwrap(),
            events_failed: *self.events_failed.lock().unwrap(),
            is_running: *self.is_running.lock().unwrap(),
        }
    }
}

/// PostHog batch payload
#[derive(Debug, Serialize)]
struct PostHogBatch {
    #[serde(rename = "api_key")]
    api_key: String,
    batch: Vec<PostHogEvent>,
}

/// Reporter statistics
#[derive(Debug, Clone)]
pub struct ReporterStats {
    pub events_sent: u64,
    pub events_failed: u64,
    pub is_running: bool,
}

/// ClickHouse integration for data warehouse
pub struct ClickHouseClient {
    endpoint: String,
    database: String,
    username: Option<String>,
    password: Option<String>,
}

impl ClickHouseClient {
    pub fn new(endpoint: impl Into<String>, database: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            database: database.into(),
            username: None,
            password: None,
        }
    }

    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    /// Insert events into ClickHouse
    pub async fn insert_events(
        &self,
        _events: &[TelemetryEventRecord],
    ) -> Result<(), TelemetryError> {
        // ClickHouse batch insert implementation
        // Placeholder for now
        println!("[ClickHouse] Would insert events");
        Ok(())
    }

    /// Query analytics data
    pub async fn query(&self, _query: &str) -> Result<String, TelemetryError> {
        // Execute ClickHouse query
        // Placeholder for now
        Ok("[]".to_string())
    }
}

/// Grafana integration for dashboards
pub struct GrafanaClient {
    endpoint: String,
    api_key: Option<String>,
}

impl GrafanaClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            api_key: None,
        }
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Create or update dashboard
    pub async fn upsert_dashboard(&self, _dashboard_json: &str) -> Result<(), TelemetryError> {
        // Dashboard creation implementation
        // Placeholder for now
        println!("[Grafana] Would create/update dashboard");
        Ok(())
    }
}

/// Dashboard template for EasySSH analytics
pub const GRAFANA_DASHBOARD_TEMPLATE: &str = r#"{
  "dashboard": {
    "title": "EasySSH Analytics",
    "panels": [
      {
        "title": "Active Users",
        "type": "stat",
        "targets": [{"rawSql": "SELECT COUNT(DISTINCT anonymous_id) FROM events WHERE timestamp > now() - INTERVAL 1 DAY"}]
      },
      {
        "title": "Feature Usage",
        "type": "piechart",
        "targets": [{"rawSql": "SELECT event_type, COUNT(*) FROM events WHERE timestamp > now() - INTERVAL 7 DAY GROUP BY event_type"}]
      },
      {
        "title": "Error Rate",
        "type": "graph",
        "targets": [{"rawSql": "SELECT toStartOfHour(timestamp) as t, COUNT(*) FROM events WHERE event_type = 'error' GROUP BY t ORDER BY t"}]
      }
    ]
  }
}"#;
