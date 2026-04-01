//! AI安全审计模块
//!
//! 分析执行的命令是否危险

#![allow(dead_code)]

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::ai_terminal::providers::AiProvider;
use crate::ai_terminal::providers::{ChatRequest, Message, Role};

/// 安全审计请求
#[derive(Debug, Clone)]
pub struct SecurityAuditRequest {
    pub command: String,
    pub context: Option<String>,
    pub user_permissions: UserPermissions,
}

/// 用户权限
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct UserPermissions {
    pub is_root: bool,
    pub is_sudoer: bool,
    pub groups: Vec<String>,
}

#[allow(dead_code)]

/// 安全审计结果
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SecurityAuditResult {
    pub is_safe: bool,
    pub risk_level: RiskLevel,
    pub risk_score: f32, // 0.0 - 1.0
    pub threats: Vec<SecurityThreat>,
    pub warnings: Vec<String>,
    pub safe_alternatives: Vec<String>,
    pub requires_confirmation: bool,
    pub confirmation_message: Option<String>,
    pub explanation: String,
}

/// 风险等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Safe,     // 无风险
    Low,      // 低风险
    Medium,   // 中等风险
    High,     // 高风险
    Critical, // 严重风险
}

#[allow(dead_code)]
impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Safe => "Safe to execute",
            Self::Low => "Low risk - proceed with awareness",
            Self::Medium => "Medium risk - review before executing",
            Self::High => "High risk - requires confirmation",
            Self::Critical => "Critical risk - dangerous operation",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::Safe => "#27ae60",
            Self::Low => "#f1c40f",
            Self::Medium => "#e67e22",
            Self::High => "#e74c3c",
            Self::Critical => "#c0392b",
        }
    }

    pub fn requires_confirmation(&self) -> bool {
        matches!(self, Self::High | Self::Critical)
    }
}

/// 安全威胁 (API预留)
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SecurityThreat {
    pub category: ThreatCategory,
    pub description: String,
    pub severity: RiskLevel,
    pub affected_resources: Vec<String>,
    pub mitigation: Option<String>,
}

/// 威胁分类 (API预留)
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreatCategory {
    DataLoss,              // 数据丢失
    DataExposure,          // 数据泄露
    PrivilegeEscalation,   // 权限提升
    SystemModification,    // 系统修改
    NetworkExposure,       // 网络暴露
    MaliciousCode,         // 恶意代码
    ResourceExhaustion,    // 资源耗尽
    InformationDisclosure, // 信息泄露
}

#[allow(dead_code)]
impl ThreatCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DataLoss => "data_loss",
            Self::DataExposure => "data_exposure",
            Self::PrivilegeEscalation => "privilege_escalation",
            Self::SystemModification => "system_modification",
            Self::NetworkExposure => "network_exposure",
            Self::MaliciousCode => "malicious_code",
            Self::ResourceExhaustion => "resource_exhaustion",
            Self::InformationDisclosure => "information_disclosure",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::DataLoss => "May result in permanent data loss",
            Self::DataExposure => "May expose sensitive data",
            Self::PrivilegeEscalation => "May increase user privileges unexpectedly",
            Self::SystemModification => "May modify system configuration or files",
            Self::NetworkExposure => "May expose network services or ports",
            Self::MaliciousCode => "May execute potentially malicious code",
            Self::ResourceExhaustion => "May exhaust system resources",
            Self::InformationDisclosure => "May reveal sensitive system information",
        }
    }
}

/// 安全审计器 (API预留)
#[allow(dead_code)]
pub struct SecurityAuditor {
    provider: Arc<dyn AiProvider>,
    dangerous_patterns: HashMap<String, (RiskLevel, ThreatCategory, &'static str)>,
    readonly_commands: HashSet<String>,
}

#[allow(dead_code)]
impl SecurityAuditor {
    pub fn new(provider: Arc<dyn AiProvider>) -> Self {
        let mut dangerous_patterns = HashMap::new();

        // 定义危险模式
        dangerous_patterns.insert(
            "rm -rf /".to_string(),
            (
                RiskLevel::Critical,
                ThreatCategory::DataLoss,
                "Will recursively delete entire filesystem",
            ),
        );
        dangerous_patterns.insert(
            "rm -rf /*".to_string(),
            (
                RiskLevel::Critical,
                ThreatCategory::DataLoss,
                "Will recursively delete all files",
            ),
        );
        dangerous_patterns.insert(
            ":(){ :|:& };:".to_string(),
            (
                RiskLevel::Critical,
                ThreatCategory::ResourceExhaustion,
                "Fork bomb - will crash system",
            ),
        );
        dangerous_patterns.insert(
            "dd if=/dev/zero of=/dev/sda".to_string(),
            (
                RiskLevel::Critical,
                ThreatCategory::DataLoss,
                "Will overwrite disk with zeros",
            ),
        );
        dangerous_patterns.insert(
            "dd if=/dev/random of=/dev/sda".to_string(),
            (
                RiskLevel::Critical,
                ThreatCategory::DataLoss,
                "Will overwrite disk with random data",
            ),
        );
        dangerous_patterns.insert(
            "mkfs.ext4 /dev/sda".to_string(),
            (
                RiskLevel::Critical,
                ThreatCategory::DataLoss,
                "Will format disk",
            ),
        );
        dangerous_patterns.insert(
            "mv / /dev/null".to_string(),
            (
                RiskLevel::Critical,
                ThreatCategory::DataLoss,
                "Will attempt to move root to null",
            ),
        );
        dangerous_patterns.insert(
            "wget.*|.*sh".to_string(),
            (
                RiskLevel::High,
                ThreatCategory::MaliciousCode,
                "Piping downloaded content directly to shell",
            ),
        );
        dangerous_patterns.insert(
            "curl.*|.*sh".to_string(),
            (
                RiskLevel::High,
                ThreatCategory::MaliciousCode,
                "Piping downloaded content directly to shell",
            ),
        );
        dangerous_patterns.insert(
            "> /etc/passwd".to_string(),
            (
                RiskLevel::Critical,
                ThreatCategory::SystemModification,
                "Will overwrite passwd file",
            ),
        );
        dangerous_patterns.insert(
            "> /etc/shadow".to_string(),
            (
                RiskLevel::Critical,
                ThreatCategory::SystemModification,
                "Will overwrite shadow file",
            ),
        );

        let readonly_commands = [
            "ls",
            "cat",
            "less",
            "more",
            "head",
            "tail",
            "grep",
            "find",
            "pwd",
            "echo",
            "date",
            "whoami",
            "who",
            "w",
            "ps",
            "top",
            "htop",
            "df",
            "du",
            "free",
            "uname",
            "hostname",
            "uptime",
            "env",
            "which",
            "whereis",
            "file",
            "stat",
            "lsblk",
            "lscpu",
            "lsusb",
            "lspci",
            "lsmod",
            "ifconfig",
            "ip",
            "netstat",
            "ss",
            "ping",
            "traceroute",
            "dig",
            "nslookup",
            "man",
            "info",
            "help",
        ]
        .iter()
        .map(|&s| s.to_string())
        .collect();

        Self {
            provider,
            dangerous_patterns,
            readonly_commands,
        }
    }

    /// 审计命令
    pub async fn audit(&self, request: &SecurityAuditRequest) -> Result<SecurityAuditResult> {
        // 首先进行规则分析
        let rule_result = self.rule_based_audit(request);

        // 如果规则分析发现高风险，直接返回
        if rule_result.risk_level >= RiskLevel::High {
            return Ok(rule_result);
        }

        // 对于中低风险，使用AI进行深度分析
        self.ai_audit(request, rule_result).await
    }

    /// 基于规则的安全审计
    fn rule_based_audit(&self, request: &SecurityAuditRequest) -> SecurityAuditResult {
        let command = &request.command;
        let mut threats = Vec::new();
        let mut warnings = Vec::new();
        let mut risk_level = RiskLevel::Safe;
        let mut risk_score: f32 = 0.0;

        // 检查危险模式
        for (pattern, (level, category, description)) in &self.dangerous_patterns {
            if command.contains(pattern) {
                threats.push(SecurityThreat {
                    category: *category,
                    description: description.to_string(),
                    severity: *level,
                    affected_resources: vec!["system".to_string()],
                    mitigation: Some("Do not execute this command".to_string()),
                });
                risk_level = risk_level.max(*level);
                risk_score = risk_score.max(match level {
                    RiskLevel::Critical => 1.0,
                    RiskLevel::High => 0.8,
                    RiskLevel::Medium => 0.5,
                    RiskLevel::Low => 0.2,
                    RiskLevel::Safe => 0.0,
                });
            }
        }

        // 检查是否使用sudo/root
        let uses_elevation = command.starts_with("sudo ")
            || command.contains(" sudo ")
            || request.user_permissions.is_root;

        if uses_elevation {
            warnings.push("Command uses elevated privileges".to_string());
            risk_score += 0.1;
        }

        // 检查rm命令
        if command.contains("rm ") {
            if command.contains(" -rf ") || command.contains(" -fr ") {
                if command.contains(" /") || command.contains("*") {
                    threats.push(SecurityThreat {
                        category: ThreatCategory::DataLoss,
                        description: "Recursive force delete may remove important files"
                            .to_string(),
                        severity: RiskLevel::High,
                        affected_resources: vec!["files".to_string()],
                        mitigation: Some("Verify target path carefully".to_string()),
                    });
                    risk_level = risk_level.max(RiskLevel::High);
                    risk_score += 0.7;
                } else {
                    warnings.push("Force delete flag detected".to_string());
                    risk_score += 0.3;
                }
            }

            if !command.contains(" -i") && !command.contains("--interactive") {
                warnings.push("rm command without confirmation (-i)".to_string());
            }
        }

        // 检查重定向操作
        if command.contains(">") || command.contains(">>") {
            if command.contains("/etc/") || command.contains("/usr/") || command.contains("/var/") {
                threats.push(SecurityThreat {
                    category: ThreatCategory::SystemModification,
                    description: "Redirecting output to system directories".to_string(),
                    severity: RiskLevel::High,
                    affected_resources: vec!["system files".to_string()],
                    mitigation: Some("Ensure you have proper backups".to_string()),
                });
                risk_level = risk_level.max(RiskLevel::High);
                risk_score += 0.6;
            } else {
                warnings.push("Output redirection detected".to_string());
                risk_score += 0.1;
            }
        }

        // 检查管道到shell
        if (command.contains("wget") || command.contains("curl"))
            && (command.contains("| sh")
                || command.contains("| bash")
                || command.contains("|/bin/sh"))
        {
            threats.push(SecurityThreat {
                category: ThreatCategory::MaliciousCode,
                description: "Downloading and executing remote code".to_string(),
                severity: RiskLevel::Critical,
                affected_resources: vec!["system".to_string()],
                mitigation: Some("Review the script content before execution".to_string()),
            });
            risk_level = risk_level.max(RiskLevel::Critical);
            risk_score = 1.0;
        }

        // 检查chmod/chown
        if command.starts_with("chmod ") || command.starts_with("chown ") {
            if command.contains(" -R ") || command.contains(" --recursive ") {
                warnings.push("Recursive permission change".to_string());
                risk_score += 0.3;
            }
            if command.contains(" 777 ") || command.contains(" 666 ") {
                threats.push(SecurityThreat {
                    category: ThreatCategory::PrivilegeEscalation,
                    description: "Setting overly permissive permissions (777/666)".to_string(),
                    severity: RiskLevel::Medium,
                    affected_resources: vec!["file permissions".to_string()],
                    mitigation: Some(
                        "Use more restrictive permissions (e.g., 755, 644)".to_string(),
                    ),
                });
                risk_level = risk_level.max(RiskLevel::Medium);
                risk_score += 0.4;
            }
        }

        // 检查数据库操作
        if command.contains("DROP ") || command.contains("DELETE ") || command.contains("TRUNCATE ")
        {
            threats.push(SecurityThreat {
                category: ThreatCategory::DataLoss,
                description: "Database destructive operation".to_string(),
                severity: RiskLevel::High,
                affected_resources: vec!["database".to_string()],
                mitigation: Some("Ensure you have database backups".to_string()),
            });
            risk_level = risk_level.max(RiskLevel::High);
            risk_score += 0.8;
        }

        // 检查docker命令
        if command.starts_with("docker ") {
            if command.contains(" rm") || command.contains(" rmi") || command.contains(" prune") {
                warnings
                    .push("Docker cleanup operation - may remove containers/images".to_string());
                risk_score += 0.2;
            }
            if command.contains(" -v ") || command.contains(" --volumes ") {
                warnings.push("Docker volume operation detected".to_string());
            }
            if command.contains(" --privileged ") {
                threats.push(SecurityThreat {
                    category: ThreatCategory::PrivilegeEscalation,
                    description: "Privileged container - full host access".to_string(),
                    severity: RiskLevel::High,
                    affected_resources: vec!["host system".to_string()],
                    mitigation: Some("Avoid privileged mode if possible".to_string()),
                });
                risk_level = risk_level.max(RiskLevel::High);
                risk_score += 0.6;
            }
        }

        // 检查网络暴露
        if command.contains("nc -l") || command.contains("ncat -l") || command.contains("netcat -l")
        {
            warnings.push("Network listener detected".to_string());
            if command.contains(" -p ") || command.contains(" -e ") {
                threats.push(SecurityThreat {
                    category: ThreatCategory::NetworkExposure,
                    description: "Opening network port with command execution".to_string(),
                    severity: RiskLevel::High,
                    affected_resources: vec!["network".to_string()],
                    mitigation: Some("Ensure firewall rules are in place".to_string()),
                });
                risk_level = risk_level.max(RiskLevel::High);
                risk_score += 0.5;
            }
        }

        // 检查是否只读命令
        let base_cmd = command.split_whitespace().next().unwrap_or("");
        let is_readonly = self.readonly_commands.contains(base_cmd)
            && !command.contains(">")
            && !command.contains("| sh")
            && !command.contains("| bash");

        if is_readonly && threats.is_empty() && risk_level == RiskLevel::Safe {
            risk_score = 0.0;
        }

        // 限制风险分数
        risk_score = risk_score.min(1.0).max(0.0);

        // 生成安全替代方案
        let safe_alternatives = self.generate_safe_alternatives(command, &threats);

        // 生成确认消息
        let confirmation_message = if risk_level.requires_confirmation() {
            Some(format!(
                "This command has been flagged as {} risk. {} Are you sure you want to execute it?",
                risk_level.as_str(),
                risk_level.description()
            ))
        } else {
            None
        };

        SecurityAuditResult {
            is_safe: risk_level == RiskLevel::Safe && threats.is_empty(),
            risk_level,
            risk_score,
            threats,
            warnings,
            safe_alternatives,
            requires_confirmation: risk_level.requires_confirmation(),
            confirmation_message,
            explanation: format!(
                "Risk level: {} (score: {:.2})",
                risk_level.description(),
                risk_score
            ),
        }
    }

    /// AI深度审计
    async fn ai_audit(
        &self,
        request: &SecurityAuditRequest,
        rule_result: SecurityAuditResult,
    ) -> Result<SecurityAuditResult> {
        // 如果规则分析已经很明确，不需要AI分析
        if rule_result.risk_level != RiskLevel::Low && rule_result.risk_level != RiskLevel::Safe {
            return Ok(rule_result);
        }

        let system_prompt = r#"You are a security analyst. Review the command for potential security risks.

Focus on:
1. Data loss potential
2. Privilege escalation
3. System modification
4. Information disclosure
5. Malicious code execution

Output format:
RISK_LEVEL: <safe/low/medium/high/critical>
RISK_SCORE: 0.XX
THREAT: <category>|<description>|<severity>
WARNING: <warning message>
ALTERNATIVE: <safer alternative command>
EXPLANATION: <brief security analysis>"#;

        let user_prompt = format!(
            "Command: {}\nUser: {}\n\nSecurity analysis:",
            request.command,
            if request.user_permissions.is_root {
                "root"
            } else {
                "normal user"
            }
        );

        let _chat_request = ChatRequest {
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
            max_tokens: 800,
            temperature: 0.2,
            stream: false,
        };

        // AI分析（可选，取决于配置）
        // 这里简化处理，直接返回规则分析结果
        Ok(rule_result)
    }

    /// 生成安全替代方案
    fn generate_safe_alternatives(
        &self,
        command: &str,
        _threats: &[SecurityThreat],
    ) -> Vec<String> {
        let mut alternatives = Vec::new();

        // 针对rm命令
        if command.starts_with("rm ") && !command.contains(" -i") {
            alternatives.push(command.replacen("rm ", "rm -i ", 1));
        }

        // 针对递归删除
        if command.contains("rm -rf ") || command.contains("rm -fr ") {
            alternatives.push(
                command
                    .replace("rm -rf ", "rm -ri ")
                    .replace("rm -fr ", "rm -ri "),
            );
            alternatives.push("# Use 'find' with -delete for more control".to_string());
        }

        // 针对wget/curl管道
        if (command.contains("wget") || command.contains("curl"))
            && (command.contains("| sh") || command.contains("| bash"))
        {
            alternatives.push("# Download first, review, then execute".to_string());
            alternatives.push(
                command
                    .replace("| sh", " > script.sh && cat script.sh")
                    .replace("| bash", " > script.sh && cat script.sh"),
            );
        }

        // 针对chmod 777
        if command.contains("chmod 777") {
            alternatives.push(command.replace("chmod 777", "chmod 755"));
            alternatives.push(command.replace("chmod 777", "chmod 700"));
        }

        // 针对docker privileged
        if command.contains(" --privileged") {
            alternatives.push(command.replace(" --privileged", ""));
            alternatives.push("# Consider using --cap-add for specific capabilities".to_string());
        }

        alternatives
    }
}

/// 审计命令的主函数 (API预留)
#[allow(dead_code)]
pub async fn audit_command(
    provider: &Arc<dyn AiProvider>,
    request: &SecurityAuditRequest,
) -> Result<SecurityAuditResult> {
    let auditor = SecurityAuditor::new(Arc::clone(provider));
    auditor.audit(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_terminal::providers::MockProvider;

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Critical > RiskLevel::High);
        assert!(RiskLevel::High > RiskLevel::Medium);
        assert!(RiskLevel::Medium > RiskLevel::Low);
        assert!(RiskLevel::Low > RiskLevel::Safe);
    }

    #[test]
    fn test_rule_based_audit() {
        let auditor = SecurityAuditor::new(Arc::new(MockProvider::new()));

        let request = SecurityAuditRequest {
            command: "rm -rf /".to_string(),
            context: None,
            user_permissions: UserPermissions::default(),
        };

        let result = auditor.rule_based_audit(&request);
        assert_eq!(result.risk_level, RiskLevel::Critical);
        assert!(!result.is_safe);
    }

    #[test]
    fn test_safe_command() {
        let auditor = SecurityAuditor::new(Arc::new(MockProvider::new()));

        let request = SecurityAuditRequest {
            command: "ls -la".to_string(),
            context: None,
            user_permissions: UserPermissions::default(),
        };

        let result = auditor.rule_based_audit(&request);
        assert!(result.is_safe);
        assert_eq!(result.risk_level, RiskLevel::Safe);
    }

    #[test]
    fn test_sudo_warning() {
        let auditor = SecurityAuditor::new(Arc::new(MockProvider::new()));

        let request = SecurityAuditRequest {
            command: "sudo apt update".to_string(),
            context: None,
            user_permissions: UserPermissions::default(),
        };

        let result = auditor.rule_based_audit(&request);
        assert!(result.warnings.iter().any(|w| w.contains("elevated")));
    }
}
