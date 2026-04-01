//! SLA (Service Level Agreement) Monitoring System
//!
//! Tracks availability, uptime, incidents, and compliance with SLA targets.

use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::metrics::{Incident, MonthlyAvailability, SlaStats};
use super::{MonitoringError, TimeRange};

/// SLA monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaConfig {
    /// Default availability target (e.g., 99.9 for 99.9%)
    pub availability_target: f64,
    /// MTTR target in minutes
    pub mttr_target_minutes: f64,
    /// Maintenance windows excluded from SLA calculation
    pub exclude_maintenance_windows: bool,
    /// Alert when SLA is at risk
    pub alert_threshold_percent: f64,
    /// Time range for SLA calculation
    pub default_time_range: TimeRange,
}

impl Default for SlaConfig {
    fn default() -> Self {
        Self {
            availability_target: 99.9,
            mttr_target_minutes: 60.0,
            exclude_maintenance_windows: true,
            alert_threshold_percent: 99.5,
            default_time_range: TimeRange::Last30Days,
        }
    }
}

/// SLA monitoring service
pub struct SlaMonitor {
    config: SlaConfig,
    storage: Arc<super::storage::MetricsStorage>,
    /// Per-server SLA configurations
    server_configs: Arc<RwLock<HashMap<String, SlaConfig>>>,
    /// Current month incidents for MTTR calculation
    current_incidents: Arc<RwLock<HashMap<String, Vec<ActiveIncident>>>>,
}

/// Active incident tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveIncident {
    pub id: String,
    pub server_id: String,
    pub started_at: u64,
    pub severity: super::alerts::AlertSeverity,
    pub description: String,
    pub acknowledged_by: Option<String>,
    pub acknowledged_at: Option<u64>,
}

/// SLA breach information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaBreach {
    pub server_id: String,
    pub breach_type: SlaBreachType,
    pub severity: SlaBreachSeverity,
    pub message: String,
    pub current_value: f64,
    pub target_value: f64,
    pub timestamp: u64,
    pub recommended_action: String,
}

/// Types of SLA breaches
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlaBreachType {
    Availability,
    Mttr,
    IncidentFrequency,
    MeanTimeBetweenFailures,
}

/// SLA breach severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlaBreachSeverity {
    Warning,
    Breach,
    Critical,
}

/// SLA health status for a server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaHealthStatus {
    pub server_id: String,
    pub status: SlaStatus,
    pub current_availability: f64,
    pub target_availability: f64,
    pub days_until_month_end: u32,
    /// Maximum allowed downtime to meet SLA (in minutes)
    pub allowed_downtime_minutes: f64,
    /// Current downtime this month (in minutes)
    pub current_downtime_minutes: f64,
    /// Remaining allowed downtime (in minutes)
    pub remaining_downtime_minutes: f64,
    /// Is SLA at risk this month
    pub is_at_risk: bool,
    /// Breach forecast if trends continue
    pub breach_forecast: Option<BreachForecast>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreachForecast {
    pub predicted_breach_date: u64,
    pub predicted_final_availability: f64,
    pub confidence: f64,
}

/// SLA status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlaStatus {
    Compliant,
    AtRisk,
    Breached,
    Unknown,
}

impl SlaMonitor {
    pub fn new(config: SlaConfig, storage: Arc<super::storage::MetricsStorage>) -> Self {
        Self {
            config,
            storage,
            server_configs: Arc::new(RwLock::new(HashMap::new())),
            current_incidents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set custom SLA config for a server
    pub async fn set_server_config(
        &self,
        server_id: &str,
        config: SlaConfig,
    ) -> Result<(), MonitoringError> {
        let mut configs = self.server_configs.write().await;
        configs.insert(server_id.to_string(), config);
        Ok(())
    }

    /// Get SLA config for a server (or default)
    pub async fn get_server_config(&self, server_id: &str) -> SlaConfig {
        let configs = self.server_configs.read().await;
        configs
            .get(server_id)
            .cloned()
            .unwrap_or_else(|| self.config.clone())
    }

    /// Calculate SLA statistics for a server
    pub async fn calculate_sla(
        &self,
        server_id: &str,
        time_range: TimeRange,
    ) -> Result<SlaStats, MonitoringError> {
        self.storage.calculate_sla(server_id, time_range).await
    }

    /// Get SLA health status for a server
    pub async fn get_health_status(
        &self,
        server_id: &str,
    ) -> Result<SlaHealthStatus, MonitoringError> {
        let config = self.get_server_config(server_id).await;
        let time_range = TimeRange::Last30Days;
        let stats = self.calculate_sla(server_id, time_range).await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Calculate days until month end
        let now_datetime =
            chrono::DateTime::from_timestamp(now as i64, 0).unwrap_or_else(|| chrono::Utc::now());
        let naive_date = now_datetime.naive_utc();
        let current_month = naive_date.month();
        let current_year = naive_date.year();
        let days_in_month = chrono::NaiveDate::from_ymd_opt(
            if current_month == 12 {
                current_year + 1
            } else {
                current_year
            },
            if current_month == 12 {
                1
            } else {
                current_month + 1
            },
            1,
        )
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(30);
        let days_until_month_end = days_in_month - naive_date.day() + 1;

        // Calculate allowed downtime for the month
        let total_minutes_in_month = (days_in_month as f64) * 24.0 * 60.0;
        let allowed_downtime_minutes =
            total_minutes_in_month * (100.0 - config.availability_target) / 100.0;
        let current_downtime_minutes = stats.downtime_minutes as f64;
        let remaining_downtime_minutes = allowed_downtime_minutes - current_downtime_minutes;

        // Determine status
        let status = if stats.uptime_percent >= config.availability_target {
            SlaStatus::Compliant
        } else if stats.uptime_percent >= config.alert_threshold_percent {
            SlaStatus::AtRisk
        } else {
            SlaStatus::Breached
        };

        // Check if at risk
        let is_at_risk = remaining_downtime_minutes < (allowed_downtime_minutes * 0.2);

        // Generate breach forecast if at risk
        let breach_forecast = if is_at_risk {
            self.calculate_breach_forecast(server_id, &stats, remaining_downtime_minutes)
                .await
        } else {
            None
        };

        Ok(SlaHealthStatus {
            server_id: server_id.to_string(),
            status,
            current_availability: stats.uptime_percent,
            target_availability: config.availability_target,
            days_until_month_end,
            allowed_downtime_minutes,
            current_downtime_minutes,
            remaining_downtime_minutes,
            is_at_risk,
            breach_forecast,
        })
    }

    /// Calculate breach forecast based on current trends
    async fn calculate_breach_forecast(
        &self,
        _server_id: &str,
        stats: &SlaStats,
        remaining_downtime_minutes: f64,
    ) -> Option<BreachForecast> {
        if stats.incidents.len() < 2 {
            return None;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Calculate average incident rate
        let total_incident_minutes: f64 = stats
            .incidents
            .iter()
            .map(|i| i.duration_minutes as f64)
            .sum();

        let days_in_period = 30.0;
        let avg_downtime_per_day = total_incident_minutes / days_in_period;

        if avg_downtime_per_day <= 0.0 {
            return None;
        }

        // Calculate days until breach
        let days_until_breach = remaining_downtime_minutes / avg_downtime_per_day;

        if days_until_breach < 0.0 {
            // Already breached
            return Some(BreachForecast {
                predicted_breach_date: now,
                predicted_final_availability: stats.uptime_percent,
                confidence: 0.9,
            });
        }

        let breach_timestamp = now + (days_until_breach * 86400.0) as u64;

        // Calculate predicted final availability
        let current_month_days = 30.0;
        let predicted_additional_downtime =
            avg_downtime_per_day * (current_month_days - days_in_period);
        let total_predicted_downtime = total_incident_minutes + predicted_additional_downtime;
        let total_minutes_in_month = current_month_days * 24.0 * 60.0;
        let predicted_availability =
            ((total_minutes_in_month - total_predicted_downtime) / total_minutes_in_month) * 100.0;

        Some(BreachForecast {
            predicted_breach_date: breach_timestamp,
            predicted_final_availability: predicted_availability.max(0.0),
            confidence: 0.75,
        })
    }

    /// Record a new incident
    pub async fn record_incident(
        &self,
        server_id: &str,
        severity: super::alerts::AlertSeverity,
        description: &str,
    ) -> Result<String, MonitoringError> {
        let incident_id = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let incident = ActiveIncident {
            id: incident_id.clone(),
            server_id: server_id.to_string(),
            started_at: now,
            severity,
            description: description.to_string(),
            acknowledged_by: None,
            acknowledged_at: None,
        };

        let mut incidents = self.current_incidents.write().await;
        incidents
            .entry(server_id.to_string())
            .or_default()
            .push(incident);

        Ok(incident_id)
    }

    /// Resolve an incident
    pub async fn resolve_incident(
        &self,
        server_id: &str,
        incident_id: &str,
    ) -> Result<(), MonitoringError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut incidents = self.current_incidents.write().await;
        if let Some(server_incidents) = incidents.get_mut(server_id) {
            if let Some(incident) = server_incidents.iter_mut().find(|i| i.id == incident_id) {
                // Calculate duration and store in SLA records
                let duration = now - incident.started_at;
                self.store_incident_record(server_id, incident, duration)
                    .await?;
            }
            server_incidents.retain(|i| i.id != incident_id);
        }

        Ok(())
    }

    /// Acknowledge an incident
    pub async fn acknowledge_incident(
        &self,
        server_id: &str,
        incident_id: &str,
        user_id: &str,
    ) -> Result<(), MonitoringError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut incidents = self.current_incidents.write().await;
        if let Some(server_incidents) = incidents.get_mut(server_id) {
            if let Some(incident) = server_incidents.iter_mut().find(|i| i.id == incident_id) {
                incident.acknowledged_by = Some(user_id.to_string());
                incident.acknowledged_at = Some(now);
            }
        }

        Ok(())
    }

    /// Store incident record in database
    async fn store_incident_record(
        &self,
        server_id: &str,
        incident: &ActiveIncident,
        duration_seconds: u64,
    ) -> Result<(), MonitoringError> {
        let _date = chrono::DateTime::from_timestamp(incident.started_at as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let availability_percent = ((86400.0 - (duration_seconds as f64 / 60.0)) / 86400.0) * 100.0;

        // Store in SLA records via storage
        // This would update the sla_records table
        log::info!(
            "[SLA] Incident recorded for {}: {} seconds, availability impact: {:.2}%",
            server_id,
            duration_seconds,
            100.0 - availability_percent
        );

        Ok(())
    }

    /// Check for SLA breaches
    pub async fn check_sla_breaches(
        &self,
        server_ids: Option<&[String]>,
    ) -> Result<Vec<SlaBreach>, MonitoringError> {
        let mut breaches = Vec::new();

        let servers = if let Some(ids) = server_ids {
            ids.to_vec()
        } else {
            self.storage.get_monitored_servers().await?
        };

        for server_id in servers {
            let stats = self
                .calculate_sla(&server_id, self.config.default_time_range)
                .await?;
            let config = self.get_server_config(&server_id).await;

            // Check availability breach
            if stats.uptime_percent < config.availability_target {
                let severity = if stats.uptime_percent < config.availability_target - 1.0 {
                    SlaBreachSeverity::Critical
                } else {
                    SlaBreachSeverity::Breach
                };

                breaches.push(SlaBreach {
                    server_id: server_id.clone(),
                    breach_type: SlaBreachType::Availability,
                    severity,
                    message: format!(
                        "SLA breach: {:.2}% availability (target: {:.2}%)",
                        stats.uptime_percent, config.availability_target
                    ),
                    current_value: stats.uptime_percent,
                    target_value: config.availability_target,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    recommended_action:
                        "Investigate root cause and implement high availability measures"
                            .to_string(),
                });
            }

            // Check MTTR breach
            if stats.mttr_minutes > config.mttr_target_minutes {
                breaches.push(SlaBreach {
                    server_id: server_id.clone(),
                    breach_type: SlaBreachType::Mttr,
                    severity: SlaBreachSeverity::Warning,
                    message: format!(
                        "MTTR SLA at risk: {:.1} minutes average (target: {:.1} minutes)",
                        stats.mttr_minutes, config.mttr_target_minutes
                    ),
                    current_value: stats.mttr_minutes,
                    target_value: config.mttr_target_minutes,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    recommended_action: "Improve incident response procedures and automation"
                        .to_string(),
                });
            }

            // Check incident frequency (more than 3 critical incidents per month)
            let critical_count = stats
                .incidents
                .iter()
                .filter(|i| {
                    matches!(
                        i.severity,
                        super::alerts::AlertSeverity::Critical
                            | super::alerts::AlertSeverity::Emergency
                    )
                })
                .count();

            if critical_count > 3 {
                breaches.push(SlaBreach {
                    server_id: server_id.clone(),
                    breach_type: SlaBreachType::IncidentFrequency,
                    severity: SlaBreachSeverity::Warning,
                    message: format!(
                        "High incident frequency: {} critical incidents this period",
                        critical_count
                    ),
                    current_value: critical_count as f64,
                    target_value: 3.0,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    recommended_action: "Review system stability and implement preventive measures"
                        .to_string(),
                });
            }
        }

        Ok(breaches)
    }

    /// Get SLA dashboard data
    pub async fn get_sla_dashboard(
        &self,
        server_ids: Option<&[String]>,
    ) -> Result<SlaDashboard, MonitoringError> {
        let servers = if let Some(ids) = server_ids {
            ids.to_vec()
        } else {
            self.storage.get_monitored_servers().await?
        };

        let mut server_statuses = Vec::new();
        let mut overall_uptime = 0.0;
        let mut total_incidents = 0;
        let mut compliant_count = 0;
        let mut at_risk_count = 0;
        let mut breached_count = 0;

        for server_id in &servers {
            if let Ok(status) = self.get_health_status(server_id).await {
                overall_uptime += status.current_availability;

                match status.status {
                    SlaStatus::Compliant => compliant_count += 1,
                    SlaStatus::AtRisk => at_risk_count += 1,
                    SlaStatus::Breached => breached_count += 1,
                    _ => {}
                }

                server_statuses.push(status);
            }

            if let Ok(stats) = self.calculate_sla(server_id, TimeRange::Last30Days).await {
                total_incidents += stats.incidents.len();
            }
        }

        let avg_uptime = if !servers.is_empty() {
            overall_uptime / servers.len() as f64
        } else {
            100.0
        };

        Ok(SlaDashboard {
            total_servers: servers.len(),
            compliant_count,
            at_risk_count,
            breached_count,
            overall_availability: avg_uptime,
            total_incidents,
            server_statuses,
            time_range: TimeRange::Last30Days,
        })
    }

    /// Generate SLA report for a period
    pub async fn generate_report(
        &self,
        server_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<SlaReport, MonitoringError> {
        let start = chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
            .map_err(|e| MonitoringError::Config(format!("Invalid start date: {}", e)))?
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| MonitoringError::Config("Invalid start date time".to_string()))?;

        let end = chrono::NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
            .map_err(|e| MonitoringError::Config(format!("Invalid end date: {}", e)))?
            .and_hms_opt(23, 59, 59)
            .ok_or_else(|| MonitoringError::Config("Invalid end date time".to_string()))?;

        let start_timestamp = start.and_utc().timestamp() as u64;
        let end_timestamp = end.and_utc().timestamp() as u64;
        let time_range = TimeRange::Custom {
            start: start_timestamp,
            end: end_timestamp,
        };

        let stats = self.calculate_sla(server_id, time_range.clone()).await?;
        let health = self.get_health_status(server_id).await?;

        let config = self.get_server_config(server_id).await;
        let is_compliant = stats.uptime_percent >= config.availability_target;

        // Generate recommendations before moving stats fields
        let recommendations = self.generate_recommendations(&stats, &health);

        Ok(SlaReport {
            server_id: server_id.to_string(),
            start_date: start_date.to_string(),
            end_date: end_date.to_string(),
            availability_percent: stats.uptime_percent,
            target_percent: config.availability_target,
            is_compliant,
            downtime_minutes: stats.downtime_minutes,
            incident_count: stats.incidents.len(),
            mttr_minutes: stats.mttr_minutes,
            mtbf_minutes: stats.mtbf_minutes,
            health_status: health,
            incidents: stats.incidents,
            monthly_breakdown: stats.monthly_availability,
            recommendations,
        })
    }

    fn generate_recommendations(&self, stats: &SlaStats, health: &SlaHealthStatus) -> Vec<String> {
        let mut recommendations = Vec::new();

        if stats.uptime_percent < health.target_availability {
            recommendations.push(
                "Implement redundant systems and failover mechanisms to improve availability"
                    .to_string(),
            );
        }

        if stats.mttr_minutes > 60.0 {
            recommendations.push(
                "Establish automated incident response and on-call procedures to reduce MTTR"
                    .to_string(),
            );
        }

        if stats.incidents.len() > 5 {
            recommendations.push(
                "Conduct root cause analysis on recurring incidents to prevent future occurrences"
                    .to_string(),
            );
        }

        if health.is_at_risk {
            recommendations.push(
                "Monitor SLA metrics closely and prepare capacity expansion plans".to_string(),
            );
        }

        recommendations
    }

    /// Start SLA monitoring background tasks
    pub async fn start(&self) -> Result<(), MonitoringError> {
        let check_interval = std::time::Duration::from_secs(300); // 5 minutes
        let _server_configs = Arc::clone(&self.server_configs);
        let _storage = Arc::clone(&self.storage);
        let current_incidents = Arc::clone(&self.current_incidents);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);

            loop {
                interval.tick().await;

                // Auto-resolve old incidents
                let mut incidents = current_incidents.write().await;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                for (_, server_incidents) in incidents.iter_mut() {
                    server_incidents.retain(|i| {
                        // Keep incidents less than 24 hours old
                        now - i.started_at < 86400
                    });
                }

                drop(incidents);

                // Check for SLA breaches and alert
                // This would trigger alerts via the alert engine
            }
        });

        Ok(())
    }
}

/// SLA dashboard summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaDashboard {
    pub total_servers: usize,
    pub compliant_count: usize,
    pub at_risk_count: usize,
    pub breached_count: usize,
    pub overall_availability: f64,
    pub total_incidents: usize,
    pub server_statuses: Vec<SlaHealthStatus>,
    pub time_range: TimeRange,
}

/// Detailed SLA report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaReport {
    pub server_id: String,
    pub start_date: String,
    pub end_date: String,
    pub availability_percent: f64,
    pub target_percent: f64,
    pub is_compliant: bool,
    pub downtime_minutes: u64,
    pub incident_count: usize,
    pub mttr_minutes: f64,
    pub mtbf_minutes: f64,
    pub health_status: SlaHealthStatus,
    pub incidents: Vec<Incident>,
    pub monthly_breakdown: Vec<MonthlyAvailability>,
    pub recommendations: Vec<String>,
}
