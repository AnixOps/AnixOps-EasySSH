//! 终端会话录制回放系统
//! 兼容 asciinema 格式，支持实时录制、回放控制、导出分享等功能

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::error::LiteError;

// ============================================================================
// Asciinema 格式定义
// ============================================================================

/// Asciinema v2 文件头
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsciinemaHeader {
    pub version: i32,
    pub width: u32,
    pub height: u32,
    pub timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_time_limit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<AsciinemaTheme>,
}

impl Default for AsciinemaHeader {
    fn default() -> Self {
        Self {
            version: 2,
            width: 80,
            height: 24,
            timestamp: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            ),
            duration: None,
            idle_time_limit: None,
            command: None,
            title: None,
            env: None,
            shell: None,
            term: Some("xterm-256color".to_string()),
            theme: None,
        }
    }
}

/// 主题配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsciinemaTheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub palette: Option<String>,
}

/// Asciinema 事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AsciinemaEventType {
    Output,    // 'o' - 终端输出
    Input,     // 'i' - 用户输入 (asciinema v2 扩展)
    Resize,    // 'r' - 终端尺寸变化
    Mark,      // 'm' - 标记/注释
}

impl AsciinemaEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AsciinemaEventType::Output => "o",
            AsciinemaEventType::Input => "i",
            AsciinemaEventType::Resize => "r",
            AsciinemaEventType::Mark => "m",
        }
    }
}

/// Asciinema 事件记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsciinemaEvent {
    pub time: f64,              // 时间戳（秒）
    #[serde(rename = "type")]
    pub event_type: AsciinemaEventType,
    pub data: String,           // 数据内容
}

/// 标记/注释
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMark {
    pub time: f64,
    pub label: String,
    pub color: Option<String>,
}

// ============================================================================
// 隐私保护 - 敏感信息过滤
// ============================================================================

/// 敏感信息过滤器
pub struct SensitiveDataFilter {
    patterns: Vec<RegexPattern>,
    enabled: bool,
}

struct RegexPattern {
    regex: regex::Regex,
    replacement: String,
    description: String,
}

impl SensitiveDataFilter {
    pub fn new() -> Self {
        let mut filter = Self {
            patterns: Vec::new(),
            enabled: true,
        };
        filter.load_default_patterns();
        filter
    }

    fn load_default_patterns(&mut self) {
        // 密码提示响应
        self.add_pattern(
            r"(?i)(password|passwd|pwd)\s*[:=]\s*\S+",
            "[PASSWORD REDACTED]",
            "Password prompts",
        );

        // API Keys
        self.add_pattern(
            r"(?i)(api[_-]?key|apikey)\s*[:=]\s*[a-zA-Z0-9_-]{16,}",
            "[API_KEY REDACTED]",
            "API Keys",
        );

        // 私钥内容
        self.add_pattern(
            r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----[\s\S]*?-----END",
            "[PRIVATE_KEY REDACTED]",
            "Private keys",
        );

        // Token
        self.add_pattern(
            r"(?i)(token|bearer)\s+([a-zA-Z0-9_-]{20,})",
            "[TOKEN REDACTED]",
            "Authentication tokens",
        );

        // AWS Keys
        self.add_pattern(
            r"AKIA[0-9A-Z]{16}",
            "[AWS_KEY REDACTED]",
            "AWS Access Keys",
        );

        // 信用卡号 (简单检测)
        self.add_pattern(
            r"\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13})\b",
            "[CARD_NUMBER REDACTED]",
            "Credit card numbers",
        );

        // SSH 密钥指纹后的密钥
        self.add_pattern(
            r"(?i)(ssh-rsa|ssh-ed25519|ecdsa-sha2-nistp256)\s+[A-Za-z0-9+/]{100,}={0,2}",
            "[SSH_KEY REDACTED]",
            "SSH public keys",
        );
    }

    fn add_pattern(&mut self, pattern: &str, replacement: &str, description: &str) {
        match regex::Regex::new(pattern) {
            Ok(regex) => {
                self.patterns.push(RegexPattern {
                    regex,
                    replacement: replacement.to_string(),
                    description: description.to_string(),
                });
            }
            Err(e) => {
                warn!("Failed to compile regex pattern '{}': {}", pattern, e);
            }
        }
    }

    /// 过滤敏感信息
    pub fn filter(&self, input: &str) -> String {
        if !self.enabled {
            return input.to_string();
        }

        let mut result = input.to_string();
        for pattern in &self.patterns {
            result = pattern.regex.replace_all(&result, &pattern.replacement).to_string();
        }
        result
    }

    /// 添加自定义过滤模式
    pub fn add_custom_pattern(&mut self, pattern: &str, replacement: &str) -> Result<(), LiteError> {
        match regex::Regex::new(pattern) {
            Ok(regex) => {
                self.patterns.push(RegexPattern {
                    regex,
                    replacement: replacement.to_string(),
                    description: "Custom pattern".to_string(),
                });
                Ok(())
            }
            Err(e) => Err(LiteError::SessionRecording(format!(
                "Invalid regex pattern: {}",
                e
            ))),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for SensitiveDataFilter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 录制管理器
// ============================================================================

/// 录制状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingState {
    Idle,
    Recording,
    Paused,
    Stopped,
}

/// 录制配置
#[derive(Debug, Clone)]
pub struct RecordingConfig {
    pub width: u32,
    pub height: u32,
    pub title: Option<String>,
    pub command: Option<String>,
    pub idle_time_limit: Option<f64>,
    pub record_input: bool,       // 是否录制输入
    pub enable_privacy_filter: bool,
    pub auto_mark_commands: bool, // 自动标记命令
    pub max_duration: Option<Duration>, // 最大录制时长
    pub output_dir: PathBuf,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            width: 80,
            height: 24,
            title: None,
            command: None,
            idle_time_limit: Some(1.0), // 压缩超过1秒的idle
            record_input: true,
            enable_privacy_filter: true,
            auto_mark_commands: true,
            max_duration: None,
            output_dir: Self::default_output_dir(),
        }
    }
}

impl RecordingConfig {
    fn default_output_dir() -> PathBuf {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("easyssh");
        path.push("recordings");
        path
    }
}

/// 活动录制会话
struct ActiveRecording {
    id: String,
    config: RecordingConfig,
    header: AsciinemaHeader,
    start_time: Instant,
    last_event_time: f64,
    events: Vec<AsciinemaEvent>,
    marks: Vec<SessionMark>,
    state: RecordingState,
    filter: SensitiveDataFilter,
    file_path: PathBuf,
    temp_file: Option<File>,
    input_buffer: String, // 用于命令检测
}

/// 录制元数据（用于数据库）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMetadata {
    pub id: String,
    pub title: Option<String>,
    pub created_at: u64,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub file_path: String,
    pub file_size: u64,
    pub command_count: usize,
    pub has_input: bool,
    pub tags: Vec<String>,
    pub server_id: Option<String>, // 关联的服务器
}

/// 会话录制管理器
pub struct SessionRecordingManager {
    storage_path: PathBuf,
    active_recordings: Arc<RwLock<HashMap<String, ActiveRecording>>>,
    recordings: Arc<RwLock<Vec<RecordingMetadata>>>,
    storage_manager: Arc<StorageManager>,
}

impl SessionRecordingManager {
    pub fn new(storage_path: impl AsRef<Path>) -> Result<Self, LiteError> {
        let storage_path = storage_path.as_ref().to_path_buf();

        // 确保目录存在
        fs::create_dir_all(&storage_path)?;
        fs::create_dir_all(storage_path.join("compressed"))?;
        fs::create_dir_all(storage_path.join("exports"))?;

        let manager = Self {
            storage_path,
            active_recordings: Arc::new(RwLock::new(HashMap::new())),
            recordings: Arc::new(RwLock::new(Vec::new())),
            storage_manager: Arc::new(StorageManager::new()?),
        };

        // 加载历史录制列表
        manager.load_recordings()?;

        Ok(manager)
    }

    /// 开始新录制
    pub async fn start_recording(
        &self,
        config: RecordingConfig,
        server_id: Option<String>,
    ) -> Result<String, LiteError> {
        let recording_id = uuid::Uuid::new_v4().to_string();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let file_name = format!("{}_{}.cast", timestamp, &recording_id[..8]);
        let file_path = config.output_dir.join(&file_name);

        // 确保输出目录存在
        fs::create_dir_all(&config.output_dir)?;

        // 创建临时文件
        let temp_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&file_path)?;

        let header = AsciinemaHeader {
            version: 2,
            width: config.width,
            height: config.height,
            timestamp: Some(timestamp),
            idle_time_limit: config.idle_time_limit,
            command: config.command.clone(),
            title: config.title.clone(),
            env: None,
            shell: None,
            term: Some("xterm-256color".to_string()),
            theme: None,
        };

        // 写入文件头
        let mut file = temp_file;
        let header_json = serde_json::to_string(&header)?;
        writeln!(file, "{}", header_json)?;

        let mut filter = SensitiveDataFilter::new();
        filter.set_enabled(config.enable_privacy_filter);

        let recording = ActiveRecording {
            id: recording_id.clone(),
            config,
            header,
            start_time: Instant::now(),
            last_event_time: 0.0,
            events: Vec::new(),
            marks: Vec::new(),
            state: RecordingState::Recording,
            filter,
            file_path: file_path.clone(),
            temp_file: Some(file),
            input_buffer: String::new(),
        };

        let mut recordings = self.active_recordings.write().await;
        recordings.insert(recording_id.clone(), recording);

        info!("Started recording session: {}", recording_id);
        Ok(recording_id)
    }

    /// 录制输出事件
    pub async fn record_output(&self, recording_id: &str, data: &str) -> Result<(), LiteError> {
        let mut recordings = self.active_recordings.write().await;
        let recording = recordings
            .get_mut(recording_id)
            .ok_or_else(|| LiteError::SessionRecording("Recording not found".to_string()))?;

        if recording.state != RecordingState::Recording {
            return Ok(());
        }

        let elapsed = recording.start_time.elapsed().as_secs_f64();
        let filtered_data = recording.filter.filter(data);

        let event = AsciinemaEvent {
            time: elapsed,
            event_type: AsciinemaEventType::Output,
            data: filtered_data,
        };

        // 写入文件
        if let Some(file) = &mut recording.temp_file {
            let line = format!(
                "[{:.6}, \"{}\", {}]",
                event.time,
                event.event_type.as_str(),
                serde_json::to_string(&event.data)?
            );
            writeln!(file, "{}", line)?;
        }

        recording.last_event_time = elapsed;
        recording.events.push(event);

        Ok(())
    }

    /// 录制输入事件
    pub async fn record_input(&self, recording_id: &str, data: &str) -> Result<(), LiteError> {
        let mut recordings = self.active_recordings.write().await;
        let recording = recordings
            .get_mut(recording_id)
            .ok_or_else(|| LiteError::SessionRecording("Recording not found".to_string()))?;

        if !recording.config.record_input || recording.state != RecordingState::Recording {
            return Ok(());
        }

        let elapsed = recording.start_time.elapsed().as_secs_f64();

        // 过滤敏感输入
        let filtered_data = recording.filter.filter(data);

        // 检测命令（以回车结尾）
        recording.input_buffer.push_str(data);
        if data.contains('\n') || data.contains('\r') {
            let cmd = recording.input_buffer.trim();
            if !cmd.is_empty() && recording.config.auto_mark_commands {
                // 添加命令标记
                let mark = SessionMark {
                    time: elapsed,
                    label: cmd.to_string(),
                    color: Some("#4CAF50".to_string()),
                };
                recording.marks.push(mark);
            }
            recording.input_buffer.clear();
        }

        let event = AsciinemaEvent {
            time: elapsed,
            event_type: AsciinemaEventType::Input,
            data: filtered_data,
        };

        if let Some(file) = &mut recording.temp_file {
            let line = format!(
                "[{:.6}, \"{}\", {}]",
                event.time,
                event.event_type.as_str(),
                serde_json::to_string(&event.data)?
            );
            writeln!(file, "{}", line)?;
        }

        recording.events.push(event);

        Ok(())
    }

    /// 录制终端尺寸变化
    pub async fn record_resize(
        &self,
        recording_id: &str,
        width: u32,
        height: u32,
    ) -> Result<(), LiteError> {
        let mut recordings = self.active_recordings.write().await;
        let recording = recordings
            .get_mut(recording_id)
            .ok_or_else(|| LiteError::SessionRecording("Recording not found".to_string()))?;

        if recording.state != RecordingState::Recording {
            return Ok(());
        }

        let elapsed = recording.start_time.elapsed().as_secs_f64();
        let data = format!("{}x{}", width, height);

        let event = AsciinemaEvent {
            time: elapsed,
            event_type: AsciinemaEventType::Resize,
            data,
        };

        if let Some(file) = &mut recording.temp_file {
            let line = format!(
                "[{:.6}, \"{}\", {}]",
                event.time,
                event.event_type.as_str(),
                serde_json::to_string(&event.data)?
            );
            writeln!(file, "{}", line)?;
        }

        recording.events.push(event);

        Ok(())
    }

    /// 添加标记/注释
    pub async fn add_mark(
        &self,
        recording_id: &str,
        label: &str,
        color: Option<&str>,
    ) -> Result<(), LiteError> {
        let mut recordings = self.active_recordings.write().await;
        let recording = recordings
            .get_mut(recording_id)
            .ok_or_else(|| LiteError::SessionRecording("Recording not found".to_string()))?;

        if recording.state != RecordingState::Recording {
            return Err(LiteError::SessionRecording(
                "Cannot add mark when not recording".to_string(),
            ));
        }

        let elapsed = recording.start_time.elapsed().as_secs_f64();
        let mark = SessionMark {
            time: elapsed,
            label: label.to_string(),
            color: color.map(|c| c.to_string()),
        };

        // 写入标记事件
        if let Some(file) = &mut recording.temp_file {
            let line = format!(
                "[{:.6}, \"m\", {}]",
                elapsed,
                serde_json::to_string(&serde_json::json!({
                    "label": label,
                    "color": color
                }))?
            );
            writeln!(file, "{}", line)?;
        }

        recording.marks.push(mark);
        info!("Added mark '{}' at {:.2}s in recording {}", label, elapsed, recording_id);

        Ok(())
    }

    /// 暂停录制
    pub async fn pause_recording(&self, recording_id: &str) -> Result<(), LiteError> {
        let mut recordings = self.active_recordings.write().await;
        let recording = recordings
            .get_mut(recording_id)
            .ok_or_else(|| LiteError::SessionRecording("Recording not found".to_string()))?;

        recording.state = RecordingState::Paused;
        info!("Paused recording: {}", recording_id);
        Ok(())
    }

    /// 恢复录制
    pub async fn resume_recording(&self, recording_id: &str) -> Result<(), LiteError> {
        let mut recordings = self.active_recordings.write().await;
        let recording = recordings
            .get_mut(recording_id)
            .ok_or_else(|| LiteError::SessionRecording("Recording not found".to_string()))?;

        recording.state = RecordingState::Recording;
        info!("Resumed recording: {}", recording_id);
        Ok(())
    }

    /// 停止录制
    pub async fn stop_recording(&self, recording_id: &str) -> Result<RecordingMetadata, LiteError> {
        let mut active_recordings = self.active_recordings.write().await;
        let mut recording = active_recordings
            .remove(recording_id)
            .ok_or_else(|| LiteError::SessionRecording("Recording not found".to_string()))?;

        recording.state = RecordingState::Stopped;

        // 关闭文件
        if let Some(mut file) = recording.temp_file.take() {
            file.flush()?;
        }

        let duration = recording.start_time.elapsed().as_secs_f64();
        let file_size = fs::metadata(&recording.file_path)?.len();

        // 压缩存储
        let compressed_path = self
            .storage_manager
            .compress_recording(&recording.file_path, recording_id)
            .await?;

        // 构建元数据
        let metadata = RecordingMetadata {
            id: recording_id.to_string(),
            title: recording.config.title.clone(),
            created_at: recording.header.timestamp.unwrap_or(0),
            duration,
            width: recording.config.width,
            height: recording.config.height,
            file_path: compressed_path.to_string_lossy().to_string(),
            file_size,
            command_count: recording.marks.len(),
            has_input: recording.config.record_input,
            tags: Vec::new(),
            server_id: None,
        };

        // 保存到列表
        let mut recordings = self.recordings.write().await;
        recordings.push(metadata.clone());
        self.save_recordings_list(&recordings)?;

        info!(
            "Stopped recording {}: duration={:.2}s, size={} bytes",
            recording_id, duration, file_size
        );

        Ok(metadata)
    }

    /// 获取录制状态
    pub async fn get_recording_state(&self, recording_id: &str) -> Option<RecordingState> {
        let recordings = self.active_recordings.read().await;
        recordings.get(recording_id).map(|r| r.state)
    }

    /// 获取所有录制列表
    pub async fn list_recordings(&self) -> Vec<RecordingMetadata> {
        self.recordings.read().await.clone()
    }

    /// 删除录制
    pub async fn delete_recording(&self, recording_id: &str) -> Result<(), LiteError> {
        // 删除文件
        let mut recordings = self.recordings.write().await;
        if let Some(pos) = recordings.iter().position(|r| r.id == recording_id) {
            let metadata = recordings.remove(pos);
            if Path::new(&metadata.file_path).exists() {
                fs::remove_file(&metadata.file_path)?;
            }
            self.save_recordings_list(&recordings)?;
            info!("Deleted recording: {}", recording_id);
        }
        Ok(())
    }

    /// 搜索录制内容
    pub async fn search_in_recording(
        &self,
        recording_id: &str,
        query: &str,
    ) -> Result<Vec<SearchResult>, LiteError> {
        let path = self.get_recording_path(recording_id).await?;
        let content = fs::read_to_string(&path)?;

        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        for (line_num, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(&query_lower) {
                // 尝试解析asciinema事件
                if line.starts_with('[') {
                    if let Ok(event) = Self::parse_event_line(line) {
                        results.push(SearchResult {
                            line_number: line_num,
                            timestamp: event.time,
                            preview: line.chars().take(100).collect(),
                            match_type: MatchType::Output,
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    /// 跨所有录制搜索
    pub async fn search_all_recordings(&self, query: &str) -> HashMap<String, Vec<SearchResult>> {
        let recordings = self.recordings.read().await;
        let mut results = HashMap::new();

        for metadata in recordings.iter() {
            if let Ok(recording_results) = self.search_in_recording(&metadata.id, query).await {
                if !recording_results.is_empty() {
                    results.insert(metadata.id.clone(), recording_results);
                }
            }
        }

        results
    }

    /// 获取回放器
    pub async fn get_player(&self, recording_id: &str) -> Result<SessionPlayer, LiteError> {
        let path = self.get_recording_path(recording_id).await?;
        SessionPlayer::load(path).await
    }

    /// 获取录制文件路径
    async fn get_recording_path(&self, recording_id: &str) -> Result<PathBuf, LiteError> {
        let recordings = self.recordings.read().await;
        let metadata = recordings
            .iter()
            .find(|r| r.id == recording_id)
            .ok_or_else(|| LiteError::SessionRecording("Recording not found".to_string()))?;
        Ok(PathBuf::from(&metadata.file_path))
    }

    /// 加载录制列表
    fn load_recordings(&self) -> Result<(), LiteError> {
        let list_path = self.storage_path.join("recordings.json");
        if list_path.exists() {
            let content = fs::read_to_string(&list_path)?;
            let recordings: Vec<RecordingMetadata> = serde_json::from_str(&content)?;

            // 由于我们在async上下文中，需要特殊处理
            // 这里先存储为字符串，稍后解析
            std::thread::spawn({
                let recordings = recordings.clone();
                let recordings_arc = self.recordings.clone();
                move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let mut list = recordings_arc.write().await;
                        *list = recordings;
                    });
                }
            });
        }
        Ok(())
    }

    /// 保存录制列表
    fn save_recordings_list(&self, recordings: &[RecordingMetadata]) -> Result<(), LiteError> {
        let list_path = self.storage_path.join("recordings.json");
        let content = serde_json::to_string_pretty(recordings)?;
        fs::write(list_path, content)?;
        Ok(())
    }

    /// 89e36790asciinema4e8b4ef6884c
    /// 
    /// 8fd9662f4e004e2a72ec7acb76848f8552a951fd6570Ff0c53ef4ee588abSessionPlayer548cSessionRecordingManager51714eab
    pub
    fn parse_event_line(line: &str) -> Result<AsciinemaEvent, LiteError> {
        let trimmed = line.trim();
        if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
            return Err(LiteError::SessionRecording("Invalid event format".to_string()));
        }

        let inner = &trimmed[1..trimmed.len()-1];
        let parts: Vec<&str> = inner.splitn(3, ',').collect();

        if parts.len() < 3 {
            return Err(LiteError::SessionRecording("Invalid event format".to_string()));
        }

        let time: f64 = parts[0]
            .trim()
            .parse()
            .map_err(|_| LiteError::SessionRecording("Invalid timestamp".to_string()))?;

        let event_type_str = parts[1].trim().trim_matches('"');
        let event_type = match event_type_str {
            "o" => AsciinemaEventType::Output,
            "i" => AsciinemaEventType::Input,
            "r" => AsciinemaEventType::Resize,
            "m" => AsciinemaEventType::Mark,
            _ => return Err(LiteError::SessionRecording("Unknown event type".to_string())),
        };

        let data = parts[2].trim().trim_matches('"').to_string();

        Ok(AsciinemaEvent {
            time,
            event_type,
            data,
        })
    }
}

/// 搜索结果
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub line_number: usize,
    pub timestamp: f64,
    pub preview: String,
    pub match_type: MatchType,
}

#[derive(Debug, Clone)]
pub enum MatchType {
    Output,
    Input,
    Command,
}

// ============================================================================
// 回放播放器
// ============================================================================

/// 播放状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Idle,
    Playing,
    Paused,
    Finished,
}

/// 播放速度
#[derive(Debug, Clone, Copy)]
pub enum PlaybackSpeed {
    Slow,      // 0.5x
    Normal,    // 1.0x
    Fast,      // 1.5x
    Faster,    // 2.0x
    Fastest,   // 4.0x
}

impl PlaybackSpeed {
    pub fn as_f64(&self) -> f64 {
        match self {
            PlaybackSpeed::Slow => 0.5,
            PlaybackSpeed::Normal => 1.0,
            PlaybackSpeed::Fast => 1.5,
            PlaybackSpeed::Faster => 2.0,
            PlaybackSpeed::Fastest => 4.0,
        }
    }
}

/// 播放器事件
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Output(String),
    Input(String),
    Resize { width: u32, height: u32 },
    Mark(SessionMark),
    Finished,
}

/// 会话播放器
pub struct SessionPlayer {
    header: AsciinemaHeader,
    events: Vec<AsciinemaEvent>,
    marks: Vec<SessionMark>,
    duration: f64,
    current_time: Arc<RwLock<f64>>,
    state: Arc<RwLock<PlaybackState>>,
    speed: Arc<RwLock<PlaybackSpeed>>,
    event_sender: mpsc::UnboundedSender<PlayerEvent>,
    event_receiver: Arc<Mutex<mpsc::UnboundedReceiver<PlayerEvent>>>,
    control_sender: mpsc::UnboundedSender<PlayerControl>,
}

#[derive(Debug, Clone)]
enum PlayerControl {
    Play,
    Pause,
    Stop,
    Seek(f64),
    SetSpeed(PlaybackSpeed),
}

impl SessionPlayer {
    /// 加载录制文件
    pub async fn load(path: impl AsRef<Path>) -> Result<Self, LiteError> {
        let content = fs::read_to_string(path)?;
        let mut lines = content.lines();

        // 解析文件头
        let header_line = lines
            .next()
            .ok_or_else(|| LiteError::SessionRecording("Empty recording file".to_string()))?;
        let header: AsciinemaHeader = serde_json::from_str(header_line)?;

        // 解析事件
        let mut events = Vec::new();
        let mut marks = Vec::new();
        let mut duration = 0.0;

        for line in lines {
            if line.trim().is_empty() {
                continue;
            }

            // 尝试解析为asciinema数组格式 [time, "type", "data"]
            if line.starts_with('[') {
                if let Ok(event) = Self::parse_event_line(line) {
                    duration = event.time.max(duration);

                    // 提取标记
                    if event.event_type == AsciinemaEventType::Mark {
                        if let Ok(mark_data) = serde_json::from_str::<serde_json::Value>(&event.data) {
                            let mark = SessionMark {
                                time: event.time,
                                label: mark_data["label"].as_str().unwrap_or("").to_string(),
                                color: mark_data["color"].as_str().map(|s| s.to_string()),
                            };
                            marks.push(mark);
                        }
                    }

                    events.push(event);
                }
            }
        }

        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let (control_sender, mut control_receiver) = mpsc::unbounded_channel::<PlayerControl>();

        let player = Self {
            header,
            events,
            marks,
            duration,
            current_time: Arc::new(RwLock::new(0.0)),
            state: Arc::new(RwLock::new(PlaybackState::Idle)),
            speed: Arc::new(RwLock::new(PlaybackSpeed::Normal)),
            event_sender,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            control_sender,
        };

        // 启动播放控制线程
        player.start_playback_loop(control_receiver).await;

        Ok(player)
    }

    /// 开始播放循环
    async fn start_playback_loop(&self, mut control_receiver: mpsc::UnboundedReceiver<PlayerControl>) {
        let events = self.events.clone();
        let current_time = self.current_time.clone();
        let state = self.state.clone();
        let speed = self.speed.clone();
        let event_sender = self.event_sender.clone();
        let duration = self.duration;

        tokio::spawn(async move {
            let mut current_index = 0;
            let mut last_real_time = Instant::now();

            loop {
                // 处理控制命令
                while let Ok(control) = control_receiver.try_recv() {
                    match control {
                        PlayerControl::Play => {
                            let mut s = state.write().await;
                            *s = PlaybackState::Playing;
                            last_real_time = Instant::now();
                        }
                        PlayerControl::Pause => {
                            let mut s = state.write().await;
                            *s = PlaybackState::Paused;
                        }
                        PlayerControl::Stop => {
                            let mut s = state.write().await;
                            *s = PlaybackState::Finished;
                            let _ = event_sender.send(PlayerEvent::Finished);
                            return;
                        }
                        PlayerControl::Seek(target_time) => {
                            let mut t = current_time.write().await;
                            *t = target_time.clamp(0.0, duration);
                            // 找到对应的事件索引
                            current_index = events.iter()
                                .position(|e| e.time >= target_time)
                                .unwrap_or(events.len());
                        }
                        PlayerControl::SetSpeed(new_speed) => {
                            let mut s = speed.write().await;
                            *s = new_speed;
                        }
                    }
                }

                // 检查播放状态
                let current_state = *state.read().await;
                if current_state != PlaybackState::Playing {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }

                // 获取当前时间和速度
                let time = *current_time.read().await;
                let speed_val = speed.read().await.as_f64();

                // 检查是否播放完成
                if current_index >= events.len() {
                    let mut s = state.write().await;
                    *s = PlaybackState::Finished;
                    let _ = event_sender.send(PlayerEvent::Finished);
                    break;
                }

                // 计算经过的时间
                let now = Instant::now();
                let elapsed_real = now.duration_since(last_real_time).as_secs_f64();
                let elapsed_playback = elapsed_real * speed_val;

                // 发送当前时间点之前的所有事件
                while current_index < events.len() {
                    let event = &events[current_index];
                    if event.time <= time + elapsed_playback {
                        // 发送事件
                        let player_event = match event.event_type {
                            AsciinemaEventType::Output => {
                                Some(PlayerEvent::Output(event.data.clone()))
                            }
                            AsciinemaEventType::Input => {
                                Some(PlayerEvent::Input(event.data.clone()))
                            }
                            AsciinemaEventType::Resize => {
                                let parts: Vec<&str> = event.data.split('x').collect();
                                if parts.len() == 2 {
                                    if let (Ok(w), Ok(h)) = (parts[0].parse(), parts[1].parse()) {
                                        Some(PlayerEvent::Resize { width: w, height: h })
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        };

                        if let Some(pe) = player_event {
                            let _ = event_sender.send(pe);
                        }

                        current_index += 1;
                    } else {
                        break;
                    }
                }

                // 更新时间
                {
                    let mut t = current_time.write().await;
                    *t = (time + elapsed_playback).min(duration);
                }
                last_real_time = now;

                // 小延迟避免CPU占用过高
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
    }

    fn parse_event_line(line: &str) -> Result<AsciinemaEvent, LiteError> {
        let trimmed = line.trim();
        if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
            return Err(LiteError::SessionRecording("Invalid event format".to_string()));
        }

        let inner = &trimmed[1..trimmed.len()-1];
        let parts: Vec<&str> = inner.splitn(3, ',').collect();

        if parts.len() < 3 {
            return Err(LiteError::SessionRecording("Invalid event format".to_string()));
        }

        let time: f64 = parts[0]
            .trim()
            .parse()
            .map_err(|_| LiteError::SessionRecording("Invalid timestamp".to_string()))?;

        let event_type_str = parts[1].trim().trim_matches('"');
        let event_type = match event_type_str {
            "o" => AsciinemaEventType::Output,
            "i" => AsciinemaEventType::Input,
            "r" => AsciinemaEventType::Resize,
            "m" => AsciinemaEventType::Mark,
            _ => return Err(LiteError::SessionRecording("Unknown event type".to_string())),
        };

        let data = serde_json::from_str(parts[2].trim())
            .unwrap_or_else(|_| parts[2].trim().trim_matches('"').to_string());

        Ok(AsciinemaEvent {
            time,
            event_type,
            data,
        })
    }

    // 播放器控制方法

    pub async fn play(&self) {
        let _ = self.control_sender.send(PlayerControl::Play);
    }

    pub async fn pause(&self) {
        let _ = self.control_sender.send(PlayerControl::Pause);
    }

    pub async fn stop(&self) {
        let _ = self.control_sender.send(PlayerControl::Stop);
    }

    pub async fn seek(&self, time: f64) {
        let _ = self.control_sender.send(PlayerControl::Seek(time));
    }

    pub async fn set_speed(&self, speed: PlaybackSpeed) {
        let _ = self.control_sender.send(PlayerControl::SetSpeed(speed));
    }

    pub async fn get_state(&self) -> PlaybackState {
        *self.state.read().await
    }

    pub async fn get_current_time(&self) -> f64 {
        *self.current_time.read().await
    }

    pub async fn get_duration(&self) -> f64 {
        self.duration
    }

    pub fn get_header(&self) -> &AsciinemaHeader {
        &self.header
    }

    pub fn get_marks(&self) -> &[SessionMark] {
        &self.marks
    }

    pub async fn next_event(&self) -> Option<PlayerEvent> {
        let mut receiver = self.event_receiver.lock().await;
        receiver.recv().await
    }
}

// ============================================================================
// 存储管理
// ============================================================================

/// 存储管理器 - 处理压缩、过期清理
pub struct StorageManager {
    compression_level: u32,
}

impl StorageManager {
    pub fn new() -> Result<Self, LiteError> {
        Ok(Self {
            compression_level: 6, // 默认压缩级别
        })
    }

    /// 压缩录制文件
    pub async fn compress_recording(
        &self,
        source_path: &Path,
        recording_id: &str,
    ) -> Result<PathBuf, LiteError> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let compressed_path = source_path.with_extension("cast.gz");

        let input = fs::read_to_string(source_path)?;
        let output = File::create(&compressed_path)?;

        let mut encoder = GzEncoder::new(output, Compression::new(self.compression_level));
        encoder.write_all(input.as_bytes())?;
        encoder.finish()?;

        // 删除原始文件
        fs::remove_file(source_path)?;

        info!("Compressed recording {} to {:?}", recording_id, compressed_path);
        Ok(compressed_path)
    }

    /// 解压录制文件
    pub fn decompress_recording(&self, compressed_path: &Path) -> Result<String, LiteError> {
        use flate2::read::GzDecoder;

        let input = File::open(compressed_path)?;
        let mut decoder = GzDecoder::new(input);
        let mut output = String::new();
        decoder.read_to_string(&mut output)?;

        Ok(output)
    }

    /// 清理过期录制
    pub async fn cleanup_expired(
        &self,
        recordings: &mut Vec<RecordingMetadata>,
        max_age_days: u32,
        max_total_size: u64, // bytes
    ) -> Result<u32, LiteError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let max_age_secs = (max_age_days as u64) * 24 * 60 * 60;
        let mut deleted_count = 0;
        let mut total_size: u64 = recordings.iter().map(|r| r.file_size).sum();

        // 按时间排序（旧的在前）
        let mut to_remove = Vec::new();

        for (i, metadata) in recordings.iter().enumerate() {
            let age = now - metadata.created_at;

            // 删除过期的
            if age > max_age_secs {
                to_remove.push(i);
                total_size -= metadata.file_size;
                deleted_count += 1;
                continue;
            }

            // 如果总大小超过限制，删除旧的直到符合
            if total_size > max_total_size {
                to_remove.push(i);
                total_size -= metadata.file_size;
                deleted_count += 1;
            }
        }

        // 删除文件
        for &index in to_remove.iter().rev() {
            let metadata = &recordings[index];
            if Path::new(&metadata.file_path).exists() {
                fs::remove_file(&metadata.file_path)?;
                info!("Deleted expired recording: {}", metadata.id);
            }
        }

        // 从列表中移除
        for &index in to_remove.iter().rev() {
            recordings.remove(index);
        }

        Ok(deleted_count)
    }

    /// 计算目录总大小
    pub fn calculate_storage_size(&self, path: &Path) -> Result<u64, LiteError> {
        let mut total = 0u64;

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                total += metadata.len();
            }
        }

        Ok(total)
    }
}

// ============================================================================
// 导出功能
// ============================================================================

/// 导出格式
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Asciicast,  // 原始格式
    Json,       // 完整JSON
    Text,       // 纯文本
    Gif,        // GIF动画（需要外部工具）
    Mp4,        // MP4视频（需要外部工具）
}

/// 导出选项
#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub quality: Option<u8>, // 1-100
}

/// 导出管理器
pub struct ExportManager {
    asciinema_install_path: Option<PathBuf>,
    ffmpeg_install_path: Option<PathBuf>,
}

impl ExportManager {
    pub fn new() -> Self {
        Self {
            asciinema_install_path: Self::find_asciinema(),
            ffmpeg_install_path: Self::find_ffmpeg(),
        }
    }

    fn find_asciinema() -> Option<PathBuf> {
        // 尝试查找 asciinema 命令
        let cmd = if cfg!(windows) { "where" } else { "which" };
        std::process::Command::new(cmd)
            .arg("asciinema")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| PathBuf::from(s.trim()))
    }

    fn find_ffmpeg() -> Option<PathBuf> {
        let cmd = if cfg!(windows) { "where" } else { "which" };
        std::process::Command::new(cmd)
            .arg("ffmpeg")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| PathBuf::from(s.trim()))
    }

    /// 导出录制
    pub async fn export(
        &self,
        source_path: &Path,
        output_path: &Path,
        options: ExportOptions,
    ) -> Result<(), LiteError> {
        match options.format {
            ExportFormat::Asciicast => {
                self.export_asciicast(source_path, output_path, &options).await
            }
            ExportFormat::Json => {
                self.export_json(source_path, output_path, &options).await
            }
            ExportFormat::Text => {
                self.export_text(source_path, output_path, &options).await
            }
            ExportFormat::Gif => {
                self.export_gif(source_path, output_path, &options).await
            }
            ExportFormat::Mp4 => {
                self.export_mp4(source_path, output_path, &options).await
            }
        }
    }

    async fn export_asciicast(
        &self,
        source_path: &Path,
        output_path: &Path,
        options: &ExportOptions,
    ) -> Result<(), LiteError> {
        let content = fs::read_to_string(source_path)?;
        let mut lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return Err(LiteError::SessionRecording("Empty recording".to_string()));
        }

        // 解析并修改头
        let mut header: AsciinemaHeader = serde_json::from_str(lines[0])?;

        if let Some(w) = options.width {
            header.width = w;
        }
        if let Some(h) = options.height {
            header.height = h;
        }

        let mut output = File::create(output_path)?;
        writeln!(output, "{}", serde_json::to_string(&header)?)?;

        // 过滤事件
        for line in &lines[1..] {
            if let Ok(event) = SessionPlayer::parse_event_line(line) {
                if Self::event_in_range(&event, options.start_time, options.end_time) {
                    let adjusted_time = event.time - options.start_time.unwrap_or(0.0);
                    writeln!(
                        output,
                        "[{:.6}, \"{}\", {}]",
                        adjusted_time,
                        event.event_type.as_str(),
                        serde_json::to_string(&event.data)?
                    )?;
                }
            }
        }

        Ok(())
    }

    async fn export_json(
        &self,
        source_path: &Path,
        output_path: &Path,
        options: &ExportOptions,
    ) -> Result<(), LiteError> {
        let content = fs::read_to_string(source_path)?;
        let mut lines: Vec<&str> = content.lines().collect();

        let header: AsciinemaHeader = serde_json::from_str(lines[0])?;

        let mut events = Vec::new();
        for line in &lines[1..] {
            if let Ok(event) = SessionPlayer::parse_event_line(line) {
                if Self::event_in_range(&event, options.start_time, options.end_time) {
                    events.push(event);
                }
            }
        }

        let output_data = serde_json::json!({
            "header": header,
            "events": events,
        });

        fs::write(output_path, serde_json::to_string_pretty(&output_data)?)?;
        Ok(())
    }

    async fn export_text(
        &self,
        source_path: &Path,
        output_path: &Path,
        options: &ExportOptions,
    ) -> Result<(), LiteError> {
        let content = fs::read_to_string(source_path)?;
        let lines: Vec<&str> = content.lines().collect();

        let mut text_output = String::new();

        for line in &lines[1..] {
            if let Ok(event) = SessionPlayer::parse_event_line(line) {
                if event.event_type == AsciinemaEventType::Output {
                    if Self::event_in_range(&event, options.start_time, options.end_time) {
                        text_output.push_str(&event.data);
                    }
                }
            }
        }

        fs::write(output_path, text_output)?;
        Ok(())
    }

    async fn export_gif(
        &self,
        source_path: &Path,
        output_path: &Path,
        options: &ExportOptions,
    ) -> Result<(), LiteError> {
        // 检查是否有 asciinema-agg 或类似工具
        if self.asciinema_install_path.is_none() {
            return Err(LiteError::SessionRecording(
                "asciinema not found. Install it to export GIF.".to_string(),
            ));
        }

        // 使用 asciinema 的 gif 导出功能（如果可用）
        // 或者使用第三方工具如 agg
        let temp_cast = source_path.with_extension("temp.cast");

        // 先导出裁剪后的 cast 文件
        self.export_asciicast(source_path, &temp_cast, options).await?;

        // 尝试使用 agg (asciinema gif generator)
        let result = std::process::Command::new("agg")
            .arg(&temp_cast)
            .arg(output_path)
            .output();

        // 清理临时文件
        let _ = fs::remove_file(&temp_cast);

        match result {
            Ok(output) if output.status.success() => Ok(()),
            _ => {
                // 如果 agg 失败，尝试其他方法
                Err(LiteError::SessionRecording(
                    "GIF export failed. Install 'agg' (asciinema gif generator)".to_string(),
                ))
            }
        }
    }

    async fn export_mp4(
        &self,
        source_path: &Path,
        output_path: &Path,
        _options: &ExportOptions,
    ) -> Result<(), LiteError> {
        if self.ffmpeg_install_path.is_none() {
            return Err(LiteError::SessionRecording(
                "ffmpeg not found. Install it to export MP4.".to_string(),
            ));
        }

        // 首先导出为 GIF，然后转换为 MP4
        // 或者使用更高级的方法直接渲染终端为视频
        Err(LiteError::SessionRecording(
            "MP4 export requires terminal renderer. Use GIF export instead.".to_string(),
        ))
    }

    fn event_in_range(event: &AsciinemaEvent, start: Option<f64>, end: Option<f64>) -> bool {
        if let Some(s) = start {
            if event.time < s {
                return false;
            }
        }
        if let Some(e) = end {
            if event.time > e {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// 云端分享
// ============================================================================

/// 云端分享配置
#[derive(Debug, Clone)]
pub struct CloudShareConfig {
    pub api_url: String,
    pub api_token: Option<String>,
}

impl Default for CloudShareConfig {
    fn default() -> Self {
        Self {
            api_url: "https://asciinema.org".to_string(),
            api_token: None,
        }
    }
}

/// 云端分享管理器
#[cfg(feature = "sync")]
pub struct CloudShareManager {
    config: CloudShareConfig,
    http_client: reqwest::Client,
}

#[cfg(feature = "sync")]
impl CloudShareManager {
    pub fn new(config: CloudShareConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// 上传到 asciinema.org
    pub async fn upload_to_asciinema(
        &self,
        recording_path: &Path,
        title: Option<&str>,
    ) -> Result<String, LiteError> {
        let upload_url = format!("{}/api/asciicasts", self.config.api_url);

        let file_content = fs::read(recording_path)?;

        let part = reqwest::multipart::Part::bytes(file_content)
            .file_name("recording.cast")
            .mime_str("application/octet-stream")?;

        let form = reqwest::multipart::Form::new()
            .part("asciicast", part);

        let form = if let Some(t) = title {
            form.text("title", t.to_string())
        } else {
            form
        };

        let mut request = self.http_client.post(&upload_url).multipart(form);

        if let Some(token) = &self.config.api_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            let url = result["url"]
                .as_str()
                .ok_or_else(|| LiteError::SessionRecording("Invalid response from server".to_string()))?;
            Ok(url.to_string())
        } else {
            Err(LiteError::SessionRecording(format!(
                "Upload failed: {}",
                response.status()
            )))
        }
    }

    /// 生成分享链接（本地托管）
    pub fn generate_share_link(&self, recording_id: &str) -> String {
        format!("easyssh://recording/{}", recording_id)
    }
}

// ============================================================================
// FFI 接口
// ============================================================================

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int};

/// C兼容的录制句柄
pub struct RecordingHandle {
    manager: Arc<SessionRecordingManager>,
    recording_id: String,
}

/// 创建录制管理器
#[no_mangle]
pub extern "C" fn session_recording_manager_new(storage_path: *const c_char) -> *mut SessionRecordingManager {
    let path = unsafe {
        if storage_path.is_null() {
            return std::ptr::null_mut();
        }
        CStr::from_ptr(storage_path).to_string_lossy().to_string()
    };

    match SessionRecordingManager::new(&path) {
        Ok(manager) => Box::into_raw(Box::new(manager)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 释放录制管理器
#[no_mangle]
pub extern "C" fn session_recording_manager_free(manager: *mut SessionRecordingManager) {
    if !manager.is_null() {
        unsafe {
            let _ = Box::from_raw(manager);
        }
    }
}

/// 开始录制（同步版本，用于FFI）
#[no_mangle]
pub extern "C" fn session_recording_start(
    manager: *mut SessionRecordingManager,
    width: c_int,
    height: c_int,
    record_input: c_int,
    title: *const c_char,
) -> *mut c_char {
    if manager.is_null() {
        return std::ptr::null_mut();
    }

    let title_str = unsafe {
        if title.is_null() {
            None
        } else {
            Some(CStr::from_ptr(title).to_string_lossy().to_string())
        }
    };

    let config = RecordingConfig {
        width: width as u32,
        height: height as u32,
        title: title_str,
        record_input: record_input != 0,
        ..Default::default()
    };

    let manager_ref = unsafe { &*manager };

    // 创建运行时来执行异步操作
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    match rt.block_on(manager_ref.start_recording(config, None)) {
        Ok(id) => {
            let c_id = CString::new(id).unwrap_or_default();
            c_id.into_raw()
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// 停止录制
#[no_mangle]
pub extern "C" fn session_recording_stop(
    manager: *mut SessionRecordingManager,
    recording_id: *const c_char,
) -> c_double {
    if manager.is_null() || recording_id.is_null() {
        return -1.0;
    }

    let id = unsafe { CStr::from_ptr(recording_id).to_string_lossy().to_string() };
    let manager_ref = unsafe { &*manager };

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1.0,
    };

    match rt.block_on(manager_ref.stop_recording(&id)) {
        Ok(metadata) => metadata.duration,
        Err(_) => -1.0,
    }
}

/// 释放字符串
#[no_mangle]
pub extern "C" fn session_recording_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sensitive_filter_passwords() {
        let filter = SensitiveDataFilter::new();

        let input = "password: secret123\nPassword=hidden456";
        let filtered = filter.filter(input);

        assert!(!filtered.contains("secret123"));
        assert!(!filtered.contains("hidden456"));
        assert!(filtered.contains("[PASSWORD REDACTED]"));
    }

    #[test]
    fn test_sensitive_filter_api_keys() {
        let filter = SensitiveDataFilter::new();

        let input = "api_key: sk-1234567890abcdef1234567890";
        let filtered = filter.filter(input);

        assert!(!filtered.contains("sk-1234567890abcdef1234567890"));
    }

    #[test]
    fn test_asciinema_header_serialization() {
        let header = AsciinemaHeader::default();
        let json = serde_json::to_string(&header).unwrap();

        assert!(json.contains("\"version\":2"));
        assert!(json.contains("\"width\":80"));
        assert!(json.contains("\"height\":24"));
    }

    #[test]
    fn test_playback_speed() {
        assert_eq!(PlaybackSpeed::Normal.as_f64(), 1.0);
        assert_eq!(PlaybackSpeed::Fast.as_f64(), 1.5);
        assert_eq!(PlaybackSpeed::Faster.as_f64(), 2.0);
    }

    #[tokio::test]
    async fn test_recording_manager_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionRecordingManager::new(temp_dir.path()).unwrap();

        let config = RecordingConfig::default();
        let recording_id = manager.start_recording(config, None).await.unwrap();

        assert!(!recording_id.is_empty());

        // 录制一些数据
        manager.record_output(&recording_id, "Hello World\n").await.unwrap();
        manager.record_input(&recording_id, "ls\n").await.unwrap();

        // 停止录制
        let metadata = manager.stop_recording(&recording_id).await.unwrap();
        assert!(metadata.duration > 0.0);
    }

    #[test]
    fn test_storage_compression() {
        use flate2::read::GzDecoder;

        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::new().unwrap();

        let test_file = temp_dir.path().join("test.cast");
        fs::write(&test_file, "{\"version\": 2}\n[0.0, \"o\", \"hello\"]\n").unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let compressed_path = rt.block_on(async {
            storage.compress_recording(&test_file, "test").await.unwrap()
        });

        assert!(compressed_path.exists());
        assert!(!test_file.exists()); // 原始文件已被删除

        // 验证可以解压
        let decompressed = storage.decompress_recording(&compressed_path).unwrap();
        assert!(decompressed.contains("hello"));
    }
}
