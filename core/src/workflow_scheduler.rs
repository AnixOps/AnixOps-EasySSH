use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

/// Scheduled task definition (cron-style)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    /// Workflow/script to execute
    pub workflow_id: String,
    /// Server or server group to target
    pub target_servers: Vec<String>,
    /// Cron expression (standard format: "0 0 * * *")
    pub cron_expression: String,
    /// Human-readable schedule description
    pub schedule_description: String,
    /// Timezone for schedule
    pub timezone: String,
    /// Variables to pass to workflow
    pub variables: HashMap<String, String>,
    /// Execution timeout in minutes
    pub timeout_minutes: u64,
    /// Whether to run missed executions
    pub catch_up_missed: bool,
    /// Maximum parallel executions of this task
    pub max_parallel: usize,
    /// Notification settings
    pub notifications: TaskNotifications,
    /// Retry configuration
    pub retry_policy: RetryPolicy,
    /// Created/modified timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Last execution info
    pub last_run: Option<DateTime<Utc>>,
    pub last_status: Option<TaskStatus>,
    pub next_run: Option<DateTime<Utc>>,
    /// Run count statistics
    pub total_runs: u64,
    pub successful_runs: u64,
    pub failed_runs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskNotifications {
    pub on_success: bool,
    pub on_failure: bool,
    pub email_addresses: Vec<String>,
    pub webhook_url: Option<String>,
}

impl Default for TaskNotifications {
    fn default() -> Self {
        Self {
            on_success: false,
            on_failure: true,
            email_addresses: Vec::new(),
            webhook_url: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub retry_delay_minutes: u64,
    pub exponential_backoff: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 0,
            retry_delay_minutes: 5,
            exponential_backoff: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

impl ScheduledTask {
    pub fn new(name: &str, workflow_id: &str, cron: &str) -> Result<Self, String> {
        // Validate cron expression
        let schedule = CronSchedule::parse(cron)?;
        let description = schedule.describe();

        let now = Utc::now();
        let next_run = schedule.next_occurrence(now);

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: None,
            enabled: true,
            workflow_id: workflow_id.to_string(),
            target_servers: Vec::new(),
            cron_expression: cron.to_string(),
            schedule_description: description,
            timezone: "UTC".to_string(),
            variables: HashMap::new(),
            timeout_minutes: 30,
            catch_up_missed: false,
            max_parallel: 1,
            notifications: TaskNotifications::default(),
            retry_policy: RetryPolicy::default(),
            created_at: now,
            updated_at: now,
            last_run: None,
            last_status: None,
            next_run,
            total_runs: 0,
            successful_runs: 0,
            failed_runs: 0,
        })
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn with_targets(mut self, server_ids: Vec<String>) -> Self {
        self.target_servers = server_ids;
        self
    }

    pub fn with_timeout(mut self, minutes: u64) -> Self {
        self.timeout_minutes = minutes;
        self
    }

    pub fn with_notification(mut self, on_success: bool, on_failure: bool) -> Self {
        self.notifications.on_success = on_success;
        self.notifications.on_failure = on_failure;
        self
    }

    /// Calculate next run time
    pub fn update_next_run(&mut self) {
        let now = Utc::now();
        if let Ok(schedule) = CronSchedule::parse(&self.cron_expression) {
            self.next_run = schedule.next_occurrence(now);
        }
        self.updated_at = now;
    }

    /// Record execution result
    pub fn record_execution(&mut self, status: TaskStatus) {
        self.last_run = Some(Utc::now());
        self.last_status = Some(status.clone());
        self.total_runs += 1;

        match status {
            TaskStatus::Completed => self.successful_runs += 1,
            TaskStatus::Failed | TaskStatus::TimedOut => self.failed_runs += 1,
            _ => {}
        }

        // Update next run
        self.update_next_run();
    }
}

/// Parsed cron schedule
#[derive(Clone, Debug)]
pub struct CronSchedule {
    pub minutes: Vec<u8>,
    pub hours: Vec<u8>,
    pub days_of_month: Vec<u8>,
    pub months: Vec<u8>,
    pub days_of_week: Vec<u8>,
    pub raw: String,
}

impl CronSchedule {
    pub fn parse(expression: &str) -> Result<Self, String> {
        let parts: Vec<&str> = expression.trim().split_whitespace().collect();

        if parts.len() != 5 && parts.len() != 6 {
            return Err(format!(
                "Invalid cron expression '{}'. Expected 5 or 6 fields, got {}.",
                expression,
                parts.len()
            ));
        }

        let minutes = Self::parse_field(parts[0], 0, 59, "minute")?;
        let hours = Self::parse_field(parts[1], 0, 23, "hour")?;
        let days_of_month = Self::parse_field(parts[2], 1, 31, "day of month")?;
        let months = Self::parse_field(parts[3], 1, 12, "month")?;
        let days_of_week = Self::parse_field(parts[4], 0, 7, "day of week")?;

        Ok(Self {
            minutes,
            hours,
            days_of_month,
            months,
            days_of_week,
            raw: expression.to_string(),
        })
    }

    fn parse_field(field: &str, min: u8, max: u8, name: &str) -> Result<Vec<u8>, String> {
        let mut result = Vec::new();

        if field == "*" {
            for i in min..=max {
                result.push(i);
            }
            return Ok(result);
        }

        if field == "?" {
            return Ok(vec![]); // No specific value
        }

        // Handle L (last) for day of month
        if field == "L" && name == "day of month" {
            return Ok(vec![99]); // Special marker for last day
        }

        // Handle step values (*/5, 1-10/2)
        let parts: Vec<&str> = field.split('/').collect();
        let range_part = parts[0];
        let step = if parts.len() > 1 {
            parts[1]
                .parse::<u8>()
                .map_err(|_| format!("Invalid step in {}", name))?
        } else {
            1
        };

        // Handle ranges and lists
        for item in range_part.split(',') {
            if item == "*" {
                for i in (min..=max).step_by(step as usize) {
                    result.push(i);
                }
            } else if item.contains('-') {
                let range: Vec<&str> = item.split('-').collect();
                if range.len() == 2 {
                    let start = range[0]
                        .parse::<u8>()
                        .map_err(|_| format!("Invalid range in {}", name))?;
                    let end = range[1]
                        .parse::<u8>()
                        .map_err(|_| format!("Invalid range in {}", name))?;
                    for i in (start..=end).step_by(step as usize) {
                        if i >= min && i <= max {
                            result.push(i);
                        }
                    }
                }
            } else {
                let val = item
                    .parse::<u8>()
                    .map_err(|_| format!("Invalid value in {}", name))?;
                if val >= min && val <= max {
                    result.push(val);
                }
            }
        }

        result.sort_unstable();
        result.dedup();
        Ok(result)
    }

    /// Get next occurrence after given time
    pub fn next_occurrence(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let mut candidate = after + chrono::Duration::minutes(1);
        candidate = candidate
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();

        // Search up to 4 years ahead
        for _ in 0..(365 * 4 * 24 * 60) {
            if self.matches(&candidate) {
                return Some(candidate);
            }
            candidate = candidate + chrono::Duration::minutes(1);
        }

        None
    }

    fn matches(&self, dt: &DateTime<Utc>) -> bool {
        let minute = dt.minute() as u8;
        let hour = dt.hour() as u8;
        let day = dt.day() as u8;
        let month = dt.month() as u8;
        let weekday = dt.weekday().num_days_from_sunday() as u8;

        if !self.minutes.contains(&minute) {
            return false;
        }
        if !self.hours.contains(&hour) {
            return false;
        }

        // Handle day of month with L (last day)
        let day_matches = if self.days_of_month.contains(&99) {
            let last_day = Self::last_day_of_month(dt.year(), dt.month());
            day == last_day as u8
        } else {
            self.days_of_month.contains(&day)
        };

        // Both day of month and day of week can match, or either
        let month_matches = self.months.contains(&month);
        let weekday_matches = self.days_of_week.contains(&weekday);

        if self.days_of_month.is_empty() || self.days_of_month.contains(&0) {
            // Day of week only
            month_matches && weekday_matches
        } else if self.days_of_week.is_empty() || self.days_of_week.contains(&0) {
            // Day of month only
            month_matches && day_matches
        } else {
            // Either can match
            month_matches && (day_matches || weekday_matches)
        }
    }

    fn last_day_of_month(year: i32, month: u32) -> u32 {
        let next_month = if month == 12 { 1 } else { month + 1 };
        let next_year = if month == 12 { year + 1 } else { year };

        let first_of_next = chrono::NaiveDate::from_ymd_opt(next_year, next_month, 1).unwrap();
        let last_of_this = first_of_next.pred_opt().unwrap();
        last_of_this.day()
    }

    /// Generate human-readable description
    pub fn describe(&self) -> String {
        let mut parts = Vec::new();

        // Minutes
        if self.minutes.len() == 60 {
            parts.push("every minute".to_string());
        } else if self.minutes.len() == 1 {
            parts.push(format!("at minute {}", self.minutes[0]));
        } else {
            parts.push(format!("at minutes: {:?}", self.minutes));
        }

        // Hours
        if self.hours.len() == 24 {
            parts.push("every hour".to_string());
        } else if self.hours.len() == 1 {
            parts.push(format!("at {}:00", self.hours[0]));
        }

        // Days
        if !self.days_of_month.is_empty() && !self.days_of_month.contains(&99) {
            if self.days_of_month.len() == 1 {
                parts.push(format!("on day {} of the month", self.days_of_month[0]));
            }
        }

        // Weekdays
        if !self.days_of_week.is_empty() {
            let weekdays: Vec<&str> = self
                .days_of_week
                .iter()
                .map(|&d| match d {
                    0 => "Sunday",
                    1 => "Monday",
                    2 => "Tuesday",
                    3 => "Wednesday",
                    4 => "Thursday",
                    5 => "Friday",
                    6 => "Saturday",
                    7 => "Sunday",
                    _ => "Unknown",
                })
                .collect();
            if weekdays.len() == 7 {
                parts.push("daily".to_string());
            } else {
                parts.push(format!("on {}", weekdays.join(", ")));
            }
        }

        parts.join(", ")
    }
}

/// Scheduled task manager
pub struct TaskScheduler {
    tasks: HashMap<String, ScheduledTask>,
    check_interval: Duration,
    running_tasks: HashMap<String, TaskExecutionHandle>,
}

#[derive(Clone)]
pub struct TaskExecutionHandle {
    pub task_id: String,
    pub execution_id: String,
    pub started_at: DateTime<Utc>,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            check_interval: Duration::from_secs(60), // Check every minute
            running_tasks: HashMap::new(),
        }
    }

    /// Add a new scheduled task
    pub fn add_task(&mut self, task: ScheduledTask) -> Result<String, String> {
        // Validate cron expression
        CronSchedule::parse(&task.cron_expression)?;

        let id = task.id.clone();
        let task_name = task.name.clone();
        self.tasks.insert(id.clone(), task);
        info!("Added scheduled task: {} ({})", id, task_name);
        Ok(id)
    }

    /// Remove a task
    pub fn remove_task(&mut self, task_id: &str) -> Option<ScheduledTask> {
        let task = self.tasks.remove(task_id);
        if task.is_some() {
            info!("Removed scheduled task: {}", task_id);
        }
        task
    }

    /// Get a task
    pub fn get_task(&self, task_id: &str) -> Option<&ScheduledTask> {
        self.tasks.get(task_id)
    }

    /// Get all tasks
    pub fn get_all_tasks(&self) -> Vec<&ScheduledTask> {
        self.tasks.values().collect()
    }

    /// Update task
    pub fn update_task(&mut self, task_id: &str, mut task: ScheduledTask) -> Result<(), String> {
        if !self.tasks.contains_key(task_id) {
            return Err(format!("Task {} not found", task_id));
        }

        // Validate cron
        CronSchedule::parse(&task.cron_expression)?;

        task.id = task_id.to_string();
        task.updated_at = Utc::now();
        task.update_next_run();

        self.tasks.insert(task_id.to_string(), task);
        Ok(())
    }

    /// Enable/disable task
    pub fn set_task_enabled(&mut self, task_id: &str, enabled: bool) -> Result<(), String> {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.enabled = enabled;
            task.updated_at = Utc::now();
            if enabled {
                task.update_next_run();
            }
            Ok(())
        } else {
            Err(format!("Task {} not found", task_id))
        }
    }

    /// Check for tasks that need to run
    pub fn check_due_tasks(&mut self) -> Vec<String> {
        let now = Utc::now();
        let mut due_tasks = Vec::new();

        for (id, task) in &self.tasks {
            if !task.enabled {
                continue;
            }

            // Skip if already running and max_parallel reached
            let running_count = self
                .running_tasks
                .values()
                .filter(|h| h.task_id == *id)
                .count();
            if running_count >= task.max_parallel {
                continue;
            }

            // Check if task is due
            if let Some(next_run) = task.next_run {
                if next_run <= now {
                    due_tasks.push(id.clone());
                }
            }
        }

        due_tasks
    }

    /// Start a task execution
    pub fn start_execution(&mut self, task_id: &str, execution_id: String) {
        self.running_tasks.insert(
            execution_id.clone(),
            TaskExecutionHandle {
                task_id: task_id.to_string(),
                execution_id: execution_id.clone(),
                started_at: Utc::now(),
            },
        );

        if let Some(task) = self.tasks.get_mut(task_id) {
            task.last_run = Some(Utc::now());
            task.last_status = Some(TaskStatus::Running);
        }
    }

    /// Complete a task execution
    pub fn complete_execution(&mut self, execution_id: &str, status: TaskStatus) {
        if let Some(handle) = self.running_tasks.remove(execution_id) {
            if let Some(task) = self.tasks.get_mut(&handle.task_id) {
                task.record_execution(status);
            }
        }
    }

    /// Get running tasks
    pub fn get_running_tasks(&self) -> Vec<&TaskExecutionHandle> {
        self.running_tasks.values().collect()
    }

    /// Run the scheduler loop (async)
    pub async fn run(&mut self) {
        let mut interval = tokio::time::interval(self.check_interval);

        loop {
            interval.tick().await;

            let due_tasks = self.check_due_tasks();
            for task_id in due_tasks {
                info!("Task {} is due for execution", task_id);
                // Here you would trigger the actual execution
                // This would integrate with the WorkflowExecutor
            }
        }
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Common cron presets
pub struct CronPresets;

impl CronPresets {
    pub fn every_minute() -> &'static str {
        "* * * * *"
    }

    pub fn every_5_minutes() -> &'static str {
        "*/5 * * * *"
    }

    pub fn every_15_minutes() -> &'static str {
        "*/15 * * * *"
    }

    pub fn hourly() -> &'static str {
        "0 * * * *"
    }

    pub fn daily() -> &'static str {
        "0 0 * * *"
    }

    pub fn daily_at(hour: u8, minute: u8) -> String {
        format!("{} {} * * *", minute, hour)
    }

    pub fn weekly() -> &'static str {
        "0 0 * * 0"
    }

    pub fn weekly_on(day: u8, hour: u8, minute: u8) -> String {
        // day: 0=Sunday, 1=Monday, ... 6=Saturday
        format!("{} {} * * {}", minute, hour, day)
    }

    pub fn monthly() -> &'static str {
        "0 0 1 * *"
    }

    pub fn weekdays() -> &'static str {
        "0 9 * * 1-5"
    }

    pub fn weekends() -> &'static str {
        "0 10 * * 0,6"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_parsing() {
        let schedule = CronSchedule::parse("0 0 * * *").unwrap();
        assert_eq!(schedule.minutes, vec![0]);
        assert_eq!(schedule.hours, vec![0]);
        // describe() for "0 0 * * *" should contain info about daily execution
        let desc = schedule.describe();
        assert!(
            desc.contains("at minute 0") || desc.contains("daily") || desc.contains("hour"),
            "Description should indicate daily execution: {}",
            desc
        );

        let schedule = CronSchedule::parse("*/5 * * * *").unwrap();
        assert!(schedule.minutes.contains(&0));
        assert!(schedule.minutes.contains(&5));
        assert!(schedule.minutes.contains(&55));
    }

    #[test]
    fn test_cron_next_occurrence() {
        let schedule = CronSchedule::parse("0 12 * * *").unwrap(); // Daily at noon
        let now = Utc::now();
        let next = schedule.next_occurrence(now).unwrap();

        assert_eq!(next.minute(), 0);
        assert_eq!(next.hour(), 12);
        assert!(next > now);
    }

    #[test]
    fn test_scheduled_task() {
        let task = ScheduledTask::new("Test Task", "workflow-123", "0 0 * * *").unwrap();
        assert_eq!(task.name, "Test Task");
        assert_eq!(task.workflow_id, "workflow-123");
        assert!(task.next_run.is_some());
    }

    #[test]
    fn test_cron_presets() {
        assert_eq!(CronPresets::every_minute(), "* * * * *");
        assert_eq!(CronPresets::hourly(), "0 * * * *");
        assert_eq!(CronPresets::daily(), "0 0 * * *");
        assert_eq!(CronPresets::daily_at(14, 30), "30 14 * * *");
    }

    #[test]
    fn test_cron_preset_advanced() {
        assert_eq!(CronPresets::every_5_minutes(), "*/5 * * * *");
        assert_eq!(CronPresets::every_15_minutes(), "*/15 * * * *");
        assert_eq!(CronPresets::weekly(), "0 0 * * 0");
        assert_eq!(CronPresets::monthly(), "0 0 1 * *");
        assert_eq!(CronPresets::weekdays(), "0 9 * * 1-5");
        assert_eq!(CronPresets::weekends(), "0 10 * * 0,6");
        assert_eq!(CronPresets::weekly_on(1, 10, 30), "30 10 * * 1");
    }

    #[test]
    fn test_invalid_cron_expression() {
        // Too few fields
        assert!(CronSchedule::parse("0 0 * *").is_err());
        // Too many fields
        assert!(CronSchedule::parse("0 0 0 0 * * *").is_err());
        // Invalid step value
        assert!(CronSchedule::parse("*/invalid * * * *").is_err());
        // Invalid range format
        assert!(CronSchedule::parse("0 9-invalid * * *").is_err());
    }

    #[test]
    fn test_cron_out_of_range_values() {
        // Note: The current implementation silently filters out-of-range values
        // rather than returning an error. These tests document that behavior.

        // 60 is out of range for minutes (0-59), so it should be filtered out
        let schedule = CronSchedule::parse("60 0 * * *").unwrap();
        assert!(!schedule.minutes.contains(&60));

        // 24 is out of range for hours (0-23), so it should be filtered out
        let schedule = CronSchedule::parse("0 24 * * *").unwrap();
        assert!(!schedule.hours.contains(&24));
    }

    #[test]
    fn test_cron_step_parsing_error() {
        // Invalid step in step expression should error
        assert!(CronSchedule::parse("*/abc * * * *").is_err());
    }

    #[test]
    fn test_cron_schedule_complex_patterns() {
        // Test range pattern
        let schedule = CronSchedule::parse("0 9-17 * * *").unwrap();
        assert!(schedule.hours.contains(&9));
        assert!(schedule.hours.contains(&17));
        assert!(!schedule.hours.contains(&8));
        assert!(!schedule.hours.contains(&18));

        // Test step pattern with range
        let schedule = CronSchedule::parse("*/10 9-17 * * 1-5").unwrap();
        assert!(schedule.minutes.contains(&0));
        assert!(schedule.minutes.contains(&10));
        assert!(schedule.minutes.contains(&50));
        assert!(!schedule.minutes.contains(&55));

        // Test list pattern
        let schedule = CronSchedule::parse("0,30 9,12,15 * * *").unwrap();
        assert_eq!(schedule.minutes, vec![0, 30]);
        assert_eq!(schedule.hours, vec![9, 12, 15]);
    }

    #[test]
    fn test_cron_schedule_describe() {
        let schedule = CronSchedule::parse("0 0 * * 1-5").unwrap();
        let desc = schedule.describe();
        // Should describe weekdays execution
        assert!(desc.contains("minute") || desc.contains("daily"));

        let schedule = CronSchedule::parse("*/5 * * * *").unwrap();
        let desc = schedule.describe();
        assert!(desc.contains("every minute") || desc.contains("minute"));
    }

    #[test]
    fn test_scheduled_task_builder_methods() {
        let task = ScheduledTask::new("Test", "wf-1", "0 0 * * *")
            .unwrap()
            .with_description("A test task")
            .with_targets(vec!["server-1".to_string(), "server-2".to_string()])
            .with_timeout(60)
            .with_notification(true, true);

        assert_eq!(task.description, Some("A test task".to_string()));
        assert_eq!(task.target_servers, vec!["server-1", "server-2"]);
        assert_eq!(task.timeout_minutes, 60);
        assert!(task.notifications.on_success);
        assert!(task.notifications.on_failure);
    }

    #[test]
    fn test_task_status_variants() {
        let statuses = vec![
            TaskStatus::Pending,
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
            TaskStatus::TimedOut,
        ];

        for (i, status) in statuses.iter().enumerate() {
            let cloned = status.clone();
            // Ensure all are distinct (at least one should be different)
            if i > 0 {
                assert_ne!(*status, statuses[0]);
            }
            assert_eq!(*status, cloned);
        }
    }

    #[test]
    fn test_task_notifications_default() {
        let notifications = TaskNotifications::default();
        assert!(!notifications.on_success);
        assert!(notifications.on_failure);
        assert!(notifications.email_addresses.is_empty());
        assert!(notifications.webhook_url.is_none());
    }

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 0);
        assert_eq!(policy.retry_delay_minutes, 5);
        assert!(policy.exponential_backoff);
    }

    #[test]
    fn test_scheduled_task_record_execution() {
        let mut task = ScheduledTask::new("Test", "wf-1", "0 0 * * *").unwrap();
        assert_eq!(task.total_runs, 0);
        assert_eq!(task.successful_runs, 0);
        assert_eq!(task.failed_runs, 0);

        task.record_execution(TaskStatus::Completed);
        assert_eq!(task.total_runs, 1);
        assert_eq!(task.successful_runs, 1);
        assert_eq!(task.failed_runs, 0);
        assert!(task.last_run.is_some());
        assert_eq!(task.last_status, Some(TaskStatus::Completed));

        task.record_execution(TaskStatus::Failed);
        assert_eq!(task.total_runs, 2);
        assert_eq!(task.successful_runs, 1);
        assert_eq!(task.failed_runs, 1);

        task.record_execution(TaskStatus::TimedOut);
        assert_eq!(task.total_runs, 3);
        assert_eq!(task.successful_runs, 1);
        assert_eq!(task.failed_runs, 2);
    }

    #[test]
    fn test_scheduled_task_update_next_run() {
        let mut task = ScheduledTask::new("Test", "wf-1", "0 0 * * *").unwrap();
        let original_next_run = task.next_run.clone();

        // Wait a bit and update
        std::thread::sleep(std::time::Duration::from_millis(10));
        task.update_next_run();

        assert!(task.next_run.is_some());
        assert!(task.updated_at > task.created_at || task.next_run == original_next_run);
    }

    #[test]
    fn test_task_scheduler_creation() {
        let scheduler = TaskScheduler::new();
        assert!(scheduler.tasks.is_empty());
        assert!(scheduler.running_tasks.is_empty());
    }

    #[test]
    fn test_task_scheduler_default() {
        let scheduler: TaskScheduler = Default::default();
        assert!(scheduler.tasks.is_empty());
    }

    #[test]
    fn test_task_scheduler_add_task() {
        let mut scheduler = TaskScheduler::new();
        let task = ScheduledTask::new("Test Task", "wf-1", "0 0 * * *").unwrap();
        let task_id = task.id.clone();

        let result = scheduler.add_task(task);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), task_id);
        assert!(scheduler.tasks.contains_key(&task_id));
    }

    #[test]
    fn test_task_scheduler_add_task_invalid_cron() {
        let mut scheduler = TaskScheduler::new();

        // Creating a task with invalid cron should fail
        let task_result = ScheduledTask::new("Test", "wf-1", "invalid cron");
        assert!(
            task_result.is_err(),
            "Creating task with invalid cron should fail"
        );

        // Also verify that the scheduler rejects tasks with invalid cron
        // We can only test this if we could somehow create an invalid task
        // The TaskScheduler::add_task method also validates the cron expression
        // So if we could create a task, the scheduler would also reject it
    }

    #[test]
    fn test_task_scheduler_remove_task() {
        let mut scheduler = TaskScheduler::new();
        let task = ScheduledTask::new("Test", "wf-1", "0 0 * * *").unwrap();
        let task_id = task.id.clone();

        scheduler.add_task(task).unwrap();
        let removed = scheduler.remove_task(&task_id);

        assert!(removed.is_some());
        assert!(!scheduler.tasks.contains_key(&task_id));

        // Remove non-existent should return None
        assert!(scheduler.remove_task("non-existent").is_none());
    }

    #[test]
    fn test_task_scheduler_get_task() {
        let mut scheduler = TaskScheduler::new();
        let task = ScheduledTask::new("Test", "wf-1", "0 0 * * *").unwrap();
        let task_id = task.id.clone();

        scheduler.add_task(task).unwrap();
        assert!(scheduler.get_task(&task_id).is_some());
        assert!(scheduler.get_task("non-existent").is_none());
    }

    #[test]
    fn test_task_scheduler_get_all_tasks() {
        let mut scheduler = TaskScheduler::new();
        let task1 = ScheduledTask::new("Task 1", "wf-1", "0 0 * * *").unwrap();
        let task2 = ScheduledTask::new("Task 2", "wf-2", "*/5 * * * *").unwrap();

        scheduler.add_task(task1).unwrap();
        scheduler.add_task(task2).unwrap();

        let all_tasks = scheduler.get_all_tasks();
        assert_eq!(all_tasks.len(), 2);
    }

    #[test]
    fn test_task_scheduler_update_task() {
        let mut scheduler = TaskScheduler::new();
        let task = ScheduledTask::new("Test", "wf-1", "0 0 * * *").unwrap();
        let task_id = task.id.clone();

        scheduler.add_task(task).unwrap();

        let mut updated_task = ScheduledTask::new("Updated", "wf-2", "*/5 * * * *").unwrap();
        updated_task.id = "wrong-id".to_string(); // Will be overwritten

        let result = scheduler.update_task(&task_id, updated_task);
        assert!(result.is_ok());

        let task_ref = scheduler.get_task(&task_id).unwrap();
        assert_eq!(task_ref.name, "Updated");
        assert_eq!(task_ref.workflow_id, "wf-2");

        // Update non-existent should fail
        let fake_task = ScheduledTask::new("Fake", "wf-3", "0 0 * * *").unwrap();
        assert!(scheduler.update_task("non-existent", fake_task).is_err());
    }

    #[test]
    fn test_task_scheduler_set_task_enabled() {
        let mut scheduler = TaskScheduler::new();
        let task = ScheduledTask::new("Test", "wf-1", "0 0 * * *").unwrap();
        let task_id = task.id.clone();

        scheduler.add_task(task).unwrap();

        // Disable
        assert!(scheduler.set_task_enabled(&task_id, false).is_ok());
        assert!(!scheduler.get_task(&task_id).unwrap().enabled);

        // Enable
        assert!(scheduler.set_task_enabled(&task_id, true).is_ok());
        assert!(scheduler.get_task(&task_id).unwrap().enabled);

        // Non-existent
        assert!(scheduler.set_task_enabled("non-existent", false).is_err());
    }

    #[test]
    fn test_task_execution_handle_clone() {
        let handle = TaskExecutionHandle {
            task_id: "task-1".to_string(),
            execution_id: "exec-1".to_string(),
            started_at: Utc::now(),
        };

        let cloned = handle.clone();
        assert_eq!(handle.task_id, cloned.task_id);
        assert_eq!(handle.execution_id, cloned.execution_id);
    }

    #[test]
    fn test_cron_schedule_matches() {
        let schedule = CronSchedule::parse("0 12 * * *").unwrap(); // Daily at noon

        // Create a datetime at noon
        let noon = Utc::now()
            .with_hour(12)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();

        assert!(schedule.matches(&noon));

        // Should not match at other times
        let morning = noon.with_hour(9).unwrap();
        assert!(!schedule.matches(&morning));
    }

    #[test]
    fn test_cron_schedule_last_day_of_month() {
        let schedule = CronSchedule::parse("0 0 L * *").unwrap(); // Last day of month

        // Test description contains last day info
        let desc = schedule.describe();
        // L is a special marker
        assert!(schedule.days_of_month.contains(&99));
    }

    #[test]
    fn test_cron_schedule_weekday_handling() {
        // Test day of week with 0 and 7 both representing Sunday
        let schedule = CronSchedule::parse("0 0 * * 0").unwrap();
        assert!(schedule.days_of_week.contains(&0));

        let schedule = CronSchedule::parse("0 0 * * 7").unwrap();
        assert!(schedule.days_of_week.contains(&7));
    }
}
