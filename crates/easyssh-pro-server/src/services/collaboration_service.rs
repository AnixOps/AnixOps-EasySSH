//! 实时协作服务
//! 处理WebSocket连接、WebRTC信令、终端同步、标注等

use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{auth::decode_token, models::*, AppState};

// ============ 协作会话管理 ============

/// 会话广播通道
#[derive(Clone)]
struct SessionChannels {
    tx: broadcast::Sender<CollaborationEvent>,
}

impl SessionChannels {
    fn new() -> Self {
        let (tx, _rx) = broadcast::channel(1000);
        Self { tx }
    }
}

/// 协作事件
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum CollaborationEvent {
    TerminalOutput {
        data: String,
        from_user_id: String,
        from_username: String,
    },
    TerminalInput {
        data: String,
        from_user_id: String,
    },
    CursorUpdate {
        user_id: String,
        username: String,
        row: u32,
        col: u32,
        color: String,
    },
    VoiceState {
        user_id: String,
        is_active: bool,
    },
    AnnotationCreated {
        annotation: serde_json::Value,
    },
    AnnotationDeleted {
        annotation_id: String,
    },
    AnnotationResolved {
        annotation_id: String,
    },
    CommentCreated {
        comment: serde_json::Value,
    },
    CommentReply {
        comment_id: String,
        reply: serde_json::Value,
    },
    CommentResolved {
        comment_id: String,
    },
    ClipboardSync {
        item: serde_json::Value,
    },
    ParticipantJoined {
        participant: serde_json::Value,
    },
    ParticipantLeft {
        user_id: String,
    },
    RoleChanged {
        user_id: String,
        new_role: String,
    },
    WebRTCOffer {
        from_user_id: String,
        to_user_id: String,
        sdp: String,
    },
    WebRTCAnswer {
        from_user_id: String,
        to_user_id: String,
        sdp: String,
    },
    WebRTCIceCandidate {
        from_user_id: String,
        to_user_id: String,
        candidate: serde_json::Value,
    },
    SessionEnded,
}

/// 全局会话管理器
pub struct CollaborationService {
    sessions: Arc<RwLock<HashMap<String, SessionChannels>>>,
    user_connections: Arc<RwLock<HashMap<String, Vec<String>>>>, // session_id -> user_ids
}

impl CollaborationService {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            user_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_or_create_session(&self, session_id: &str) -> SessionChannels {
        let mut sessions = self.sessions.write().await;
        sessions
            .entry(session_id.to_string())
            .or_insert_with(SessionChannels::new)
            .clone()
    }

    async fn register_user(&self, session_id: &str, user_id: &str) {
        let mut connections = self.user_connections.write().await;
        connections
            .entry(session_id.to_string())
            .or_insert_with(Vec::new)
            .push(user_id.to_string());
    }

    async fn unregister_user(&self, session_id: &str, user_id: &str) {
        let mut connections = self.user_connections.write().await;
        if let Some(users) = connections.get_mut(session_id) {
            users.retain(|u| u != user_id);
        }
    }

    async fn broadcast(&self, session_id: &str, event: CollaborationEvent) -> Result<()> {
        let sessions = self.sessions.read().await;
        if let Some(channels) = sessions.get(session_id) {
            let _ = channels.tx.send(event);
        }
        Ok(())
    }

    async fn subscribe(&self, session_id: &str) -> broadcast::Receiver<CollaborationEvent> {
        let session = self.get_or_create_session(session_id).await;
        session.tx.subscribe()
    }
}

impl Default for CollaborationService {
    fn default() -> Self {
        Self::new()
    }
}

// ============ API路由 ============

pub fn collaboration_routes() -> Router<AppState> {
    Router::new()
        .route("/sessions", post(create_session))
        .route(
            "/sessions/:session_id",
            get(get_session).delete(end_session),
        )
        .route("/sessions/:session_id/join", post(join_session))
        .route("/sessions/:session_id/leave", post(leave_session))
        .route("/sessions/:session_id/participants", get(list_participants))
        .route(
            "/sessions/:session_id/participants/:user_id/role",
            post(change_role),
        )
        .route(
            "/sessions/:session_id/annotations",
            get(list_annotations).post(create_annotation),
        )
        .route(
            "/sessions/:session_id/annotations/:annotation_id",
            delete(delete_annotation),
        )
        .route(
            "/sessions/:session_id/annotations/:annotation_id/resolve",
            post(resolve_annotation),
        )
        .route(
            "/sessions/:session_id/comments",
            get(list_comments).post(create_comment),
        )
        .route(
            "/sessions/:session_id/comments/:comment_id/replies",
            post(add_reply),
        )
        .route(
            "/sessions/:session_id/comments/:comment_id/resolve",
            post(resolve_comment),
        )
        .route(
            "/sessions/:session_id/clipboard",
            get(list_clipboard).post(add_clipboard),
        )
        .route("/sessions/:session_id/history", get(get_history))
        .route(
            "/sessions/:session_id/recording/start",
            post(start_recording),
        )
        .route("/sessions/:session_id/recording/stop", post(stop_recording))
        .route("/join/:share_link", get(join_by_link))
        .route("/ws/:session_id", get(websocket_handler))
}

// ============ 请求/响应类型 ============

#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    team_id: String,
    server_id: String,
    server_name: String,
    settings: Option<CollaborationSettings>,
}

#[derive(Debug, Serialize)]
struct SessionResponse {
    session: CollaborationSession,
    participants: Vec<CollaborationParticipant>,
}

#[derive(Debug, Deserialize)]
struct JoinRequest {
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChangeRoleRequest {
    new_role: String,
}

#[derive(Debug, Deserialize)]
struct CreateAnnotationRequest {
    annotation_type: String,
    position: AnnotationPosition,
    content: String,
    color: String,
}

#[derive(Debug, Deserialize)]
struct CreateCommentRequest {
    line_number: u32,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AddReplyRequest {
    content: String,
}

#[derive(Debug, Deserialize)]
struct AddClipboardRequest {
    content: String,
    content_type: String,
}

// ============ 处理函数 ============

async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    // 获取当前用户（从JWT）
    let user_id = "current_user".to_string(); // 实际应从auth提取
    let username = "current_user".to_string();

    let mut settings = req.settings.unwrap_or_default();
    settings.max_participants = settings.max_participants.min(20); // 限制最大参与者

    let session = easyssh_core::collaboration::create_collaboration_session(
        &user_id,
        &username,
        &req.team_id,
        &req.server_id,
        &req.server_name,
    );

    // 存储到数据库
    if let Err(e) = store_session(&state.db, &session).await {
        error!("Failed to store session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create session" })),
        );
    }

    // 创建参与者
    let participant = easyssh_core::collaboration::create_participant(
        &session.id,
        &user_id,
        &username,
        easyssh_core::collaboration::CollaborationRole::Admin,
    );

    if let Err(e) = store_participant(&state.db, &participant).await {
        error!("Failed to store participant: {}", e);
    }

    info!("Created collaboration session: {}", session.id);

    (
        StatusCode::CREATED,
        Json(json!({
            "session": session,
            "share_link": format!("{}/collaboration/join/{}", state.config.base_url, session.share_link),
        })),
    )
}

async fn get_session(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match get_session_from_db(&state.db, &session_id).await {
        Ok(Some(session)) => {
            let participants = get_participants_from_db(&state.db, &session_id)
                .await
                .unwrap_or_default();
            (
                StatusCode::OK,
                Json(json!({
                    "session": session,
                    "participants": participants,
                })),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Session not found"})),
        ),
        Err(e) => {
            error!("Failed to get session: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        }
    }
}

async fn join_session(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<JoinRequest>,
) -> impl IntoResponse {
    let user_id = "current_user".to_string();
    let username = "current_user".to_string();

    let role = match req.role.as_deref() {
        Some("operator") => easyssh_core::collaboration::CollaborationRole::Operator,
        Some("admin") => easyssh_core::collaboration::CollaborationRole::Admin,
        _ => easyssh_core::collaboration::CollaborationRole::Observer,
    };

    // 检查会话是否存在且活跃
    match get_session_from_db(&state.db, &session_id).await {
        Ok(Some(session)) => {
            if session.state != easyssh_core::collaboration::CollaborationState::Active
                && session.state != easyssh_core::collaboration::CollaborationState::Recording
            {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({"error": "Session is not active"})),
                );
            }

            let participant = easyssh_core::collaboration::create_participant(
                &session_id,
                &user_id,
                &username,
                role,
            );

            if let Err(e) = store_participant(&state.db, &participant).await {
                error!("Failed to store participant: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Database error"})),
                );
            }

            // 添加到历史记录
            let history = easyssh_core::collaboration::create_history_entry(
                &session_id,
                &user_id,
                &username,
                easyssh_core::collaboration::CollaborationActionType::Join,
                None,
                None,
            );
            let _ = store_history(&state.db, &history).await;

            info!("User {} joined session {}", user_id, session_id);

            (
                StatusCode::OK,
                Json(json!({
                    "participant": participant,
                    "websocket_url": format!("/api/v1/collaboration/ws/{}", session_id),
                })),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Session not found"})),
        ),
        Err(e) => {
            error!("Failed to join session: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        }
    }
}

async fn join_by_link(
    Path(share_link): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // 通过分享链接查找会话
    match get_session_by_link_from_db(&state.db, &share_link).await {
        Ok(Some(session)) => (
            StatusCode::OK,
            Json(json!({
                "session_id": session.id,
                "server_name": session.server_name,
                "host_username": session.host_username,
                "state": session.state,
            })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Invalid share link"})),
        ),
        Err(e) => {
            error!("Failed to get session by link: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        }
    }
}

async fn leave_session(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user_id = "current_user".to_string();
    let username = "current_user".to_string();

    if let Err(e) = remove_participant(&state.db, &session_id, &user_id).await {
        error!("Failed to remove participant: {}", e);
    }

    // 添加历史记录
    let history = easyssh_core::collaboration::create_history_entry(
        &session_id,
        &user_id,
        &username,
        easyssh_core::collaboration::CollaborationActionType::Leave,
        None,
        None,
    );
    let _ = store_history(&state.db, &history).await;

    info!("User {} left session {}", user_id, session_id);

    (StatusCode::OK, Json(json!({"success": true})))
}

async fn end_session(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user_id = "current_user".to_string();

    // 验证是host
    match get_session_from_db(&state.db, &session_id).await {
        Ok(Some(session)) => {
            if session.host_id != user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({"error": "Only host can end session"})),
                );
            }

            if let Err(e) = update_session_state(&state.db, &session_id, "ended").await {
                error!("Failed to end session: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Database error"})),
                );
            }

            info!("Session {} ended by host", session_id);
            (StatusCode::OK, Json(json!({"success": true})))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Session not found"})),
        ),
        Err(e) => {
            error!("Failed to end session: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        }
    }
}

async fn list_participants(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match get_participants_from_db(&state.db, &session_id).await {
        Ok(participants) => (StatusCode::OK, Json(json!({"participants": participants}))),
        Err(e) => {
            error!("Failed to list participants: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        }
    }
}

async fn change_role(
    Path((session_id, user_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(req): Json<ChangeRoleRequest>,
) -> impl IntoResponse {
    let current_user = "current_user".to_string();

    // 验证当前用户是admin
    match get_participant_role(&state.db, &session_id, &current_user).await {
        Ok(Some(role)) if role == "admin" || role == "owner" => {
            if let Err(e) =
                update_participant_role(&state.db, &session_id, &user_id, &req.new_role).await
            {
                error!("Failed to change role: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Database error"})),
                );
            }

            // 添加历史记录
            let history = easyssh_core::collaboration::create_history_entry(
                &session_id,
                &user_id,
                &user_id,
                easyssh_core::collaboration::CollaborationActionType::RoleChange,
                Some(&format!("Role changed to {}", req.new_role)),
                None,
            );
            let _ = store_history(&state.db, &history).await;

            info!(
                "User {} role changed to {} in session {}",
                user_id, req.new_role, session_id
            );

            (StatusCode::OK, Json(json!({"success": true})))
        }
        Ok(_) => (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "No permission"})),
        ),
        Err(e) => {
            error!("Failed to check role: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        }
    }
}

// 标注API
async fn list_annotations(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match get_annotations_from_db(&state.db, &session_id).await {
        Ok(annotations) => (StatusCode::OK, Json(json!({"annotations": annotations}))),
        Err(e) => {
            error!("Failed to list annotations: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        }
    }
}

async fn create_annotation(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<CreateAnnotationRequest>,
) -> impl IntoResponse {
    let user_id = "current_user".to_string();
    let username = "current_user".to_string();

    let annotation_type = match req.annotation_type.as_str() {
        "draw" => easyssh_core::collaboration::AnnotationType::Draw,
        "highlight" => easyssh_core::collaboration::AnnotationType::Highlight,
        "arrow" => easyssh_core::collaboration::AnnotationType::Arrow,
        "text" => easyssh_core::collaboration::AnnotationType::Text,
        "circle" => easyssh_core::collaboration::AnnotationType::Circle,
        "rectangle" => easyssh_core::collaboration::AnnotationType::Rectangle,
        _ => easyssh_core::collaboration::AnnotationType::Highlight,
    };

    let annotation = easyssh_core::collaboration::create_annotation(
        &session_id,
        &user_id,
        &username,
        annotation_type,
        req.position,
        &req.content,
        &req.color,
    );

    if let Err(e) = store_annotation(&state.db, &annotation).await {
        error!("Failed to store annotation: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        );
    }

    (StatusCode::CREATED, Json(json!({"annotation": annotation})))
}

async fn delete_annotation(
    Path((session_id, annotation_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Err(e) = delete_annotation_from_db(&state.db, &annotation_id).await {
        error!("Failed to delete annotation: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        );
    }

    (StatusCode::OK, Json(json!({"success": true})))
}

async fn resolve_annotation(
    Path((session_id, annotation_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Err(e) = resolve_annotation_in_db(&state.db, &annotation_id).await {
        error!("Failed to resolve annotation: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        );
    }

    (StatusCode::OK, Json(json!({"success": true})))
}

// 评论API
async fn list_comments(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match get_comments_from_db(&state.db, &session_id).await {
        Ok(comments) => (StatusCode::OK, Json(json!({"comments": comments}))),
        Err(e) => {
            error!("Failed to list comments: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        }
    }
}

async fn create_comment(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<CreateCommentRequest>,
) -> impl IntoResponse {
    let user_id = "current_user".to_string();
    let username = "current_user".to_string();

    let comment = easyssh_core::collaboration::create_comment(
        &session_id,
        &user_id,
        &username,
        req.line_number,
        &req.content,
    );

    if let Err(e) = store_comment(&state.db, &comment).await {
        error!("Failed to store comment: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        );
    }

    (StatusCode::CREATED, Json(json!({"comment": comment})))
}

async fn add_reply(
    Path((session_id, comment_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(req): Json<AddReplyRequest>,
) -> impl IntoResponse {
    let user_id = "current_user".to_string();
    let username = "current_user".to_string();

    let reply = easyssh_core::collaboration::CommentReply {
        id: Uuid::new_v4().to_string(),
        author_id: user_id,
        author_name: username,
        content: req.content,
        created_at: Utc::now(),
    };

    if let Err(e) = add_reply_to_db(&state.db, &comment_id, &reply).await {
        error!("Failed to add reply: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        );
    }

    (StatusCode::OK, Json(json!({"reply": reply})))
}

async fn resolve_comment(
    Path((session_id, comment_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Err(e) = resolve_comment_in_db(&state.db, &comment_id).await {
        error!("Failed to resolve comment: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        );
    }

    (StatusCode::OK, Json(json!({"success": true})))
}

// 剪贴板API
async fn list_clipboard(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match get_clipboard_from_db(&state.db, &session_id, 50).await {
        Ok(items) => (StatusCode::OK, Json(json!({"items": items}))),
        Err(e) => {
            error!("Failed to list clipboard: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        }
    }
}

async fn add_clipboard(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<AddClipboardRequest>,
) -> impl IntoResponse {
    let user_id = "current_user".to_string();
    let username = "current_user".to_string();

    let content_type = match req.content_type.as_str() {
        "code" => easyssh_core::collaboration::ClipboardContentType::Code,
        "url" => easyssh_core::collaboration::ClipboardContentType::Url,
        "command" => easyssh_core::collaboration::ClipboardContentType::Command,
        _ => easyssh_core::collaboration::ClipboardContentType::Text,
    };

    let item = easyssh_core::collaboration::create_clipboard_item(
        &session_id,
        &user_id,
        &username,
        &req.content,
        content_type,
    );

    if let Err(e) = store_clipboard_item(&state.db, &item).await {
        error!("Failed to store clipboard item: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        );
    }

    (StatusCode::CREATED, Json(json!({"item": item})))
}

// 历史记录
async fn get_history(
    Path(session_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(100);

    match get_history_from_db(&state.db, &session_id, limit).await {
        Ok(history) => (StatusCode::OK, Json(json!({"history": history}))),
        Err(e) => {
            error!("Failed to get history: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        }
    }
}

// 录制控制
async fn start_recording(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user_id = "current_user".to_string();

    // 验证是host
    match get_session_from_db(&state.db, &session_id).await {
        Ok(Some(session)) => {
            if session.host_id != user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({"error": "Only host can start recording"})),
                );
            }

            if let Err(e) = update_session_state(&state.db, &session_id, "recording").await {
                error!("Failed to start recording: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Database error"})),
                );
            }

            // 创建录制记录
            let recording_id = Uuid::new_v4().to_string();
            let _ = sqlx::query(
                "INSERT INTO collaboration_recordings (id, session_id, host_id, started_at) VALUES (?, ?, ?, ?)"
            )
            .bind(&recording_id)
            .bind(&session_id)
            .bind(&user_id)
            .bind(Utc::now())
            .execute(state.db.pool())
            .await;

            info!("Started recording for session {}", session_id);

            (
                StatusCode::OK,
                Json(json!({
                    "recording_id": recording_id,
                    "started_at": Utc::now(),
                })),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Session not found"})),
        ),
        Err(e) => {
            error!("Failed to start recording: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        }
    }
}

async fn stop_recording(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user_id = "current_user".to_string();

    if let Err(e) = update_session_state(&state.db, &session_id, "active").await {
        error!("Failed to stop recording: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        );
    }

    // 更新录制记录
    let _ = sqlx::query(
        "UPDATE collaboration_recordings SET ended_at = ? WHERE session_id = ? AND ended_at IS NULL"
    )
    .bind(Utc::now())
    .bind(&session_id)
    .execute(state.db.pool())
    .await;

    info!("Stopped recording for session {}", session_id);

    (StatusCode::OK, Json(json!({"success": true})))
}

// ============ WebSocket处理 ============

#[derive(Debug, Deserialize)]
struct WSQueryParams {
    token: String,
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    State(state): State<AppState>,
    Query(params): Query<WSQueryParams>,
) -> Response {
    ws.on_upgrade(move |socket| {
        handle_collaboration_socket(socket, state, session_id, params.token)
    })
}

async fn handle_collaboration_socket(
    socket: WebSocket,
    state: AppState,
    session_id: String,
    token: String,
) {
    let connection_id = Uuid::new_v4().to_string();
    let mut user_id = String::new();
    let mut username = String::new();

    // 验证token
    match decode_token(&token, &state.config.jwt_secret) {
        Ok(claims) => {
            user_id = claims.sub.clone();
            username = claims.sub.clone(); // 或使用claims中的username字段
            info!(
                "Collaboration WebSocket connection {} authenticated for user {}",
                connection_id, user_id
            );
        }
        Err(e) => {
            warn!("WebSocket authentication failed: {}", e);
            return;
        }
    }

    // 检查用户是否在会话中
    match get_participant_from_db(&state.db, &session_id, &user_id).await {
        Ok(Some(_)) => {}
        _ => {
            warn!(
                "User {} is not a participant of session {}",
                user_id, session_id
            );
            return;
        }
    }

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<String>(100);

    // 启动发送任务
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // 订阅广播频道
    let service = CollaborationService::new();
    let mut broadcast_rx = service.subscribe(&session_id).await;

    // 启动广播接收任务
    let broadcast_tx = tx.clone();
    let broadcast_task = tokio::spawn(async move {
        while let Ok(event) = broadcast_rx.recv().await {
            let json = serde_json::to_string(&event).unwrap_or_default();
            if broadcast_tx.send(json).await.is_err() {
                break;
            }
        }
    });

    // 注册连接
    service.register_user(&session_id, &user_id).await;

    // 处理传入消息
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => match serde_json::from_str::<ClientMessage>(&text) {
                Ok(client_msg) => {
                    handle_client_message(
                        client_msg,
                        &tx,
                        &session_id,
                        &user_id,
                        &username,
                        &state,
                        &service,
                    )
                    .await;
                }
                Err(e) => {
                    let error_msg = json!({
                        "type": "error",
                        "message": format!("Invalid message: {}", e)
                    })
                    .to_string();
                    let _ = tx.send(error_msg).await;
                }
            },
            Ok(Message::Binary(bin)) => {
                // 处理二进制数据（如WebRTC数据）
                handle_binary_message(bin, &tx, &session_id, &user_id, &service).await;
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket connection {} closed", connection_id);
                break;
            }
            _ => {}
        }
    }

    // 清理
    service.unregister_user(&session_id, &user_id).await;
    broadcast_task.abort();
    send_task.abort();
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum ClientMessage {
    TerminalOutput {
        data: String,
    },
    TerminalInput {
        data: String,
    },
    CursorUpdate {
        row: u32,
        col: u32,
    },
    VoiceStart,
    VoiceStop,
    WebRTCOffer {
        to_user_id: String,
        sdp: String,
    },
    WebRTCAnswer {
        to_user_id: String,
        sdp: String,
    },
    WebRTCIceCandidate {
        to_user_id: String,
        candidate: serde_json::Value,
    },
    AnnotationCreate {
        annotation_type: String,
        position: serde_json::Value,
        content: String,
        color: String,
    },
    AnnotationDelete {
        annotation_id: String,
    },
    CommentCreate {
        line_number: u32,
        content: String,
    },
    ClipboardShare {
        content: String,
        content_type: String,
    },
    Ping,
}

async fn handle_client_message(
    msg: ClientMessage,
    tx: &mpsc::Sender<String>,
    session_id: &str,
    user_id: &str,
    username: &str,
    state: &AppState,
    service: &CollaborationService,
) {
    match msg {
        ClientMessage::TerminalOutput { data } => {
            let event = CollaborationEvent::TerminalOutput {
                data,
                from_user_id: user_id.to_string(),
                from_username: username.to_string(),
            };
            let _ = service.broadcast(session_id, event).await;

            // 记录命令历史（如果是命令）
            if data.trim().starts_with('$') || data.trim().starts_with('#') {
                let history = easyssh_core::collaboration::create_history_entry(
                    session_id,
                    user_id,
                    username,
                    easyssh_core::collaboration::CollaborationActionType::ExecuteCommand,
                    Some(&data),
                    None,
                );
                let _ = store_history(&state.db, &history).await;
            }
        }
        ClientMessage::TerminalInput { data } => {
            let event = CollaborationEvent::TerminalInput {
                data,
                from_user_id: user_id.to_string(),
            };
            let _ = service.broadcast(session_id, event).await;
        }
        ClientMessage::CursorUpdate { row, col } => {
            let colors = vec![
                "#FF6B6B", "#4ECDC4", "#45B7D1", "#FFA07A", "#98D8C8", "#F7DC6F",
            ];
            let color_idx = user_id
                .bytes()
                .fold(0u32, |acc, b| acc.wrapping_add(b as u32))
                as usize
                % colors.len();
            let color = colors[color_idx].to_string();

            let event = CollaborationEvent::CursorUpdate {
                user_id: user_id.to_string(),
                username: username.to_string(),
                row,
                col,
                color,
            };
            let _ = service.broadcast(session_id, event).await;
        }
        ClientMessage::VoiceStart => {
            let event = CollaborationEvent::VoiceState {
                user_id: user_id.to_string(),
                is_active: true,
            };
            let _ = service.broadcast(session_id, event).await;

            // 更新参与者语音状态
            let _ = update_participant_voice_state(&state.db, session_id, user_id, true).await;
        }
        ClientMessage::VoiceStop => {
            let event = CollaborationEvent::VoiceState {
                user_id: user_id.to_string(),
                is_active: false,
            };
            let _ = service.broadcast(session_id, event).await;

            let _ = update_participant_voice_state(&state.db, session_id, user_id, false).await;
        }
        ClientMessage::WebRTCOffer { to_user_id, sdp } => {
            let event = CollaborationEvent::WebRTCOffer {
                from_user_id: user_id.to_string(),
                to_user_id,
                sdp,
            };
            let _ = service.broadcast(session_id, event).await;
        }
        ClientMessage::WebRTCAnswer { to_user_id, sdp } => {
            let event = CollaborationEvent::WebRTCAnswer {
                from_user_id: user_id.to_string(),
                to_user_id,
                sdp,
            };
            let _ = service.broadcast(session_id, event).await;
        }
        ClientMessage::WebRTCIceCandidate {
            to_user_id,
            candidate,
        } => {
            let event = CollaborationEvent::WebRTCIceCandidate {
                from_user_id: user_id.to_string(),
                to_user_id,
                candidate,
            };
            let _ = service.broadcast(session_id, event).await;
        }
        ClientMessage::AnnotationCreate {
            annotation_type,
            position,
            content,
            color,
        } => {
            // 存储标注
            let annotation = easyssh_core::collaboration::create_annotation(
                session_id,
                user_id,
                username,
                easyssh_core::collaboration::AnnotationType::Highlight, // 简化处理
                easyssh_core::collaboration::AnnotationPosition {
                    x: 0.0,
                    y: 0.0,
                    width: None,
                    height: None,
                    points: None,
                },
                &content,
                &color,
            );
            let _ = store_annotation(&state.db, &annotation).await;

            let event = CollaborationEvent::AnnotationCreated {
                annotation: serde_json::to_value(&annotation).unwrap_or_default(),
            };
            let _ = service.broadcast(session_id, event).await;
        }
        ClientMessage::CommentCreate {
            line_number,
            content,
        } => {
            let comment = easyssh_core::collaboration::create_comment(
                session_id,
                user_id,
                username,
                line_number,
                &content,
            );
            let _ = store_comment(&state.db, &comment).await;

            let event = CollaborationEvent::CommentCreated {
                comment: serde_json::to_value(&comment).unwrap_or_default(),
            };
            let _ = service.broadcast(session_id, event).await;
        }
        ClientMessage::ClipboardShare {
            content,
            content_type,
        } => {
            let item = easyssh_core::collaboration::create_clipboard_item(
                session_id,
                user_id,
                username,
                &content,
                easyssh_core::collaboration::ClipboardContentType::Text,
            );
            let _ = store_clipboard_item(&state.db, &item).await;

            let event = CollaborationEvent::ClipboardSync {
                item: serde_json::to_value(&item).unwrap_or_default(),
            };
            let _ = service.broadcast(session_id, event).await;
        }
        ClientMessage::Ping => {
            let pong = json!({"type": "pong", "timestamp": Utc::now()});
            let _ = tx.send(pong.to_string()).await;
        }
        _ => {}
    }
}

async fn handle_binary_message(
    _data: Vec<u8>,
    _tx: &mpsc::Sender<String>,
    _session_id: &str,
    _user_id: &str,
    _service: &CollaborationService,
) {
    // 处理二进制数据，如压缩的终端数据
}

// ============ 数据库操作 ============

async fn store_session(
    db: &crate::db::Database,
    session: &easyssh_core::collaboration::CollaborationSession,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO collaboration_sessions
           (id, host_id, host_username, team_id, server_id, server_name, state, share_link, created_at, settings)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(&session.id)
    .bind(&session.host_id)
    .bind(&session.host_username)
    .bind(&session.team_id)
    .bind(&session.server_id)
    .bind(&session.server_name)
    .bind(format!("{:?}", session.state))
    .bind(&session.share_link)
    .bind(session.created_at)
    .bind(serde_json::to_string(&session.settings)?)
    .execute(db.pool())
    .await?;

    Ok(())
}

async fn get_session_from_db(
    db: &crate::db::Database,
    session_id: &str,
) -> Result<Option<easyssh_core::collaboration::CollaborationSession>> {
    let row = sqlx::query_as::<_, CollaborationSessionRow>(
        "SELECT * FROM collaboration_sessions WHERE id = ?",
    )
    .bind(session_id)
    .fetch_optional(db.pool())
    .await?;

    Ok(row.map(|r| r.into()))
}

async fn get_session_by_link_from_db(
    db: &crate::db::Database,
    share_link: &str,
) -> Result<Option<easyssh_core::collaboration::CollaborationSession>> {
    let row = sqlx::query_as::<_, CollaborationSessionRow>(
        "SELECT * FROM collaboration_sessions WHERE share_link = ?",
    )
    .bind(share_link)
    .fetch_optional(db.pool())
    .await?;

    Ok(row.map(|r| r.into()))
}

async fn update_session_state(
    db: &crate::db::Database,
    session_id: &str,
    state: &str,
) -> Result<()> {
    sqlx::query("UPDATE collaboration_sessions SET state = ? WHERE id = ?")
        .bind(state)
        .bind(session_id)
        .execute(db.pool())
        .await?;

    Ok(())
}

async fn store_participant(
    db: &crate::db::Database,
    participant: &easyssh_core::collaboration::CollaborationParticipant,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO collaboration_participants
           (id, session_id, user_id, username, role, joined_at, is_online)
           VALUES (?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&participant.id)
    .bind(&participant.session_id)
    .bind(&participant.user_id)
    .bind(&participant.username)
    .bind(format!("{:?}", participant.role))
    .bind(participant.joined_at)
    .bind(true)
    .execute(db.pool())
    .await?;

    Ok(())
}

async fn get_participants_from_db(
    db: &crate::db::Database,
    session_id: &str,
) -> Result<Vec<easyssh_core::collaboration::CollaborationParticipant>> {
    let rows = sqlx::query_as::<_, CollaborationParticipantRow>(
        "SELECT * FROM collaboration_participants WHERE session_id = ? AND is_online = TRUE",
    )
    .bind(session_id)
    .fetch_all(db.pool())
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

async fn get_participant_from_db(
    db: &crate::db::Database,
    session_id: &str,
    user_id: &str,
) -> Result<Option<easyssh_core::collaboration::CollaborationParticipant>> {
    let row = sqlx::query_as::<_, CollaborationParticipantRow>(
        "SELECT * FROM collaboration_participants WHERE session_id = ? AND user_id = ?",
    )
    .bind(session_id)
    .bind(user_id)
    .fetch_optional(db.pool())
    .await?;

    Ok(row.map(|r| r.into()))
}

async fn remove_participant(
    db: &crate::db::Database,
    session_id: &str,
    user_id: &str,
) -> Result<()> {
    sqlx::query("UPDATE collaboration_participants SET is_online = FALSE WHERE session_id = ? AND user_id = ?")
        .bind(session_id)
        .bind(user_id)
        .execute(db.pool())
        .await?;

    Ok(())
}

async fn get_participant_role(
    db: &crate::db::Database,
    session_id: &str,
    user_id: &str,
) -> Result<Option<String>> {
    let role: Option<(String,)> = sqlx::query_as(
        "SELECT role FROM collaboration_participants WHERE session_id = ? AND user_id = ?",
    )
    .bind(session_id)
    .bind(user_id)
    .fetch_optional(db.pool())
    .await?;

    Ok(role.map(|r| r.0))
}

async fn update_participant_role(
    db: &crate::db::Database,
    session_id: &str,
    user_id: &str,
    role: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE collaboration_participants SET role = ? WHERE session_id = ? AND user_id = ?",
    )
    .bind(role)
    .bind(session_id)
    .bind(user_id)
    .execute(db.pool())
    .await?;

    Ok(())
}

async fn update_participant_voice_state(
    db: &crate::db::Database,
    session_id: &str,
    user_id: &str,
    is_active: bool,
) -> Result<()> {
    sqlx::query("UPDATE collaboration_participants SET is_voice_active = ? WHERE session_id = ? AND user_id = ?")
        .bind(is_active)
        .bind(session_id)
        .bind(user_id)
        .execute(db.pool())
    .await?;

    Ok(())
}

async fn store_annotation(
    db: &crate::db::Database,
    annotation: &easyssh_core::collaboration::Annotation,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO collaboration_annotations
           (id, session_id, author_id, author_name, annotation_type, position, content, color, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(&annotation.id)
    .bind(&annotation.session_id)
    .bind(&annotation.author_id)
    .bind(&annotation.author_name)
    .bind(format!("{:?}", annotation.annotation_type))
    .bind(serde_json::to_string(&annotation.position)?)
    .bind(&annotation.content)
    .bind(&annotation.color)
    .bind(annotation.created_at)
    .execute(db.pool())
    .await?;

    Ok(())
}

async fn get_annotations_from_db(
    db: &crate::db::Database,
    session_id: &str,
) -> Result<Vec<easyssh_core::collaboration::Annotation>> {
    let rows = sqlx::query_as::<_, AnnotationRow>(
        "SELECT * FROM collaboration_annotations WHERE session_id = ? AND resolved_at IS NULL",
    )
    .bind(session_id)
    .fetch_all(db.pool())
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

async fn delete_annotation_from_db(db: &crate::db::Database, annotation_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM collaboration_annotations WHERE id = ?")
        .bind(annotation_id)
        .execute(db.pool())
        .await?;

    Ok(())
}

async fn resolve_annotation_in_db(db: &crate::db::Database, annotation_id: &str) -> Result<()> {
    sqlx::query("UPDATE collaboration_annotations SET resolved_at = ? WHERE id = ?")
        .bind(Utc::now())
        .bind(annotation_id)
        .execute(db.pool())
        .await?;

    Ok(())
}

async fn store_comment(
    db: &crate::db::Database,
    comment: &easyssh_core::collaboration::Comment,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO collaboration_comments
           (id, session_id, author_id, author_name, line_number, content, created_at, resolved)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&comment.id)
    .bind(&comment.session_id)
    .bind(&comment.author_id)
    .bind(&comment.author_name)
    .bind(comment.line_number as i32)
    .bind(&comment.content)
    .bind(comment.created_at)
    .bind(false)
    .execute(db.pool())
    .await?;

    Ok(())
}

async fn get_comments_from_db(
    db: &crate::db::Database,
    session_id: &str,
) -> Result<Vec<easyssh_core::collaboration::Comment>> {
    let rows = sqlx::query_as::<_, CommentRow>(
        "SELECT * FROM collaboration_comments WHERE session_id = ? AND resolved = FALSE ORDER BY line_number"
    )
    .bind(session_id)
    .fetch_all(db.pool())
    .await?;

    // 简化处理，实际应加载replies
    Ok(rows.into_iter().map(|r| r.into()).collect())
}

async fn add_reply_to_db(
    db: &crate::db::Database,
    comment_id: &str,
    reply: &easyssh_core::collaboration::CommentReply,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO collaboration_comment_replies
           (id, comment_id, author_id, author_name, content, created_at)
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&reply.id)
    .bind(comment_id)
    .bind(&reply.author_id)
    .bind(&reply.author_name)
    .bind(&reply.content)
    .bind(reply.created_at)
    .execute(db.pool())
    .await?;

    sqlx::query("UPDATE collaboration_comments SET updated_at = ? WHERE id = ?")
        .bind(Utc::now())
        .bind(comment_id)
        .execute(db.pool())
        .await?;

    Ok(())
}

async fn resolve_comment_in_db(db: &crate::db::Database, comment_id: &str) -> Result<()> {
    sqlx::query("UPDATE collaboration_comments SET resolved = TRUE WHERE id = ?")
        .bind(comment_id)
        .execute(db.pool())
        .await?;

    Ok(())
}

async fn store_clipboard_item(
    db: &crate::db::Database,
    item: &easyssh_core::collaboration::SharedClipboardItem,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO collaboration_clipboard
           (id, session_id, author_id, author_name, content, content_type, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&item.id)
    .bind(&item.session_id)
    .bind(&item.author_id)
    .bind(&item.author_name)
    .bind(&item.content)
    .bind(format!("{:?}", item.content_type))
    .bind(item.created_at)
    .execute(db.pool())
    .await?;

    Ok(())
}

async fn get_clipboard_from_db(
    db: &crate::db::Database,
    session_id: &str,
    limit: i64,
) -> Result<Vec<easyssh_core::collaboration::SharedClipboardItem>> {
    let rows = sqlx::query_as::<_, ClipboardItemRow>(
        "SELECT * FROM collaboration_clipboard WHERE session_id = ? ORDER BY created_at DESC LIMIT ?"
    )
    .bind(session_id)
    .bind(limit)
    .fetch_all(db.pool())
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

async fn store_history(
    db: &crate::db::Database,
    entry: &easyssh_core::collaboration::CollaborationHistory,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO collaboration_history
           (id, session_id, participant_id, participant_name, action_type, command, output_preview, timestamp)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(&entry.id)
    .bind(&entry.session_id)
    .bind(&entry.participant_id)
    .bind(&entry.participant_name)
    .bind(format!("{:?}", entry.action_type))
    .bind(&entry.command)
    .bind(&entry.output_preview)
    .bind(entry.timestamp)
    .execute(db.pool())
    .await?;

    Ok(())
}

async fn get_history_from_db(
    db: &crate::db::Database,
    session_id: &str,
    limit: i64,
) -> Result<Vec<easyssh_core::collaboration::CollaborationHistory>> {
    let rows = sqlx::query_as::<_, HistoryRow>(
        "SELECT * FROM collaboration_history WHERE session_id = ? ORDER BY timestamp DESC LIMIT ?",
    )
    .bind(session_id)
    .bind(limit)
    .fetch_all(db.pool())
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

// ============ 数据库行类型 ============

#[derive(sqlx::FromRow)]
struct CollaborationSessionRow {
    id: String,
    host_id: String,
    host_username: String,
    team_id: String,
    server_id: String,
    server_name: String,
    state: String,
    share_link: String,
    created_at: chrono::DateTime<Utc>,
    settings: String,
}

impl From<CollaborationSessionRow> for easyssh_core::collaboration::CollaborationSession {
    fn from(row: CollaborationSessionRow) -> Self {
        Self {
            id: row.id,
            host_id: row.host_id,
            host_username: row.host_username,
            team_id: row.team_id,
            server_id: row.server_id,
            server_name: row.server_name,
            state: match row.state.as_str() {
                "Active" => easyssh_core::collaboration::CollaborationState::Active,
                "Paused" => easyssh_core::collaboration::CollaborationState::Paused,
                "Ended" => easyssh_core::collaboration::CollaborationState::Ended,
                "Recording" => easyssh_core::collaboration::CollaborationState::Recording,
                _ => easyssh_core::collaboration::CollaborationState::Active,
            },
            share_link: row.share_link,
            created_at: row.created_at,
            ended_at: None,
            settings: serde_json::from_str(&row.settings).unwrap_or_default(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct CollaborationParticipantRow {
    id: String,
    session_id: String,
    user_id: String,
    username: String,
    role: String,
    joined_at: chrono::DateTime<Utc>,
    is_voice_active: Option<bool>,
    is_online: Option<bool>,
}

impl From<CollaborationParticipantRow> for easyssh_core::collaboration::CollaborationParticipant {
    fn from(row: CollaborationParticipantRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            user_id: row.user_id,
            username: row.username,
            avatar_url: None,
            role: match row.role.as_str() {
                "Observer" => easyssh_core::collaboration::CollaborationRole::Observer,
                "Operator" => easyssh_core::collaboration::CollaborationRole::Operator,
                "Admin" => easyssh_core::collaboration::CollaborationRole::Admin,
                _ => easyssh_core::collaboration::CollaborationRole::Observer,
            },
            joined_at: row.joined_at,
            last_active_at: row.joined_at,
            is_voice_active: row.is_voice_active.unwrap_or(false),
            cursor_position: None,
            is_online: row.is_online.unwrap_or(true),
        }
    }
}

#[derive(sqlx::FromRow)]
struct AnnotationRow {
    id: String,
    session_id: String,
    author_id: String,
    author_name: String,
    annotation_type: String,
    position: String,
    content: String,
    color: String,
    created_at: chrono::DateTime<Utc>,
    resolved_at: Option<chrono::DateTime<Utc>>,
}

impl From<AnnotationRow> for easyssh_core::collaboration::Annotation {
    fn from(row: AnnotationRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            author_id: row.author_id,
            author_name: row.author_name,
            annotation_type: easyssh_core::collaboration::AnnotationType::Highlight, // 简化
            position: serde_json::from_str(&row.position).unwrap_or(
                easyssh_core::collaboration::AnnotationPosition {
                    x: 0.0,
                    y: 0.0,
                    width: None,
                    height: None,
                    points: None,
                },
            ),
            content: row.content,
            color: row.color,
            created_at: row.created_at,
            resolved_at: row.resolved_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct CommentRow {
    id: String,
    session_id: String,
    author_id: String,
    author_name: String,
    line_number: i32,
    content: String,
    created_at: chrono::DateTime<Utc>,
    updated_at: Option<chrono::DateTime<Utc>>,
    resolved: bool,
}

impl From<CommentRow> for easyssh_core::collaboration::Comment {
    fn from(row: CommentRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            author_id: row.author_id,
            author_name: row.author_name,
            line_number: row.line_number as u32,
            content: row.content,
            created_at: row.created_at,
            updated_at: row.updated_at,
            replies: Vec::new(), // 简化
            resolved: row.resolved,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ClipboardItemRow {
    id: String,
    session_id: String,
    author_id: String,
    author_name: String,
    content: String,
    content_type: String,
    created_at: chrono::DateTime<Utc>,
}

impl From<ClipboardItemRow> for easyssh_core::collaboration::SharedClipboardItem {
    fn from(row: ClipboardItemRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            author_id: row.author_id,
            author_name: row.author_name,
            content: row.content,
            content_type: easyssh_core::collaboration::ClipboardContentType::Text,
            created_at: row.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct HistoryRow {
    id: String,
    session_id: String,
    participant_id: String,
    participant_name: String,
    action_type: String,
    command: Option<String>,
    output_preview: Option<String>,
    timestamp: chrono::DateTime<Utc>,
}

impl From<HistoryRow> for easyssh_core::collaboration::CollaborationHistory {
    fn from(row: HistoryRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            participant_id: row.participant_id,
            participant_name: row.participant_name,
            action_type: easyssh_core::collaboration::CollaborationActionType::Join, // 简化
            command: row.command,
            output_preview: row.output_preview,
            timestamp: row.timestamp,
        }
    }
}

// 兼容类型
#[derive(sqlx::FromRow)]
struct CollaborationSettingsRow {
    #[allow(dead_code)]
    allow_observers: bool,
    #[allow(dead_code)]
    require_approval: bool,
    #[allow(dead_code)]
    record_session: bool,
    #[allow(dead_code)]
    enable_voice: bool,
    #[allow(dead_code)]
    enable_annotations: bool,
    #[allow(dead_code)]
    max_participants: i32,
    #[allow(dead_code)]
    allow_clipboard_sync: bool,
}
