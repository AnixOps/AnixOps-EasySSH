//! Metrics storage with time-series data retention

use std::collections::HashMap;
use std::sync::Arc;

use crate::monitoring::alerts::{Alert, AlertRule};
use crate::monitoring::metrics::{
    CapacityForecast, MetricPoint, MetricQuery, MetricQueryResult, MetricSeries, MetricSummary,
    MetricType, PerformanceComparison, ResourceType, ServerMetrics, SlaStats,
};
use crate::monitoring::{
    CustomDashboard, MonitoringConfig, MonitoringError, ServerOverview, TimeRange,
};

/// Time-series metrics storage using SQLite with rollups
pub struct MetricsStorage {
    conn: Arc<tokio::sync::Mutex<rusqlite::Connection>>,
    config: StorageConfig,
}

#[derive(Debug, Clone)]
struct StorageConfig {
    retention_days: u32,
}

impl MetricsStorage {
    pub async fn new(config: &MonitoringConfig) -> Result<Self, MonitoringError> {
        let db_path = Self::metrics_db_path()?;
        let conn = rusqlite::Connection::open(db_path)?;

        // Initialize schema
        Self::init_schema(&conn)?;

        Ok(Self {
            conn: Arc::new(tokio::sync::Mutex::new(conn)),
            config: StorageConfig {
                retention_days: config.retention_days,
            },
        })
    }

    fn metrics_db_path() -> Result<std::path::PathBuf, MonitoringError> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| MonitoringError::Storage("Could not find data directory".to_string()))?;
        let easyssh_dir = data_dir.join("easyssh");
        std::fs::create_dir_all(&easyssh_dir)?;
        Ok(easyssh_dir.join("monitoring.db"))
    }

    fn init_schema(conn: &rusqlite::Connection) -> Result<(), MonitoringError> {
        conn.execute_batch(
            r#"
            -- Metrics raw data (high resolution, auto-expire)
            CREATE TABLE IF NOT EXISTS metrics_raw (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                server_id TEXT NOT NULL,
                metric_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                value REAL NOT NULL,
                labels TEXT, -- JSON
                INDEX idx_metrics_server_time (server_id, timestamp),
                INDEX idx_metrics_type_time (metric_type, timestamp)
            );

            -- Hourly rollups
            CREATE TABLE IF NOT EXISTS metrics_hourly (
                server_id TEXT NOT NULL,
                metric_type TEXT NOT NULL,
                hour_timestamp INTEGER NOT NULL,
                avg_value REAL NOT NULL,
                min_value REAL NOT NULL,
                max_value REAL NOT NULL,
                count INTEGER NOT NULL,
                PRIMARY KEY (server_id, metric_type, hour_timestamp)
            );

            -- Daily rollups
            CREATE TABLE IF NOT EXISTS metrics_daily (
                server_id TEXT NOT NULL,
                metric_type TEXT NOT NULL,
                day_timestamp INTEGER NOT NULL,
                avg_value REAL NOT NULL,
                min_value REAL NOT NULL,
                max_value REAL NOT NULL,
                p95_value REAL,
                p99_value REAL,
                count INTEGER NOT NULL,
                PRIMARY KEY (server_id, metric_type, day_timestamp)
            );

            -- Server health snapshots
            CREATE TABLE IF NOT EXISTS server_health (
                server_id TEXT PRIMARY KEY,
                last_seen INTEGER,
                uptime_seconds INTEGER,
                cpu_percent REAL,
                memory_percent REAL,
                disk_percent REAL,
                network_rx_mbps REAL,
                network_tx_mbps REAL,
                active_alerts INTEGER DEFAULT 0,
                status TEXT,
                os_info TEXT,
                location TEXT,
                updated_at INTEGER NOT NULL
            );

            -- Alert rules
            CREATE TABLE IF NOT EXISTS alert_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                severity TEXT NOT NULL,
                metric_type TEXT NOT NULL,
                condition TEXT NOT NULL,
                threshold REAL NOT NULL,
                duration_secs INTEGER NOT NULL,
                cooldown_secs INTEGER NOT NULL,
                server_ids TEXT, -- JSON array
                server_groups TEXT, -- JSON array
                tags TEXT, -- JSON array
                notification_channels TEXT, -- JSON array
                auto_resolve INTEGER NOT NULL DEFAULT 0,
                resolve_after_secs INTEGER NOT NULL DEFAULT 0,
                runbook_url TEXT,
                dashboard_url TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            -- Alert instances
            CREATE TABLE IF NOT EXISTS alerts (
                id TEXT PRIMARY KEY,
                rule_id TEXT NOT NULL,
                rule_name TEXT NOT NULL,
                server_id TEXT NOT NULL,
                server_name TEXT,
                severity TEXT NOT NULL,
                status TEXT NOT NULL,
                title TEXT NOT NULL,
                message TEXT NOT NULL,
                metric_type TEXT NOT NULL,
                metric_value REAL NOT NULL,
                threshold REAL NOT NULL,
                started_at INTEGER NOT NULL,
                acknowledged_at INTEGER,
                acknowledged_by TEXT,
                resolved_at INTEGER,
                flapping_count INTEGER DEFAULT 0,
                tags TEXT, -- JSON array
                runbook_url TEXT,
                dashboard_url TEXT,
                INDEX idx_alerts_server (server_id),
                INDEX idx_alerts_status (status),
                INDEX idx_alerts_time (started_at)
            );

            -- Custom dashboards
            CREATE TABLE IF NOT EXISTS dashboards (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                widgets TEXT NOT NULL, -- JSON
                is_default INTEGER NOT NULL DEFAULT 0,
                is_large_screen INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            -- SLA tracking
            CREATE TABLE IF NOT EXISTS sla_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                server_id TEXT NOT NULL,
                date TEXT NOT NULL, -- YYYY-MM-DD
                uptime_seconds INTEGER NOT NULL,
                downtime_seconds INTEGER NOT NULL,
                incidents INTEGER NOT NULL DEFAULT 0,
                availability_percent REAL NOT NULL,
                UNIQUE (server_id, date)
            );

            -- Maintenance windows
            CREATE TABLE IF NOT EXISTS maintenance_windows (
                id TEXT PRIMARY KEY,
                server_id TEXT NOT NULL,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                description TEXT,
                created_by TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );

            -- Retention policy
            CREATE TABLE IF NOT EXISTS retention_policy (
                id INTEGER PRIMARY KEY,
                raw_retention_days INTEGER NOT NULL DEFAULT 7,
                hourly_retention_days INTEGER NOT NULL DEFAULT 30,
                daily_retention_days INTEGER NOT NULL DEFAULT 365,
                last_cleanup INTEGER
            );

            INSERT OR IGNORE INTO retention_policy (id) VALUES (1);
            "#,
        )?;

        Ok(())
    }

    /// Store server metrics
    pub async fn store_metrics(&self, metrics: &ServerMetrics) -> Result<(), MonitoringError> {
        let conn = self.conn.lock().await;

        // Store individual metrics
        let mut stmt = conn.prepare(
            "INSERT INTO metrics_raw (server_id, metric_type, timestamp, value, labels) VALUES (?1, ?2, ?3, ?4, ?5)"
        )?;

        let metrics_data: Vec<(MetricType, f64)> = vec![
            (MetricType::CpuUsage, metrics.cpu_usage),
            (MetricType::CpuUser, metrics.cpu_user),
            (MetricType::CpuSystem, metrics.cpu_system),
            (MetricType::CpuIowait, metrics.cpu_iowait),
            (MetricType::CpuSteal, metrics.cpu_steal),
            (MetricType::CpuLoad1, metrics.cpu_load1),
            (MetricType::CpuLoad5, metrics.cpu_load5),
            (MetricType::CpuLoad15, metrics.cpu_load15),
            (MetricType::MemoryUsed, metrics.memory_used as f64),
            (MetricType::MemoryTotal, metrics.memory_total as f64),
            (MetricType::MemoryFree, metrics.memory_free as f64),
            (MetricType::MemoryUsage, metrics.memory_usage_percent()),
            (MetricType::DiskUsed, metrics.disk_used as f64),
            (MetricType::DiskTotal, metrics.disk_total as f64),
            (MetricType::DiskUsage, metrics.disk_usage_percent()),
            (MetricType::NetworkRxBytes, metrics.network_rx_bytes as f64),
            (MetricType::NetworkTxBytes, metrics.network_tx_bytes as f64),
            (MetricType::ProcessCount, metrics.process_count as f64),
            (MetricType::ProcessRunning, metrics.process_running as f64),
            (MetricType::ProcessZombie, metrics.process_zombie as f64),
        ];

        for (metric_type, value) in metrics_data {
            stmt.execute(rusqlite::params![
                &metrics.server_id,
                format!("{:?}", metric_type),
                metrics.timestamp as i64,
                value,
                None::<&str>,
            ])?;
        }

        // Update server health snapshot
        conn.execute(
            r#"INSERT INTO server_health (server_id, last_seen, uptime_seconds, cpu_percent,
                memory_percent, disk_percent, network_rx_mbps, network_tx_mbps,
                status, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(server_id) DO UPDATE SET
                last_seen = excluded.last_seen,
                uptime_seconds = excluded.uptime_seconds,
                cpu_percent = excluded.cpu_percent,
                memory_percent = excluded.memory_percent,
                disk_percent = excluded.disk_percent,
                network_rx_mbps = excluded.network_rx_mbps,
                network_tx_mbps = excluded.network_tx_mbps,
                status = excluded.status,
                updated_at = excluded.updated_at"#,
            rusqlite::params![
                &metrics.server_id,
                metrics.timestamp as i64,
                metrics.uptime_seconds as i64,
                metrics.cpu_usage,
                metrics.memory_usage_percent(),
                metrics.disk_usage_percent(),
                metrics.network_rx_mbps(30), // Assume 30s interval
                metrics.network_tx_mbps(30),
                format!("{:?}", metrics.health_status()),
                metrics.timestamp as i64,
            ],
        )?;

        Ok(())
    }

    /// Get latest metrics for a server
    pub async fn get_latest_metrics(
        &self,
        server_id: &str,
    ) -> Result<ServerMetrics, MonitoringError> {
        let conn = self.conn.lock().await;

        // Get from server health snapshot and raw metrics
        let health: Option<(i64, i64, f64, f64, f64, f64, f64)> = conn
            .query_row(
                "SELECT last_seen, uptime_seconds, cpu_percent, memory_percent, disk_percent,
                    network_rx_mbps, network_tx_mbps
             FROM server_health WHERE server_id = ?1",
                [server_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .ok();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let (last_seen, uptime, cpu, _memory, _disk, _net_rx, _net_tx) =
            health.unwrap_or((0, 0, 0.0, 0.0, 0.0, 0.0, 0.0));

        // Construct ServerMetrics from snapshot data
        // In production, we'd query actual raw metrics
        Ok(ServerMetrics {
            server_id: server_id.to_string(),
            timestamp,
            collected_at: last_seen as u64,
            cpu_usage: cpu,
            cpu_user: 0.0,
            cpu_system: 0.0,
            cpu_iowait: 0.0,
            cpu_steal: 0.0,
            cpu_cores: 1,
            cpu_load1: 0.0,
            cpu_load5: 0.0,
            cpu_load15: 0.0,
            memory_used: 0,
            memory_total: 0,
            memory_free: 0,
            memory_buffers: 0,
            memory_cached: 0,
            memory_available: 0,
            swap_used: 0,
            swap_total: 0,
            disk_used: 0,
            disk_total: 0,
            disk_free: 0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            disk_read_iops: 0.0,
            disk_write_iops: 0.0,
            disk_io_util: 0.0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            network_rx_packets: 0,
            network_tx_packets: 0,
            network_rx_errors: 0,
            network_tx_errors: 0,
            network_rx_dropped: 0,
            network_tx_dropped: 0,
            process_count: 0,
            process_running: 0,
            process_sleeping: 0,
            process_zombie: 0,
            thread_count: 0,
            open_files: 0,
            uptime_seconds: uptime as u64,
            boot_time: 0,
            context_switches: 0,
            interrupts: 0,
            cpu_temp: None,
            system_temp: None,
            extra: HashMap::new(),
        })
    }

    /// Get metrics history
    pub async fn get_metrics_history(
        &self,
        server_id: &str,
        metric_type: MetricType,
        time_range: TimeRange,
    ) -> Result<Vec<MetricPoint>, MonitoringError> {
        let conn = self.conn.lock().await;

        let start_time = time_range.get_start_timestamp() as i64;
        let end_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut stmt = conn.prepare(
            "SELECT timestamp, value FROM metrics_raw
             WHERE server_id = ?1 AND metric_type = ?2 AND timestamp >= ?3 AND timestamp <= ?4
             ORDER BY timestamp",
        )?;

        let rows = stmt.query_map(
            rusqlite::params![
                server_id,
                format!("{:?}", metric_type),
                start_time,
                end_time,
            ],
            |row| {
                Ok(MetricPoint {
                    timestamp: row.get(0)?,
                    value: row.get(1)?,
                    metric_type: metric_type.clone(),
                    labels: HashMap::new(),
                })
            },
        )?;

        let mut points = Vec::new();
        for row in rows {
            points.push(row?);
        }

        Ok(points)
    }

    /// Get health summary for all servers
    pub async fn get_health_summary(&self) -> Result<super::HealthSummary, MonitoringError> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT server_id, last_seen, uptime_seconds, cpu_percent, memory_percent,
                    disk_percent, network_rx_mbps, network_tx_mbps, status, os_info, location
             FROM server_health",
        )?;

        let rows = stmt.query_map([], |row| {
            let status_str: String = row.get(8)?;
            let status = match status_str.as_str() {
                "Healthy" => super::ServerHealthStatus::Healthy,
                "Warning" => super::ServerHealthStatus::Warning,
                "Critical" => super::ServerHealthStatus::Critical,
                _ => super::ServerHealthStatus::Unknown,
            };

            Ok(ServerOverview {
                server_id: row.get(0)?,
                server_name: row.get(0)?, // Would fetch actual name from servers table
                host: "".to_string(),
                status,
                last_seen: row.get(1)?,
                uptime_seconds: row.get(2)?,
                cpu_percent: row.get(3)?,
                memory_percent: row.get(4)?,
                disk_percent: row.get(5)?,
                network_rx_mbps: row.get(6)?,
                network_tx_mbps: row.get(7)?,
                active_alerts: 0, // Would query alerts table
                os_info: row.get(9)?,
                location: row.get(10)?,
            })
        })?;

        let mut servers = Vec::new();
        let mut healthy = 0;
        let mut warning = 0;
        let mut critical = 0;
        let mut offline = 0;
        let mut unknown = 0;

        for row in rows {
            let server = row?;
            match server.status {
                super::ServerHealthStatus::Healthy => healthy += 1,
                super::ServerHealthStatus::Warning => warning += 1,
                super::ServerHealthStatus::Critical => critical += 1,
                super::ServerHealthStatus::Offline => offline += 1,
                super::ServerHealthStatus::Unknown => unknown += 1,
            }
            servers.push(server);
        }

        let total_alerts = conn
            .query_row(
                "SELECT COUNT(*) FROM alerts WHERE status = 'active'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;

        Ok(super::HealthSummary {
            total_servers: servers.len(),
            healthy,
            warning,
            critical,
            offline,
            unknown,
            total_alerts,
            servers,
        })
    }

    /// Get monitored servers list
    pub async fn get_monitored_servers(&self) -> Result<Vec<String>, MonitoringError> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare("SELECT DISTINCT server_id FROM server_health")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

        let mut servers = Vec::new();
        for row in rows {
            servers.push(row?);
        }

        Ok(servers)
    }

    /// Save alert
    pub async fn save_alert(&self, alert: &Alert) -> Result<(), MonitoringError> {
        let conn = self.conn.lock().await;

        conn.execute(
            r#"INSERT INTO alerts (id, rule_id, rule_name, server_id, server_name, severity,
                status, title, message, metric_type, metric_value, threshold, started_at,
                tags, runbook_url, dashboard_url)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                acknowledged_at = excluded.acknowledged_at,
                acknowledged_by = excluded.acknowledged_by,
                resolved_at = excluded.resolved_at,
                flapping_count = excluded.flapping_count"#,
            rusqlite::params![
                &alert.id,
                &alert.rule_id,
                &alert.rule_name,
                &alert.server_id,
                &alert.server_name,
                format!("{:?}", alert.severity),
                format!("{:?}", alert.status),
                &alert.title,
                &alert.message,
                format!("{:?}", alert.metric_type),
                alert.metric_value,
                alert.threshold,
                alert.started_at as i64,
                serde_json::to_string(&alert.tags)?,
                alert.runbook_url.as_ref(),
                alert.dashboard_url.as_ref(),
            ],
        )?;

        Ok(())
    }

    /// Update alert
    pub async fn update_alert(&self, alert: &Alert) -> Result<(), MonitoringError> {
        self.save_alert(alert).await
    }

    /// Get alert history
    pub async fn get_alert_history(
        &self,
        server_id: Option<&str>,
        severity: Option<super::alerts::AlertSeverity>,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<Alert>, MonitoringError> {
        let conn = self.conn.lock().await;

        let mut query = String::from(
            "SELECT id, rule_id, rule_name, server_id, server_name, severity, status,
                    title, message, metric_type, metric_value, threshold, started_at,
                    acknowledged_at, acknowledged_by, resolved_at, tags, runbook_url, dashboard_url
             FROM alerts WHERE started_at >= ?1 AND started_at <= ?2",
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(start_time as i64), Box::new(end_time as i64)];

        if server_id.is_some() {
            query.push_str(" AND server_id = ?3");
            params.push(Box::new(server_id.unwrap().to_string()));
        }

        if severity.is_some() {
            query.push_str(&format!(" AND severity = ?{}", params.len() + 1));
            params.push(Box::new(format!("{:?}", severity.unwrap())));
        }

        query.push_str(" ORDER BY started_at DESC");

        let mut stmt = conn.prepare(&query)?;

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let severity_str: String = row.get(5)?;
            let status_str: String = row.get(6)?;
            let metric_type_str: String = row.get(9)?;
            let tags_json: String = row.get(16)?;

            Ok(Alert {
                id: row.get(0)?,
                rule_id: row.get(1)?,
                rule_name: row.get(2)?,
                server_id: row.get(3)?,
                server_name: row.get(4)?,
                severity: parse_severity(&severity_str),
                status: parse_status(&status_str),
                title: row.get(7)?,
                message: row.get(8)?,
                metric_type: parse_metric_type(&metric_type_str),
                metric_value: row.get(10)?,
                threshold: row.get(11)?,
                started_at: row.get::<_, i64>(12)? as u64,
                acknowledged_at: row.get::<_, Option<i64>>(13)?.map(|v| v as u64),
                acknowledged_by: row.get(14)?,
                resolved_at: row.get::<_, Option<i64>>(15)?.map(|v| v as u64),
                flapping_count: 0,
                tags: serde_json::from_str(&tags_json).unwrap_or_default(),
                runbook_url: row.get(17)?,
                dashboard_url: row.get(18)?,
            })
        })?;

        let mut alerts = Vec::new();
        for row in rows {
            alerts.push(row?);
        }

        Ok(alerts)
    }

    /// Save alert rule
    pub async fn save_alert_rule(&self, rule: &AlertRule) -> Result<(), MonitoringError> {
        let conn = self.conn.lock().await;

        conn.execute(
            r#"INSERT INTO alert_rules (id, name, description, enabled, severity, metric_type,
                condition, threshold, duration_secs, cooldown_secs, server_ids, server_groups,
                tags, notification_channels, auto_resolve, resolve_after_secs, runbook_url,
                dashboard_url, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                enabled = excluded.enabled,
                severity = excluded.severity,
                metric_type = excluded.metric_type,
                condition = excluded.condition,
                threshold = excluded.threshold,
                duration_secs = excluded.duration_secs,
                cooldown_secs = excluded.cooldown_secs,
                server_ids = excluded.server_ids,
                server_groups = excluded.server_groups,
                tags = excluded.tags,
                notification_channels = excluded.notification_channels,
                auto_resolve = excluded.auto_resolve,
                resolve_after_secs = excluded.resolve_after_secs,
                runbook_url = excluded.runbook_url,
                dashboard_url = excluded.dashboard_url,
                updated_at = excluded.updated_at"#,
            rusqlite::params![
                &rule.id,
                &rule.name,
                rule.description.as_ref(),
                rule.enabled as i32,
                format!("{:?}", rule.severity),
                format!("{:?}", rule.metric_type),
                format!("{:?}", rule.condition),
                rule.threshold,
                rule.duration_secs as i64,
                rule.cooldown_secs as i64,
                serde_json::to_string(&rule.server_ids)?,
                serde_json::to_string(&rule.server_groups)?,
                serde_json::to_string(&rule.tags)?,
                serde_json::to_string(&rule.notification_channels)?,
                rule.auto_resolve as i32,
                rule.resolve_after_secs as i64,
                rule.runbook_url.as_ref(),
                rule.dashboard_url.as_ref(),
                rule.created_at as i64,
                rule.updated_at as i64,
            ],
        )?;

        Ok(())
    }

    /// Delete alert rule
    pub async fn delete_alert_rule(&self, rule_id: &str) -> Result<(), MonitoringError> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM alert_rules WHERE id = ?1", [rule_id])?;
        Ok(())
    }

    /// Load alert rules
    pub async fn load_alert_rules(&self) -> Result<Vec<AlertRule>, MonitoringError> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT id, name, description, enabled, severity, metric_type, condition, threshold,
                    duration_secs, cooldown_secs, server_ids, server_groups, tags,
                    notification_channels, auto_resolve, resolve_after_secs, runbook_url,
                    dashboard_url, created_at, updated_at
             FROM alert_rules",
        )?;

        let rows = stmt.query_map([], |row| {
            let severity_str: String = row.get(4)?;
            let condition_str: String = row.get(6)?;
            let metric_type_str: String = row.get(5)?;

            Ok(AlertRule {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                severity: parse_severity(&severity_str),
                metric_type: parse_metric_type(&metric_type_str),
                condition: parse_condition(&condition_str),
                threshold: row.get(7)?,
                duration_secs: row.get::<_, i64>(8)? as u64,
                cooldown_secs: row.get::<_, i64>(9)? as u64,
                server_ids: serde_json::from_str(row.get::<_, String>(10)?.as_str())
                    .unwrap_or_default(),
                server_groups: serde_json::from_str(row.get::<_, String>(11)?.as_str())
                    .unwrap_or_default(),
                tags: serde_json::from_str(row.get::<_, String>(12)?.as_str()).unwrap_or_default(),
                notification_channels: serde_json::from_str(row.get::<_, String>(13)?.as_str())
                    .unwrap_or_default(),
                auto_resolve: row.get::<_, i32>(14)? != 0,
                resolve_after_secs: row.get::<_, i64>(15)? as u64,
                runbook_url: row.get(16)?,
                dashboard_url: row.get(17)?,
                created_at: row.get::<_, i64>(18)? as u64,
                updated_at: row.get::<_, i64>(19)? as u64,
            })
        })?;

        let mut rules = Vec::new();
        for row in rows {
            rules.push(row?);
        }

        Ok(rules)
    }

    /// Save dashboard
    pub async fn save_dashboard(&self, dashboard: &CustomDashboard) -> Result<(), MonitoringError> {
        let conn = self.conn.lock().await;

        conn.execute(
            r#"INSERT INTO dashboards (id, name, description, widgets, is_default, is_large_screen, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                widgets = excluded.widgets,
                is_default = excluded.is_default,
                is_large_screen = excluded.is_large_screen,
                updated_at = excluded.updated_at"#,
            rusqlite::params![
                &dashboard.id,
                &dashboard.name,
                dashboard.description.as_ref(),
                serde_json::to_string(&dashboard.widgets)?,
                dashboard.is_default as i32,
                dashboard.is_large_screen as i32,
                dashboard.created_at as i64,
                dashboard.updated_at as i64,
            ],
        )?;

        Ok(())
    }

    /// Delete dashboard
    pub async fn delete_dashboard(&self, dashboard_id: &str) -> Result<(), MonitoringError> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM dashboards WHERE id = ?1", [dashboard_id])?;
        Ok(())
    }

    /// Compare performance across servers
    pub async fn compare_performance(
        &self,
        server_ids: Vec<String>,
        metric_type: MetricType,
        time_range: TimeRange,
    ) -> Result<PerformanceComparison, MonitoringError> {
        let conn = self.conn.lock().await;

        let start_time = time_range.get_start_timestamp() as i64;
        let end_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let metric_type_str = format!("{:?}", metric_type);

        let mut servers = Vec::new();
        let mut all_values = Vec::new();

        for server_id in server_ids {
            let mut stmt = conn.prepare(
                "SELECT value FROM metrics_raw
                 WHERE server_id = ?1 AND metric_type = ?2 AND timestamp >= ?3 AND timestamp <= ?4
                 ORDER BY timestamp",
            )?;

            let rows = stmt.query_map(
                rusqlite::params![&server_id, &metric_type_str, start_time, end_time],
                |row| row.get::<_, f64>(0),
            )?;

            let values: Vec<f64> = rows.filter_map(|r| r.ok()).collect();

            if !values.is_empty() {
                let current = *values.last().unwrap_or(&0.0);
                let avg = values.iter().sum::<f64>() / values.len() as f64;
                let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

                servers.push(super::metrics::ServerPerformanceData {
                    server_id: server_id.clone(),
                    server_name: server_id, // Would fetch actual name
                    current,
                    average: avg,
                    min,
                    max,
                    p95: 0.0, // Would calculate
                    p99: 0.0, // Would calculate
                    trend: crate::TrendDirection::Stable,
                    sparkline: values
                        .iter()
                        .step_by(values.len() / 20 + 1)
                        .copied()
                        .collect(),
                });

                all_values.extend(values);
            }
        }

        let avg = if all_values.is_empty() {
            0.0
        } else {
            all_values.iter().sum::<f64>() / all_values.len() as f64
        };

        let median = if all_values.is_empty() {
            0.0
        } else {
            let mut sorted = all_values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            sorted[sorted.len() / 2]
        };

        let std_dev = if all_values.len() > 1 {
            let mean = avg;
            let variance: f64 = all_values.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                / (all_values.len() - 1) as f64;
            variance.sqrt()
        } else {
            0.0
        };

        Ok(PerformanceComparison {
            metric_type,
            time_range,
            servers,
            winner: None,
            average: avg,
            median,
            std_dev,
        })
    }

    /// Predict capacity depletion
    pub async fn predict_capacity(
        &self,
        server_id: &str,
        resource_type: ResourceType,
        days_ahead: u32,
    ) -> Result<CapacityForecast, MonitoringError> {
        // Get historical data for the past 30 days
        let time_range = TimeRange::Last30Days;
        let metric_type = match resource_type {
            ResourceType::Cpu => MetricType::CpuUsage,
            ResourceType::Memory => MetricType::MemoryUsage,
            ResourceType::Disk => MetricType::DiskUsage,
            ResourceType::Network => MetricType::NetworkRxMbps,
        };

        let history = self
            .get_metrics_history(server_id, metric_type, time_range)
            .await?;

        if history.len() < 2 {
            return Ok(CapacityForecast {
                server_id: server_id.to_string(),
                resource_type,
                current_usage: 0.0,
                current_capacity: 100.0,
                predicted_depletion_date: None,
                days_until_critical: None,
                growth_rate_per_day: 0.0,
                confidence: 0.0,
                forecast_points: Vec::new(),
                recommendations: Vec::new(),
            });
        }

        // Simple linear regression for prediction
        let n = history.len() as f64;
        let sum_x: f64 = (0..history.len()).map(|i| i as f64).sum();
        let sum_y: f64 = history.iter().map(|p| p.value).sum();
        let sum_xy: f64 = history
            .iter()
            .enumerate()
            .map(|(i, p)| i as f64 * p.value)
            .sum();
        let sum_x2: f64 = (0..history.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
        let intercept = (sum_y - slope * sum_x) / n;

        let current = history.last().map(|p| p.value).unwrap_or(0.0);
        let growth_per_day = slope * (history.len() as f64 / 30.0); // Scale to daily

        // Calculate when we'll hit critical thresholds
        let days_to_critical = if slope > 0.0 {
            let critical_threshold = match resource_type {
                ResourceType::Cpu => 90.0,
                ResourceType::Memory => 90.0,
                ResourceType::Disk => 95.0,
                ResourceType::Network => 1000.0, // Mbps threshold
            };
            Some(((critical_threshold - intercept) / slope - n) / (n / 30.0))
        } else {
            None
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let depletion_date = days_to_critical
            .filter(|&d| d > 0.0 && d < days_ahead as f64)
            .map(|d| now + (d * 86400.0) as u64);

        // Generate forecast points
        let mut forecast_points = Vec::new();
        for day in 0..days_ahead {
            let predicted = intercept + slope * (history.len() as f64 + day as f64 * (n / 30.0));
            let timestamp = now + (day as u64 * 86400);
            let std_error = (sum_y - slope * sum_x - intercept * n).abs().sqrt() / n;

            forecast_points.push(super::metrics::ForecastPoint {
                timestamp,
                predicted_usage: predicted.max(0.0).min(100.0),
                lower_bound: (predicted - 2.0 * std_error).max(0.0),
                upper_bound: (predicted + 2.0 * std_error).min(100.0),
            });
        }

        let mut recommendations = Vec::new();
        if let Some(days) = days_to_critical {
            if days < 7.0 {
                recommendations.push(super::metrics::CapacityRecommendation {
                    priority: super::metrics::Priority::Critical,
                    message: format!(
                        "{} will be critically full within {} days",
                        format!("{:?}", resource_type),
                        days as u32
                    ),
                    action: "Scale up immediately or add more capacity".to_string(),
                    estimated_cost: None,
                });
            } else if days < 30.0 {
                recommendations.push(super::metrics::CapacityRecommendation {
                    priority: super::metrics::Priority::High,
                    message: format!(
                        "{} approaching capacity in {} days",
                        format!("{:?}", resource_type),
                        days as u32
                    ),
                    action: "Plan capacity expansion within this month".to_string(),
                    estimated_cost: None,
                });
            }
        }

        Ok(CapacityForecast {
            server_id: server_id.to_string(),
            resource_type,
            current_usage: current,
            current_capacity: 100.0,
            predicted_depletion_date: depletion_date,
            days_until_critical: days_to_critical.map(|d| d as u32),
            growth_rate_per_day: growth_per_day,
            confidence: 0.85, // Simplified confidence calculation
            forecast_points,
            recommendations,
        })
    }

    /// Calculate SLA statistics
    pub async fn calculate_sla(
        &self,
        server_id: &str,
        time_range: TimeRange,
    ) -> Result<SlaStats, MonitoringError> {
        let conn = self.conn.lock().await;

        let start_time = time_range.get_start_timestamp();
        let end_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let total_duration = end_time - start_time;

        // Get downtime from alerts
        let mut stmt = conn.prepare(
            "SELECT started_at, resolved_at, severity
             FROM alerts
             WHERE server_id = ?1 AND severity IN ('critical', 'emergency')
               AND started_at >= ?2 AND started_at <= ?3",
        )?;

        let rows = stmt.query_map(
            rusqlite::params![server_id, start_time as i64, end_time as i64],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? as u64,
                    row.get::<_, Option<i64>>(1)?.map(|v| v as u64),
                    row.get::<_, String>(2)?,
                ))
            },
        )?;

        let mut downtime_seconds = 0u64;
        let mut incidents = Vec::new();

        for row in rows {
            let (started, resolved, severity_str) = row?;
            let ended = resolved.unwrap_or(end_time);
            let duration = ended - started;
            downtime_seconds += duration;

            incidents.push(super::metrics::Incident {
                start_time: started,
                end_time: resolved,
                duration_minutes: duration / 60,
                severity: parse_severity(&severity_str),
                description: format!("Critical incident"),
            });
        }

        let uptime_seconds = total_duration - downtime_seconds;
        let uptime_percent = if total_duration > 0 {
            (uptime_seconds as f64 / total_duration as f64) * 100.0
        } else {
            100.0
        };

        // Calculate MTTR
        let resolved_incidents: Vec<_> =
            incidents.iter().filter(|i| i.end_time.is_some()).collect();
        let mttr = if !resolved_incidents.is_empty() {
            resolved_incidents
                .iter()
                .map(|i| i.duration_minutes as f64)
                .sum::<f64>()
                / resolved_incidents.len() as f64
        } else {
            0.0
        };

        // Calculate MTBF
        let mtbf = if incidents.len() > 1 {
            uptime_seconds as f64 / 60.0 / incidents.len() as f64
        } else {
            0.0
        };

        Ok(SlaStats {
            server_id: server_id.to_string(),
            time_range,
            uptime_percent,
            downtime_minutes: downtime_seconds / 60,
            incidents,
            availability_target: 99.9,
            meets_sla: uptime_percent >= 99.9,
            mttr_minutes: mttr,
            mtbf_minutes: mtbf,
            monthly_availability: Vec::new(), // Would query monthly aggregation
        })
    }

    /// Execute flexible metric query
    pub async fn query_metrics(
        &self,
        query: MetricQuery,
    ) -> Result<MetricQueryResult, MonitoringError> {
        let conn = self.conn.lock().await;

        let metric_type_strs: Vec<String> = query
            .metric_types
            .iter()
            .map(|m| format!("{:?}", m))
            .collect();

        let placeholders = metric_type_strs
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT server_id, metric_type, timestamp, value FROM metrics_raw
             WHERE server_id IN ({})
               AND metric_type IN ({})
               AND timestamp >= ? AND timestamp <= ?
             ORDER BY server_id, metric_type, timestamp",
            query
                .server_ids
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(","),
            placeholders
        );

        let mut stmt = conn.prepare(&sql)?;

        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for sid in &query.server_ids {
            params.push(sid);
        }
        for mt in &metric_type_strs {
            params.push(mt);
        }
        params.push(&query.start_time as &dyn rusqlite::ToSql);
        params.push(&query.end_time as &dyn rusqlite::ToSql);

        let rows = stmt.query_map(params.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u64>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })?;

        let mut series_map: HashMap<(String, String), Vec<MetricPoint>> = HashMap::new();
        let mut all_values = Vec::new();

        for row in rows {
            let (server_id, metric_type_str, timestamp, value) = row?;
            let point = MetricPoint {
                timestamp,
                value,
                metric_type: parse_metric_type(&metric_type_str),
                labels: HashMap::new(),
            };

            series_map
                .entry((server_id.clone(), metric_type_str.clone()))
                .or_default()
                .push(point);
            all_values.push(value);
        }

        let series: Vec<MetricSeries> = series_map
            .into_iter()
            .map(|((server_id, metric_type_str), points)| MetricSeries {
                server_id,
                metric_type: parse_metric_type(&metric_type_str),
                points,
            })
            .collect();

        let summary = if all_values.is_empty() {
            MetricSummary {
                total_points: 0,
                min_value: 0.0,
                max_value: 0.0,
                avg_value: 0.0,
            }
        } else {
            MetricSummary {
                total_points: all_values.len(),
                min_value: all_values.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
                max_value: all_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
                avg_value: all_values.iter().sum::<f64>() / all_values.len() as f64,
            }
        };

        Ok(MetricQueryResult {
            query,
            series,
            summary,
        })
    }

    /// Run data retention cleanup
    pub async fn cleanup_old_data(&self) -> Result<u64, MonitoringError> {
        let conn = self.conn.lock().await;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Get retention policy
        let (raw_retention, hourly_retention, daily_retention): (i64, i64, i64) = conn.query_row(
            "SELECT raw_retention_days, hourly_retention_days, daily_retention_days FROM retention_policy WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;

        // Delete old raw data
        let raw_cutoff = now - (raw_retention * 86400);
        let raw_deleted =
            conn.execute("DELETE FROM metrics_raw WHERE timestamp < ?1", [raw_cutoff])?;

        // Delete old hourly rollups
        let hourly_cutoff = now - (hourly_retention * 86400);
        let hourly_deleted = conn.execute(
            "DELETE FROM metrics_hourly WHERE hour_timestamp < ?1",
            [hourly_cutoff],
        )?;

        // Delete old daily rollups
        let daily_cutoff = now - (daily_retention * 86400);
        let daily_deleted = conn.execute(
            "DELETE FROM metrics_daily WHERE day_timestamp < ?1",
            [daily_cutoff],
        )?;

        // Update last cleanup time
        conn.execute(
            "UPDATE retention_policy SET last_cleanup = ?1 WHERE id = 1",
            [now],
        )?;

        let total_deleted = raw_deleted + hourly_deleted + daily_deleted;
        log::info!("Cleaned up {} old metric records", total_deleted);

        Ok(total_deleted.try_into().unwrap_or(0u64))
    }
}

// Helper functions
fn parse_severity(s: &str) -> super::alerts::AlertSeverity {
    match s {
        "Info" => super::alerts::AlertSeverity::Info,
        "Warning" => super::alerts::AlertSeverity::Warning,
        "Critical" => super::alerts::AlertSeverity::Critical,
        "Emergency" => super::alerts::AlertSeverity::Emergency,
        _ => super::alerts::AlertSeverity::Warning,
    }
}

fn parse_status(s: &str) -> super::alerts::AlertStatus {
    match s {
        "Active" => super::alerts::AlertStatus::Active,
        "Acknowledged" => super::alerts::AlertStatus::Acknowledged,
        "Resolved" => super::alerts::AlertStatus::Resolved,
        "Silenced" => super::alerts::AlertStatus::Silenced,
        "Flapping" => super::alerts::AlertStatus::Flapping,
        _ => super::alerts::AlertStatus::Active,
    }
}

fn parse_metric_type(s: &str) -> MetricType {
    // Parse metric type from string representation
    // This is simplified - full implementation would parse all variants
    match s {
        "CpuUsage" => MetricType::CpuUsage,
        "MemoryUsage" => MetricType::MemoryUsage,
        "DiskUsage" => MetricType::DiskUsage,
        "NetworkRxBytes" => MetricType::NetworkRxBytes,
        _ => MetricType::Custom(s.to_string()),
    }
}

fn parse_condition(s: &str) -> super::alerts::AlertCondition {
    match s {
        "GreaterThan" => super::alerts::AlertCondition::GreaterThan,
        "GreaterThanOrEqual" => super::alerts::AlertCondition::GreaterThanOrEqual,
        "LessThan" => super::alerts::AlertCondition::LessThan,
        "LessThanOrEqual" => super::alerts::AlertCondition::LessThanOrEqual,
        "Equal" => super::alerts::AlertCondition::Equal,
        "NotEqual" => super::alerts::AlertCondition::NotEqual,
        _ => super::alerts::AlertCondition::GreaterThan,
    }
}
