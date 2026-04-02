//! Dashboard components and view models

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::monitoring::metrics::{MetricPoint, MetricType};
use crate::monitoring::{ServerHealthStatus, TimeRange, WidgetConfig, WidgetType};

/// Dashboard view model for UI binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardViewModel {
    pub id: String,
    pub name: String,
    pub is_large_screen: bool,
    pub widgets: Vec<WidgetViewModel>,
    pub refresh_interval_secs: u64,
    pub last_updated: u64,
}

/// Widget view model with real-time data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetViewModel {
    pub config: WidgetConfig,
    pub data: WidgetData,
    pub loading: bool,
    pub error: Option<String>,
    pub last_updated: u64,
}

/// Widget data types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum WidgetData {
    ServerHealth {
        servers: Vec<ServerHealthCard>,
        summary: HealthSummaryCard,
    },
    RealTimeMetrics {
        server_id: String,
        metrics: Vec<RealtimeMetricValue>,
        sparklines: HashMap<MetricType, Vec<f64>>,
    },
    HistoricalChart {
        series: Vec<ChartSeries>,
        x_axis: Vec<String>,
        chart_type: ChartType,
    },
    TopologyMap {
        nodes: Vec<TopologyNodeView>,
        edges: Vec<TopologyEdgeView>,
        selected_node: Option<String>,
    },
    AlertList {
        alerts: Vec<AlertCard>,
        stats: AlertStatsCard,
    },
    PerformanceComparison {
        servers: Vec<PerformanceCard>,
        leaderboard: Vec<LeaderboardEntry>,
    },
    CapacityPlanning {
        forecasts: Vec<CapacityCard>,
        recommendations: Vec<String>,
    },
    SlaDashboard {
        stats: SlaStatsCard,
        monthly_trend: Vec<MonthlySlaPoint>,
        incidents: Vec<IncidentCard>,
    },
    SystemInfo {
        server_id: String,
        os_info: OsInfoCard,
        hardware: HardwareInfoCard,
    },
    ProcessList {
        server_id: String,
        processes: Vec<ProcessCard>,
        total_count: u32,
    },
    LogViewer {
        server_id: String,
        logs: Vec<LogEntry>,
        has_more: bool,
    },
    CustomMetric {
        metric_type: MetricType,
        current_value: f64,
        trend: TrendDirection,
        history: Vec<MetricPoint>,
    },
}

/// Server health card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHealthCard {
    pub server_id: String,
    pub server_name: String,
    pub host: String,
    pub status: ServerHealthStatus,
    pub uptime: String,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub disk_percent: f64,
    pub network_rx: String,
    pub network_tx: String,
    pub active_alerts: u32,
    pub last_seen_secs: u64,
}

/// Health summary card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummaryCard {
    pub total_servers: u32,
    pub healthy_count: u32,
    pub warning_count: u32,
    pub critical_count: u32,
    pub offline_count: u32,
    pub total_alerts: u32,
}

/// Real-time metric value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeMetricValue {
    pub metric_type: MetricType,
    pub value: f64,
    pub unit: String,
    pub formatted: String,
    pub status: MetricStatus,
    pub delta: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricStatus {
    Normal,
    Warning,
    Critical,
}

/// Chart series data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartSeries {
    pub name: String,
    pub data: Vec<f64>,
    pub color: String,
    pub metric_type: MetricType,
}

/// Chart types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartType {
    Line,
    Area,
    Bar,
    StackedArea,
    Gauge,
    Pie,
    Heatmap,
    Sparkline,
}

/// Topology node view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNodeView {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub status: String,
    pub icon: String,
    pub color: String,
    pub metrics: Vec<(String, String)>,
}

/// Topology edge view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdgeView {
    pub id: String,
    pub source: String,
    pub target: String,
    pub status: String,
    pub thickness: f64,
    pub color: String,
    pub animated: bool,
}

/// Alert card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCard {
    pub id: String,
    pub severity: String,
    pub severity_color: String,
    pub title: String,
    pub message: String,
    pub server_name: String,
    pub server_id: String,
    pub started_at: String,
    pub duration: String,
    pub acknowledged: bool,
    pub acknowledged_by: Option<String>,
    pub runbook_url: Option<String>,
}

/// Alert stats card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertStatsCard {
    pub active_critical: u32,
    pub active_warning: u32,
    pub active_info: u32,
    pub today_total: u32,
    pub avg_resolution_time: String,
}

/// Performance card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceCard {
    pub server_id: String,
    pub server_name: String,
    pub current: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
    pub p95: f64,
    pub trend: TrendDirection,
    pub rank: u32,
    pub sparkline: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Up,
    Down,
    Stable,
    Unknown,
}

/// Leaderboard entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: u32,
    pub server_id: String,
    pub server_name: String,
    pub score: f64,
    pub metric_type: MetricType,
}

/// Capacity forecast card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityCard {
    pub resource_type: String,
    pub current_usage: f64,
    pub days_until_critical: Option<u32>,
    pub growth_rate: String,
    pub forecast_chart: Vec<(String, f64, f64, f64)>, // (date, predicted, lower, upper)
    pub status: CapacityStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityStatus {
    Critical,
    Warning,
    Healthy,
    Unknown,
}

/// SLA stats card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaStatsCard {
    pub uptime_percent: f64,
    pub target_percent: f64,
    pub status: SlaWidgetStatus,
    pub total_downtime: String,
    pub incident_count: u32,
    pub mttr: String,
    pub mtbf: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlaWidgetStatus {
    Exceeding,
    Meeting,
    AtRisk,
    Breached,
}

/// Monthly SLA point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlySlaPoint {
    pub month: String,
    pub uptime_percent: f64,
    pub downtime_hours: f64,
}

/// Incident card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentCard {
    pub id: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub duration: String,
    pub severity: String,
    pub description: String,
    pub resolved: bool,
}

/// OS info card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsInfoCard {
    pub name: String,
    pub version: String,
    pub kernel: String,
    pub architecture: String,
    pub uptime: String,
    pub boot_time: String,
}

/// Hardware info card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfoCard {
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub cpu_threads: u32,
    pub memory_total: String,
    pub disk_total: String,
    pub network_interfaces: Vec<String>,
}

/// Process card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessCard {
    pub pid: u32,
    pub name: String,
    pub user: String,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub memory_mb: f64,
    pub status: String,
    pub started: String,
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub source: String,
    pub message: String,
}

/// Dashboard builder
pub struct DashboardBuilder {
    view_model: DashboardViewModel,
}

impl DashboardBuilder {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            view_model: DashboardViewModel {
                id: id.to_string(),
                name: name.to_string(),
                is_large_screen: false,
                widgets: Vec::new(),
                refresh_interval_secs: 30,
                last_updated: 0,
            },
        }
    }

    pub fn large_screen(mut self) -> Self {
        self.view_model.is_large_screen = true;
        self.view_model.refresh_interval_secs = 5;
        self
    }

    pub fn add_server_health_widget(mut self, x: u32, y: u32, w: u32, h: u32) -> Self {
        let widget = WidgetViewModel {
            config: WidgetConfig {
                id: format!("health-{}", self.view_model.widgets.len()),
                widget_type: WidgetType::ServerHealth,
                title: "Server Health".to_string(),
                x,
                y,
                width: w,
                height: h,
                server_ids: Vec::new(),
                metric_types: vec![MetricType::CpuUsage, MetricType::MemoryUsage],
                refresh_interval_secs: 30,
                time_range: TimeRange::Last15Minutes,
                custom_config: HashMap::new(),
            },
            data: WidgetData::ServerHealth {
                servers: Vec::new(),
                summary: HealthSummaryCard {
                    total_servers: 0,
                    healthy_count: 0,
                    warning_count: 0,
                    critical_count: 0,
                    offline_count: 0,
                    total_alerts: 0,
                },
            },
            loading: false,
            error: None,
            last_updated: 0,
        };
        self.view_model.widgets.push(widget);
        self
    }

    pub fn add_realtime_metrics_widget(
        mut self,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        server_id: &str,
    ) -> Self {
        let widget = WidgetViewModel {
            config: WidgetConfig {
                id: format!("realtime-{}", self.view_model.widgets.len()),
                widget_type: WidgetType::RealTimeMetrics,
                title: "Real-time Metrics".to_string(),
                x,
                y,
                width: w,
                height: h,
                server_ids: vec![server_id.to_string()],
                metric_types: vec![
                    MetricType::CpuUsage,
                    MetricType::MemoryUsage,
                    MetricType::DiskUsage,
                ],
                refresh_interval_secs: 5,
                time_range: TimeRange::Last5Minutes,
                custom_config: HashMap::new(),
            },
            data: WidgetData::RealTimeMetrics {
                server_id: server_id.to_string(),
                metrics: Vec::new(),
                sparklines: HashMap::new(),
            },
            loading: false,
            error: None,
            last_updated: 0,
        };
        self.view_model.widgets.push(widget);
        self
    }

    pub fn add_topology_widget(mut self, x: u32, y: u32, w: u32, h: u32) -> Self {
        let widget = WidgetViewModel {
            config: WidgetConfig {
                id: format!("topology-{}", self.view_model.widgets.len()),
                widget_type: WidgetType::TopologyMap,
                title: "Network Topology".to_string(),
                x,
                y,
                width: w,
                height: h,
                server_ids: Vec::new(),
                metric_types: Vec::new(),
                refresh_interval_secs: 60,
                time_range: TimeRange::Last1Hour,
                custom_config: HashMap::new(),
            },
            data: WidgetData::TopologyMap {
                nodes: Vec::new(),
                edges: Vec::new(),
                selected_node: None,
            },
            loading: false,
            error: None,
            last_updated: 0,
        };
        self.view_model.widgets.push(widget);
        self
    }

    pub fn add_alerts_widget(mut self, x: u32, y: u32, w: u32, h: u32) -> Self {
        let widget = WidgetViewModel {
            config: WidgetConfig {
                id: format!("alerts-{}", self.view_model.widgets.len()),
                widget_type: WidgetType::AlertList,
                title: "Active Alerts".to_string(),
                x,
                y,
                width: w,
                height: h,
                server_ids: Vec::new(),
                metric_types: Vec::new(),
                refresh_interval_secs: 10,
                time_range: TimeRange::Last1Hour,
                custom_config: HashMap::new(),
            },
            data: WidgetData::AlertList {
                alerts: Vec::new(),
                stats: AlertStatsCard {
                    active_critical: 0,
                    active_warning: 0,
                    active_info: 0,
                    today_total: 0,
                    avg_resolution_time: "-".to_string(),
                },
            },
            loading: false,
            error: None,
            last_updated: 0,
        };
        self.view_model.widgets.push(widget);
        self
    }

    pub fn add_chart_widget(
        mut self,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        title: &str,
        metric_types: Vec<MetricType>,
        chart_type: ChartType,
    ) -> Self {
        let widget = WidgetViewModel {
            config: WidgetConfig {
                id: format!("chart-{}", self.view_model.widgets.len()),
                widget_type: WidgetType::HistoricalChart,
                title: title.to_string(),
                x,
                y,
                width: w,
                height: h,
                server_ids: Vec::new(),
                metric_types,
                refresh_interval_secs: 30,
                time_range: TimeRange::Last6Hours,
                custom_config: HashMap::new(),
            },
            data: WidgetData::HistoricalChart {
                series: Vec::new(),
                x_axis: Vec::new(),
                chart_type,
            },
            loading: false,
            error: None,
            last_updated: 0,
        };
        self.view_model.widgets.push(widget);
        self
    }

    pub fn build(self) -> DashboardViewModel {
        self.view_model
    }
}

/// Pre-built dashboard templates
pub struct DashboardTemplates;

impl DashboardTemplates {
    /// Executive overview dashboard
    pub fn executive_overview() -> DashboardViewModel {
        DashboardBuilder::new("exec-overview", "Executive Overview")
            .add_server_health_widget(0, 0, 12, 4)
            .add_alerts_widget(0, 4, 4, 4)
            .add_chart_widget(
                4,
                4,
                8,
                4,
                "System Load Trend",
                vec![MetricType::CpuUsage, MetricType::MemoryUsage],
                ChartType::Area,
            )
            .build()
    }

    /// Operations center dashboard
    pub fn operations_center() -> DashboardViewModel {
        DashboardBuilder::new("ops-center", "Operations Center")
            .add_alerts_widget(0, 0, 4, 8)
            .add_topology_widget(4, 0, 8, 6)
            .add_server_health_widget(4, 6, 8, 2)
            .build()
    }

    /// Large screen NOC (Network Operations Center) display
    pub fn noc_display() -> DashboardViewModel {
        DashboardBuilder::new("noc-display", "NOC Display")
            .large_screen()
            .add_server_health_widget(0, 0, 6, 6)
            .add_topology_widget(6, 0, 6, 6)
            .add_alerts_widget(0, 6, 6, 4)
            .add_chart_widget(
                6,
                6,
                6,
                4,
                "Traffic Overview",
                vec![MetricType::NetworkRxMbps, MetricType::NetworkTxMbps],
                ChartType::Area,
            )
            .build()
    }

    /// Single server detailed dashboard
    pub fn server_detail(server_id: &str) -> DashboardViewModel {
        DashboardBuilder::new(
            &format!("server-{}", server_id),
            &format!("Server: {}", server_id),
        )
        .add_realtime_metrics_widget(0, 0, 4, 4, server_id)
        .add_chart_widget(
            4,
            0,
            8,
            4,
            "Historical CPU/Memory",
            vec![MetricType::CpuUsage, MetricType::MemoryUsage],
            ChartType::Line,
        )
        .build()
    }

    /// Performance comparison dashboard
    pub fn performance_comparison() -> DashboardViewModel {
        DashboardBuilder::new("perf-compare", "Performance Comparison")
            .add_chart_widget(
                0,
                0,
                12,
                6,
                "Multi-Server Performance",
                vec![
                    MetricType::CpuUsage,
                    MetricType::MemoryUsage,
                    MetricType::DiskUsage,
                ],
                ChartType::StackedArea,
            )
            .add_server_health_widget(0, 6, 12, 4)
            .build()
    }

    /// Capacity planning dashboard
    pub fn capacity_planning() -> DashboardViewModel {
        DashboardBuilder::new("capacity", "Capacity Planning")
            .add_chart_widget(
                0,
                0,
                12,
                6,
                "Resource Growth Forecast",
                vec![MetricType::DiskUsage, MetricType::MemoryUsage],
                ChartType::Line,
            )
            .add_alerts_widget(0, 6, 6, 4)
            .add_server_health_widget(6, 6, 6, 4)
            .build()
    }
}

/// Dashboard data formatter
pub struct DashboardFormatter;

impl DashboardFormatter {
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
        let mut size = bytes as f64;
        let mut unit_idx = 0;

        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_idx])
    }

    pub fn format_duration(seconds: u64) -> String {
        let days = seconds / 86400;
        let hours = (seconds % 86400) / 3600;
        let minutes = (seconds % 3600) / 60;

        if days > 0 {
            format!("{}d {}h {}m", days, hours, minutes)
        } else if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }

    pub fn format_percentage(value: f64) -> String {
        format!("{:.1}%", value)
    }

    pub fn format_timestamp(timestamp: u64) -> String {
        let datetime = chrono::DateTime::from_timestamp(timestamp as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| timestamp.to_string());
        datetime
    }

    pub fn format_relative_time(timestamp: u64) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let diff = now.saturating_sub(timestamp);

        if diff < 60 {
            "just now".to_string()
        } else if diff < 3600 {
            format!("{}m ago", diff / 60)
        } else if diff < 86400 {
            format!("{}h ago", diff / 3600)
        } else {
            format!("{}d ago", diff / 86400)
        }
    }
}
