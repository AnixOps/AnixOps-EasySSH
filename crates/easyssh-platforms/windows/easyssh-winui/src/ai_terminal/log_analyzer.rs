//! 智能日志分析模块
//!
//! 自动识别日志中的错误模式

#![allow(dead_code)]

use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::ai_terminal::providers::AiProvider;
use crate::ai_terminal::providers::{ChatRequest, Message, Role};

/// 日志分析请求
#[derive(Debug, Clone)]
pub struct LogAnalysisRequest {
    pub log_content: String,
    pub log_type: Option<LogType>,
    pub max_issues: usize,
    pub time_range: Option<(String, String)>, // (start, end)
}

/// 日志类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogType {
    System,      // /var/log/syslog, messages
    Application, // Application logs
    WebServer,   // nginx, apache
    Database,    // PostgreSQL, MySQL
    Container,   // Docker, Kubernetes
    Security,    // Auth logs
    Custom,      // User-defined
}

impl LogType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Application => "application",
            Self::WebServer => "webserver",
            Self::Database => "database",
            Self::Container => "container",
            Self::Security => "security",
            Self::Custom => "custom",
        }
    }

    pub fn common_patterns(&self) -> Vec<(&'static str, &'static str, LogSeverity)> {
        match self {
            Self::System => vec![
                (r"(?i)kernel.*error", "Kernel error", LogSeverity::High),
                (r"(?i)out of memory", "Out of memory", LogSeverity::Critical),
                (
                    r"(?i)oom killer",
                    "OOM killer triggered",
                    LogSeverity::Critical,
                ),
                (
                    r"(?i)segmentation fault",
                    "Segmentation fault",
                    LogSeverity::High,
                ),
                (r"(?i)segfault", "Segmentation fault", LogSeverity::High),
                (r"(?i)panic", "System panic", LogSeverity::Critical),
                (
                    r"(?i)fail|failed|failure",
                    "Operation failed",
                    LogSeverity::Medium,
                ),
                (r"(?i)warning", "Warning", LogSeverity::Low),
            ],
            Self::WebServer => vec![
                (r"(?i)404", "Not found error", LogSeverity::Low),
                (r"(?i)500", "Internal server error", LogSeverity::High),
                (r"(?i)502", "Bad gateway", LogSeverity::High),
                (r"(?i)503", "Service unavailable", LogSeverity::High),
                (
                    r"(?i)connection timeout",
                    "Connection timeout",
                    LogSeverity::Medium,
                ),
                (r"(?i)upstream.*error", "Upstream error", LogSeverity::High),
            ],
            Self::Database => vec![
                (r"(?i)deadlock", "Deadlock detected", LogSeverity::High),
                (r"(?i)lock.*timeout", "Lock timeout", LogSeverity::Medium),
                (
                    r"(?i)connection.*refused",
                    "Connection refused",
                    LogSeverity::High,
                ),
                (
                    r"(?i)too many connections",
                    "Too many connections",
                    LogSeverity::High,
                ),
                (r"(?i)disk full", "Disk full", LogSeverity::Critical),
                (r"(?i)corruption", "Data corruption", LogSeverity::Critical),
            ],
            Self::Container => vec![
                (r"(?i)crashloopbackoff", "Crash loop", LogSeverity::Critical),
                (
                    r"(?i)imagepullbackoff",
                    "Image pull failed",
                    LogSeverity::High,
                ),
                (r"(?i)evicted", "Pod evicted", LogSeverity::High),
                (
                    r"(?i)outofmemory",
                    "Container OOM killed",
                    LogSeverity::Critical,
                ),
                (r"(?i)outofcpu", "CPU throttled", LogSeverity::Medium),
                (r"(?i)unhealthy", "Health check failed", LogSeverity::High),
            ],
            Self::Security => vec![
                (
                    r"(?i)authentication failure",
                    "Auth failure",
                    LogSeverity::High,
                ),
                (r"(?i)failed password", "Failed password", LogSeverity::High),
                (
                    r"(?i)invalid user",
                    "Invalid user login attempt",
                    LogSeverity::High,
                ),
                (
                    r"(?i)brute force",
                    "Possible brute force",
                    LogSeverity::Critical,
                ),
                (
                    r"(?i)privilege escalation",
                    "Privilege escalation",
                    LogSeverity::Critical,
                ),
                (r"(?i)root login", "Root login attempt", LogSeverity::High),
            ],
            _ => vec![
                (
                    r"(?i)error|exception",
                    "Error/Exception",
                    LogSeverity::Medium,
                ),
                (r"(?i)warning|warn", "Warning", LogSeverity::Low),
                (r"(?i)fatal|critical", "Fatal error", LogSeverity::Critical),
            ],
        }
    }
}

/// 日志严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogSeverity {
    Info,     // 信息性
    Low,      // 轻微问题
    Medium,   // 中等问题
    High,     // 严重问题
    Critical, // 关键问题
}

impl LogSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::Info => "#3498db",
            Self::Low => "#f1c40f",
            Self::Medium => "#e67e22",
            Self::High => "#e74c3c",
            Self::Critical => "#c0392b",
        }
    }
}

/// 日志分析结果
#[derive(Debug, Clone)]
pub struct LogAnalysisResult {
    pub summary: LogSummary,
    pub issues: Vec<LogIssue>,
    pub patterns: Vec<LogPattern>,
    pub trends: Vec<LogTrend>,
    pub recommendations: Vec<String>,
}

/// 日志摘要
#[derive(Debug, Clone)]
pub struct LogSummary {
    pub total_lines: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub time_range: Option<(String, String)>,
    pub most_common_errors: Vec<(String, usize)>,
}

/// 日志问题
#[derive(Debug, Clone)]
pub struct LogIssue {
    pub timestamp: Option<String>,
    pub severity: LogSeverity,
    pub category: String,
    pub message: String,
    pub context: Vec<String>,
    pub line_number: Option<usize>,
    pub suggested_action: Option<String>,
}

/// 日志模式
#[derive(Debug, Clone)]
pub struct LogPattern {
    pub pattern_name: String,
    pub occurrences: usize,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
    pub severity: LogSeverity,
    pub sample_lines: Vec<String>,
}

/// 日志趋势
#[derive(Debug, Clone)]
pub struct LogTrend {
    pub metric: String,
    pub direction: TrendDirection,
    pub change_percentage: f32,
    pub description: String,
}

/// 趋势方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
    Spiking,
}

/// 日志分析器
pub struct LogAnalyzer {
    provider: Arc<dyn AiProvider>,
    pattern_cache: RwLock<HashMap<String, Regex>>,
}

impl LogAnalyzer {
    pub fn new(provider: Arc<dyn AiProvider>) -> Self {
        Self {
            provider,
            pattern_cache: RwLock::new(HashMap::new()),
        }
    }

    /// 分析日志
    pub async fn analyze(&self, request: &LogAnalysisRequest) -> Result<LogAnalysisResult> {
        // 首先进行模式匹配分析
        let pattern_result = self.pattern_analysis(request);

        // 如果模式分析发现问题较多，使用AI进行深度分析
        if pattern_result.issues.len() >= 3 || request.log_content.len() > 10000 {
            let ai_result = self.ai_analysis(request).await?;

            // 合并结果
            return Ok(self.merge_results(pattern_result, ai_result));
        }

        Ok(pattern_result)
    }

    /// 基于模式的日志分析
    fn pattern_analysis(&self, request: &LogAnalysisRequest) -> LogAnalysisResult {
        let log_type = request.log_type.unwrap_or(LogType::System);
        let patterns = log_type.common_patterns();
        let lines: Vec<&str> = request.log_content.lines().collect();

        let mut issues = Vec::new();
        let mut pattern_counts: HashMap<String, (usize, Vec<String>)> = HashMap::new();
        let mut error_count = 0usize;
        let mut warning_count = 0usize;
        let mut info_count = 0usize;

        for (line_num, line) in lines.iter().enumerate() {
            let line_lower = line.to_lowercase();

            // 统计严重程度
            if line_lower.contains("error")
                || line_lower.contains("exception")
                || line_lower.contains("fatal")
            {
                error_count += 1;
            } else if line_lower.contains("warning") || line_lower.contains("warn") {
                warning_count += 1;
            } else if line_lower.contains("info") {
                info_count += 1;
            }

            // 模式匹配
            for (pattern_str, category, severity) in &patterns {
                let regex = self.get_or_compile_regex(pattern_str);
                if regex.is_match(line) {
                    // 记录问题
                    let issue = LogIssue {
                        timestamp: self.extract_timestamp(line),
                        severity: *severity,
                        category: category.to_string(),
                        message: line.to_string(),
                        context: self.get_context(&lines, line_num, 2),
                        line_number: Some(line_num + 1),
                        suggested_action: self.get_suggested_action(category),
                    };
                    issues.push(issue);

                    // 统计模式
                    let entry = pattern_counts
                        .entry(category.to_string())
                        .or_insert_with(|| (0, Vec::new()));
                    entry.0 += 1;
                    if entry.1.len() < 3 {
                        entry.1.push(line.to_string());
                    }
                }
            }
        }

        // 创建日志模式
        let mut log_patterns: Vec<LogPattern> = pattern_counts
            .into_iter()
            .map(|(name, (count, samples))| LogPattern {
                pattern_name: name.clone(),
                occurrences: count,
                first_seen: None,
                last_seen: None,
                severity: self.get_pattern_severity(&name),
                sample_lines: samples,
            })
            .collect();

        // 按严重程度排序
        log_patterns.sort_by(|a, b| b.severity.cmp(&a.severity));

        // 计算最常见错误
        let mut common_errors: Vec<(String, usize)> = log_patterns
            .iter()
            .map(|p| (p.pattern_name.clone(), p.occurrences))
            .collect();
        common_errors.sort_by(|a, b| b.1.cmp(&a.1));
        common_errors.truncate(5);

        // 生成摘要
        let summary = LogSummary {
            total_lines: lines.len(),
            error_count,
            warning_count,
            info_count,
            time_range: request.time_range.clone(),
            most_common_errors: common_errors,
        };

        // 生成建议
        let recommendations = self.generate_recommendations(&log_patterns);

        LogAnalysisResult {
            summary,
            issues: issues.into_iter().take(request.max_issues).collect(),
            patterns: log_patterns,
            trends: vec![], // 模式分析不处理趋势
            recommendations,
        }
    }

    /// AI深度分析
    async fn ai_analysis(&self, request: &LogAnalysisRequest) -> Result<LogAnalysisResult> {
        let system_prompt = r#"You are a log analysis expert.
Analyze the provided log content and identify issues, patterns, and trends.

Format your response as:
SUMMARY:
Total lines: <count>
Errors: <count>
Warnings: <count>
Time range: <range>

ISSUES:
SEVERITY: <critical/high/medium/low>
CATEGORY: <category>
TIMESTAMP: <timestamp or N/A>
MESSAGE: <issue message>
CONTEXT: <context lines>
ACTION: <suggested action>

PATTERNS:
PATTERN: <pattern name>
COUNT: <occurrences>
SEVERITY: <level>
SAMPLES: <sample lines>

RECOMMENDATIONS:
- <recommendation 1>
- <recommendation 2>"#;

        let log_preview = if request.log_content.len() > 8000 {
            &request.log_content[..8000]
        } else {
            &request.log_content
        };

        let user_prompt = format!(
            "Log type: {}\n\nAnalyze this log:\n{}\n\nProvide analysis:",
            request.log_type.unwrap_or(LogType::System).as_str(),
            log_preview
        );

        let chat_request = ChatRequest {
            messages: vec![
                Message {
                    role: Role::System,
                    content: system_prompt.to_string(),
                },
                Message {
                    role: Role::User,
                    content: user_prompt,
                },
            ],
            max_tokens: 2000,
            temperature: 0.2,
            stream: false,
        };

        let response = self.provider.chat(chat_request).await?;
        let result = self.parse_ai_response(&response.content, request);

        Ok(result)
    }

    /// 解析AI响应
    fn parse_ai_response(&self, _content: &str, request: &LogAnalysisRequest) -> LogAnalysisResult {
        // 这是一个简化实现
        // 实际应该完整解析AI响应

        let lines: Vec<&str> = request.log_content.lines().collect();

        LogAnalysisResult {
            summary: LogSummary {
                total_lines: lines.len(),
                error_count: 0,
                warning_count: 0,
                info_count: 0,
                time_range: request.time_range.clone(),
                most_common_errors: vec![],
            },
            issues: vec![],
            patterns: vec![],
            trends: vec![],
            recommendations: vec!["Review log for detailed analysis".to_string()],
        }
    }

    /// 合并模式分析和AI分析结果
    fn merge_results(
        &self,
        pattern: LogAnalysisResult,
        _ai: LogAnalysisResult,
    ) -> LogAnalysisResult {
        // 优先使用模式分析的结果，因为它更准确
        pattern
    }

    /// 获取或编译正则
    fn get_or_compile_regex(&self, pattern: &str) -> Regex {
        if let Ok(cache) = self.pattern_cache.read() {
            if let Some(regex) = cache.get(pattern) {
                return regex.clone();
            }
        }
        let regex = Regex::new(pattern).unwrap_or_else(|_| Regex::new(".*").unwrap());
        if let Ok(mut cache) = self.pattern_cache.write() {
            cache.insert(pattern.to_string(), regex.clone());
        }
        regex
    }

    /// 提取时间戳
    fn extract_timestamp(&self, line: &str) -> Option<String> {
        // 常见时间戳格式
        let timestamp_patterns = [
            r"(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})",
            r"(\w{3}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})",
            r"(\d{2}/\w{3}/\d{4}:\d{2}:\d{2}:\d{2})",
            r"(\d{10}\.\d+)", // Unix timestamp
        ];

        for pattern in &timestamp_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(caps) = regex.captures(line) {
                    return caps.get(1).map(|m| m.as_str().to_string());
                }
            }
        }

        None
    }

    /// 获取上下文
    fn get_context(&self, lines: &[&str], index: usize, context_lines: usize) -> Vec<String> {
        let start = index.saturating_sub(context_lines);
        let end = (index + context_lines + 1).min(lines.len());

        lines[start..end].iter().map(|&s| s.to_string()).collect()
    }

    /// 获取建议操作
    fn get_suggested_action(&self, category: &str) -> Option<String> {
        let actions: HashMap<&str, &str> = [
            (
                "Out of memory",
                "Check memory usage with 'free -h' and consider killing processes or adding swap",
            ),
            (
                "Kernel error",
                "Check kernel logs with 'dmesg' and consider system updates",
            ),
            (
                "Segmentation fault",
                "Check application logs and consider updating or debugging the application",
            ),
            (
                "Not found error",
                "Verify the resource exists and check paths/URLs",
            ),
            (
                "Internal server error",
                "Check application logs and restart services if needed",
            ),
            (
                "Deadlock detected",
                "Review database transactions and connection pooling settings",
            ),
            (
                "Too many connections",
                "Increase connection limits or implement connection pooling",
            ),
            (
                "Disk full",
                "Free up disk space by removing old files or expanding storage",
            ),
            (
                "Crash loop",
                "Check container logs and resource limits, fix underlying issue",
            ),
            (
                "OOM killed",
                "Increase memory limits for the container or optimize application",
            ),
            (
                "Auth failure",
                "Review authentication configuration and check for brute force attacks",
            ),
            (
                "Invalid user login attempt",
                "Review user accounts and check for unauthorized access attempts",
            ),
        ]
        .iter()
        .cloned()
        .collect();

        actions.get(category).map(|&s| s.to_string())
    }

    /// 获取模式严重程度
    fn get_pattern_severity(&self, pattern_name: &str) -> LogSeverity {
        match pattern_name {
            "Out of memory"
            | "OOM killer triggered"
            | "System panic"
            | "Fatal error"
            | "Disk full"
            | "Data corruption"
            | "Crash loop"
            | "Container OOM killed"
            | "Possible brute force"
            | "Privilege escalation" => LogSeverity::Critical,

            "Kernel error"
            | "Segmentation fault"
            | "Internal server error"
            | "Bad gateway"
            | "Service unavailable"
            | "Deadlock detected"
            | "Too many connections"
            | "Image pull failed"
            | "Pod evicted"
            | "Health check failed"
            | "Auth failure"
            | "Failed password"
            | "Root login attempt" => LogSeverity::High,

            "Operation failed"
            | "Connection timeout"
            | "Lock timeout"
            | "Connection refused"
            | "Upstream error"
            | "CPU throttled"
            | "Invalid user login attempt" => LogSeverity::Medium,

            _ => LogSeverity::Low,
        }
    }

    /// 生成建议
    fn generate_recommendations(&self, patterns: &[LogPattern]) -> Vec<String> {
        let mut recommendations = Vec::new();

        // 检查是否存在严重问题
        let critical_count = patterns
            .iter()
            .filter(|p| p.severity == LogSeverity::Critical)
            .count();

        if critical_count > 0 {
            recommendations.push(format!(
                "URGENT: Found {} critical issues requiring immediate attention",
                critical_count
            ));
        }

        // 检查OOM模式
        let oom_patterns: Vec<_> = patterns
            .iter()
            .filter(|p| p.pattern_name.contains("memory") || p.pattern_name.contains("OOM"))
            .collect();

        if !oom_patterns.is_empty() {
            recommendations.push(
                "Memory issues detected: Consider increasing memory limits or optimizing applications".to_string()
            );
        }

        // 检查认证失败
        let auth_patterns: Vec<_> = patterns
            .iter()
            .filter(|p| p.pattern_name.contains("auth") || p.pattern_name.contains("login"))
            .collect();

        if !auth_patterns.is_empty() {
            recommendations.push(
                "Authentication issues detected: Review security settings and check for brute force attacks".to_string()
            );
        }

        // 通用建议
        if patterns.len() > 5 {
            recommendations.push(
                "High number of error patterns: Consider a comprehensive system review".to_string(),
            );
        }

        recommendations
    }
}

/// 分析日志的主函数
pub async fn analyze_logs(
    provider: &Arc<dyn AiProvider>,
    request: &LogAnalysisRequest,
) -> Result<LogAnalysisResult> {
    let analyzer = LogAnalyzer::new(Arc::clone(provider));
    analyzer.analyze(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_terminal::providers::MockProvider;

    #[test]
    fn test_log_severity_ordering() {
        assert!(LogSeverity::Critical > LogSeverity::High);
        assert!(LogSeverity::High > LogSeverity::Medium);
        assert!(LogSeverity::Medium > LogSeverity::Low);
        assert!(LogSeverity::Low > LogSeverity::Info);
    }

    #[test]
    fn test_timestamp_extraction() {
        let analyzer = LogAnalyzer::new(Arc::new(MockProvider::new()));

        let line1 = "2024-01-15 10:30:45 Server started";
        assert!(analyzer.extract_timestamp(line1).is_some());

        let line2 = "Jan 15 10:30:45 Server started";
        assert!(analyzer.extract_timestamp(line2).is_some());
    }

    #[test]
    fn test_common_patterns() {
        let patterns = LogType::System.common_patterns();
        assert!(!patterns.is_empty());

        let web_patterns = LogType::WebServer.common_patterns();
        assert!(!web_patterns.is_empty());
    }
}
