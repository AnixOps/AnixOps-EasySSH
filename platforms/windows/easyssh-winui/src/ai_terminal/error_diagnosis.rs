#![allow(dead_code)]

//! 智能错误诊断模块
//!
//! 自动分析命令错误输出并给出解决方案

use std::sync::Arc;
use anyhow::Result;

use crate::ai_terminal::providers::AiProvider;
use crate::ai_terminal::context::TerminalContext;
use crate::ai_terminal::providers::{ChatRequest, Message, Role};

/// 错误诊断请求
#[derive(Debug, Clone)]
pub struct ErrorDiagnosisRequest {
    pub command: String,
    pub error_output: String,
    pub exit_code: Option<i32>,
    pub context: TerminalContext,
    pub session_id: String,
}

/// 诊断结果
#[derive(Debug, Clone)]
pub struct DiagnosisResult {
    pub error_summary: String,
    pub root_cause: String,
    pub solutions: Vec<Solution>,
    pub prevention_tips: Vec<String>,
    pub severity: ErrorSeverity,
    pub confidence: f32,
}

/// 解决方案
#[derive(Debug, Clone)]
pub struct Solution {
    pub description: String,
    pub command: Option<String>,
    pub explanation: String,
    pub estimated_success_rate: f32,
}

/// 错误严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Low,      // 警告，可忽略
    Medium,   // 一般错误，可修复
    High,     // 严重错误，需要立即处理
    Critical, // 系统级错误
}

impl ErrorSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Low => "Warning - can be ignored",
            Self::Medium => "Error - should be fixed",
            Self::High => "Serious error - requires immediate attention",
            Self::Critical => "Critical error - system-level issue",
        }
    }
}

/// 错误诊断器
pub struct ErrorDiagnoser {
    provider: Arc<dyn AiProvider>,
}

impl ErrorDiagnoser {
    pub fn new(provider: Arc<dyn AiProvider>) -> Self {
        Self { provider }
    }

    /// 诊断错误
    pub async fn diagnose(&self, request: &ErrorDiagnosisRequest) -> Result<DiagnosisResult> {
        // 首先尝试模式匹配（快速响应）
        if let Some(pattern_result) = self.pattern_based_diagnosis(request) {
            return Ok(pattern_result);
        }

        // 使用AI进行深度诊断
        self.ai_diagnosis(request).await
    }

    /// 基于模式的快速诊断
    fn pattern_based_diagnosis(&self, request: &ErrorDiagnosisRequest) -> Option<DiagnosisResult> {
        let error = &request.error_output.to_lowercase();
        let command = &request.command.to_lowercase();

        // 权限错误
        if error.contains("permission denied") || error.contains("access denied") {
            return Some(DiagnosisResult {
                error_summary: "Permission denied".to_string(),
                root_cause: "Insufficient privileges to execute the command".to_string(),
                solutions: vec![
                    Solution {
                        description: "Run with sudo (Linux/macOS)".to_string(),
                        command: Some(format!("sudo {}", request.command)),
                        explanation: "Elevates privileges to root user".to_string(),
                        estimated_success_rate: 0.95,
                    },
                    Solution {
                        description: "Check file permissions".to_string(),
                        command: Some("ls -la".to_string()),
                        explanation: "View file permissions to understand the issue".to_string(),
                        estimated_success_rate: 0.8,
                    },
                ],
                prevention_tips: vec![
                    "Use appropriate user permissions".to_string(),
                    "Check file ownership before operations".to_string(),
                ],
                severity: ErrorSeverity::Medium,
                confidence: 0.95,
            });
        }

        // 命令未找到
        if error.contains("command not found") || error.contains("is not recognized") {
            let cmd = command.split_whitespace().next().unwrap_or(command);
            return Some(DiagnosisResult {
                error_summary: format!("Command '{}' not found", cmd),
                root_cause: "The command is not installed or not in PATH".to_string(),
                solutions: vec![
                    Solution {
                        description: "Install the package (Ubuntu/Debian)".to_string(),
                        command: Some(format!("sudo apt install {}", cmd)),
                        explanation: "Installs the missing package".to_string(),
                        estimated_success_rate: 0.9,
                    },
                    Solution {
                        description: "Install the package (CentOS/RHEL)".to_string(),
                        command: Some(format!("sudo yum install {}", cmd)),
                        explanation: "Installs the missing package".to_string(),
                        estimated_success_rate: 0.9,
                    },
                    Solution {
                        description: "Install the package (macOS)".to_string(),
                        command: Some(format!("brew install {}", cmd)),
                        explanation: "Installs via Homebrew".to_string(),
                        estimated_success_rate: 0.85,
                    },
                ],
                prevention_tips: vec![
                    "Ensure required tools are installed".to_string(),
                    "Add tools to your setup script".to_string(),
                ],
                severity: ErrorSeverity::Medium,
                confidence: 0.95,
            });
        }

        // 文件未找到
        if error.contains("no such file or directory") || error.contains("cannot find the file") {
            return Some(DiagnosisResult {
                error_summary: "File or directory not found".to_string(),
                root_cause: "The specified path does not exist".to_string(),
                solutions: vec![
                    Solution {
                        description: "Create the directory".to_string(),
                        command: Some("mkdir -p <directory>".to_string()),
                        explanation: "Creates missing directories".to_string(),
                        estimated_success_rate: 0.9,
                    },
                    Solution {
                        description: "Check current directory".to_string(),
                        command: Some("pwd".to_string()),
                        explanation: "Verify you're in the correct directory".to_string(),
                        estimated_success_rate: 0.8,
                    },
                    Solution {
                        description: "List available files".to_string(),
                        command: Some("ls -la".to_string()),
                        explanation: "See what files are available".to_string(),
                        estimated_success_rate: 0.8,
                    },
                ],
                prevention_tips: vec![
                    "Verify paths before using them".to_string(),
                    "Use absolute paths when possible".to_string(),
                ],
                severity: ErrorSeverity::Medium,
                confidence: 0.9,
            });
        }

        // 端口被占用
        if error.contains("address already in use") || error.contains("port is already in use") {
            return Some(DiagnosisResult {
                error_summary: "Port already in use".to_string(),
                root_cause: "Another process is using the required port".to_string(),
                solutions: vec![
                    Solution {
                        description: "Find process using the port".to_string(),
                        command: Some("lsof -i :<port>".to_string()),
                        explanation: "Identify which process is using the port".to_string(),
                        estimated_success_rate: 0.9,
                    },
                    Solution {
                        description: "Kill the process".to_string(),
                        command: Some("kill -9 <PID>".to_string()),
                        explanation: "Forcefully terminate the process".to_string(),
                        estimated_success_rate: 0.85,
                    },
                    Solution {
                        description: "Use a different port".to_string(),
                        command: None,
                        explanation: "Configure your application to use an available port".to_string(),
                        estimated_success_rate: 1.0,
                    },
                ],
                prevention_tips: vec![
                    "Use process managers to avoid port conflicts".to_string(),
                    "Document which ports your services use".to_string(),
                ],
                severity: ErrorSeverity::Medium,
                confidence: 0.9,
            });
        }

        // 网络连接错误
        if error.contains("connection refused") || error.contains("could not connect") {
            return Some(DiagnosisResult {
                error_summary: "Connection refused".to_string(),
                root_cause: "Unable to establish network connection".to_string(),
                solutions: vec![
                    Solution {
                        description: "Check if service is running".to_string(),
                        command: Some("systemctl status <service>".to_string()),
                        explanation: "Verify the target service is active".to_string(),
                        estimated_success_rate: 0.85,
                    },
                    Solution {
                        description: "Test connectivity".to_string(),
                        command: Some("ping <host>".to_string()),
                        explanation: "Check network connectivity".to_string(),
                        estimated_success_rate: 0.8,
                    },
                    Solution {
                        description: "Check firewall rules".to_string(),
                        command: Some("sudo iptables -L".to_string()),
                        explanation: "Verify firewall isn't blocking the connection".to_string(),
                        estimated_success_rate: 0.75,
                    },
                ],
                prevention_tips: vec![
                    "Ensure services start on boot".to_string(),
                    "Monitor service health".to_string(),
                ],
                severity: ErrorSeverity::High,
                confidence: 0.85,
            });
        }

        // 内存不足
        if error.contains("out of memory") || error.contains("cannot allocate memory") {
            return Some(DiagnosisResult {
                error_summary: "Out of memory".to_string(),
                root_cause: "System ran out of available memory".to_string(),
                solutions: vec![
                    Solution {
                        description: "Check memory usage".to_string(),
                        command: Some("free -h".to_string()),
                        explanation: "View current memory status".to_string(),
                        estimated_success_rate: 1.0,
                    },
                    Solution {
                        description: "Find memory-heavy processes".to_string(),
                        command: Some("ps aux --sort=-%mem | head".to_string()),
                        explanation: "Identify processes consuming most memory".to_string(),
                        estimated_success_rate: 0.9,
                    },
                    Solution {
                        description: "Kill heavy processes".to_string(),
                        command: Some("kill -9 <PID>".to_string()),
                        explanation: "Terminate memory-intensive processes".to_string(),
                        estimated_success_rate: 0.85,
                    },
                    Solution {
                        description: "Add swap space".to_string(),
                        command: Some("sudo swapon -a".to_string()),
                        explanation: "Enable swap if available".to_string(),
                        estimated_success_rate: 0.7,
                    },
                ],
                prevention_tips: vec![
                    "Monitor memory usage regularly".to_string(),
                    "Set up memory alerts".to_string(),
                    "Optimize application memory usage".to_string(),
                ],
                severity: ErrorSeverity::High,
                confidence: 0.9,
            });
        }

        // 磁盘空间不足
        if error.contains("no space left on device") || error.contains("disk full") {
            return Some(DiagnosisResult {
                error_summary: "Disk space full".to_string(),
                root_cause: "No available disk space on the device".to_string(),
                solutions: vec![
                    Solution {
                        description: "Check disk usage".to_string(),
                        command: Some("df -h".to_string()),
                        explanation: "View disk space usage by filesystem".to_string(),
                        estimated_success_rate: 1.0,
                    },
                    Solution {
                        description: "Find large files".to_string(),
                        command: Some("du -h / | sort -rh | head -20".to_string()),
                        explanation: "Identify largest files and directories".to_string(),
                        estimated_success_rate: 0.9,
                    },
                    Solution {
                        description: "Clean package cache (Ubuntu/Debian)".to_string(),
                        command: Some("sudo apt clean".to_string()),
                        explanation: "Remove cached package files".to_string(),
                        estimated_success_rate: 0.85,
                    },
                    Solution {
                        description: "Remove old logs".to_string(),
                        command: Some("sudo find /var/log -type f -name '*.log' -mtime +7 -delete".to_string()),
                        explanation: "Delete log files older than 7 days".to_string(),
                        estimated_success_rate: 0.8,
                    },
                ],
                prevention_tips: vec![
                    "Set up disk usage monitoring".to_string(),
                    "Configure log rotation".to_string(),
                    "Regular cleanup of temporary files".to_string(),
                ],
                severity: ErrorSeverity::High,
                confidence: 0.9,
            });
        }

        // SSH密钥错误
        if error.contains("permission denied (publickey)") || error.contains("too many authentication failures") {
            return Some(DiagnosisResult {
                error_summary: "SSH authentication failed".to_string(),
                root_cause: "SSH key authentication failed or rejected".to_string(),
                solutions: vec![
                    Solution {
                        description: "Check SSH key permissions".to_string(),
                        command: Some("chmod 600 ~/.ssh/id_rsa".to_string()),
                        explanation: "Fix private key permissions".to_string(),
                        estimated_success_rate: 0.9,
                    },
                    Solution {
                        description: "Add key to SSH agent".to_string(),
                        command: Some("ssh-add ~/.ssh/id_rsa".to_string()),
                        explanation: "Add private key to SSH agent".to_string(),
                        estimated_success_rate: 0.85,
                    },
                    Solution {
                        description: "Test with verbose output".to_string(),
                        command: Some("ssh -vvv user@host".to_string()),
                        explanation: "Get detailed connection debugging".to_string(),
                        estimated_success_rate: 0.8,
                    },
                ],
                prevention_tips: vec![
                    "Use SSH agent for key management".to_string(),
                    "Keep backup SSH access methods".to_string(),
                ],
                severity: ErrorSeverity::High,
                confidence: 0.9,
            });
        }

        // 没有找到匹配的模式
        None
    }

    /// AI深度诊断
    async fn ai_diagnosis(&self, request: &ErrorDiagnosisRequest) -> Result<DiagnosisResult> {
        let system_prompt = r#"You are an expert system administrator and debugger.
Analyze the error and provide a structured diagnosis.

Format your response as:
SUMMARY: Brief error summary
ROOT_CAUSE: Detailed explanation of why this happened
SEVERITY: low/medium/high/critical

SOLUTION 1:
Description: What this solution does
Command: The exact command to run (if applicable)
Explanation: Why this should work
Success Rate: 0.XX

SOLUTION 2:
...

PREVENTION:
- Tip 1
- Tip 2
- Tip 3"#;

        let user_prompt = format!(
            "Command: {}\nExit code: {}\nError output:\n{}\n\nProvide diagnosis:",
            request.command,
            request.exit_code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string()),
            request.error_output
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
            max_tokens: 1500,
            temperature: 0.2,
            stream: false,
        };

        let response = self.provider.chat(chat_request).await?;
        let result = self.parse_diagnosis_response(&response.content);

        Ok(result)
    }

    /// 解析AI诊断响应
    fn parse_diagnosis_response(&self, content: &str) -> DiagnosisResult {
        let mut summary = String::new();
        let mut root_cause = String::new();
        let mut severity = ErrorSeverity::Medium;
        let mut solutions = Vec::new();
        let mut prevention_tips = Vec::new();

        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            if line.starts_with("SUMMARY:") {
                summary = line[8..].trim().to_string();
                i += 1;
                while i < lines.len() && !lines[i].trim().starts_with("ROOT_CAUSE:") {
                    summary.push(' ');
                    summary.push_str(lines[i].trim());
                    i += 1;
                }
                continue;
            }

            if line.starts_with("ROOT_CAUSE:") {
                root_cause = line[11..].trim().to_string();
                i += 1;
                while i < lines.len() && !lines[i].trim().starts_with("SEVERITY:") {
                    root_cause.push(' ');
                    root_cause.push_str(lines[i].trim());
                    i += 1;
                }
                continue;
            }

            if line.starts_with("SEVERITY:") {
                let sev_str = line[9..].trim().to_lowercase();
                severity = match sev_str.as_str() {
                    "low" => ErrorSeverity::Low,
                    "high" => ErrorSeverity::High,
                    "critical" => ErrorSeverity::Critical,
                    _ => ErrorSeverity::Medium,
                };
            }

            if line.starts_with("SOLUTION") && line.contains(":") {
                // 解析解决方案
                let mut desc = String::new();
                let mut cmd = None;
                let mut expl = String::new();
                let mut success_rate = 0.8;

                i += 1;
                while i < lines.len() && !lines[i].trim().starts_with("SOLUTION") && !lines[i].trim().starts_with("PREVENTION") {
                    let inner = lines[i].trim();
                    if inner.starts_with("Description:") {
                        desc = inner[12..].trim().to_string();
                    } else if inner.starts_with("Command:") {
                        let c = inner[8..].trim().to_string();
                        if !c.is_empty() && c != "N/A" {
                            cmd = Some(c);
                        }
                    } else if inner.starts_with("Explanation:") {
                        expl = inner[12..].trim().to_string();
                    } else if inner.starts_with("Success Rate:") {
                        let rate_str = inner[13..].trim().replace("0.", "");
                        success_rate = rate_str.parse().unwrap_or(0.8);
                    }
                    i += 1;
                }

                if !desc.is_empty() {
                    solutions.push(Solution {
                        description: desc,
                        command: cmd,
                        explanation: expl,
                        estimated_success_rate: success_rate,
                    });
                }
                continue;
            }

            if line.starts_with("PREVENTION:") {
                i += 1;
                while i < lines.len() {
                    let tip = lines[i].trim();
                    if tip.starts_with("-") {
                        prevention_tips.push(tip[1..].trim().to_string());
                    }
                    i += 1;
                }
                break;
            }

            i += 1;
        }

        // 如果解析失败，使用默认内容
        if summary.is_empty() {
            summary = "Unknown error".to_string();
        }
        if root_cause.is_empty() {
            root_cause = "Could not determine root cause".to_string();
        }

        DiagnosisResult {
            error_summary: summary,
            root_cause,
            solutions,
            prevention_tips,
            severity,
            confidence: 0.8,
        }
    }
}

/// 诊断错误的主函数
pub async fn diagnose_error(
    provider: &Arc<dyn AiProvider>,
    request: &ErrorDiagnosisRequest,
) -> Result<DiagnosisResult> {
    let diagnoser = ErrorDiagnoser::new(Arc::clone(provider));
    diagnoser.diagnose(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_terminal::providers::MockProvider;

    #[test]
    fn test_pattern_matching() {
        let diagnoser = ErrorDiagnoser::new(Arc::new(MockProvider::new()));

        let request = ErrorDiagnosisRequest {
            command: "ls /root".to_string(),
            error_output: "ls: cannot open directory '/root': Permission denied".to_string(),
            exit_code: Some(2),
            context: TerminalContext::default(),
            session_id: "test".to_string(),
        };

        let result = diagnoser.pattern_based_diagnosis(&request);
        assert!(result.is_some());

        let diagnosis = result.unwrap();
        assert_eq!(diagnosis.severity, ErrorSeverity::Medium);
    }
}
