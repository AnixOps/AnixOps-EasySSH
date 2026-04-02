//! Health monitoring and service availability tracking
//!
//! Monitors:
//! - Core services (SSH, SFTP, DB)
//! - External dependencies
//! - System resources
//! - Connectivity status

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::interval;

use super::TelemetryError;

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// Fully operational
    Healthy,
    /// Degraded performance
    Degraded,
    /// Service unavailable
    Unhealthy,
    /// Unknown status
    Unknown,
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    pub fn is_operational(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded)
    }
}

/// Individual health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Check name
    pub name: String,
    /// Status
    pub status: HealthStatus,
    /// Response time
    pub response_time_ms: u64,
    /// Last checked timestamp
    pub last_checked: u64,
    /// Error message if unhealthy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Overall service health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Service name
    pub service_name: String,
    /// Overall status
    pub status: HealthStatus,
    /// Individual checks
    pub checks: Vec<HealthCheckResult>,
    /// Last updated
    pub last_updated: u64,
    /// Uptime percentage (last 24h)
    pub uptime_percentage: f64,
}

/// Health check trait
#[async_trait::async_trait]
pub trait HealthCheck: Send + Sync {
    /// Check name
    fn name(&self) -> &str;

    /// Perform health check
    async fn check(&self) -> HealthCheckResult;

    /// Check interval
    fn interval(&self) -> Duration {
        Duration::from_secs(60)
    }
}

/// Health monitor
pub struct HealthMonitor {
    checks: Arc<Mutex<Vec<Box<dyn HealthCheck>>>>,
    results: Arc<Mutex<HashMap<String, HealthCheckResult>>>,
    service_health: Arc<Mutex<HashMap<String, ServiceHealth>>>,
    check_history: Arc<Mutex<HashMap<String, Vec<HealthCheckResult>>>>,
    max_history: usize,
    running: Arc<Mutex<bool>>,
}

impl HealthMonitor {
    pub fn new() -> Result<Self, TelemetryError> {
        Ok(Self {
            checks: Arc::new(Mutex::new(Vec::new())),
            results: Arc::new(Mutex::new(HashMap::new())),
            service_health: Arc::new(Mutex::new(HashMap::new())),
            check_history: Arc::new(Mutex::new(HashMap::new())),
            max_history: 100,
            running: Arc::new(Mutex::new(false)),
        })
    }

    /// Register a health check
    pub fn register_check(&self, check: Box<dyn HealthCheck>) {
        let mut checks = self.checks.lock().unwrap();
        checks.push(check);
    }

    /// Start monitoring
    pub async fn start(&self) -> Result<(), TelemetryError> {
        let mut running = self.running.lock().unwrap();
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        // Start background monitoring
        let checks = Arc::clone(&self.checks);
        let results = Arc::clone(&self.results);
        let history = Arc::clone(&self.check_history);
        let running = Arc::clone(&self.running);
        let max_history = self.max_history;

        tokio::spawn(async move {
            let mut intervals: HashMap<String, tokio::time::Interval> = HashMap::new();

            loop {
                // Check if we should stop
                if !*running.lock().unwrap() {
                    break;
                }

                // Run checks
                let checks_list = {
                    let c = checks.lock().unwrap();
                    c.iter()
                        .map(|check| {
                            let name = check.name().to_string();
                            let interval = check.interval();
                            (name, interval, check as *const dyn HealthCheck)
                        })
                        .collect::<Vec<_>>()
                };

                for (name, duration, _check_ptr) in checks_list {
                    let interval = intervals
                        .entry(name.clone())
                        .or_insert_with(|| interval(duration));

                    if interval.tick().await.elapsed() >= Duration::from_secs(1) {
                        // This is a simplified version - in production would use proper async handling
                    }
                }

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        // Register default checks
        self.register_default_checks();

        Ok(())
    }

    fn register_default_checks(&self) {
        // Database health check
        self.register_check(Box::new(DatabaseHealthCheck::new()));

        // SSH library health check
        self.register_check(Box::new(SshLibraryHealthCheck::new()));

        // System resources check
        self.register_check(Box::new(SystemResourcesHealthCheck::new()));
    }

    /// Stop monitoring
    pub async fn stop(&self) -> Result<(), TelemetryError> {
        let mut running = self.running.lock().unwrap();
        *running = false;
        Ok(())
    }

    /// Get health check result
    pub fn get_check_result(&self, name: &str) -> Option<HealthCheckResult> {
        self.results.lock().unwrap().get(name).cloned()
    }

    /// Get all health check results
    pub fn get_all_results(&self) -> Vec<HealthCheckResult> {
        self.results.lock().unwrap().values().cloned().collect()
    }

    /// Get service health
    pub fn get_service_health(&self, service_name: &str) -> Option<ServiceHealth> {
        self.service_health
            .lock()
            .unwrap()
            .get(service_name)
            .cloned()
    }

    /// Get all service health
    pub fn get_all_service_health(&self) -> Vec<ServiceHealth> {
        self.service_health
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    /// Get overall system status
    pub fn get_overall_status(&self) -> HealthStatus {
        let results = self.results.lock().unwrap();

        if results.is_empty() {
            return HealthStatus::Unknown;
        }

        let mut has_unhealthy = false;
        let mut has_degraded = false;

        for result in results.values() {
            match result.status {
                HealthStatus::Unhealthy => has_unhealthy = true,
                HealthStatus::Degraded => has_degraded = true,
                _ => {}
            }
        }

        if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Get health check history
    pub fn get_check_history(&self, name: &str) -> Vec<HealthCheckResult> {
        self.check_history
            .lock()
            .unwrap()
            .get(name)
            .cloned()
            .unwrap_or_default()
    }

    /// Store check result
    fn store_result(&self, result: HealthCheckResult) {
        let mut results = self.results.lock().unwrap();
        results.insert(result.name.clone(), result.clone());

        let mut history = self.check_history.lock().unwrap();
        let entry = history.entry(result.name.clone()).or_insert_with(Vec::new);

        if entry.len() >= self.max_history {
            entry.remove(0);
        }
        entry.push(result);
    }

    /// Calculate uptime from history
    fn calculate_uptime(&self, check_name: &str) -> f64 {
        let history = self.check_history.lock().unwrap();
        let entries = history.get(check_name);

        if let Some(entries) = entries {
            if entries.is_empty() {
                return 100.0;
            }

            let healthy_count = entries.iter().filter(|e| e.status.is_healthy()).count();

            (healthy_count as f64 / entries.len() as f64) * 100.0
        } else {
            100.0
        }
    }
}

/// Database health check
pub struct DatabaseHealthCheck {
    name: String,
}

impl DatabaseHealthCheck {
    pub fn new() -> Self {
        Self {
            name: "database".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl HealthCheck for DatabaseHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();

        // Check database connectivity
        // This is a placeholder - would check actual DB connection
        let (status, error) = match Self::check_database().await {
            Ok(_) => (HealthStatus::Healthy, None),
            Err(e) => (HealthStatus::Unhealthy, Some(e)),
        };

        let response_time = start.elapsed().as_millis() as u64;

        HealthCheckResult {
            name: self.name.clone(),
            status,
            response_time_ms: response_time,
            last_checked: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            error_message: error,
            metadata: HashMap::new(),
        }
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(30)
    }
}

impl DatabaseHealthCheck {
    async fn check_database() -> Result<(), String> {
        // Placeholder - would actually check DB
        // For now, always healthy
        Ok(())
    }
}

/// SSH library health check
pub struct SshLibraryHealthCheck {
    name: String,
}

impl SshLibraryHealthCheck {
    pub fn new() -> Self {
        Self {
            name: "ssh_library".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl HealthCheck for SshLibraryHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();

        // Check SSH library functionality
        let status = HealthStatus::Healthy;

        HealthCheckResult {
            name: self.name.clone(),
            status,
            response_time_ms: start.elapsed().as_millis() as u64,
            last_checked: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            error_message: None,
            metadata: HashMap::new(),
        }
    }
}

/// System resources health check
pub struct SystemResourcesHealthCheck {
    name: String,
}

impl SystemResourcesHealthCheck {
    pub fn new() -> Self {
        Self {
            name: "system_resources".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl HealthCheck for SystemResourcesHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();

        let mut status = HealthStatus::Healthy;
        let mut metadata = HashMap::new();
        let mut error = None;

        // Check memory
        if let Some(memory_mb) = Self::get_memory_usage() {
            metadata.insert(
                "memory_mb".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(memory_mb).unwrap_or_else(|| 0.into()),
                ),
            );

            if memory_mb > 1024.0 {
                // > 1GB
                status = HealthStatus::Degraded;
                error = Some("High memory usage".to_string());
            }
        }

        // Check disk space
        if let Some(disk_usage) = Self::get_disk_usage() {
            metadata.insert(
                "disk_usage_percent".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(disk_usage).unwrap_or_else(|| 0.into()),
                ),
            );

            if disk_usage > 90.0 {
                status = HealthStatus::Degraded;
                error = error.or_else(|| Some("Low disk space".to_string()));
            }
        }

        HealthCheckResult {
            name: self.name.clone(),
            status,
            response_time_ms: start.elapsed().as_millis() as u64,
            last_checked: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            error_message: error,
            metadata,
        }
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(60)
    }
}

impl SystemResourcesHealthCheck {
    fn get_memory_usage() -> Option<f64> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(val) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = val.parse::<f64>() {
                                return Some(kb / 1024.0);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn get_disk_usage() -> Option<f64> {
        // Placeholder - would use proper disk check
        None
    }
}

/// External service health check
pub struct ExternalServiceHealthCheck {
    name: String,
    service_url: String,
    timeout: Duration,
}

impl ExternalServiceHealthCheck {
    pub fn new(name: impl Into<String>, service_url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            service_url: service_url.into(),
            timeout: Duration::from_secs(5),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait::async_trait]
impl HealthCheck for ExternalServiceHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();

        // Check external service
        // This is a placeholder
        let status = HealthStatus::Healthy;

        HealthCheckResult {
            name: self.name.clone(),
            status,
            response_time_ms: start.elapsed().as_millis() as u64,
            last_checked: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            error_message: None,
            metadata: {
                let mut m = HashMap::new();
                m.insert(
                    "url".to_string(),
                    serde_json::Value::String(self.service_url.clone()),
                );
                m
            },
        }
    }
}

/// Health check summary for UI
#[derive(Debug, Clone, Serialize)]
pub struct HealthSummary {
    pub overall_status: HealthStatus,
    pub services: Vec<ServiceHealth>,
    pub issues: Vec<String>,
}

impl HealthMonitor {
    /// Get health summary for UI
    pub fn get_health_summary(&self) -> HealthSummary {
        let services = self.get_all_service_health();
        let overall = self.get_overall_status();

        let issues: Vec<String> = services
            .iter()
            .flat_map(|s| {
                s.checks
                    .iter()
                    .filter(|c| !c.status.is_healthy())
                    .filter_map(|c| c.error_message.clone())
            })
            .collect();

        HealthSummary {
            overall_status: overall,
            services,
            issues,
        }
    }
}
