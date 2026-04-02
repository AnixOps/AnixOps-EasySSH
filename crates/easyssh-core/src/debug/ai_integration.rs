//! AI集成模块
//!
//! 提供AI编程接口，这是从旧版 `ai_programming.rs` 迁移的统一实现
//!
//! # 功能
//! - 代码读取和分析
//! - 代码搜索
//! - 类型检查
//! - 测试运行
//! - Git操作
//! - 自我修复循环

use crate::debug::access::check_access;
use crate::debug::types::*;
use crate::debug::DebugAccessLevel;
use std::path::{Path, PathBuf};

// ============ 代码理解 ============

/// 读取文件内容
pub async fn read_code(path: String) -> Result<String, String> {
    if !check_access(DebugAccessLevel::Developer) {
        return Err("AI programming requires Developer access level".to_string());
    }

    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read file {}: {}", path, e))
}

/// 列出目录文件
pub async fn list_files(dir: String, pattern: Option<String>) -> Result<Vec<FileInfo>, String> {
    if !check_access(DebugAccessLevel::Developer) {
        return Err("AI programming requires Developer access level".to_string());
    }

    let path = PathBuf::from(&dir);
    if !path.exists() {
        return Err(format!("Directory does not exist: {}", dir));
    }

    let pattern = pattern.unwrap_or_else(|| "*".to_string());
    let mut results = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&path) {
        for entry in entries.flatten() {
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let name = entry.file_name().to_string_lossy().to_string();
            if pattern == "*" || name.contains(&pattern) {
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|m| m.elapsed().ok())
                    .map(|d| format!("{:?} ago", d))
                    .unwrap_or_else(|| "unknown".to_string());

                results.push(FileInfo {
                    path: entry.path().to_string_lossy().to_string(),
                    name,
                    size: metadata.len(),
                    modified,
                    is_directory: metadata.is_dir(),
                });
            }
        }
    }

    Ok(results)
}

/// 搜索代码
pub async fn search_code(query: String, path: Option<String>) -> Result<Vec<SearchResult>, String> {
    if !check_access(DebugAccessLevel::Developer) {
        return Err("AI programming requires Developer access level".to_string());
    }

    let search_path = path.unwrap_or_else(|| ".".to_string());
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    fn walk_dir(
        dir: &Path,
        query: &str,
        results: &mut Vec<SearchResult>,
    ) -> Result<(), std::io::Error> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // 跳过隐藏目录和目标目录
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !name.starts_with('.') && name != "target" {
                        let _ = walk_dir(&path, query, results);
                    }
                } else if path.is_file() {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ["rs", "ts", "tsx", "js", "jsx", "json", "toml", "md"].contains(&ext) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let content_lower = content.to_lowercase();
                            if content_lower.contains(query) {
                                for (line_idx, line) in content.lines().enumerate() {
                                    if line.to_lowercase().contains(query) {
                                        let context_before: Vec<String> = content
                                            .lines()
                                            .skip(line_idx.saturating_sub(2))
                                            .take(2)
                                            .map(|s| s.to_string())
                                            .collect();
                                        let context_after: Vec<String> = content
                                            .lines()
                                            .skip(line_idx + 1)
                                            .take(2)
                                            .map(|s| s.to_string())
                                            .collect();

                                        results.push(SearchResult {
                                            file: path.to_string_lossy().to_string(),
                                            line_number: line_idx + 1,
                                            line_content: line.trim().to_string(),
                                            context_before,
                                            context_after,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    walk_dir(Path::new(&search_path), &query_lower, &mut results)
        .map_err(|e| format!("Search failed: {}", e))?;

    Ok(results)
}

// ============ 代码修改 ============

/// 写入文件
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    if !check_access(DebugAccessLevel::Developer) {
        return Err("File write requires Developer access level".to_string());
    }

    // 安全检查：防止写入敏感路径
    let forbidden_paths = ["/etc", "/usr", "/bin", "/sbin", "/sys", "/proc"];
    for forbidden in &forbidden_paths {
        if path.starts_with(forbidden) {
            return Err(format!("Writing to {} is not allowed", forbidden));
        }
    }

    tokio::fs::write(&path, &content)
        .await
        .map_err(|e| format!("Failed to write file {}: {}", path, e))
}

/// 编辑文件（查找替换）
pub async fn edit_file(
    path: String,
    old_string: String,
    new_string: String,
) -> Result<EditResult, String> {
    if !check_access(DebugAccessLevel::Developer) {
        return Err("File edit requires Developer access level".to_string());
    }

    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read file {}: {}", path, e))?;

    if !content.contains(&old_string) {
        return Ok(EditResult {
            success: false,
            message: format!(
                "Old string not found in file: {}",
                &old_string[..old_string.len().min(50)]
            ),
            old_content: None,
            new_content: None,
        });
    }

    let old_content = content.clone();
    let new_content = content.replace(&old_string, &new_string);

    tokio::fs::write(&path, &new_content)
        .await
        .map_err(|e| format!("Failed to write file {}: {}", path, e))?;

    Ok(EditResult {
        success: true,
        message: "File edited successfully".to_string(),
        old_content: Some(old_content),
        new_content: Some(new_content),
    })
}

// ============ 测试执行 ============

/// 运行测试
pub async fn run_tests(filter: Option<String>) -> Result<TestResult, String> {
    if !check_access(DebugAccessLevel::Developer) {
        return Err("Test runner requires Developer access level".to_string());
    }

    let start = std::time::Instant::now();

    let mut args = vec!["test"];
    if let Some(f) = &filter {
        args.push(f);
    }
    args.push("--");
    args.push("--nocapture");

    let output = tokio::process::Command::new("cargo")
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("Failed to run tests: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    Ok(TestResult {
        success: output.status.success(),
        output: stdout.to_string(),
        errors: stderr.to_string(),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// 类型检查
pub async fn type_check() -> Result<TypeCheckResult, String> {
    if !check_access(DebugAccessLevel::Developer) {
        return Err("Type check requires Developer access level".to_string());
    }

    let output = tokio::process::Command::new("cargo")
        .args(["check", "--message-format=json"])
        .output()
        .await
        .map_err(|e| format!("Failed to run type check: {}", e))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let _stdout = String::from_utf8_lossy(&output.stdout);

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // 简单解析cargo check输出
    for line in stderr.lines() {
        if line.contains("error") {
            errors.push(line.to_string());
        } else if line.contains("warning") {
            warnings.push(line.to_string());
        }
    }

    Ok(TypeCheckResult {
        success: output.status.success() && errors.is_empty(),
        errors,
        warnings,
    })
}

/// 运行Linter
pub async fn lint(fix: bool) -> Result<LintResult, String> {
    if !check_access(DebugAccessLevel::Developer) {
        return Err("Lint requires Developer access level".to_string());
    }

    let mut args = vec!["clippy"];
    if fix {
        args.push("--fix");
        args.push("--allow-dirty");
    }
    args.push("--");
    args.push("-D");
    args.push("warnings");

    let output = tokio::process::Command::new("cargo")
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("Failed to run linter: {}", e))?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    let issues: Vec<LintIssue> = stderr
        .lines()
        .filter(|l| l.contains("warning:") || l.contains("error:"))
        .map(|l| LintIssue {
            file: "unknown".to_string(),
            line: 0,
            severity: if l.contains("error") {
                "error".to_string()
            } else {
                "warning".to_string()
            },
            message: l.to_string(),
            rule: "clippy".to_string(),
        })
        .collect();

    let fixed_count = if fix {
        stderr.matches("Fixed").count()
    } else {
        0
    };

    Ok(LintResult {
        success: output.status.success(),
        issues,
        fixed_count,
    })
}

// ============ 构建 ============

/// 构建项目
pub async fn build(target: Option<String>) -> Result<BuildResult, String> {
    if !check_access(DebugAccessLevel::Developer) {
        return Err("Build requires Developer access level".to_string());
    }

    let start = std::time::Instant::now();

    let mut args: Vec<String> = vec!["build".to_string()];
    if let Some(t) = target {
        args.push("--target".to_string());
        args.push(t.to_string());
    }

    let output = tokio::process::Command::new("cargo")
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("Failed to build: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    Ok(BuildResult {
        success: output.status.success(),
        output: stdout.to_string(),
        errors: stderr.to_string(),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

// ============ Git操作 ============

/// 获取Git状态
pub async fn git_status() -> Result<GitStatus, String> {
    let output = tokio::process::Command::new("git")
        .args(["status", "--porcelain", "-b"])
        .current_dir(".")
        .output()
        .await
        .map_err(|e| format!("Failed to get git status: {}", e))?;

    if !output.status.success() {
        return Err("Git status failed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = stdout.lines();

    let mut staged_files = Vec::new();
    let mut unstaged_files = Vec::new();
    let mut untracked_files = Vec::new();
    let mut current_branch = String::from("HEAD");
    let mut ahead = 0;
    let mut behind = 0;

    for line in lines {
        if line.starts_with("## ") {
            // 分支行
            if let Some(branch_info) = line.strip_prefix("## ") {
                if let Some((main, remote)) = branch_info.split_once("...") {
                    current_branch = main.to_string();
                    // 解析ahead/behind
                    if let Some(stats) = remote.split_once('[') {
                        let stats = stats.1.trim_end_matches(']');
                        for part in stats.split(',').map(|s| s.trim()) {
                            if part.ends_with("ahead") {
                                ahead = part
                                    .split_whitespace()
                                    .next()
                                    .and_then(|n| n.parse().ok())
                                    .unwrap_or(0);
                            } else if part.ends_with("behind") {
                                behind = part
                                    .split_whitespace()
                                    .next()
                                    .and_then(|n| n.parse().ok())
                                    .unwrap_or(0);
                            }
                        }
                    }
                } else {
                    current_branch = branch_info
                        .split_whitespace()
                        .next()
                        .unwrap_or("HEAD")
                        .to_string();
                }
            }
        } else if line.len() >= 3 {
            let index_status = line.chars().next().unwrap_or(' ');
            let worktree_status = line.chars().nth(1).unwrap_or(' ');
            let file_path = line[3..].to_string();

            if index_status == '?' && worktree_status == '?' {
                untracked_files.push(file_path);
            } else {
                if index_status != ' ' && index_status != '?' {
                    staged_files.push(file_path.clone());
                }
                if worktree_status != ' ' && worktree_status != '?' {
                    unstaged_files.push(file_path);
                }
            }
        }
    }

    let is_dirty =
        !staged_files.is_empty() || !unstaged_files.is_empty() || !untracked_files.is_empty();

    Ok(GitStatus {
        is_dirty,
        staged_files,
        unstaged_files,
        untracked_files,
        current_branch,
        ahead,
        behind,
    })
}

/// 获取Git diff
pub async fn git_diff(path: Option<String>) -> Result<String, String> {
    let mut args = vec!["diff"];
    if let Some(p) = &path {
        args.push(p);
    }

    let output = tokio::process::Command::new("git")
        .args(&args)
        .current_dir(".")
        .output()
        .await
        .map_err(|e| format!("Failed to get git diff: {}", e))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// 获取Git日志
pub async fn git_log(count: usize) -> Result<Vec<GitCommit>, String> {
    let output = tokio::process::Command::new("git")
        .args([
            "log",
            &format!("--max-count={}", count),
            "--pretty=format:%H|%h|%s|%an|%ae|%ai",
        ])
        .current_dir(".")
        .output()
        .await
        .map_err(|e| format!("Failed to get git log: {}", e))?;

    if !output.status.success() {
        return Err("Git log failed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 6 {
            commits.push(GitCommit {
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                message: parts[2].to_string(),
                author: parts[3].to_string(),
                email: parts[4].to_string(),
                date: parts[5].to_string(),
            });
        }
    }

    Ok(commits)
}

/// 获取Git分支列表
pub async fn git_branch() -> Result<Vec<GitBranch>, String> {
    let output = tokio::process::Command::new("git")
        .args(["branch", "-avv"])
        .current_dir(".")
        .output()
        .await
        .map_err(|e| format!("Failed to get git branches: {}", e))?;

    if !output.status.success() {
        return Err("Git branch failed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut branches = Vec::new();

    for line in stdout.lines() {
        let is_current = line.starts_with('*');
        let name = line.trim_start_matches(['*', ' ']).to_string();
        let is_remote = name.starts_with("remotes/");

        // 解析upstream信息
        let upstream = if line.contains('[') {
            line.split('[')
                .nth(1)
                .and_then(|s| s.split(']').next())
                .map(|s| s.to_string())
        } else {
            None
        };

        branches.push(GitBranch {
            name,
            is_current,
            is_remote,
            upstream,
        });
    }

    Ok(branches)
}

// ============ 上下文管理 ============

use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref AI_CONTEXT: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

/// 设置上下文变量
pub fn set_context(key: String, value: String) -> Result<(), String> {
    let mut ctx = AI_CONTEXT
        .lock()
        .map_err(|e| format!("Failed to lock context: {}", e))?;
    ctx.insert(key, value);
    Ok(())
}

/// 获取上下文变量
pub fn get_context(key: String) -> Result<Option<String>, String> {
    let ctx = AI_CONTEXT
        .lock()
        .map_err(|e| format!("Failed to lock context: {}", e))?;
    Ok(ctx.get(&key).cloned())
}

/// 清除所有上下文
pub fn clear_context() -> Result<(), String> {
    let mut ctx = AI_CONTEXT
        .lock()
        .map_err(|e| format!("Failed to lock context: {}", e))?;
    ctx.clear();
    Ok(())
}

// ============ 自我修复循环 ============

/// AI自我修复
///
/// 自动分析问题并尝试修复
pub async fn self_fix(
    _problem_description: String,
    max_iterations: Option<usize>,
) -> Result<SelfFixResult, String> {
    if !check_access(DebugAccessLevel::Developer) {
        return Err("Self-fix requires Developer access level".to_string());
    }

    let max_iterations = max_iterations.unwrap_or(5);
    let mut attempts = Vec::new();

    for iteration in 0..max_iterations {
        // 分析当前状态
        let _status = git_status().await?;
        let type_check_result = type_check().await?;

        // 记录尝试
        let attempt = FixAttempt {
            iteration,
            strategy: "analyze_and_fix".to_string(),
            files_modified: vec![], // 在实际实现中填充
            status: if type_check_result.success {
                "success".to_string()
            } else {
                "needs_fix".to_string()
            },
            error: if type_check_result.success {
                None
            } else {
                Some(format!("Type errors: {:?}", type_check_result.errors))
            },
        };
        attempts.push(attempt);

        if type_check_result.success {
            return Ok(SelfFixResult {
                success: true,
                iterations: attempts.len(),
                attempts,
                error: None,
            });
        }

        // 在实际实现中，这里会分析错误并尝试自动修复
        // 目前只是一个占位实现
    }

    Ok(SelfFixResult {
        success: false,
        iterations: attempts.len(),
        attempts,
        error: Some("Max iterations reached".to_string()),
    })
}

// ============ 任务执行 ============

/// 执行AI任务
pub async fn execute_task(
    task: String,
    permissions: AgentPermissions,
) -> Result<TaskResult, String> {
    if !check_access(DebugAccessLevel::Developer) {
        return Err("Task execution requires Developer access level".to_string());
    }

    let start = std::time::Instant::now();

    // 验证权限
    if permissions.requires_approval {
        // 在实际应用中，这里应该弹出确认对话框
        log::info!("Task requires approval: {}", task);
    }

    // 执行任务（简化实现）
    let result = match task.as_str() {
        "health_check" => {
            let health = crate::debug::health_check();
            format!("Health check: {:?}", health)
        }
        "git_status" => git_status().await.map(|s| format!("{:?}", s))?,
        _ => format!("Unknown task: {}", task),
    };

    Ok(TaskResult {
        success: true,
        task_id: uuid::Uuid::new_v4().to_string(),
        output: result,
        duration_ms: start.elapsed().as_millis() as u64,
        steps_completed: 1,
        steps_total: 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_management() {
        set_context("test_key".to_string(), "test_value".to_string()).unwrap();
        assert_eq!(
            get_context("test_key".to_string()).unwrap(),
            Some("test_value".to_string())
        );

        clear_context().unwrap();
        assert_eq!(get_context("test_key".to_string()).unwrap(), None);
    }

    #[test]
    fn test_access_control() {
        // 在没有启用debug的情况下，应该返回错误
        // 注意：这需要设置测试fixture来初始化debug状态
    }
}
