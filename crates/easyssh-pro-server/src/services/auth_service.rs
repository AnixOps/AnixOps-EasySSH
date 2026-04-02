use crate::{models::*, redis_cache::RedisCache};
use anyhow::Result;
use chrono::Utc;
use sqlx::AnyPool;
use uuid::Uuid;

pub struct AuthService {
    db: AnyPool,
    redis: std::sync::Arc<RedisCache>,
}

impl AuthService {
    pub fn new(db: AnyPool, redis: std::sync::Arc<RedisCache>) -> Self {
        Self { db, redis }
    }

    pub async fn create_user(&self, email: &str, password_hash: &str, name: &str) -> Result<User> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO users (id, email, password_hash, name, created_at, updated_at, is_active) VALUES (?, ?, ?, ?, ?, ?, TRUE)"
        )
        .bind(&id)
        .bind(email)
        .bind(password_hash)
        .bind(name)
        .bind(now)
        .bind(now)
        .execute(&self.db)
        .await?;

        Ok(User {
            id,
            email: email.to_string(),
            password_hash: Some(password_hash.to_string()),
            name: name.to_string(),
            avatar_url: None,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            is_active: true,
            is_admin: false,
            sso_provider: None,
            sso_id: None,
            mfa_enabled: false,
            mfa_secret: None,
        })
    }

    pub async fn authenticate_user(&self, email: &str, password: &str) -> Result<User> {
        use bcrypt::verify;

        let user: User =
            sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ? AND is_active = TRUE")
                .bind(email)
                .fetch_one(&self.db)
                .await?;

        let hash = user
            .password_hash
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No password set for user"))?;

        if !verify(password, hash)? {
            return Err(anyhow::anyhow!("Invalid credentials"));
        }

        // Update last login
        sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(&user.id)
            .execute(&self.db)
            .await?;

        Ok(user)
    }

    pub async fn get_user_by_id(&self, user_id: &str) -> Result<User> {
        let user =
            sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ? AND is_active = TRUE")
                .bind(user_id)
                .fetch_one(&self.db)
                .await?;

        Ok(user)
    }

    pub async fn create_session(
        &self,
        user_id: &str,
        token_hash: &str,
        refresh_token_hash: &str,
        _device_info: Option<&str>,
        jti: &str,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::days(7);

        sqlx::query(
            "INSERT INTO sessions (id, user_id, token_hash, refresh_token_hash, created_at, expires_at, is_active) VALUES (?, ?, ?, ?, ?, ?, TRUE)"
        )
        .bind(&id)
        .bind(user_id)
        .bind(token_hash)
        .bind(refresh_token_hash)
        .bind(now)
        .bind(expires_at)
        .execute(&self.db)
        .await?;

        Ok(jti.to_string())
    }

    pub async fn create_api_key(
        &self,
        user_id: &str,
        name: &str,
        key_hash: &str,
        key_prefix: &str,
        scopes: Option<serde_json::Value>,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> Result<ApiKey> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO api_keys (id, user_id, name, key_hash, key_prefix, scopes, created_at, expires_at, is_active) VALUES (?, ?, ?, ?, ?, ?, ?, ?, TRUE)"
        )
        .bind(&id)
        .bind(user_id)
        .bind(name)
        .bind(key_hash)
        .bind(key_prefix)
        .bind(scopes)
        .bind(now)
        .bind(expires_at)
        .execute(&self.db)
        .await?;

        Ok(ApiKey {
            id,
            user_id: user_id.to_string(),
            name: name.to_string(),
            key_hash: key_hash.to_string(),
            key_prefix: key_prefix.to_string(),
            scopes,
            created_at: now,
            expires_at,
            last_used_at: None,
            is_active: true,
        })
    }

    pub async fn list_api_keys(&self, user_id: &str) -> Result<Vec<ApiKey>> {
        let keys = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE user_id = ? AND is_active = TRUE",
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(keys)
    }

    pub async fn get_api_key(&self, id: &str) -> Result<ApiKey> {
        let key =
            sqlx::query_as::<_, ApiKey>("SELECT * FROM api_keys WHERE id = ? AND is_active = TRUE")
                .bind(id)
                .fetch_one(&self.db)
                .await?;

        Ok(key)
    }

    pub async fn delete_api_key(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE api_keys SET is_active = FALSE WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
    }
}
