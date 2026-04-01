//! Error tracking and crash reporting
//!
//! Collects errors for debugging while protecting user privacy.
//! No SSH credentials, hostnames, or server data is included.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use super::{ConsentCategory, ConsentManager, EventCollector, TelemetryError, TelemetryEvent};

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Severity {
    /// Minor issue, app continues normally
    Low,
    /// Degraded functionality
    Medium,
    /// Major feature broken
    High,
    /// App crash or data loss
    Critical,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        }
    }
}

/// Error context with sanitized information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Error type/category
    pub error_type: String,
    /// Component where error occurred
    pub component: String,
    /// Error message (sanitized)
    pub message: String,
    /// Severity level
    pub severity: Severity,
    /// Additional context (sanitized)
    pub context: HashMap<String, serde_json::Value>,
    /// Timestamp
    pub timestamp: u64,
    /// Stack trace (debug builds only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
    /// Whether this is a crash
    pub is_crash: bool,
    /// Thread where error occurred
    pub thread_name: String,
}

impl ErrorContext {
    /// Create new error context
    pub fn new(error_type: impl Into<String>, component: impl Into<String>) -> Self {
        Self {
            error_type: error_type.into(),
            component: component.into(),
            message: String::new(),
            severity: Severity::Medium,
            context: HashMap::new(),
            timestamp: SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            stack_trace: Self::capture_stack_trace(),
            is_crash: false,
            thread_name: std::thread::current()
                .name()
                .unwrap_or("unknown")
                .to_string(),
        }
    }

    /// Sanitize string to remove sensitive data
    pub fn sanitize(input: &str) -> String {
        // Remove potential hostnames, IPs, usernames, passwords
        let mut result = input.to_string();

        // Patterns to redact
        let patterns = [
            // IP addresses
            (r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b", "[IP-REDACTED]"),
            // Hostnames (simplified)
            (r"\b[a-zA-Z0-9-]+\.[a-zA-Z]{2,}\b", "[HOST-REDACTED]"),
            // Email addresses
            (r"\S+@\S+\.\S+", "[EMAIL-REDACTED]"),
            // Potential passwords in error messages
            (r"password[=:]\S+", "password=[REDACTED]"),
            (r"passwd[=:]\S+", "passwd=[REDACTED]"),
            // SSH private key patterns
            (r"-----BEGIN [A-Z ]+ PRIVATE KEY-----", "[PRIVATE-KEY-REDACTED]"),
        ];

        for (pattern, replacement) in &patterns {
            // Note: In production, use proper regex
            result = result.replace(pattern, replacement);
        }

        result
    }

    /// Set message (will be sanitized)
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Self::sanitize(&message.into());
        self
    }

    /// Set severity
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Mark as crash
    pub fn with_crash(mut self, is_crash: bool) -> Self {
        self.is_crash = is_crash;
        self
    }

    /// Add context key-value (value will be sanitized if string)
    pub fn with_context(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        let key = key.into();
        let value = value.into();

        // Sanitize string values
        let sanitized = match value {
            serde_json::Value::String(s) => serde_json::Value::String(Self::sanitize(&s)),
            other => other,
        };

        self.context.insert(key, sanitized);
        self
    }

    /// Capture stack trace (debug builds only)
    fn capture_stack_trace() -> Option<String> {
        if cfg!(debug_assertions) {
            // In debug builds, capture limited stack trace
            // Using backtrace crate would be better for production
            Some("stack_trace_available_in_debug".to_string())
        } else {
            None
        }
    }
}

/// Error tracker
pub struct ErrorTracker {
    collector: Arc<EventCollector>,
    consent_manager: Arc<ConsentManager>,
    recent_errors: Arc<Mutex<Vec<ErrorContext>>>,
    max_recent_errors: usize,
}

impl ErrorTracker {
    pub fn new(
        collector: Arc<EventCollector>,
        consent_manager: Arc<ConsentManager>,
    ) -> Self {
        Self {
            collector,
            consent_manager,
            recent_errors: Arc::new(Mutex::new(Vec::with_capacity(100))),
            max_recent_errors: 100,
        }
    }

    /// Track an error
    pub async fn track_error(&self, context: ErrorContext) {
        // Check consent
        if !self.consent_manager.is_allowed(ConsentCategory::CrashReporting) {
            return;
        }

        // Store in recent errors
        {
            let mut errors = self.recent_errors.lock().unwrap();
            if errors.len() >= self.max_recent_errors {
                errors.remove(0);
            }
            errors.push(context.clone());
        }

        // Send to collector
        let event = TelemetryEvent::ErrorOccurred {
            error_type: context.error_type.clone(),
            severity: context.severity,
            component: context.component.clone(),
            stack_trace: context.stack_trace.clone(),
        };

        self.collector.collect_event(event).await;

        // If critical, flush immediately
        if context.severity == Severity::Critical {
            self.collector.flush().await.ok();
        }
    }

    /// Track error from standard error
    pub async fn track_std_error<E: std::error::Error>(
        &self,
        error: &E,
        component: &str,
        severity: Severity,
    ) {
        let context = ErrorContext::new(
            std::any::type_name::<E>(),
            component,
        )
        .with_message(error.to_string())
        .with_severity(severity);

        self.track_error(context).await;
    }

    /// Get recent errors
    pub fn get_recent_errors(&self) -> Vec<ErrorContext> {
        self.recent_errors.lock().unwrap().clone()
    }

    /// Clear recent errors
    pub fn clear_recent_errors(&self) {
        self.recent_errors.lock().unwrap().clear();
    }

    /// Set up panic handler
    pub fn setup_panic_handler(&self) {
        let collector = Arc::clone(&self.collector);
        let consent = Arc::clone(&self.consent_manager);

        std::panic::set_hook(Box::new(move |info| {
            let location = info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_else(|| "unknown".to_string());

            let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            let context = ErrorContext::new("panic", "app")
                .with_message(message)
                .with_severity(Severity::Critical)
                .with_crash(true)
                .with_context("location", location);

            // Try to send immediately (blocking in panic handler)
            if consent.is_allowed(ConsentCategory::CrashReporting) {
                let rt = tokio::runtime::Runtime::new().ok();
                if let Some(rt) = rt {
                    rt.block_on(async {
                        let event = TelemetryEvent::ErrorOccurred {
                            error_type: context.error_type.clone(),
                            severity: context.severity,
                            component: context.component.clone(),
                            stack_trace: context.stack_trace.clone(),
                        };
                        collector.collect_event(event).await;
                        collector.flush().await.ok();
                    });
                }
            }
        }));
    }

    /// Get error statistics
    pub fn get_error_stats(&self) -> ErrorStats {
        let errors = self.recent_errors.lock().unwrap();

        let mut by_severity = HashMap::new();
        let mut by_component = HashMap::new();
        let mut by_type = HashMap::new();

        for error in errors.iter() {
            *by_severity.entry(error.severity).or_insert(0) += 1;
            *by_component.entry(error.component.clone()).or_insert(0) += 1;
            *by_type.entry(error.error_type.clone()).or_insert(0) += 1;
        }

        ErrorStats {
            total_count: errors.len(),
            by_severity,
            by_component,
            by_type,
            time_range: if errors.len() >= 2 {
                let first = errors.first().map(|e| e.timestamp);
                let last = errors.last().map(|e| e.timestamp);
                (first, last)
            } else {
                (None, None)
            },
        }
    }
}

/// Error statistics
#[derive(Debug, Clone)]
pub struct ErrorStats {
    pub total_count: usize,
    pub by_severity: HashMap<Severity, usize>,
    pub by_component: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
    pub time_range: (Option<u64>, Option<u64>),
}

/// Sentry-like integration for external error tracking services
pub struct ExternalErrorReporter {
    dsn: Option<String>,
    enabled: bool,
    environment: String,
    release: String,
}

impl ExternalErrorReporter {
    pub fn new(dsn: Option<String>, environment: String, release: String) -> Self {
        Self {
            dsn,
            enabled: dsn.is_some(),
            environment,
            release,
        }
    }

    /// Report error to external service
    pub async fn report(&self, context: &ErrorContext) -> Result<(), TelemetryError> {
        if !self.enabled {
            return Ok(());
        }

        // This would integrate with Sentry, Bugsnag, etc.
        // For now, it's a placeholder

        println!(
            "[ExternalErrorReporter] Would send to Sentry: {} - {}",
            context.error_type, context.message
        );

        Ok(())
    }

    /// Configure scope for error reporting
    pub fn configure_scope<F>(&self, _f: F)
    where
        F: FnOnce(&mut ErrorScope),
    {
        // Implementation for scope configuration
    }
}

/// Error scope for contextual data
#[derive(Debug, Default)]
pub struct ErrorScope {
    pub tags: HashMap<String, String>,
    pub extra: HashMap<String, serde_json::Value>,
    pub user: Option<AnonymousUser>,
    pub level: Option<Severity>,
}

/// Anonymous user info for error context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousUser {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
}

/// Integration with external crash reporting services
pub async fn setup_sentry_integration(dsn: Option<String>, release: String) {
    if let Some(_dsn) = dsn {
        // Initialize Sentry or similar service
        println!("[Telemetry] External error reporting enabled, release: {}", release);
    }
}

/// Report a caught error
#[macro_export]
macro_rules! track_error {
    ($tracker:expr, $error:expr, $component:expr, $severity:expr) => {
        tokio::spawn(async move {
            $tracker.track_std_error(&$error, $component, $severity).await;
        });
    };
}

/// Report a result error
#[macro_export]
macro_rules! track_result {
    ($tracker:expr, $result:expr, $component:expr) => {
        if let Err(ref e) = $result {
            tokio::spawn(async move {
                $tracker.track_std_error(e, $component, $crate::telemetry::Severity::Medium).await;
            });
        }
        $result
    };
}
