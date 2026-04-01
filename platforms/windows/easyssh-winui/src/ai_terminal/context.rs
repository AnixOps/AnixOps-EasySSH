#![allow(dead_code)]

//! 终端上下文追踪模块
//!
//! 追踪终端会话上下文，为AI提供历史信息

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

/// 终端上下文
#[derive(Debug, Clone, Default)]
pub struct TerminalContext {
    pub working_directory: String,
    pub command_history: Vec<String>,
    pub environment_variables: HashMap<String, String>,
    pub user: String,
    pub hostname: String,
    pub shell: String,
    pub os_type: String,
}

/// 命令历史记录
#[derive(Debug, Clone)]
pub struct CommandHistory {
    pub command: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub timestamp: std::time::SystemTime,
    pub duration_ms: u64,
}

/// 会话上下文
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub session_id: String,
    pub start_time: std::time::SystemTime,
    pub command_history: VecDeque<CommandHistory>,
    pub working_directory: String,
    pub last_activity: std::time::SystemTime,
}

impl SessionContext {
    pub fn new(session_id: String) -> Self {
        let now = std::time::SystemTime::now();
        Self {
            session_id,
            start_time: now,
            command_history: VecDeque::with_capacity(100),
            working_directory: String::new(),
            last_activity: now,
        }
    }

    pub fn add_command(&mut self, command: String, output: String, exit_code: Option<i32>) {
        let now = std::time::SystemTime::now();

        self.command_history.push_back(CommandHistory {
            command,
            output,
            exit_code,
            timestamp: now,
            duration_ms: 0, // Could be measured if needed
        });

        // 限制历史记录大小
        if self.command_history.len() > 100 {
            self.command_history.pop_front();
        }

        self.last_activity = now;
    }

    pub fn update_directory(&mut self, directory: String) {
        self.working_directory = directory;
        self.last_activity = std::time::SystemTime::now();
    }

    pub fn get_recent_commands(&self, count: usize) -> Vec<&CommandHistory> {
        self.command_history
            .iter()
            .rev()
            .take(count)
            .collect()
    }
}

/// 上下文追踪器
pub struct ContextTracker {
    sessions: Arc<RwLock<HashMap<String, SessionContext>>>,
    max_history: usize,
}

impl ContextTracker {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_history: 50,
        }
    }

    pub fn with_capacity(max_history: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_history,
        }
    }

    /// 创建新会话
    pub fn create_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(
            session_id.to_string(),
            SessionContext::new(session_id.to_string()),
        );
    }

    /// 添加命令到会话
    pub fn add_command(
        &self,
        session_id: &str,
        command: &str,
        output: &str,
    ) {
        let mut sessions = self.sessions.write().unwrap();

        if let Some(session) = sessions.get_mut(session_id) {
            session.add_command(command.to_string(), output.to_string(), None);
        } else {
            // 创建新会话并添加命令
            let mut session = SessionContext::new(session_id.to_string());
            session.add_command(command.to_string(), output.to_string(), None);
            sessions.insert(session_id.to_string(), session);
        }
    }

    /// 更新工作目录
    pub fn update_directory(&self, session_id: &str, directory: &str) {
        let mut sessions = self.sessions.write().unwrap();

        if let Some(session) = sessions.get_mut(session_id) {
            session.update_directory(directory.to_string());
        }
    }

    /// 获取会话上下文
    pub fn get_context(&self, session_id: &str) -> TerminalContext {
        let sessions = self.sessions.read().unwrap();

        if let Some(session) = sessions.get(session_id) {
            TerminalContext {
                working_directory: session.working_directory.clone(),
                command_history: session
                    .command_history
                    .iter()
                    .map(|h| h.command.clone())
                    .collect(),
                environment_variables: HashMap::new(),
                user: String::new(),
                hostname: String::new(),
                shell: String::new(),
                os_type: String::new(),
            }
        } else {
            TerminalContext::default()
        }
    }

    /// 获取会话
    pub fn get_session(&self, session_id: &str) -> Option<SessionContext> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(session_id).cloned()
    }

    /// 删除会话
    pub fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(session_id);
    }

    /// 列出所有会话
    pub fn list_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.read().unwrap();
        sessions.keys().cloned().collect()
    }

    /// 清理不活跃的会话
    pub fn cleanup_inactive(&self, max_age_secs: u64) {
        let mut sessions = self.sessions.write().unwrap();
        let now = std::time::SystemTime::now();

        sessions.retain(|_, session| {
            if let Ok(elapsed) = now.duration_since(session.last_activity) {
                elapsed.as_secs() < max_age_secs
            } else {
                true
            }
        });
    }

    /// 获取全局上下文（所有会话的汇总）
    pub fn get_global_context(&self) -> TerminalContext {
        let sessions = self.sessions.read().unwrap();

        let mut all_commands = Vec::new();
        let mut working_dir = String::new();

        for session in sessions.values() {
            all_commands.extend(
                session
                    .command_history
                    .iter()
                    .map(|h| h.command.clone()),
            );
            if working_dir.is_empty() && !session.working_directory.is_empty() {
                working_dir = session.working_directory.clone();
            }
        }

        // 去重并保留最近的命令
        all_commands.reverse();
        all_commands.dedup();
        all_commands.reverse();
        all_commands.truncate(self.max_history);

        TerminalContext {
            working_directory: working_dir,
            command_history: all_commands,
            environment_variables: HashMap::new(),
            user: String::new(),
            hostname: String::new(),
            shell: String::new(),
            os_type: String::new(),
        }
    }
}

impl Default for ContextTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_tracker() {
        let tracker = ContextTracker::new();

        tracker.create_session("test-session");
        tracker.add_command("test-session", "ls -la", "file1 file2");
        tracker.update_directory("test-session", "/home/user");

        let context = tracker.get_context("test-session");
        assert_eq!(context.working_directory, "/home/user");
        assert_eq!(context.command_history.len(), 1);
        assert_eq!(context.command_history[0], "ls -la");
    }

    #[test]
    fn test_session_history_limit() {
        let tracker = ContextTracker::new();
        tracker.create_session("test-session");

        // 添加超过限制的命令
        for i in 0..150 {
            tracker.add_command("test-session", &format!("cmd{}", i), "output");
        }

        let session = tracker.get_session("test-session").unwrap();
        assert!(session.command_history.len() <= 100);
    }

    #[test]
    fn test_cleanup_inactive() {
        let tracker = ContextTracker::new();
        tracker.create_session("active-session");
        tracker.create_session("inactive-session");

        // 模拟不活跃会话（通过直接修改）
        {
            let mut sessions = tracker.sessions.write().unwrap();
            if let Some(session) = sessions.get_mut("inactive-session") {
                session.last_activity = std::time::SystemTime::UNIX_EPOCH;
            }
        }

        tracker.cleanup_inactive(60); // 60秒

        assert!(tracker.get_session("active-session").is_some());
        assert!(tracker.get_session("inactive-session").is_none());
    }
}
