use crate::models::*;
use anyhow::Result;
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

pub struct RbacService {
    db: Pool<Sqlite>,
}

impl RbacService {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    pub async fn list_system_roles(
        &self,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<(Vec<Role>, i64)> {
        let offset = ((page.unwrap_or(1) - 1) * limit.unwrap_or(20)) as i64;
        let limit = limit.unwrap_or(20) as i64;

        let roles = sqlx::query_as::<_, Role>(
            "SELECT * FROM roles WHERE is_system = TRUE ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        let total =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM roles WHERE is_system = TRUE")
                .fetch_one(&self.db)
                .await?;

        Ok((roles, total))
    }

    pub async fn list_team_roles(&self, team_id: &str) -> Result<Vec<Role>> {
        let roles = sqlx::query_as::<_, Role>(
            "SELECT * FROM roles WHERE team_id = ? ORDER BY created_at DESC",
        )
        .bind(team_id)
        .fetch_all(&self.db)
        .await?;

        Ok(roles)
    }

    pub async fn create_system_role(&self, name: &str, description: Option<&str>) -> Result<Role> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO roles (id, name, description, is_system, created_at) VALUES (?, ?, ?, TRUE, ?)"
        )
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(now)
        .execute(&self.db)
        .await?;

        Ok(Role {
            id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            team_id: None,
            is_system: true,
            created_at: now,
            permissions: vec![],
        })
    }

    pub async fn create_team_role(
        &self,
        team_id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<Role> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO roles (id, name, description, team_id, is_system, created_at) VALUES (?, ?, ?, ?, FALSE, ?)"
        )
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(team_id)
        .bind(now)
        .execute(&self.db)
        .await?;

        Ok(Role {
            id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            team_id: Some(team_id.to_string()),
            is_system: false,
            created_at: now,
            permissions: vec![],
        })
    }

    pub async fn get_role(&self, role_id: &str) -> Result<Role> {
        let role = sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE id = ?")
            .bind(role_id)
            .fetch_one(&self.db)
            .await?;

        Ok(role)
    }

    pub async fn update_role(
        &self,
        role_id: &str,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<Role> {
        sqlx::query(
            "UPDATE roles SET name = COALESCE(?, name), description = COALESCE(?, description) WHERE id = ?"
        )
        .bind(name)
        .bind(description)
        .bind(role_id)
        .execute(&self.db)
        .await?;

        self.get_role(role_id).await
    }

    pub async fn delete_role(&self, role_id: &str) -> Result<()> {
        // Delete role permissions first
        sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
            .bind(role_id)
            .execute(&self.db)
            .await?;

        // Delete role
        sqlx::query("DELETE FROM roles WHERE id = ?")
            .bind(role_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    pub async fn list_permissions(&self) -> Result<Vec<Permission>> {
        let permissions = sqlx::query_as::<_, Permission>(
            "SELECT * FROM permissions ORDER BY resource_type, action",
        )
        .fetch_all(&self.db)
        .await?;

        Ok(permissions)
    }

    pub async fn get_role_permissions(&self, role_id: &str) -> Result<Vec<Permission>> {
        let permissions = sqlx::query_as::<_, Permission>(
            "SELECT p.* FROM permissions p
             INNER JOIN role_permissions rp ON p.id = rp.permission_id
             WHERE rp.role_id = ?",
        )
        .bind(role_id)
        .fetch_all(&self.db)
        .await?;

        Ok(permissions)
    }

    pub async fn add_permission_to_role(&self, role_id: &str, permission_id: &str) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)",
        )
        .bind(role_id)
        .bind(permission_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn remove_permission_from_role(
        &self,
        role_id: &str,
        permission_id: &str,
    ) -> Result<()> {
        sqlx::query("DELETE FROM role_permissions WHERE role_id = ? AND permission_id = ?")
            .bind(role_id)
            .bind(permission_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    pub async fn check_user_permission(
        &self,
        user_id: &str,
        team_id: Option<&str>,
        resource_type: &str,
        action: &str,
        _resource_id: Option<&str>,
    ) -> Result<bool> {
        // Check system roles
        let has_system_permission: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM role_permissions rp
                INNER JOIN roles r ON rp.role_id = r.id
                INNER JOIN team_members tm ON tm.user_id = ?
                WHERE r.is_system = TRUE
                AND rp.permission_id = (SELECT id FROM permissions WHERE resource_type = ? AND action = ?)
            )"
        )
        .bind(user_id)
        .bind(resource_type)
        .bind(action)
        .fetch_one(&self.db)
        .await?;

        if has_system_permission {
            return Ok(true);
        }

        // Check team-specific roles
        if let Some(tid) = team_id {
            let has_team_permission: bool = sqlx::query_scalar(
                "SELECT EXISTS(
                    SELECT 1 FROM role_permissions rp
                    INNER JOIN roles r ON rp.role_id = r.id
                    INNER JOIN team_members tm ON tm.team_id = r.team_id AND tm.user_id = ?
                    WHERE r.team_id = ?
                    AND rp.permission_id = (SELECT id FROM permissions WHERE resource_type = ? AND action = ?)
                )"
            )
            .bind(user_id)
            .bind(tid)
            .bind(resource_type)
            .bind(action)
            .fetch_one(&self.db)
            .await?;

            return Ok(has_team_permission);
        }

        Ok(false)
    }

    pub async fn get_user_permissions(
        &self,
        user_id: &str,
        team_id: Option<&str>,
    ) -> Result<Vec<String>> {
        let mut query = String::from(
            "SELECT DISTINCT p.resource_type || ':' || p.action as permission
             FROM role_permissions rp
             INNER JOIN roles r ON rp.role_id = r.id
             INNER JOIN permissions p ON rp.permission_id = p.id
             INNER JOIN team_members tm ON tm.user_id = ?",
        );

        query.push_str(" WHERE (r.is_system = TRUE OR r.team_id = tm.team_id)");

        if let Some(tid) = team_id {
            query.push_str(" AND tm.team_id = ?");

            let permissions: Vec<String> = sqlx::query_scalar(&query)
                .bind(user_id)
                .bind(tid)
                .fetch_all(&self.db)
                .await?;

            return Ok(permissions);
        }

        let permissions: Vec<String> = sqlx::query_scalar(&query)
            .bind(user_id)
            .fetch_all(&self.db)
            .await?;

        Ok(permissions)
    }

    pub async fn assign_role_to_member(
        &self,
        team_id: &str,
        user_id: &str,
        role_id: &str,
    ) -> Result<()> {
        // This would insert into a member_roles table if we had one
        // For now, we'll just update the team_members role field
        // In a full RBAC system, you'd have a separate table for member roles

        // Get the role name
        let role: Role = sqlx::query_as("SELECT * FROM roles WHERE id = ?")
            .bind(role_id)
            .fetch_one(&self.db)
            .await?;

        sqlx::query("UPDATE team_members SET role = ? WHERE team_id = ? AND user_id = ?")
            .bind(&role.name)
            .bind(team_id)
            .bind(user_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    pub async fn revoke_role_from_member(
        &self,
        team_id: &str,
        user_id: &str,
        _role_id: Option<&str>,
    ) -> Result<()> {
        // Reset to default member role
        sqlx::query("UPDATE team_members SET role = 'member' WHERE team_id = ? AND user_id = ?")
            .bind(team_id)
            .bind(user_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }
}
