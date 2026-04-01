use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Recorded macro action types
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MacroActionType {
    /// SSH command executed
    SshCommand,
    /// File uploaded
    FileUpload,
    /// File downloaded
    FileDownload,
    /// Directory changed
    ChangeDirectory,
    /// Command output captured
    CaptureOutput,
    /// Wait/pause
    Wait,
    /// Input provided (response to prompt)
    ProvideInput,
    /// File edited
    EditFile,
    /// Local command executed
    LocalCommand,
}

/// Single recorded action in a macro
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacroAction {
    pub id: String,
    pub action_type: MacroActionType,
    /// Timestamp when action occurred
    pub timestamp: DateTime<Utc>,
    /// Time since previous action (for replay timing)
    pub delay_ms: u64,
    /// Action-specific data
    pub data: MacroActionData,
    /// Whether this action can be edited
    pub editable: bool,
    /// Whether this action is enabled for replay
    pub enabled: bool,
    /// Optional description/comment
    pub description: Option<String>,
    /// Screenshot or visual reference (for UI automation)
    pub visual_ref: Option<String>,
}

/// Action-specific data payload
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MacroActionData {
    SshCommand {
        command: String,
        working_dir: Option<String>,
        expected_prompt: Option<String>,
        timeout_secs: u64,
    },
    FileUpload {
        local_path: String,
        remote_path: String,
        create_dirs: bool,
    },
    FileDownload {
        remote_path: String,
        local_path: String,
        create_dirs: bool,
    },
    ChangeDirectory {
        path: String,
    },
    CaptureOutput {
        pattern: Option<String>,
        save_to_variable: Option<String>,
    },
    Wait {
        duration_secs: u64,
        /// Wait for specific pattern in output
        wait_for_pattern: Option<String>,
    },
    ProvideInput {
        input: String,
        /// Whether input is sensitive (password, etc.)
        is_sensitive: bool,
        /// Expected prompt that triggered this input
        prompt_pattern: Option<String>,
    },
    EditFile {
        remote_path: String,
        /// Edit operations: replace, insert, delete
        operations: Vec<FileEditOperation>,
    },
    LocalCommand {
        command: String,
        working_dir: Option<String>,
    },
}

/// File edit operation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileEditOperation {
    pub operation_type: EditOperationType,
    /// Line number or pattern to match
    pub target: String,
    /// New content (for replace/insert)
    pub content: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EditOperationType {
    Replace,
    Insert,
    Delete,
    Append,
}

/// Complete recorded macro
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// When macro was recorded
    pub recorded_at: DateTime<Utc>,
    /// Server used during recording (for context)
    pub server_context: Option<MacroServerContext>,
    /// Sequence of actions
    pub actions: Vec<MacroAction>,
    /// Total recording duration in milliseconds
    pub total_duration_ms: u64,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Whether to use original timing during replay
    pub use_original_timing: bool,
    /// Default replay speed multiplier (1.0 = normal)
    pub replay_speed: f64,
    /// Variables extracted from the macro
    pub variables: Vec<MacroVariable>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacroServerContext {
    pub server_id: String,
    pub server_name: String,
    pub host: String,
    pub username: String,
    /// Working directory at start of recording
    pub initial_dir: String,
    /// Environment variables
    pub env_vars: std::collections::HashMap<String, String>,
}

/// Variable extracted from macro
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacroVariable {
    pub name: String,
    pub source_action_id: String,
    pub extraction_pattern: String,
    pub default_value: Option<String>,
}

impl Macro {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: None,
            recorded_at: Utc::now(),
            server_context: None,
            actions: Vec::new(),
            total_duration_ms: 0,
            tags: Vec::new(),
            use_original_timing: false,
            replay_speed: 1.0,
            variables: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn add_action(&mut self, action: MacroAction) {
        self.actions.push(action);
        self.update_duration();
    }

    pub fn remove_action(&mut self, action_id: &str) -> Option<MacroAction> {
        let idx = self.actions.iter().position(|a| a.id == action_id)?;
        let action = self.actions.remove(idx);
        self.update_duration();
        Some(action)
    }

    pub fn update_action(
        &mut self,
        action_id: &str,
        new_action: MacroAction,
    ) -> Result<(), String> {
        if let Some(idx) = self.actions.iter().position(|a| a.id == action_id) {
            self.actions[idx] = new_action;
            self.update_duration();
            Ok(())
        } else {
            Err(format!("Action {} not found", action_id))
        }
    }

    fn update_duration(&mut self) {
        if self.actions.is_empty() {
            self.total_duration_ms = 0;
            return;
        }

        let first_ts = self.actions.first().unwrap().timestamp;
        let last_ts = self.actions.last().unwrap().timestamp;
        self.total_duration_ms = (last_ts - first_ts).num_milliseconds() as u64;
    }

    /// Convert macro to workflow
    pub fn to_workflow(&self) -> crate::workflow_engine::Workflow {
        let mut workflow = crate::workflow_engine::Workflow::new(&self.name)
            .with_description(self.description.as_deref().unwrap_or(""));

        let mut prev_step_id: Option<String> = None;

        for action in &self.actions {
            if !action.enabled {
                continue;
            }

            let step = action.to_workflow_step();
            let step_id = workflow.add_step(step);

            if let Some(prev) = prev_step_id {
                let _ = workflow.connect(&prev, &step_id);
            }

            prev_step_id = Some(step_id);
        }

        workflow
    }

    /// Extract suggested variables from command patterns
    pub fn suggest_variables(&self) -> Vec<MacroVariable> {
        let mut suggestions = Vec::new();
        let mut var_counter = 0;

        for action in &self.actions {
            if let MacroActionData::SshCommand { command, .. } = &action.data {
                // Look for patterns that might be variables
                // Simple IP-like pattern detection (contains dots and digits)
                if command.contains('.') && command.chars().any(|c| c.is_ascii_digit()) {
                    // Check for potential IP addresses
                    let parts: Vec<&str> = command.split_whitespace().collect();
                    for part in &parts {
                        if part.split('.').count() == 4 {
                            var_counter += 1;
                            suggestions.push(MacroVariable {
                                name: format!("ip_address_{}", var_counter),
                                source_action_id: action.id.clone(),
                                extraction_pattern: "IP_ADDRESS".to_string(),
                                default_value: None,
                            });
                            break;
                        }
                    }
                }

                // Common variable patterns
                for pattern in &["production", "staging", "development", "test"] {
                    if command.contains(pattern) {
                        suggestions.push(MacroVariable {
                            name: "environment".to_string(),
                            source_action_id: action.id.clone(),
                            extraction_pattern: pattern.to_string(),
                            default_value: Some(pattern.to_string()),
                        });
                    }
                }
            }
        }

        suggestions
    }
}

impl MacroAction {
    /// Convert macro action to workflow step
    pub fn to_workflow_step(&self) -> crate::workflow_engine::WorkflowStep {
        use crate::workflow_engine::*;

        let step_type = match self.action_type {
            MacroActionType::SshCommand => StepType::SshCommand,
            MacroActionType::FileUpload => StepType::SftpUpload,
            MacroActionType::FileDownload => StepType::SftpDownload,
            MacroActionType::Wait => StepType::Wait,
            MacroActionType::LocalCommand => StepType::LocalCommand,
            _ => StepType::SshCommand, // Default fallback
        };

        let config = match &self.data {
            MacroActionData::SshCommand {
                command,
                working_dir,
                ..
            } => StepConfig::SshCommand {
                command: command.clone(),
                working_dir: working_dir.clone(),
                env_vars: std::collections::HashMap::new(),
                capture_output: true,
                fail_on_error: true,
            },
            MacroActionData::FileUpload {
                local_path,
                remote_path,
                create_dirs,
            } => StepConfig::SftpUpload {
                local_path: local_path.clone(),
                remote_path: remote_path.clone(),
                permissions: None,
                create_dirs: *create_dirs,
            },
            MacroActionData::FileDownload {
                remote_path,
                local_path,
                create_dirs,
            } => StepConfig::SftpDownload {
                remote_path: remote_path.clone(),
                local_path: local_path.clone(),
                create_dirs: *create_dirs,
            },
            MacroActionData::Wait { duration_secs, .. } => StepConfig::Wait {
                duration_secs: *duration_secs,
            },
            MacroActionData::LocalCommand {
                command,
                working_dir,
            } => StepConfig::LocalCommand {
                command: command.clone(),
                working_dir: working_dir.clone(),
                env_vars: std::collections::HashMap::new(),
                capture_output: true,
            },
            _ => StepConfig::default_for(&step_type),
        };

        let mut step = WorkflowStep::new(step_type, &format!("Action: {:?}", self.action_type));
        step.config = config;
        step.description = self.description.clone();
        step
    }
}

/// Macro recorder state machine
pub struct MacroRecorder {
    state: RecorderState,
    current_macro: Option<Macro>,
    last_action_time: Option<Instant>,
    recording_buffer: VecDeque<MacroAction>,
    server_context: Option<MacroServerContext>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RecorderState {
    Idle,
    Recording,
    Paused,
    Saving,
}

impl MacroRecorder {
    pub fn new() -> Self {
        Self {
            state: RecorderState::Idle,
            current_macro: None,
            last_action_time: None,
            recording_buffer: VecDeque::new(),
            server_context: None,
        }
    }

    pub fn start_recording(&mut self, name: &str, server_context: Option<MacroServerContext>) {
        self.current_macro = Some(Macro::new(name));
        self.server_context = server_context.clone();

        if let Some(ref mut m) = self.current_macro {
            m.server_context = server_context;
        }

        self.state = RecorderState::Recording;
        self.last_action_time = Some(Instant::now());
        self.recording_buffer.clear();
    }

    pub fn pause_recording(&mut self) {
        if self.state == RecorderState::Recording {
            self.state = RecorderState::Paused;
        }
    }

    pub fn resume_recording(&mut self) {
        if self.state == RecorderState::Paused {
            self.state = RecorderState::Recording;
            self.last_action_time = Some(Instant::now());
        }
    }

    pub fn stop_recording(&mut self) -> Option<Macro> {
        if self.state != RecorderState::Idle {
            // Flush buffer to macro
            if let Some(ref mut m) = self.current_macro {
                while let Some(action) = self.recording_buffer.pop_front() {
                    m.add_action(action);
                }
            }

            self.state = RecorderState::Idle;
            self.last_action_time = None;
            self.recording_buffer.clear();
            self.current_macro.take()
        } else {
            None
        }
    }

    pub fn record_action(&mut self, action_type: MacroActionType, data: MacroActionData) {
        if self.state != RecorderState::Recording {
            return;
        }

        let now = Instant::now();
        let delay_ms = self
            .last_action_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);

        let action = MacroAction {
            id: Uuid::new_v4().to_string(),
            action_type,
            timestamp: Utc::now(),
            delay_ms,
            data,
            editable: true,
            enabled: true,
            description: None,
            visual_ref: None,
        };

        self.recording_buffer.push_back(action);
        self.last_action_time = Some(now);

        // Auto-flush if buffer gets too large
        if self.recording_buffer.len() > 100 {
            self.flush_buffer();
        }
    }

    fn flush_buffer(&mut self) {
        if let Some(ref mut m) = self.current_macro {
            while let Some(action) = self.recording_buffer.pop_front() {
                m.add_action(action);
            }
        }
    }

    pub fn record_ssh_command(&mut self, command: &str, working_dir: Option<&str>) {
        self.record_action(
            MacroActionType::SshCommand,
            MacroActionData::SshCommand {
                command: command.to_string(),
                working_dir: working_dir.map(|s| s.to_string()),
                expected_prompt: None,
                timeout_secs: 30,
            },
        );
    }

    pub fn record_file_upload(&mut self, local_path: &str, remote_path: &str) {
        self.record_action(
            MacroActionType::FileUpload,
            MacroActionData::FileUpload {
                local_path: local_path.to_string(),
                remote_path: remote_path.to_string(),
                create_dirs: true,
            },
        );
    }

    pub fn record_file_download(&mut self, remote_path: &str, local_path: &str) {
        self.record_action(
            MacroActionType::FileDownload,
            MacroActionData::FileDownload {
                remote_path: remote_path.to_string(),
                local_path: local_path.to_string(),
                create_dirs: true,
            },
        );
    }

    pub fn record_wait(&mut self, duration_secs: u64) {
        self.record_action(
            MacroActionType::Wait,
            MacroActionData::Wait {
                duration_secs,
                wait_for_pattern: None,
            },
        );
    }

    pub fn record_input(&mut self, input: &str, is_sensitive: bool) {
        self.record_action(
            MacroActionType::ProvideInput,
            MacroActionData::ProvideInput {
                input: input.to_string(),
                is_sensitive,
                prompt_pattern: None,
            },
        );
    }

    pub fn get_state(&self) -> RecorderState {
        self.state
    }

    pub fn is_recording(&self) -> bool {
        self.state == RecorderState::Recording
    }

    pub fn get_current_macro(&self) -> Option<&Macro> {
        self.current_macro.as_ref()
    }

    pub fn get_buffered_action_count(&self) -> usize {
        self.recording_buffer.len()
    }

    pub fn get_recording_duration(&self) -> Duration {
        self.last_action_time
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO)
    }
}

impl Default for MacroRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro replay/simulation engine
pub struct MacroReplayer {
    macro_data: Macro,
    current_action_index: usize,
    state: ReplayState,
    variables: std::collections::HashMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReplayState {
    Idle,
    Running,
    Paused,
    Completed,
    Failed,
}

impl MacroReplayer {
    pub fn new(macro_data: Macro) -> Self {
        Self {
            macro_data,
            current_action_index: 0,
            state: ReplayState::Idle,
            variables: std::collections::HashMap::new(),
        }
    }

    pub fn start(&mut self) {
        self.state = ReplayState::Running;
        self.current_action_index = 0;
    }

    pub fn pause(&mut self) {
        if self.state == ReplayState::Running {
            self.state = ReplayState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == ReplayState::Paused {
            self.state = ReplayState::Running;
        }
    }

    pub fn stop(&mut self) {
        self.state = ReplayState::Idle;
        self.current_action_index = 0;
    }

    pub fn get_next_action(&self) -> Option<&MacroAction> {
        if self.current_action_index < self.macro_data.actions.len() {
            Some(&self.macro_data.actions[self.current_action_index])
        } else {
            None
        }
    }

    pub fn advance(&mut self) {
        self.current_action_index += 1;
        if self.current_action_index >= self.macro_data.actions.len() {
            self.state = ReplayState::Completed;
        }
    }

    pub fn set_variable(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_string(), value.to_string());
    }

    pub fn resolve_variables(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (key, value) in &self.variables {
            let pattern = format!("{{{{{}}}}}", key);
            result = result.replace(&pattern, value);
        }
        result
    }

    pub fn get_state(&self) -> ReplayState {
        self.state
    }

    pub fn get_progress(&self) -> f32 {
        if self.macro_data.actions.is_empty() {
            return 1.0;
        }
        self.current_action_index as f32 / self.macro_data.actions.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_creation() {
        let mut m = Macro::new("Test Macro");
        assert_eq!(m.name, "Test Macro");
        assert!(m.actions.is_empty());

        let action = MacroAction {
            id: Uuid::new_v4().to_string(),
            action_type: MacroActionType::SshCommand,
            timestamp: Utc::now(),
            delay_ms: 0,
            data: MacroActionData::SshCommand {
                command: "ls -la".to_string(),
                working_dir: None,
                expected_prompt: None,
                timeout_secs: 30,
            },
            editable: true,
            enabled: true,
            description: None,
            visual_ref: None,
        };
        m.add_action(action);
        assert_eq!(m.actions.len(), 1);
    }

    #[test]
    fn test_macro_recorder() {
        let mut recorder = MacroRecorder::new();
        assert_eq!(recorder.get_state(), RecorderState::Idle);

        recorder.start_recording("Test Recording", None);
        assert_eq!(recorder.get_state(), RecorderState::Recording);

        recorder.record_ssh_command("echo hello", None);
        recorder.record_wait(1);
        recorder.record_ssh_command("echo world", None);

        assert_eq!(recorder.get_buffered_action_count(), 3);

        let m = recorder.stop_recording();
        assert!(m.is_some());
        assert_eq!(recorder.get_state(), RecorderState::Idle);
    }
}
