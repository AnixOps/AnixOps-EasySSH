//! Team types - Data structures for team management

use crate::error::LiteError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Team role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

impl Default for TeamRole {
    fn default() -> Self {
        TeamRole::Viewer
    }
}

impl TeamRole {
    /// Get permission level (higher = more permissions)
    pub fn level(&self) -> u8 {
        match self {
            TeamRole::Owner => 100,
            TeamRole::Admin => 80,
            TeamRole::Member => 50,
            TeamRole::Viewer => 20,
        }
    }

    /// Check if can manage team
    pub fn can_manage_team(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// Check if can manage members
    pub fn can_manage_members(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// Check if can manage servers
    pub fn can_manage_servers(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin | TeamRole::Member)
    }

    /// Check if can view audit
    pub fn can_view_audit(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// Check if can manage settings
    pub fn can_manage_settings(&self) -> bool {
        matches!(self, TeamRole::Owner)
    }

    /// Check if can delete team
    pub fn can_delete_team(&self) -> bool {
        matches!(self, TeamRole::Owner)
    }

    /// Check if can invite members
    pub fn can_invite_members(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// Check if can share resources
    pub fn can_share_resources(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin | TeamRole::Member)
    }

    /// Check if can remove member
    pub fn can_remove_member(&self, target_role: &TeamRole) -> bool {
        self.level() > target_role.level()
    }

    /// Get display name (Chinese)
    pub fn display_name(&self) -> &'static str {
        match self {
            TeamRole::Owner => "所有者",
            TeamRole::Admin => "管理员",
            TeamRole::Member => "成员",
            TeamRole::Viewer => "观察者",
        }
    }
}

impl std::fmt::Display for TeamRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Team member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub role: TeamRole,
    pub status: MemberStatus,
    pub joined_at: DateTime<Utc>,
    pub last_active_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl TeamMember {
    /// Create new member
    pub fn new(team_id: &str, user_id: &str, username: &str, email: &str, role: TeamRole) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            team_id: team_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            email: email.to_string(),
            role,
            status: MemberStatus::Active,
            joined_at: Utc::now(),
            last_active_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if has permission
    pub fn has_permission(&self, permission: super::permissions::TeamPermission) -> bool {
        use super::permissions::TeamPermission;
        match permission {
            TeamPermission::ViewServers => true,
            TeamPermission::ManageServers => self.role.can_manage_servers(),
            TeamPermission::ManageMembers => self.role.can_manage_members(),
            TeamPermission::ViewAudit => self.role.can_view_audit(),
            TeamPermission::ManageTeam => self.role.can_manage_team(),
            TeamPermission::DeleteTeam => self.role.can_delete_team(),
            TeamPermission::InviteMembers => self.role.can_invite_members(),
            TeamPermission::ShareResources => self.role.can_share_resources(),
        }
    }

    /// Mark active
    pub fn mark_active(&mut self) {
        self.last_active_at = Some(Utc::now());
    }

    /// Suspend member
    pub fn suspend(&mut self) {
        self.status = MemberStatus::Suspended;
    }

    /// Activate member
    pub fn activate(&mut self) {
        self.status = MemberStatus::Active;
    }

    /// Check if active
    pub fn is_active(&self) -> bool {
        self.status == MemberStatus::Active
    }
}

/// Member status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberStatus {
    Active,
    Inactive,
    Suspended,
    Pending,
}

/// Shareable resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareableResourceType {
    Server,
    Snippet,
    Key,
    Layout,
    Config,
}

/// Shared resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedResource {
    pub id: String,
    pub team_id: String,
    pub resource_type: ShareableResourceType,
    pub resource_id: String,
    pub resource_name: String,
    pub shared_by: String,
    pub shared_at: DateTime<Utc>,
    pub share_type: ShareType,
    pub permissions: Vec<ResourceAccessPermission>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Share type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareType {
    Full,
    Selective,
    ReadOnly,
}

/// Resource access permission
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceAccessPermission {
    View,
    Edit,
    Execute,
    Delete,
    Share,
}

impl SharedResource {
    /// Create new shared resource
    pub fn new(
        team_id: &str,
        resource_type: ShareableResourceType,
        resource_id: &str,
        resource_name: &str,
        shared_by: &str,
        share_type: ShareType,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            team_id: team_id.to_string(),
            resource_type,
            resource_id: resource_id.to_string(),
            resource_name: resource_name.to_string(),
            shared_by: shared_by.to_string(),
            shared_at: Utc::now(),
            share_type,
            permissions: vec![ResourceAccessPermission::View],
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Set permissions
    pub fn with_permissions(mut self, permissions: Vec<ResourceAccessPermission>) -> Self {
        self.permissions = permissions;
        self
    }

    /// Set expiry
    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Check if expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|e| Utc::now() > e).unwrap_or(false)
    }

    /// Check if has permission
    pub fn has_permission(&self, permission: ResourceAccessPermission) -> bool {
        self.permissions.contains(&permission)
    }

    /// Check if user can access
    pub fn can_access(&self, user_id: &str, team_members: &[TeamMember]) -> bool {
        if self.is_expired() {
            return false;
        }

        let member = team_members.iter().find(|m| m.user_id == user_id);
        if member.is_none() {
            return false;
        }

        let member = member.unwrap();

        if member.role.can_manage_team() {
            return true;
        }

        if self.shared_by == user_id {
            return true;
        }

        match self.share_type {
            ShareType::Full => true,
            ShareType::Selective => self
                .metadata
                .get("allowed_members")
                .map(|allowed| allowed.split(',').any(|id| id == user_id))
                .unwrap_or(false),
            ShareType::ReadOnly => true,
        }
    }
}

/// Team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub settings: TeamSettings,
    #[serde(skip)]
    pub members: Vec<TeamMember>,
    #[serde(skip)]
    pub shared_resources: Vec<SharedResource>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Team {
    /// Create new team
    pub fn new(name: &str, owner_id: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: None,
            owner_id: owner_id.to_string(),
            created_at: now,
            updated_at: now,
            settings: TeamSettings::default(),
            members: Vec::new(),
            shared_resources: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// Set settings
    pub fn with_settings(mut self, settings: TeamSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Get active member count
    pub fn active_member_count(&self) -> usize {
        self.members.iter().filter(|m| m.is_active()).count()
    }

    /// Find member
    pub fn find_member(&self, user_id: &str) -> Option<&TeamMember> {
        self.members.iter().find(|m| m.user_id == user_id)
    }

    /// Find member (mutable)
    pub fn find_member_mut(&mut self, user_id: &str) -> Option<&mut TeamMember> {
        self.members.iter_mut().find(|m| m.user_id == user_id)
    }

    /// Get all admins
    pub fn get_admins(&self) -> Vec<&TeamMember> {
        self.members
            .iter()
            .filter(|m| m.role.can_manage_members())
            .collect()
    }

    /// Get all owners
    pub fn get_owners(&self) -> Vec<&TeamMember> {
        self.members
            .iter()
            .filter(|m| matches!(m.role, TeamRole::Owner))
            .collect()
    }

    /// Update settings
    pub fn update_settings(&mut self, settings: TeamSettings) {
        self.settings = settings;
        self.updated_at = Utc::now();
    }

    /// Check if at capacity
    pub fn is_at_capacity(&self) -> bool {
        self.settings
            .max_members
            .map(|max| self.members.len() >= max as usize)
            .unwrap_or(false)
    }

    /// Add shared resource
    pub fn add_shared_resource(&mut self, resource: SharedResource) {
        self.shared_resources.push(resource);
        self.updated_at = Utc::now();
    }

    /// Remove shared resource
    pub fn remove_shared_resource(&mut self, resource_id: &str) -> Option<SharedResource> {
        let pos = self
            .shared_resources
            .iter()
            .position(|r| r.id == resource_id)?;
        Some(self.shared_resources.remove(pos))
    }

    /// Find shared resource
    pub fn find_shared_resource(&self, resource_id: &str) -> Option<&SharedResource> {
        self.shared_resources.iter().find(|r| r.id == resource_id)
    }

    /// Cleanup expired shares
    pub fn cleanup_expired_shares(&mut self) -> Vec<SharedResource> {
        let expired: Vec<_> = self
            .shared_resources
            .iter()
            .filter(|r| r.is_expired())
            .cloned()
            .collect();

        self.shared_resources.retain(|r| !r.is_expired());

        if !expired.is_empty() {
            self.updated_at = Utc::now();
        }

        expired
    }

    /// Get accessible resources for user
    pub fn get_accessible_resources(&self, user_id: &str) -> Vec<&SharedResource> {
        self.shared_resources
            .iter()
            .filter(|r| r.can_access(user_id, &self.members))
            .collect()
    }

    /// Check if user can access resource
    pub fn can_access_resource(&self, user_id: &str, resource_id: &str) -> bool {
        self.find_shared_resource(resource_id)
            .map(|r| r.can_access(user_id, &self.members))
            .unwrap_or(false)
    }
}

/// Team settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamSettings {
    pub allow_invite_links: bool,
    pub default_role: TeamRole,
    pub require_approval: bool,
    pub max_members: Option<i32>,
    pub sso_enabled: bool,
    pub sso_provider_id: Option<String>,
    #[serde(default)]
    pub allow_guest_access: bool,
    #[serde(default)]
    pub require_2fa: bool,
    #[serde(default)]
    pub auto_expire_shares: Option<i64>,
    #[serde(default)]
    pub notification_settings: NotificationSettings,
}

/// Notification settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub member_join_email: bool,
    pub member_leave_email: bool,
    pub security_alert_email: bool,
    pub daily_digest_email: bool,
}

/// Team invite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInvite {
    pub id: String,
    pub team_id: String,
    pub email: String,
    pub role: TeamRole,
    pub invited_by: String,
    pub status: InviteStatus,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub invite_code: String,
    #[serde(default)]
    pub invite_link: Option<String>,
    #[serde(default)]
    pub custom_message: Option<String>,
}

/// Invite status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InviteStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
    Revoked,
}

impl TeamInvite {
    /// Create new invite
    pub fn new(
        team_id: &str,
        email: &str,
        role: TeamRole,
        invited_by: &str,
        expires_hours: i64,
    ) -> Self {
        let now = Utc::now();
        let invite_code = Self::generate_invite_code();
        Self {
            id: Uuid::new_v4().to_string(),
            team_id: team_id.to_string(),
            email: email.to_string(),
            role,
            invited_by: invited_by.to_string(),
            status: InviteStatus::Pending,
            expires_at: now + chrono::Duration::hours(expires_hours),
            created_at: now,
            accepted_at: None,
            invite_code,
            invite_link: None,
            custom_message: None,
        }
    }

    /// Generate invite code
    fn generate_invite_code() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let mut rng = rand::thread_rng();
        (0..8)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Set custom message
    pub fn with_message(mut self, message: &str) -> Self {
        self.custom_message = Some(message.to_string());
        self
    }

    /// Set invite link
    pub fn with_invite_link(mut self, base_url: &str) -> Self {
        self.invite_link = Some(format!(
            "{}/join/{}?code={}",
            base_url, self.team_id, self.invite_code
        ));
        self
    }

    /// Check if expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Accept invite
    pub fn accept(&mut self) {
        self.status = InviteStatus::Accepted;
        self.accepted_at = Some(Utc::now());
    }

    /// Decline invite
    pub fn decline(&mut self) {
        self.status = InviteStatus::Declined;
    }

    /// Revoke invite
    pub fn revoke(&mut self) {
        self.status = InviteStatus::Revoked;
    }

    /// Mark as expired
    pub fn mark_expired(&mut self) {
        self.status = InviteStatus::Expired;
    }
}

/// Team stats
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamStats {
    pub total_members: usize,
    pub active_members: usize,
    pub servers_count: usize,
    pub sessions_today: i64,
    pub pending_invites: usize,
    pub shared_resources: usize,
    pub last_activity_at: Option<DateTime<Utc>>,
}

/// Team activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamActivity {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub activity_type: ActivityType,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

/// Activity type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    MemberJoined,
    MemberLeft,
    MemberRemoved,
    RoleChanged,
    ServerAdded,
    ServerRemoved,
    ResourceShared,
    ResourceUnshared,
    SettingsChanged,
    InviteSent,
    InviteAccepted,
    InviteRevoked,
}

/// Cleanup result
#[derive(Debug, Clone)]
pub struct CleanupResult {
    pub expired_invites: Vec<TeamInvite>,
    pub expired_shares: Vec<SharedResource>,
}

/// Team operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamOperationResult {
    pub success: bool,
    pub message: String,
    pub team_id: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl TeamOperationResult {
    pub fn success(team_id: impl Into<String>) -> Self {
        Self {
            success: true,
            message: "Success".to_string(),
            team_id: Some(team_id.into()),
            data: None,
        }
    }

    pub fn success_with_data(team_id: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: "Success".to_string(),
            team_id: Some(team_id.into()),
            data: Some(data),
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            team_id: None,
            data: None,
        }
    }
}
