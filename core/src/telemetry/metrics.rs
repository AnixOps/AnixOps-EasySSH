//! Performance metrics collection
//!
//! Tracks:
//! - Startup time
//! - Operation latency
//! - Memory usage
//! - Connection performance
//! - UI responsiveness

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::TelemetryError;

/// Metric value types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
    Timing(u64), // milliseconds
}

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Timing,
}

/// Metric definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub metric_type: MetricType,
    pub description: String,
    pub unit: String,
    pub labels: HashMap<String, String>,
    pub value: MetricValue,
    pub timestamp: u64,
}

/// Counter metric - monotonically increasing
pub struct Counter {
    name: String,
    value: Arc<Mutex<u64>>,
    labels: HashMap<String, String>,
}

impl Counter {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: Arc::new(Mutex::new(0)),
            labels: HashMap::new(),
        }
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn increment(&self) {
        let mut val = self.value.lock().unwrap();
        *val += 1;
    }

    pub fn add(&self, delta: u64) {
        let mut val = self.value.lock().unwrap();
        *val += delta;
    }

    pub fn get(&self) -> u64 {
        *self.value.lock().unwrap()
    }
}

/// Gauge metric - can go up and down
pub struct Gauge {
    name: String,
    value: Arc<Mutex<f64>>,
    labels: HashMap<String, String>,
}

impl Gauge {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: Arc::new(Mutex::new(0.0)),
            labels: HashMap::new(),
        }
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn set(&self, value: f64) {
        let mut val = self.value.lock().unwrap();
        *val = value;
    }

    pub fn increment(&self, delta: f64) {
        let mut val = self.value.lock().unwrap();
        *val += delta;
    }

    pub fn decrement(&self, delta: f64) {
        let mut val = self.value.lock().unwrap();
        *val -= delta;
    }

    pub fn get(&self) -> f64 {
        *self.value.lock().unwrap()
    }
}

/// Histogram metric - distribution of values
pub struct Histogram {
    name: String,
    values: Arc<Mutex<Vec<f64>>>,
    max_samples: usize,
    labels: HashMap<String, String>,
}

impl Histogram {
    pub fn new(name: impl Into<String>, max_samples: usize) -> Self {
        Self {
            name: name.into(),
            values: Arc::new(Mutex::new(Vec::with_capacity(max_samples))),
            max_samples,
            labels: HashMap::new(),
        }
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn record(&self, value: f64) {
        let mut values = self.values.lock().unwrap();
        if values.len() >= self.max_samples {
            values.remove(0);
        }
        values.push(value);
    }

    pub fn get_statistics(&self) -> HistogramStatistics {
        let values = self.values.lock().unwrap();
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let count = sorted.len();
        if count == 0 {
            return HistogramStatistics::default();
        }

        let sum: f64 = sorted.iter().sum();
        let mean = sum / count as f64;

        let min = *sorted.first().unwrap();
        let max = *sorted.last().unwrap();

        let p50 = Self::percentile(&sorted, 0.5);
        let p95 = Self::percentile(&sorted, 0.95);
        let p99 = Self::percentile(&sorted, 0.99);

        HistogramStatistics {
            count,
            min,
            max,
            mean,
            p50,
            p95,
            p99,
        }
    }

    fn percentile(sorted: &[f64], p: f64) -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        let index = ((sorted.len() as f64 - 1.0) * p) as usize;
        sorted[index.min(sorted.len() - 1)]
    }
}

/// Histogram statistics
#[derive(Debug, Default, Clone, Copy)]
pub struct HistogramStatistics {
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

/// Timer for measuring durations
pub struct Timer {
    name: String,
    start: Instant,
    labels: HashMap<String, String>,
}

impl Timer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
            labels: HashMap::new(),
        }
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    pub fn elapsed_micros(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }

    pub fn stop(self) -> Duration {
        self.start.elapsed()
    }
}

/// Metrics registry
pub struct MetricsRegistry {
    counters: Arc<Mutex<HashMap<String, Counter>>>,
    gauges: Arc<Mutex<HashMap<String, Gauge>>>,
    histograms: Arc<Mutex<HashMap<String, Histogram>>>,
}

impl MetricsRegistry {
    pub fn new() -> Result<Self, TelemetryError> {
        Ok(Self {
            counters: Arc::new(Mutex::new(HashMap::new())),
            gauges: Arc::new(Mutex::new(HashMap::new())),
            histograms: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get or create counter
    pub fn counter(&self, name: impl Into<String>) -> Counter {
        let name = name.into();
        let mut counters = self.counters.lock().unwrap();

        if let Some(counter) = counters.get(&name) {
            return Counter {
                name: counter.name.clone(),
                value: Arc::clone(&counter.value),
                labels: counter.labels.clone(),
            };
        }

        let counter = Counter::new(name.clone());
        counters.insert(name.clone(), Counter {
            name: name.clone(),
            value: Arc::clone(&counter.value),
            labels: HashMap::new(),
        });
        counter
    }

    /// Get or create gauge
    pub fn gauge(&self, name: impl Into<String>) -> Gauge {
        let name = name.into();
        let mut gauges = self.gauges.lock().unwrap();

        if let Some(gauge) = gauges.get(&name) {
            return Gauge {
                name: gauge.name.clone(),
                value: Arc::clone(&gauge.value),
                labels: gauge.labels.clone(),
            };
        }

        let gauge = Gauge::new(name.clone());
        gauges.insert(name.clone(), Gauge {
            name: name.clone(),
            value: Arc::clone(&gauge.value),
            labels: HashMap::new(),
        });
        gauge
    }

    /// Get or create histogram
    pub fn histogram(&self, name: impl Into<String>, max_samples: usize) -> Histogram {
        let name = name.into();
        let mut histograms = self.histograms.lock().unwrap();

        if let Some(hist) = histograms.get(&name) {
            return Histogram {
                name: hist.name.clone(),
                values: Arc::clone(&hist.values),
                max_samples: hist.max_samples,
                labels: hist.labels.clone(),
            };
        }

        let hist = Histogram::new(name.clone(), max_samples);
        histograms.insert(name.clone(), Histogram {
            name: name.clone(),
            values: Arc::clone(&hist.values),
            max_samples,
            labels: HashMap::new(),
        });
        hist
    }

    /// Start a timer
    pub fn start_timer(&self, name: impl Into<String>) -> Timer {
        Timer::new(name)
    }

    /// Get all metrics snapshot
    pub fn snapshot(&self) -> Vec<Metric> {
        let mut metrics = Vec::new();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Collect counters
        let counters = self.counters.lock().unwrap();
        for (name, counter) in counters.iter() {
            metrics.push(Metric {
                name: name.clone(),
                metric_type: MetricType::Counter,
                description: String::new(),
                unit: "count".to_string(),
                labels: counter.labels.clone(),
                value: MetricValue::Counter(counter.get()),
                timestamp,
            });
        }
        drop(counters);

        // Collect gauges
        let gauges = self.gauges.lock().unwrap();
        for (name, gauge) in gauges.iter() {
            metrics.push(Metric {
                name: name.clone(),
                metric_type: MetricType::Gauge,
                description: String::new(),
                unit: "value".to_string(),
                labels: gauge.labels.clone(),
                value: MetricValue::Gauge(gauge.get()),
                timestamp,
            });
        }
        drop(gauges);

        // Collect histograms
        let histograms = self.histograms.lock().unwrap();
        for (name, hist) in histograms.iter() {
            let stats = hist.get_statistics();
            metrics.push(Metric {
                name: name.clone(),
                metric_type: MetricType::Histogram,
                description: String::new(),
                unit: "value".to_string(),
                labels: hist.labels.clone(),
                value: MetricValue::Histogram(vec![
                    stats.count as f64,
                    stats.min,
                    stats.max,
                    stats.mean,
                    stats.p50,
                    stats.p95,
                    stats.p99,
                ]),
                timestamp,
            });
        }

        metrics
    }

    /// Get system memory metrics
    pub fn get_memory_metrics() -> HashMap<String, f64> {
        let mut metrics = HashMap::new();

        // Try to get memory info from system
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(val) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = val.parse::<f64>() {
                                metrics.insert("memory_rss_mb".to_string(), kb / 1024.0);
                            }
                        }
                    } else if line.starts_with("VmSize:") {
                        if let Some(val) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = val.parse::<f64>() {
                                metrics.insert("memory_vms_mb".to_string(), kb / 1024.0);
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            // On macOS, use task_info or ps
            use std::process::Command;
            if let Ok(output) = Command::new("ps")
                .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
                .output()
            {
                if let Ok(rss) = String::from_utf8(output.stdout) {
                    if let Ok(kb) = rss.trim().parse::<f64>() {
                        metrics.insert("memory_rss_mb".to_string(), kb / 1024.0);
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows, use GetProcessMemoryInfo
            unsafe {
                use windows_sys::Win32::System::ProcessStatus::GetProcessMemoryInfo;
                use windows_sys::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS;
                use windows_sys::Win32::System::Threading::GetCurrentProcess;

                let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
                counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

                if GetProcessMemoryInfo(
                    GetCurrentProcess(),
                    &mut counters,
                    counters.cb,
                ) != 0
                {
                    metrics.insert(
                        "memory_working_set_mb".to_string(),
                        counters.WorkingSetSize as f64 / (1024.0 * 1024.0),
                    );
                    metrics.insert(
                        "memory_peak_mb".to_string(),
                        counters.PeakWorkingSetSize as f64 / (1024.0 * 1024.0),
                    );
                }
            }
        }

        metrics
    }

    /// Performance benchmark helper
    pub async fn benchmark<F, Fut, T>(name: &str, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = f().await;
        let elapsed = start.elapsed();

        println!("[Benchmark] {}: {:?}", name, elapsed);

        result
    }
}

/// Predefined performance metrics
pub struct PerformanceMetrics;

impl PerformanceMetrics {
    /// Track startup time
    pub fn track_startup_time(registry: &MetricsRegistry, duration_ms: u64) {
        registry
            .histogram("app_startup_time_ms", 100)
            .record(duration_ms as f64);
    }

    /// Track SSH connection time
    pub fn track_ssh_connection_time(registry: &MetricsRegistry, duration_ms: u64) {
        registry
            .histogram("ssh_connection_time_ms", 1000)
            .record(duration_ms as f64);
    }

    /// Track UI operation latency
    pub fn track_ui_latency(registry: &MetricsRegistry, operation: &str, duration_ms: u64) {
        registry
            .histogram(&format!("ui_latency_{}_ms", operation), 1000)
            .record(duration_ms as f64);
    }

    /// Track database operation time
    pub fn track_db_operation_time(registry: &MetricsRegistry, operation: &str, duration_ms: u64) {
        registry
            .histogram(&format!("db_operation_{}_ms", operation), 1000)
            .record(duration_ms as f64);
    }

    /// Track memory usage
    pub fn track_memory_usage(registry: &MetricsRegistry) {
        let memory_metrics = MetricsRegistry::get_memory_metrics();
        for (key, value) in memory_metrics {
            registry.gauge(&format!("system_{}", key)).set(value);
        }
    }

    /// Track active connections
    pub fn track_active_connections(registry: &MetricsRegistry, count: usize) {
        registry.gauge("active_ssh_connections").set(count as f64);
    }

    /// Track command execution time
    pub fn track_command_execution_time(registry: &MetricsRegistry, duration_ms: u64) {
        registry
            .histogram("command_execution_time_ms", 1000)
            .record(duration_ms as f64);
    }

    /// Track SFTP transfer speed
    pub fn track_sftp_transfer_speed(registry: &MetricsRegistry, bytes_per_second: f64) {
        registry
            .histogram("sftp_transfer_speed_bps", 1000)
            .record(bytes_per_second);
    }
}
