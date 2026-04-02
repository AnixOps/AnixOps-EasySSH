//! Professional Monitoring Dashboard System for EasySSH
//!
//! Enterprise-grade server monitoring with:
//! - Real-time metrics collection (CPU, Memory, Disk, Network)
//! - Historical data storage and trending
//! - Network topology visualization
//! - Alert management
//! - Performance comparison
//! - Capacity planning
//! - SLA tracking
//! - Custom dashboard views
//! - Large screen display mode

pub mod alerts;
pub mod collector;
pub mod dashboard;
pub mod metrics;
pub mod notifications;
pub mod sla;
pub mod storage;
pub mod topology;

#[cfg(test)]
pub mod tests;

// Standard version monitoring session module
pub mod session;

pub use alerts::*;
pub use collector::*;
pub use dashboard::*;
pub use metrics::*;
pub use notifications::*;
pub use sla::*;
pub use storage::*;
pub use topology::*;

// Export session types for Standard version
pub use session::{AuthMethod, ChartData, MetricStats, MonitoringSession, ServerConnectionInfo};

// Export collector types
pub use collector::{CollectionScript, CollectionStatus, MetricsCollector, SimpleCollector};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Server health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerHealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
    Offline,
}

impl ServerHealthStatus {
    pub fn color(&self) -> &'static str {
        match self {
            ServerHealthStatus::Healthy => "#22c55e",
            ServerHealthStatus::Warning => "#f59e0b",
            ServerHealthStatus::Critical => "#ef4444",
            ServerHealthStatus::Unknown => "#6b7280",
            ServerHealthStatus::Offline => "#374151",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ServerHealthStatus::Healthy => "check-circle",
            ServerHealthStatus::Warning => "alert-triangle",
            ServerHealthStatus::Critical => "x-circle",
            ServerHealthStatus::Unknown => "help-circle",
            ServerHealthStatus::Offline => "power-off",
        }
    }
}

/// Server overview for health dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerOverview {
    pub server_id: String,
    pub server_name: String,
    pub host: String,
    pub status: ServerHealthStatus,
    pub last_seen: Option<u64>,
    pub uptime_seconds: Option<u64>,
    pub cpu_percent: Option<f64>,
    pub memory_percent: Option<f64>,
    pub disk_percent: Option<f64>,
    pub network_rx_mbps: Option<f64>,
    pub network_tx_mbps: Option<f64>,
    pub active_alerts: usize,
    pub os_info: Option<String>,
    pub location: Option<String>,
}

/// Health summary for all servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub total_servers: usize,
    pub healthy: usize,
    pub warning: usize,
    pub critical: usize,
    pub offline: usize,
    pub unknown: usize,
    pub total_alerts: usize,
    pub servers: Vec<ServerOverview>,
}

/// Dashboard widget types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    ServerHealth,
    RealTimeMetrics,
    HistoricalChart,
    TopologyMap,
    AlertList,
    PerformanceComparison,
    CapacityPlanning,
    SlaDashboard,
    CustomMetric,
    LogViewer,
    SystemInfo,
    ProcessList,
}

/// Dashboard widget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub id: String,
    pub widget_type: WidgetType,
    pub title: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub server_ids: Vec<String>,
    pub metric_types: Vec<MetricType>,
    pub refresh_interval_secs: u64,
    pub time_range: TimeRange,
    pub custom_config: HashMap<String, serde_json::Value>,
}

/// Time range for historical data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeRange {
    Last5Minutes,
    Last15Minutes,
    Last30Minutes,
    Last1Hour,
    Last6Hours,
    Last12Hours,
    Last24Hours,
    Last7Days,
    Last30Days,
    Custom { start: u64, end: u64 },
}

impl TimeRange {
    pub fn to_seconds(&self) -> u64 {
        match self {
            TimeRange::Last5Minutes => 300,
            TimeRange::Last15Minutes => 900,
            TimeRange::Last30Minutes => 1800,
            TimeRange::Last1Hour => 3600,
            TimeRange::Last6Hours => 21600,
            TimeRange::Last12Hours => 43200,
            TimeRange::Last24Hours => 86400,
            TimeRange::Last7Days => 604800,
            TimeRange::Last30Days => 2592000,
            TimeRange::Custom { start, end } => end - start,
        }
    }

    pub fn get_start_timestamp(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now - self.to_seconds()
    }
}

/// Custom dashboard definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomDashboard {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub widgets: Vec<WidgetConfig>,
    pub is_default: bool,
    pub is_large_screen: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub collection_interval_secs: u64,
    pub retention_days: u32,
    pub alert_check_interval_secs: u64,
    pub enable_predictive_alerts: bool,
    pub enable_anomaly_detection: bool,
    pub large_screen_refresh_secs: u64,
    pub default_dashboard_id: Option<String>,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            collection_interval_secs: 30,
            retention_days: 90,
            alert_check_interval_secs: 60,
            enable_predictive_alerts: true,
            enable_anomaly_detection: true,
            large_screen_refresh_secs: 5,
            default_dashboard_id: None,
        }
    }
}

/// Monitoring manager - main entry point
pub struct MonitoringManager {
    config: MonitoringConfig,
    collector: Arc<MetricsCollector>,
    storage: Arc<MetricsStorage>,
    alert_engine: Arc<AlertEngine>,
    notification_manager: Arc<RwLock<NotificationManager>>,
    sla_monitor: Arc<SlaMonitor>,
    dashboards: Arc<RwLock<HashMap<String, CustomDashboard>>>,
    topology: Arc<RwLock<ServerTopology>>,
}

impl MonitoringManager {
    pub async fn new(config: MonitoringConfig) -> Result<Self, MonitoringError> {
        let storage = Arc::new(MetricsStorage::new(&config).await?);
        let collector = Arc::new(MetricsCollector::new(
            Arc::clone(&storage),
            config.collection_interval_secs,
        ));
        let alert_engine = Arc::new(AlertEngine::new(Arc::clone(&storage), &config).await?);
        let notification_manager = Arc::new(RwLock::new(NotificationManager::new()));
        let sla_monitor = Arc::new(SlaMonitor::new(
            sla::SlaConfig::default(),
            Arc::clone(&storage),
        ));

        Ok(Self {
            config,
            collector,
            storage,
            alert_engine,
            notification_manager,
            sla_monitor,
            dashboards: Arc::new(RwLock::new(HashMap::new())),
            topology: Arc::new(RwLock::new(ServerTopology::new())),
        })
    }

    /// Initialize monitoring for a server
    pub async fn add_server(
        &self,
        server_id: String,
        connection_config: ServerConnectionConfig,
    ) -> Result<(), MonitoringError> {
        self.collector
            .register_server(server_id.clone(), connection_config)
            .await?;

        // Update topology
        let mut topology = self.topology.write().await;
        topology.add_node(TopologyNode {
            id: server_id.clone(),
            node_type: TopologyNodeType::Server,
            label: server_id.clone(),
            status: TopologyStatus::Unknown,
            metrics: HashMap::new(),
            x: 0.0,
            y: 0.0,
            group_id: None,
            icon: None,
            color: None,
            metadata: HashMap::new(),
        });

        Ok(())
    }

    /// Remove server from monitoring
    pub async fn remove_server(&self, server_id: &str) -> Result<(), MonitoringError> {
        self.collector.unregister_server(server_id).await;

        let mut topology = self.topology.write().await;
        topology.remove_node(server_id);

        Ok(())
    }

    /// Get health summary for all servers
    pub async fn get_health_summary(&self) -> Result<HealthSummary, MonitoringError> {
        self.storage.get_health_summary().await
    }

    /// Get real-time metrics for a server
    pub async fn get_realtime_metrics(
        &self,
        server_id: &str,
    ) -> Result<ServerMetrics, MonitoringError> {
        self.storage.get_latest_metrics(server_id).await
    }

    /// Get historical metrics
    pub async fn get_historical_metrics(
        &self,
        server_id: &str,
        metric_type: MetricType,
        time_range: TimeRange,
    ) -> Result<Vec<MetricPoint>, MonitoringError> {
        self.storage
            .get_metrics_history(server_id, metric_type, time_range)
            .await
    }

    /// Get current alerts
    pub async fn get_active_alerts(&self) -> Result<Vec<Alert>, MonitoringError> {
        self.alert_engine.get_active_alerts().await
    }

    /// Acknowledge an alert
    pub async fn acknowledge_alert(
        &self,
        alert_id: &str,
        user_id: &str,
    ) -> Result<(), MonitoringError> {
        self.alert_engine.acknowledge_alert(alert_id, user_id).await
    }

    /// Get server topology
    pub async fn get_topology(&self) -> Result<ServerTopology, MonitoringError> {
        Ok(self.topology.read().await.clone())
    }

    /// Update topology layout
    pub async fn update_topology_layout(
        &self,
        layout: TopologyLayout,
    ) -> Result<(), MonitoringError> {
        let mut topology = self.topology.write().await;
        topology.apply_layout(layout);
        Ok(())
    }

    /// Get performance comparison
    pub async fn compare_performance(
        &self,
        server_ids: Vec<String>,
        metric_type: MetricType,
        time_range: TimeRange,
    ) -> Result<PerformanceComparison, MonitoringError> {
        self.storage
            .compare_performance(server_ids, metric_type, time_range)
            .await
    }

    /// Get capacity planning forecast
    pub async fn get_capacity_forecast(
        &self,
        server_id: &str,
        resource_type: ResourceType,
        days_ahead: u32,
    ) -> Result<CapacityForecast, MonitoringError> {
        self.storage
            .predict_capacity(server_id, resource_type, days_ahead)
            .await
    }

    /// Get SLA statistics
    pub async fn get_sla_stats(
        &self,
        server_id: &str,
        time_range: TimeRange,
    ) -> Result<SlaStats, MonitoringError> {
        self.sla_monitor.calculate_sla(server_id, time_range).await
    }

    /// Get SLA health status
    pub async fn get_sla_health_status(
        &self,
        server_id: &str,
    ) -> Result<sla::SlaHealthStatus, MonitoringError> {
        self.sla_monitor.get_health_status(server_id).await
    }

    /// Get SLA dashboard
    pub async fn get_sla_dashboard(
        &self,
        server_ids: Option<&[String]>,
    ) -> Result<sla::SlaDashboard, MonitoringError> {
        self.sla_monitor.get_sla_dashboard(server_ids).await
    }

    /// Generate SLA report
    pub async fn generate_sla_report(
        &self,
        server_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<sla::SlaReport, MonitoringError> {
        self.sla_monitor
            .generate_report(server_id, start_date, end_date)
            .await
    }

    /// Add notification channel
    pub async fn add_notification_channel(
        &self,
        channel: notifications::NotificationChannel,
    ) -> Result<(), MonitoringError> {
        let mut manager = self.notification_manager.write().await;
        manager
            .add_channel(&channel)
            .map_err(|e| MonitoringError::Config(e.to_string()))
    }

    /// Remove notification channel
    pub async fn remove_notification_channel(&self, channel_id: &str) {
        let mut manager = self.notification_manager.write().await;
        manager.remove_channel(channel_id);
    }

    /// Send test notification
    pub async fn send_test_notification(&self, channel_id: &str) -> Result<(), MonitoringError> {
        let manager = self.notification_manager.read().await;
        let payload = notifications::NotificationPayload {
            title: "Test Notification".to_string(),
            message: "This is a test notification from EasySSH monitoring".to_string(),
            severity: AlertSeverity::Info,
            server_id: "test-server".to_string(),
            server_name: "Test Server".to_string(),
            metric_type: "Test".to_string(),
            metric_value: 0.0,
            threshold: 0.0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            alert_id: "test-alert".to_string(),
            runbook_url: None,
            dashboard_url: None,
            tags: vec!["test".to_string()],
        };

        manager
            .send_to_channel(channel_id, &payload, 60)
            .await
            .map_err(|e| MonitoringError::Alert(e.to_string()))
    }

    /// Create custom dashboard
    pub async fn create_dashboard(
        &self,
        dashboard: CustomDashboard,
    ) -> Result<(), MonitoringError> {
        let mut dashboards = self.dashboards.write().await;
        let dashboard_id = dashboard.id.clone();
        dashboards.insert(dashboard_id.clone(), dashboard.clone());
        self.storage.save_dashboard(&dashboard).await?;
        Ok(())
    }

    /// Get dashboard
    pub async fn get_dashboard(
        &self,
        dashboard_id: &str,
    ) -> Result<Option<CustomDashboard>, MonitoringError> {
        let dashboards = self.dashboards.read().await;
        Ok(dashboards.get(dashboard_id).cloned())
    }

    /// Get all dashboards
    pub async fn list_dashboards(&self) -> Result<Vec<CustomDashboard>, MonitoringError> {
        let dashboards = self.dashboards.read().await;
        Ok(dashboards.values().cloned().collect())
    }

    /// Delete dashboard
    pub async fn delete_dashboard(&self, dashboard_id: &str) -> Result<(), MonitoringError> {
        let mut dashboards = self.dashboards.write().await;
        dashboards.remove(dashboard_id);
        self.storage.delete_dashboard(dashboard_id).await?;
        Ok(())
    }

    /// Start monitoring
    pub async fn start(&self) -> Result<(), MonitoringError> {
        self.collector.start().await?;
        self.alert_engine.start().await?;
        Ok(())
    }

    /// Stop monitoring
    pub async fn stop(&self) -> Result<(), MonitoringError> {
        self.collector.stop().await;
        self.alert_engine.stop().await;
        Ok(())
    }
}

/// Monitoring error types
#[derive(Debug, thiserror::Error)]
pub enum MonitoringError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Collection error: {0}")]
    Collection(String),
    #[error("Alert error: {0}")]
    Alert(String),
    #[error("SSH error: {0}")]
    Ssh(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Server connection configuration for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConnectionConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
}
