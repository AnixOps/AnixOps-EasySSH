//! Debug类型定义
//!
//! 共享的数据类型和结构定义

/// 健康状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub timestamp: String,
    pub debug_enabled: bool,
    pub access_level: String,
}

/// 测试结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestResult {
    pub success: bool,
    pub output: String,
    pub errors: String,
    pub duration_ms: u64,
}

/// 构建结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildResult {
    pub success: bool,
    pub output: String,
    pub errors: String,
    pub duration_ms: u64,
}

/// 类型检查结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeCheckResult {
    pub success: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Lint结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LintResult {
    pub success: bool,
    pub issues: Vec<LintIssue>,
    pub fixed_count: usize,
}

/// Lint问题
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LintIssue {
    pub file: String,
    pub line: usize,
    pub severity: String, // "error", "warning", "info"
    pub message: String,
    pub rule: String,
}

/// 搜索代码结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub file: String,
    pub line_number: usize,
    pub line_content: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

/// 文件信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified: String,
    pub is_directory: bool,
}

/// 编辑结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditResult {
    pub success: bool,
    pub message: String,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
}

/// Git状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitStatus {
    pub is_dirty: bool,
    pub staged_files: Vec<String>,
    pub unstaged_files: Vec<String>,
    pub untracked_files: Vec<String>,
    pub current_branch: String,
    pub ahead: usize,
    pub behind: usize,
}

/// Git分支
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitBranch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
}

/// Git提交
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub date: String,
}

/// 性能指标
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceMetrics {
    pub cpu_usage: f64,
    pub memory_usage_mb: f64,
    pub memory_total_mb: f64,
    pub disk_usage_gb: f64,
    pub disk_total_gb: f64,
    pub network_latency_ms: Option<f64>,
    pub timestamp: String,
}

/// Debug功能清单
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DebugCapabilities {
    pub ai_programming: bool,
    pub performance_monitoring: bool,
    pub network_check: bool,
    pub database_console: bool,
    pub log_viewer: bool,
    pub test_runner: bool,
    pub feature_flags: bool,
    pub audit_logs: bool,
}

/// 网络检查结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkCheckResult {
    pub host: String,
    pub port: u16,
    pub reachable: bool,
    pub latency_ms: Option<f64>,
    pub error: Option<String>,
}

/// 日志条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String, // "ERROR", "WARN", "INFO", "DEBUG", "TRACE"
    pub target: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
}

/// 特性开关状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeatureFlag {
    pub name: String,
    pub enabled: bool,
    pub description: String,
    pub modified_at: String,
}

/// 调试测试报告（兼容旧版）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DebugTestReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<DebugTestResult>,
    pub duration_ms: u64,
}

/// 调试测试结果（兼容旧版）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DebugTestResult {
    pub name: String,
    pub category: String,
    pub passed: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    pub duration_ms: Option<u64>,
}

/// AI Agent权限配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentPermissions {
    pub read_files: bool,
    pub write_files: bool,
    pub run_tests: bool,
    pub run_commands: bool,
    pub git_operations: bool,
    pub create_commits: bool,
    pub max_iterations: usize,
    pub requires_approval: bool,
}

impl Default for AgentPermissions {
    fn default() -> Self {
        Self {
            read_files: true,
            write_files: true,
            run_tests: true,
            run_commands: true,
            git_operations: true,
            create_commits: false,     // 禁止自动提交
            max_iterations: 5,
            requires_approval: true,   // 默认需要批准
        }
    }
}

impl AgentPermissions {
    /// 创建保守权限配置
    pub fn conservative() -> Self {
        Self {
            read_files: true,
            write_files: false,        // 需要批准
            run_tests: true,
            run_commands: false,       // 需要批准
            git_operations: false,     // 需要批准
            create_commits: false,
            max_iterations: 3,
            requires_approval: true,
        }
    }

    /// 创建开发权限配置
    pub fn developer() -> Self {
        Self {
            read_files: true,
            write_files: true,
            run_tests: true,
            run_commands: true,
            git_operations: true,
            create_commits: false,     // 始终禁止自动提交
            max_iterations: 10,
            requires_approval: false,
        }
    }
}

/// 自我修复结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SelfFixResult {
    pub success: bool,
    pub iterations: usize,
    pub attempts: Vec<FixAttempt>,
    pub error: Option<String>,
}

/// 修复尝试记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FixAttempt {
    pub iteration: usize,
    pub strategy: String,
    pub files_modified: Vec<String>,
    pub status: String,
    pub error: Option<String>,
}

/// 任务执行结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub task_id: String,
    pub output: String,
    pub duration_ms: u64,
    pub steps_completed: usize,
    pub steps_total: usize,
}

/// 代码理解结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeUnderstanding {
    pub symbol: String,
    pub file: String,
    pub line: usize,
    pub doc_comment: Option<String>,
    pub signature: String,
    pub usages: Vec<CodeUsage>,
    pub complexity: CodeComplexity,
}

/// 代码使用位置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeUsage {
    pub file: String,
    pub line: usize,
    pub context: String,
}

/// 代码复杂度
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeComplexity {
    pub cyclomatic: usize,
    pub cognitive: usize,
    pub lines_of_code: usize,
    pub score: String, // "low", "medium", "high"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_permissions_default() {
        let perms = AgentPermissions::default();
        assert!(perms.read_files);
        assert!(perms.write_files);
        assert!(!perms.create_commits); // 关键安全设置
        assert!(perms.requires_approval);
    }

    #[test]
    fn test_agent_permissions_conservative() {
        let perms = AgentPermissions::conservative();
        assert!(perms.read_files);
        assert!(!perms.write_files);
        assert!(!perms.run_commands);
        assert!(perms.requires_approval);
    }

    #[test]
    fn test_serialization() {
        let status = HealthStatus {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            debug_enabled: true,
            access_level: "Standard".to_string(),
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("ok"));
        assert!(json.contains("1.0.0"));
    }
}
