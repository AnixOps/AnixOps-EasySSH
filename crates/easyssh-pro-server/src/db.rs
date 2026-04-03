use anyhow::Result;
use sqlx::{migrate::Migrator, sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::path::Path;

pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = if database_url.starts_with("sqlite:") {
            SqlitePoolOptions::new()
                .max_connections(5)
                .connect(database_url)
                .await?
        } else {
            SqlitePoolOptions::new()
                .max_connections(5)
                .connect(&format!("sqlite:{}", database_url))
                .await?
        };

        // Run migrations
        let migrator = Migrator::new(Path::new("./migrations")).await?;
        migrator.run(&pool).await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    pub async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await?;
        Ok(())
    }
}

// Database migration SQL
pub const MIGRATIONS_SQL: &str = r#"
-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT,
    name TEXT NOT NULL,
    avatar_url TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_login_at TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    is_admin BOOLEAN DEFAULT FALSE,
    sso_provider TEXT,
    sso_id TEXT,
    mfa_enabled BOOLEAN DEFAULT FALSE,
    mfa_secret TEXT
);

-- Teams table
CREATE TABLE IF NOT EXISTS teams (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    created_by TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    settings TEXT, -- JSON
    FOREIGN KEY (created_by) REFERENCES users(id)
);

-- Team Members table
CREATE TABLE IF NOT EXISTS team_members (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL, -- owner, admin, member, guest
    joined_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    invited_by TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (invited_by) REFERENCES users(id),
    UNIQUE(team_id, user_id)
);

-- Invitations table
CREATE TABLE IF NOT EXISTS invitations (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL,
    email TEXT NOT NULL,
    role TEXT NOT NULL,
    invited_by TEXT NOT NULL,
    token TEXT UNIQUE NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    accepted_at TIMESTAMP,
    accepted_by TEXT,
    status TEXT DEFAULT 'pending', -- pending, accepted, expired, cancelled
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    FOREIGN KEY (invited_by) REFERENCES users(id)
);

-- Roles and Permissions (RBAC)
CREATE TABLE IF NOT EXISTS roles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    team_id TEXT, -- NULL for global roles
    is_system BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    UNIQUE(team_id, name)
);

CREATE TABLE IF NOT EXISTS permissions (
    id TEXT PRIMARY KEY,
    resource_type TEXT NOT NULL, -- server, snippet, team, etc.
    action TEXT NOT NULL, -- create, read, update, delete, execute, etc.
    description TEXT,
    UNIQUE(resource_type, action)
);

CREATE TABLE IF NOT EXISTS role_permissions (
    role_id TEXT NOT NULL,
    permission_id TEXT NOT NULL,
    conditions TEXT, -- JSON for conditional permissions
    PRIMARY KEY (role_id, permission_id),
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
);

-- API Keys
CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    key_hash TEXT UNIQUE NOT NULL,
    key_prefix TEXT NOT NULL,
    scopes TEXT, -- JSON array
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,
    last_used_at TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Audit Logs
CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    user_id TEXT,
    team_id TEXT,
    action TEXT NOT NULL, -- create, update, delete, login, logout, etc.
    resource_type TEXT NOT NULL, -- user, team, server, etc.
    resource_id TEXT,
    details TEXT, -- JSON
    ip_address TEXT,
    user_agent TEXT,
    session_id TEXT,
    success BOOLEAN DEFAULT TRUE,
    error_message TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (team_id) REFERENCES teams(id)
);

-- Shared Resources (Servers, Snippets)
CREATE TABLE IF NOT EXISTS shared_servers (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL, -- Reference to local server config
    team_id TEXT NOT NULL,
    shared_by TEXT NOT NULL,
    shared_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    permissions TEXT, -- JSON: { can_execute: bool, can_edit: bool, can_share: bool }
    is_active BOOLEAN DEFAULT TRUE,
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    FOREIGN KEY (shared_by) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS snippets (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL,
    created_by TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    content TEXT NOT NULL,
    language TEXT,
    tags TEXT, -- JSON array
    is_public BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users(id)
);

-- Sessions
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token_hash TEXT UNIQUE NOT NULL,
    refresh_token_hash TEXT UNIQUE NOT NULL,
    device_info TEXT,
    ip_address TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,
    last_active_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- SSO Configurations
CREATE TABLE IF NOT EXISTS sso_configs (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL,
    provider_type TEXT NOT NULL, -- saml, oidc
    provider_name TEXT NOT NULL,
    is_enabled BOOLEAN DEFAULT TRUE,
    config TEXT NOT NULL, -- JSON with provider-specific settings
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    UNIQUE(team_id, provider_name)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_team_id ON audit_logs(team_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_team_members_team_id ON team_members(team_id);
CREATE INDEX IF NOT EXISTS idx_team_members_user_id ON team_members(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token_hash);

-- Insert default permissions
INSERT OR IGNORE INTO permissions (id, resource_type, action, description) VALUES
('perm_server_create', 'server', 'create', 'Create new server connections'),
('perm_server_read', 'server', 'read', 'View server connections'),
('perm_server_update', 'server', 'update', 'Update server connections'),
('perm_server_delete', 'server', 'delete', 'Delete server connections'),
('perm_server_execute', 'server', 'execute', 'Execute commands on servers'),
('perm_snippet_create', 'snippet', 'create', 'Create code snippets'),
('perm_snippet_read', 'snippet', 'read', 'View code snippets'),
('perm_snippet_update', 'snippet', 'update', 'Update code snippets'),
('perm_snippet_delete', 'snippet', 'delete', 'Delete code snippets'),
('perm_team_invite', 'team', 'invite', 'Invite members to team'),
('perm_team_manage', 'team', 'manage', 'Manage team settings and members'),
('perm_audit_read', 'audit', 'read', 'View audit logs'),
('perm_rbac_manage', 'rbac', 'manage', 'Manage roles and permissions');

-- Insert default system roles
INSERT OR IGNORE INTO roles (id, name, description, is_system) VALUES
('role_admin', 'Admin', 'Full access to all resources', TRUE),
('role_member', 'Member', 'Standard member with limited access', TRUE),
('role_guest', 'Guest', 'Read-only access', TRUE);

-- Assign permissions to system roles
INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES
('role_admin', 'perm_server_create'),
('role_admin', 'perm_server_read'),
('role_admin', 'perm_server_update'),
('role_admin', 'perm_server_delete'),
('role_admin', 'perm_server_execute'),
('role_admin', 'perm_snippet_create'),
('role_admin', 'perm_snippet_read'),
('role_admin', 'perm_snippet_update'),
('role_admin', 'perm_snippet_delete'),
('role_admin', 'perm_team_invite'),
('role_admin', 'perm_team_manage'),
('role_admin', 'perm_audit_read'),
('role_admin', 'perm_rbac_manage');

-- ==================== Collaboration Tables ====================

-- Collaboration Sessions
CREATE TABLE IF NOT EXISTS collaboration_sessions (
    id TEXT PRIMARY KEY,
    host_id TEXT NOT NULL,
    host_username TEXT NOT NULL,
    team_id TEXT NOT NULL,
    server_id TEXT NOT NULL,
    server_name TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'Active', -- Active, Paused, Ended, Recording
    share_link TEXT UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    ended_at TIMESTAMP,
    settings TEXT, -- JSON: { allow_observers, require_approval, record_session, enable_voice, enable_annotations, max_participants, allow_clipboard_sync }
    FOREIGN KEY (host_id) REFERENCES users(id),
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
);

-- Collaboration Participants
CREATE TABLE IF NOT EXISTS collaboration_participants (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'Observer', -- Observer, Operator, Admin
    joined_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_active_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_voice_active BOOLEAN DEFAULT FALSE,
    cursor_row INTEGER,
    cursor_col INTEGER,
    is_online BOOLEAN DEFAULT TRUE,
    FOREIGN KEY (session_id) REFERENCES collaboration_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id),
    UNIQUE(session_id, user_id)
);

-- Annotations (Screen drawings/markers)
CREATE TABLE IF NOT EXISTS collaboration_annotations (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    author_name TEXT NOT NULL,
    annotation_type TEXT NOT NULL, -- draw, highlight, arrow, text, circle, rectangle
    position TEXT NOT NULL, -- JSON: { x, y, width, height, points }
    content TEXT,
    color TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    resolved_at TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES collaboration_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (author_id) REFERENCES users(id)
);

-- Comments on specific lines
CREATE TABLE IF NOT EXISTS collaboration_comments (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    author_name TEXT NOT NULL,
    line_number INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP,
    resolved BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (session_id) REFERENCES collaboration_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (author_id) REFERENCES users(id)
);

-- Comment Replies
CREATE TABLE IF NOT EXISTS collaboration_comment_replies (
    id TEXT PRIMARY KEY,
    comment_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    author_name TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (comment_id) REFERENCES collaboration_comments(id) ON DELETE CASCADE,
    FOREIGN KEY (author_id) REFERENCES users(id)
);

-- Shared Clipboard
CREATE TABLE IF NOT EXISTS collaboration_clipboard (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    author_name TEXT NOT NULL,
    content TEXT NOT NULL,
    content_type TEXT NOT NULL DEFAULT 'text', -- text, code, url, command
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES collaboration_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (author_id) REFERENCES users(id)
);

-- Collaboration History (Audit trail)
CREATE TABLE IF NOT EXISTS collaboration_history (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    participant_id TEXT NOT NULL,
    participant_name TEXT NOT NULL,
    action_type TEXT NOT NULL, -- Join, Leave, ExecuteCommand, Input, RoleChange, VoiceStart, VoiceEnd, Annotate, Comment, ClipboardSync
    command TEXT,
    output_preview TEXT,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES collaboration_sessions(id) ON DELETE CASCADE
);

-- Session Recordings
CREATE TABLE IF NOT EXISTS collaboration_recordings (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    host_id TEXT NOT NULL,
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    ended_at TIMESTAMP,
    file_path TEXT,
    file_size INTEGER DEFAULT 0,
    total_events INTEGER DEFAULT 0,
    FOREIGN KEY (session_id) REFERENCES collaboration_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (host_id) REFERENCES users(id)
);

-- Recording Segments
CREATE TABLE IF NOT EXISTS collaboration_recording_segments (
    id TEXT PRIMARY KEY,
    recording_id TEXT NOT NULL,
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    ended_at TIMESTAMP,
    file_path TEXT NOT NULL,
    file_size INTEGER DEFAULT 0,
    events_count INTEGER DEFAULT 0,
    FOREIGN KEY (recording_id) REFERENCES collaboration_recordings(id) ON DELETE CASCADE
);

-- WebRTC Signaling (for voice calls)
CREATE TABLE IF NOT EXISTS collaboration_webrtc_signals (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    from_user_id TEXT NOT NULL,
    to_user_id TEXT NOT NULL,
    signal_type TEXT NOT NULL, -- offer, answer, ice_candidate
    data TEXT NOT NULL, -- JSON
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_processed BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (session_id) REFERENCES collaboration_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (from_user_id) REFERENCES users(id),
    FOREIGN KEY (to_user_id) REFERENCES users(id)
);

-- Indexes for collaboration tables
CREATE INDEX IF NOT EXISTS idx_collab_sessions_team_id ON collaboration_sessions(team_id);
CREATE INDEX IF NOT EXISTS idx_collab_sessions_share_link ON collaboration_sessions(share_link);
CREATE INDEX IF NOT EXISTS idx_collab_participants_session_id ON collaboration_participants(session_id);
CREATE INDEX IF NOT EXISTS idx_collab_participants_user_id ON collaboration_participants(user_id);
CREATE INDEX IF NOT EXISTS idx_collab_annotations_session_id ON collaboration_annotations(session_id);
CREATE INDEX IF NOT EXISTS idx_collab_comments_session_id ON collaboration_comments(session_id);
CREATE INDEX IF NOT EXISTS idx_collab_comments_line ON collaboration_comments(session_id, line_number);
CREATE INDEX IF NOT EXISTS idx_collab_clipboard_session_id ON collaboration_clipboard(session_id);
CREATE INDEX IF NOT EXISTS idx_collab_history_session_id ON collaboration_history(session_id);
CREATE INDEX IF NOT EXISTS idx_collab_history_timestamp ON collaboration_history(timestamp);
CREATE INDEX IF NOT EXISTS idx_collab_recordings_session_id ON collaboration_recordings(session_id);

-- Insert default permissions for collaboration
INSERT OR IGNORE INTO permissions (id, resource_type, action, description) VALUES
('perm_collab_create', 'collaboration', 'create', 'Create collaboration sessions'),
('perm_collab_join', 'collaboration', 'join', 'Join collaboration sessions'),
('perm_collab_manage', 'collaboration', 'manage', 'Manage collaboration settings'),
('perm_collab_record', 'collaboration', 'record', 'Record collaboration sessions'),
('perm_collab_annotate', 'collaboration', 'annotate', 'Add annotations to sessions'),
('perm_collab_comment', 'collaboration', 'comment', 'Add comments to sessions');

-- Assign collaboration permissions
INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES
('role_admin', 'perm_collab_create'),
('role_admin', 'perm_collab_join'),
('role_admin', 'perm_collab_manage'),
('role_admin', 'perm_collab_record'),
('role_admin', 'perm_collab_annotate'),
('role_admin', 'perm_collab_comment'),
('role_member', 'perm_collab_join'),
('role_member', 'perm_collab_annotate'),
('role_member', 'perm_collab_comment'),
('role_guest', 'perm_collab_join');
"#;
