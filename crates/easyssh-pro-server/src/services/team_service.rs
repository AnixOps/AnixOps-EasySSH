use crate::{models::*, redis_cache::RedisCache};
use anyhow::Result;
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

pub struct TeamService {
    db: Pool<Sqlite>,
    redis: std::sync::Arc<RedisCache>,
}

impl TeamService {
    pub fn new(db: Pool<Sqlite>, redis: std::sync::Arc<RedisCache>) -> Self {
        Self { db, redis }
    }

    pub async fn create_team(
        &self,
        created_by: &str,
        name: &str,
        description: Option<&str>,
        settings: Option<serde_json::Value>,
    ) -> Result<Team> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO teams (id, name, description, created_by, created_at, updated_at, is_active, settings) VALUES (?, ?, ?, ?, ?, ?, TRUE, ?)"
        )
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(created_by)
        .bind(now)
        .bind(now)
        .bind(settings)
        .execute(&self.db)
        .await?;

        // Add creator as owner
        let member_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO team_members (id, team_id, user_id, role, joined_at, is_active) VALUES (?, ?, ?, 'owner', ?, TRUE)"
        )
        .bind(&member_id)
        .bind(&id)
        .bind(created_by)
        .bind(now)
        .execute(&self.db)
        .await?;

        Ok(Team {
            id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            created_by: created_by.to_string(),
            created_at: now,
            updated_at: now,
            is_active: true,
            settings: None,
        })
    }

    pub async fn list_user_teams(
        &self,
        user_id: &str,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<(Vec<Team>, i64)> {
        let offset = ((page.unwrap_or(1) - 1) * limit.unwrap_or(20)) as i64;
        let limit = limit.unwrap_or(20) as i64;

        let teams = sqlx::query_as::<_, Team>(
            "SELECT t.* FROM teams t
             INNER JOIN team_members tm ON t.id = tm.team_id
             WHERE tm.user_id = ? AND tm.is_active = TRUE AND t.is_active = TRUE
             ORDER BY t.created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM teams t
             INNER JOIN team_members tm ON t.id = tm.team_id
             WHERE tm.user_id = ? AND tm.is_active = TRUE AND t.is_active = TRUE",
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok((teams, total))
    }

    pub async fn get_team(&self, team_id: &str) -> Result<Team> {
        let team =
            sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ? AND is_active = TRUE")
                .bind(team_id)
                .fetch_one(&self.db)
                .await?;

        Ok(team)
    }

    pub async fn update_team(
        &self,
        team_id: &str,
        name: Option<&str>,
        description: Option<&str>,
        settings: Option<serde_json::Value>,
    ) -> Result<Team> {
        let now = Utc::now();

        sqlx::query(
            "UPDATE teams SET name = COALESCE(?, name), description = COALESCE(?, description), settings = COALESCE(?, settings), updated_at = ? WHERE id = ?"
        )
        .bind(name)
        .bind(description)
        .bind(settings)
        .bind(now)
        .bind(team_id)
        .execute(&self.db)
        .await?;

        self.get_team(team_id).await
    }

    pub async fn delete_team(&self, team_id: &str) -> Result<()> {
        sqlx::query("UPDATE teams SET is_active = FALSE WHERE id = ?")
            .bind(team_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    pub async fn list_team_members(&self, team_id: &str) -> Result<Vec<TeamMember>> {
        let members = sqlx::query_as::<_, TeamMember>(
            "SELECT * FROM team_members WHERE team_id = ? AND is_active = TRUE",
        )
        .bind(team_id)
        .fetch_all(&self.db)
        .await?;

        Ok(members)
    }

    pub async fn invite_member(
        &self,
        team_id: &str,
        email: &str,
        role: TeamRole,
        invited_by: &str,
    ) -> Result<Invitation> {
        let id = Uuid::new_v4().to_string();
        let token = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::days(7);

        sqlx::query(
            "INSERT INTO invitations (id, team_id, email, role, invited_by, token, expires_at, created_at, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending')"
        )
        .bind(&id)
        .bind(team_id)
        .bind(email)
        .bind(role.to_string())
        .bind(invited_by)
        .bind(&token)
        .bind(expires_at)
        .bind(now)
        .execute(&self.db)
        .await?;

        Ok(Invitation {
            id,
            team_id: team_id.to_string(),
            email: email.to_string(),
            role,
            invited_by: invited_by.to_string(),
            token,
            expires_at,
            created_at: now,
            status: InvitationStatus::Pending,
            accepted_at: None,
            accepted_by: None,
        })
    }

    pub async fn remove_member(&self, team_id: &str, member_id: &str) -> Result<()> {
        sqlx::query("UPDATE team_members SET is_active = FALSE WHERE team_id = ? AND user_id = ?")
            .bind(team_id)
            .bind(member_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    pub async fn update_member_role(
        &self,
        team_id: &str,
        member_id: &str,
        role: TeamRole,
    ) -> Result<TeamMember> {
        sqlx::query("UPDATE team_members SET role = ? WHERE team_id = ? AND user_id = ?")
            .bind(role.to_string())
            .bind(team_id)
            .bind(member_id)
            .execute(&self.db)
            .await?;

        let member = sqlx::query_as::<_, TeamMember>(
            "SELECT * FROM team_members WHERE team_id = ? AND user_id = ?",
        )
        .bind(team_id)
        .bind(member_id)
        .fetch_one(&self.db)
        .await?;

        Ok(member)
    }

    pub async fn accept_invitation(&self, token: &str, user_id: &str) -> Result<TeamMember> {
        let invitation: Invitation = sqlx::query_as::<_, Invitation>(
            "SELECT * FROM invitations WHERE token = ? AND status = 'pending' AND expires_at > ?",
        )
        .bind(token)
        .bind(Utc::now())
        .fetch_one(&self.db)
        .await?;

        // Create team member
        let member_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO team_members (id, team_id, user_id, role, joined_at, invited_by, is_active) VALUES (?, ?, ?, ?, ?, ?, TRUE)"
        )
        .bind(&member_id)
        .bind(&invitation.team_id)
        .bind(user_id)
        .bind(invitation.role.to_string())
        .bind(now)
        .bind(&invitation.invited_by)
        .execute(&self.db)
        .await?;

        // Update invitation
        sqlx::query("UPDATE invitations SET status = 'accepted', accepted_at = ?, accepted_by = ? WHERE id = ?")
            .bind(now)
            .bind(user_id)
            .bind(&invitation.id)
            .execute(&self.db)
            .await?;

        let member = sqlx::query_as::<_, TeamMember>("SELECT * FROM team_members WHERE id = ?")
            .bind(member_id)
            .fetch_one(&self.db)
            .await?;

        Ok(member)
    }

    pub async fn decline_invitation(&self, token: &str, _user_id: &str) -> Result<()> {
        sqlx::query("UPDATE invitations SET status = 'cancelled' WHERE token = ?")
            .bind(token)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    pub async fn list_invitations(&self, team_id: &str) -> Result<Vec<Invitation>> {
        let invitations = sqlx::query_as::<_, Invitation>(
            "SELECT * FROM invitations WHERE team_id = ? ORDER BY created_at DESC",
        )
        .bind(team_id)
        .fetch_all(&self.db)
        .await?;

        Ok(invitations)
    }

    pub async fn cancel_invitation(&self, invitation_id: &str, _user_id: &str) -> Result<()> {
        sqlx::query("UPDATE invitations SET status = 'cancelled' WHERE id = ?")
            .bind(invitation_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    pub async fn is_team_member(&self, team_id: &str, user_id: &str) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = ? AND user_id = ? AND is_active = TRUE)"
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(exists)
    }

    pub async fn is_team_owner(&self, team_id: &str, user_id: &str) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = ? AND user_id = ? AND role = 'owner' AND is_active = TRUE)"
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(exists)
    }

    pub async fn has_team_permission(
        &self,
        team_id: &str,
        user_id: &str,
        _permission: &str,
    ) -> Result<bool> {
        // Simplified: owner or admin has all permissions
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = ? AND user_id = ? AND role IN ('owner', 'admin') AND is_active = TRUE)"
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(exists)
    }

    pub async fn get_team_settings(&self, team_id: &str) -> Result<serde_json::Value> {
        let settings: serde_json::Value =
            sqlx::query_scalar::<_, serde_json::Value>("SELECT settings FROM teams WHERE id = ?")
                .bind(team_id)
                .fetch_one(&self.db)
                .await?;

        Ok(settings)
    }

    pub async fn update_team_settings(
        &self,
        team_id: &str,
        settings: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let now = Utc::now();

        sqlx::query("UPDATE teams SET settings = ?, updated_at = ? WHERE id = ?")
            .bind(&settings)
            .bind(now)
            .bind(team_id)
            .execute(&self.db)
            .await?;

        Ok(settings)
    }
}
