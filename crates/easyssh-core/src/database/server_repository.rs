//! Server repository
//!
//! This module provides CRUD operations for server entities,
//! including filtering, pagination, and group association.

use crate::database::{
    error::{DatabaseError, Result},
    models::{NewServer, Server, ServerFilters, ServerWithGroup, UpdateServer},
};
use sqlx::SqlitePool;

/// Repository for server operations
#[derive(Debug, Clone)]
pub struct ServerRepository {
    pub(super) pool: SqlitePool,
}

impl ServerRepository {
    /// Create a new server repository
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new server
    ///
    /// # Arguments
    ///
    /// * `new_server` - The server data to create
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Validation fails
    /// - A server with the same ID already exists
    /// - The referenced group doesn't exist
    pub async fn create(&self, new_server: &NewServer) -> Result<()> {
        // Validate input
        new_server.validate()?;

        sqlx::query(
            r#"
            INSERT INTO servers (
                id, name, host, port, username, auth_method,
                encrypted_credentials, group_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&new_server.id)
        .bind(&new_server.name)
        .bind(&new_server.host)
        .bind(new_server.port as i64)
        .bind(&new_server.username)
        .bind(&new_server.auth_method)
        .bind(&new_server.encrypted_credentials)
        .bind(&new_server.group_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a server by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The server ID
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NotFound` if the server doesn't exist.
    pub async fn get_by_id(&self, id: &str) -> Result<Server> {
        let server: Server = sqlx::query_as(
            r#"
            SELECT id, name, host, port, username, auth_method,
                   encrypted_credentials, group_id, created_at, updated_at
            FROM servers WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DatabaseError::NotFound {
                entity: "Server".to_string(),
                id: id.to_string(),
            },
            _ => e.into(),
        })?;

        Ok(server)
    }

    /// Get a server with its group information
    ///
    /// Returns the server along with group name and color if associated.
    pub async fn get_with_group(&self, id: &str) -> Result<ServerWithGroup> {
        let server: ServerWithGroup = sqlx::query_as(
            r#"
            SELECT
                s.id, s.name, s.host, s.port, s.username, s.auth_method,
                s.encrypted_credentials, s.created_at as server_created_at,
                s.updated_at as server_updated_at,
                g.id as group_id, g.name as group_name, g.color as group_color
            FROM servers s
            LEFT JOIN groups g ON s.group_id = g.id
            WHERE s.id = ?
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DatabaseError::NotFound {
                entity: "Server".to_string(),
                id: id.to_string(),
            },
            _ => e.into(),
        })?;

        Ok(server)
    }

    /// Get all servers
    ///
    /// Returns a list of all servers ordered by creation date (newest first).
    pub async fn get_all(&self) -> Result<Vec<Server>> {
        let servers: Vec<Server> = sqlx::query_as(
            r#"
            SELECT id, name, host, port, username, auth_method,
                   encrypted_credentials, group_id, created_at, updated_at
            FROM servers
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(servers)
    }

    /// Get servers with filtering
    ///
    /// # Arguments
    ///
    /// * `filters` - Optional filters to apply
    pub async fn get_filtered(&self, filters: Option<ServerFilters>) -> Result<Vec<Server>> {
        let mut query = String::from(
            r#"
            SELECT id, name, host, port, username, auth_method,
                   encrypted_credentials, group_id, created_at, updated_at
            FROM servers
            WHERE 1=1
            "#,
        );

        let mut group_id: Option<String> = None;
        let mut search: Option<String> = None;

        if let Some(f) = filters {
            if f.group_id.is_some() {
                query.push_str(" AND group_id = ?");
                group_id = f.group_id;
            }
            if f.search.is_some() {
                query.push_str(" AND (name LIKE ? OR host LIKE ?)");
                search = f.search.map(|s| format!("%{}%", s));
            }
        }

        query.push_str(" ORDER BY created_at DESC");

        let mut q = sqlx::query_as(&query);

        if let Some(gid) = group_id {
            q = q.bind(gid);
        }
        if let Some(s) = search.as_ref() {
            q = q.bind(s).bind(s);
        }

        let servers: Vec<Server> = q.fetch_all(&self.pool).await?;

        Ok(servers)
    }

    /// Get servers by group ID
    pub async fn get_by_group(&self, group_id: &str) -> Result<Vec<Server>> {
        let servers: Vec<Server> = sqlx::query_as(
            r#"
            SELECT id, name, host, port, username, auth_method,
                   encrypted_credentials, group_id, created_at, updated_at
            FROM servers WHERE group_id = ?
            ORDER BY name ASC
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(servers)
    }

    /// Get servers without a group (ungrouped)
    pub async fn get_ungrouped(&self) -> Result<Vec<Server>> {
        let servers: Vec<Server> = sqlx::query_as(
            r#"
            SELECT id, name, host, port, username, auth_method,
                   encrypted_credentials, group_id, created_at, updated_at
            FROM servers WHERE group_id IS NULL
            ORDER BY name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(servers)
    }

    /// Update a server
    ///
    /// Only updates fields that are specified (Some). Fields set to None
    /// retain their current values.
    pub async fn update(&self, update: &UpdateServer) -> Result<()> {
        // Get current server to apply partial updates
        let current = self.get_by_id(&update.id).await?;

        let name = update.name.as_ref().unwrap_or(&current.name);
        let host = update.host.as_ref().unwrap_or(&current.host);
        let port = update.port.unwrap_or(current.port as u16) as i64;
        let username = update.username.as_ref().unwrap_or(&current.username);
        let auth_method = update.auth_method.as_ref().unwrap_or(&current.auth_method);
        let encrypted_credentials = update
            .encrypted_credentials
            .as_ref()
            .unwrap_or(&current.encrypted_credentials);
        let group_id = match &update.group_id {
            Some(g) => g.as_deref(),
            None => current.group_id.as_deref(),
        };

        sqlx::query(
            r#"
            UPDATE servers SET
                name = ?,
                host = ?,
                port = ?,
                username = ?,
                auth_method = ?,
                encrypted_credentials = ?,
                group_id = ?,
                updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(name)
        .bind(host)
        .bind(port)
        .bind(username)
        .bind(auth_method)
        .bind(encrypted_credentials)
        .bind(group_id)
        .bind(&update.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a server
    ///
    /// # Arguments
    ///
    /// * `id` - The server ID to delete
    pub async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM servers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(DatabaseError::NotFound {
                entity: "Server".to_string(),
                id: id.to_string(),
            });
        }

        Ok(())
    }

    /// Count total servers
    pub async fn count(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM servers")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    /// Count servers in a group
    pub async fn count_by_group(&self, group_id: &str) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM servers WHERE group_id = ?")
            .bind(group_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    /// Check if a server exists
    pub async fn exists(&self, id: &str) -> Result<bool> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM servers WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0 > 0)
    }

    /// Move servers from one group to another
    ///
    /// If `new_group_id` is None, servers will be ungrouped.
    pub async fn move_to_group(
        &self,
        server_ids: &[String],
        new_group_id: Option<&str>,
    ) -> Result<u64> {
        if server_ids.is_empty() {
            return Ok(0);
        }

        // Build parameterized query
        let placeholders: Vec<String> = server_ids.iter().map(|_| "?".to_string()).collect();
        let placeholders_str = placeholders.join(",");

        let query = format!(
            "UPDATE servers SET group_id = ?, updated_at = datetime('now') WHERE id IN ({})",
            placeholders_str
        );

        let mut q = sqlx::query(&query).bind(new_group_id);
        for id in server_ids {
            q = q.bind(id);
        }

        let result = q.execute(&self.pool).await?;

        Ok(result.rows_affected())
    }

    /// Search servers by name or host
    ///
    /// Performs a case-insensitive search.
    pub async fn search(&self, query: &str) -> Result<Vec<Server>> {
        let pattern = format!("%{}%", query);

        let servers: Vec<Server> = sqlx::query_as(
            r#"
            SELECT id, name, host, port, username, auth_method,
                   encrypted_credentials, group_id, created_at, updated_at
            FROM servers
            WHERE name LIKE ? OR host LIKE ?
            ORDER BY name ASC
            "#,
        )
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        Ok(servers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{models::ServerFilters, Database, GroupRepository, NewGroup};
    use tempfile::TempDir;

    async fn create_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::new(&db_path).await.unwrap();
        db.init().await.unwrap();

        (db, temp_dir)
    }

    fn create_test_server(id: &str, name: &str, host: &str) -> NewServer {
        NewServer {
            id: id.to_string(),
            name: name.to_string(),
            host: host.to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_method: "password".to_string(),
            encrypted_credentials: vec![1, 2, 3, 4],
            group_id: None,
        }
    }

    #[tokio::test]
    async fn test_create_server() {
        let (db, _temp) = create_test_db().await;
        let repo = db.server_repository();

        let server = create_test_server("srv1", "Test Server", "192.168.1.1");
        repo.create(&server).await.unwrap();

        let found = repo.get_by_id("srv1").await.unwrap();
        assert_eq!(found.name, "Test Server");
        assert_eq!(found.host, "192.168.1.1");
        assert_eq!(found.port, 22);
    }

    #[tokio::test]
    async fn test_create_duplicate_id_fails() {
        let (db, _temp) = create_test_db().await;
        let repo = db.server_repository();

        let server = create_test_server("srv1", "Test Server", "192.168.1.1");
        repo.create(&server).await.unwrap();

        let result = repo.create(&server).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let (db, _temp) = create_test_db().await;
        let repo = db.server_repository();

        let result = repo.get_by_id("nonexistent").await;
        assert!(matches!(result, Err(DatabaseError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_get_all() {
        let (db, _temp) = create_test_db().await;
        let repo = db.server_repository();

        let server1 = create_test_server("srv1", "Server 1", "192.168.1.1");
        let server2 = create_test_server("srv2", "Server 2", "192.168.1.2");

        repo.create(&server1).await.unwrap();
        repo.create(&server2).await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_update_server() {
        let (db, _temp) = create_test_db().await;
        let repo = db.server_repository();

        let server = create_test_server("srv1", "Test Server", "192.168.1.1");
        repo.create(&server).await.unwrap();

        let update = UpdateServer {
            id: "srv1".to_string(),
            name: Some("Updated Name".to_string()),
            ..Default::default()
        };

        repo.update(&update).await.unwrap();

        let found = repo.get_by_id("srv1").await.unwrap();
        assert_eq!(found.name, "Updated Name");
        assert_eq!(found.host, "192.168.1.1"); // Unchanged
    }

    #[tokio::test]
    async fn test_delete_server() {
        let (db, _temp) = create_test_db().await;
        let repo = db.server_repository();

        let server = create_test_server("srv1", "Test Server", "192.168.1.1");
        repo.create(&server).await.unwrap();

        repo.delete("srv1").await.unwrap();

        let result = repo.get_by_id("srv1").await;
        assert!(matches!(result, Err(DatabaseError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_delete_nonexistent_fails() {
        let (db, _temp) = create_test_db().await;
        let repo = db.server_repository();

        let result = repo.delete("nonexistent").await;
        assert!(matches!(result, Err(DatabaseError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_server_with_group() {
        let (db, _temp) = create_test_db().await;
        let server_repo = db.server_repository();
        let group_repo = db.group_repository();

        // Create a group
        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: Some("#FF0000".to_string()),
        };
        group_repo.create(&group).await.unwrap();

        // Create server in group
        let mut server = create_test_server("srv1", "Test Server", "192.168.1.1");
        server.group_id = Some("group1".to_string());
        server_repo.create(&server).await.unwrap();

        // Get with group info
        let with_group = server_repo.get_with_group("srv1").await.unwrap();
        assert_eq!(with_group.group_name, Some("Production".to_string()));
        assert_eq!(with_group.group_color, Some("#FF0000".to_string()));
    }

    #[tokio::test]
    async fn test_get_by_group() {
        let (db, _temp) = create_test_db().await;
        let server_repo = db.server_repository();
        let group_repo = db.group_repository();

        // Create group
        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: None,
        };
        group_repo.create(&group).await.unwrap();

        // Create servers
        let mut server1 = create_test_server("srv1", "Server 1", "192.168.1.1");
        server1.group_id = Some("group1".to_string());
        server_repo.create(&server1).await.unwrap();

        let mut server2 = create_test_server("srv2", "Server 2", "192.168.1.2");
        server2.group_id = Some("group1".to_string());
        server_repo.create(&server2).await.unwrap();

        let server3 = create_test_server("srv3", "Server 3", "192.168.1.3");
        server_repo.create(&server3).await.unwrap();

        // Get by group
        let grouped = server_repo.get_by_group("group1").await.unwrap();
        assert_eq!(grouped.len(), 2);

        // Get ungrouped
        let ungrouped = server_repo.get_ungrouped().await.unwrap();
        assert_eq!(ungrouped.len(), 1);
        assert_eq!(ungrouped[0].id, "srv3");
    }

    #[tokio::test]
    async fn test_filter_by_group() {
        let (db, _temp) = create_test_db().await;
        let server_repo = db.server_repository();
        let group_repo = db.group_repository();

        // Create group and server
        let group = NewGroup {
            id: "group1".to_string(),
            name: "Production".to_string(),
            color: None,
        };
        group_repo.create(&group).await.unwrap();

        let mut server = create_test_server("srv1", "Test Server", "192.168.1.1");
        server.group_id = Some("group1".to_string());
        server_repo.create(&server).await.unwrap();

        // Filter by group
        let filters = ServerFilters {
            group_id: Some("group1".to_string()),
            search: None,
        };
        let filtered = server_repo.get_filtered(Some(filters)).await.unwrap();
        assert_eq!(filtered.len(), 1);

        // Filter by non-existent group
        let filters = ServerFilters {
            group_id: Some("nonexistent".to_string()),
            search: None,
        };
        let filtered = server_repo.get_filtered(Some(filters)).await.unwrap();
        assert!(filtered.is_empty());
    }

    #[tokio::test]
    async fn test_search() {
        let (db, _temp) = create_test_db().await;
        let repo = db.server_repository();

        let server1 = create_test_server("srv1", "Production Server", "192.168.1.1");
        let server2 = create_test_server("srv2", "Development Box", "10.0.0.1");

        repo.create(&server1).await.unwrap();
        repo.create(&server2).await.unwrap();

        // Search by name
        let results = repo.search("Production").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "srv1");

        // Search by host
        let results = repo.search("10.0.0").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "srv2");

        // Case insensitive search
        let results = repo.search("server").await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_count() {
        let (db, _temp) = create_test_db().await;
        let repo = db.server_repository();

        assert_eq!(repo.count().await.unwrap(), 0);

        let server = create_test_server("srv1", "Test", "192.168.1.1");
        repo.create(&server).await.unwrap();

        assert_eq!(repo.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_exists() {
        let (db, _temp) = create_test_db().await;
        let repo = db.server_repository();

        assert!(!repo.exists("srv1").await.unwrap());

        let server = create_test_server("srv1", "Test", "192.168.1.1");
        repo.create(&server).await.unwrap();

        assert!(repo.exists("srv1").await.unwrap());
    }

    #[tokio::test]
    async fn test_move_to_group() {
        let (db, _temp) = create_test_db().await;
        let server_repo = db.server_repository();
        let group_repo = db.group_repository();

        // Create groups
        let group1 = NewGroup {
            id: "group1".to_string(),
            name: "Group 1".to_string(),
            color: None,
        };
        group_repo.create(&group1).await.unwrap();

        let group2 = NewGroup {
            id: "group2".to_string(),
            name: "Group 2".to_string(),
            color: None,
        };
        group_repo.create(&group2).await.unwrap();

        // Create servers
        let mut server = create_test_server("srv1", "Test", "192.168.1.1");
        server.group_id = Some("group1".to_string());
        server_repo.create(&server).await.unwrap();

        // Move to new group
        server_repo
            .move_to_group(&["srv1".to_string()], Some("group2"))
            .await
            .unwrap();

        let moved = server_repo.get_by_id("srv1").await.unwrap();
        assert_eq!(moved.group_id, Some("group2".to_string()));

        // Move to no group (ungroup)
        server_repo
            .move_to_group(&["srv1".to_string()], None)
            .await
            .unwrap();

        let ungrouped = server_repo.get_by_id("srv1").await.unwrap();
        assert!(ungrouped.group_id.is_none());
    }
}
