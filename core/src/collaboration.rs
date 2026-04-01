//! 团队实时协作系统 (Pro版本)
//! 提供实时终端共享、协作光标、语音通话、屏幕标注等功能

use crate::error::LiteError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============ 协作会话类型 ============

/// 协作角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollaborationRole {
    Observer,   // 观察者 - 只能观看
    Operator,   // 操作者 - 可以输入命令
    Admin,      // 管理员 - 完整控制
}

impl CollaborationRole {
    pub fn can_input(&self) -> bool {
        matches!(self, CollaborationRole::Operator | CollaborationRole::Admin)
    }

    pub fn can_manage(&self) -> bool {
        matches!(self, CollaborationRole::Admin)
    }

    pub fn can_annotate(&self) -> bool {
        matches!(self, CollaborationRole::Operator | CollaborationRole::Admin)
    }

    pub fn can_voice(&self) -> bool {
        true // 所有角色都可以语音
    }
}

/// 协作会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollaborationState {
    Active,     // 活跃中
    Paused,     // 已暂停
    Ended,      // 已结束
    Recording,  // 录制中
}

/// 协作会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSession {
    pub id: String,
    pub host_id: String,
    pub host_username: String,
    pub team_id: String,
    pub server_id: String,
    pub server_name: String,
    pub state: CollaborationState,
    pub share_link: String,
    pub created_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub settings: CollaborationSettings,
}

/// 协作设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSettings {
    pub allow_observers: bool,
    pub require_approval: bool,
    pub record_session: bool,
    pub enable_voice: bool,
    pub enable_annotations: bool,
    pub max_participants: i32,
    pub allow_clipboard_sync: bool,
}

impl Default for CollaborationSettings {
    fn default() -> Self {
        Self {
            allow_observers: true,
            require_approval: false,
            record_session: false,
            enable_voice: true,
            enable_annotations: true,
            max_participants: 10,
            allow_clipboard_sync: true,
        }
    }
}

/// 协作参与者
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationParticipant {
    pub id: String,
    pub session_id: String,
    pub user_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub role: CollaborationRole,
    pub joined_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub is_voice_active: bool,
    pub cursor_position: Option<CursorPosition>,
    pub is_online: bool,
}

/// 光标位置
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CursorPosition {
    pub row: u32,
    pub col: u32,
    pub timestamp: DateTime<Utc>,
}

/// 终端内容更新
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalUpdate {
    pub session_id: String,
    pub participant_id: String,
    pub update_type: TerminalUpdateType,
    pub data: String,
    pub timestamp: DateTime<Utc>,
}

/// 终端更新类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalUpdateType {
    Output,     // 终端输出
    Input,      // 用户输入
    Resize,     // 终端大小改变
    Scroll,     // 滚动位置
    Selection,  // 文本选择
}

/// 屏幕标注
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    pub session_id: String,
    pub author_id: String,
    pub author_name: String,
    pub annotation_type: AnnotationType,
    pub position: AnnotationPosition,
    pub content: String,
    pub color: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// 标注类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationType {
    Draw,       // 手绘涂鸦
    Highlight,  // 高亮
    Arrow,      // 箭头
    Text,       // 文本
    Circle,     // 圆圈
    Rectangle,  // 矩形
}

/// 标注位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationPosition {
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub points: Option<Vec<(f64, f64)>>, // 用于手绘路径
}

/// 评论
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub session_id: String,
    pub author_id: String,
    pub author_name: String,
    pub line_number: u32,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub replies: Vec<CommentReply>,
    pub resolved: bool,
}

/// 评论回复
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentReply {
    pub id: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// 协作历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationHistory {
    pub id: String,
    pub session_id: String,
    pub participant_id: String,
    pub participant_name: String,
    pub action_type: CollaborationActionType,
    pub command: Option<String>,
    pub output_preview: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// 协作动作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollaborationActionType {
    Join,           // 加入会话
    Leave,          // 离开会话
    ExecuteCommand, // 执行命令
    Input,          // 输入
    RoleChange,     // 角色变更
    VoiceStart,     // 开始语音
    VoiceEnd,       // 结束语音
    Annotate,       // 添加标注
    Comment,        // 添加评论
    ClipboardSync,  // 剪贴板同步
}

/// 共享剪贴板项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedClipboardItem {
    pub id: String,
    pub session_id: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub content_type: ClipboardContentType,
    pub created_at: DateTime<Utc>,
}

/// 剪贴板内容类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardContentType {
    Text,
    Code,
    Url,
    Command,
}

/// WebRTC信令消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRTCSignal {
    pub session_id: String,
    pub from_user_id: String,
    pub to_user_id: String,
    pub signal_type: WebRTCSignalType,
    pub data: serde_json::Value,
}

/// WebRTC信令类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebRTCSignalType {
    Offer,
    Answer,
    IceCandidate,
    Join,
    Leave,
}

/// 录制片段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSegment {
    pub id: String,
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub file_path: String,
    pub file_size: i64,
    pub events_count: i32,
}

/// 协作会话录制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationRecording {
    pub id: String,
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub host_id: String,
    pub segments: Vec<RecordingSegment>,
    pub total_events: i32,
    pub file_size: i64,
}

// ============ 创建函数 ============

pub fn create_collaboration_session(
    host_id: &str,
    host_username: &str,
    team_id: &str,
    server_id: &str,
    server_name: &str,
) -> CollaborationSession {
    let id = Uuid::new_v4().to_string();
    let share_link = format!("collab-{}", Uuid::new_v4().to_string().split('-').next().unwrap());

    CollaborationSession {
        id,
        host_id: host_id.to_string(),
        host_username: host_username.to_string(),
        team_id: team_id.to_string(),
        server_id: server_id.to_string(),
        server_name: server_name.to_string(),
        state: CollaborationState::Active,
        share_link,
        created_at: Utc::now(),
        ended_at: None,
        settings: CollaborationSettings::default(),
    }
}

pub fn create_participant(
    session_id: &str,
    user_id: &str,
    username: &str,
    role: CollaborationRole,
) -> CollaborationParticipant {
    let now = Utc::now();
    CollaborationParticipant {
        id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        user_id: user_id.to_string(),
        username: username.to_string(),
        avatar_url: None,
        role,
        joined_at: now,
        last_active_at: now,
        is_voice_active: false,
        cursor_position: None,
        is_online: true,
    }
}

pub fn create_annotation(
    session_id: &str,
    author_id: &str,
    author_name: &str,
    annotation_type: AnnotationType,
    position: AnnotationPosition,
    content: &str,
    color: &str,
) -> Annotation {
    Annotation {
        id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        author_id: author_id.to_string(),
        author_name: author_name.to_string(),
        annotation_type,
        position,
        content: content.to_string(),
        color: color.to_string(),
        created_at: Utc::now(),
        resolved_at: None,
    }
}

pub fn create_comment(
    session_id: &str,
    author_id: &str,
    author_name: &str,
    line_number: u32,
    content: &str,
) -> Comment {
    Comment {
        id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        author_id: author_id.to_string(),
        author_name: author_name.to_string(),
        line_number,
        content: content.to_string(),
        created_at: Utc::now(),
        updated_at: None,
        replies: Vec::new(),
        resolved: false,
    }
}

pub fn create_history_entry(
    session_id: &str,
    participant_id: &str,
    participant_name: &str,
    action_type: CollaborationActionType,
    command: Option<&str>,
    output_preview: Option<&str>,
) -> CollaborationHistory {
    CollaborationHistory {
        id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        participant_id: participant_id.to_string(),
        participant_name: participant_name.to_string(),
        action_type,
        command: command.map(|s| s.to_string()),
        output_preview: output_preview.map(|s| s.to_string()),
        timestamp: Utc::now(),
    }
}

pub fn create_clipboard_item(
    session_id: &str,
    author_id: &str,
    author_name: &str,
    content: &str,
    content_type: ClipboardContentType,
) -> SharedClipboardItem {
    SharedClipboardItem {
        id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        author_id: author_id.to_string(),
        author_name: author_name.to_string(),
        content: content.to_string(),
        content_type,
        created_at: Utc::now(),
    }
}

// ============ 协作管理器 ============

pub struct CollaborationManager {
    sessions: HashMap<String, CollaborationSession>,
    participants: HashMap<String, Vec<CollaborationParticipant>>,
    annotations: HashMap<String, Vec<Annotation>>,
    comments: HashMap<String, Vec<Comment>>,
    history: HashMap<String, Vec<CollaborationHistory>>,
    clipboard: HashMap<String, Vec<SharedClipboardItem>>,
    recordings: HashMap<String, CollaborationRecording>,
}

impl CollaborationManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            participants: HashMap::new(),
            annotations: HashMap::new(),
            comments: HashMap::new(),
            history: HashMap::new(),
            clipboard: HashMap::new(),
            recordings: HashMap::new(),
        }
    }

    // 会话管理
    pub fn create_session(
        &mut self,
        host_id: &str,
        host_username: &str,
        team_id: &str,
        server_id: &str,
        server_name: &str,
    ) -> CollaborationSession {
        let session = create_collaboration_session(
            host_id,
            host_username,
            team_id,
            server_id,
            server_name,
        );
        let id = session.id.clone();

        // 创建参与者列表
        let host_participant = create_participant(&id, host_id, host_username, CollaborationRole::Admin);
        self.participants.insert(id.clone(), vec![host_participant]);

        // 初始化其他存储
        self.annotations.insert(id.clone(), Vec::new());
        self.comments.insert(id.clone(), Vec::new());
        self.history.insert(id.clone(), Vec::new());
        self.clipboard.insert(id.clone(), Vec::new());

        // 添加历史记录
        let entry = create_history_entry(&id, host_id, host_username, CollaborationActionType::Join, None, None);
        if let Some(hist) = self.history.get_mut(&id) {
            hist.push(entry);
        }

        self.sessions.insert(id.clone(), session.clone());
        session
    }

    pub fn get_session(&self, session_id: &str) -> Option<&CollaborationSession> {
        self.sessions.get(session_id)
    }

    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut CollaborationSession> {
        self.sessions.get_mut(session_id)
    }

    pub fn join_session(
        &mut self,
        session_id: &str,
        user_id: &str,
        username: &str,
        role: CollaborationRole,
    ) -> Result<CollaborationParticipant, LiteError> {
        let session = self.sessions.get(session_id)
            .ok_or_else(|| LiteError::Config("Session not found".to_string()))?;

        // 检查会话状态
        if session.state != CollaborationState::Active && session.state != CollaborationState::Recording {
            return Err(LiteError::Config("Session is not active".to_string()));
        }

        // 检查参与者数量
        let participants = self.participants.get(session_id)
            .ok_or_else(|| LiteError::Config("Session not found".to_string()))?;

        if participants.len() >= session.settings.max_participants as usize {
            return Err(LiteError::Config("Session is full".to_string()));
        }

        // 检查是否已参与
        if participants.iter().any(|p| p.user_id == user_id) {
            return Err(LiteError::Config("Already in session".to_string()));
        }

        let participant = create_participant(session_id, user_id, username, role);

        if let Some(parts) = self.participants.get_mut(session_id) {
            parts.push(participant.clone());
        }

        // 添加历史记录
        let entry = create_history_entry(session_id, user_id, username, CollaborationActionType::Join, None, None);
        if let Some(hist) = self.history.get_mut(session_id) {
            hist.push(entry);
        }

        Ok(participant)
    }

    pub fn leave_session(&mut self, session_id: &str, user_id: &str) -> Result<(), LiteError> {
        let parts = self.participants.get_mut(session_id)
            .ok_or_else(|| LiteError::Config("Session not found".to_string()))?;

        if let Some(participant) = parts.iter().find(|p| p.user_id == user_id) {
            let entry = create_history_entry(
                session_id,
                user_id,
                &participant.username,
                CollaborationActionType::Leave,
                None,
                None,
            );
            if let Some(hist) = self.history.get_mut(session_id) {
                hist.push(entry);
            }
        }

        parts.retain(|p| p.user_id != user_id);

        // 如果没有参与者了，结束会话
        if parts.is_empty() {
            if let Some(session) = self.sessions.get_mut(session_id) {
                session.state = CollaborationState::Ended;
                session.ended_at = Some(Utc::now());
            }
        }

        Ok(())
    }

    pub fn get_participants(&self, session_id: &str) -> Vec<&CollaborationParticipant> {
        self.participants.get(session_id)
            .map(|p| p.iter().collect())
            .unwrap_or_default()
    }

    pub fn update_cursor_position(
        &mut self,
        session_id: &str,
        user_id: &str,
        row: u32,
        col: u32,
    ) -> Result<(), LiteError> {
        let parts = self.participants.get_mut(session_id)
            .ok_or_else(|| LiteError::Config("Session not found".to_string()))?;

        if let Some(participant) = parts.iter_mut().find(|p| p.user_id == user_id) {
            participant.cursor_position = Some(CursorPosition {
                row,
                col,
                timestamp: Utc::now(),
            });
            participant.last_active_at = Utc::now();
        }

        Ok(())
    }

    pub fn update_voice_state(&mut self, session_id: &str, user_id: &str, is_active: bool) -> Result<(), LiteError> {
        let parts = self.participants.get_mut(session_id)
            .ok_or_else(|| LiteError::Config("Session not found".to_string()))?;

        if let Some(participant) = parts.iter_mut().find(|p| p.user_id == user_id) {
            participant.is_voice_active = is_active;
            participant.last_active_at = Utc::now();
        }

        Ok(())
    }

    // 标注管理
    pub fn add_annotation(&mut self, annotation: Annotation) -> Result<(), LiteError> {
        let session_id = annotation.session_id.clone();

        if let Some(anns) = self.annotations.get_mut(&session_id) {
            anns.push(annotation.clone());
        }

        // 添加历史记录
        let entry = create_history_entry(
            &session_id,
            &annotation.author_id,
            &annotation.author_name,
            CollaborationActionType::Annotate,
            None,
            Some(&format!("{:?} annotation", annotation.annotation_type)),
        );
        if let Some(hist) = self.history.get_mut(&session_id) {
            hist.push(entry);
        }

        Ok(())
    }

    pub fn get_annotations(&self, session_id: &str) -> Vec<&Annotation> {
        self.annotations.get(session_id)
            .map(|a| a.iter().filter(|ann| ann.resolved_at.is_none()).collect())
            .unwrap_or_default()
    }

    pub fn resolve_annotation(&mut self, session_id: &str, annotation_id: &str) -> Result<(), LiteError> {
        let anns = self.annotations.get_mut(session_id)
            .ok_or_else(|| LiteError::Config("Session not found".to_string()))?;

        if let Some(ann) = anns.iter_mut().find(|a| a.id == annotation_id) {
            ann.resolved_at = Some(Utc::now());
        }

        Ok(())
    }

    // 评论管理
    pub fn add_comment(&mut self, comment: Comment) -> Result<(), LiteError> {
        let session_id = comment.session_id.clone();

        if let Some(comments) = self.comments.get_mut(&session_id) {
            comments.push(comment.clone());
        }

        // 添加历史记录
        let entry = create_history_entry(
            &session_id,
            &comment.author_id,
            &comment.author_name,
            CollaborationActionType::Comment,
            None,
            Some(&comment.content[..comment.content.len().min(50)]),
        );
        if let Some(hist) = self.history.get_mut(&session_id) {
            hist.push(entry);
        }

        Ok(())
    }

    pub fn get_comments(&self, session_id: &str) -> Vec<&Comment> {
        self.comments.get(session_id)
            .map(|c| c.iter().filter(|comm| !comm.resolved).collect())
            .unwrap_or_default()
    }

    pub fn add_reply(&mut self, session_id: &str, comment_id: &str, reply: CommentReply) -> Result<(), LiteError> {
        let comments = self.comments.get_mut(session_id)
            .ok_or_else(|| LiteError::Config("Session not found".to_string()))?;

        if let Some(comment) = comments.iter_mut().find(|c| c.id == comment_id) {
            comment.replies.push(reply);
            comment.updated_at = Some(Utc::now());
        }

        Ok(())
    }

    pub fn resolve_comment(&mut self, session_id: &str, comment_id: &str) -> Result<(), LiteError> {
        let comments = self.comments.get_mut(session_id)
            .ok_or_else(|| LiteError::Config("Session not found".to_string()))?;

        if let Some(comment) = comments.iter_mut().find(|c| c.id == comment_id) {
            comment.resolved = true;
        }

        Ok(())
    }

    // 剪贴板管理
    pub fn add_clipboard_item(&mut self, item: SharedClipboardItem) -> Result<(), LiteError> {
        let session_id = item.session_id.clone();
        let author_id = item.author_id.clone();
        let author_name = item.author_name.clone();
        let content_preview = item.content.clone();
        let content_type = item.content_type.clone();

        if let Some(items) = self.clipboard.get_mut(&session_id) {
            items.push(item);
            // 只保留最近50个
            if items.len() > 50 {
                items.remove(0);
            }
        }

        // 添加历史记录
        let entry = create_history_entry(
            &session_id,
            &author_id,
            &author_name,
            CollaborationActionType::ClipboardSync,
            None,
            Some(&format!("{:?}: {}", content_type, &content_preview[..content_preview.len().min(30)])),
        );
        if let Some(hist) = self.history.get_mut(&session_id) {
            hist.push(entry);
        }

        Ok(())
    }

    pub fn get_clipboard_items(&self, session_id: &str, limit: usize) -> Vec<&SharedClipboardItem> {
        self.clipboard.get(session_id)
            .map(|items| {
                items.iter().rev().take(limit).collect()
            })
            .unwrap_or_default()
    }

    // 历史记录
    pub fn get_history(&self, session_id: &str, limit: usize) -> Vec<&CollaborationHistory> {
        self.history.get(session_id)
            .map(|h| h.iter().rev().take(limit).collect())
            .unwrap_or_default()
    }

    pub fn add_command_history(
        &mut self,
        session_id: &str,
        participant_id: &str,
        participant_name: &str,
        command: &str,
        output_preview: Option<&str>,
    ) {
        let entry = create_history_entry(
            session_id,
            participant_id,
            participant_name,
            CollaborationActionType::ExecuteCommand,
            Some(command),
            output_preview,
        );
        if let Some(hist) = self.history.get_mut(session_id) {
            hist.push(entry);
        }
    }

    // 录制管理
    pub fn start_recording(&mut self, session_id: &str) -> Result<(), LiteError> {
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| LiteError::Config("Session not found".to_string()))?;

        if session.state == CollaborationState::Recording {
            return Err(LiteError::Config("Already recording".to_string()));
        }

        session.state = CollaborationState::Recording;
        session.settings.record_session = true;

        let recording = CollaborationRecording {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            started_at: Utc::now(),
            ended_at: None,
            host_id: session.host_id.clone(),
            segments: Vec::new(),
            total_events: 0,
            file_size: 0,
        };

        self.recordings.insert(session_id.to_string(), recording);

        Ok(())
    }

    pub fn stop_recording(&mut self, session_id: &str) -> Result<CollaborationRecording, LiteError> {
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| LiteError::Config("Session not found".to_string()))?;

        if session.state != CollaborationState::Recording {
            return Err(LiteError::Config("Not recording".to_string()));
        }

        session.state = CollaborationState::Active;

        let recording = self.recordings.get_mut(session_id)
            .ok_or_else(|| LiteError::Config("Recording not found".to_string()))?;

        recording.ended_at = Some(Utc::now());

        Ok(recording.clone())
    }

    pub fn get_recording(&self, session_id: &str) -> Option<&CollaborationRecording> {
        self.recordings.get(session_id)
    }

    // 通过分享链接获取会话
    pub fn get_session_by_link(&self, share_link: &str) -> Option<&CollaborationSession> {
        self.sessions.values().find(|s| s.share_link == share_link)
    }

    // 结束会话
    pub fn end_session(&mut self, session_id: &str, user_id: &str) -> Result<(), LiteError> {
        let is_recording = {
            let session = self.sessions.get(session_id)
                .ok_or_else(|| LiteError::Config("Session not found".to_string()))?;
            if session.host_id != user_id {
                return Err(LiteError::Config("Only host can end session".to_string()));
            }
            session.state == CollaborationState::Recording
        };

        // 如果正在录制，先停止
        if is_recording {
            self.stop_recording(session_id).ok();
        }

        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| LiteError::Config("Session not found".to_string()))?;
        session.state = CollaborationState::Ended;
        session.ended_at = Some(Utc::now());

        Ok(())
    }
}

impl Default for CollaborationManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============ WebSocket消息协议 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CollaborationMessage {
    // 会话管理
    SessionCreate {
        team_id: String,
        server_id: String,
        server_name: String,
        settings: CollaborationSettings,
    },
    SessionJoin {
        session_id: String,
        share_link: Option<String>,
    },
    SessionLeave {
        session_id: String,
    },
    SessionEnd {
        session_id: String,
    },
    SessionState {
        session: CollaborationSession,
        participants: Vec<CollaborationParticipant>,
    },

    // 参与者
    ParticipantJoined {
        participant: CollaborationParticipant,
    },
    ParticipantLeft {
        user_id: String,
    },
    ParticipantUpdated {
        participant: CollaborationParticipant,
    },
    ParticipantList {
        participants: Vec<CollaborationParticipant>,
    },

    // 终端同步
    TerminalOutput {
        session_id: String,
        data: String,
        from_user_id: String,
    },
    TerminalInput {
        session_id: String,
        data: String,
        from_user_id: String,
    },
    TerminalResize {
        session_id: String,
        rows: u32,
        cols: u32,
    },
    TerminalScroll {
        session_id: String,
        position: u32,
        from_user_id: String,
    },

    // 光标同步
    CursorUpdate {
        session_id: String,
        user_id: String,
        username: String,
        position: CursorPosition,
        color: String,
    },

    // 标注
    AnnotationCreate {
        annotation: Annotation,
    },
    AnnotationDelete {
        annotation_id: String,
    },
    AnnotationResolve {
        annotation_id: String,
    },
    AnnotationList {
        annotations: Vec<Annotation>,
    },

    // 评论
    CommentCreate {
        comment: Comment,
    },
    CommentReply {
        comment_id: String,
        reply: CommentReply,
    },
    CommentResolve {
        comment_id: String,
    },
    CommentList {
        comments: Vec<Comment>,
    },

    // 剪贴板
    ClipboardSync {
        item: SharedClipboardItem,
    },
    ClipboardRequest {
        session_id: String,
    },
    ClipboardList {
        items: Vec<SharedClipboardItem>,
    },

    // WebRTC信令
    WebRTCSignal {
        signal: WebRTCSignal,
    },
    VoiceState {
        user_id: String,
        is_active: bool,
    },

    // 历史记录
    HistoryRequest {
        session_id: String,
        limit: Option<usize>,
    },
    HistoryResponse {
        history: Vec<CollaborationHistory>,
    },

    // 错误
    Error {
        code: String,
        message: String,
    },

    // 心跳
    Ping {
        timestamp: DateTime<Utc>,
    },
    Pong {
        timestamp: DateTime<Utc>,
    },
}

// ============ 单元测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collaboration_role_permissions() {
        assert!(CollaborationRole::Operator.can_input());
        assert!(CollaborationRole::Admin.can_input());
        assert!(!CollaborationRole::Observer.can_input());

        assert!(CollaborationRole::Admin.can_manage());
        assert!(!CollaborationRole::Operator.can_manage());
        assert!(!CollaborationRole::Observer.can_manage());

        assert!(CollaborationRole::Observer.can_voice());
    }

    #[test]
    fn test_create_session() {
        let session = create_collaboration_session(
            "user1",
            "TestUser",
            "team1",
            "server1",
            "My Server",
        );
        assert_eq!(session.host_id, "user1");
        assert_eq!(session.server_name, "My Server");
        assert!(!session.share_link.is_empty());
    }

    #[test]
    fn test_manager_create_and_join() {
        let mut manager = CollaborationManager::new();

        let session = manager.create_session("host1", "Host", "team1", "srv1", "Server 1");
        assert_eq!(session.host_id, "host1");

        let participant = manager.join_session(&session.id, "user1", "User1", CollaborationRole::Observer).unwrap();
        assert_eq!(participant.user_id, "user1");
        assert_eq!(participant.role, CollaborationRole::Observer);

        let participants = manager.get_participants(&session.id);
        assert_eq!(participants.len(), 2); // host + joined user
    }

    #[test]
    fn test_annotations() {
        let mut manager = CollaborationManager::new();
        let session = manager.create_session("host1", "Host", "team1", "srv1", "Server");

        let annotation = create_annotation(
            &session.id,
            "user1",
            "User",
            AnnotationType::Highlight,
            AnnotationPosition {
                x: 100.0,
                y: 200.0,
                width: Some(50.0),
                height: Some(20.0),
                points: None,
            },
            "",
            "#FFFF00",
        );

        manager.add_annotation(annotation.clone()).unwrap();

        let annotations = manager.get_annotations(&session.id);
        assert_eq!(annotations.len(), 1);
    }

    #[test]
    fn test_comments() {
        let mut manager = CollaborationManager::new();
        let session = manager.create_session("host1", "Host", "team1", "srv1", "Server");

        let comment = create_comment(&session.id, "user1", "User", 42, "Check this line");
        manager.add_comment(comment).unwrap();

        let comments = manager.get_comments(&session.id);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].line_number, 42);
    }

    #[test]
    fn test_clipboard() {
        let mut manager = CollaborationManager::new();
        let session = manager.create_session("host1", "Host", "team1", "srv1", "Server");

        let item = create_clipboard_item(&session.id, "user1", "User", "ssh user@host", ClipboardContentType::Command);
        manager.add_clipboard_item(item).unwrap();

        let items = manager.get_clipboard_items(&session.id, 10);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].content_type, ClipboardContentType::Command);
    }

    #[test]
    fn test_recording() {
        let mut manager = CollaborationManager::new();
        let session = manager.create_session("host1", "Host", "team1", "srv1", "Server");

        manager.start_recording(&session.id).unwrap();

        let session_ref = manager.get_session(&session.id).unwrap();
        assert_eq!(session_ref.state, CollaborationState::Recording);

        let recording = manager.stop_recording(&session.id).unwrap();
        assert!(recording.ended_at.is_some());
    }

    #[test]
    fn test_share_link() {
        let mut manager = CollaborationManager::new();
        let session = manager.create_session("host1", "Host", "team1", "srv1", "Server");

        let found = manager.get_session_by_link(&session.share_link);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, session.id);
    }
}
