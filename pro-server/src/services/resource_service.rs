use crate::{
    models::*,
    redis_cache::RedisCache,
};
use anyhow::Result;
use chrono::Utc;
use sqlx::AnyPool;
use uuid::Uuid;

pub struct ResourceService {
    db: AnyPool,
    redis: std::sync::Arc<RedisCache>,
}

impl ResourceService {
    pub fn new(db: AnyPool, redis: std::sync::Arc<RedisCache>) -> Self {
        Self { db, redis }
    }

    // Server sharing methods
    pub async fn share_server(
        &self,
        server_id: &str,
        team_id: &str,
        shared_by: &str,
        permissions: Option<serde_json::Value>,
    ) -> Result<SharedServer> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let default_permissions = serde_json::json!({
            "can_execute": true,
            "can_edit": false,
            "can_share": false,
            "can_delete": false
        });

        sqlx::query(
            "INSERT INTO shared_servers (id, server_id, team_id, shared_by, shared_at, permissions, is_active) VALUES (?, ?, ?, ?, ?, ?, TRUE)"
        )
        .bind(&id)
        .bind(server_id)
        .bind(team_id)
        .bind(shared_by)
        .bind(now)
        .bind(permissions.unwrap_or(default_permissions))
        .execute(&self.db)
        .await?;

        Ok(SharedServer {
            id,
            server_id: server_id.to_string(),
            team_id: team_id.to_string(),
            shared_by: shared_by.to_string(),
            shared_at: now,
            permissions,
            is_active: true,
        })
    }

    pub async fn list_shared_servers(
        &self,
        team_id: Option<&str>,
        _user_id: &str,
    ) -> Result<Vec<SharedServer>> {
        let servers = if let Some(tid) = team_id {
            sqlx::query_as::<_, SharedServer>(
                "SELECT * FROM shared_servers WHERE team_id = ? AND is_active = TRUE ORDER BY shared_at DESC"
            )
            .bind(tid)
            .fetch_all(&self.db)
            .await?
        } else {
            sqlx::query_as::<_, SharedServer>(
                "SELECT * FROM shared_servers WHERE is_active = TRUE ORDER BY shared_at DESC"
            )
            .fetch_all(&self.db)
            .await?
        };

        Ok(servers)
    }

    pub async fn get_shared_server(&self, id: &str) -> Result<SharedServer> {
        let server = sqlx::query_as::<_, SharedServer>(
            "SELECT * FROM shared_servers WHERE id = ? AND is_active = TRUE"
        )
        .bind(id)
        .fetch_one(&self.db)
        .await?;

        Ok(server)
    }

    pub async fn unshare_server(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE shared_servers SET is_active = FALSE WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    pub async fn update_server_permissions(
        &self,
        id: &str,
        permissions: serde_json::Value,
    ) -> Result<SharedServer> {
        sqlx::query("UPDATE shared_servers SET permissions = ? WHERE id = ?")
            .bind(&permissions)
            .bind(id)
            .execute(&self.db)
            .await?;

        self.get_shared_server(id).await
    }

    // Snippet methods
    pub async fn create_snippet(
        &self,
        req: &CreateSnippetRequest,
        created_by: &str,
    ) -> Result<Snippet> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO snippets (id, team_id, created_by, name, description, content, language, tags, is_public, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&req.team_id)
        .bind(created_by)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.content)
        .bind(&req.language)
        .bind(req.tags.as_ref().map(|t| serde_json::to_value(t).unwrap()))
        .bind(req.is_public.unwrap_or(false))
        .bind(now)
        .bind(now)
        .execute(&self.db)
        .await?;

        Ok(Snippet {
            id,
            team_id: req.team_id.clone(),
            created_by: created_by.to_string(),
            name: req.name.clone(),
            description: req.description.clone(),
            content: req.content.clone(),
            language: req.language.clone(),
            tags: req.tags.as_ref().map(|t| serde_json::to_value(t).unwrap()),
            is_public: req.is_public.unwrap_or(false),
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn list_snippets(
        &self,
        team_id: Option<&str>,
        user_id: &str,
    ) -> Result<Vec<Snippet>> {
        let snippets = if let Some(tid) = team_id {
            sqlx::query_as::<_, Snippet>(
                "SELECT * FROM snippets WHERE team_id = ? AND (is_public = TRUE OR created_by = ?) ORDER BY updated_at DESC"
            )
            .bind(tid)
            .bind(user_id)
            .fetch_all(&self.db)
            .await?
        } else {
            sqlx::query_as::<_, Snippet>(
                "SELECT * FROM snippets WHERE created_by = ? OR is_public = TRUE ORDER BY updated_at DESC"
            )
            .bind(user_id)
            .fetch_all(&self.db)
            .await?
        };

        Ok(snippets)
    }

    pub async fn get_snippet(&self, id: &str) -> Result<Snippet> {
        let snippet = sqlx::query_as::<_, Snippet>(
            "SELECT * FROM snippets WHERE id = ?"
        )
        .bind(id)
        .fetch_one(&self.db)
        .await?;

        Ok(snippet)
    }

    pub async fn update_snippet(&self, id: &str, req: UpdateSnippetRequest) -> Result<Snippet> {
        let now = Utc::now();

        sqlx::query(
            "UPDATE snippets SET name = COALESCE(?, name), description = COALESCE(?, description), content = COALESCE(?, content), language = COALESCE(?, language), tags = COALESCE(?, tags), is_public = COALESCE(?, is_public), updated_at = ? WHERE id = ?"
        )
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.content)
        .bind(&req.language)
        .bind(req.tags.as_ref().map(|t| serde_json::to_value(t).unwrap()))
        .bind(req.is_public)
        .bind(now)
        .bind(id)
        .execute(&self.db)
        .await?;

        self.get_snippet(id).await
    }

    pub async fn delete_snippet(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM snippets WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    pub async fn set_snippet_public(&self, id: &str, is_public: bool) -> Result<Snippet> {
        let now = Utc::now();

        sqlx::query("UPDATE snippets SET is_public = ?, updated_at = ? WHERE id = ?")
            .bind(is_public)
            .bind(now)
            .bind(id)
            .execute(&self.db)
            .await?;

        self.get_snippet(id).await
    }
}

