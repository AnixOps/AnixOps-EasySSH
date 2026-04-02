//! Real-time monitoring session for individual servers
//!
//! Provides lightweight, real-time system metrics collection via SSH
//! using direct /proc filesystem reads (avoiding command execution).

use crate::monitoring::alerts::{condition_description, AlertRule};
use crate::monitoring::metrics::SystemMetrics;
use crate::monitoring::MonitoringError;
use std::collections::VecDeque;
use std::io::Read;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};

/// Maximum history size to prevent unbounded memory growth
const MAX_HISTORY_SIZE: usize = 8640; // 24 hours at 10-second intervals

/// Default collection interval in seconds
const DEFAULT_INTERVAL_SECS: u64 = 5;

/// Real-time monitoring session for a single server
pub struct MonitoringSession {
    /// Server identifier
    pub server_id: String,
    /// SSH session for executing commands
    ssh_session: Arc<RwLock<Option<ssh2::Session>>>,
    /// Historical metrics data (circular buffer)
    metrics_history: Arc<RwLock<VecDeque<SystemMetrics>>>,
    /// Alert rules for this session
    alert_rules: Arc<RwLock<Vec<AlertRule>>>,
    /// Collection interval in seconds
    interval_secs: u64,
    /// Channel for control messages
    control_tx: Arc<RwLock<Option<mpsc::Sender<SessionControl>>>>,
    /// Whether the session is running
    running: Arc<RwLock<bool>>,
    /// Last network counters for calculating delta
    last_network_rx: Arc<RwLock<u64>>,
    last_network_tx: Arc<RwLock<u64>>,
    /// Server connection configuration
    connection_config: Arc<RwLock<Option<ServerConnectionInfo>>>,
}

/// Server connection information
#[derive(Debug, Clone)]
pub struct ServerConnectionInfo {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: AuthMethod,
}

/// Authentication method
#[derive(Debug, Clone)]
pub enum AuthMethod {
    Password(String),
    PrivateKey {
        key_path: String,
        passphrase: Option<String>,
    },
    Agent,
}

/// Control messages for the monitoring session
#[derive(Debug, Clone)]
enum SessionControl {
    Stop,
    UpdateInterval(u64),
    AddAlertRule(AlertRule),
    RemoveAlertRule(String),
    ForceCollection,
}

impl MonitoringSession {
    /// Create a new monitoring session
    pub fn new(server_id: String) -> Self {
        Self {
            server_id,
            ssh_session: Arc::new(RwLock::new(None)),
            metrics_history: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_HISTORY_SIZE))),
            alert_rules: Arc::new(RwLock::new(Vec::new())),
            interval_secs: DEFAULT_INTERVAL_SECS,
            control_tx: Arc::new(RwLock::new(None)),
            running: Arc::new(RwLock::new(false)),
            last_network_rx: Arc::new(RwLock::new(0)),
            last_network_tx: Arc::new(RwLock::new(0)),
            connection_config: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new session with specific interval
    pub fn with_interval(server_id: String, interval_secs: u64) -> Self {
        let mut session = Self::new(server_id);
        session.interval_secs = interval_secs.max(1);
        session
    }

    /// Configure server connection
    pub async fn configure_connection(
        &self,
        host: String,
        port: u16,
        username: String,
        auth_method: AuthMethod,
    ) {
        let mut config = self.connection_config.write().await;
        *config = Some(ServerConnectionInfo {
            host,
            port,
            username,
            auth_method,
        });
    }

    /// Connect to the server via SSH
    pub async fn connect(&self) -> Result<(), MonitoringError> {
        let config = self
            .connection_config
            .read()
            .await
            .clone()
            .ok_or_else(|| MonitoringError::Config("Connection not configured".to_string()))?;

        // Establish SSH connection
        let tcp = std::net::TcpStream::connect(format!("{}:{}", config.host, config.port))
            .map_err(|e| MonitoringError::Ssh(format!("TCP connection failed: {}", e)))?;

        let mut session = ssh2::Session::new()
            .map_err(|e| MonitoringError::Ssh(format!("Failed to create SSH session: {}", e)))?;

        session
            .set_tcp_stream(tcp);

        session
            .handshake()
            .map_err(|e| MonitoringError::Ssh(format!("SSH handshake failed: {}", e)))?;

        // Authenticate
        match config.auth_method {
            AuthMethod::Password(password) => {
                session
                    .userauth_password(&config.username, &password)
                    .map_err(|e| MonitoringError::Ssh(format!("Password auth failed: {}", e)))?;
            }
            AuthMethod::PrivateKey { key_path, passphrase } => {
                let pubkey = std::path::Path::new(&key_path);
                if pubkey.exists() {
                    session
                        .userauth_pubkey_file(
                            &config.username,
                            None,
                            pubkey,
                            passphrase.as_deref(),
                        )
                        .map_err(|e| MonitoringError::Ssh(format!("Key auth failed: {}", e)))?;
                } else {
                    return Err(MonitoringError::Ssh(format!(
                        "Private key not found: {}",
                        key_path
                    )));
                }
            }
            AuthMethod::Agent => {
                session
                    .userauth_agent(&config.username)
                    .map_err(|e| MonitoringError::Ssh(format!("Agent auth failed: {}", e)))?;
            }
        }

        let mut ssh_session = self.ssh_session.write().await;
        *ssh_session = Some(session);

        log::info!("Monitoring session connected for {}", self.server_id);
        Ok(())
    }

    /// Disconnect from the server
    pub async fn disconnect(&self) {
        let mut session = self.ssh_session.write().await;
        if let Some(mut s) = session.take() {
            let _ = s.disconnect(None, "Monitoring session closed", None);
        }
        *self.running.write().await = false;
        log::info!("Monitoring session disconnected for {}", self.server_id);
    }

    /// Start the monitoring loop
    pub async fn start(&self) -> Result<(), MonitoringError> {
        if *self.running.read().await {
            return Ok(());
        }

        // Ensure we have a connection
        if self.ssh_session.read().await.is_none() {
            self.connect().await?;
        }

        *self.running.write().await = true;

        let (tx, mut rx) = mpsc::channel(10);
        *self.control_tx.write().await = Some(tx);

        let server_id = self.server_id.clone();
        let interval_secs = self.interval_secs;
        let running = Arc::clone(&self.running);
        let history = Arc::clone(&self.metrics_history);
        let ssh_session = Arc::clone(&self.ssh_session);
        let last_rx = Arc::clone(&self.last_network_rx);
        let last_tx = Arc::clone(&self.last_network_tx);
        let alert_rules = Arc::clone(&self.alert_rules);

        // Spawn collection task
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));

            while *running.read().await {
                tokio::select! {
                    _ = ticker.tick() => {
                        match Self::collect_metrics(&ssh_session, &last_rx, &last_tx).await {
                            Ok(metrics) => {
                                // Store in history
                                let mut hist = history.write().await;
                                if hist.len() >= MAX_HISTORY_SIZE {
                                    hist.pop_front();
                                }
                                hist.push_back(metrics.clone());
                                drop(hist);

                                // Check alert rules
                                Self::check_alerts(&alert_rules, &metrics, &server_id).await;

                                log::debug!("Collected metrics for {}: CPU={:.1}%, Mem={:.1}%",
                                    server_id, metrics.cpu_percent, metrics.memory_percent());
                            }
                            Err(e) => {
                                log::error!("Failed to collect metrics for {}: {}", server_id, e);
                            }
                        }
                    }
                    Some(control) = rx.recv() => {
                        match control {
                            SessionControl::Stop => {
                                log::info!("Stopping monitoring for {}", server_id);
                                break;
                            }
                            SessionControl::UpdateInterval(new_interval) => {
                                ticker = interval(Duration::from_secs(new_interval));
                                log::info!("Updated interval for {} to {}s", server_id, new_interval);
                            }
                            SessionControl::AddAlertRule(rule) => {
                                let mut rules = alert_rules.write().await;
                                rules.push(rule);
                                log::info!("Added alert rule for {}", server_id);
                            }
                            SessionControl::RemoveAlertRule(rule_id) => {
                                let mut rules = alert_rules.write().await;
                                rules.retain(|r| r.id != rule_id);
                                log::info!("Removed alert rule {} from {}", rule_id, server_id);
                            }
                            SessionControl::ForceCollection => {
                                // Force immediate collection
                                match Self::collect_metrics(&ssh_session, &last_rx, &last_tx).await {
                                    Ok(metrics) => {
                                        let mut hist = history.write().await;
                                        if hist.len() >= MAX_HISTORY_SIZE {
                                            hist.pop_front();
                                        }
                                        hist.push_back(metrics);
                                    }
                                    Err(e) => {
                                        log::error!("Forced collection failed for {}: {}", server_id, e);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            *running.write().await = false;
        });

        log::info!(
            "Started monitoring session for {} with {}s interval",
            self.server_id,
            self.interval_secs
        );
        Ok(())
    }

    /// Stop the monitoring session
    pub async fn stop(&self) {
        if let Some(tx) = self.control_tx.write().await.take() {
            let _ = tx.send(SessionControl::Stop).await;
        }
    }

    /// Collect metrics via SSH using /proc filesystem reads
    async fn collect_metrics(
        ssh_session: &Arc<RwLock<Option<ssh2::Session>>>,
        last_rx: &Arc<RwLock<u64>>,
        last_tx: &Arc<RwLock<u64>>,
    ) -> Result<SystemMetrics, MonitoringError> {
        let session_guard = ssh_session.read().await;
        let session = session_guard
            .as_ref()
            .ok_or_else(|| MonitoringError::Collection("Not connected".to_string()))?;

        // Execute metrics collection script that reads from /proc
        let mut channel = session
            .channel_session()
            .map_err(|e| MonitoringError::Ssh(format!("Failed to create channel: {}", e)))?;

        let script = r#"#!/bin/bash
# Read CPU metrics from /proc/stat
read cpu user nice system idle iowait irq softirq steal guest guest_nice < /proc/stat
echo "CPU:$user $nice $system $idle $iowait $irq $softirq $steal"

# Read memory from /proc/meminfo
mem_total=$(grep '^MemTotal:' /proc/meminfo | awk '{print $2}')
mem_free=$(grep '^MemFree:' /proc/meminfo | awk '{print $2}')
mem_buffers=$(grep '^Buffers:' /proc/meminfo | awk '{print $2}')
mem_cached=$(grep '^Cached:' /proc/meminfo | awk '{print $2}')
echo "MEM:$mem_total $mem_free $mem_buffers $mem_cached"

# Read disk usage from df (more reliable than /proc for filesystem info)
df -B1 / 2>/dev/null | tail -1 | awk '{print "DISK:"$2","$3}'

# Read network from /proc/net/dev
net_line=$(grep -E '^\s*(eth|ens|enp|wlan|wlp)' /proc/net/dev | head -1 | awk '{print $2","$10}')
echo "NET:$net_line"

# Read load average from /proc/loadavg
read load1 load5 load15 rest < /proc/loadavg
echo "LOAD:$load1 $load5 $load15"
"#;

        channel
            .exec(script)
            .map_err(|e| MonitoringError::Ssh(format!("Failed to execute: {}", e)))?;

        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .map_err(|e| MonitoringError::Ssh(format!("Failed to read output: {}", e)))?;

        channel
            .wait_close()
            .map_err(|e| MonitoringError::Ssh(format!("Failed to close channel: {}", e)))?;

        // Parse the output
        Self::parse_metrics_output(&output, last_rx, last_tx).await
    }

    /// Parse metrics output from the collection script
    async fn parse_metrics_output(
        output: &str,
        last_rx: &Arc<RwLock<u64>>,
        last_tx: &Arc<RwLock<u64>>,
    ) -> Result<SystemMetrics, MonitoringError> {
        let mut cpu_percent = 0.0f32;
        let mut memory_total = 0u64;
        let mut memory_used = 0u64;
        let mut disk_total = 0u64;
        let mut disk_used = 0u64;
        let mut network_rx = 0u64;
        let mut network_tx = 0u64;
        let mut load_avg = [0.0f32; 3];

        for line in output.lines() {
            if line.starts_with("CPU:") {
                // Parse: CPU:user nice system idle iowait irq softirq steal
                let parts: Vec<&str> = line[4..].split_whitespace().collect();
                if parts.len() >= 8 {
                    let user: f64 = parts[0].parse().unwrap_or(0.0);
                    let nice: f64 = parts[1].parse().unwrap_or(0.0);
                    let system: f64 = parts[2].parse().unwrap_or(0.0);
                    let idle: f64 = parts[3].parse().unwrap_or(0.0);
                    let iowait: f64 = parts[4].parse().unwrap_or(0.0);
                    let irq: f64 = parts[5].parse().unwrap_or(0.0);
                    let softirq: f64 = parts[6].parse().unwrap_or(0.0);
                    let steal: f64 = parts[7].parse().unwrap_or(0.0);

                    let total = user + nice + system + idle + iowait + irq + softirq + steal;
                    let active = user + nice + system + irq + softirq + steal;

                    if total > 0.0 {
                        cpu_percent = ((active / total) * 100.0) as f32;
                    }
                }
            } else if line.starts_with("MEM:") {
                // Parse: MEM:total free buffers cached
                let parts: Vec<&str> = line[4..].split_whitespace().collect();
                if parts.len() >= 4 {
                    let total_kb: u64 = parts[0].parse().unwrap_or(0);
                    let free_kb: u64 = parts[1].parse().unwrap_or(0);
                    let buffers_kb: u64 = parts[2].parse().unwrap_or(0);
                    let cached_kb: u64 = parts[3].parse().unwrap_or(0);

                    // Convert KB to bytes
                    memory_total = total_kb * 1024;
                    let memory_free = free_kb * 1024;
                    let memory_buffers = buffers_kb * 1024;
                    let memory_cached = cached_kb * 1024;

                    // Calculate used memory (excluding buffers/cache)
                    memory_used = memory_total.saturating_sub(memory_free + memory_buffers + memory_cached);
                }
            } else if line.starts_with("DISK:") {
                // Parse: DISK:total,used
                let disk_data = &line[5..];
                let parts: Vec<&str> = disk_data.split(',').collect();
                if parts.len() >= 2 {
                    disk_total = parts[0].parse().unwrap_or(0);
                    disk_used = parts[1].parse().unwrap_or(0);
                }
            } else if line.starts_with("NET:") {
                // Parse: NET:rx_bytes,tx_bytes
                let net_data = &line[4..];
                let parts: Vec<&str> = net_data.split(',').collect();
                if parts.len() >= 2 {
                    let current_rx: u64 = parts[0].trim().parse().unwrap_or(0);
                    let current_tx: u64 = parts[1].trim().parse().unwrap_or(0);

                    // Calculate delta from last measurement
                    let last_rx_val = *last_rx.read().await;
                    let last_tx_val = *last_tx.read().await;

                    if last_rx_val > 0 && current_rx >= last_rx_val {
                        network_rx = current_rx - last_rx_val;
                    }
                    if last_tx_val > 0 && current_tx >= last_tx_val {
                        network_tx = current_tx - last_tx_val;
                    }

                    // Update last values
                    *last_rx.write().await = current_rx;
                    *last_tx.write().await = current_tx;
                }
            } else if line.starts_with("LOAD:") {
                // Parse: LOAD:1min 5min 15min
                let parts: Vec<&str> = line[5..].split_whitespace().collect();
                if parts.len() >= 3 {
                    load_avg[0] = parts[0].parse().unwrap_or(0.0);
                    load_avg[1] = parts[1].parse().unwrap_or(0.0);
                    load_avg[2] = parts[2].parse().unwrap_or(0.0);
                }
            }
        }

        Ok(SystemMetrics::new(
            cpu_percent,
            memory_used,
            memory_total,
            disk_used,
            disk_total,
            network_rx,
            network_tx,
            load_avg,
        ))
    }

    /// Check alert rules against current metrics
    async fn check_alerts(
        alert_rules: &Arc<RwLock<Vec<AlertRule>>>,
        metrics: &SystemMetrics,
        server_id: &str,
    ) {
        let rules = alert_rules.read().await;

        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            // Extract value based on metric type
            let value = match rule.metric_type {
                crate::monitoring::metrics::MetricType::CpuUsage => metrics.cpu_percent as f64,
                crate::monitoring::metrics::MetricType::MemoryUsage => metrics.memory_percent() as f64,
                crate::monitoring::metrics::MetricType::DiskUsage => metrics.disk_percent() as f64,
                crate::monitoring::metrics::MetricType::CpuLoad1 => metrics.load_avg[0] as f64,
                crate::monitoring::metrics::MetricType::CpuLoad5 => metrics.load_avg[1] as f64,
                crate::monitoring::metrics::MetricType::CpuLoad15 => metrics.load_avg[2] as f64,
                _ => continue,
            };

            let triggered = rule.condition.evaluate(value, rule.threshold);

            if triggered {
                log::warn!(
                    "Alert triggered for {}: {} {} {} (value: {:.2})",
                    server_id,
                    format!("{:?}", rule.metric_type),
                    crate::monitoring::alerts::condition_description(&rule.condition),
                    rule.threshold,
                    value
                );
            }
        }
    }

    /// Get the latest metrics
    pub async fn get_latest_metrics(&self) -> Option<SystemMetrics> {
        let history = self.metrics_history.read().await;
        history.back().cloned()
    }

    /// Get metrics history
    pub async fn get_history(&self, limit: Option<usize>) -> Vec<SystemMetrics> {
        let history = self.metrics_history.read().await;
        let limit = limit.unwrap_or(history.len());
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Add an alert rule
    pub async fn add_alert_rule(&self, rule: AlertRule) -> Result<(), MonitoringError> {
        if let Some(tx) = self.control_tx.read().await.as_ref() {
            tx.send(SessionControl::AddAlertRule(rule))
                .await
                .map_err(|e| MonitoringError::Alert(e.to_string()))?;
        }
        Ok(())
    }

    /// Remove an alert rule
    pub async fn remove_alert_rule(&self, rule_id: &str) -> Result<(), MonitoringError> {
        if let Some(tx) = self.control_tx.read().await.as_ref() {
            tx.send(SessionControl::RemoveAlertRule(rule_id.to_string()))
                .await
                .map_err(|e| MonitoringError::Alert(e.to_string()))?;
        }
        Ok(())
    }

    /// Update collection interval
    pub async fn update_interval(&self, interval_secs: u64) -> Result<(), MonitoringError> {
        if let Some(tx) = self.control_tx.read().await.as_ref() {
            tx.send(SessionControl::UpdateInterval(interval_secs.max(1)))
                .await
                .map_err(|e| MonitoringError::Config(e.to_string()))?;
        }
        Ok(())
    }

    /// Force immediate metric collection
    pub async fn force_collection(&self) -> Result<(), MonitoringError> {
        if let Some(tx) = self.control_tx.read().await.as_ref() {
            tx.send(SessionControl::ForceCollection)
                .await
                .map_err(|e| MonitoringError::Collection(e.to_string()))?;
        }
        Ok(())
    }

    /// Check if session is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get the number of stored metric points
    pub async fn history_count(&self) -> usize {
        self.metrics_history.read().await.len()
    }

    /// Get alert rules count
    pub async fn alert_rules_count(&self) -> usize {
        self.alert_rules.read().await.len()
    }
}

impl Drop for MonitoringSession {
    fn drop(&mut self) {
        // Attempt to clean up
        let running = self.running.clone();
        let session = self.ssh_session.clone();

        // We can't use .await in drop, so we just set the flag
        // The spawned task will clean up when it sees running=false
        let _ = try_block_on(async move {
            *running.write().await = false;
            let mut s = session.write().await;
            if let Some(mut sess) = s.take() {
                let _ = sess.disconnect(None, "Session dropped", None);
            }
        });
    }
}

/// Helper function to try to run async code in sync context
fn try_block_on<F, T>(f: F) -> Option<T>
where
    F: std::future::Future<Output = T>,
{
    // Try to get current runtime, if not available return None
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => Some(handle.block_on(f)),
        Err(_) => None,
    }
}

/// Chart data generator for metrics visualization
pub struct ChartData;

impl ChartData {
    /// Generate sparkline data (compact line chart) from metrics history
    pub fn generate_sparkline(history: &[SystemMetrics], metric_type: &str) -> Vec<f64> {
        match metric_type {
            "cpu" => history.iter().map(|m| m.cpu_percent as f64).collect(),
            "memory" => history.iter().map(|m| m.memory_percent() as f64).collect(),
            "disk" => history.iter().map(|m| m.disk_percent() as f64).collect(),
            "load1" => history.iter().map(|m| m.load_avg[0] as f64).collect(),
            "load5" => history.iter().map(|m| m.load_avg[1] as f64).collect(),
            "load15" => history.iter().map(|m| m.load_avg[2] as f64).collect(),
            "network_rx" => history.iter().map(|m| m.network_rx as f64).collect(),
            "network_tx" => history.iter().map(|m| m.network_tx as f64).collect(),
            _ => Vec::new(),
        }
    }

    /// Generate time series data for charts
    pub fn generate_timeseries(
        history: &[SystemMetrics],
        metric_type: &str,
    ) -> Vec<(i64, f64)> {
        let values = Self::generate_sparkline(history, metric_type);
        history
            .iter()
            .zip(values.iter())
            .map(|(m, v)| (m.timestamp.timestamp(), *v))
            .collect()
    }

    /// Calculate min/max/avg statistics for a metric
    pub fn calculate_stats(history: &[SystemMetrics], metric_type: &str) -> MetricStats {
        let values = Self::generate_sparkline(history, metric_type);

        if values.is_empty() {
            return MetricStats::default();
        }

        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let avg = values.iter().sum::<f64>() / values.len() as f64;

        // Calculate percentiles
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95_idx = ((sorted.len() as f64) * 0.95) as usize;
        let p99_idx = ((sorted.len() as f64) * 0.99) as usize;
        let p95 = sorted.get(p95_idx).copied().unwrap_or(0.0);
        let p99 = sorted.get(p99_idx).copied().unwrap_or(0.0);

        MetricStats {
            min,
            max,
            avg,
            p95,
            p99,
            count: values.len(),
        }
    }

    /// Generate bar chart data for resource usage comparison
    pub fn generate_resource_comparison(
        history: &[SystemMetrics],
    ) -> Vec<(String, f64, String)> {
        if history.is_empty() {
            return Vec::new();
        }

        let latest = history.last().unwrap();

        vec![
            (
                "CPU".to_string(),
                latest.cpu_percent as f64,
                Self::percentage_color(latest.cpu_percent as f64),
            ),
            (
                "Memory".to_string(),
                latest.memory_percent() as f64,
                Self::percentage_color(latest.memory_percent() as f64),
            ),
            (
                "Disk".to_string(),
                latest.disk_percent() as f64,
                Self::percentage_color(latest.disk_percent() as f64),
            ),
        ]
    }

    /// Get color based on percentage (for UI)
    fn percentage_color(pct: f64) -> String {
        if pct >= 90.0 {
            "#ef4444".to_string() // Red
        } else if pct >= 70.0 {
            "#f59e0b".to_string() // Orange
        } else {
            "#22c55e".to_string() // Green
        }
    }
}

/// Metric statistics
#[derive(Debug, Clone, Default)]
pub struct MetricStats {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub p95: f64,
    pub p99: f64,
    pub count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_metrics() {
        let metrics = SystemMetrics::new(
            45.5,
            4 * 1024 * 1024 * 1024,  // 4GB used
            16 * 1024 * 1024 * 1024, // 16GB total
            100 * 1024 * 1024 * 1024,
            500 * 1024 * 1024 * 1024,
            1024 * 1024, // 1MB RX
            512 * 1024,  // 512KB TX
            [0.5, 0.3, 0.2],
        );

        assert_eq!(metrics.cpu_percent, 45.5);
        assert!(metrics.memory_percent() > 24.0 && metrics.memory_percent() < 26.0);
        assert!(metrics.disk_percent() > 19.0 && metrics.disk_percent() < 21.0);
    }

    #[test]
    fn test_chart_data_sparkline() {
        let history = vec![
            SystemMetrics::new(10.0, 0, 16, 0, 500, 0, 0, [0.1, 0.1, 0.1]),
            SystemMetrics::new(20.0, 0, 16, 0, 500, 0, 0, [0.2, 0.2, 0.2]),
            SystemMetrics::new(30.0, 0, 16, 0, 500, 0, 0, [0.3, 0.3, 0.3]),
        ];

        let sparkline = ChartData::generate_sparkline(&history, "cpu");
        assert_eq!(sparkline, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_metric_stats() {
        let history = vec![
            SystemMetrics::new(10.0, 0, 16, 0, 500, 0, 0, [0.1, 0.1, 0.1]),
            SystemMetrics::new(20.0, 0, 16, 0, 500, 0, 0, [0.2, 0.2, 0.2]),
            SystemMetrics::new(30.0, 0, 16, 0, 500, 0, 0, [0.3, 0.3, 0.3]),
        ];

        let stats = ChartData::calculate_stats(&history, "cpu");
        assert_eq!(stats.min, 10.0);
        assert_eq!(stats.max, 30.0);
        assert_eq!(stats.avg, 20.0);
        assert_eq!(stats.count, 3);
    }
}
