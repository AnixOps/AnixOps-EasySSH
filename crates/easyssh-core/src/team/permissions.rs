//! Team permissions - Permission checking utilities

use super::types::{TeamMember, TeamRole};

/// Team permission
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamPermission {
    ViewServers,
    ManageServers,
    ManageMembers,
    ViewAudit,
    ManageTeam,
    DeleteTeam,
    InviteMembers,
    ShareResources,
}

/// Check if member has permission
pub fn check_permission(member: &TeamMember, permission: TeamPermission) -> bool {
    member.has_permission(permission)
}

/// Check role hierarchy
pub fn can_manage_role(manager_role: TeamRole, target_role: TeamRole) -> bool {
    manager_role.level() > target_role.level()
}

/// Get permissions for role
pub fn get_role_permissions(role: TeamRole) -> Vec<TeamPermission> {
    match role {
        TeamRole::Owner => vec![
            TeamPermission::ViewServers,
            TeamPermission::ManageServers,
            TeamPermission::ManageMembers,
            TeamPermission::ViewAudit,
            TeamPermission::ManageTeam,
            TeamPermission::DeleteTeam,
            TeamPermission::InviteMembers,
            TeamPermission::ShareResources,
        ],
        TeamRole::Admin => vec![
            TeamPermission::ViewServers,
            TeamPermission::ManageServers,
            TeamPermission::ManageMembers,
            TeamPermission::ViewAudit,
            TeamPermission::ManageTeam,
            TeamPermission::InviteMembers,
            TeamPermission::ShareResources,
        ],
        TeamRole::Member => vec![
            TeamPermission::ViewServers,
            TeamPermission::ManageServers,
            TeamPermission::ShareResources,
        ],
        TeamRole::Viewer => vec![TeamPermission::ViewServers],
    }
}
