//! Team manager - Team management implementation

#[cfg(feature = "audit")]
use crate::audit::{Actor, AuditAction, AuditEntry, AuditLogger, AuditResult, AuditTarget};
use crate::error::LiteError;
use chrono::Utc;
use std::collections::HashMap;

use super::permissions::TeamPermission;
use super::types::{
    ActivityType, CleanupResult, InviteStatus, MemberStatus, ShareType, ShareableResourceType,
    SharedResource, Team, TeamActivity, TeamInvite, TeamMember, TeamRole, TeamStats,
};

/// Team manager
pub struct TeamManager {
    teams: HashMap<String, Team>,
    invites: HashMap<String, TeamInvite>,
    user_teams: HashMap<String, Vec<String>>,
    activities: Vec<TeamActivity>,
    #[cfg(feature = "audit")]
    audit_logger: Option<AuditLogger>,
}

impl TeamManager {
    /// Create new team manager
    pub fn new() -> Self {
        Self {
            teams: HashMap::new(),
            invites: HashMap::new(),
            user_teams: HashMap::new(),
            activities: Vec::new(),
            #[cfg(feature = "audit")]
            audit_logger: None,
        }
    }

    /// Enable audit logging
    #[cfg(feature = "audit")]
    pub fn with_audit(mut self, audit: AuditLogger) -> Self {
        self.audit_logger = Some(audit);
        self
    }

    /// Create team
    pub fn create_team(
        &mut self,
        name: &str,
        owner_id: &str,
        owner_name: Option<&str>,
    ) -> Result<Team, LiteError> {
        let mut team = Team::new(name, owner_id);
        let id = team.id.clone();

        // Add owner as member
        let owner_member = TeamMember::new(
            &id,
            owner_id,
            owner_name.unwrap_or(owner_id),
            "",
            TeamRole::Owner,
        );
        team.members.push(owner_member);

        self.teams.insert(id.clone(), team.clone());
        self.add_user_to_team(owner_id, &id);

        // Record activity
        self.record_activity(&id, owner_id, ActivityType::MemberJoined, "Created team");

        // Log audit
        #[cfg(feature = "audit")]
        self.log_audit(
            AuditAction::TeamCreate,
            owner_id,
            &id,
            AuditTarget::Team { id: id.clone() },
        );

        Ok(team)
    }

    /// Get team
    pub fn get_team(&self, team_id: &str) -> Option<&Team> {
        self.teams.get(team_id)
    }

    /// Get team (mutable)
    pub fn get_team_mut(&mut self, team_id: &str) -> Option<&mut Team> {
        self.teams.get_mut(team_id)
    }

    /// Update team
    pub fn update_team(&mut self, team: Team, updated_by: &str) -> Result<(), LiteError> {
        if self.teams.contains_key(&team.id) {
            let team_id = team.id.clone();
            self.teams.insert(team_id.clone(), team);

            #[cfg(feature = "audit")]
            self.log_audit(
                AuditAction::TeamUpdate,
                updated_by,
                &team_id,
                AuditTarget::Team {
                    id: team_id.clone(),
                },
            );

            Ok(())
        } else {
            Err(LiteError::Team(format!("Team {} not found", team.id)))
        }
    }

    /// Delete team
    pub fn delete_team(&mut self, team_id: &str, requester_id: &str) -> Result<(), LiteError> {
        let team = self
            .teams
            .get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        if team.owner_id != requester_id {
            return Err(LiteError::Team("Only owner can delete team".to_string()));
        }

        let member_ids: Vec<String> = team.members.iter().map(|m| m.user_id.clone()).collect();
        let team_id_owned = team_id.to_string();
        self.teams.remove(team_id);

        for user_id in member_ids {
            self.remove_user_from_team(&user_id, &team_id_owned);
        }

        // Cleanup invites
        self.invites
            .retain(|_, invite| invite.team_id != team_id_owned);

        #[cfg(feature = "audit")]
        self.log_audit(
            AuditAction::TeamDelete,
            requester_id,
            &team_id_owned,
            AuditTarget::Team {
                id: team_id_owned.clone(),
            },
        );

        Ok(())
    }

    /// List user teams
    pub fn list_user_teams(&self, user_id: &str) -> Vec<&Team> {
        self.user_teams
            .get(user_id)
            .map(|team_ids| {
                team_ids
                    .iter()
                    .filter_map(|id| self.teams.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// List all teams
    pub fn list_all_teams(&self) -> Vec<&Team> {
        self.teams.values().collect()
    }

    /// Invite member
    pub fn invite_member(
        &mut self,
        team_id: &str,
        email: &str,
        role: TeamRole,
        invited_by: &str,
        custom_message: Option<&str>,
    ) -> Result<TeamInvite, LiteError> {
        let team = self
            .teams
            .get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        // Check permission
        if let Some(member) = team.find_member(invited_by) {
            if !member.role.can_invite_members() {
                return Err(LiteError::Team(
                    "No permission to invite members".to_string(),
                ));
            }
        } else {
            return Err(LiteError::Team("Not a team member".to_string()));
        }

        // Check capacity
        if team.is_at_capacity() {
            return Err(LiteError::Team("Team is at capacity".to_string()));
        }

        // Revoke existing pending invites for same email
        let existing_ids: Vec<String> = self
            .invites
            .values()
            .filter(|i| {
                i.team_id == team_id && i.email == email && i.status == InviteStatus::Pending
            })
            .map(|i| i.id.clone())
            .collect();

        for id in existing_ids {
            if let Some(invite) = self.invites.get_mut(&id) {
                invite.revoke();
            }
        }

        // Check if already member
        if team.members.iter().any(|m| m.email == email) {
            return Err(LiteError::Team("Already a member".to_string()));
        }

        let mut invite = TeamInvite::new(team_id, email, role, invited_by, 168); // 7 days

        if let Some(msg) = custom_message {
            invite.custom_message = Some(msg.to_string());
        }

        invite.invite_link = Some(format!("easyssh://join/{}/{}", team_id, invite.invite_code));

        let id = invite.id.clone();
        self.invites.insert(id.clone(), invite.clone());

        self.record_activity(
            team_id,
            invited_by,
            ActivityType::InviteSent,
            &format!("Invited {}", email),
        );

        #[cfg(feature = "audit")]
        self.log_audit(
            AuditAction::MemberInvite,
            invited_by,
            team_id,
            AuditTarget::User {
                id: email.to_string(),
            },
        );

        Ok(invite)
    }

    /// Join by invite code
    pub fn join_by_invite_code(
        &mut self,
        invite_code: &str,
        user_id: &str,
        username: &str,
        email: &str,
    ) -> Result<TeamMember, LiteError> {
        let invite = self
            .invites
            .values_mut()
            .find(|i| i.invite_code == invite_code && i.status == InviteStatus::Pending)
            .ok_or_else(|| LiteError::Team("Invalid or expired invite code".to_string()))?;

        if invite.is_expired() {
            invite.mark_expired();
            return Err(LiteError::Team("Invite has expired".to_string()));
        }

        let team_id = invite.team_id.clone();
        let role = invite.role;

        invite.accept();

        let member = self.add_member_internal(&team_id, user_id, username, email, role)?;

        self.record_activity(
            &team_id,
            user_id,
            ActivityType::InviteAccepted,
            "Joined via invite code",
        );

        Ok(member)
    }

    /// Accept invite
    pub fn accept_invite(
        &mut self,
        invite_id: &str,
        user_id: &str,
        username: &str,
        email: &str,
    ) -> Result<TeamMember, LiteError> {
        let invite = self
            .invites
            .get_mut(invite_id)
            .ok_or_else(|| LiteError::Team(format!("Invite {} not found", invite_id)))?;

        if invite.status != InviteStatus::Pending {
            return Err(LiteError::Team("Invite is not pending".to_string()));
        }

        if invite.is_expired() {
            invite.mark_expired();
            return Err(LiteError::Team("Invite has expired".to_string()));
        }

        if invite.email != email {
            return Err(LiteError::Team(
                "Email does not match invitation".to_string(),
            ));
        }

        let team_id = invite.team_id.clone();
        let role = invite.role;

        invite.accept();

        let member = self.add_member_internal(&team_id, user_id, username, email, role)?;

        #[cfg(feature = "audit")]
        self.log_audit(
            AuditAction::MemberJoin,
            user_id,
            &team_id,
            AuditTarget::Team {
                id: team_id.clone(),
            },
        );

        Ok(member)
    }

    /// Add member (internal)
    fn add_member_internal(
        &mut self,
        team_id: &str,
        user_id: &str,
        username: &str,
        email: &str,
        role: TeamRole,
    ) -> Result<TeamMember, LiteError> {
        let team = self
            .teams
            .get_mut(team_id)
            .ok_or_else(|| LiteError::Team("Team not found".to_string()))?;

        if team.is_at_capacity() {
            return Err(LiteError::Team("Team is at capacity".to_string()));
        }

        if team.find_member(user_id).is_some() {
            return Err(LiteError::Team("Already a team member".to_string()));
        }

        let member = TeamMember::new(team_id, user_id, username, email, role);

        team.members.push(member.clone());
        team.updated_at = Utc::now();

        self.add_user_to_team(user_id, team_id);

        self.record_activity(
            team_id,
            user_id,
            ActivityType::MemberJoined,
            &format!("{} joined the team", username),
        );

        Ok(member)
    }

    /// Remove member
    pub fn remove_member(
        &mut self,
        team_id: &str,
        member_user_id: &str,
        removed_by: &str,
    ) -> Result<(), LiteError> {
        let team = self
            .teams
            .get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        let remover_role = team.find_member(removed_by).map(|m| m.role);
        let target_role = team.find_member(member_user_id).map(|m| m.role);

        match (remover_role, target_role) {
            (Some(remover), Some(target)) => {
                if !remover.can_remove_member(&target) {
                    return Err(LiteError::Team(
                        "Cannot remove member with higher or equal role".to_string(),
                    ));
                }
            }
            _ => return Err(LiteError::Team("Member not found".to_string())),
        }

        if team.owner_id == member_user_id {
            return Err(LiteError::Team("Cannot remove team owner".to_string()));
        }

        let team = self.teams.get_mut(team_id).unwrap();
        let member_name = team
            .find_member(member_user_id)
            .map(|m| m.username.clone())
            .unwrap_or_default();
        team.members.retain(|m| m.user_id != member_user_id);
        team.updated_at = Utc::now();

        self.remove_user_from_team(member_user_id, team_id);

        self.record_activity(
            team_id,
            removed_by,
            ActivityType::MemberRemoved,
            &format!("Removed member {}", member_name),
        );

        #[cfg(feature = "audit")]
        self.log_audit(
            AuditAction::MemberRemove,
            removed_by,
            team_id,
            AuditTarget::User {
                id: member_user_id.to_string(),
            },
        );

        Ok(())
    }

    /// Change member role
    pub fn change_member_role(
        &mut self,
        team_id: &str,
        member_user_id: &str,
        new_role: TeamRole,
        changed_by: &str,
    ) -> Result<(), LiteError> {
        let team = self
            .teams
            .get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        if let Some(changer) = team.find_member(changed_by) {
            if !changer.role.can_manage_members() {
                return Err(LiteError::Team("No permission to change roles".to_string()));
            }
            if changer.role.level() <= new_role.level() {
                return Err(LiteError::Team(
                    "Cannot assign role higher or equal to yours".to_string(),
                ));
            }
        } else {
            return Err(LiteError::Team("Not a team member".to_string()));
        }

        if team.owner_id == member_user_id {
            return Err(LiteError::Team("Cannot change owner role".to_string()));
        }

        let team = self.teams.get_mut(team_id).unwrap();
        let member_info = team.find_member_mut(member_user_id).map(|member| {
            let old_role = member.role;
            member.role = new_role;
            (member.username.clone(), old_role)
        });

        if let Some((username, old_role)) = member_info {
            team.updated_at = Utc::now();

            self.record_activity(
                team_id,
                changed_by,
                ActivityType::RoleChanged,
                &format!(
                    "Changed {}'s role from {:?} to {:?}",
                    username, old_role, new_role
                ),
            );
        }

        Ok(())
    }

    /// Get pending invites for team
    pub fn get_pending_invites(&self, team_id: &str) -> Vec<&TeamInvite> {
        self.invites
            .values()
            .filter(|i| i.team_id == team_id && i.status == InviteStatus::Pending)
            .collect()
    }

    /// Share resource
    pub fn share_resource(
        &mut self,
        team_id: &str,
        shared_by: &str,
        resource_type: ShareableResourceType,
        resource_id: &str,
        resource_name: &str,
        share_type: ShareType,
        permissions: Vec<super::types::ResourceAccessPermission>,
    ) -> Result<SharedResource, LiteError> {
        let team = self
            .teams
            .get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        if let Some(member) = team.find_member(shared_by) {
            if !member.role.can_share_resources() {
                return Err(LiteError::Team(
                    "No permission to share resources".to_string(),
                ));
            }
        } else {
            return Err(LiteError::Team("Not a team member".to_string()));
        }

        let resource = SharedResource::new(
            team_id,
            resource_type,
            resource_id,
            resource_name,
            shared_by,
            share_type,
        )
        .with_permissions(permissions);

        let team = self.teams.get_mut(team_id).unwrap();
        team.add_shared_resource(resource.clone());

        self.record_activity(
            team_id,
            shared_by,
            ActivityType::ResourceShared,
            &format!("Shared {:?}: {}", resource_type, resource_name),
        );

        Ok(resource)
    }

    /// Get team stats
    pub fn get_team_stats(&self, team_id: &str) -> Option<TeamStats> {
        let team = self.teams.get(team_id)?;

        let active_members = team
            .members
            .iter()
            .filter(|m| m.status == MemberStatus::Active)
            .count();

        let pending_invites = self.get_pending_invites(team_id).len();
        let last_activity = team.members.iter().filter_map(|m| m.last_active_at).max();

        Some(TeamStats {
            total_members: team.members.len(),
            active_members,
            servers_count: team
                .shared_resources
                .iter()
                .filter(|r| matches!(r.resource_type, ShareableResourceType::Server))
                .count(),
            sessions_today: 0,
            pending_invites,
            shared_resources: team.shared_resources.len(),
            last_activity_at: last_activity,
        })
    }

    /// Check if user is team member
    pub fn is_team_member(&self, team_id: &str, user_id: &str) -> bool {
        self.teams
            .get(team_id)
            .map(|t| t.find_member(user_id).is_some())
            .unwrap_or(false)
    }

    /// Get member role
    pub fn get_member_role(&self, team_id: &str, user_id: &str) -> Option<TeamRole> {
        self.teams
            .get(team_id)
            .and_then(|t| t.find_member(user_id).map(|m| m.role))
    }

    /// Check permission
    pub fn check_permission(
        &self,
        team_id: &str,
        user_id: &str,
        permission: TeamPermission,
    ) -> bool {
        self.teams
            .get(team_id)
            .and_then(|t| t.find_member(user_id))
            .map(|m| m.has_permission(permission))
            .unwrap_or(false)
    }

    /// Cleanup expired invites
    pub fn cleanup_expired_invites(&mut self) -> Vec<TeamInvite> {
        let expired: Vec<_> = self
            .invites
            .values_mut()
            .filter(|i| i.status == InviteStatus::Pending && i.is_expired())
            .map(|i| {
                i.mark_expired();
                i.clone()
            })
            .collect();

        expired
    }

    /// Cleanup all expired
    pub fn cleanup_all_expired(&mut self) -> CleanupResult {
        let expired_invites = self.cleanup_expired_invites();

        let mut expired_shares = Vec::new();
        for team in self.teams.values_mut() {
            let expired = team.cleanup_expired_shares();
            expired_shares.extend(expired);
        }

        CleanupResult {
            expired_invites,
            expired_shares,
        }
    }

    /// Record activity
    fn record_activity(
        &mut self,
        team_id: &str,
        user_id: &str,
        activity_type: ActivityType,
        description: &str,
    ) {
        let activity = TeamActivity {
            id: uuid::Uuid::new_v4().to_string(),
            team_id: team_id.to_string(),
            user_id: user_id.to_string(),
            activity_type,
            description: description.to_string(),
            created_at: Utc::now(),
            metadata: None,
        };
        self.activities.push(activity);
    }

    /// Get team activities
    pub fn get_team_activities(&self, team_id: &str, limit: usize) -> Vec<&TeamActivity> {
        self.activities
            .iter()
            .filter(|a| a.team_id == team_id)
            .rev()
            .take(limit)
            .collect()
    }

    /// Log audit (internal)
    #[cfg(feature = "audit")]
    fn log_audit(
        &mut self,
        action: AuditAction,
        actor_id: &str,
        team_id: &str,
        target: AuditTarget,
    ) {
        if let Some(ref mut audit) = self.audit_logger {
            let actor = Actor {
                user_id: actor_id.to_string(),
                username: actor_id.to_string(),
                team_id: Some(team_id.to_string()),
                role: None,
            };

            let entry = AuditEntry::new(action, actor, target).with_result(AuditResult::Success);

            audit.log(entry);
        }
    }

    /// Add user to team mapping
    fn add_user_to_team(&mut self, user_id: &str, team_id: &str) {
        self.user_teams
            .entry(user_id.to_string())
            .or_default()
            .push(team_id.to_string());
    }

    /// Remove user from team mapping
    fn remove_user_from_team(&mut self, user_id: &str, team_id: &str) {
        if let Some(teams) = self.user_teams.get_mut(user_id) {
            teams.retain(|id| id != team_id);
        }
    }
}

impl Default for TeamManager {
    fn default() -> Self {
        Self::new()
    }
}
