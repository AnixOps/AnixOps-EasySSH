//! Backup scheduler with cron-style expression support

use super::{BackupConfig, BackupError, BackupJobId, BackupResult};
use chrono::{DateTime, Duration, Utc};
use cron_parser::parse as parse_cron;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time;
use tracing::{error, info, warn};

/// Cron-style schedule expression
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CronSchedule {
    /// Cron expression (e.g., "0 2 * * *" for daily at 2 AM)
    pub expression: String,
    /// Timezone (defaults to UTC)
    pub timezone: Option<String>,
    /// Optional start date
    pub start_date: Option<DateTime<Utc>>,
    /// Optional end date
    pub end_date: Option<DateTime<Utc>>,
    /// Maximum number of runs
    pub max_runs: Option<u32>,
}

impl CronSchedule {
    /// Create a new cron schedule
    pub fn new(expression: &str) -> Self {
        Self {
            expression: expression.to_string(),
            timezone: None,
            start_date: None,
            end_date: None,
            max_runs: None,
        }
    }

    /// Set timezone
    pub fn with_timezone(mut self, timezone: &str) -> Self {
        self.timezone = Some(timezone.to_string());
        self
    }

    /// Set start date
    pub fn with_start_date(mut self, start: DateTime<Utc>) -> Self {
        self.start_date = Some(start);
        self
    }

    /// Set end date
    pub fn with_end_date(mut self, end: DateTime<Utc>) -> Self {
        self.end_date = Some(end);
        self
    }

    /// Set max runs
    pub fn with_max_runs(mut self, runs: u32) -> Self {
        self.max_runs = Some(runs);
        self
    }

    /// Get the next execution time after the given time
    pub fn next_execution(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        // Parse the cron expression
        let base_time = self.start_date.unwrap_or(after);
        let effective_after = if after < base_time { base_time } else { after };

        match parse_cron(&self.expression, effective_after) {
            Ok(next) => {
                let next_utc: DateTime<Utc> = next.into();

                // Check if it's before the end date
                if let Some(end) = self.end_date {
                    if next_utc > end {
                        return None;
                    }
                }

                Some(next_utc)
            }
            Err(e) => {
                warn!(
                    "Failed to parse cron expression '{}': {}",
                    self.expression, e
                );
                None
            }
        }
    }

    /// Validate the cron expression
    pub fn is_valid(&self) -> bool {
        parse_cron(&self.expression, Utc::now()).is_ok()
    }

    /// Get human-readable description
    pub fn description(&self) -> String {
        // Simple description generation
        let parts: Vec<&str> = self.expression.split_whitespace().collect();
        if parts.len() != 5 {
            return "Invalid cron expression".to_string();
        }

        let minute = parts[0];
        let hour = parts[1];
        let day = parts[2];
        let month = parts[3];
        let weekday = parts[4];

        if minute == "0" && hour == "0" && day == "*" && month == "*" && weekday == "*" {
            "Daily at midnight".to_string()
        } else if minute == "0" && day == "*" && month == "*" && weekday == "*" {
            format!("Daily at {}:00", hour)
        } else if minute == "0" && hour == "0" && day == "*" && month == "*" {
            format!("Weekly on day {}", weekday)
        } else if minute == "0" && hour == "0" && day == "1" && month == "*" {
            "Monthly on the 1st".to_string()
        } else {
            format!("Cron: {}", self.expression)
        }
    }
}

impl Default for CronSchedule {
    fn default() -> Self {
        Self {
            expression: "0 2 * * *".to_string(), // Daily at 2 AM
            timezone: None,
            start_date: None,
            end_date: None,
            max_runs: None,
        }
    }
}

/// Schedule configuration for a backup job
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// Cron schedule
    pub cron: CronSchedule,
    /// Enable the schedule
    pub enabled: bool,
    /// Delay before first run (seconds)
    pub initial_delay_seconds: u64,
    /// Retry on failure
    pub retry_on_failure: bool,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry delay (seconds)
    pub retry_delay_seconds: u64,
    /// Missed execution policy
    pub missed_execution_policy: MissedExecutionPolicy,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            cron: CronSchedule::default(),
            enabled: true,
            initial_delay_seconds: 0,
            retry_on_failure: true,
            max_retries: 3,
            retry_delay_seconds: 300, // 5 minutes
            missed_execution_policy: MissedExecutionPolicy::Skip,
        }
    }
}

/// Policy for handling missed executions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissedExecutionPolicy {
    /// Skip missed executions
    Skip,
    /// Run immediately when scheduler starts
    RunImmediately,
    /// Run only the most recent missed execution
    RunLastOnly,
    /// Run all missed executions
    RunAll,
}

/// Scheduled job information
#[derive(Debug, Clone)]
pub struct ScheduledJob {
    pub job_id: BackupJobId,
    pub config: BackupConfig,
    pub schedule: ScheduleConfig,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub run_count: u32,
    pub is_running: bool,
}

/// Job trigger event
#[derive(Debug, Clone)]
pub enum JobTrigger {
    /// Triggered by schedule
    Scheduled,
    /// Manual trigger
    Manual,
    /// Retry after failure
    Retry,
    /// Missed execution
    MissedExecution,
}

/// Scheduler event
#[derive(Debug, Clone)]
pub enum SchedulerEvent {
    /// Job should be executed
    JobTriggered(BackupJobId, JobTrigger),
    /// Job completed
    JobCompleted(BackupJobId, bool), // bool = success
    /// Schedule updated
    ScheduleUpdated(BackupJobId),
    /// Schedule removed
    ScheduleRemoved(BackupJobId),
}

/// Backup scheduler with cron support
pub struct BackupScheduler {
    jobs: Arc<RwLock<HashMap<BackupJobId, ScheduledJob>>>,
    event_tx: mpsc::Sender<SchedulerEvent>,
    event_rx: Arc<Mutex<mpsc::Receiver<SchedulerEvent>>>,
    shutdown_tx: Arc<Mutex<mpsc::Sender<()>>>,
    shutdown_rx: Arc<Mutex<mpsc::Receiver<()>>>,
    is_running: Arc<RwLock<bool>>,
}

impl Clone for BackupScheduler {
    fn clone(&self) -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        Self {
            jobs: self.jobs.clone(),
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            shutdown_tx: Arc::new(Mutex::new(shutdown_tx)),
            shutdown_rx: Arc::new(Mutex::new(shutdown_rx)),
            is_running: self.is_running.clone(),
        }
    }
}

impl BackupScheduler {
    /// Create a new backup scheduler
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            shutdown_tx: Arc::new(Mutex::new(shutdown_tx)),
            shutdown_rx: Arc::new(Mutex::new(shutdown_rx)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Add a scheduled job
    pub async fn add_job(
        &self,
        job_id: BackupJobId,
        config: BackupConfig,
        schedule: ScheduleConfig,
    ) -> BackupResult<()> {
        if !schedule.cron.is_valid() {
            return Err(BackupError::Schedule(format!(
                "Invalid cron expression: {}",
                schedule.cron.expression
            )));
        }

        let next_run = if schedule.enabled {
            let now = Utc::now();
            let after = now + Duration::seconds(schedule.initial_delay_seconds as i64);
            schedule.cron.next_execution(after)
        } else {
            None
        };

        let job = ScheduledJob {
            job_id,
            config,
            schedule,
            last_run: None,
            next_run,
            run_count: 0,
            is_running: false,
        };

        let mut jobs = self.jobs.write().await;
        jobs.insert(job_id, job);

        info!(
            "Added scheduled job {} with next run at {:?}",
            job_id.0, next_run
        );

        self.event_tx
            .send(SchedulerEvent::ScheduleUpdated(job_id))
            .await
            .map_err(|e| BackupError::Schedule(e.to_string()))?;

        Ok(())
    }

    /// Remove a scheduled job
    pub async fn remove_job(&self, job_id: BackupJobId) -> BackupResult<()> {
        let mut jobs = self.jobs.write().await;
        jobs.remove(&job_id);

        info!("Removed scheduled job {}", job_id.0);

        self.event_tx
            .send(SchedulerEvent::ScheduleRemoved(job_id))
            .await
            .map_err(|e| BackupError::Schedule(e.to_string()))?;

        Ok(())
    }

    /// Get a scheduled job
    pub async fn get_job(&self, job_id: BackupJobId) -> Option<ScheduledJob> {
        let jobs = self.jobs.read().await;
        jobs.get(&job_id).cloned()
    }

    /// Get all scheduled jobs
    pub async fn get_all_jobs(&self) -> Vec<ScheduledJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Update a job's schedule
    pub async fn update_schedule(
        &self,
        job_id: BackupJobId,
        schedule: ScheduleConfig,
    ) -> BackupResult<()> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(&job_id) {
            if !schedule.cron.is_valid() {
                return Err(BackupError::Schedule(format!(
                    "Invalid cron expression: {}",
                    schedule.cron.expression
                )));
            }

            job.schedule = schedule;
            job.next_run = if job.schedule.enabled {
                job.schedule.cron.next_execution(Utc::now())
            } else {
                None
            };

            info!(
                "Updated schedule for job {}: next run at {:?}",
                job_id.0, job.next_run
            );

            self.event_tx
                .send(SchedulerEvent::ScheduleUpdated(job_id))
                .await
                .map_err(|e| BackupError::Schedule(e.to_string()))?;

            Ok(())
        } else {
            Err(BackupError::JobNotFound(job_id))
        }
    }

    /// Enable/disable a job
    pub async fn set_job_enabled(&self, job_id: BackupJobId, enabled: bool) -> BackupResult<()> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(&job_id) {
            job.schedule.enabled = enabled;
            job.next_run = if enabled {
                job.schedule.cron.next_execution(Utc::now())
            } else {
                None
            };

            info!(
                "Job {} {}",
                job_id.0,
                if enabled { "enabled" } else { "disabled" }
            );
            Ok(())
        } else {
            Err(BackupError::JobNotFound(job_id))
        }
    }

    /// Mark a job as running
    pub async fn mark_job_running(&self, job_id: BackupJobId, running: bool) -> BackupResult<()> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(&job_id) {
            job.is_running = running;
            Ok(())
        } else {
            Err(BackupError::JobNotFound(job_id))
        }
    }

    /// Mark a job as completed and update next run time
    pub async fn mark_job_completed(&self, job_id: BackupJobId, success: bool) -> BackupResult<()> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(&job_id) {
            let now = Utc::now();
            job.last_run = Some(now);
            job.run_count += 1;
            job.is_running = false;

            // Calculate next run
            job.next_run = if job.schedule.enabled {
                if !success && job.schedule.retry_on_failure {
                    // Schedule retry
                    Some(now + Duration::seconds(job.schedule.retry_delay_seconds as i64))
                } else {
                    job.schedule.cron.next_execution(now)
                }
            } else {
                None
            };

            // Check max runs
            if let Some(max_runs) = job.schedule.cron.max_runs {
                if job.run_count >= max_runs {
                    job.schedule.enabled = false;
                    job.next_run = None;
                    info!("Job {} reached max runs and was disabled", job_id.0);
                }
            }

            info!(
                "Job {} completed with success={}. Next run: {:?}",
                job_id.0, success, job.next_run
            );

            self.event_tx
                .send(SchedulerEvent::JobCompleted(job_id, success))
                .await
                .map_err(|e| BackupError::Schedule(e.to_string()))?;

            Ok(())
        } else {
            Err(BackupError::JobNotFound(job_id))
        }
    }

    /// Start the scheduler loop
    pub async fn start(&self) -> BackupResult<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(BackupError::AlreadyRunning);
        }
        *is_running = true;
        drop(is_running);

        info!("Backup scheduler started");

        let jobs = self.jobs.clone();
        let event_tx = self.event_tx.clone();
        let shutdown_rx = self.shutdown_rx.clone();
        let is_running_flag = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(time::Duration::from_secs(1));
            let mut shutdown = shutdown_rx.lock().await;

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let now = Utc::now();
                        let jobs_read = jobs.read().await;

                        for (job_id, job) in jobs_read.iter() {
                            if !job.schedule.enabled || job.is_running {
                                continue;
                            }

                            if let Some(next_run) = job.next_run {
                                if now >= next_run {
                                    info!("Triggering scheduled job {}", job_id.0);

                                    if let Err(e) = event_tx
                                        .send(SchedulerEvent::JobTriggered(*job_id, JobTrigger::Scheduled))
                                        .await
                                    {
                                        error!("Failed to send job trigger event: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    _ = shutdown.recv() => {
                        info!("Scheduler shutting down");
                        let mut running = is_running_flag.write().await;
                        *running = false;
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&self) -> BackupResult<()> {
        let tx = self.shutdown_tx.lock().await;
        tx.send(())
            .await
            .map_err(|e| BackupError::Schedule(e.to_string()))?;
        Ok(())
    }

    /// Get event receiver
    pub async fn get_event_receiver(&self) -> Arc<Mutex<mpsc::Receiver<SchedulerEvent>>> {
        self.event_rx.clone()
    }

    /// Get next execution time for a job
    pub async fn get_next_execution(&self, job_id: BackupJobId) -> Option<DateTime<Utc>> {
        let jobs = self.jobs.read().await;
        jobs.get(&job_id).and_then(|job| job.next_run)
    }

    /// Check if scheduler is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Handle missed executions based on policy
    pub async fn handle_missed_executions(
        &self,
        job_id: BackupJobId,
    ) -> BackupResult<Vec<DateTime<Utc>>> {
        let jobs = self.jobs.read().await;

        if let Some(job) = jobs.get(&job_id) {
            let policy = job.schedule.missed_execution_policy;
            let now = Utc::now();
            let last_run = job.last_run.unwrap_or_else(|| now - Duration::days(30));

            let mut missed = Vec::new();
            let mut next = job.schedule.cron.next_execution(last_run);

            while let Some(time) = next {
                if time >= now {
                    break;
                }
                missed.push(time);
                next = job.schedule.cron.next_execution(time);
            }

            drop(jobs);

            match policy {
                MissedExecutionPolicy::Skip => Ok(vec![]),
                MissedExecutionPolicy::RunImmediately => {
                    if !missed.is_empty() {
                        self.event_tx
                            .send(SchedulerEvent::JobTriggered(
                                job_id,
                                JobTrigger::MissedExecution,
                            ))
                            .await
                            .map_err(|e| BackupError::Schedule(e.to_string()))?;
                    }
                    Ok(vec![])
                }
                MissedExecutionPolicy::RunLastOnly => {
                    if let Some(last) = missed.last() {
                        Ok(vec![*last])
                    } else {
                        Ok(vec![])
                    }
                }
                MissedExecutionPolicy::RunAll => Ok(missed),
            }
        } else {
            Err(BackupError::JobNotFound(job_id))
        }
    }
}

impl Default for BackupScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Preset schedules for common patterns
pub mod presets {
    use super::CronSchedule;

    /// Daily at midnight
    pub fn daily_midnight() -> CronSchedule {
        CronSchedule::new("0 0 * * *")
    }

    /// Daily at 2 AM
    pub fn daily_2am() -> CronSchedule {
        CronSchedule::new("0 2 * * *")
    }

    /// Every hour
    pub fn hourly() -> CronSchedule {
        CronSchedule::new("0 * * * *")
    }

    /// Every 6 hours
    pub fn every_6_hours() -> CronSchedule {
        CronSchedule::new("0 */6 * * *")
    }

    /// Weekly on Sunday at midnight
    pub fn weekly() -> CronSchedule {
        CronSchedule::new("0 0 * * 0")
    }

    /// Monthly on the 1st at midnight
    pub fn monthly() -> CronSchedule {
        CronSchedule::new("0 0 1 * *")
    }

    /// Every 15 minutes
    pub fn every_15_minutes() -> CronSchedule {
        CronSchedule::new("*/15 * * * *")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_cron_schedule_next_execution() {
        let schedule = CronSchedule::new("0 2 * * *"); // Daily at 2 AM

        // Test from midnight
        let midnight = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();
        let next = schedule.next_execution(midnight).unwrap();

        assert_eq!(next.hour(), 2);
        assert_eq!(next.minute(), 0);
    }

    #[test]
    fn test_cron_schedule_description() {
        let schedule = CronSchedule::new("0 2 * * *");
        assert_eq!(schedule.description(), "Daily at 2:00");

        let schedule = CronSchedule::new("0 0 * * *");
        assert_eq!(schedule.description(), "Daily at midnight");
    }

    #[test]
    fn test_presets() {
        let daily = presets::daily_midnight();
        assert_eq!(daily.expression, "0 0 * * *");

        let hourly = presets::hourly();
        assert_eq!(hourly.expression, "0 * * * *");

        let weekly = presets::weekly();
        assert_eq!(weekly.expression, "0 0 * * 0");
    }

    #[tokio::test]
    async fn test_scheduler_add_job() {
        let scheduler = BackupScheduler::new();
        let job_id = BackupJobId::new();

        let config = BackupConfig::default();
        let schedule = ScheduleConfig::default();

        scheduler.add_job(job_id, config, schedule).await.unwrap();

        let job = scheduler.get_job(job_id).await.unwrap();
        assert!(job.schedule.enabled);
    }
}
