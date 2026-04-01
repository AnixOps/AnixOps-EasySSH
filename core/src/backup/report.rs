//! Backup reporting and notifications

use super::{BackupError, BackupJobId, BackupResult, BackupSnapshot, BackupStats, SnapshotId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn, error};

/// Backup report types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportType {
    /// Daily summary report
    Daily,
    /// Weekly summary report
    Weekly,
    /// Monthly summary report
    Monthly,
    /// Report for specific job
    JobSpecific,
    /// Failure report
    Failure,
    /// Success report
    Success,
    /// Detailed audit report
    Audit,
}

/// Report format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportFormat {
    /// Plain text
    Text,
    /// Markdown
    Markdown,
    /// HTML
    Html,
    /// JSON
    Json,
    /// CSV
    Csv,
    /// PDF (if available)
    Pdf,
}

/// Notification channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    /// Email notification
    Email {
        recipients: Vec<String>,
        smtp_server: String,
        smtp_port: u16,
        username: Option<String>,
        password: Option<String>,
        use_tls: bool,
    },
    /// Slack notification
    Slack {
        webhook_url: String,
        channel: String,
    },
    /// Discord notification
    Discord {
        webhook_url: String,
    },
    /// Microsoft Teams notification
    Teams {
        webhook_url: String,
    },
    /// Telegram notification
    Telegram {
        bot_token: String,
        chat_id: String,
    },
    /// Webhook (generic)
    Webhook {
        url: String,
        headers: HashMap<String, String>,
        method: String, // GET, POST, PUT
    },
    /// SMS notification (via Twilio)
    Sms {
        account_sid: String,
        auth_token: String,
        from_number: String,
        to_numbers: Vec<String>,
    },
    /// Desktop notification
    Desktop,
    /// Log only
    Log,
}

/// Backup metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupMetrics {
    /// Total backups
    pub total_backups: u64,
    /// Successful backups
    pub successful_backups: u64,
    /// Failed backups
    pub failed_backups: u64,
    /// Cancelled backups
    pub cancelled_backups: u64,
    /// Total data backed up (bytes)
    pub total_bytes_backed_up: u64,
    /// Total compressed size (bytes)
    pub total_compressed_bytes: u64,
    /// Compression ratio (0-1, lower is better)
    pub compression_ratio: f64,
    /// Average backup duration (seconds)
    pub avg_duration_seconds: f64,
    /// Average transfer speed (bytes/sec)
    pub avg_transfer_speed: f64,
    /// Storage used (bytes)
    pub storage_used_bytes: u64,
    /// Oldest backup
    pub oldest_backup: Option<DateTime<Utc>>,
    /// Newest backup
    pub newest_backup: Option<DateTime<Utc>>,
}

/// Backup report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupReport {
    pub report_id: String,
    pub report_type: ReportType,
    pub generated_at: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub metrics: BackupMetrics,
    pub jobs: Vec<JobReport>,
    pub failures: Vec<FailureReport>,
    pub storage_stats: StorageStats,
    pub recommendations: Vec<String>,
    pub summary: String,
}

/// Job-specific report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobReport {
    pub job_id: BackupJobId,
    pub job_name: String,
    pub total_runs: u64,
    pub successful_runs: u64,
    pub failed_runs: u64,
    pub total_bytes: u64,
    pub last_run: Option<DateTime<Utc>>,
    pub last_status: super::BackupStatus,
    pub next_run: Option<DateTime<Utc>>,
    pub avg_duration: f64,
}

/// Failure report entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureReport {
    pub job_id: BackupJobId,
    pub snapshot_id: SnapshotId,
    pub timestamp: DateTime<Utc>,
    pub error_message: String,
    pub error_category: ErrorCategory,
    pub retry_attempted: bool,
    pub retry_succeeded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCategory {
    Connection,
    Authentication,
    StorageFull,
    PermissionDenied,
    FileNotFound,
    ChecksumMismatch,
    Timeout,
    Unknown,
}

/// Storage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_snapshots: u64,
    pub total_size_bytes: u64,
    pub compressed_size_bytes: u64,
    pub deduplication_ratio: f64,
    pub storage_breakdown: HashMap<String, u64>, // location -> size
}

/// Report generator
pub struct BackupReportGenerator {
    format: ReportFormat,
    channels: Vec<NotificationChannel>,
}

impl BackupReportGenerator {
    /// Create a new report generator
    pub fn new(format: ReportFormat) -> Self {
        Self {
            format,
            channels: Vec::new(),
        }
    }

    /// Add notification channel
    pub fn add_channel(mut self, channel: NotificationChannel) -> Self {
        self.channels.push(channel);
        self
    }

    /// Generate a daily report
    pub fn generate_daily(
        &self,
        snapshots: &[BackupSnapshot],
        jobs: &[JobReport],
    ) -> BackupReport {
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);

        let daily_snapshots: Vec<_> = snapshots
            .iter()
            .filter(|s| s.created_at >= yesterday)
            .collect();

        let successful = daily_snapshots.iter().filter(|s| s.status == super::BackupStatus::Completed).count() as u64;
        let failed = daily_snapshots.iter().filter(|s| s.status == super::BackupStatus::Failed).count() as u64;

        let total_bytes: u64 = daily_snapshots.iter().map(|s| s.size_bytes).sum();
        let compressed_bytes: u64 = daily_snapshots.iter().map(|s| s.compressed_size_bytes).sum();

        let metrics = BackupMetrics {
            total_backups: daily_snapshots.len() as u64,
            successful_backups: successful,
            failed_backups: failed,
            total_bytes_backed_up: total_bytes,
            total_compressed_bytes: compressed_bytes,
            compression_ratio: if total_bytes > 0 {
                compressed_bytes as f64 / total_bytes as f64
            } else {
                0.0
            },
            ..Default::default()
        };

        let summary = format!(
            "Daily Backup Report: {} successful, {} failed, {} total data",
            successful, failed, super::format_bytes(total_bytes)
        );

        let recommendations = self.generate_recommendations(&metrics, &daily_snapshots);

        BackupReport {
            report_id: uuid::Uuid::new_v4().to_string(),
            report_type: ReportType::Daily,
            generated_at: now,
            period_start: yesterday,
            period_end: now,
            metrics,
            jobs: jobs.to_vec(),
            failures: self.extract_failures(&daily_snapshots),
            storage_stats: self.calculate_storage_stats(&daily_snapshots),
            recommendations,
            summary,
        }
    }

    /// Generate a failure notification
    pub fn generate_failure_notification(
        &self,
        job_id: BackupJobId,
        job_name: &str,
        error: &str,
    ) -> BackupReport {
        let now = Utc::now();

        let failure = FailureReport {
            job_id,
            snapshot_id: SnapshotId::new(),
            timestamp: now,
            error_message: error.to_string(),
            error_category: self.categorize_error(error),
            retry_attempted: false,
            retry_succeeded: false,
        };

        let summary = format!("Backup FAILED: {} - {}", job_name, error);

        BackupReport {
            report_id: uuid::Uuid::new_v4().to_string(),
            report_type: ReportType::Failure,
            generated_at: now,
            period_start: now,
            period_end: now,
            metrics: BackupMetrics::default(),
            jobs: vec![],
            failures: vec![failure],
            storage_stats: StorageStats::default(),
            recommendations: vec!["Check the error and retry the backup".to_string()],
            summary,
        }
    }

    /// Format report as string
    pub fn format_report(&self, report: &BackupReport) -> String {
        match self.format {
            ReportFormat::Text => self.format_as_text(report),
            ReportFormat::Markdown => self.format_as_markdown(report),
            ReportFormat::Html => self.format_as_html(report),
            ReportFormat::Json => self.format_as_json(report),
            ReportFormat::Csv => self.format_as_csv(report),
            ReportFormat::Pdf => "PDF format not implemented".to_string(),
        }
    }

    fn format_as_text(&self, report: &BackupReport) -> String {
        let mut output = String::new();

        output.push_str(&format!("{}\n", report.summary));
        output.push_str(&format!("Generated: {}\n", report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")));
        output.push_str(&format!("Period: {} to {}\n\n",
            report.period_start.format("%Y-%m-%d %H:%M"),
            report.period_end.format("%Y-%m-%d %H:%M")));

        output.push_str("=== METRICS ===\n");
        output.push_str(&format!("Total Backups: {}\n", report.metrics.total_backups));
        output.push_str(&format!("Successful: {}\n", report.metrics.successful_backups));
        output.push_str(&format!("Failed: {}\n", report.metrics.failed_backups));
        output.push_str(&format!("Data Backed Up: {}\n", super::format_bytes(report.metrics.total_bytes_backed_up)));
        output.push_str(&format!("Compressed Size: {}\n", super::format_bytes(report.metrics.total_compressed_bytes)));
        output.push_str(&format!("Compression Ratio: {:.1}%\n\n", report.metrics.compression_ratio * 100.0));

        if !report.jobs.is_empty() {
            output.push_str("=== JOBS ===\n");
            for job in &report.jobs {
                output.push_str(&format!("{}: {} runs, {} successful, {} failed\n",
                    job.job_name, job.total_runs, job.successful_runs, job.failed_runs));
            }
            output.push('\n');
        }

        if !report.failures.is_empty() {
            output.push_str("=== FAILURES ===\n");
            for failure in &report.failures {
                output.push_str(&format!("[{}] {}: {}\n",
                    failure.timestamp.format("%Y-%m-%d %H:%M"),
                    failure.job_id.0,
                    failure.error_message));
            }
            output.push('\n');
        }

        if !report.recommendations.is_empty() {
            output.push_str("=== RECOMMENDATIONS ===\n");
            for rec in &report.recommendations {
                output.push_str(&format!("- {}\n", rec));
            }
        }

        output
    }

    fn format_as_markdown(&self, report: &BackupReport) -> String {
        let mut output = String::new();

        output.push_str(&format!("# {}\n\n", report.summary));
        output.push_str(&format!("**Generated:** {}\n\n", report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")));

        output.push_str("## Metrics\n\n");
        output.push_str(&format!("- **Total Backups:** {}\n", report.metrics.total_backups));
        output.push_str(&format!("- **Successful:** {}\n", report.metrics.successful_backups));
        output.push_str(&format!("- **Failed:** {}\n", report.metrics.failed_backups));
        output.push_str(&format!("- **Data Backed Up:** {}\n", super::format_bytes(report.metrics.total_bytes_backed_up)));
        output.push_str(&format!("- **Compression Ratio:** {:.1}%\n\n", report.metrics.compression_ratio * 100.0));

        if !report.failures.is_empty() {
            output.push_str("## Failures\n\n");
            output.push_str("| Time | Job | Error |\n");
            output.push_str("|------|-----|-------|\n");
            for failure in &report.failures {
                output.push_str(&format!("| {} | {} | {} |\n",
                    failure.timestamp.format("%Y-%m-%d %H:%M"),
                    failure.job_id.0,
                    failure.error_message));
            }
            output.push('\n');
        }

        if !report.recommendations.is_empty() {
            output.push_str("## Recommendations\n\n");
            for rec in &report.recommendations {
                output.push_str(&format!("- {}\n", rec));
            }
        }

        output
    }

    fn format_as_html(&self, report: &BackupReport) -> String {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<title>Backup Report</title>\n");
        html.push_str("<style>");
        html.push_str("body { font-family: sans-serif; margin: 40px; }");
        html.push_str("h1 { color: #333; }");
        html.push_str("table { border-collapse: collapse; width: 100%; }");
        html.push_str("th, td { text-align: left; padding: 8px; border: 1px solid #ddd; }");
        html.push_str("th { background-color: #f2f2f2; }");
        html.push_str(".success { color: green; }");
        html.push_str(".failure { color: red; }");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");

        html.push_str(&format!("<h1>{}</h1>\n", report.summary));
        html.push_str(&format!("<p><strong>Generated:</strong> {}</p>\n",
            report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")));

        html.push_str("<h2>Metrics</h2>\n");
        html.push_str("<table>\n");
        html.push_str("<tr><th>Metric</th><th>Value</th></tr>\n");
        html.push_str(&format!("<tr><td>Total Backups</td><td>{}</td></tr>\n", report.metrics.total_backups));
        html.push_str(&format!("<tr><td>Successful</td><td class='success'>{}</td></tr>\n", report.metrics.successful_backups));
        html.push_str(&format!("<tr><td>Failed</td><td class='failure'>{}</td></tr>\n", report.metrics.failed_backups));
        html.push_str(&format!("<tr><td>Data Backed Up</td><td>{}</td></tr>\n", super::format_bytes(report.metrics.total_bytes_backed_up)));
        html.push_str("</table>\n");

        if !report.failures.is_empty() {
            html.push_str("<h2>Failures</h2>\n");
            html.push_str("<table>\n");
            html.push_str("<tr><th>Time</th><th>Job</th><th>Error</th></tr>\n");
            for failure in &report.failures {
                html.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td class='failure'>{}</td></tr>\n",
                    failure.timestamp.format("%Y-%m-%d %H:%M"),
                    failure.job_id.0,
                    failure.error_message
                ));
            }
            html.push_str("</table>\n");
        }

        html.push_str("</body>\n</html>");

        html
    }

    fn format_as_json(&self, report: &BackupReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
    }

    fn format_as_csv(&self, report: &BackupReport) -> String {
        let mut csv = String::new();
        csv.push_str("Type,ID,Timestamp,Status,Size,Error\n");

        for failure in &report.failures {
            csv.push_str(&format!("Failure,{},{},{},{},{}\n",
                failure.job_id.0,
                failure.timestamp.format("%Y-%m-%d %H:%M:%S"),
                "Failed",
                "0",
                failure.error_message.replace(',', ";")));
        }

        csv
    }

    /// Send report to all channels
    pub async fn send_report(&self, report: &BackupReport) -> BackupResult<()> {
        let formatted = self.format_report(report);

        for channel in &self.channels {
            match self.send_to_channel(channel, &formatted, report).await {
                Ok(_) => info!("Report sent to {:?}", channel),
                Err(e) => warn!("Failed to send report to {:?}: {}", channel, e),
            }
        }

        Ok(())
    }

    async fn send_to_channel(
        &self,
        channel: &NotificationChannel,
        message: &str,
        report: &BackupReport,
    ) -> BackupResult<()> {
        match channel {
            NotificationChannel::Email { recipients, smtp_server, smtp_port, username, password, use_tls } => {
                self.send_email(recipients, smtp_server, *smtp_port, username.as_deref(), password.as_deref(), *use_tls, message, report).await
            }
            NotificationChannel::Slack { webhook_url, channel: slack_channel } => {
                self.send_slack(webhook_url, slack_channel, report).await
            }
            NotificationChannel::Webhook { url, headers, method } => {
                self.send_webhook(url, headers, method, report).await
            }
            NotificationChannel::Log => {
                info!("Backup Report: {}", message);
                Ok(())
            }
            NotificationChannel::Desktop => {
                // Desktop notification would use notify-rust or similar
                info!("Desktop notification: {}", report.summary);
                Ok(())
            }
            _ => {
                warn!("Notification channel not implemented");
                Ok(())
            }
        }
    }

    async fn send_email(
        &self,
        recipients: &[String],
        smtp_server: &str,
        smtp_port: u16,
        username: Option<&str>,
        password: Option<&str>,
        _use_tls: bool,
        _message: &str,
        report: &BackupReport,
    ) -> BackupResult<()> {
        // Note: Full email implementation would use lettre crate
        // For now, just log
        info!("Email report would be sent to {:?} via {}:{}", recipients, smtp_server, smtp_port);
        info!("Subject: {}", report.summary);
        Ok(())
    }

    async fn send_slack(
        &self,
        webhook_url: &str,
        _channel: &str,
        report: &BackupReport,
    ) -> BackupResult<()> {
        let payload = serde_json::json!({
            "text": report.summary,
            "attachments": [{
                "color": if report.metrics.failed_backups > 0 { "danger" } else { "good" },
                "fields": [
                    {
                        "title": "Total Backups",
                        "value": report.metrics.total_backups,
                        "short": true
                    },
                    {
                        "title": "Successful",
                        "value": report.metrics.successful_backups,
                        "short": true
                    },
                    {
                        "title": "Failed",
                        "value": report.metrics.failed_backups,
                        "short": true
                    },
                    {
                        "title": "Data Backed Up",
                        "value": super::format_bytes(report.metrics.total_bytes_backed_up),
                        "short": true
                    }
                ]
            }]
        });

        let client = reqwest::Client::new();
        client.post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| BackupError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn send_webhook(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
        method: &str,
        report: &BackupReport,
    ) -> BackupResult<()> {
        let client = reqwest::Client::new();
        let mut request = match method.to_uppercase().as_str() {
            "GET" => client.get(url),
            "PUT" => client.put(url),
            _ => client.post(url),
        };

        for (key, value) in headers {
            request = request.header(key, value);
        }

        request
            .json(report)
            .send()
            .await
            .map_err(|e| BackupError::Storage(e.to_string()))?;

        Ok(())
    }

    // Helper methods

    fn extract_failures(&self, snapshots: &[&BackupSnapshot]) -> Vec<FailureReport> {
        snapshots
            .iter()
            .filter(|s| s.status == super::BackupStatus::Failed)
            .map(|s| FailureReport {
                job_id: s.job_id,
                snapshot_id: s.id,
                timestamp: s.created_at,
                error_message: s.error_message.clone().unwrap_or_else(|| "Unknown error".to_string()),
                error_category: ErrorCategory::Unknown,
                retry_attempted: false,
                retry_succeeded: false,
            })
            .collect()
    }

    fn calculate_storage_stats(&self, snapshots: &[&BackupSnapshot]) -> StorageStats {
        let total_size: u64 = snapshots.iter().map(|s| s.compressed_size_bytes).sum();
        let original_size: u64 = snapshots.iter().map(|s| s.size_bytes).sum();

        StorageStats {
            total_snapshots: snapshots.len() as u64,
            total_size_bytes: total_size,
            compressed_size_bytes: total_size,
            deduplication_ratio: if original_size > 0 {
                1.0 - (total_size as f64 / original_size as f64)
            } else {
                0.0
            },
            storage_breakdown: HashMap::new(),
        }
    }

    fn generate_recommendations(
        &self,
        metrics: &BackupMetrics,
        _snapshots: &[&BackupSnapshot],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if metrics.failed_backups > metrics.successful_backups {
            recommendations.push("High failure rate detected. Check backup configuration and storage availability.".to_string());
        }

        if metrics.compression_ratio > 0.9 {
            recommendations.push("Compression ratio is low. Consider changing compression algorithm or level.".to_string());
        }

        if metrics.total_backups == 0 {
            recommendations.push("No backups in this period. Ensure backup jobs are scheduled and enabled.".to_string());
        }

        recommendations
    }

    fn categorize_error(&self, error: &str) -> ErrorCategory {
        let error_lower = error.to_lowercase();
        if error_lower.contains("connect") || error_lower.contains("network") || error_lower.contains("timeout") {
            ErrorCategory::Connection
        } else if error_lower.contains("auth") || error_lower.contains("password") || error_lower.contains("credential") {
            ErrorCategory::Authentication
        } else if error_lower.contains("full") || error_lower.contains("space") || error_lower.contains("quota") {
            ErrorCategory::StorageFull
        } else if error_lower.contains("permission") || error_lower.contains("access denied") {
            ErrorCategory::PermissionDenied
        } else if error_lower.contains("not found") || error_lower.contains("no such file") {
            ErrorCategory::FileNotFound
        } else if error_lower.contains("checksum") || error_lower.contains("corrupt") {
            ErrorCategory::ChecksumMismatch
        } else {
            ErrorCategory::Unknown
        }
    }
}

/// Generate a quick status message
pub fn generate_status_message(stats: &BackupStats) -> String {
    format!(
        "Backups: {} total, {} successful, {} failed. Last backup: {:?}",
        stats.total_snapshots,
        stats.successful_backups,
        stats.failed_backups,
        stats.last_backup_time
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_metrics() {
        let metrics = BackupMetrics {
            total_backups: 10,
            successful_backups: 9,
            failed_backups: 1,
            total_bytes_backed_up: 1024 * 1024 * 100, // 100 MB
            total_compressed_bytes: 1024 * 1024 * 50, // 50 MB
            compression_ratio: 0.5,
            ..Default::default()
        };

        assert_eq!(metrics.total_backups, 10);
        assert_eq!(metrics.compression_ratio, 0.5);
    }

    #[test]
    fn test_format_report_text() {
        let generator = BackupReportGenerator::new(ReportFormat::Text);
        let report = BackupReport {
            report_id: "test".to_string(),
            report_type: ReportType::Daily,
            generated_at: Utc::now(),
            period_start: Utc::now(),
            period_end: Utc::now(),
            metrics: BackupMetrics::default(),
            jobs: vec![],
            failures: vec![],
            storage_stats: StorageStats::default(),
            recommendations: vec!["Test recommendation".to_string()],
            summary: "Test Report".to_string(),
        };

        let formatted = generator.format_report(&report);
        assert!(formatted.contains("Test Report"));
        assert!(formatted.contains("Test recommendation"));
    }

    #[test]
    fn test_error_categorization() {
        let generator = BackupReportGenerator::new(ReportFormat::Text);

        assert_eq!(generator.categorize_error("Connection timeout"), ErrorCategory::Connection);
        assert_eq!(generator.categorize_error("Authentication failed"), ErrorCategory::Authentication);
        assert_eq!(generator.categorize_error("Storage full"), ErrorCategory::StorageFull);
        assert_eq!(generator.categorize_error("Permission denied"), ErrorCategory::PermissionDenied);
    }
}
