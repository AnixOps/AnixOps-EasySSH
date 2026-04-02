//! Group repository
//!
//! This module provides CRUD operations for server group entities.

use crate::database::{
    error::{DatabaseError, Result},
    models::{Group, NewGroup, UpdateGroup},
};
use sqlx::SqlitePool;

/// Repository for group operations
#[derive(Debug, Clone)]
pub struct GroupRepository {
    pub(super) pool: SqlitePool,
}

impl GroupRepository {
    /// Create a new group repository
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new group
    ///
    /// # Arguments
    ///
    /// * `new_group` - The group data to create
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Validation fails
    /// - A group with the same ID already exists
    /// - A group with the same name already exists (unique constraint)
    pub async fn create(&self, new_group: &NewGroup) -> Result<()> {
        // Validate input
        new_group.validate()?;

        let color = new_group.color();

        sqlx::query(
            r#"
            INSERT INTO groups (id, name, color) VALUES (?, ?, ?)
            "#,
        )
        .bind(&new_group.id)
        .bind(&new_group.name)
        .bind(color)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a group by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The group ID
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NotFound` if the group doesn't exist.
    pub async fn get_by_id(&self, id: &str) -> Result<Group> {
        let group: Group = sqlx::query_as(
            r#"
            SELECT id, name, color, created_at FROM groups WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DatabaseError::NotFound {
                entity: "Group".to_string(),
                id: id.to_string(),
            },
            _ => e.into(),
        })?;

        Ok(group)
    }

    /// Get a group by name
    ///
    /// # Arguments
    ///
    /// * `name` - The group name (exact match, case-sensitive)
    pub async fn get_by_name(&self, name: &str) -> Result<Group> {
        let group: Group = sqlx::query_as(
            r#"
            SELECT id, name, color, created_at FROM groups WHERE name = ?
            "#,
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DatabaseError::NotFound {
                entity: "Group".to_string(),
                id: name.to_string(),
            },
            _ => e.into(),
        })?;

        Ok(group)
    }

    /// Get all groups
    ///
    /// Returns a list of all groups ordered by name.
    pub async fn get_all(&self) -> Result<Vec<Group>> {
        let groups: Vec<Group> = sqlx::query_as(
            r#"
            SELECT id, name, color, created_at FROM groups ORDER BY name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(groups)
    }

    /// Update a group
    ///
    /// Only updates fields that are specified (Some). Fields set to None
    /// retain their current values.
    pub async fn update(&self, update: &UpdateGroup) -> Result<()> {
        // Get current group to apply partial updates
        let current = self.get_by_id(&update.id).await?;

        let name = update.name.as_ref().unwrap_or(&current.name);
        let color = update.color.as_ref().unwrap_or(&current.color);

        sqlx::query(
            r#"
            UPDATE groups SET name = ?, color = ? WHERE id = ?
            "#,
        )
        .bind(name)
        .bind(color)
        .bind(&update.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a group
    ///
    /// # Arguments
    ///
    /// * `id` - The group ID to delete
    ///
    /// # Note
    ///
    /// Servers in this group will have their group_id set to NULL
    /// due to the ON DELETE SET NULL foreign key constraint.
    pub async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM groups WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(DatabaseError::NotFound {
                entity: "Group".to_string(),
                id: id.to_string(),
            });
        }

        Ok(())
    }

    /// Count total groups
    pub async fn count(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM groups")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    /// Count servers in a group
    pub async fn count_servers(&self, group_id: &str) -> Result<i64> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM servers WHERE group_id = ?")
                .bind(group_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(count.0)
    }

    /// Check if a group exists by ID
    pub async fn exists(&self, id: &str) -> Result<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM groups WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 > 0)
    }

    /// Check if a group name already exists
    pub async fn name_exists(&self, name: &str) -> Result<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM groups WHERE name = ?",
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 > 0)
    }

    /// Get groups with server counts
    ///
    /// Returns each group with the number of servers it contains.
    pub async fn get_all_with_counts(&self) -> Result<Vec<GroupWithCount>> {
        let groups: Vec<GroupWithCount> = sqlx::query_as(
            r#"
            SELECT
                g.id,
                g.name,
                g.color,
                g.created_at,
                COUNT(s.id) as server_count
            FROM groups g
            LEFT JOIN servers s ON g.id = s.group_id
            GROUP BY g.id, g.name, g.color, g.created_at
            ORDER BY g.name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(groups)
    }

    /// Rename a group
    ///
    /// Convenience method for updating just the name.
    pub async fn rename(&self, id: &str, new_name: &str) -> Result<()> {
        let update = UpdateGroup {
            id: id.to_string(),
            name: Some(new_name.to_string()),
            color: None,
        };
        self.update(&update).await
    }

    /// Change group color
    ///
    /// Convenience method for updating just the color.
    pub async fn set_color(&self, id: &str, new_color: &str) -> Result<()> {
        let update = UpdateGroup {
            id: id.to_string(),
            name: None,
            color: Some(new_color.to_string()),
        };
        self.update(&update).await
    }
}

/// Group with server count information
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GroupWithCount {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub server_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{Database, NewServer, ServerRepository};
    use tempfile::TempDir;

    async fn create_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::new(&db_path).await.unwrap();
        db.init().await.unwrap();

        (db, temp_dir)
    }

    #[tokio::test]
    async fn test_create_group() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: Some("#FF0000".to_string()),
        };
        repo.create(&group).await.unwrap();

        let found = repo.get_by_id("group1").await.unwrap();
        assert_eq!(found.name, "Production");
        assert_eq!(found.color, "#FF0000");
    }

    #[tokio::test]
    async fn test_create_group_default_color() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: None,
        };
        repo.create(&group).await.unwrap();

        let found = repo.get_by_id("group1").await.unwrap();
        assert_eq!(found.color, "#4A90D9"); // Default color
    }

    #[tokio::test]
    async fn test_create_duplicate_id_fails() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: None,
        };
        repo.create(&group).await.unwrap();

        let result = repo.create(&group).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_duplicate_name_fails() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        let group1 = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: None,
        };
        repo.create(&group1).await.unwrap();

        let group2 = NewGroup {
            id: "group2".to_string(),
            name: "Production".to_string(), // Same name
            color: None,
        };
        let result = repo.create(&group2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        let result = repo.get_by_id("nonexistent").await;
        assert!(matches!(result, Err(DatabaseError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_get_by_name() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: None,
        };
        repo.create(&group).await.unwrap();

        let found = repo.get_by_name("Production").await.unwrap();
        assert_eq!(found.id, "group1");
    }

    #[tokio::test]
    async fn test_get_all() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        let group1 = NewGroup {
            id: "group1".to_string(),
            name: "Alpha".to_string(),
            color: None,
        };
        let group2 = NewGroup {
            id: "group2".to_string(),
            name: "Beta".to_string(),
            color: None,
        };

        repo.create(&group1).await.unwrap();
        repo.create(&group2).await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "Alpha"); // Sorted by name
        assert_eq!(all[1].name, "Beta");
    }

    #[tokio::test]
    async fn test_update_group() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        let group = NewGroup {
            id: "group1".to_string(),
            name: "Old Name".to_string(),
            color: Some("#000000".to_string()),
        };
        repo.create(&group).await.unwrap();

        let update = UpdateGroup {
            id: "group1".to_string(),
            name: Some("New Name".to_string()),
            color: Some("#FFFFFF".to_string()),
        };
        repo.update(&update).await.unwrap();

        let found = repo.get_by_id("group1").await.unwrap();
        assert_eq!(found.name, "New Name");
        assert_eq!(found.color, "#FFFFFF");
    }

    #[tokio::test]
    async fn test_partial_update() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        let group = NewGroup {
            id: "group1".to_string(),
            name: "Original".to_string(),
            color: Some("#000000".to_string()),
        };
        repo.create(&group).await.unwrap();

        // Update only name
        let update = UpdateGroup {
            id: "group1".to_string(),
            name: Some("Updated".to_string()),
            color: None, // No change
        };
        repo.update(&update).await.unwrap();

        let found = repo.get_by_id("group1").await.unwrap();
        assert_eq!(found.name, "Updated");
        assert_eq!(found.color, "#000000"); // Unchanged
    }

    #[tokio::test]
    async fn test_delete_group() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: None,
        };
        repo.create(&group).await.unwrap();

        repo.delete("group1").await.unwrap();

        let result = repo.get_by_id("group1").await;
        assert!(matches!(result, Err(DatabaseError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_delete_nonexistent_fails() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        let result = repo.delete("nonexistent").await;
        assert!(matches!(result, Err(DatabaseError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_delete_group_unlinks_servers() {
        let (db, _temp) = create_test_db().await;
        let group_repo = db.group_repository();
        let server_repo = db.server_repository();

        // Create group
        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: None,
        };
        group_repo.create(&group).await.unwrap();

        // Create server in group
        let server = NewServer {
            id: "srv1".to_string(),
            name: "Test Server".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_method: "password".to_string(),
            encrypted_credentials: vec![1, 2, 3],
            group_id: Some("group1".to_string()),
        };
        server_repo.create(&server).await.unwrap();

        // Delete group
        group_repo.delete("group1").await.unwrap();

        // Server should now be ungrouped
        let server = server_repo.get_by_id("srv1").await.unwrap();
        assert!(server.group_id.is_none());
    }

    #[tokio::test]
    async fn test_count() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        assert_eq!(repo.count().await.unwrap(), 0);

        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: None,
        };
        repo.create(&group).await.unwrap();

        assert_eq!(repo.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_count_servers() {
        let (db, _temp) = create_test_db().await;
        let group_repo = db.group_repository();
        let server_repo = db.server_repository();

        // Create group
        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: None,
        };
        group_repo.create(&group).await.unwrap();

        // Initially 0 servers
        assert_eq!(group_repo.count_servers("group1").await.unwrap(), 0);

        // Add servers
        for i in 0..3 {
            let server = NewServer {
                id: format!("srv{}", i),
                name: format!("Server {}", i),
                host: format!("192.168.1.{}", i),
                port: 22,
                username: "admin".to_string(),
                auth_method: "password".to_string(),
                encrypted_credentials: vec![1, 2, 3],
                group_id: Some("group1".to_string()),
            };
            server_repo.create(&server).await.unwrap();
        }

        assert_eq!(group_repo.count_servers("group1").await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_exists() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        assert!(!repo.exists("group1").await.unwrap());

        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: None,
        };
        repo.create(&group).await.unwrap();

        assert!(repo.exists("group1").await.unwrap());
    }

    #[tokio::test]
    async fn test_name_exists() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        assert!(!repo.name_exists("Production").await.unwrap());

        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: None,
        };
        repo.create(&group).await.unwrap();

        assert!(repo.name_exists("Production").await.unwrap());
    }

    #[tokio::test]
    async fn test_get_all_with_counts() {
        let (db, _temp) = create_test_db().await;
        let group_repo = db.group_repository();
        let server_repo = db.server_repository();

        // Create groups
        let group1 = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: None,
        };
        let group2 = NewGroup {
            id: "group2".to_string(),
            name: "Development".to_string(),
            color: None,
        };
        group_repo.create(&group1).await.unwrap();
        group_repo.create(&group2).await.unwrap();

        // Add servers to production only
        for i in 0..3 {
            let server = NewServer {
                id: format!("srv{}", i),
                name: format!("Server {}", i),
                host: format!("192.168.1.{}", i),
                port: 22,
                username: "admin".to_string(),
                auth_method: "password".to_string(),
                encrypted_credentials: vec![1, 2, 3],
                group_id: Some("group1".to_string()),
            };
            server_repo.create(&server).await.unwrap();
        }

        let groups_with_counts = group_repo.get_all_with_counts().await.unwrap();
        assert_eq!(groups_with_counts.len(), 2);

        let prod = groups_with_counts.iter().find(|g| g.id == "group1").unwrap();
        let dev = groups_with_counts.iter().find(|g| g.id == "group2").unwrap();

        assert_eq!(prod.server_count, 3);
        assert_eq!(dev.server_count, 0);
    }

    #[tokio::test]
    async fn test_rename() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        let group = NewGroup {
            id: "group1".to_string(),
            name: "Old Name".to_string(),
            color: None,
        };
        repo.create(&group).await.unwrap();

        repo.rename("group1", "New Name").await.unwrap();

        let found = repo.get_by_id("group1").await.unwrap();
        assert_eq!(found.name, "New Name");
    }

    #[tokio::test]
    async fn test_set_color() {
        let (db, _temp) = create_test_db().await;
        let repo = db.group_repository();

        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: Some("#000000".to_string()),
        };
        repo.create(&group).await.unwrap();

        repo.set_color("group1", "#FFFFFF").await.unwrap();

        let found = repo.get_by_id("group1").await.unwrap();
        assert_eq!(found.color, "#FFFFFF");
    }
}
