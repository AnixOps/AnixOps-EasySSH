#![allow(dead_code)]

//! 自然语言转命令模块
//!
//! 将自然语言描述转换为Shell命令

use anyhow::Result;
use std::sync::Arc;

use crate::ai_terminal::context::TerminalContext;
use crate::ai_terminal::providers::AiProvider;
use crate::ai_terminal::providers::{ChatRequest, Message, Role};

/// 自然语言转命令请求
#[derive(Debug, Clone)]
pub struct NlToCommandRequest {
    pub natural_language: String,
    pub context: TerminalContext,
    pub session_id: String,
    /// 期望的输出格式
    pub output_format: Option<String>,
    /// 操作系统类型（影响命令选择）
    pub os_type: Option<OsType>,
}

/// 操作系统类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsType {
    Linux,
    MacOS,
    Windows,
    Unknown,
}

impl OsType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Linux => "Linux",
            Self::MacOS => "macOS",
            Self::Windows => "Windows",
            Self::Unknown => "unknown",
        }
    }
}

/// 自然语言转命令结果
#[derive(Debug, Clone)]
pub struct NlToCommandResult {
    pub generated_commands: Vec<GeneratedCommand>,
    pub explanation: String,
    pub alternatives: Vec<String>,
}

/// 生成的命令
#[derive(Debug, Clone)]
pub struct GeneratedCommand {
    pub command: String,
    pub description: String,
    pub confidence: f32,
    pub is_safe: bool,
    pub risk_level: RiskLevel,
    pub requires_confirmation: bool,
}

/// 风险等级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Safe,        // 安全，无副作用
    ReadOnly,    // 只读操作
    Moderate,    // 可能有副作用
    Destructive, // 可能破坏数据
    Dangerous,   // 高风险操作
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::ReadOnly => "readonly",
            Self::Moderate => "moderate",
            Self::Destructive => "destructive",
            Self::Dangerous => "dangerous",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Safe => "Safe operation with no side effects",
            Self::ReadOnly => "Read-only operation, cannot modify data",
            Self::Moderate => "May have side effects, use with caution",
            Self::Destructive => "Can delete or modify important data",
            Self::Dangerous => "High risk operation, requires confirmation",
        }
    }

    pub fn requires_confirmation(&self) -> bool {
        matches!(self, Self::Destructive | Self::Dangerous)
    }
}

/// 自然语言处理器
pub struct NaturalLanguageProcessor {
    provider: Arc<dyn AiProvider>,
}

impl NaturalLanguageProcessor {
    pub fn new(provider: Arc<dyn AiProvider>) -> Self {
        Self { provider }
    }

    /// 将自然语言转换为命令
    pub async fn convert(&self, request: &NlToCommandRequest) -> Result<NlToCommandResult> {
        // 首先检查常见模式（快速路径）
        if let Some(pattern_result) = self.pattern_based_conversion(request) {
            return Ok(pattern_result);
        }

        // 使用AI进行转换
        self.ai_conversion(request).await
    }

    /// 基于模式的快速转换
    fn pattern_based_conversion(&self, request: &NlToCommandRequest) -> Option<NlToCommandResult> {
        let input = request.natural_language.to_lowercase();

        let patterns: Vec<(&str, &str, &str, RiskLevel)> = vec![
            // 查看当前目录
            (
                "show current directory",
                "pwd",
                "Print working directory",
                RiskLevel::ReadOnly,
            ),
            (
                "what directory am i in",
                "pwd",
                "Print working directory",
                RiskLevel::ReadOnly,
            ),
            (
                "current path",
                "pwd",
                "Print working directory",
                RiskLevel::ReadOnly,
            ),
            // 列出文件
            (
                "list files",
                "ls -la",
                "List all files with details",
                RiskLevel::ReadOnly,
            ),
            (
                "show files",
                "ls -la",
                "List all files with details",
                RiskLevel::ReadOnly,
            ),
            (
                "what files are here",
                "ls -la",
                "List all files with details",
                RiskLevel::ReadOnly,
            ),
            // 查看磁盘空间
            (
                "disk space",
                "df -h",
                "Show disk space usage",
                RiskLevel::ReadOnly,
            ),
            (
                "how much space is left",
                "df -h",
                "Show disk space usage",
                RiskLevel::ReadOnly,
            ),
            (
                "check disk usage",
                "df -h",
                "Show disk space usage",
                RiskLevel::ReadOnly,
            ),
            // 查看内存
            (
                "memory usage",
                "free -h",
                "Show memory usage",
                RiskLevel::ReadOnly,
            ),
            (
                "how much memory is free",
                "free -h",
                "Show memory usage",
                RiskLevel::ReadOnly,
            ),
            (
                "ram usage",
                "free -h",
                "Show memory usage",
                RiskLevel::ReadOnly,
            ),
            // 查看进程
            (
                "running processes",
                "ps aux",
                "List running processes",
                RiskLevel::ReadOnly,
            ),
            (
                "what processes are running",
                "ps aux",
                "List running processes",
                RiskLevel::ReadOnly,
            ),
            (
                "show processes",
                "ps aux",
                "List running processes",
                RiskLevel::ReadOnly,
            ),
            // CPU监控
            (
                "cpu usage",
                "top",
                "Show CPU and process usage",
                RiskLevel::ReadOnly,
            ),
            (
                "what's using cpu",
                "top",
                "Show CPU and process usage",
                RiskLevel::ReadOnly,
            ),
            // 查看时间
            (
                "current time",
                "date",
                "Show current date and time",
                RiskLevel::ReadOnly,
            ),
            (
                "what time is it",
                "date",
                "Show current date and time",
                RiskLevel::ReadOnly,
            ),
            // 查看用户信息
            (
                "who am i",
                "whoami",
                "Show current user",
                RiskLevel::ReadOnly,
            ),
            (
                "current user",
                "whoami",
                "Show current user",
                RiskLevel::ReadOnly,
            ),
            // 网络信息
            (
                "ip address",
                "ip addr",
                "Show network interfaces",
                RiskLevel::ReadOnly,
            ),
            (
                "my ip",
                "ip addr",
                "Show network interfaces",
                RiskLevel::ReadOnly,
            ),
            (
                "network interfaces",
                "ip addr",
                "Show network interfaces",
                RiskLevel::ReadOnly,
            ),
        ];

        for (pattern, cmd, desc, risk) in patterns {
            if input.contains(pattern) {
                return Some(NlToCommandResult {
                    generated_commands: vec![GeneratedCommand {
                        command: cmd.to_string(),
                        description: desc.to_string(),
                        confidence: 0.95,
                        is_safe: true,
                        risk_level: risk,
                        requires_confirmation: risk.requires_confirmation(),
                    }],
                    explanation: format!(
                        "Converted '{}' to '{}': {}",
                        request.natural_language, cmd, desc
                    ),
                    alternatives: vec![],
                });
            }
        }

        None
    }

    /// AI智能转换
    async fn ai_conversion(&self, request: &NlToCommandRequest) -> Result<NlToCommandResult> {
        let os_type = request.os_type.unwrap_or(OsType::Linux);

        let system_prompt = format!(
            r#"You are an expert shell command translator.
Convert natural language descriptions into precise shell commands.

Operating System: {}

Rules:
1. Generate 1-3 most relevant commands
2. Each command should be a single line
3. Consider the OS when generating commands
4. Mark destructive operations clearly
5. Include brief descriptions

Output format:
COMMAND: <shell command>
DESCRIPTION: <what it does>
SAFETY: <safe|readonly|moderate|destructive|dangerous>
CONFIDENCE: 0.XX

ALTERNATIVE 1: <alternative command>
ALTERNATIVE 2: <alternative command>

EXPLANATION: <brief explanation of the conversion>"#,
            os_type.as_str()
        );

        let context_str = if request.context.command_history.is_empty() {
            "No command history".to_string()
        } else {
            format!(
                "Recent commands: {}",
                request.context.command_history.join(", ")
            )
        };

        let user_prompt = format!(
            "Context: {}\nWorking directory: {}\n\nConvert to command: {}\n\nOutput:",
            context_str, request.context.working_directory, request.natural_language
        );

        let chat_request = ChatRequest {
            messages: vec![
                Message {
                    role: Role::System,
                    content: system_prompt,
                },
                Message {
                    role: Role::User,
                    content: user_prompt,
                },
            ],
            max_tokens: 800,
            temperature: 0.3,
            stream: false,
        };

        let response = self.provider.chat(chat_request).await?;
        let result = self.parse_conversion_response(&response.content);

        Ok(result)
    }

    /// 解析转换响应
    fn parse_conversion_response(&self, content: &str) -> NlToCommandResult {
        let mut commands = Vec::new();
        let mut explanation = String::new();
        let mut alternatives = Vec::new();

        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        let mut current_cmd: Option<String> = None;
        let mut current_desc: Option<String> = None;
        let mut current_safety: Option<RiskLevel> = None;
        let mut current_confidence: f32 = 0.8;

        while i < lines.len() {
            let line = lines[i].trim();

            if line.starts_with("COMMAND:") {
                // 保存之前的命令
                if let (Some(cmd), Some(desc), Some(safety)) =
                    (&current_cmd, &current_desc, &current_safety)
                {
                    commands.push(GeneratedCommand {
                        command: cmd.clone(),
                        description: desc.clone(),
                        confidence: current_confidence,
                        is_safe: matches!(safety, RiskLevel::Safe | RiskLevel::ReadOnly),
                        risk_level: *safety,
                        requires_confirmation: safety.requires_confirmation(),
                    });
                }

                current_cmd = Some(line[8..].trim().to_string());
                current_desc = None;
                current_safety = None;
                current_confidence = 0.8;
            } else if line.starts_with("DESCRIPTION:") {
                current_desc = Some(line[12..].trim().to_string());
            } else if line.starts_with("SAFETY:") {
                let safety_str = line[7..].trim().to_lowercase();
                current_safety = Some(match safety_str.as_str() {
                    "safe" => RiskLevel::Safe,
                    "readonly" | "read-only" | "read only" => RiskLevel::ReadOnly,
                    "destructive" => RiskLevel::Destructive,
                    "dangerous" => RiskLevel::Dangerous,
                    _ => RiskLevel::Moderate,
                });
            } else if line.starts_with("CONFIDENCE:") {
                let conf_str = line[11..].trim();
                current_confidence = conf_str.parse().unwrap_or(0.8);
            } else if line.starts_with("ALTERNATIVE") && line.contains(":") {
                let alt = line.split_once(':').map(|x| x.1).unwrap_or("").trim();
                if !alt.is_empty() {
                    alternatives.push(alt.to_string());
                }
            } else if line.starts_with("EXPLANATION:") {
                explanation = line[12..].trim().to_string();
                // 收集剩余行作为解释
                i += 1;
                while i < lines.len() {
                    explanation.push(' ');
                    explanation.push_str(lines[i].trim());
                    i += 1;
                }
                break;
            }

            i += 1;
        }

        // 保存最后一个命令
        if let (Some(cmd), Some(desc), Some(safety)) =
            (&current_cmd, &current_desc, &current_safety)
        {
            commands.push(GeneratedCommand {
                command: cmd.clone(),
                description: desc.clone(),
                confidence: current_confidence,
                is_safe: matches!(safety, RiskLevel::Safe | RiskLevel::ReadOnly),
                risk_level: *safety,
                requires_confirmation: safety.requires_confirmation(),
            });
        }

        if explanation.is_empty() {
            explanation = "Commands generated based on natural language input".to_string();
        }

        NlToCommandResult {
            generated_commands: commands,
            explanation,
            alternatives,
        }
    }
}

/// 转换自然语言到命令的主函数
pub async fn convert_to_command(
    provider: &Arc<dyn AiProvider>,
    request: &NlToCommandRequest,
) -> Result<NlToCommandResult> {
    let processor = NaturalLanguageProcessor::new(Arc::clone(provider));
    processor.convert(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_terminal::providers::MockProvider;

    #[test]
    fn test_pattern_conversion() {
        let processor = NaturalLanguageProcessor::new(Arc::new(MockProvider::new()));

        let request = NlToCommandRequest {
            natural_language: "show me the current directory".to_string(),
            context: TerminalContext::default(),
            session_id: "test".to_string(),
            output_format: None,
            os_type: Some(OsType::Linux),
        };

        let result = processor.pattern_based_conversion(&request);
        assert!(result.is_some());

        let conversion = result.unwrap();
        assert!(!conversion.generated_commands.is_empty());
    }

    #[test]
    fn test_risk_level() {
        assert!(!RiskLevel::Safe.requires_confirmation());
        assert!(!RiskLevel::ReadOnly.requires_confirmation());
        assert!(!RiskLevel::Moderate.requires_confirmation());
        assert!(RiskLevel::Destructive.requires_confirmation());
        assert!(RiskLevel::Dangerous.requires_confirmation());
    }
}
