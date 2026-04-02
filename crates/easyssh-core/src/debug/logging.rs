//! 日志系统模块
//!
//! 提供日志查看和管理功能，所有版本可用

use crate::debug::types::LogEntry;
use std::collections::VecDeque;
use std::sync::Mutex;

/// 内存日志缓冲区大小
const MAX_LOG_BUFFER_SIZE: usize = 10000;

lazy_static::lazy_static! {
    static ref LOG_BUFFER: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::with_capacity(MAX_LOG_BUFFER_SIZE));
}

/// 添加日志条目到缓冲区
pub fn add_log_entry(entry: LogEntry) {
    if let Ok(mut buffer) = LOG_BUFFER.lock() {
        if buffer.len() >= MAX_LOG_BUFFER_SIZE {
            buffer.pop_front();
        }
        buffer.push_back(entry);
    }
}

/// 获取日志条目
///
/// # Arguments
/// * `level` - 最低日志级别 (ERROR, WARN, INFO, DEBUG, TRACE)
/// * `limit` - 最大返回条目数
/// * `filter` - 可选的文本过滤
pub fn get_logs(
    level: Option<String>,
    limit: Option<usize>,
    filter: Option<String>,
) -> Vec<LogEntry> {
    let buffer = match LOG_BUFFER.lock() {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };

    let limit = limit.unwrap_or(100);
    let level_filter = level.unwrap_or_else(|| "DEBUG".to_string());
    let text_filter = filter.map(|f| f.to_lowercase());

    // 日志级别优先级
    let level_priority = |l: &str| match l {
        "ERROR" => 4,
        "WARN" => 3,
        "INFO" => 2,
        "DEBUG" => 1,
        "TRACE" => 0,
        _ => 0,
    };

    let min_priority = level_priority(&level_filter);

    buffer
        .iter()
        .rev() // 最新的在前
        .filter(|entry| {
            // 级别过滤
            level_priority(&entry.level) >= min_priority
        })
        .filter(|entry| {
            // 文本过滤
            if let Some(ref f) = text_filter {
                entry.message.to_lowercase().contains(f)
                    || entry.target.to_lowercase().contains(f)
            } else {
                true
            }
        })
        .take(limit)
        .cloned()
        .collect()
}

/// 获取最近的错误日志
pub fn get_recent_errors(limit: usize) -> Vec<LogEntry> {
    get_logs(Some("ERROR".to_string()), Some(limit), None)
}

/// 清除日志缓冲区
pub fn clear_logs() {
    if let Ok(mut buffer) = LOG_BUFFER.lock() {
        buffer.clear();
    }
}

/// 获取日志统计
pub fn get_log_stats() -> LogStats {
    let buffer = match LOG_BUFFER.lock() {
        Ok(b) => b,
        Err(_) => return LogStats::default(),
    };

    let mut error_count = 0;
    let mut warn_count = 0;
    let mut info_count = 0;
    let mut debug_count = 0;

    for entry in buffer.iter() {
        match entry.level.as_str() {
            "ERROR" => error_count += 1,
            "WARN" => warn_count += 1,
            "INFO" => info_count += 1,
            "DEBUG" => debug_count += 1,
            _ => {}
        }
    }

    LogStats {
        total_entries: buffer.len(),
        error_count,
        warn_count,
        info_count,
        debug_count,
        other_count: buffer.len() - error_count - warn_count - info_count - debug_count,
    }
}

/// 日志统计
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LogStats {
    pub total_entries: usize,
    pub error_count: usize,
    pub warn_count: usize,
    pub info_count: usize,
    pub debug_count: usize,
    pub other_count: usize,
}

/// 日志导出
pub fn export_logs(format: ExportFormat) -> Result<String, String> {
    let logs = get_logs(None, None, None);

    match format {
        ExportFormat::Json => {
            serde_json::to_string_pretty(&logs).map_err(|e| e.to_string())
        }
        ExportFormat::Csv => {
            let mut csv = String::new();
            csv.push_str("timestamp,level,target,message\n");
            for entry in logs {
                csv.push_str(&format!(
                    "{},{},{},{}\n",
                    entry.timestamp, entry.level, entry.target, entry.message
                ));
            }
            Ok(csv)
        }
        ExportFormat::Txt => {
            let mut txt = String::new();
            for entry in logs {
                txt.push_str(&format!(
                    "[{}] {} [{}] {}\n",
                    entry.timestamp, entry.level, entry.target, entry.message
                ));
            }
            Ok(txt)
        }
    }
}

/// 导出格式
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Json,
    Csv,
    Txt,
}

/// 实时日志订阅（简化实现）
pub struct LogSubscriber {
    last_position: usize,
}

impl LogSubscriber {
    pub fn new() -> Self {
        Self { last_position: 0 }
    }

    /// 获取新日志条目
    pub fn poll(&mut self) -> Vec<LogEntry> {
        let buffer = match LOG_BUFFER.lock() {
            Ok(b) => b,
            Err(_) => return Vec::new(),
        };

        let current_len = buffer.len();
        if current_len <= self.last_position {
            return Vec::new();
        }

        let new_entries: Vec<LogEntry> = buffer
            .iter()
            .skip(self.last_position)
            .cloned()
            .collect();

        self.last_position = current_len;
        new_entries
    }
}

impl Default for LogSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

/// 日志级别配置
pub fn set_log_level(level: &str) {
    let level_filter = match level.to_uppercase().as_str() {
        "ERROR" => log::LevelFilter::Error,
        "WARN" => log::LevelFilter::Warn,
        "INFO" => log::LevelFilter::Info,
        "DEBUG" => log::LevelFilter::Debug,
        "TRACE" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    };

    // 注意：这只会影响后续的日志记录
    // 实际设置全局日志级别需要初始化时配置
    log::set_max_level(level_filter);
}

/// 获取当前日志级别
pub fn get_log_level() -> String {
    match log::max_level() {
        log::LevelFilter::Off => "OFF".to_string(),
        log::LevelFilter::Error => "ERROR".to_string(),
        log::LevelFilter::Warn => "WARN".to_string(),
        log::LevelFilter::Info => "INFO".to_string(),
        log::LevelFilter::Debug => "DEBUG".to_string(),
        log::LevelFilter::Trace => "TRACE".to_string(),
    }
}

/// 日志过滤配置
#[derive(Debug, Clone)]
pub struct LogFilterConfig {
    pub target_filters: Vec<String>,
    pub level_filter: log::LevelFilter,
    pub message_filter: Option<String>,
}

impl Default for LogFilterConfig {
    fn default() -> Self {
        Self {
            target_filters: Vec::new(),
            level_filter: log::LevelFilter::Debug,
            message_filter: None,
        }
    }
}

/// 应用日志过滤
pub fn apply_filter(entries: &[LogEntry], config: &LogFilterConfig) -> Vec<LogEntry> {
    entries
        .iter()
        .filter(|entry| {
            // 级别过滤
            let level = match entry.level.as_str() {
                "ERROR" => log::Level::Error,
                "WARN" => log::Level::Warn,
                "INFO" => log::Level::Info,
                "DEBUG" => log::Level::Debug,
                "TRACE" => log::Level::Trace,
                _ => log::Level::Debug,
            };
            level <= config.level_filter.to_level().unwrap_or(log::Level::Debug)
        })
        .filter(|entry| {
            // 目标过滤
            if config.target_filters.is_empty() {
                return true;
            }
            config.target_filters.iter().any(|t| entry.target.contains(t))
        })
        .filter(|entry| {
            // 消息过滤
            if let Some(ref f) = config.message_filter {
                entry.message.contains(f)
            } else {
                true
            }
        })
        .cloned()
        .collect()
}

/// 日志查看器设置
#[derive(Debug, Clone)]
pub struct LogViewerSettings {
    pub auto_scroll: bool,
    pub show_timestamp: bool,
    pub show_target: bool,
    pub max_lines: usize,
    pub wrap_lines: bool,
}

impl Default for LogViewerSettings {
    fn default() -> Self {
        Self {
            auto_scroll: true,
            show_timestamp: true,
            show_target: true,
            max_lines: 1000,
            wrap_lines: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_buffer() {
        clear_logs();

        add_log_entry(LogEntry {
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            level: "INFO".to_string(),
            target: "test".to_string(),
            message: "Test message".to_string(),
            file: None,
            line: None,
        });

        let logs = get_logs(None, None, None);
        assert!(!logs.is_empty());

        let stats = get_log_stats();
        assert_eq!(stats.info_count, 1);

        clear_logs();
        let logs = get_logs(None, None, None);
        assert!(logs.is_empty());
    }

    #[test]
    fn test_log_filter() {
        let config = LogFilterConfig {
            target_filters: vec!["test".to_string()],
            level_filter: log::LevelFilter::Info,
            message_filter: Some("important".to_string()),
        };

        let entries = vec![
            LogEntry {
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                level: "INFO".to_string(),
                target: "test".to_string(),
                message: "Important message".to_string(),
                file: None,
                line: None,
            },
            LogEntry {
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                level: "DEBUG".to_string(),
                target: "other".to_string(),
                message: "Other message".to_string(),
                file: None,
                line: None,
            },
        ];

        let filtered = apply_filter(&entries, &config);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].message, "Important message");
    }

    #[test]
    fn test_export_logs() {
        clear_logs();

        add_log_entry(LogEntry {
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            level: "ERROR".to_string(),
            target: "test".to_string(),
            message: "Error message".to_string(),
            file: None,
            line: None,
        });

        let json = export_logs(ExportFormat::Json).unwrap();
        assert!(json.contains("ERROR"));

        let csv = export_logs(ExportFormat::Csv).unwrap();
        assert!(csv.contains("timestamp,level,target,message"));

        let txt = export_logs(ExportFormat::Txt).unwrap();
        assert!(txt.contains("Error message"));

        clear_logs();
    }
}
