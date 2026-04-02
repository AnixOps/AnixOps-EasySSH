#![allow(dead_code)]

//! AI命令补全模块
//!
//! 基于上下文预测下一个命令，提供智能补全建议

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

use crate::ai_terminal::context::TerminalContext;
use crate::ai_terminal::providers::AiProvider;
use crate::ai_terminal::providers::{ChatRequest, Message, Role};

/// 补全请求
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub current_input: String,
    pub cursor_position: usize,
    pub context: TerminalContext,
    pub session_id: String,
}

/// 补全结果
#[derive(Debug, Clone)]
pub struct CompletionResult {
    pub suggestions: Vec<CompletionSuggestion>,
    pub context_info: Option<String>,
}

/// 补全建议
#[derive(Debug, Clone)]
pub struct CompletionSuggestion {
    pub text: String,
    pub display_text: String,
    pub description: Option<String>,
    pub confidence: f32, // 0.0 - 1.0
    pub category: SuggestionCategory,
    pub replace_range: (usize, usize), // 要替换的起始和结束位置
}

/// 建议分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionCategory {
    Command,
    Argument,
    FilePath,
    Option,
    Variable,
    Subcommand,
    History,
    AiGenerated,
}

impl SuggestionCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Argument => "argument",
            Self::FilePath => "filepath",
            Self::Option => "option",
            Self::Variable => "variable",
            Self::Subcommand => "subcommand",
            Self::History => "history",
            Self::AiGenerated => "ai",
        }
    }
}

/// 命令补全器
pub struct CommandCompleter {
    provider: Arc<dyn AiProvider>,
    cache: HashMap<String, CompletionResult>,
}

impl CommandCompleter {
    pub fn new(provider: Arc<dyn AiProvider>) -> Self {
        Self {
            provider,
            cache: HashMap::new(),
        }
    }

    /// 生成命令补全
    pub async fn complete(&mut self, request: &CompletionRequest) -> Result<CompletionResult> {
        // 首先尝试基于规则的补全
        let rule_based = self.rule_based_completion(request);

        // 如果输入较短或规则补全不足，使用AI补全
        if request.current_input.len() < 2 || rule_based.suggestions.len() < 3 {
            let ai_suggestions = self.ai_completion(request).await?;

            // 合并结果
            let mut combined = rule_based;
            combined.suggestions.extend(ai_suggestions.suggestions);

            // 去重并按置信度排序
            combined.suggestions = self.deduplicate_and_sort(combined.suggestions);

            return Ok(combined);
        }

        Ok(rule_based)
    }

    /// 基于规则的补全（快速响应，无需AI）
    fn rule_based_completion(&self, request: &CompletionRequest) -> CompletionResult {
        let mut suggestions = Vec::new();
        let input = &request.current_input;

        // 常见命令补全
        let common_commands = vec![
            ("ls", "List directory contents", SuggestionCategory::Command),
            ("cd", "Change directory", SuggestionCategory::Command),
            (
                "pwd",
                "Print working directory",
                SuggestionCategory::Command,
            ),
            ("cat", "Concatenate files", SuggestionCategory::Command),
            ("grep", "Search text patterns", SuggestionCategory::Command),
            ("find", "Find files", SuggestionCategory::Command),
            ("ps", "Process status", SuggestionCategory::Command),
            ("top", "Process viewer", SuggestionCategory::Command),
            (
                "htop",
                "Interactive process viewer",
                SuggestionCategory::Command,
            ),
            ("df", "Disk free", SuggestionCategory::Command),
            ("du", "Disk usage", SuggestionCategory::Command),
            ("free", "Memory usage", SuggestionCategory::Command),
            ("ssh", "Secure shell", SuggestionCategory::Command),
            ("scp", "Secure copy", SuggestionCategory::Command),
            ("git", "Version control", SuggestionCategory::Command),
            (
                "docker",
                "Container management",
                SuggestionCategory::Command,
            ),
            ("kubectl", "Kubernetes CLI", SuggestionCategory::Command),
            ("curl", "Transfer data", SuggestionCategory::Command),
            ("wget", "Download files", SuggestionCategory::Command),
            ("tar", "Archive files", SuggestionCategory::Command),
            ("zip", "Compress files", SuggestionCategory::Command),
            ("unzip", "Extract files", SuggestionCategory::Command),
            ("chmod", "Change permissions", SuggestionCategory::Command),
            ("chown", "Change owner", SuggestionCategory::Command),
            ("mkdir", "Make directory", SuggestionCategory::Command),
            ("rm", "Remove files", SuggestionCategory::Command),
            ("cp", "Copy files", SuggestionCategory::Command),
            ("mv", "Move files", SuggestionCategory::Command),
            ("touch", "Create empty file", SuggestionCategory::Command),
            ("echo", "Print text", SuggestionCategory::Command),
            ("head", "Output first lines", SuggestionCategory::Command),
            ("tail", "Output last lines", SuggestionCategory::Command),
            ("less", "Pager", SuggestionCategory::Command),
            ("more", "Pager", SuggestionCategory::Command),
            ("vim", "Vim editor", SuggestionCategory::Command),
            ("nano", "Nano editor", SuggestionCategory::Command),
            ("systemctl", "System control", SuggestionCategory::Command),
            ("journalctl", "View logs", SuggestionCategory::Command),
            ("apt", "Package manager", SuggestionCategory::Command),
            ("yum", "Package manager", SuggestionCategory::Command),
            ("dnf", "Package manager", SuggestionCategory::Command),
            ("pacman", "Package manager", SuggestionCategory::Command),
            ("brew", "Homebrew", SuggestionCategory::Command),
            ("pip", "Python package manager", SuggestionCategory::Command),
            ("npm", "Node package manager", SuggestionCategory::Command),
            ("cargo", "Rust package manager", SuggestionCategory::Command),
        ];

        for (cmd, desc, category) in common_commands {
            if cmd.starts_with(input) && cmd != input {
                suggestions.push(CompletionSuggestion {
                    text: cmd.to_string(),
                    display_text: format!("{} - {}", cmd, desc),
                    description: Some(desc.to_string()),
                    confidence: 0.9,
                    category,
                    replace_range: (0, request.cursor_position),
                });
            }
        }

        // 基于上下文的补全
        if let Some(last_cmd) = request.context.command_history.last() {
            // 根据上一个命令推荐后续命令
            match last_cmd.as_str() {
                "git" => {
                    let git_subcommands = vec![
                        ("status", "Show working tree status"),
                        ("log", "Show commit logs"),
                        ("add", "Add files"),
                        ("commit", "Record changes"),
                        ("push", "Push changes"),
                        ("pull", "Fetch and merge"),
                        ("branch", "List branches"),
                        ("checkout", "Switch branches"),
                        ("merge", "Join development histories"),
                        ("rebase", "Reapply commits"),
                    ];
                    for (sub, desc) in git_subcommands {
                        if sub.starts_with(input) {
                            suggestions.push(CompletionSuggestion {
                                text: format!("git {}", sub),
                                display_text: format!("git {} - {}", sub, desc),
                                description: Some(desc.to_string()),
                                confidence: 0.85,
                                category: SuggestionCategory::Subcommand,
                                replace_range: (0, request.cursor_position),
                            });
                        }
                    }
                }
                "docker" => {
                    let docker_subcommands = vec![
                        ("ps", "List containers"),
                        ("images", "List images"),
                        ("run", "Run container"),
                        ("exec", "Execute command"),
                        ("logs", "Fetch logs"),
                        ("build", "Build image"),
                        ("pull", "Pull image"),
                        ("push", "Push image"),
                        ("rm", "Remove container"),
                        ("rmi", "Remove image"),
                    ];
                    for (sub, desc) in docker_subcommands {
                        if sub.starts_with(input) {
                            suggestions.push(CompletionSuggestion {
                                text: format!("docker {}", sub),
                                display_text: format!("docker {} - {}", sub, desc),
                                description: Some(desc.to_string()),
                                confidence: 0.85,
                                category: SuggestionCategory::Subcommand,
                                replace_range: (0, request.cursor_position),
                            });
                        }
                    }
                }
                "kubectl" => {
                    let kubectl_subcommands = vec![
                        ("get", "Display resources"),
                        ("describe", "Show details"),
                        ("apply", "Apply configuration"),
                        ("delete", "Delete resources"),
                        ("logs", "Print logs"),
                        ("exec", "Execute command"),
                        ("port-forward", "Forward ports"),
                    ];
                    for (sub, desc) in kubectl_subcommands {
                        if sub.starts_with(input) {
                            suggestions.push(CompletionSuggestion {
                                text: format!("kubectl {}", sub),
                                display_text: format!("kubectl {} - {}", sub, desc),
                                description: Some(desc.to_string()),
                                confidence: 0.85,
                                category: SuggestionCategory::Subcommand,
                                replace_range: (0, request.cursor_position),
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        // 历史命令补全
        for cmd in &request.context.command_history {
            if cmd.starts_with(input) && cmd != input {
                // 避免与常用命令重复
                if !suggestions.iter().any(|s| s.text == *cmd) {
                    suggestions.push(CompletionSuggestion {
                        text: cmd.clone(),
                        display_text: format!("{} [history]", cmd),
                        description: Some("From command history".to_string()),
                        confidence: 0.7,
                        category: SuggestionCategory::History,
                        replace_range: (0, request.cursor_position),
                    });
                }
            }
        }

        CompletionResult {
            suggestions: suggestions.into_iter().take(10).collect(),
            context_info: None,
        }
    }

    /// AI智能补全
    async fn ai_completion(&self, request: &CompletionRequest) -> Result<CompletionResult> {
        let system_prompt = r#"You are an intelligent shell command assistant.
Your task is to predict the most likely next command based on the context.

Rules:
1. Suggest 1-5 relevant commands
2. Each suggestion should be a valid shell command
3. Include a brief explanation for each
4. Format: command|explanation
5. Be concise and accurate

Example output:
ls -la|List all files with details
grep -r "pattern" .|Search recursively for pattern
find . -name "*.txt"|Find all text files"#;

        let context_str = if request.context.command_history.is_empty() {
            "No previous commands".to_string()
        } else {
            format!(
                "Previous commands:\n{}",
                request.context.command_history.join("\n")
            )
        };

        let user_prompt = format!(
            "Current working directory: {}\n{}\n\nCurrent input: {}\n\nSuggest commands:",
            request.context.working_directory, context_str, request.current_input
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
            max_tokens: 500,
            temperature: 0.3,
            stream: false,
        };

        let response = self.provider.chat(chat_request).await?;

        let suggestions = self.parse_ai_response(&response.content);

        Ok(CompletionResult {
            suggestions,
            context_info: Some("AI-powered".to_string()),
        })
    }

    /// 解析AI响应
    fn parse_ai_response(&self, content: &str) -> Vec<CompletionSuggestion> {
        let mut suggestions = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // 尝试解析 command|explanation 格式
            let parts: Vec<&str> = line.splitn(2, '|').collect();
            if parts.len() == 2 {
                let cmd = parts[0].trim();
                let desc = parts[1].trim();

                if !cmd.is_empty() {
                    suggestions.push(CompletionSuggestion {
                        text: cmd.to_string(),
                        display_text: format!("{} - {}", cmd, desc),
                        description: Some(desc.to_string()),
                        confidence: 0.8,
                        category: SuggestionCategory::AiGenerated,
                        replace_range: (0, 0),
                    });
                }
            } else if !line.starts_with(" ") && !line.starts_with("\t") {
                // 可能只是一个命令
                let cmd = line.trim();
                suggestions.push(CompletionSuggestion {
                    text: cmd.to_string(),
                    display_text: cmd.to_string(),
                    description: None,
                    confidence: 0.7,
                    category: SuggestionCategory::AiGenerated,
                    replace_range: (0, 0),
                });
            }
        }

        suggestions
    }

    /// 去重并排序建议
    fn deduplicate_and_sort(
        &self,
        suggestions: Vec<CompletionSuggestion>,
    ) -> Vec<CompletionSuggestion> {
        let mut seen = std::collections::HashSet::new();
        let mut unique: Vec<_> = suggestions
            .into_iter()
            .filter(|s| seen.insert(s.text.clone()))
            .collect();

        // 按置信度降序排序
        unique.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        unique.into_iter().take(10).collect()
    }
}

/// 生成补全的主函数
pub async fn generate_completion(
    provider: &Arc<dyn AiProvider>,
    request: &CompletionRequest,
) -> Result<CompletionResult> {
    let mut completer = CommandCompleter::new(Arc::clone(provider));
    completer.complete(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_terminal::providers::MockProvider;

    #[test]
    fn test_rule_based_completion() {
        let request = CompletionRequest {
            current_input: "gi".to_string(),
            cursor_position: 2,
            context: TerminalContext::default(),
            session_id: "test".to_string(),
        };

        let completer = CommandCompleter::new(Arc::new(MockProvider::new()));
        let result = completer.rule_based_completion(&request);

        assert!(!result.suggestions.is_empty());
        assert!(result.suggestions.iter().any(|s| s.text == "git"));
    }

    #[test]
    fn test_parse_ai_response() {
        let completer = CommandCompleter::new(Arc::new(MockProvider::new()));

        let response = "ls -la|List all files\ngrep pattern|Search pattern";
        let suggestions = completer.parse_ai_response(response);

        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].text, "ls -la");
    }
}
