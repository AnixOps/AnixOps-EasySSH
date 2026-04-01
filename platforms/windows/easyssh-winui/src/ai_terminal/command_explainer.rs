#![allow(dead_code)]

//! 命令解释器模块
//!
//! 解释Shell命令的工作原理

use std::sync::Arc;
use anyhow::Result;

use crate::ai_terminal::providers::AiProvider;
use crate::ai_terminal::providers::{ChatRequest, Message, Role};

/// 解释请求
#[derive(Debug, Clone)]
pub struct ExplanationRequest {
    pub command: String,
    pub detail_level: DetailLevel,
    pub focus_area: Option<FocusArea>,
}

/// 详细程度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailLevel {
    Brief,      // 一句话解释
    Standard,   // 基本解释
    Detailed,   // 详细解释
    Technical,  // 技术细节
}

impl DetailLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Brief => "brief",
            Self::Standard => "standard",
            Self::Detailed => "detailed",
            Self::Technical => "technical",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Brief => "One-sentence summary",
            Self::Standard => "Basic explanation of what the command does",
            Self::Detailed => "Detailed explanation with examples",
            Self::Technical => "Technical deep-dive with implementation details",
        }
    }
}

/// 关注领域
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    General,        // 一般解释
    Security,       // 安全影响
    Performance,    // 性能影响
    Compatibility,  // 兼容性
    BestPractices,  // 最佳实践
}

impl FocusArea {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Security => "security",
            Self::Performance => "performance",
            Self::Compatibility => "compatibility",
            Self::BestPractices => "best_practices",
        }
    }
}

/// 解释结果
#[derive(Debug, Clone)]
pub struct ExplanationResult {
    pub summary: String,
    pub detailed_explanation: String,
    pub components: Vec<CommandComponent>,
    pub examples: Vec<Example>,
    pub warnings: Vec<String>,
    pub related_commands: Vec<String>,
    pub documentation_links: Vec<String>,
}

/// 命令组件
#[derive(Debug, Clone)]
pub struct CommandComponent {
    pub part: String,
    pub meaning: String,
    pub category: ComponentCategory,
}

/// 组件分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentCategory {
    Command,     // 主命令
    Option,      // 选项/标志
    Argument,    // 参数
    Pipe,        // 管道
    Redirection, // 重定向
    Variable,    // 变量
    Subcommand,  // 子命令
    Operator,    // 运算符
}

impl ComponentCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Option => "option",
            Self::Argument => "argument",
            Self::Pipe => "pipe",
            Self::Redirection => "redirection",
            Self::Variable => "variable",
            Self::Subcommand => "subcommand",
            Self::Operator => "operator",
        }
    }
}

/// 示例
#[derive(Debug, Clone)]
pub struct Example {
    pub description: String,
    pub command: String,
    pub explanation: String,
}

/// 命令解释器
pub struct CommandExplainer {
    provider: Arc<dyn AiProvider>,
}

impl CommandExplainer {
    pub fn new(provider: Arc<dyn AiProvider>) -> Self {
        Self { provider }
    }

    /// 解释命令
    pub async fn explain(&self, request: &ExplanationRequest) -> Result<ExplanationResult> {
        // 首先尝试模式匹配（快速响应）
        if let Some(pattern_result) = self.pattern_based_explanation(request) {
            if request.detail_level == DetailLevel::Brief {
                return Ok(pattern_result);
            }
        }

        // 使用AI进行详细解释
        self.ai_explanation(request).await
    }

    /// 基于模式的快速解释
    fn pattern_based_explanation(&self, request: &ExplanationRequest) -> Option<ExplanationResult> {
        let cmd = request.command.trim();

        // 简单命令快速解释
        let explanations: Vec<(&str, &str, Vec<CommandComponent>, Vec<&str>)> = vec![
            // ls 命令
            ("ls", "List directory contents", vec![
                CommandComponent {
                    part: "ls".to_string(),
                    meaning: "List directory contents".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["ls -la", "ls -lh", "ls -R"]),

            // pwd 命令
            ("pwd", "Print working directory", vec![
                CommandComponent {
                    part: "pwd".to_string(),
                    meaning: "Print name of current/working directory".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["pwd -P"]),

            // cd 命令
            ("cd", "Change directory", vec![
                CommandComponent {
                    part: "cd".to_string(),
                    meaning: "Change the shell working directory".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["cd ~", "cd -", "cd .."]),

            // cat 命令
            ("cat", "Concatenate files and print", vec![
                CommandComponent {
                    part: "cat".to_string(),
                    meaning: "Concatenate FILE(s) to standard output".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["cat file.txt", "cat -n file.txt"]),

            // grep 命令
            ("grep", "Search text using patterns", vec![
                CommandComponent {
                    part: "grep".to_string(),
                    meaning: "Print lines matching a pattern".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["grep 'pattern' file", "grep -r 'pattern' dir/"]),

            // find 命令
            ("find", "Search for files", vec![
                CommandComponent {
                    part: "find".to_string(),
                    meaning: "Search for files in a directory hierarchy".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["find . -name '*.txt'", "find . -type f -size +1M"]),

            // ps 命令
            ("ps", "Report process status", vec![
                CommandComponent {
                    part: "ps".to_string(),
                    meaning: "Report a snapshot of current processes".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["ps aux", "ps -ef"]),

            // df 命令
            ("df", "Report file system disk space", vec![
                CommandComponent {
                    part: "df".to_string(),
                    meaning: "Report file system disk space usage".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["df -h", "df -T"]),

            // du 命令
            ("du", "Estimate file space usage", vec![
                CommandComponent {
                    part: "du".to_string(),
                    meaning: "Estimate file space usage".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["du -sh *", "du -h --max-depth=1"]),

            // free 命令
            ("free", "Display memory usage", vec![
                CommandComponent {
                    part: "free".to_string(),
                    meaning: "Display amount of free and used memory".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["free -h", "free -m"]),

            // top 命令
            ("top", "Display processes", vec![
                CommandComponent {
                    part: "top".to_string(),
                    meaning: "Display Linux processes".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec![]),

            // tar 命令
            ("tar", "Archive utility", vec![
                CommandComponent {
                    part: "tar".to_string(),
                    meaning: "An archiving utility".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["tar -czf archive.tar.gz dir/", "tar -xzf archive.tar.gz"]),

            // chmod 命令
            ("chmod", "Change file mode bits", vec![
                CommandComponent {
                    part: "chmod".to_string(),
                    meaning: "Change file mode bits (permissions)".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["chmod 755 file", "chmod +x script.sh"]),

            // chown 命令
            ("chown", "Change file owner", vec![
                CommandComponent {
                    part: "chown".to_string(),
                    meaning: "Change file owner and group".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["chown user:group file", "chown -R user: dir/"]),

            // ssh 命令
            ("ssh", "OpenSSH client", vec![
                CommandComponent {
                    part: "ssh".to_string(),
                    meaning: "OpenSSH remote login client".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["ssh user@host", "ssh -p 2222 user@host"]),

            // scp 命令
            ("scp", "Secure copy", vec![
                CommandComponent {
                    part: "scp".to_string(),
                    meaning: "Secure copy (remote file copy program)".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["scp file user@host:/path", "scp -r dir/ user@host:/path"]),

            // git 命令
            ("git", "Version control system", vec![
                CommandComponent {
                    part: "git".to_string(),
                    meaning: "The stupid content tracker".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["git status", "git log", "git add ."]),

            // docker 命令
            ("docker", "Container platform", vec![
                CommandComponent {
                    part: "docker".to_string(),
                    meaning: "Docker container management".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["docker ps", "docker run -it ubuntu", "docker build -t myapp ."]),

            // curl 命令
            ("curl", "Transfer data from/to server", vec![
                CommandComponent {
                    part: "curl".to_string(),
                    meaning: "Transfer a URL".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["curl https://api.example.com", "curl -O http://example.com/file"]),

            // wget 命令
            ("wget", "Network downloader", vec![
                CommandComponent {
                    part: "wget".to_string(),
                    meaning: "The non-interactive network downloader".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["wget http://example.com/file", "wget -r -np -nH http://site.com/dir/"]),

            // rm 命令（带警告）
            ("rm", "Remove files or directories", vec![
                CommandComponent {
                    part: "rm".to_string(),
                    meaning: "Remove files or directories".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["rm file.txt", "rm -r directory/"]),

            // cp 命令
            ("cp", "Copy files and directories", vec![
                CommandComponent {
                    part: "cp".to_string(),
                    meaning: "Copy files and directories".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["cp file.txt backup.txt", "cp -r dir/ backup/"]),

            // mv 命令
            ("mv", "Move/rename files", vec![
                CommandComponent {
                    part: "mv".to_string(),
                    meaning: "Move (rename) files".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["mv old.txt new.txt", "mv file.txt dir/"]),

            // mkdir 命令
            ("mkdir", "Make directories", vec![
                CommandComponent {
                    part: "mkdir".to_string(),
                    meaning: "Make directories".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["mkdir newdir", "mkdir -p a/b/c"]),

            // rmdir 命令
            ("rmdir", "Remove empty directories", vec![
                CommandComponent {
                    part: "rmdir".to_string(),
                    meaning: "Remove empty directories".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["rmdir emptydir/"]),

            // touch 命令
            ("touch", "Change file timestamps", vec![
                CommandComponent {
                    part: "touch".to_string(),
                    meaning: "Change file timestamps or create empty files".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["touch newfile.txt", "touch -t 202301011200 file.txt"]),

            // echo 命令
            ("echo", "Display a line of text", vec![
                CommandComponent {
                    part: "echo".to_string(),
                    meaning: "Display a line of text".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["echo 'Hello World'", "echo $PATH"]),

            // head 命令
            ("head", "Output first part of files", vec![
                CommandComponent {
                    part: "head".to_string(),
                    meaning: "Output the first part of files".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["head file.txt", "head -n 20 file.txt"]),

            // tail 命令
            ("tail", "Output last part of files", vec![
                CommandComponent {
                    part: "tail".to_string(),
                    meaning: "Output the last part of files".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["tail file.txt", "tail -f /var/log/syslog"]),

            // less 命令
            ("less", "Pager program", vec![
                CommandComponent {
                    part: "less".to_string(),
                    meaning: "Opposite of more (pager program)".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["less file.txt"]),

            // more 命令
            ("more", "Pager program", vec![
                CommandComponent {
                    part: "more".to_string(),
                    meaning: "File perusal filter for crt viewing".to_string(),
                    category: ComponentCategory::Command,
                },
            ], vec!["more file.txt"]),
        ];

        // 提取基本命令
        let base_cmd = cmd.split_whitespace().next().unwrap_or(cmd);

        for (pattern, summary, components, examples) in explanations {
            if base_cmd == pattern {
                let examples: Vec<Example> = examples.into_iter().map(|e| Example {
                    description: "Example usage".to_string(),
                    command: e.to_string(),
                    explanation: format!("Common usage of {}", pattern),
                }).collect();

                let warnings = if pattern == "rm" {
                    vec![
                        "rm permanently deletes files - they cannot be recovered from trash".to_string(),
                        "Be careful with wildcards and recursive flags (-r, -rf)".to_string(),
                    ]
                } else if pattern == "chmod" || pattern == "chown" {
                    vec![
                        "Changing permissions or ownership can affect system security".to_string(),
                        "Be cautious when modifying system files".to_string(),
                    ]
                } else {
                    vec![]
                };

                return Some(ExplanationResult {
                    summary: summary.to_string(),
                    detailed_explanation: format!("The {} command {}.", pattern, summary.to_lowercase()),
                    components,
                    examples,
                    warnings,
                    related_commands: vec![],
                    documentation_links: vec![
                        format!("man {}", pattern),
                        format!("https://man7.org/linux/man-pages/man1/{}.1.html", pattern),
                    ],
                });
            }
        }

        None
    }

    /// AI详细解释
    async fn ai_explanation(&self, request: &ExplanationRequest) -> Result<ExplanationResult> {
        let detail_level = request.detail_level;
        let focus_area = request.focus_area.unwrap_or(FocusArea::General);

        let system_prompt = format!(
            r#"You are an expert shell command explainer.
Provide clear, educational explanations of shell commands.

Detail Level: {}
Focus Area: {}

Format your response as:
SUMMARY: One-sentence summary
DETAILED: Detailed explanation (2-4 sentences)

COMPONENT 1:
Part: <command part>
Meaning: <what it means>
Category: <command/option/argument/pipe/redirection/variable/subcommand/operator>

COMPONENT 2:
...

EXAMPLE 1:
Description: <what this example shows>
Command: <example command>
Explanation: <why/when to use this>

EXAMPLE 2:
...

WARNINGS:
- <warning 1>
- <warning 2>

RELATED:
- <related command 1>
- <related command 2>

DOCS:
- <man page>
- <online doc URL>"#,
            detail_level.description(),
            focus_area.as_str()
        );

        let user_prompt = format!(
            "Explain this command: {}\n\nOutput:",
            request.command
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
            max_tokens: 1500,
            temperature: 0.3,
            stream: false,
        };

        let response = self.provider.chat(chat_request).await?;
        let result = self.parse_explanation_response(&response.content);

        Ok(result)
    }

    /// 解析解释响应
    fn parse_explanation_response(&self, content: &str) -> ExplanationResult {
        let mut summary = String::new();
        let mut detailed_explanation = String::new();
        let mut components = Vec::new();
        let mut examples = Vec::new();
        let mut warnings = Vec::new();
        let mut related_commands = Vec::new();
        let mut documentation_links = Vec::new();

        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        let mut current_component: Option<(String, String, String)> = None;
        let mut current_example: Option<(String, String, String)> = None;

        while i < lines.len() {
            let line = lines[i].trim();

            if line.starts_with("SUMMARY:") {
                summary = line[8..].trim().to_string();
            } else if line.starts_with("DETAILED:") {
                detailed_explanation = line[9..].trim().to_string();
                i += 1;
                while i < lines.len() && !lines[i].trim().starts_with("COMPONENT") && !lines[i].trim().starts_with("EXAMPLE") {
                    detailed_explanation.push(' ');
                    detailed_explanation.push_str(lines[i].trim());
                    i += 1;
                }
                continue;
            } else if line.starts_with("COMPONENT") {
                // 保存之前的组件
                if let Some((part, meaning, category)) = current_component.as_ref() {
                    let cat = match category.to_lowercase().as_str() {
                        "option" => ComponentCategory::Option,
                        "argument" => ComponentCategory::Argument,
                        "pipe" => ComponentCategory::Pipe,
                        "redirection" => ComponentCategory::Redirection,
                        "variable" => ComponentCategory::Variable,
                        "subcommand" => ComponentCategory::Subcommand,
                        "operator" => ComponentCategory::Operator,
                        _ => ComponentCategory::Command,
                    };
                    components.push(CommandComponent {
                        part: part.clone(),
                        meaning: meaning.clone(),
                        category: cat,
                    });
                }
                current_component = Some((String::new(), String::new(), String::new()));
            } else if line.starts_with("Part:") && current_component.is_some() {
                current_component.as_mut().unwrap().0 = line[5..].trim().to_string();
            } else if line.starts_with("Meaning:") && current_component.is_some() {
                current_component.as_mut().unwrap().1 = line[8..].trim().to_string();
            } else if line.starts_with("Category:") && current_component.is_some() {
                current_component.as_mut().unwrap().2 = line[9..].trim().to_string();
            } else if line.starts_with("EXAMPLE") {
                // 保存之前的示例
                if let Some((desc, cmd, expl)) = current_example.as_ref() {
                    examples.push(Example {
                        description: desc.clone(),
                        command: cmd.clone(),
                        explanation: expl.clone(),
                    });
                }
                current_example = Some((String::new(), String::new(), String::new()));
            } else if line.starts_with("Description:") && current_example.is_some() {
                current_example.as_mut().unwrap().0 = line[12..].trim().to_string();
            } else if line.starts_with("Command:") && current_example.is_some() {
                current_example.as_mut().unwrap().1 = line[8..].trim().to_string();
            } else if line.starts_with("Explanation:") && current_example.is_some() {
                current_example.as_mut().unwrap().2 = line[12..].trim().to_string();
            } else if line.starts_with("WARNINGS:") {
                // 保存最后的组件和示例
                if let Some((part, meaning, category)) = current_component.as_ref() {
                    let cat = match category.to_lowercase().as_str() {
                        "option" => ComponentCategory::Option,
                        "argument" => ComponentCategory::Argument,
                        _ => ComponentCategory::Command,
                    };
                    components.push(CommandComponent {
                        part: part.clone(),
                        meaning: meaning.clone(),
                        category: cat,
                    });
                }
                if let Some((desc, cmd, expl)) = current_example.as_ref() {
                    examples.push(Example {
                        description: desc.clone(),
                        command: cmd.clone(),
                        explanation: expl.clone(),
                    });
                }

                i += 1;
                while i < lines.len() && !lines[i].trim().starts_with("RELATED") && !lines[i].trim().starts_with("DOCS") {
                    let warning = lines[i].trim();
                    if warning.starts_with("-") {
                        warnings.push(warning[1..].trim().to_string());
                    }
                    i += 1;
                }
                continue;
            } else if line.starts_with("RELATED:") {
                i += 1;
                while i < lines.len() && !lines[i].trim().starts_with("DOCS") {
                    let related = lines[i].trim();
                    if related.starts_with("-") {
                        related_commands.push(related[1..].trim().to_string());
                    }
                    i += 1;
                }
                continue;
            } else if line.starts_with("DOCS:") {
                i += 1;
                while i < lines.len() {
                    let doc = lines[i].trim();
                    if doc.starts_with("-") {
                        documentation_links.push(doc[1..].trim().to_string());
                    }
                    i += 1;
                }
                break;
            }

            i += 1;
        }

        // 默认值
        if summary.is_empty() {
            summary = "Command explanation unavailable".to_string();
        }
        if detailed_explanation.is_empty() {
            detailed_explanation = summary.clone();
        }

        ExplanationResult {
            summary,
            detailed_explanation,
            components,
            examples,
            warnings,
            related_commands,
            documentation_links,
        }
    }
}

/// 解释命令的主函数
pub async fn explain_command(
    provider: &Arc<dyn AiProvider>,
    request: &ExplanationRequest,
) -> Result<ExplanationResult> {
    let explainer = CommandExplainer::new(Arc::clone(provider));
    explainer.explain(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_terminal::providers::MockProvider;

    #[test]
    fn test_pattern_explanation() {
        let explainer = CommandExplainer::new(Arc::new(MockProvider::new()));

        let request = ExplanationRequest {
            command: "ls".to_string(),
            detail_level: DetailLevel::Brief,
            focus_area: None,
        };

        let result = explainer.pattern_based_explanation(&request);
        assert!(result.is_some());

        let explanation = result.unwrap();
        assert_eq!(explanation.summary, "List directory contents");
    }

    #[test]
    fn test_detail_levels() {
        assert_eq!(DetailLevel::Brief.as_str(), "brief");
        assert_eq!(DetailLevel::Standard.as_str(), "standard");
        assert_eq!(DetailLevel::Detailed.as_str(), "detailed");
        assert_eq!(DetailLevel::Technical.as_str(), "technical");
    }
}
