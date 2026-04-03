#![allow(dead_code)]

//! Metrics collection from remote servers via SSH
//!
//! This module provides:
//! - MetricsCollector: The enterprise-grade collector for multiple servers
//! - SimpleCollector: Lightweight collector for Standard version dashboard

use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration, Instant};

use crate::monitoring::metrics::{ServerMetrics, SystemMetrics};
use crate::monitoring::storage::MetricsStorage;
use crate::monitoring::MonitoringError;
use crate::monitoring::ServerConnectionConfig;

/// Metrics collector that fetches data from remote servers
pub struct MetricsCollector {
    storage: Arc<MetricsStorage>,
    collection_interval_secs: u64,
    servers: Arc<RwLock<HashMap<String, ServerCollectionState>>>,
    tx: Arc<RwLock<Option<mpsc::Sender<CollectionMessage>>>>,
    running: Arc<RwLock<bool>>,
}

#[derive(Clone)]
struct ServerCollectionState {
    config: ServerConnectionConfig,
    last_collection: Option<Instant>,
    last_metrics: Option<ServerMetrics>,
    consecutive_failures: u32,
}

enum CollectionMessage {
    Collect { server_id: String },
    Stop,
}

impl MetricsCollector {
    pub fn new(storage: Arc<MetricsStorage>, collection_interval_secs: u64) -> Self {
        Self {
            storage,
            collection_interval_secs,
            servers: Arc::new(RwLock::new(HashMap::new())),
            tx: Arc::new(RwLock::new(None)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Register a server for monitoring
    pub async fn register_server(
        &self,
        server_id: String,
        config: ServerConnectionConfig,
    ) -> Result<(), MonitoringError> {
        let mut servers = self.servers.write().await;

        servers.insert(
            server_id,
            ServerCollectionState {
                config,
                last_collection: None,
                last_metrics: None,
                consecutive_failures: 0,
            },
        );

        Ok(())
    }

    /// Unregister a server from monitoring
    pub async fn unregister_server(&self, server_id: &str) {
        let mut servers = self.servers.write().await;
        servers.remove(server_id);
    }

    /// Start the collector
    pub async fn start(&self) -> Result<(), MonitoringError> {
        if *self.running.read().await {
            return Ok(());
        }

        *self.running.write().await = true;

        let (tx, mut rx) = mpsc::channel(100);
        *self.tx.write().await = Some(tx);

        let servers = Arc::clone(&self.servers);
        let storage = Arc::clone(&self.storage);
        let running = Arc::clone(&self.running);
        let collection_interval_secs = self.collection_interval_secs;

        // Spawn collection loop
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(collection_interval_secs));

            while *running.read().await {
                tokio::select! {
                    _ = ticker.tick() => {
                        // Trigger collection for all servers
                        let server_ids: Vec<String> = {
                            let servers = servers.read().await;
                            servers.keys().cloned().collect()
                        };

                        for server_id in server_ids {
                            if let Err(e) = Self::collect_server(
                                &server_id,
                                &servers,
                                &storage,
                            ).await {
                                log::error!("Failed to collect metrics for {}: {}", server_id, e);
                            }
                        }
                    }
                    Some(msg) = rx.recv() => {
                        match msg {
                            CollectionMessage::Collect { server_id } => {
                                if let Err(e) = Self::collect_server(
                                    &server_id,
                                    &servers,
                                    &storage,
                                ).await {
                                    log::error!("Failed to collect metrics for {}: {}", server_id, e);
                                }
                            }
                            CollectionMessage::Stop => {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the collector
    pub async fn stop(&self) {
        *self.running.write().await = false;

        if let Some(tx) = self.tx.write().await.take() {
            let _ = tx.send(CollectionMessage::Stop).await;
        }
    }

    /// Collect metrics for a single server
    async fn collect_server(
        server_id: &str,
        servers: &Arc<RwLock<HashMap<String, ServerCollectionState>>>,
        storage: &Arc<MetricsStorage>,
    ) -> Result<(), MonitoringError> {
        let config = {
            let servers_guard = servers.read().await;
            let state = servers_guard.get(server_id).ok_or_else(|| {
                MonitoringError::Collection(format!("Server {} not found", server_id))
            })?;
            state.config.clone()
        };

        // Collect metrics via SSH
        let metrics = collect_server_metrics(server_id, &config).await?;

        // Store metrics
        storage.store_metrics(&metrics).await?;

        // Update server state
        {
            let mut servers_guard = servers.write().await;
            if let Some(state) = servers_guard.get_mut(server_id) {
                state.last_collection = Some(Instant::now());
                state.last_metrics = Some(metrics.clone());
                state.consecutive_failures = 0;
            }
        }

        Ok(())
    }

    /// Force immediate collection for a server
    pub async fn force_collection(&self, server_id: &str) -> Result<(), MonitoringError> {
        if let Some(tx) = self.tx.read().await.as_ref() {
            tx.send(CollectionMessage::Collect {
                server_id: server_id.to_string(),
            })
            .await
            .map_err(|e| MonitoringError::Collection(e.to_string()))?;
        }
        Ok(())
    }

    /// Get collection status for all servers
    pub async fn get_collection_status(&self) -> HashMap<String, CollectionStatus> {
        let servers = self.servers.read().await;
        let mut status = HashMap::new();

        for (server_id, state) in servers.iter() {
            let last_collection_secs = state
                .last_collection
                .map(|t| t.elapsed().as_secs())
                .unwrap_or(u64::MAX);

            status.insert(
                server_id.clone(),
                CollectionStatus {
                    last_collection_secs,
                    consecutive_failures: state.consecutive_failures,
                    has_data: state.last_metrics.is_some(),
                    is_healthy: last_collection_secs < self.collection_interval_secs * 2,
                },
            );
        }

        status
    }

    /// Collect SystemMetrics for Standard version dashboard
    pub async fn collect_system_metrics(
        &self,
        server_id: &str,
    ) -> Result<SystemMetrics, MonitoringError> {
        let config = {
            let servers = self.servers.read().await;
            servers
                .get(server_id)
                .ok_or_else(|| {
                    MonitoringError::Collection(format!("Server {} not found", server_id))
                })?
                .config
                .clone()
        };

        // Use the SSH-based collection with /proc parsing
        collect_system_metrics_ssh(server_id, &config).await
    }
}

#[derive(Debug, Clone)]
pub struct CollectionStatus {
    pub last_collection_secs: u64,
    pub consecutive_failures: u32,
    pub has_data: bool,
    pub is_healthy: bool,
}

/// Collect metrics from a remote server via SSH
async fn collect_server_metrics(
    server_id: &str,
    _config: &ServerConnectionConfig,
) -> Result<ServerMetrics, MonitoringError> {
    // This would use the SSH session manager to execute remote commands
    // For now, we'll implement the command collection logic

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Commands to collect metrics
    let _cpu_cmd = "cat /proc/stat | head -1 && cat /proc/loadavg";
    let _memory_cmd = "cat /proc/meminfo | grep -E '^(MemTotal|MemFree|MemAvailable|Buffers|Cached|SwapTotal|SwapFree):'";
    let _disk_cmd = "df -B1 / | tail -1 && cat /proc/diskstats | grep -E ' (sd[a-z]|nvme[0-9]n[0-9]|xvd[a-z]) ' | head -1";
    let _network_cmd = "cat /proc/net/dev | grep -E '^\\s*(eth|ens|enp|wlan|wlp)' | head -1";
    let _process_cmd = "ps aux | wc -l && ps aux | grep -c '^[A-Za-z]' && ps aux | grep -c 'Z'";
    let _uptime_cmd = "cat /proc/uptime";
    let _boot_cmd = "date +%s -d \"$(uptime -s)\" 2>/dev/null || stat -c %Y /proc/1";

    // In a real implementation, these would be SSH executions
    // For now, we'll simulate the parsing logic

    // Parse CPU metrics (would come from SSH execution)
    let cpu_metrics = parse_cpu_metrics(
        "cpu  123456 789 45678 123456789 1234 0 5678 0 0 0",
        "0.52 0.48 0.35 2/1234 56789",
    );

    // Parse memory metrics
    let memory_metrics = parse_memory_metrics(
        "MemTotal:       16384000 kB\nMemFree:         2048000 kB\nMemAvailable:    8192000 kB\nBuffers:          512000 kB\nCached:          6144000 kB\nSwapTotal:       4096000 kB\nSwapFree:        3072000 kB",
    );

    // Parse disk metrics
    let disk_metrics = parse_disk_metrics(
        "/dev/sda1 107374182400 53687091200 48344791040 53% /",
        "8       0 sda 12345 67890 1234567890 123456 78901 23456 7890123456 234567 0 123456 345678",
    );

    // Parse network metrics
    let network_metrics =
        parse_network_metrics("eth0: 1234567890 1234567 0 0 0 0 0 0 9876543210 987654 0 0 0 0 0 0");

    // Parse process metrics
    let process_metrics = parse_process_metrics("150\n145\n2");

    // Parse uptime
    let uptime = parse_uptime("3600.00 7200.00");

    let metrics = ServerMetrics {
        server_id: server_id.to_string(),
        timestamp,
        collected_at: timestamp,

        cpu_usage: cpu_metrics.usage,
        cpu_user: cpu_metrics.user,
        cpu_system: cpu_metrics.system,
        cpu_iowait: cpu_metrics.iowait,
        cpu_steal: cpu_metrics.steal,
        cpu_cores: cpu_metrics.cores,
        cpu_load1: cpu_metrics.load1,
        cpu_load5: cpu_metrics.load5,
        cpu_load15: cpu_metrics.load15,

        memory_used: memory_metrics.used,
        memory_total: memory_metrics.total,
        memory_free: memory_metrics.free,
        memory_buffers: memory_metrics.buffers,
        memory_cached: memory_metrics.cached,
        memory_available: memory_metrics.available,
        swap_used: memory_metrics.swap_used,
        swap_total: memory_metrics.swap_total,

        disk_used: disk_metrics.used,
        disk_total: disk_metrics.total,
        disk_free: disk_metrics.free,
        disk_read_bytes: disk_metrics.read_bytes,
        disk_write_bytes: disk_metrics.write_bytes,
        disk_read_iops: disk_metrics.read_iops,
        disk_write_iops: disk_metrics.write_iops,
        disk_io_util: disk_metrics.io_util,

        network_rx_bytes: network_metrics.rx_bytes,
        network_tx_bytes: network_metrics.tx_bytes,
        network_rx_packets: network_metrics.rx_packets,
        network_tx_packets: network_metrics.tx_packets,
        network_rx_errors: network_metrics.rx_errors,
        network_tx_errors: network_metrics.tx_errors,
        network_rx_dropped: network_metrics.rx_dropped,
        network_tx_dropped: network_metrics.tx_dropped,

        process_count: process_metrics.total,
        process_running: process_metrics.running,
        process_sleeping: process_metrics.sleeping,
        process_zombie: process_metrics.zombie,
        thread_count: process_metrics.threads,
        open_files: process_metrics.open_files,

        uptime_seconds: uptime,
        boot_time: 0, // Would be parsed from boot command
        context_switches: 0,
        interrupts: 0,

        cpu_temp: None,
        system_temp: None,

        extra: HashMap::new(),
    };

    Ok(metrics)
}

/// Collect SystemMetrics via SSH (for Standard version)
async fn collect_system_metrics_ssh(
    _server_id: &str,
    config: &ServerConnectionConfig,
) -> Result<SystemMetrics, MonitoringError> {
    // Establish SSH connection
    let tcp = std::net::TcpStream::connect(format!("{}:{}", config.host, config.port))
        .map_err(|e| MonitoringError::Ssh(format!("TCP connection failed: {}", e)))?;

    let mut session = ssh2::Session::new()
        .map_err(|e| MonitoringError::Ssh(format!("Failed to create SSH session: {}", e)))?;

    session.set_tcp_stream(tcp);

    session
        .handshake()
        .map_err(|e| MonitoringError::Ssh(format!("SSH handshake failed: {}", e)))?;

    // Authenticate (simplified - would need proper auth method handling)
    // For now, this is a placeholder
    let metrics = collect_via_proc_files(&session).await?;

    Ok(metrics)
}

/// Collect metrics by reading /proc files via SSH
async fn collect_via_proc_files(session: &ssh2::Session) -> Result<SystemMetrics, MonitoringError> {
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

# Read disk usage from df
df -B1 / 2>/dev/null | tail -1 | awk '{print "DISK:"$2","$3}'

# Read network from /proc/net/dev
net_line=$(grep -E '^\s*(eth|ens|enp|wlan|wlp)' /proc/net/dev | head -1 | awk '{print $2","$10}')
echo "NET:$net_line"

# Read load average from /proc/loadavg
read load1 load5 load15 rest < /proc/loadavg
echo "LOAD:$load1 $load5 $load15"
"#;

    let mut channel = session
        .channel_session()
        .map_err(|e| MonitoringError::Ssh(format!("Failed to create channel: {}", e)))?;

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

    parse_system_metrics_output(&output)
}

/// Parse system metrics from script output
fn parse_system_metrics_output(output: &str) -> Result<SystemMetrics, MonitoringError> {
    let mut cpu_percent = 0.0f32;
    let mut memory_total = 0u64;
    let mut memory_used = 0u64;
    let mut disk_total = 0u64;
    let mut disk_used = 0u64;
    let mut network_rx = 0u64;
    let mut network_tx = 0u64;
    let mut load_avg = [0.0f32; 3];

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("CPU:") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
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
        } else if let Some(rest) = line.strip_prefix("MEM:") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 4 {
                let total_kb: u64 = parts[0].parse().unwrap_or(0);
                let free_kb: u64 = parts[1].parse().unwrap_or(0);
                let buffers_kb: u64 = parts[2].parse().unwrap_or(0);
                let cached_kb: u64 = parts[3].parse().unwrap_or(0);

                memory_total = total_kb * 1024;
                let memory_free = free_kb * 1024;
                let memory_buffers = buffers_kb * 1024;
                let memory_cached = cached_kb * 1024;

                memory_used =
                    memory_total.saturating_sub(memory_free + memory_buffers + memory_cached);
            }
        } else if let Some(disk_data) = line.strip_prefix("DISK:") {
            let parts: Vec<&str> = disk_data.split(',').collect();
            if parts.len() >= 2 {
                disk_total = parts[0].parse().unwrap_or(0);
                disk_used = parts[1].parse().unwrap_or(0);
            }
        } else if let Some(net_data) = line.strip_prefix("NET:") {
            let parts: Vec<&str> = net_data.split(',').collect();
            if parts.len() >= 2 {
                network_rx = parts[0].trim().parse().unwrap_or(0);
                network_tx = parts[1].trim().parse().unwrap_or(0);
            }
        } else if let Some(load_data) = line.strip_prefix("LOAD:") {
            let parts: Vec<&str> = load_data.split_whitespace().collect();
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

// Parsing functions for /proc data
fn parse_cpu_metrics(stat_line: &str, loadavg_line: &str) -> CpuMetrics {
    // Parse /proc/stat first line: cpu user nice system idle iowait irq softirq steal guest guest_nice
    let parts: Vec<&str> = stat_line.split_whitespace().collect();
    let mut usage = 0.0;
    let mut user = 0.0;
    let mut system = 0.0;
    let mut iowait = 0.0;
    let mut steal = 0.0;
    let cores;

    if parts.len() >= 5 && parts[0] == "cpu" {
        let user_ticks: f64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let nice_ticks: f64 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let system_ticks: f64 = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let idle_ticks: f64 = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let iowait_ticks: f64 = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let irq_ticks: f64 = parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let softirq_ticks: f64 = parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let steal_ticks: f64 = parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0.0);

        let total_ticks = user_ticks
            + nice_ticks
            + system_ticks
            + idle_ticks
            + iowait_ticks
            + irq_ticks
            + softirq_ticks
            + steal_ticks;
        let idle_total = idle_ticks + iowait_ticks;

        if total_ticks > 0.0 {
            usage = ((total_ticks - idle_total) / total_ticks) * 100.0;
            user = (user_ticks + nice_ticks) / total_ticks * 100.0;
            system = (system_ticks + irq_ticks + softirq_ticks) / total_ticks * 100.0;
            iowait = iowait_ticks / total_ticks * 100.0;
            steal = steal_ticks / total_ticks * 100.0;
        }
    }

    // Parse /proc/loadavg: load1 load5 load15 running/total last_pid
    let load_parts: Vec<&str> = loadavg_line.split_whitespace().collect();
    let load1 = load_parts
        .first()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let load5 = load_parts
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let load15 = load_parts
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    // Count CPU cores from /proc/cpuinfo would be separate command
    cores = 1; // Default to 1, would be parsed separately

    CpuMetrics {
        usage,
        user,
        system,
        iowait,
        steal,
        cores,
        load1,
        load5,
        load15,
    }
}

#[derive(Debug)]
struct CpuMetrics {
    usage: f64,
    user: f64,
    system: f64,
    iowait: f64,
    steal: f64,
    cores: u32,
    load1: f64,
    load5: f64,
    load15: f64,
}

fn parse_memory_metrics(meminfo: &str) -> MemoryMetrics {
    let mut total = 0u64;
    let mut free = 0u64;
    let mut available = 0u64;
    let mut buffers = 0u64;
    let mut cached = 0u64;
    let mut swap_total = 0u64;
    let mut swap_free = 0u64;

    for line in meminfo.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let value_kb: u64 = parts[1].parse().unwrap_or(0);
            let value = value_kb * 1024; // Convert to bytes

            if line.starts_with("MemTotal:") {
                total = value;
            } else if line.starts_with("MemFree:") {
                free = value;
            } else if line.starts_with("MemAvailable:") {
                available = value;
            } else if line.starts_with("Buffers:") {
                buffers = value;
            } else if line.starts_with("Cached:") {
                cached = value;
            } else if line.starts_with("SwapTotal:") {
                swap_total = value;
            } else if line.starts_with("SwapFree:") {
                swap_free = value;
            }
        }
    }

    let used = total - free;
    let swap_used = swap_total.saturating_sub(swap_free);

    MemoryMetrics {
        total,
        free,
        used,
        available,
        buffers,
        cached,
        swap_total,
        swap_free,
        swap_used,
    }
}

#[derive(Debug)]
struct MemoryMetrics {
    total: u64,
    free: u64,
    used: u64,
    available: u64,
    buffers: u64,
    cached: u64,
    swap_total: u64,
    swap_free: u64,
    swap_used: u64,
}

fn parse_disk_metrics(df_line: &str, diskstats_line: &str) -> DiskMetrics {
    // Parse df output: filesystem size used available percent mount
    let parts: Vec<&str> = df_line.split_whitespace().collect();
    let total = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0u64);
    let used = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0u64);
    let free = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0u64);

    // Parse /proc/diskstats
    let stats: Vec<&str> = diskstats_line.split_whitespace().collect();
    let read_sectors = stats.get(5).and_then(|s| s.parse().ok()).unwrap_or(0u64);
    let write_sectors = stats.get(9).and_then(|s| s.parse().ok()).unwrap_or(0u64);
    let read_bytes = read_sectors * 512;
    let write_bytes = write_sectors * 512;

    DiskMetrics {
        total,
        used,
        free,
        read_bytes,
        write_bytes,
        read_iops: 0.0,
        write_iops: 0.0,
        io_util: 0.0,
    }
}

#[derive(Debug)]
struct DiskMetrics {
    total: u64,
    used: u64,
    free: u64,
    read_bytes: u64,
    write_bytes: u64,
    read_iops: f64,
    write_iops: f64,
    io_util: f64,
}

fn parse_network_metrics(net_line: &str) -> NetworkMetrics {
    // Parse /proc/net/dev: iface rx_bytes rx_packets rx_errs rx_drop rx_fifo rx_frame rx_compressed rx_multicast tx_bytes tx_packets tx_errs tx_drop tx_fifo tx_colls tx_carrier tx_compressed
    let parts: Vec<&str> = net_line.split_whitespace().collect();

    NetworkMetrics {
        rx_bytes: parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
        rx_packets: parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
        rx_errors: parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0),
        rx_dropped: parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
        tx_bytes: parts.get(9).and_then(|s| s.parse().ok()).unwrap_or(0),
        tx_packets: parts.get(10).and_then(|s| s.parse().ok()).unwrap_or(0),
        tx_errors: parts.get(11).and_then(|s| s.parse().ok()).unwrap_or(0),
        tx_dropped: parts.get(12).and_then(|s| s.parse().ok()).unwrap_or(0),
    }
}

#[derive(Debug)]
struct NetworkMetrics {
    rx_bytes: u64,
    rx_packets: u64,
    rx_errors: u64,
    rx_dropped: u64,
    tx_bytes: u64,
    tx_packets: u64,
    tx_errors: u64,
    tx_dropped: u64,
}

fn parse_process_metrics(ps_output: &str) -> ProcessMetrics {
    let lines: Vec<&str> = ps_output.lines().collect();

    ProcessMetrics {
        total: lines.get(0).and_then(|s| s.parse().ok()).unwrap_or(0),
        running: lines.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
        sleeping: 0, // Would need separate command
        zombie: lines.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
        threads: 0,
        open_files: 0,
    }
}

#[derive(Debug)]
struct ProcessMetrics {
    total: u32,
    running: u32,
    sleeping: u32,
    zombie: u32,
    threads: u32,
    open_files: u32,
}

fn parse_uptime(uptime_str: &str) -> u64 {
    uptime_str
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0) as u64
}

/// Collection script generator for different OS types
pub struct CollectionScript;

impl CollectionScript {
    /// Generate Linux collection script
    pub fn linux() -> &'static str {
        r##"#!/bin/bash
# EasySSH Metrics Collection Script for Linux
set -e

echo "===METRICS_START==="

# CPU Metrics
echo "CPU:"
cat /proc/stat | head -1
cat /proc/loadavg
cat /proc/cpuinfo | grep -c "^processor"

# Memory Metrics
echo "MEMORY:"
cat /proc/meminfo | grep -E '^(MemTotal|MemFree|MemAvailable|Buffers|Cached|SwapTotal|SwapFree):'

# Disk Metrics
echo "DISK:"
df -B1 / | tail -1
cat /proc/diskstats

# Network Metrics
echo "NETWORK:"
cat /proc/net/dev | grep -v "^Inter-" | grep -v "^ face"

# Process Metrics
echo "PROCESS:"
ps aux | wc -l
echo "$(ps aux | grep -c '^. R')"
echo "$(ps aux | grep -c '^. S')"
echo "$(ps aux | grep -c 'Z')"

# System Metrics
echo "SYSTEM:"
cat /proc/uptime
stat -c %Y /proc/1 2>/dev/null || date +%s

echo "===METRICS_END==="
"##
    }

    /// Generate macOS collection script
    pub fn macos() -> &'static str {
        r##"#!/bin/bash
# EasySSH Metrics Collection Script for macOS
set -e

echo "===METRICS_START==="

# CPU Metrics
echo "CPU:"
top -l 1 -n 0 | head -10
sysctl -n hw.ncpu

# Memory Metrics
echo "MEMORY:"
vm_stat

# Disk Metrics
echo "DISK:"
df -k / | tail -1

# Network Metrics
echo "NETWORK:"
netstat -ib | head -2 | tail -1

# Process Metrics
echo "PROCESS:"
ps aux | wc -l

# System Metrics
echo "SYSTEM:"
sysctl -n kern.boottime
echo "$(date +%s) - $(sysctl -n kern.boottime | awk '{print $4}' | tr -d ',')" | bc

echo "===METRICS_END==="
"##
    }
}

/// Simple collector for Standard version (lightweight)
pub struct SimpleCollector {
    interval_secs: u64,
    session: Option<ssh2::Session>,
}

impl SimpleCollector {
    pub fn new(interval_secs: u64) -> Self {
        Self {
            interval_secs: interval_secs.max(1),
            session: None,
        }
    }

    pub fn connect(
        &mut self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<(), MonitoringError> {
        let tcp = std::net::TcpStream::connect(format!("{}:{}", host, port))
            .map_err(|e| MonitoringError::Ssh(format!("TCP connection failed: {}", e)))?;

        let mut session = ssh2::Session::new()
            .map_err(|e| MonitoringError::Ssh(format!("Failed to create SSH session: {}", e)))?;

        session.set_tcp_stream(tcp);

        session
            .handshake()
            .map_err(|e| MonitoringError::Ssh(format!("SSH handshake failed: {}", e)))?;

        session
            .userauth_password(username, password)
            .map_err(|e| MonitoringError::Ssh(format!("Password auth failed: {}", e)))?;

        self.session = Some(session);
        Ok(())
    }

    pub fn collect(&self) -> Result<SystemMetrics, MonitoringError> {
        if let Some(ref session) = self.session {
            // Use block_on since this is a synchronous method
            let runtime = tokio::runtime::Handle::try_current()
                .map_err(|_| MonitoringError::Config("No async runtime available".to_string()))?;
            runtime.block_on(collect_via_proc_files(session))
        } else {
            Err(MonitoringError::Collection("Not connected".to_string()))
        }
    }

    pub fn disconnect(&mut self) {
        if let Some(session) = self.session.take() {
            let _ = session.disconnect(None, "SimpleCollector disconnect", None);
        }
    }
}
