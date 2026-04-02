//! EasySSH Logging System
//!
//! This module provides a comprehensive logging solution with:
//!
//! - Multiple log levels (Error, Warn, Info, Debug, Trace)
//! - Rotating file logs with size-based rotation
//! - Structured logging with JSON format support
//! - Context tracking (request_id, session_id, etc.)
//! - Optional remote log reporting (Pro feature)
//! - Thread-safe async logging
//!
//! # Example
//!
//! ```rust
//! use easyssh_core::logger::{Logger, LogLevel, LogContext};
//!
//! // Initialize the logger
//! let logger = Logger::new()
//!     .with_level(LogLevel::Debug)
//!     .with_file_output("logs/easyssh.log")
//!     .init()
//!     .expect("Failed to initialize logger");
//!
//! // Log with context
//! let ctx = LogContext::new().with_request_id("req-123");
//! logger.info_with_context("Application started", &ctx);
//! ```

use chrono::{DateTime, Local, Utc};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

/// Log levels in order of severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Critical errors that prevent operation
    Error = 1,
    /// Warning conditions
    Warn = 2,
    /// Informational messages
    Info = 3,
    /// Debug information
    Debug = 4,
    /// Detailed trace information
    Trace = 5,
}

impl LogLevel {
    /// Convert from string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "error" => Some(LogLevel::Error),
            "warn" | "warning" => Some(LogLevel::Warn),
            "info" => Some(LogLevel::Info),
            "debug" => Some(LogLevel::Debug),
            "trace" => Some(LogLevel::Trace),
            _ => None,
        }
    }

    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }

    /// Check if this level should be logged given a minimum level
    pub fn should_log(&self, min_level: LogLevel) -> bool {
        *self <= min_level
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Log context for tracking request/operation metadata
#[derive(Debug, Clone, Default)]
pub struct LogContext {
    /// Unique request identifier
    pub request_id: Option<String>,
    /// Session identifier
    pub session_id: Option<String>,
    /// User identifier
    pub user_id: Option<String>,
    /// Operation name
    pub operation: Option<String>,
    /// Source module/component
    pub module: Option<String>,
    /// Additional custom fields
    pub custom_fields: HashMap<String, String>,
}

impl LogContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add request ID
    pub fn with_request_id<T: Into<String>>(mut self, id: T) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// Add session ID
    pub fn with_session_id<T: Into<String>>(mut self, id: T) -> Self {
        self.session_id = Some(id.into());
        self
    }

    /// Add user ID
    pub fn with_user_id<T: Into<String>>(mut self, id: T) -> Self {
        self.user_id = Some(id.into());
        self
    }

    /// Add operation name
    pub fn with_operation<T: Into<String>>(mut self, op: T) -> Self {
        self.operation = Some(op.into());
        self
    }

    /// Add module name
    pub fn with_module<T: Into<String>>(mut self, module: T) -> Self {
        self.module = Some(module.into());
        self
    }

    /// Add a custom field
    pub fn with_field<T: Into<String>>(mut self, key: T, value: T) -> Self {
        self.custom_fields.insert(key.into(), value.into());
        self
    }

    /// Convert to HashMap for structured logging
    pub fn to_map(&self) -> HashMap<String, String> {
        let mut map = self.custom_fields.clone();
        if let Some(ref id) = self.request_id {
            map.insert("request_id".to_string(), id.clone());
        }
        if let Some(ref id) = self.session_id {
            map.insert("session_id".to_string(), id.clone());
        }
        if let Some(ref id) = self.user_id {
            map.insert("user_id".to_string(), id.clone());
        }
        if let Some(ref op) = self.operation {
            map.insert("operation".to_string(), op.clone());
        }
        if let Some(ref m) = self.module {
            map.insert("module".to_string(), m.clone());
        }
        map
    }
}

/// Log entry structure
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Timestamp (UTC)
    pub timestamp: DateTime<Utc>,
    /// Log level
    pub level: LogLevel,
    /// Module path
    pub module: String,
    /// Log message
    pub message: String,
    /// Optional context
    pub context: Option<LogContext>,
    /// Source file (optional)
    pub file: Option<String>,
    /// Line number (optional)
    pub line: Option<u32>,
}

impl LogEntry {
    /// Format as plain text
    pub fn format_text(&self) -> String {
        let local_time = self.timestamp.with_timezone(&Local);
        let ctx_str = self
            .context
            .as_ref()
            .map(|c| {
                let parts: Vec<String> = c
                    .to_map()
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                if parts.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", parts.join(", "))
                }
            })
            .unwrap_or_default();

        format!(
            "[{}] [{}] {}{} - {}",
            local_time.format("%Y-%m-%d %H:%M:%S%.3f"),
            self.level,
            self.module,
            ctx_str,
            self.message
        )
    }

    /// Format as JSON
    pub fn format_json(&self) -> String {
        let mut map = HashMap::new();
        map.insert("timestamp".to_string(), self.timestamp.to_rfc3339());
        map.insert("level".to_string(), self.level.to_string());
        map.insert("module".to_string(), self.module.clone());
        map.insert("message".to_string(), self.message.clone());

        if let Some(ref ctx) = self.context {
            for (k, v) in ctx.to_map() {
                map.insert(format!("ctx.{}", k), v);
            }
        }

        serde_json::to_string(&map).unwrap_or_default()
    }
}

/// Log output destination
#[derive(Debug, Clone)]
pub enum LogOutput {
    /// Console/stdout
    Console,
    /// File with path
    File(PathBuf),
    /// Multiple outputs
    Multiple(Vec<LogOutput>),
}

/// Configuration for log rotation
#[derive(Debug, Clone)]
pub struct RotationConfig {
    /// Maximum file size in bytes before rotation
    pub max_size: u64,
    /// Maximum number of backup files to keep
    pub max_files: usize,
    /// Whether to compress rotated files
    pub compress: bool,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            max_size: 10 * 1024 * 1024, // 10 MB
            max_files: 5,
            compress: true,
        }
    }
}

/// Builder for creating Logger instances
pub struct LoggerBuilder {
    level: LogLevel,
    outputs: Vec<LogOutput>,
    rotation: RotationConfig,
    use_json: bool,
    remote_endpoint: Option<String>,
    remote_enabled: bool,
}

impl LoggerBuilder {
    /// Set the minimum log level
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    /// Add console output
    pub fn with_console(mut self) -> Self {
        self.outputs.push(LogOutput::Console);
        self
    }

    /// Add file output
    pub fn with_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.outputs.push(LogOutput::File(path.as_ref().to_path_buf()));
        self
    }

    /// Configure log rotation
    pub fn with_rotation(mut self, rotation: RotationConfig) -> Self {
        self.rotation = rotation;
        self
    }

    /// Use JSON format instead of plain text
    pub fn with_json_format(mut self, use_json: bool) -> Self {
        self.use_json = use_json;
        self
    }

    /// Enable remote logging (Pro feature)
    pub fn with_remote_logging<T: Into<String>>(mut self, endpoint: T) -> Self {
        self.remote_endpoint = Some(endpoint.into());
        self.remote_enabled = true;
        self
    }

    /// Build and initialize the logger
    pub fn build(self) -> io::Result<Logger> {
        Logger::init_with_config(self)
    }
}

impl Default for LoggerBuilder {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            outputs: vec![LogOutput::Console],
            rotation: RotationConfig::default(),
            use_json: false,
            remote_endpoint: None,
            remote_enabled: false,
        }
    }
}

/// Main logger struct
pub struct Logger {
    level: Arc<RwLock<LogLevel>>,
    outputs: Arc<Mutex<Vec<LogOutput>>>,
    rotation: RotationConfig,
    use_json: bool,
    remote_enabled: bool,
    remote_endpoint: Option<String>,
    file_handles: Arc<Mutex<HashMap<PathBuf, fs::File>>>,
}

impl Logger {
    /// Create a new logger builder
    pub fn builder() -> LoggerBuilder {
        LoggerBuilder::default()
    }

    /// Quick initialization with console output
    pub fn new() -> io::Result<Self> {
        Self::builder().with_console().build()
    }

    /// Initialize with file output
    pub fn with_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::builder().with_file(path).build()
    }

    /// Initialize with custom configuration
    fn init_with_config(config: LoggerBuilder) -> io::Result<Self> {
        let mut file_handles = HashMap::new();

        // Initialize file outputs
        for output in &config.outputs {
            if let LogOutput::File(path) = output {
                // Ensure parent directory exists
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)?;
                file_handles.insert(path.clone(), file);
            }
        }

        let logger = Self {
            level: Arc::new(RwLock::new(config.level)),
            outputs: Arc::new(Mutex::new(config.outputs)),
            rotation: config.rotation,
            use_json: config.use_json,
            remote_enabled: config.remote_enabled,
            remote_endpoint: config.remote_endpoint,
            file_handles: Arc::new(Mutex::new(file_handles)),
        };

        // Set as global logger (if needed)
        // This is a simplified version; in production you might use log crate integration

        Ok(logger)
    }

    /// Set the log level at runtime
    pub fn set_level(&self, level: LogLevel) {
        if let Ok(mut guard) = self.level.write() {
            *guard = level;
        }
    }

    /// Get the current log level
    pub fn level(&self) -> LogLevel {
        self.level.read().map(|g| *g).unwrap_or(LogLevel::Info)
    }

    /// Log a message with level and context
    pub fn log(&self, level: LogLevel, message: &str, context: Option<&LogContext>) {
        // Check level
        if !level.should_log(self.level()) {
            return;
        }

        let module = context
            .as_ref()
            .and_then(|c| c.module.clone())
            .unwrap_or_else(|| module_path!().to_string());

        let entry = LogEntry {
            timestamp: Utc::now(),
            level,
            module,
            message: message.to_string(),
            context: context.cloned(),
            file: None,
            line: None,
        };

        // Write to outputs
        self.write_entry(&entry);

        // Send to remote if enabled (Pro feature - fire and forget)
        if self.remote_enabled {
            self.send_remote(&entry);
        }
    }

    /// Write log entry to all configured outputs
    fn write_entry(&self, entry: &LogEntry) {
        let formatted = if self.use_json {
            entry.format_json()
        } else {
            entry.format_text()
        };

        let outputs = match self.outputs.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => return,
        };

        for output in outputs {
            match output {
                LogOutput::Console => {
                    let _ = writeln!(io::stdout(), "{}", formatted);
                }
                LogOutput::File(path) => {
                    self.write_to_file(&path, &formatted);
                }
                LogOutput::Multiple(_) => {}
            }
        }
    }

    /// Write to file with rotation support
    fn write_to_file(&self, path: &Path, message: &str) {
        let mut handles = match self.file_handles.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        // Check if rotation is needed
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.len() >= self.rotation.max_size {
                drop(handles); // Release lock before rotation
                self.rotate_file(path);
                handles = self.file_handles.lock().unwrap_or_else(|e| e.into_inner());
            }
        }

        // Write to file
        if let Some(file) = handles.get_mut(path) {
            let _ = writeln!(file, "{}", message);
            let _ = file.flush();
        }
    }

    /// Rotate log file
    fn rotate_file(&self, path: &Path) {
        let base_path = path.to_string_lossy();

        // Remove oldest file if at max
        let oldest = format!("{}.{}.gz", base_path, self.rotation.max_files);
        let _ = fs::remove_file(&oldest);

        // Shift existing files
        for i in (1..self.rotation.max_files).rev() {
            let old_path = format!("{}.{}", base_path, i);
            let new_path = format!("{}.{}", base_path, i + 1);
            let _ = fs::rename(&old_path, &new_path);
        }

        // Move current file to .1
        let current_backup = format!("{}.1", base_path);
        let _ = fs::rename(path, &current_backup);

        // Reopen file
        if let Ok(mut handles) = self.file_handles.lock() {
            if let Ok(file) = OpenOptions::new()
                .create(true)
                .append(true)
                .truncate(true)
                .open(path)
            {
                handles.insert(path.to_path_buf(), file);
            }
        }
    }

    /// Send log to remote endpoint (Pro feature - fire and forget)
    fn send_remote(&self, entry: &LogEntry) {
        if let Some(ref endpoint) = self.remote_endpoint {
            let endpoint = endpoint.clone();
            let payload = entry.format_json();

            // Spawn a thread to send (fire and forget)
            thread::spawn(move || {
                // This is a placeholder for actual HTTP implementation
                // In production, use reqwest or similar
                let _ = (endpoint, payload);
                // Actual implementation would POST to endpoint
            });
        }
    }

    // Convenience methods for different log levels

    /// Log at Error level
    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message, None);
    }

    /// Log at Error level with context
    pub fn error_with_context(&self, message: &str, context: &LogContext) {
        self.log(LogLevel::Error, message, Some(context));
    }

    /// Log at Warn level
    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message, None);
    }

    /// Log at Warn level with context
    pub fn warn_with_context(&self, message: &str, context: &LogContext) {
        self.log(LogLevel::Warn, message, Some(context));
    }

    /// Log at Info level
    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message, None);
    }

    /// Log at Info level with context
    pub fn info_with_context(&self, message: &str, context: &LogContext) {
        self.log(LogLevel::Info, message, Some(context));
    }

    /// Log at Debug level
    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message, None);
    }

    /// Log at Debug level with context
    pub fn debug_with_context(&self, message: &str, context: &LogContext) {
        self.log(LogLevel::Debug, message, Some(context));
    }

    /// Log at Trace level
    pub fn trace(&self, message: &str) {
        self.log(LogLevel::Trace, message, None);
    }

    /// Log at Trace level with context
    pub fn trace_with_context(&self, message: &str, context: &LogContext) {
        self.log(LogLevel::Trace, message, Some(context));
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new().expect("Failed to create default logger")
    }
}

unsafe impl Send for Logger {}
unsafe impl Sync for Logger {}

/// Global logger instance (optional)
static mut GLOBAL_LOGGER: Option<Arc<Logger>> = None;
static LOGGER_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global logger
pub fn init_global_logger(logger: Logger) {
    LOGGER_INIT.call_once(|| {
        unsafe {
            GLOBAL_LOGGER = Some(Arc::new(logger));
        }
    });
}

/// Get global logger instance
pub fn global_logger() -> Option<Arc<Logger>> {
    unsafe { GLOBAL_LOGGER.clone() }
}

/// Log using the global logger
pub fn log_global(level: LogLevel, message: &str) {
    if let Some(ref logger) = global_logger() {
        logger.log(level, message, None);
    }
}

/// Initialize logger from environment variables
///
/// Uses:
/// - `EASYSSH_LOG_LEVEL`: Log level (error, warn, info, debug, trace)
/// - `EASYSSH_LOG_FILE`: Path to log file (optional)
/// - `EASYSSH_LOG_JSON`: Use JSON format (true/false)
pub fn init_from_env() -> io::Result<Logger> {
    use std::env;

    let level = env::var("EASYSSH_LOG_LEVEL")
        .ok()
        .and_then(|s| LogLevel::from_str(&s))
        .unwrap_or(LogLevel::Info);

    let mut builder = Logger::builder().with_level(level);

    if let Ok(file_path) = env::var("EASYSSH_LOG_FILE") {
        builder = builder.with_file(file_path);
    } else {
        builder = builder.with_console();
    }

    if env::var("EASYSSH_LOG_JSON").ok() == Some("true".to_string()) {
        builder = builder.with_json_format(true);
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("error"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("WARN"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::from_str("Info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("debug"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::from_str("trace"), Some(LogLevel::Trace));
        assert_eq!(LogLevel::from_str("invalid"), None);
    }

    #[test]
    fn test_log_level_should_log() {
        let min = LogLevel::Info;
        assert!(LogLevel::Error.should_log(min));
        assert!(LogLevel::Warn.should_log(min));
        assert!(LogLevel::Info.should_log(min));
        assert!(!LogLevel::Debug.should_log(min));
        assert!(!LogLevel::Trace.should_log(min));
    }

    #[test]
    fn test_log_context_builder() {
        let ctx = LogContext::new()
            .with_request_id("req-123")
            .with_session_id("sess-456")
            .with_user_id("user-789")
            .with_operation("connect")
            .with_module("ssh")
            .with_field("custom", "value");

        assert_eq!(ctx.request_id, Some("req-123".to_string()));
        assert_eq!(ctx.session_id, Some("sess-456".to_string()));
        assert_eq!(ctx.user_id, Some("user-789".to_string()));
        assert_eq!(ctx.operation, Some("connect".to_string()));
        assert_eq!(ctx.module, Some("ssh".to_string()));
        assert_eq!(ctx.custom_fields.get("custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_log_context_to_map() {
        let ctx = LogContext::new()
            .with_request_id("req-123")
            .with_field("custom", "value");

        let map = ctx.to_map();
        assert_eq!(map.get("request_id"), Some(&"req-123".to_string()));
        assert_eq!(map.get("custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_log_entry_format_text() {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: LogLevel::Info,
            module: "test".to_string(),
            message: "test message".to_string(),
            context: None,
            file: None,
            line: None,
        };

        let formatted = entry.format_text();
        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("test"));
        assert!(formatted.contains("test message"));
    }

    #[test]
    fn test_log_entry_format_json() {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: LogLevel::Info,
            module: "test".to_string(),
            message: "test message".to_string(),
            context: Some(LogContext::new().with_request_id("req-123")),
            file: None,
            line: None,
        };

        let formatted = entry.format_json();
        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("test message"));
        assert!(formatted.contains("req-123"));
    }

    #[test]
    fn test_logger_builder() {
        let builder = Logger::builder()
            .with_level(LogLevel::Debug)
            .with_console()
            .with_file("/tmp/test.log")
            .with_json_format(true);

        // Just verify it builds without error
        // Can't test actual file output in unit test without temp dir
        assert_eq!(builder.level, LogLevel::Debug);
        assert_eq!(builder.use_json, true);
    }

    #[test]
    fn test_rotation_config_default() {
        let config = RotationConfig::default();
        assert_eq!(config.max_size, 10 * 1024 * 1024);
        assert_eq!(config.max_files, 5);
        assert_eq!(config.compress, true);
    }

    #[test]
    fn test_logger_level_management() {
        let logger = Logger::new().unwrap();

        logger.set_level(LogLevel::Debug);
        assert_eq!(logger.level(), LogLevel::Debug);

        logger.set_level(LogLevel::Error);
        assert_eq!(logger.level(), LogLevel::Error);
    }

    #[test]
    fn test_logger_with_context() {
        let logger = Logger::new().unwrap();
        let ctx = LogContext::new()
            .with_request_id("test-req")
            .with_module("test_module");

        // Just verify it doesn't panic
        logger.info_with_context("test message", &ctx);
        logger.debug_with_context("debug message", &ctx);
        logger.error_with_context("error message", &ctx);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(format!("{}", LogLevel::Error), "ERROR");
        assert_eq!(format!("{}", LogLevel::Warn), "WARN");
        assert_eq!(format!("{}", LogLevel::Info), "INFO");
        assert_eq!(format!("{}", LogLevel::Debug), "DEBUG");
        assert_eq!(format!("{}", LogLevel::Trace), "TRACE");
    }

    #[test]
    fn test_file_logging() {
        let temp_dir = TempDir::new().unwrap();
        let log_file = temp_dir.path().join("test.log");

        let logger = Logger::with_file(&log_file).unwrap();
        logger.info("test message in file");

        // Give it a moment to write
        thread::sleep(Duration::from_millis(100));

        // Read and verify
        let content = fs::read_to_string(&log_file).unwrap();
        assert!(content.contains("test message in file"));
        assert!(content.contains("INFO"));
    }

    #[test]
    fn test_logger_new() {
        let logger = Logger::new();
        assert!(logger.is_ok());
    }

    #[test]
    fn test_init_from_env() {
        // Save current env vars
        let old_level = env::var("EASYSSH_LOG_LEVEL").ok();
        let old_file = env::var("EASYSSH_LOG_FILE").ok();
        let old_json = env::var("EASYSSH_LOG_JSON").ok();

        // Set test values
        env::set_var("EASYSSH_LOG_LEVEL", "debug");
        env::remove_var("EASYSSH_LOG_FILE");
        env::remove_var("EASYSSH_LOG_JSON");

        // Should create logger successfully
        let result = init_from_env();
        assert!(result.is_ok());

        // Restore env vars
        match old_level {
            Some(v) => env::set_var("EASYSSH_LOG_LEVEL", v),
            None => env::remove_var("EASYSSH_LOG_LEVEL"),
        }
        match old_file {
            Some(v) => env::set_var("EASYSSH_LOG_FILE", v),
            None => env::remove_var("EASYSSH_LOG_FILE"),
        }
        match old_json {
            Some(v) => env::set_var("EASYSSH_LOG_JSON", v),
            None => env::remove_var("EASYSSH_LOG_JSON"),
        }
    }
}
