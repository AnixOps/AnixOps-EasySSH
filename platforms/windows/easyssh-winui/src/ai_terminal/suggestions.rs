#![allow(dead_code)]

//! AI建议引擎模块
//!
//! 基于上下文生成智能建议

use super::TerminalContext;

/// AI建议
#[derive(Debug, Clone)]
pub struct AiSuggestion {
    pub suggestion_type: SuggestionType,
    pub message: String,
    pub action: Option<String>,
    pub priority: SuggestionPriority,
    pub icon: Option<String>,
}

/// 建议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionType {
    CommandCompletion,
    ErrorRecovery,
    BestPractice,
    SecurityWarning,
    PerformanceTip,
    TimeSaver,
    LearningOpportunity,
}

impl SuggestionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CommandCompletion => "completion",
            Self::ErrorRecovery => "recovery",
            Self::BestPractice => "best_practice",
            Self::SecurityWarning => "security",
            Self::PerformanceTip => "performance",
            Self::TimeSaver => "timesaver",
            Self::LearningOpportunity => "learning",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::CommandCompletion => "command",
            Self::ErrorRecovery => "error",
            Self::BestPractice => "check",
            Self::SecurityWarning => "shield",
            Self::PerformanceTip => "zap",
            Self::TimeSaver => "clock",
            Self::LearningOpportunity => "book",
        }
    }
}

/// 建议优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SuggestionPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl SuggestionPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// 建议引擎
pub struct SuggestionEngine;

impl SuggestionEngine {
    pub fn new() -> Self {
        Self
    }

    /// 基于上下文生成建议
    pub fn get_suggestions(&self, context: &TerminalContext) -> Vec<AiSuggestion> {
        let mut suggestions = Vec::new();

        // 根据命令历史生成建议
        if let Some(last_cmd) = context.command_history.last() {
            suggestions.extend(self.suggest_for_command(last_cmd, context));
        }

        // 基于工作目录的建议
        suggestions.extend(self.suggest_for_directory(&context.working_directory));

        // 基于使用模式的建议
        suggestions.extend(self.suggest_from_patterns(&context.command_history));

        // 按优先级排序
        suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));

        suggestions.truncate(5);
        suggestions
    }

    /// 针对特定命令的建议
    fn suggest_for_command(&self, command: &str, _context: &TerminalContext) -> Vec<AiSuggestion> {
        let mut suggestions = Vec::new();

        // 建议更高效的替代方案
        if command == "ls" {
            suggestions.push(AiSuggestion {
                suggestion_type: SuggestionType::TimeSaver,
                message: "Use 'exa' for a modern alternative to ls with better output".to_string(),
                action: Some("exa".to_string()),
                priority: SuggestionPriority::Low,
                icon: Some("zap".to_string()),
            });
        }

        if command == "cat" {
            suggestions.push(AiSuggestion {
                suggestion_type: SuggestionType::PerformanceTip,
                message: "For large files, consider using 'less' instead of 'cat'".to_string(),
                action: Some("less".to_string()),
                priority: SuggestionPriority::Medium,
                icon: Some("zap".to_string()),
            });
        }

        if command == "grep" && !command.contains("-r") {
            suggestions.push(AiSuggestion {
                suggestion_type: SuggestionType::TimeSaver,
                message: "Use 'rg' (ripgrep) for faster recursive searching".to_string(),
                action: Some("rg".to_string()),
                priority: SuggestionPriority::Low,
                icon: Some("zap".to_string()),
            });
        }

        // 建议安全实践
        if command.starts_with("rm ") && !command.contains("-i") {
            suggestions.push(AiSuggestion {
                suggestion_type: SuggestionType::SecurityWarning,
                message: "Consider adding -i flag to rm for confirmation prompts".to_string(),
                action: Some(command.replacen("rm ", "rm -i ", 1)),
                priority: SuggestionPriority::High,
                icon: Some("shield".to_string()),
            });
        }

        if command.starts_with("chmod ") && command.contains(" 777 ") {
            suggestions.push(AiSuggestion {
                suggestion_type: SuggestionType::SecurityWarning,
                message:
                    "777 permissions are too broad. Consider 755 for directories or 644 for files."
                        .to_string(),
                action: Some(command.replace(" 777 ", " 755 ")),
                priority: SuggestionPriority::High,
                icon: Some("shield".to_string()),
            });
        }

        // 建议学习机会
        if command.starts_with("cd ") && command.contains("..") {
            suggestions.push(AiSuggestion {
                suggestion_type: SuggestionType::LearningOpportunity,
                message: "Pro tip: 'cd -' switches to the previous directory".to_string(),
                action: Some("cd -".to_string()),
                priority: SuggestionPriority::Low,
                icon: Some("book".to_string()),
            });
        }

        suggestions
    }

    /// 基于目录的建议
    fn suggest_for_directory(&self, directory: &str) -> Vec<AiSuggestion> {
        let mut suggestions = Vec::new();

        if directory.contains(".git") {
            suggestions.push(AiSuggestion {
                suggestion_type: SuggestionType::BestPractice,
                message: "You're in a git repository. Use 'git status' to check changes."
                    .to_string(),
                action: Some("git status".to_string()),
                priority: SuggestionPriority::Low,
                icon: Some("check".to_string()),
            });
        }

        if directory.contains("node_modules") {
            suggestions.push(AiSuggestion {
                suggestion_type: SuggestionType::PerformanceTip,
                message: "node_modules can be large. Use 'ncdu' to analyze disk usage.".to_string(),
                action: Some("ncdu".to_string()),
                priority: SuggestionPriority::Low,
                icon: Some("zap".to_string()),
            });
        }

        suggestions
    }

    /// 基于模式的建议
    fn suggest_from_patterns(&self, history: &[String]) -> Vec<AiSuggestion> {
        let mut suggestions = Vec::new();

        // 检测重复命令
        if history.len() >= 3 {
            let recent: Vec<_> = history.iter().rev().take(3).collect();
            if recent[0] == recent[1] && recent[1] == recent[2] {
                suggestions.push(AiSuggestion {
                    suggestion_type: SuggestionType::TimeSaver,
                    message: "You've run this command 3 times. Consider creating an alias."
                        .to_string(),
                    action: Some(format!("alias {}='{}'", "mycommand", recent[0])),
                    priority: SuggestionPriority::Low,
                    icon: Some("clock".to_string()),
                });
            }
        }

        // 检测可能的打字错误
        if let Some(last_cmd) = history.last() {
            if last_cmd.contains("sl") && !last_cmd.contains("ls") {
                suggestions.push(AiSuggestion {
                    suggestion_type: SuggestionType::ErrorRecovery,
                    message: "Did you mean 'ls' instead of 'sl'?".to_string(),
                    action: Some(last_cmd.replace("sl", "ls")),
                    priority: SuggestionPriority::Medium,
                    icon: Some("error".to_string()),
                });
            }
        }

        suggestions
    }
}

impl Default for SuggestionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggestions_for_rm() {
        let engine = SuggestionEngine::new();

        let context = TerminalContext {
            working_directory: "/home".to_string(),
            command_history: vec!["rm file.txt".to_string()],
            ..Default::default()
        };

        let suggestions = engine.get_suggestions(&context);
        assert!(suggestions
            .iter()
            .any(|s| s.suggestion_type == SuggestionType::SecurityWarning));
    }

    #[test]
    fn test_suggestions_for_chmod() {
        let engine = SuggestionEngine::new();

        let context = TerminalContext {
            working_directory: "/home".to_string(),
            command_history: vec!["chmod 777 file.txt".to_string()],
            ..Default::default()
        };

        let suggestions = engine.get_suggestions(&context);
        assert!(suggestions.iter().any(|s| s.message.contains("too broad")));
    }

    #[test]
    fn test_priority_ordering() {
        assert!(SuggestionPriority::Critical > SuggestionPriority::High);
        assert!(SuggestionPriority::High > SuggestionPriority::Medium);
        assert!(SuggestionPriority::Medium > SuggestionPriority::Low);
    }
}
