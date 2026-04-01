//! Metrics data structures and types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metric types supported by the monitoring system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    // CPU metrics
    CpuUsage,
    CpuUser,
    CpuSystem,
    CpuIowait,
    CpuSteal,
    CpuCores,
    CpuLoad1,
    CpuLoad5,
    CpuLoad15,

    // Memory metrics
    MemoryUsage,
    MemoryUsed,
    MemoryTotal,
    MemoryFree,
    MemoryBuffers,
    MemoryCached,
    MemoryAvailable,
    SwapUsage,
    SwapUsed,
    SwapTotal,
    SwapFree,

    // Disk metrics
    DiskUsage,
    DiskUsed,
    DiskTotal,
    DiskFree,
    DiskReadBytes,
    DiskWriteBytes,
    DiskReadIops,
    DiskWriteIops,
    DiskReadLatency,
    DiskWriteLatency,
    DiskIoUtil,

    // Network metrics
    NetworkRxBytes,
    NetworkTxBytes,
    NetworkRxPackets,
    NetworkTxPackets,
    NetworkRxErrors,
    NetworkTxErrors,
    NetworkRxDropped,
    NetworkTxDropped,
    NetworkRxMbps,
    NetworkTxMbps,

    // Process metrics
    ProcessCount,
    ProcessRunning,
    ProcessSleeping,
    ProcessZombie,
    ThreadCount,
    OpenFiles,

    // System metrics
    Uptime,
    BootTime,
    ContextSwitches,
    Interrupts,
    Forks,

    // Temperature metrics
    CpuTemp,
    GpuTemp,
    SystemTemp,

    // Power metrics
    PowerUsage,
    BatteryLevel,

    // Custom metrics
    Custom(String),
}

impl MetricType {
    pub fn unit(&self) -> &'static str {
        match self {
            // Percentage
            MetricType::CpuUsage | MetricType::CpuUser | MetricType::CpuSystem |
            MetricType::CpuIowait | MetricType::CpuSteal | MetricType::MemoryUsage |
            MetricType::SwapUsage | MetricType::DiskUsage | MetricType::DiskIoUtil => "%",

            // Bytes
            MetricType::MemoryUsed | MetricType::MemoryTotal | MetricType::MemoryFree |
            MetricType::MemoryBuffers | MetricType::MemoryCached | MetricType::MemoryAvailable |
            MetricType::SwapUsed | MetricType::SwapTotal | MetricType::SwapFree |
            MetricType::DiskUsed | MetricType::DiskTotal | MetricType::DiskFree |
            MetricType::DiskReadBytes | MetricType::DiskWriteBytes |
            MetricType::NetworkRxBytes | MetricType::NetworkTxBytes => "bytes",

            // Count
            MetricType::CpuCores | MetricType::ProcessCount | MetricType::ProcessRunning |
            MetricType::ProcessSleeping | MetricType::ProcessZombie | MetricType::ThreadCount |
            MetricType::OpenFiles | MetricType::NetworkRxPackets | MetricType::NetworkTxPackets |
            MetricType::NetworkRxErrors | MetricType::NetworkTxErrors |
            MetricType::NetworkRxDropped | MetricType::NetworkTxDropped |
            MetricType::ContextSwitches | MetricType::Interrupts | MetricType::Forks => "count",

            // IOPS
            MetricType::DiskReadIops | MetricType::DiskWriteIops => "iops",

            // Latency
            MetricType::DiskReadLatency | MetricType::DiskWriteLatency => "ms",

            // Throughput
            MetricType::NetworkRxMbps | MetricType::NetworkTxMbps => "mbps",

            // Time
            MetricType::Uptime | MetricType::BootTime => "seconds",

            // Load
            MetricType::CpuLoad1 | MetricType::CpuLoad5 | MetricType::CpuLoad15 => "load",

            // Temperature
            MetricType::CpuTemp | MetricType::GpuTemp | MetricType::SystemTemp => "celsius",

            // Power
            MetricType::PowerUsage => "watts",
            MetricType::BatteryLevel => "%",

            // Custom
            MetricType::Custom(_) => "",
        }
    }

    pub fn category(&self) -> MetricCategory {
        match self {
            MetricType::CpuUsage | MetricType::CpuUser | MetricType::CpuSystem |
            MetricType::CpuIowait | MetricType::CpuSteal | MetricType::CpuCores |
            MetricType::CpuLoad1 | MetricType::CpuLoad5 | MetricType::CpuLoad15 => MetricCategory::Cpu,

            MetricType::MemoryUsage | MetricType::MemoryUsed | MetricType::MemoryTotal |
            MetricType::MemoryFree | MetricType::MemoryBuffers | MetricType::MemoryCached |
            MetricType::MemoryAvailable | MetricType::SwapUsage | MetricType::SwapUsed |
            MetricType::SwapTotal | MetricType::SwapFree => MetricCategory::Memory,

            MetricType::DiskUsage | MetricType::DiskUsed | MetricType::DiskTotal |
            MetricType::DiskFree | MetricType::DiskReadBytes | MetricType::DiskWriteBytes |
            MetricType::DiskReadIops | MetricType::DiskWriteIops |
            MetricType::DiskReadLatency | MetricType::DiskWriteLatency |
            MetricType::DiskIoUtil => MetricCategory::Disk,

            MetricType::NetworkRxBytes | MetricType::NetworkTxBytes |
            MetricType::NetworkRxPackets | MetricType::NetworkTxPackets |
            MetricType::NetworkRxErrors | MetricType::NetworkTxErrors |
            MetricType::NetworkRxDropped | MetricType::NetworkTxDropped |
            MetricType::NetworkRxMbps | MetricType::NetworkTxMbps => MetricCategory::Network,

            MetricType::ProcessCount | MetricType::ProcessRunning |
            MetricType::ProcessSleeping | MetricType::ProcessZombie |
            MetricType::ThreadCount | MetricType::OpenFiles => MetricCategory::Process,

            MetricType::Uptime | MetricType::BootTime | MetricType::ContextSwitches |
            MetricType::Interrupts | MetricType::Forks => MetricCategory::System,

            MetricType::CpuTemp | MetricType::GpuTemp | MetricType::SystemTemp => MetricCategory::Temperature,

            MetricType::PowerUsage | MetricType::BatteryLevel => MetricCategory::Power,

            MetricType::Custom(_) => MetricCategory::Custom,
        }
    }
}

/// Metric categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricCategory {
    Cpu,
    Memory,
    Disk,
    Network,
    Process,
    System,
    Temperature,
    Power,
    Custom,
}

/// Resource types for capacity planning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Cpu,
    Memory,
    Disk,
    Network,
}

/// A single metric data point with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub timestamp: u64,
    pub value: f64,
    pub metric_type: MetricType,
    pub labels: HashMap<String, String>,
}

/// Complete server metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetrics {
    pub server_id: String,
    pub timestamp: u64,
    pub collected_at: u64,

    // CPU metrics
    pub cpu_usage: f64,
    pub cpu_user: f64,
    pub cpu_system: f64,
    pub cpu_iowait: f64,
    pub cpu_steal: f64,
    pub cpu_cores: u32,
    pub cpu_load1: f64,
    pub cpu_load5: f64,
    pub cpu_load15: f64,

    // Memory metrics (in bytes)
    pub memory_used: u64,
    pub memory_total: u64,
    pub memory_free: u64,
    pub memory_buffers: u64,
    pub memory_cached: u64,
    pub memory_available: u64,
    pub swap_used: u64,
    pub swap_total: u64,

    // Disk metrics
    pub disk_used: u64,
    pub disk_total: u64,
    pub disk_free: u64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub disk_read_iops: f64,
    pub disk_write_iops: f64,
    pub disk_io_util: f64,

    // Network metrics
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub network_rx_packets: u64,
    pub network_tx_packets: u64,
    pub network_rx_errors: u64,
    pub network_tx_errors: u64,
    pub network_rx_dropped: u64,
    pub network_tx_dropped: u64,

    // Process metrics
    pub process_count: u32,
    pub process_running: u32,
    pub process_sleeping: u32,
    pub process_zombie: u32,
    pub thread_count: u32,
    pub open_files: u32,

    // System metrics
    pub uptime_seconds: u64,
    pub boot_time: u64,
    pub context_switches: u64,
    pub interrupts: u64,

    // Temperature (optional)
    pub cpu_temp: Option<f64>,
    pub system_temp: Option<f64>,

    // Extra metrics for extensibility
    pub extra: HashMap<String, f64>,
}

impl ServerMetrics {
    pub fn memory_usage_percent(&self) -> f64 {
        if self.memory_total == 0 {
            0.0
        } else {
            ((self.memory_used as f64) / (self.memory_total as f64)) * 100.0
        }
    }

    pub fn swap_usage_percent(&self) -> f64 {
        if self.swap_total == 0 {
            0.0
        } else {
            ((self.swap_used as f64) / (self.swap_total as f64)) * 100.0
        }
    }

    pub fn disk_usage_percent(&self) -> f64 {
        if self.disk_total == 0 {
            0.0
        } else {
            ((self.disk_used as f64) / (self.disk_total as f64)) * 100.0
        }
    }

    pub fn network_rx_mbps(&self, interval_secs: u64) -> f64 {
        if interval_secs == 0 {
            0.0
        } else {
            ((self.network_rx_bytes as f64) * 8.0) / (interval_secs as f64) / 1_000_000.0
        }
    }

    pub fn network_tx_mbps(&self, interval_secs: u64) -> f64 {
        if interval_secs == 0 {
            0.0
        } else {
            ((self.network_tx_bytes as f64) * 8.0) / (interval_secs as f64) / 1_000_000.0
        }
    }

    /// Get health status based on thresholds
    pub fn health_status(&self) -> crate::monitoring::ServerHealthStatus {
        let cpu_critical = self.cpu_usage > 90.0;
        let memory_critical = self.memory_usage_percent() > 90.0;
        let disk_critical = self.disk_usage_percent() > 90.0;
        let load_critical = self.cpu_load1 > (self.cpu_cores as f64 * 2.0);

        if cpu_critical || memory_critical || disk_critical || load_critical {
            crate::monitoring::ServerHealthStatus::Critical
        } else if self.cpu_usage > 70.0 || self.memory_usage_percent() > 70.0 || self.disk_usage_percent() > 80.0 {
            crate::monitoring::ServerHealthStatus::Warning
        } else {
            crate::monitoring::ServerHealthStatus::Healthy
        }
    }
}

/// Disk partition metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskPartitionMetrics {
    pub device: String,
    pub mount_point: String,
    pub filesystem: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub usage_percent: f64,
}

/// Network interface metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceMetrics {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_dropped: u64,
    pub tx_dropped: u64,
    pub speed_mbps: Option<u64>,
    pub is_up: bool,
}

/// CPU core metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuCoreMetrics {
    pub core_id: u32,
    pub usage: f64,
    pub user: f64,
    pub system: f64,
    pub iowait: f64,
    pub steal: f64,
}

/// Process information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub user: String,
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub memory_percent: f64,
    pub status: ProcessStatus,
    pub started_at: u64,
    pub command: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStatus {
    Running,
    Sleeping,
    Stopped,
    Zombie,
    Dead,
    Unknown,
}

/// Performance comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceComparison {
    pub metric_type: MetricType,
    pub time_range: crate::monitoring::TimeRange,
    pub servers: Vec<ServerPerformanceData>,
    pub winner: Option<String>,
    pub average: f64,
    pub median: f64,
    pub std_dev: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerPerformanceData {
    pub server_id: String,
    pub server_name: String,
    pub current: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
    pub p95: f64,
    pub p99: f64,
    pub trend: super::dashboard::TrendDirection,
    pub sparkline: Vec<f64>,
}

/// Capacity forecast for resource planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityForecast {
    pub server_id: String,
    pub resource_type: ResourceType,
    pub current_usage: f64,
    pub current_capacity: f64,
    pub predicted_depletion_date: Option<u64>,
    pub days_until_critical: Option<u32>,
    pub growth_rate_per_day: f64,
    pub confidence: f64,
    pub forecast_points: Vec<ForecastPoint>,
    pub recommendations: Vec<CapacityRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPoint {
    pub timestamp: u64,
    pub predicted_usage: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityRecommendation {
    pub priority: Priority,
    pub message: String,
    pub action: String,
    pub estimated_cost: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

/// SLA statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaStats {
    pub server_id: String,
    pub time_range: crate::monitoring::TimeRange,
    pub uptime_percent: f64,
    pub downtime_minutes: u64,
    pub incidents: Vec<Incident>,
    pub availability_target: f64,
    pub meets_sla: bool,
    pub mttr_minutes: f64, // Mean Time To Recovery
    pub mtbf_minutes: f64, // Mean Time Between Failures
    pub monthly_availability: Vec<MonthlyAvailability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub duration_minutes: u64,
    pub severity: crate::monitoring::alerts::AlertSeverity,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyAvailability {
    pub month: String,
    pub uptime_percent: f64,
    pub downtime_minutes: u64,
}

/// Metric aggregation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationType {
    Average,
    Min,
    Max,
    Sum,
    Count,
    P95,
    P99,
    StdDev,
}

/// Metric query for flexible data retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricQuery {
    pub server_ids: Vec<String>,
    pub metric_types: Vec<MetricType>,
    pub start_time: u64,
    pub end_time: u64,
    pub aggregation: AggregationType,
    pub interval_secs: u64,
    pub filters: HashMap<String, String>,
}

/// Metric query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricQueryResult {
    pub query: MetricQuery,
    pub series: Vec<MetricSeries>,
    pub summary: MetricSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSeries {
    pub server_id: String,
    pub metric_type: MetricType,
    pub points: Vec<MetricPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSummary {
    pub total_points: usize,
    pub min_value: f64,
    pub max_value: f64,
    pub avg_value: f64,
}
