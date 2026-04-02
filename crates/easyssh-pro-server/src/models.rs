use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ============= User Models =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub name: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub is_admin: bool,
    pub sso_provider: Option<String>,
    pub sso_id: Option<String>,
    pub mfa_enabled: bool,
    #[serde(skip_serializing)]
    pub mfa_secret: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    #[serde(rename = "mfaCode")]
    pub mfa_code: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
    pub mfa_enabled: bool,
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            avatar_url: user.avatar_url,
            is_admin: user.is_admin,
            mfa_enabled: user.mfa_enabled,
        }
    }
}

// ============= Team Models =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TeamMember {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub role: TeamRole,
    pub joined_at: DateTime<Utc>,
    pub invited_by: Option<String>,
    pub is_active: bool,
    #[sqlx(skip)]
    pub user: Option<UserProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq)]
#[sqlx(type_name = "TEXT")]
pub enum TeamRole {
    Owner,
    Admin,
    Member,
    Guest,
}

impl std::fmt::Display for TeamRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamRole::Owner => write!(f, "owner"),
            TeamRole::Admin => write!(f, "admin"),
            TeamRole::Member => write!(f, "member"),
            TeamRole::Guest => write!(f, "guest"),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTeamRequest {
    pub name: String,
    pub description: Option<String>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTeamRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InviteMemberRequest {
    pub email: String,
    pub role: TeamRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Invitation {
    pub id: String,
    pub team_id: String,
    pub email: String,
    pub role: TeamRole,
    pub invited_by: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub status: InvitationStatus,
    pub accepted_at: Option<DateTime<Utc>>,
    pub accepted_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema)]
#[sqlx(type_name = "TEXT")]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Expired,
    Cancelled,
}

// ============= RBAC Models =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub team_id: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    #[sqlx(skip)]
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Permission {
    pub id: String,
    pub resource_type: String,
    pub action: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckPermissionRequest {
    pub resource_type: String,
    pub action: String,
    pub resource_id: Option<String>,
    pub team_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CheckPermissionResponse {
    pub allowed: bool,
    pub reason: Option<String>,
}

// ============= Audit Log Models =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct AuditLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<String>,
    pub team_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub session_id: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct QueryAuditLogsRequest {
    pub team_id: Option<String>,
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditLogListResponse {
    pub logs: Vec<AuditLog>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// ============= Shared Resources Models =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SharedServer {
    pub id: String,
    pub server_id: String,
    pub team_id: String,
    pub shared_by: String,
    pub shared_at: DateTime<Utc>,
    pub permissions: Option<serde_json::Value>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerPermissions {
    pub can_execute: bool,
    pub can_edit: bool,
    pub can_share: bool,
    pub can_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Snippet {
    pub id: String,
    pub team_id: String,
    pub created_by: String,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub language: Option<String>,
    pub tags: Option<serde_json::Value>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSnippetRequest {
    pub team_id: String,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub language: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_public: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSnippetRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub language: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_public: Option<bool>,
}

// ============= API Key Models =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ApiKey {
    pub id: String,
    pub user_id: String,
    pub name: String,
    #[serde(skip_serializing)]
    pub key_hash: String,
    pub key_prefix: String,
    pub scopes: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub scopes: Option<Vec<String>>,
    pub expires_in_days: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
    pub id: String,
    pub name: String,
    pub api_key: String, // Only returned once at creation
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

// ============= SSO Models =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SsoConfig {
    pub id: String,
    pub team_id: String,
    pub provider_type: SsoProviderType,
    pub provider_name: String,
    pub is_enabled: bool,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema)]
#[sqlx(type_name = "TEXT")]
pub enum SsoProviderType {
    #[serde(rename = "saml")]
    Saml,
    #[serde(rename = "oidc")]
    Oidc,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSamlConfigRequest {
    pub provider_name: String,
    pub metadata_url: Option<String>,
    pub metadata_xml: Option<String>,
    pub sso_url: String,
    pub issuer: String,
    pub certificate: String,
    pub name_id_format: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOidcConfigRequest {
    pub provider_name: String,
    pub client_id: String,
    #[serde(skip_serializing)]
    pub client_secret: String,
    pub authorization_url: String,
    pub token_url: String,
    pub userinfo_url: String,
    pub redirect_url: String,
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SsoLoginUrl {
    pub url: String,
    pub state: String,
}

// ============= WebSocket Models =============

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WebSocketMessage {
    Ping {
        timestamp: DateTime<Utc>,
    },
    Pong {
        timestamp: DateTime<Utc>,
    },
    Subscribe {
        channels: Vec<String>,
    },
    Unsubscribe {
        channels: Vec<String>,
    },
    CollaborationUpdate {
        resource_type: String,
        resource_id: String,
        action: String,
        data: serde_json::Value,
        user_id: String,
        timestamp: DateTime<Utc>,
    },
}

// ============= Collaboration Models =============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CollaborationSession {
    pub id: String,
    pub host_id: String,
    pub host_username: String,
    pub team_id: String,
    pub server_id: String,
    pub server_name: String,
    pub state: String, // Active, Paused, Ended, Recording
    pub share_link: String,
    pub created_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub settings: Option<String>, // JSON
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CollaborationSettings {
    pub allow_observers: bool,
    pub require_approval: bool,
    pub record_session: bool,
    pub enable_voice: bool,
    pub enable_annotations: bool,
    pub max_participants: i32,
    pub allow_clipboard_sync: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CollaborationParticipant {
    pub id: String,
    pub session_id: String,
    pub user_id: String,
    pub username: String,
    pub role: String, // Observer, Operator, Admin
    pub joined_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub is_voice_active: bool,
    pub is_online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CursorPosition {
    pub row: u32,
    pub col: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Annotation {
    pub id: String,
    pub session_id: String,
    pub author_id: String,
    pub author_name: String,
    pub annotation_type: String, // draw, highlight, arrow, text, circle, rectangle
    pub position: String,        // JSON
    pub content: String,
    pub color: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnnotationPosition {
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub points: Option<Vec<(f64, f64)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Comment {
    pub id: String,
    pub session_id: String,
    pub author_id: String,
    pub author_name: String,
    pub line_number: i32,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CommentReply {
    pub id: String,
    pub comment_id: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SharedClipboardItem {
    pub id: String,
    pub session_id: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub content_type: String, // text, code, url, command
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CollaborationHistory {
    pub id: String,
    pub session_id: String,
    pub participant_id: String,
    pub participant_name: String,
    pub action_type: String, // Join, Leave, ExecuteCommand, etc.
    pub command: Option<String>,
    pub output_preview: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CollaborationRecording {
    pub id: String,
    pub session_id: String,
    pub host_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub file_path: Option<String>,
    pub file_size: i64,
    pub total_events: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct RecordingSegment {
    pub id: String,
    pub recording_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub file_path: String,
    pub file_size: i64,
    pub events_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebRTCSignal {
    pub session_id: String,
    pub from_user_id: String,
    pub to_user_id: String,
    pub signal_type: String, // offer, answer, ice_candidate
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSessionRequest {
    pub team_id: String,
    pub server_id: String,
    pub server_name: String,
    pub settings: Option<CollaborationSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionResponse {
    pub session: CollaborationSession,
    pub participants: Vec<CollaborationParticipant>,
    pub websocket_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JoinSessionRequest {
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JoinSessionResponse {
    pub participant: CollaborationParticipant,
    pub websocket_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChangeRoleRequest {
    pub new_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAnnotationRequest {
    pub annotation_type: String,
    pub position: AnnotationPosition,
    pub content: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCommentRequest {
    pub line_number: u32,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AddReplyRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AddClipboardRequest {
    pub content: String,
    pub content_type: String,
}

// ============= Common Models =============

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationInfo {
    pub page: i64,
    pub limit: i64,
    pub total: i64,
    pub total_pages: i64,
    pub has_next: bool,
    pub has_prev: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub code: Option<String>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SuccessResponse<T> {
    pub success: bool,
    pub data: T,
    pub message: Option<String>,
}
