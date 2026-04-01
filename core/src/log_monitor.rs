#![allow(dead_code)]

use crate::error::LiteError;
use crate::ssh::SshSessionManager;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;
use tokio::time::interval;

// ============================================================================
// 日志级别和颜色定义
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogLevel {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR,
    FATAL,
    UNKNOWN,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::TRACE
    }
}

impl LogLevel {
    pub fn color(&self) -> &'static str {
        match self {
            LogLevel::TRACE => "#6c757d",   // 灰色
            LogLevel::DEBUG => "#0d6efd",   // 蓝色
            LogLevel::INFO => "#198754",    // 绿色
            LogLevel::WARN => "#ffc107",    // 黄色
            LogLevel::ERROR => "#dc3545",   // 红色
            LogLevel::FATAL => "#721c24",   // 深红色
            LogLevel::UNKNOWN => "#6c757d", // 灰色
        }
    }

    pub fn priority(&self) -> u8 {
        match self {
            LogLevel::TRACE => 0,
            LogLevel::DEBUG => 1,
            LogLevel::INFO => 2,
            LogLevel::WARN => 3,
            LogLevel::ERROR => 4,
            LogLevel::FATAL => 5,
            LogLevel::UNKNOWN => 2,
        }
    }

    pub fn from_str(s: &str) -> Self {
        let upper = s.to_uppercase();
        if upper.contains("TRACE") || upper.contains("TRC") {
            LogLevel::TRACE
        } else if upper.contains("DEBUG") || upper.contains("DBG") {
            LogLevel::DEBUG
        } else if upper.contains("INFO") || upper.contains("INF") {
            LogLevel::INFO
        } else if upper.contains("WARN") || upper.contains("WARNING") || upper.contains("WRN") {
            LogLevel::WARN
        } else if upper.contains("ERROR") || upper.contains("ERR") || upper.contains("FAILED") {
            LogLevel::ERROR
        } else if upper.contains("FATAL") || upper.contains("CRITICAL") || upper.contains("CRT") {
            LogLevel::FATAL
        } else {
            LogLevel::UNKNOWN
        }
    }
}

// ============================================================================
// 日志条目结构
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub source_id: String,
    pub source_name: String,
    pub timestamp: u64,
    pub level: LogLevel,
    pub message: String,
    pub raw_line: String,
    pub metadata: HashMap<String, String>,
    pub color: String,
    pub sequence: u64,
}

impl LogEntry {
    pub fn new(source_id: String, source_name: String, raw_line: String, sequence: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let level = Self::detect_level(&raw_line);
        let message = Self::extract_message(&raw_line);
        let metadata = Self::extract_metadata(&raw_line);
        let color = level.color().to_string();

        Self {
            id: format!("{}-{}-{}", source_id, timestamp, sequence),
            source_id,
            source_name,
            timestamp,
            level,
            message,
            raw_line,
            metadata,
            color,
            sequence,
        }
    }

    fn detect_level(line: &str) -> LogLevel {
        // 检测常见的日志级别模式
        let patterns = [
            (r"\b(FATAL|CRITICAL|PANIC)\b", LogLevel::FATAL),
            (r"\b(ERROR|ERR|FAILED|FAILURE)\b", LogLevel::ERROR),
            (r"\b(WARN|WARNING|WRN)\b", LogLevel::WARN),
            (r"\b(INFO|INF|NOTICE)\b", LogLevel::INFO),
            (r"\b(DEBUG|DBG)\b", LogLevel::DEBUG),
            (r"\b(TRACE|TRC)\b", LogLevel::TRACE),
        ];

        for (pattern, level) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(line) {
                    return *level;
                }
            }
        }

        // 检查特殊错误指示器
        if line.contains("Exception")
            || line.contains("exception")
            || line.contains("Stack trace")
            || line.contains("stacktrace")
            || line.contains("panic:")
            || line.contains("Panic")
        {
            return LogLevel::ERROR;
        }

        LogLevel::UNKNOWN
    }

    fn extract_message(line: &str) -> String {
        // 移除常见的前缀（时间戳、日志级别等）
        let patterns = [
            r"^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:?\d{2})?\s*",
            r"^\[?[\d\-:T\.Z\s]+\]?\s*",
            r"\b(TRACE|DEBUG|INFO|WARN|WARNING|ERROR|ERR|FATAL|CRITICAL|NOTICE)\b\s*:?\s*",
            r"^\[\s*\w+\s*\]\s*",
        ];

        let mut result = line.to_string();
        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                result = re.replace(&result, "").to_string();
            }
        }

        result.trim().to_string()
    }

    fn extract_metadata(line: &str) -> HashMap<String, String> {
        let mut metadata = HashMap::new();

        // 提取时间戳
        let ts_patterns = [
            r"(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:?\d{2})?)",
            r"\[([\d\-:T\.Z\s]+)\]",
        ];

        for pattern in &ts_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(line) {
                    if let Some(ts) = caps.get(1) {
                        metadata.insert("timestamp".to_string(), ts.as_str().to_string());
                        break;
                    }
                }
            }
        }

        // 提取服务/组件名
        if let Ok(re) = Regex::new(r"\[([^\]]+)\]") {
            for caps in re.captures_iter(line) {
                if let Some(component) = caps.get(1) {
                    let comp = component.as_str();
                    if !comp.contains("ERROR") && !comp.contains("WARN") && !comp.contains("INFO") {
                        metadata.insert("component".to_string(), comp.to_string());
                        break;
                    }
                }
            }
        }

        // 提取请求ID
        if let Ok(re) =
            Regex::new(r"(req[_-]?id[:=]\s*\w+|request[_-]?id[:=]\s*\w+|trace[_-]?id[:=]\s*\w+)")
        {
            if let Some(caps) = re.captures(line) {
                metadata.insert(
                    "request_id".to_string(),
                    caps.get(0).unwrap().as_str().to_string(),
                );
            }
        }

        // 提取IP地址
        if let Ok(re) = Regex::new(r"\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\b") {
            if let Some(caps) = re.captures(line) {
                metadata.insert(
                    "ip_address".to_string(),
                    caps.get(1).unwrap().as_str().to_string(),
                );
            }
        }

        metadata
    }

    pub fn matches_filter(&self, filter: &LogFilter) -> bool {
        // 级别过滤
        if self.level.priority() < filter.min_level.priority() {
            return false;
        }

        // 关键词过滤
        if let Some(ref keywords) = filter.keywords {
            let content = format!("{} {}", self.message, self.raw_line).to_lowercase();
            for keyword in keywords {
                if !content.contains(&keyword.to_lowercase()) {
                    return false;
                }
            }
        }

        // 正则过滤
        if let Some(ref pattern) = filter.regex_pattern {
            if let Ok(re) = Regex::new(pattern) {
                if !re.is_match(&self.raw_line) {
                    return false;
                }
            }
        }

        // 时间范围过滤
        if let Some(start) = filter.start_time {
            if self.timestamp < start {
                return false;
            }
        }
        if let Some(end) = filter.end_time {
            if self.timestamp > end {
                return false;
            }
        }

        // 源过滤
        if !filter.source_ids.is_empty() && !filter.source_ids.contains(&self.source_id) {
            return false;
        }

        true
    }
}

// ============================================================================
// 日志过滤配置
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogFilter {
    pub min_level: LogLevel,
    pub keywords: Option<Vec<String>>,
    pub regex_pattern: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub source_ids: Vec<String>,
    pub limit: Option<usize>,
}

impl LogFilter {
    pub fn new() -> Self {
        Self {
            min_level: LogLevel::TRACE,
            keywords: None,
            regex_pattern: None,
            start_time: None,
            end_time: None,
            source_ids: Vec::new(),
            limit: None,
        }
    }

    pub fn with_min_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = Some(keywords);
        self
    }

    pub fn with_regex(mut self, pattern: String) -> Self {
        self.regex_pattern = Some(pattern);
        self
    }

    pub fn with_time_range(mut self, start: u64, end: u64) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    pub fn with_sources(mut self, sources: Vec<String>) -> Self {
        self.source_ids = sources;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

// ============================================================================
// 日志源配置
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSource {
    pub id: String,
    pub name: String,
    pub server_id: String,
    pub log_path: String,
    pub log_type: LogType,
    pub parser_config: ParserConfig,
    pub color: String,
    pub enabled: bool,
    pub max_lines_per_batch: usize,
    pub poll_interval_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogType {
    SystemdJournal,
    Syslog,
    Application,
    Nginx,
    Apache,
    Docker,
    Kubernetes,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    pub timestamp_format: Option<String>,
    pub level_pattern: Option<String>,
    pub custom_patterns: HashMap<String, String>,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            timestamp_format: None,
            level_pattern: None,
            custom_patterns: HashMap::new(),
        }
    }
}

impl LogSource {
    pub fn new(name: String, server_id: String, log_path: String, log_type: LogType) -> Self {
        let colors = [
            "#0d6efd", "#6610f2", "#6f42c1", "#d63384", "#dc3545", "#fd7e14", "#ffc107", "#198754",
            "#20c997", "#0dcaf0", "#adb5bd",
        ];

        let color = colors[name.len() % colors.len()].to_string();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            server_id,
            log_path,
            log_type,
            parser_config: ParserConfig::default(),
            color,
            enabled: true,
            max_lines_per_batch: 100,
            poll_interval_ms: 1000,
        }
    }

    pub fn get_tail_command(&self, lines: usize) -> String {
        match self.log_type {
            LogType::SystemdJournal => {
                format!("journalctl -u {} --no-pager -n {} -f", self.log_path, lines)
            }
            LogType::Docker => {
                format!("docker logs -f --tail {} {}", lines, self.log_path)
            }
            LogType::Kubernetes => {
                format!("kubectl logs -f --tail {} {}", lines, self.log_path)
            }
            _ => {
                format!("tail -n {} -f {}", lines, self.log_path)
            }
        }
    }
}

// ============================================================================
// 告警规则
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogAlertRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub condition: LogAlertCondition,
    pub actions: Vec<LogAlertAction>,
    pub cooldown_seconds: u64,
    pub last_triggered: Option<u64>,
    pub trigger_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogAlertCondition {
    KeywordMatch {
        keywords: Vec<String>,
        case_sensitive: bool,
    },
    LevelThreshold {
        min_level: LogLevel,
        consecutive_count: usize,
    },
    RateThreshold {
        log_level: LogLevel,
        logs_per_minute: u32,
    },
    PatternMatch {
        regex: String,
    },
    Composite {
        conditions: Vec<LogAlertCondition>,
        operator: LogicalOperator,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicalOperator {
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogAlertAction {
    Webhook {
        url: String,
        headers: HashMap<String, String>,
    },
    Email {
        recipients: Vec<String>,
        subject_template: String,
    },
    DesktopNotification {
        title: String,
        body_template: String,
    },
    Sound {
        file_path: String,
    },
    ExecuteCommand {
        command: String,
    },
    PauseStream {
        source_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogAlertEvent {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub triggered_at: u64,
    pub log_entry: LogEntry,
    pub message: String,
}

impl LogAlertRule {
    pub fn new(name: String, condition: LogAlertCondition) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            enabled: true,
            condition,
            actions: Vec::new(),
            cooldown_seconds: 60,
            last_triggered: None,
            trigger_count: 0,
        }
    }

    pub fn with_action(mut self, action: LogAlertAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn with_cooldown(mut self, seconds: u64) -> Self {
        self.cooldown_seconds = seconds;
        self
    }

    pub fn check(
        &mut self,
        entry: &LogEntry,
        recent_entries: &[LogEntry],
    ) -> Option<LogAlertEvent> {
        if !self.enabled {
            return None;
        }

        // 检查冷却时间
        if let Some(last) = self.last_triggered {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if now - last < self.cooldown_seconds {
                return None;
            }
        }

        let triggered = match &self.condition {
            LogAlertCondition::KeywordMatch {
                keywords,
                case_sensitive,
            } => {
                let content = if *case_sensitive {
                    entry.raw_line.clone()
                } else {
                    entry.raw_line.to_lowercase()
                };
                keywords.iter().any(|kw| {
                    let keyword = if *case_sensitive {
                        kw.clone()
                    } else {
                        kw.to_lowercase()
                    };
                    content.contains(&keyword)
                })
            }
            LogAlertCondition::LevelThreshold {
                min_level,
                consecutive_count,
            } => {
                if entry.level.priority() < min_level.priority() {
                    return None;
                }
                // 检查连续的日志条目
                let mut count = 1;
                for e in recent_entries.iter().rev().take(*consecutive_count) {
                    if e.level.priority() >= min_level.priority() && e.source_id == entry.source_id
                    {
                        count += 1;
                    } else {
                        break;
                    }
                }
                count >= *consecutive_count
            }
            LogAlertCondition::RateThreshold {
                log_level,
                logs_per_minute,
            } => {
                if entry.level.priority() < log_level.priority() {
                    return None;
                }
                let one_minute_ago = entry.timestamp.saturating_sub(60);
                let count = recent_entries
                    .iter()
                    .filter(|e| {
                        e.source_id == entry.source_id
                            && e.timestamp >= one_minute_ago
                            && e.level.priority() >= log_level.priority()
                    })
                    .count();
                count as u32 >= *logs_per_minute
            }
            LogAlertCondition::PatternMatch { regex } => {
                if let Ok(re) = Regex::new(regex) {
                    re.is_match(&entry.raw_line)
                } else {
                    false
                }
            }
            LogAlertCondition::Composite {
                conditions,
                operator,
            } => {
                let results: Vec<bool> = conditions
                    .iter()
                    .map(|c| {
                        let mut temp_rule = LogAlertRule::new("temp".to_string(), c.clone());
                        temp_rule.check(entry, recent_entries).is_some()
                    })
                    .collect();
                match operator {
                    LogicalOperator::And => results.iter().all(|&x| x),
                    LogicalOperator::Or => results.iter().any(|&x| x),
                }
            }
        };

        if triggered {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            self.last_triggered = Some(now);
            self.trigger_count += 1;

            Some(LogAlertEvent {
                id: uuid::Uuid::new_v4().to_string(),
                rule_id: self.id.clone(),
                rule_name: self.name.clone(),
                triggered_at: now,
                log_entry: entry.clone(),
                message: format!("告警规则 '{}' 触发: {}", self.name, entry.message),
            })
        } else {
            None
        }
    }
}

// ============================================================================
// 日志统计和分析
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStats {
    pub total_entries: u64,
    pub entries_by_level: HashMap<LogLevel, u64>,
    pub entries_by_source: HashMap<String, u64>,
    pub entries_per_minute: f64,
    pub error_rate: f64,
    pub top_error_patterns: Vec<ErrorPattern>,
    pub time_series: Vec<TimeSeriesPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub pattern: String,
    pub count: u64,
    pub sample_message: String,
    pub first_seen: u64,
    pub last_seen: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: u64,
    pub total_count: u64,
    pub error_count: u64,
    pub warn_count: u64,
    pub info_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogAnalysisResult {
    pub patterns: Vec<ErrorPattern>,
    pub anomalies: Vec<Anomaly>,
    pub trends: Vec<Trend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub timestamp: u64,
    pub description: String,
    pub severity: LogLevel,
    pub related_entries: Vec<String>, // entry IDs
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trend {
    pub metric: String,
    pub direction: LogTrendDirection,
    pub change_percent: f64,
    pub period_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogTrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

// ============================================================================
// 导出配置
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub filter: LogFilter,
    pub include_raw: bool,
    pub include_metadata: bool,
    pub max_size_mb: u64,
    pub compress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    JSON,
    CSV,
    PlainText,
    HTML,
}

// ============================================================================
// WebSocket消息类型
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LogMonitorMessage {
    #[serde(rename = "entry")]
    NewEntry { entry: LogEntry },
    #[serde(rename = "batch")]
    BatchEntries { entries: Vec<LogEntry> },
    #[serde(rename = "stats")]
    StatsUpdate { stats: LogStats },
    #[serde(rename = "alert")]
    Alert { alert: LogAlertEvent },
    #[serde(rename = "source_connected")]
    SourceConnected {
        source_id: String,
        source_name: String,
    },
    #[serde(rename = "source_disconnected")]
    SourceDisconnected { source_id: String, reason: String },
    #[serde(rename = "error")]
    Error { message: String },
}

// ============================================================================
// 日志监控中心
// ============================================================================

pub struct LogMonitorCenter {
    sources: Arc<RwLock<HashMap<String, LogSource>>>,
    entries: Arc<RwLock<Vec<LogEntry>>>,
    alert_rules: Arc<RwLock<Vec<LogAlertRule>>>,
    broadcast_tx: broadcast::Sender<LogMonitorMessage>,
    running: Arc<AtomicBool>,
    sequence_counter: Arc<AtomicU64>,
    max_entries: usize,
    rotation_enabled: bool,
    retention_hours: u64,
    ssh_manager: Arc<RwLock<SshSessionManager>>,
    active_streams: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
}

impl LogMonitorCenter {
    pub fn new(ssh_manager: Arc<RwLock<SshSessionManager>>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(10000);

        Self {
            sources: Arc::new(RwLock::new(HashMap::new())),
            entries: Arc::new(RwLock::new(Vec::with_capacity(10000))),
            alert_rules: Arc::new(RwLock::new(Vec::new())),
            broadcast_tx,
            running: Arc::new(AtomicBool::new(true)),
            sequence_counter: Arc::new(AtomicU64::new(0)),
            max_entries: 100000,
            rotation_enabled: true,
            retention_hours: 24,
            ssh_manager,
            active_streams: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LogMonitorMessage> {
        self.broadcast_tx.subscribe()
    }

    pub async fn add_source(&self, source: LogSource) -> Result<(), LiteError> {
        let mut sources = self.sources.write().await;
        sources.insert(source.id.clone(), source.clone());
        drop(sources);

        // 启动该源的日志流
        let source_for_stream = source.clone();
        self.start_source_stream(source_for_stream).await;

        // 广播源连接消息
        let _ = self.broadcast_tx.send(LogMonitorMessage::SourceConnected {
            source_id: source.id.clone(),
            source_name: source.name.clone(),
        });

        Ok(())
    }

    pub async fn remove_source(&self, source_id: &str) -> Result<(), LiteError> {
        // 停止流
        let mut streams = self.active_streams.write().await;
        if let Some(handle) = streams.remove(source_id) {
            handle.abort();
        }
        drop(streams);

        // 移除源
        let mut sources = self.sources.write().await;
        sources.remove(source_id);

        // 广播断开消息
        let _ = self
            .broadcast_tx
            .send(LogMonitorMessage::SourceDisconnected {
                source_id: source_id.to_string(),
                reason: "User removed".to_string(),
            });

        Ok(())
    }

    pub async fn get_sources(&self) -> Vec<LogSource> {
        let sources = self.sources.read().await;
        sources.values().cloned().collect()
    }

    pub async fn add_alert_rule(&self, rule: LogAlertRule) {
        let mut rules = self.alert_rules.write().await;
        rules.push(rule);
    }

    pub async fn remove_alert_rule(&self, rule_id: &str) {
        let mut rules = self.alert_rules.write().await;
        rules.retain(|r| r.id != rule_id);
    }

    pub async fn get_alert_rules(&self) -> Vec<LogAlertRule> {
        let rules = self.alert_rules.read().await;
        rules.clone()
    }

    async fn start_source_stream(&self, source: LogSource) {
        if !source.enabled {
            return;
        }

        let source_id = source.id.clone();
        let source_id_for_insert = source_id.clone();
        let source_name = source.name.clone();
        let server_id = source.server_id.clone();
        let log_path = source.log_path.clone();
        let log_type = source.log_type.clone();
        let poll_interval = source.poll_interval_ms;
        let max_lines = source.max_lines_per_batch;

        let broadcast_tx = self.broadcast_tx.clone();
        let entries = self.entries.clone();
        let sequence = Arc::clone(&self.sequence_counter);
        let ssh_manager = self.ssh_manager.clone();
        let running = self.running.clone();

        let handle = tokio::spawn(async move {
            let mut last_position: Option<String> = None;

            loop {
                if !running.load(Ordering::Relaxed) {
                    break;
                }

                // 构建命令
                let cmd = match log_type {
                    LogType::SystemdJournal => {
                        if let Some(ref cursor) = last_position {
                            format!(
                                "journalctl --after-cursor='{}' --no-pager -n {} --show-cursor",
                                cursor, max_lines
                            )
                        } else {
                            format!("journalctl --no-pager -n {} -f --show-cursor", max_lines)
                        }
                    }
                    _ => format!("tail -n {} {}", max_lines, log_path),
                };

                // 执行命令获取日志
                let ssh = ssh_manager.read().await;
                match ssh.execute(&server_id, &cmd).await {
                    Ok(output) => {
                        let lines: Vec<&str> = output.lines().collect();

                        // 解析cursor位置
                        if matches!(log_type, LogType::SystemdJournal) {
                            for line in &lines {
                                if line.starts_with("-- cursor: ") {
                                    last_position =
                                        Some(line.trim_start_matches("-- cursor: ").to_string());
                                    break;
                                }
                            }
                        }

                        let mut new_entries = Vec::new();
                        for line in &lines {
                            if line.is_empty() || line.starts_with("-- ") {
                                continue;
                            }

                            let seq = sequence.fetch_add(1, Ordering::SeqCst);
                            let entry = LogEntry::new(
                                source_id.clone(),
                                source_name.clone(),
                                line.to_string(),
                                seq,
                            );
                            new_entries.push(entry);
                        }

                        // 存储条目
                        if !new_entries.is_empty() {
                            let mut entries_guard = entries.write().await;

                            // 添加新条目
                            for entry in &new_entries {
                                entries_guard.push(entry.clone());
                            }

                            // 轮转旧日志
                            if entries_guard.len() > 100000 {
                                let excess = entries_guard.len() - 100000;
                                entries_guard.drain(0..excess);
                            }

                            drop(entries_guard);

                            // 批量广播
                            let _ = broadcast_tx.send(LogMonitorMessage::BatchEntries {
                                entries: new_entries.clone(),
                            });

                            // 检查告警
                            let entries_read = entries.read().await;
                            let _recent: Vec<LogEntry> =
                                entries_read.iter().rev().take(100).cloned().collect();
                            drop(entries_read);

                            // 检查每条新条目的告警规则
                            // 注意：这里简化处理，实际应该异步检查
                        }
                    }
                    Err(e) => {
                        let _ = broadcast_tx.send(LogMonitorMessage::Error {
                            message: format!("Failed to fetch logs from {}: {}", source_name, e),
                        });
                    }
                }
                drop(ssh);

                tokio::time::sleep(Duration::from_millis(poll_interval)).await;
            }
        });

        let mut streams = self.active_streams.write().await;
        streams.insert(source.id.clone(), handle);
    }

    pub async fn search(&self, filter: &LogFilter) -> Vec<LogEntry> {
        let entries = self.entries.read().await;

        let mut results: Vec<LogEntry> = entries
            .iter()
            .filter(|e| e.matches_filter(filter))
            .cloned()
            .collect();

        // 按时间倒序
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        results
    }

    pub async fn search_text(&self, query: &str, limit: usize) -> Vec<LogEntry> {
        let filter = LogFilter::new()
            .with_keywords(vec![query.to_string()])
            .with_limit(limit);
        self.search(&filter).await
    }

    pub async fn search_regex(
        &self,
        pattern: &str,
        limit: usize,
    ) -> Result<Vec<LogEntry>, LiteError> {
        let _ =
            Regex::new(pattern).map_err(|e| LiteError::Config(format!("Invalid regex: {}", e)))?;

        let filter = LogFilter::new()
            .with_regex(pattern.to_string())
            .with_limit(limit);
        Ok(self.search(&filter).await)
    }

    pub async fn get_stats(&self, time_range_seconds: u64) -> LogStats {
        let entries = self.entries.read().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cutoff = now.saturating_sub(time_range_seconds);

        let filtered: Vec<&LogEntry> = entries.iter().filter(|e| e.timestamp >= cutoff).collect();

        let total = filtered.len() as u64;

        let mut by_level = HashMap::new();
        let mut by_source = HashMap::new();

        for entry in &filtered {
            *by_level.entry(entry.level.clone()).or_insert(0u64) += 1;
            *by_source.entry(entry.source_id.clone()).or_insert(0u64) += 1;
        }

        let error_count = *by_level.get(&LogLevel::ERROR).unwrap_or(&0)
            + *by_level.get(&LogLevel::FATAL).unwrap_or(&0);
        let error_rate = if total > 0 {
            (error_count as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let minutes = (time_range_seconds as f64 / 60.0).max(1.0);
        let entries_per_minute = total as f64 / minutes;

        // 生成时间序列
        let mut time_series = Vec::new();
        let intervals = 60u64; // 60个时间点
        let interval_size = time_range_seconds / intervals;

        for i in 0..intervals {
            let start = cutoff + (i * interval_size);
            let end = start + interval_size;

            let interval_entries: Vec<&LogEntry> = filtered
                .iter()
                .filter(|e| e.timestamp >= start && e.timestamp < end)
                .copied()
                .collect();

            let total_count = interval_entries.len() as u64;
            let error_count = interval_entries
                .iter()
                .filter(|e| e.level == LogLevel::ERROR || e.level == LogLevel::FATAL)
                .count() as u64;
            let warn_count = interval_entries
                .iter()
                .filter(|e| e.level == LogLevel::WARN)
                .count() as u64;
            let info_count = interval_entries
                .iter()
                .filter(|e| e.level == LogLevel::INFO)
                .count() as u64;

            time_series.push(TimeSeriesPoint {
                timestamp: start,
                total_count,
                error_count,
                warn_count,
                info_count,
            });
        }

        LogStats {
            total_entries: total,
            entries_by_level: by_level,
            entries_by_source: by_source,
            entries_per_minute,
            error_rate,
            top_error_patterns: Vec::new(),
            time_series,
        }
    }

    pub async fn analyze(&self, time_range_seconds: u64) -> LogAnalysisResult {
        let entries = self.entries.read().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cutoff = now.saturating_sub(time_range_seconds);

        let filtered: Vec<&LogEntry> = entries
            .iter()
            .filter(|e| {
                e.timestamp >= cutoff && (e.level == LogLevel::ERROR || e.level == LogLevel::FATAL)
            })
            .collect();

        // 错误模式识别
        let mut pattern_counts: HashMap<String, Vec<&LogEntry>> = HashMap::new();

        // 常见的错误模式
        let error_patterns = [
            r"Exception[:\s]+(\w+)",
            r"Error[:\s]+(\w+)",
            r"Failed to (\w+)",
            r"Cannot (\w+)",
            r"Unable to (\w+)",
            r"panic[:\s]+(.+)",
            r"timeout",
            r"connection refused",
            r"permission denied",
            r"not found",
            r"invalid",
        ];

        for entry in &filtered {
            for pattern in &error_patterns {
                if let Ok(re) = Regex::new(pattern) {
                    if re.is_match(&entry.message) {
                        pattern_counts
                            .entry(pattern.to_string())
                            .or_default()
                            .push(entry);
                    }
                }
            }
        }

        let mut patterns: Vec<ErrorPattern> = pattern_counts
            .iter()
            .map(|(p, entries)| {
                let sorted: Vec<&&LogEntry> = entries.iter().map(|e| e).collect();
                ErrorPattern {
                    pattern: p.clone(),
                    count: entries.len() as u64,
                    sample_message: sorted
                        .first()
                        .map(|e| e.message.clone())
                        .unwrap_or_default(),
                    first_seen: sorted.first().map(|e| e.timestamp).unwrap_or(0),
                    last_seen: sorted.last().map(|e| e.timestamp).unwrap_or(0),
                }
            })
            .collect();

        patterns.sort_by(|a, b| b.count.cmp(&a.count));
        patterns.truncate(10);

        // 异常检测
        let mut anomalies = Vec::new();

        // 检测错误率突增
        let error_spike_threshold = 100; // 100秒内超过100个错误
        let window_size = 100u64;

        for i in (cutoff..=now).step_by(window_size as usize) {
            let window_errors: Vec<&&LogEntry> = filtered
                .iter()
                .filter(|e| e.timestamp >= i && e.timestamp < i + window_size)
                .collect();

            if window_errors.len() > error_spike_threshold as usize {
                anomalies.push(Anomaly {
                    timestamp: i,
                    description: format!(
                        "错误激增: {} 个错误在 {} 秒内",
                        window_errors.len(),
                        window_size
                    ),
                    severity: LogLevel::ERROR,
                    related_entries: window_errors
                        .iter()
                        .take(10)
                        .map(|e| e.id.clone())
                        .collect(),
                });
            }
        }

        // 趋势分析
        let mut trends = Vec::new();
        let mid_point = cutoff + (time_range_seconds / 2);

        let first_half_errors = filtered.iter().filter(|e| e.timestamp < mid_point).count() as f64;
        let second_half_errors =
            filtered.iter().filter(|e| e.timestamp >= mid_point).count() as f64;

        if first_half_errors > 0.0 {
            let change = ((second_half_errors - first_half_errors) / first_half_errors) * 100.0;
            let direction = if change > 10.0 {
                LogTrendDirection::Increasing
            } else if change < -10.0 {
                LogTrendDirection::Decreasing
            } else {
                LogTrendDirection::Stable
            };

            trends.push(Trend {
                metric: "错误率".to_string(),
                direction,
                change_percent: change.abs(),
                period_seconds: time_range_seconds,
            });
        }

        LogAnalysisResult {
            patterns,
            anomalies,
            trends,
        }
    }

    pub async fn export(&self, config: &ExportConfig, output_path: &str) -> Result<(), LiteError> {
        let entries = self.search(&config.filter).await;

        let output = match config.format {
            ExportFormat::JSON => serde_json::to_string_pretty(&entries)
                .map_err(|e| LiteError::Config(format!("JSON serialize error: {}", e)))?,
            ExportFormat::CSV => {
                let mut wtr = csv::Writer::from_path(output_path)
                    .map_err(|e| LiteError::Io(e.to_string()))?;

                for entry in entries {
                    wtr.write_record(&[
                        entry.id,
                        entry.timestamp.to_string(),
                        format!("{:?}", entry.level),
                        entry.source_name,
                        if config.include_raw {
                            entry.raw_line
                        } else {
                            entry.message
                        },
                    ])
                    .map_err(|e| LiteError::Io(e.to_string()))?;
                }
                wtr.flush().map_err(|e| LiteError::Io(e.to_string()))?;
                return Ok(());
            }
            ExportFormat::PlainText => entries
                .iter()
                .map(|e| {
                    format!(
                        "[{}] [{}] {}: {}",
                        e.timestamp,
                        format!("{:?}", e.level),
                        e.source_name,
                        if config.include_raw {
                            &e.raw_line
                        } else {
                            &e.message
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            ExportFormat::HTML => {
                let mut html = String::from("<html><head><style>");
                html.push_str("body{font-family:monospace;padding:20px;}");
                html.push_str(".entry{margin:2px 0;padding:4px;border-bottom:1px solid #eee;}");
                html.push_str(".ERROR{color:#dc3545;font-weight:bold;}");
                html.push_str(".WARN{color:#ffc107;}");
                html.push_str(".INFO{color:#198754;}");
                html.push_str(".DEBUG{color:#0d6efd;}");
                html.push_str("</style></head><body>");
                html.push_str("<h1>日志导出</h1>");
                html.push_str("<table width='100%'>");
                html.push_str("<tr><th>时间</th><th>级别</th><th>源</th><th>消息</th></tr>");

                for entry in entries {
                    html.push_str(&format!(
                        "<tr class='entry {}'><td>{}</td><td>{:?}</td><td>{}</td><td>{}</td></tr>",
                        format!("{:?}", entry.level),
                        entry.timestamp,
                        entry.level,
                        entry.source_name,
                        html_escape(&if config.include_raw {
                            entry.raw_line
                        } else {
                            entry.message
                        })
                    ));
                }

                html.push_str("</table></body></html>");
                html
            }
        };

        if config.compress {
            let encoder = flate2::write::GzEncoder::new(
                std::fs::File::create(format!("{}.gz", output_path))
                    .map_err(|e| LiteError::Io(e.to_string()))?,
                flate2::Compression::default(),
            );
            use std::io::Write;
            let mut encoder = encoder;
            encoder
                .write_all(output.as_bytes())
                .map_err(|e| LiteError::Io(e.to_string()))?;
            encoder.finish().map_err(|e| LiteError::Io(e.to_string()))?;
        } else {
            std::fs::write(output_path, output).map_err(|e| LiteError::Io(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn rotate_logs(&self) -> usize {
        let mut entries = self.entries.write().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cutoff = now.saturating_sub(self.retention_hours * 3600);

        let original_len = entries.len();
        entries.retain(|e| e.timestamp >= cutoff);
        let removed = original_len - entries.len();

        // 限制最大条目数
        if entries.len() > self.max_entries {
            let excess = entries.len() - self.max_entries;
            entries.drain(0..excess);
            removed + excess
        } else {
            removed
        }
    }

    pub async fn start_rotation_task(&self) {
        let entries = self.entries.clone();
        let retention_hours = self.retention_hours;
        let max_entries = self.max_entries;
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // 每小时检查一次

            loop {
                interval.tick().await;

                if !running.load(Ordering::Relaxed) {
                    break;
                }

                let mut entries_guard = entries.write().await;
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let cutoff = now.saturating_sub(retention_hours * 3600);

                let original_len = entries_guard.len();
                entries_guard.retain(|e| e.timestamp >= cutoff);

                if entries_guard.len() > max_entries {
                    let excess = entries_guard.len() - max_entries;
                    entries_guard.drain(0..excess);
                }

                log::info!(
                    "Log rotation: removed {} old entries",
                    original_len - entries_guard.len()
                );
            }
        });
    }

    pub async fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);

        let mut streams = self.active_streams.write().await;
        for (_, handle) in streams.drain() {
            handle.abort();
        }
    }

    pub async fn get_recent_entries(&self, count: usize) -> Vec<LogEntry> {
        let entries = self.entries.read().await;
        entries.iter().rev().take(count).cloned().collect()
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ============================================================================
// WebSocket服务器
// ============================================================================

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

pub struct LogMonitorWebSocketServer {
    center: Arc<LogMonitorCenter>,
    port: u16,
}

impl LogMonitorWebSocketServer {
    pub fn new(center: Arc<LogMonitorCenter>, port: u16) -> Self {
        Self { center, port }
    }

    pub async fn start(&self) -> Result<(), LiteError> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| LiteError::Io(format!("Failed to bind: {}", e)))?;

        log::info!("Log monitor WebSocket server started on {}", addr);

        let center = self.center.clone();

        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let center = center.clone();
                tokio::spawn(handle_connection(stream, center));
            }
        });

        Ok(())
    }
}

async fn handle_connection(stream: TcpStream, center: Arc<LogMonitorCenter>) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            log::error!("WebSocket accept error: {}", e);
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut broadcast_rx = center.subscribe();

    // 发送初始连接确认
    let confirm = serde_json::json!({
        "type": "connected",
        "message": "Log monitor WebSocket connected"
    });
    let _ = ws_sender.send(Message::Text(confirm.to_string())).await;

    // 处理消息循环
    loop {
        tokio::select! {
            // 接收广播消息
            Ok(msg) = broadcast_rx.recv() => {
                let json = serde_json::to_string(&msg).unwrap_or_default();
                if ws_sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }

            // 接收客户端消息
            Some(Ok(msg)) = ws_receiver.next() => {
                match msg {
                    Message::Text(text) => {
                        if let Ok(cmd) = serde_json::from_str::<ClientCommand>(&text) {
                            handle_client_command(cmd, &center, &mut ws_sender).await;
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct ClientCommand {
    action: String,
    #[serde(flatten)]
    params: serde_json::Value,
}

async fn handle_client_command(
    cmd: ClientCommand,
    center: &LogMonitorCenter,
    sender: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        Message,
    >,
) {
    match cmd.action.as_str() {
        "search" => {
            if let Ok(filter) = serde_json::from_value::<LogFilter>(cmd.params) {
                let results = center.search(&filter).await;
                let response = serde_json::json!({
                    "type": "search_results",
                    "entries": results
                });
                let _ = sender.send(Message::Text(response.to_string())).await;
            }
        }
        "stats" => {
            let range = cmd
                .params
                .get("range_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(3600);
            let stats = center.get_stats(range).await;
            let response = serde_json::json!({
                "type": "stats",
                "stats": stats
            });
            let _ = sender.send(Message::Text(response.to_string())).await;
        }
        "analyze" => {
            let range = cmd
                .params
                .get("range_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(3600);
            let analysis = center.analyze(range).await;
            let response = serde_json::json!({
                "type": "analysis",
                "result": analysis
            });
            let _ = sender.send(Message::Text(response.to_string())).await;
        }
        "get_sources" => {
            let sources = center.get_sources().await;
            let response = serde_json::json!({
                "type": "sources",
                "sources": sources
            });
            let _ = sender.send(Message::Text(response.to_string())).await;
        }
        "get_alerts" => {
            let alerts = center.get_alert_rules().await;
            let response = serde_json::json!({
                "type": "alert_rules",
                "rules": alerts
            });
            let _ = sender.send(Message::Text(response.to_string())).await;
        }
        _ => {}
    }
}

use futures_util::{SinkExt, StreamExt};

// ============================================================================
// FFI导出
// ============================================================================

#[no_mangle]
pub extern "C" fn log_monitor_center_create(
    ssh_manager_ptr: *mut Arc<RwLock<SshSessionManager>>,
) -> *mut LogMonitorCenter {
    if ssh_manager_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let ssh_manager = unsafe { &*ssh_manager_ptr };
    let center = Box::new(LogMonitorCenter::new(ssh_manager.clone()));
    Box::into_raw(center)
}

#[no_mangle]
pub extern "C" fn log_monitor_center_destroy(center_ptr: *mut LogMonitorCenter) {
    if !center_ptr.is_null() {
        unsafe { drop(Box::from_raw(center_ptr)) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("ERROR"), LogLevel::ERROR);
        assert_eq!(LogLevel::from_str("error"), LogLevel::ERROR);
        assert_eq!(LogLevel::from_str("WARN"), LogLevel::WARN);
        assert_eq!(LogLevel::from_str("warning"), LogLevel::WARN);
        assert_eq!(LogLevel::from_str("INFO"), LogLevel::INFO);
        assert_eq!(LogLevel::from_str("DEBUG"), LogLevel::DEBUG);
    }

    #[test]
    fn test_log_level_color() {
        assert_eq!(LogLevel::ERROR.color(), "#dc3545");
        assert_eq!(LogLevel::WARN.color(), "#ffc107");
        assert_eq!(LogLevel::INFO.color(), "#198754");
    }

    #[test]
    fn test_log_level_priority() {
        assert!(LogLevel::ERROR.priority() > LogLevel::WARN.priority());
        assert!(LogLevel::WARN.priority() > LogLevel::INFO.priority());
        assert!(LogLevel::INFO.priority() > LogLevel::DEBUG.priority());
    }

    #[test]
    fn test_log_entry_detect_level() {
        let entry = LogEntry::new(
            "src1".to_string(),
            "Test".to_string(),
            "2024-01-01 10:00:00 ERROR Something failed".to_string(),
            1,
        );
        assert_eq!(entry.level, LogLevel::ERROR);

        let entry2 = LogEntry::new(
            "src1".to_string(),
            "Test".to_string(),
            "2024-01-01 10:00:00 WARN Low disk space".to_string(),
            2,
        );
        assert_eq!(entry2.level, LogLevel::WARN);
    }

    #[test]
    fn test_alert_rule_keyword_match() {
        let mut rule = LogAlertRule::new(
            "Test Alert".to_string(),
            LogAlertCondition::KeywordMatch {
                keywords: vec!["error".to_string(), "failed".to_string()],
                case_sensitive: false,
            },
        );

        let entry = LogEntry::new(
            "src1".to_string(),
            "Test".to_string(),
            "Connection failed".to_string(),
            1,
        );

        assert!(rule.check(&entry, &[]).is_some());

        let entry2 = LogEntry::new(
            "src1".to_string(),
            "Test".to_string(),
            "Everything is fine".to_string(),
            2,
        );

        assert!(rule.check(&entry2, &[]).is_none());
    }

    #[test]
    fn test_log_filter() {
        let filter = LogFilter::new()
            .with_min_level(LogLevel::WARN)
            .with_keywords(vec!["error".to_string()]);

        let entry1 = LogEntry::new(
            "src1".to_string(),
            "Test".to_string(),
            "ERROR connection failed".to_string(),
            1,
        );
        assert!(entry1.matches_filter(&filter));

        let entry2 = LogEntry::new(
            "src1".to_string(),
            "Test".to_string(),
            "DEBUG some info".to_string(),
            2,
        );
        assert!(!entry2.matches_filter(&filter));
    }
}
