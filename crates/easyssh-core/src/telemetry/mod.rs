//! Telemetry and Analytics System for EasySSH
//!
//! Privacy-first analytics with:
//! - Anonymous usage statistics
//! - Performance telemetry
//! - Opt-in consent management
//! - GDPR/CCPA compliance
//! - Feature flags (A/B testing)
//! - Error tracking
//! - In-app feedback
//!
//! All data collection is opt-in and anonymized.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use uuid::Uuid;

mod collector;
mod consent;
mod error_tracker;
mod feature_flags;
mod feedback;
mod health_monitor;
mod metrics;
mod reporter;
mod storage;

pub use collector::{EventCollector, EventFilter};
pub use consent::{ConsentManager, ConsentStatus, PrivacyRegion};
pub use error_tracker::{ErrorContext, ErrorTracker, Severity};
pub use feature_flags::{FeatureFlag, FeatureFlagManager, Variant};
pub use feedback::{FeedbackCollector, FeedbackRating, UserFeedback};
pub use health_monitor::{HealthCheck, HealthMonitor, ServiceHealth};
pub use metrics::{Counter, Gauge, Histogram, MetricType, MetricsRegistry, Timer};
pub use reporter::{AnalyticsReporter, BatchConfig, ReporterConfig};
pub use storage::{AnalyticsStorage, DataRetentionPolicy};

/// Unique anonymous user ID (rotated periodically)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousId(String);

impl AnonymousId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for AnonymousId {
    fn default() -> Self {
        Self::new()
    }
}

/// Application edition for telemetry context
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryEdition {
    Lite,
    Standard,
    Pro,
}

/// Platform information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub app_version: String,
    pub edition: TelemetryEdition,
}

impl PlatformInfo {
    pub fn current(edition: TelemetryEdition) -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            os_version: Self::get_os_version(),
            arch: std::env::consts::ARCH.to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            edition,
        }
    }

    fn get_os_version() -> String {
        #[cfg(target_os = "windows")]
        {
            unsafe {
                let mut info: windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW =
                    std::mem::zeroed();
                info.dwOSVersionInfoSize = std::mem::size_of::<
                    windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW,
                >() as u32;

                if windows_sys::Win32::System::SystemInformation::GetVersionExW(&mut info) != 0 {
                    format!(
                        "{}.{}.{}",
                        info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber
                    )
                } else {
                    "unknown".to_string()
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            std::fs::read_to_string("/etc/os-release")
                .ok()
                .and_then(|s| {
                    s.lines().find(|l| l.starts_with("VERSION_ID=")).map(|l| {
                        l.trim_start_matches("VERSION_ID=")
                            .trim_matches('"')
                            .to_string()
                    })
                })
                .unwrap_or_else(|| "unknown".to_string())
        }
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            Command::new("sw_vers")
                .arg("-productVersion")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            "unknown".to_string()
        }
    }
}

/// Event types for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "event_type")]
pub enum TelemetryEvent {
    /// Application lifecycle
    AppStarted {
        startup_time_ms: u64,
        cold_start: bool,
    },
    AppClosed {
        session_duration_ms: u64,
    },

    /// Feature usage
    FeatureUsed {
        feature: String,
        context: HashMap<String, serde_json::Value>,
    },

    /// Navigation
    ScreenViewed {
        screen: String,
        time_spent_ms: Option<u64>,
    },

    /// Performance metrics
    PerformanceMetric {
        metric_name: String,
        value: f64,
        unit: String,
        context: HashMap<String, serde_json::Value>,
    },

    /// SSH operations (anonymized)
    SshConnected {
        auth_method: String, // "password", "key", "agent"
        connection_time_ms: u64,
        // NO hostnames, IPs, or usernames collected
    },
    SshDisconnected {
        session_duration_ms: u64,
        bytes_transferred: Option<u64>,
    },

    /// Error events
    ErrorOccurred {
        error_type: String,
        severity: Severity,
        component: String,
        // Stack traces only in debug builds
        #[serde(skip_serializing_if = "Option::is_none")]
        stack_trace: Option<String>,
    },

    /// User feedback
    FeedbackSubmitted {
        rating: FeedbackRating,
        category: String,
    },

    /// Consent changes
    ConsentChanged {
        analytics_enabled: bool,
        crash_reporting_enabled: bool,
    },
}

/// Complete telemetry event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEventRecord {
    pub id: String,
    pub anonymous_id: AnonymousId,
    pub timestamp: u64,
    pub platform: PlatformInfo,
    pub event: TelemetryEvent,
    pub session_id: String,
}

impl TelemetryEventRecord {
    pub fn new(
        anonymous_id: AnonymousId,
        platform: PlatformInfo,
        event: TelemetryEvent,
        session_id: String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: Uuid::new_v4().to_string(),
            anonymous_id,
            timestamp,
            platform,
            event,
            session_id,
        }
    }
}

/// Telemetry manager - main entry point
pub struct TelemetryManager {
    config: TelemetryConfig,
    consent_manager: Arc<ConsentManager>,
    collector: Arc<EventCollector>,
    metrics: Arc<MetricsRegistry>,
    error_tracker: Arc<ErrorTracker>,
    feature_flags: Arc<FeatureFlagManager>,
    feedback: Arc<FeedbackCollector>,
    health_monitor: Arc<HealthMonitor>,
    reporter: Arc<AnalyticsReporter>,
    storage: Arc<AnalyticsStorage>,
    session_id: String,
    anonymous_id: AnonymousId,
    platform: PlatformInfo,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

/// Configuration for telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Edition of the app
    pub edition: TelemetryEdition,
    /// PostHog/Segment API key (if enabled)
    pub api_key: Option<String>,
    /// Server endpoint for analytics
    pub endpoint: Option<String>,
    /// Batch size for events
    pub batch_size: usize,
    /// Flush interval in seconds
    pub flush_interval_secs: u64,
    /// Enable local storage buffering
    pub enable_local_buffer: bool,
    /// Maximum events to buffer locally
    pub max_local_events: usize,
    /// Data retention days
    pub retention_days: u32,
    /// Debug mode - print to console
    pub debug_mode: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            edition: TelemetryEdition::Lite,
            api_key: None,
            endpoint: None,
            batch_size: 50,
            flush_interval_secs: 30,
            enable_local_buffer: true,
            max_local_events: 1000,
            retention_days: 90,
            debug_mode: cfg!(debug_assertions),
        }
    }
}

impl TelemetryManager {
    /// Create new telemetry manager
    pub fn new(config: TelemetryConfig) -> Result<Self, TelemetryError> {
        let anonymous_id = AnonymousId::new();
        let session_id = Uuid::new_v4().to_string();
        let platform = PlatformInfo::current(config.edition);

        let consent_manager = Arc::new(ConsentManager::new()?);
        let storage = Arc::new(AnalyticsStorage::new(&config)?);
        let collector = Arc::new(EventCollector::new(
            Arc::clone(&storage),
            Arc::clone(&consent_manager),
            config.batch_size,
        ));
        let metrics = Arc::new(MetricsRegistry::new()?);
        let error_tracker = Arc::new(ErrorTracker::new(
            Arc::clone(&collector),
            Arc::clone(&consent_manager),
        ));
        let feature_flags = Arc::new(FeatureFlagManager::new()?);
        let feedback = Arc::new(FeedbackCollector::new(Arc::clone(&collector))?);
        let health_monitor = Arc::new(HealthMonitor::new()?);
        let reporter = Arc::new(AnalyticsReporter::new(&config, Arc::clone(&storage))?);

        Ok(Self {
            config,
            consent_manager,
            collector,
            metrics,
            error_tracker,
            feature_flags,
            feedback,
            health_monitor,
            reporter,
            storage,
            session_id,
            anonymous_id,
            platform,
            shutdown_tx: None,
        })
    }

    /// Initialize telemetry (call at app startup)
    pub async fn initialize(&mut self) -> Result<(), TelemetryError> {
        // Load persisted consent
        self.consent_manager.load().await?;

        // Start background reporter
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        self.reporter
            .start(
                shutdown_rx,
                Duration::from_secs(self.config.flush_interval_secs),
            )
            .await?;

        // Track app start
        self.track_event(TelemetryEvent::AppStarted {
            startup_time_ms: 0, // Updated by caller
            cold_start: true,
        })
        .await;

        // Start health monitoring
        self.health_monitor.start().await?;

        Ok(())
    }

    /// Track an event
    pub async fn track_event(&self, event: TelemetryEvent) {
        // Check consent
        if !self.consent_manager.can_collect(&event) {
            return;
        }

        let record = TelemetryEventRecord::new(
            self.anonymous_id.clone(),
            self.platform.clone(),
            event,
            self.session_id.clone(),
        );

        if self.config.debug_mode {
            println!("[Telemetry] {:?}", record);
        }

        self.collector.collect(record).await;
    }

    /// Track feature usage
    pub async fn track_feature(&self, feature: &str, context: HashMap<String, serde_json::Value>) {
        self.track_event(TelemetryEvent::FeatureUsed {
            feature: feature.to_string(),
            context,
        })
        .await;
    }

    /// Track screen view
    pub async fn track_screen(&self, screen: &str, time_spent_ms: Option<u64>) {
        self.track_event(TelemetryEvent::ScreenViewed {
            screen: screen.to_string(),
            time_spent_ms,
        })
        .await;
    }

    /// Track SSH connection (anonymized)
    pub async fn track_ssh_connected(&self, auth_method: &str, connection_time_ms: u64) {
        self.track_event(TelemetryEvent::SshConnected {
            auth_method: auth_method.to_string(),
            connection_time_ms,
        })
        .await;
    }

    /// Track SSH disconnect
    pub async fn track_ssh_disconnected(&self, session_duration_ms: u64) {
        self.track_event(TelemetryEvent::SshDisconnected {
            session_duration_ms,
            bytes_transferred: None, // Optional metric
        })
        .await;
    }

    /// Get metrics registry
    pub fn metrics(&self) -> &MetricsRegistry {
        &self.metrics
    }

    /// Get error tracker
    pub fn error_tracker(&self) -> &ErrorTracker {
        &self.error_tracker
    }

    /// Get feature flag manager
    pub fn feature_flags(&self) -> &FeatureFlagManager {
        &self.feature_flags
    }

    /// Get feedback collector
    pub fn feedback(&self) -> &FeedbackCollector {
        &self.feedback
    }

    /// Get health monitor
    pub fn health_monitor(&self) -> &HealthMonitor {
        &self.health_monitor
    }

    /// Get consent manager
    pub fn consent(&self) -> &ConsentManager {
        &self.consent_manager
    }

    /// Start a performance timer
    pub fn start_timer(&self, name: &str) -> Timer {
        self.metrics.start_timer(name)
    }

    /// Record a performance metric
    pub async fn record_performance(&self, metric_name: &str, value: f64, unit: &str) {
        let mut context = HashMap::new();
        context.insert(
            "unit".to_string(),
            serde_json::Value::String(unit.to_string()),
        );

        self.track_event(TelemetryEvent::PerformanceMetric {
            metric_name: metric_name.to_string(),
            value,
            unit: unit.to_string(),
            context,
        })
        .await;
    }

    /// Shutdown telemetry
    pub async fn shutdown(&self) -> Result<(), TelemetryError> {
        // Track app close
        self.track_event(TelemetryEvent::AppClosed {
            session_duration_ms: 0, // Would be calculated from actual session start
        })
        .await;

        // Flush remaining events
        self.collector.flush().await?;
        self.reporter.flush().await?;

        // Signal background tasks to stop
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }

        self.health_monitor.stop().await?;

        Ok(())
    }

    /// Export user data (GDPR/CCPA compliance)
    pub async fn export_user_data(&self) -> Result<String, TelemetryError> {
        let data = self.storage.export_data(&self.anonymous_id).await?;
        Ok(data)
    }

    /// Delete user data (GDPR/CCPA compliance)
    pub async fn delete_user_data(&self) -> Result<(), TelemetryError> {
        self.storage.delete_data(&self.anonymous_id).await?;
        Ok(())
    }

    /// Check if feature is enabled (A/B testing)
    pub fn is_feature_enabled(&self, flag_name: &str) -> bool {
        self.feature_flags.is_enabled(flag_name, &self.anonymous_id)
    }

    /// Get feature variant for user
    pub fn get_feature_variant(&self, flag_name: &str) -> Option<Variant> {
        self.feature_flags
            .get_variant(flag_name, &self.anonymous_id)
    }
}

/// Telemetry error types
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Consent error: {0}")]
    Consent(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Database(String),
}

/// Global telemetry instance for convenience
static mut TELEMETRY: Option<Arc<TelemetryManager>> = None;
static TELEMETRY_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global telemetry
pub fn init_global(config: TelemetryConfig) -> Result<Arc<TelemetryManager>, TelemetryError> {
    let mut result = None;
    TELEMETRY_INIT.call_once(|| match TelemetryManager::new(config) {
        Ok(manager) => {
            unsafe {
                TELEMETRY = Some(Arc::new(manager));
            }
            result = unsafe { TELEMETRY.clone() };
        }
        Err(e) => {
            eprintln!("Failed to initialize telemetry: {}", e);
        }
    });
    result.ok_or_else(|| TelemetryError::Config("Telemetry already initialized".to_string()))
}

/// Get global telemetry instance
pub fn global() -> Option<Arc<TelemetryManager>> {
    unsafe { TELEMETRY.clone() }
}

/// Macro for easy event tracking
#[macro_export]
macro_rules! track_event {
    ($event:expr) => {
        if let Some(t) = $crate::telemetry::global() {
            tokio::spawn(async move {
                t.track_event($event).await;
            });
        }
    };
}

/// Macro for feature tracking
#[macro_export]
macro_rules! track_feature {
    ($feature:expr) => {
        if let Some(t) = $crate::telemetry::global() {
            tokio::spawn(async move {
                t.track_feature($feature, std::collections::HashMap::new())
                    .await;
            });
        }
    };
    ($feature:expr, $context:expr) => {
        if let Some(t) = $crate::telemetry::global() {
            tokio::spawn(async move {
                t.track_feature($feature, $context).await;
            });
        }
    };
}
