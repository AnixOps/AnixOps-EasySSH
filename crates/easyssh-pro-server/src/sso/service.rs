//! Pro Server - SSO服务
//!
//! 提供SSO相关的业务逻辑和数据访问

use chrono::Utc;
use sqlx::Row;

use crate::{db::Database, redis_cache::RedisCache};
use easyssh_core::sso::{SsoProvider, SsoProviderType, SsoSession};

/// SSO数据服务
pub struct SsoDataService {
    db: Database,
    redis: RedisCache,
}

impl SsoDataService {
    /// 创建新的SSO数据服务
    pub fn new(db: Database, redis: RedisCache) -> Self {
        Self { db, redis }
    }

    /// 保存提供商到数据库
    pub async fn save_provider(&self, provider: &SsoProvider) -> Result<(), sqlx::Error> {
        let config_json = serde_json::to_string(&provider.config).map_err(|e| {
            sqlx::Error::Protocol(format!("JSON serialization failed: {}", e).into())
        })?;

        sqlx::query(
            r#"
            INSERT INTO sso_providers (id, name, provider_type, config, enabled, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                config = excluded.config,
                enabled = excluded.enabled,
                updated_at = excluded.updated_at
            "#
        )
        .bind(&provider.id)
        .bind(&provider.name)
        .bind(provider.provider_type.to_string())
        .bind(config_json)
        .bind(provider.enabled)
        .bind(provider.created_at)
        .bind(provider.updated_at)
        .execute(self.db.pool())
        .await?;

        // 更新缓存
        let _ = self.cache_provider(provider).await;

        Ok(())
    }

    /// 从数据库加载提供商
    pub async fn load_provider(
        &self,
        provider_id: &str,
    ) -> Result<Option<SsoProvider>, sqlx::Error> {
        // 先检查缓存
        if let Ok(Some(cached)) = self.get_cached_provider(provider_id).await {
            return Ok(Some(cached));
        }

        let row = sqlx::query("SELECT * FROM sso_providers WHERE id = ?1 AND enabled = true")
            .bind(provider_id)
            .fetch_optional(self.db.pool())
            .await?;

        if let Some(row) = row {
            let provider = self.row_to_provider(row)?;

            // 更新缓存
            let _ = self.cache_provider(&provider).await;

            Ok(Some(provider))
        } else {
            Ok(None)
        }
    }

    /// 列出所有提供商
    pub async fn list_providers(&self) -> Result<Vec<SsoProvider>, sqlx::Error> {
        let rows = sqlx::query("SELECT * FROM sso_providers ORDER BY created_at DESC")
            .fetch_all(self.db.pool())
            .await?;

        let mut providers = Vec::new();
        for row in rows {
            if let Ok(provider) = self.row_to_provider(row) {
                providers.push(provider);
            }
        }

        Ok(providers)
    }

    /// 删除提供商
    pub async fn delete_provider(&self, provider_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sso_providers WHERE id = ?1")
            .bind(provider_id)
            .execute(self.db.pool())
            .await?;

        // 清除缓存
        let _ = self
            .redis
            .delete(&format!("sso:provider:{}", provider_id))
            .await;

        Ok(())
    }

    /// 缓存提供商到Redis
    async fn cache_provider(&self, provider: &SsoProvider) -> Result<(), String> {
        let key = format!("sso:provider:{}", provider.id);
        let value =
            serde_json::to_string(provider).map_err(|e| format!("Serialization failed: {}", e))?;

        self.redis
            .set(&key, &value, std::time::Duration::from_secs(3600))
            .await
            .map_err(|e| format!("Redis error: {}", e))
    }

    /// 从Redis获取缓存的提供商
    pub async fn get_cached_provider(
        &self,
        provider_id: &str,
    ) -> Result<Option<SsoProvider>, String> {
        let key = format!("sso:provider:{}", provider_id);

        let value = self
            .redis
            .get(&key)
            .await
            .map_err(|e| format!("Redis error: {}", e))?;

        if let Some(json_str) = value {
            let provider = serde_json::from_str(&json_str)
                .map_err(|e| format!("Deserialization failed: {}", e))?;
            Ok(Some(provider))
        } else {
            Ok(None)
        }
    }

    /// 保存SSO会话到数据库
    pub async fn save_session(&self, session: &SsoSession) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO sso_sessions (
                id, user_id, provider_id, created_at, expires_at, last_used_at,
                ip_address, user_agent, status, metadata
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                last_used_at = excluded.last_used_at,
                status = excluded.status
            "#,
        )
        .bind(&session.id)
        .bind(&session.user_id)
        .bind(&session.provider_id)
        .bind(session.created_at)
        .bind(session.expires_at)
        .bind(session.last_used_at)
        .bind(&session.ip_address)
        .bind(&session.user_agent)
        .bind(format!("{:?}", session.status))
        .bind(serde_json::to_string(&session.metadata).unwrap_or_default())
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// 加载用户会话
    pub async fn load_user_sessions(&self, user_id: &str) -> Result<Vec<SsoSession>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM sso_sessions
            WHERE user_id = ?1 AND expires_at > datetime('now')
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(self.db.pool())
        .await?;

        let mut sessions = Vec::new();
        for row in rows {
            if let Ok(session) = self.row_to_session(row) {
                sessions.push(session);
            }
        }

        Ok(sessions)
    }

    /// 删除会话
    pub async fn delete_session(&self, session_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sso_sessions WHERE id = ?1")
            .bind(session_id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// 清理过期会话
    pub async fn cleanup_expired_sessions(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM sso_sessions WHERE expires_at < datetime('now') OR status = 'Revoked'",
        )
        .execute(self.db.pool())
        .await?;

        Ok(result.rows_affected())
    }

    /// 保存团队SSO映射
    pub async fn save_team_sso_mapping(
        &self,
        team_id: &str,
        provider_id: &str,
        group_mappings: &str,
        auto_provision: bool,
        default_role: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO team_sso_mappings (
                team_id, provider_id, group_mappings, auto_provision, default_role, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(team_id, provider_id) DO UPDATE SET
                group_mappings = excluded.group_mappings,
                auto_provision = excluded.auto_provision,
                default_role = excluded.default_role,
                updated_at = excluded.updated_at
            "#
        )
        .bind(team_id)
        .bind(provider_id)
        .bind(group_mappings)
        .bind(auto_provision)
        .bind(default_role)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// 获取团队SSO映射
    pub async fn get_team_sso_mapping(
        &self,
        team_id: &str,
    ) -> Result<Option<easyssh_core::sso::TeamSsoMapping>, sqlx::Error> {
        let row = sqlx::query("SELECT * FROM team_sso_mappings WHERE team_id = ?1")
            .bind(team_id)
            .fetch_optional(self.db.pool())
            .await?;

        if let Some(row) = row {
            let mapping = easyssh_core::sso::TeamSsoMapping {
                team_id: row.get("team_id"),
                provider_id: row.get("provider_id"),
                group_mappings: serde_json::from_str(
                    row.get::<String, _>("group_mappings").as_str(),
                )
                .unwrap_or_default(),
                auto_provision: row.get("auto_provision"),
                default_role: row.get("default_role"),
            };
            Ok(Some(mapping))
        } else {
            Ok(None)
        }
    }

    /// 辅助方法：将数据库行转换为SsoProvider
    fn row_to_provider(&self, row: sqlx::any::AnyRow) -> Result<SsoProvider, sqlx::Error> {
        let provider_type_str: String = row.get("provider_type");
        let provider_type = match provider_type_str.as_str() {
            "SAML 2.0" => SsoProviderType::Saml,
            "OpenID Connect" => SsoProviderType::Oidc,
            "OAuth 2.0" => SsoProviderType::OAuth2,
            "LDAP/AD" => SsoProviderType::Ldap,
            _ => SsoProviderType::Oidc,
        };

        let config_json: String = row.get("config");
        let config = match provider_type {
            SsoProviderType::Saml => {
                let cfg: easyssh_core::sso::SamlConfig = serde_json::from_str(&config_json)
                    .map_err(|e| {
                        sqlx::Error::Protocol(format!("Config parse error: {}", e).into())
                    })?;
                easyssh_core::sso::SsoProviderConfig::Saml(cfg)
            }
            SsoProviderType::Oidc => {
                let cfg: easyssh_core::sso::OidcConfig = serde_json::from_str(&config_json)
                    .map_err(|e| {
                        sqlx::Error::Protocol(format!("Config parse error: {}", e).into())
                    })?;
                easyssh_core::sso::SsoProviderConfig::Oidc(cfg)
            }
            SsoProviderType::OAuth2 => {
                let cfg: easyssh_core::sso::OAuth2Config = serde_json::from_str(&config_json)
                    .map_err(|e| {
                        sqlx::Error::Protocol(format!("Config parse error: {}", e).into())
                    })?;
                easyssh_core::sso::SsoProviderConfig::OAuth2(cfg)
            }
            _ => {
                let cfg: easyssh_core::sso::OidcConfig = serde_json::from_str(&config_json)
                    .map_err(|e| {
                        sqlx::Error::Protocol(format!("Config parse error: {}", e).into())
                    })?;
                easyssh_core::sso::SsoProviderConfig::Oidc(cfg)
            }
        };

        Ok(SsoProvider {
            id: row.get("id"),
            name: row.get("name"),
            provider_type,
            enabled: row.get("enabled"),
            config,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// 辅助方法：将数据库行转换为SsoSession
    fn row_to_session(&self, row: sqlx::any::AnyRow) -> Result<SsoSession, sqlx::Error> {
        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "Active" => easyssh_core::sso::SessionStatus::Active,
            "Expired" => easyssh_core::sso::SessionStatus::Expired,
            "Revoked" => easyssh_core::sso::SessionStatus::Revoked,
            "Suspended" => easyssh_core::sso::SessionStatus::Suspended,
            _ => easyssh_core::sso::SessionStatus::Active,
        };

        let metadata_str: String = row.get("metadata");
        let metadata: std::collections::HashMap<String, String> =
            serde_json::from_str(&metadata_str).unwrap_or_default();

        Ok(SsoSession {
            id: row.get("id"),
            user_id: row.get("user_id"),
            provider_id: row.get("provider_id"),
            created_at: row.get("created_at"),
            expires_at: row.get("expires_at"),
            last_used_at: row.get("last_used_at"),
            encrypted_sso_token: None,
            encrypted_id_token: None,
            encrypted_access_token: None,
            encrypted_refresh_token: None,
            ip_address: row.get("ip_address"),
            user_agent: row.get("user_agent"),
            status,
            metadata,
        })
    }
}

/// Redis扩展方法
pub trait SsoRedisExt {
    /// 缓存提供商
    async fn cache_provider(
        &self,
        provider_id: &str,
        provider: &crate::sso::ProviderResponse,
    ) -> Result<(), String>;
    /// 获取缓存的提供商
    async fn get_cached_provider(
        &self,
        provider_id: &str,
    ) -> Result<Option<crate::sso::ProviderResponse>, String>;
    /// 删除缓存的提供商
    async fn delete_cached_provider(&self, provider_id: &str) -> Result<(), String>;
}

impl SsoRedisExt for RedisCache {
    async fn cache_provider(
        &self,
        provider_id: &str,
        provider: &crate::sso::ProviderResponse,
    ) -> Result<(), String> {
        let key = format!("sso:provider:{}", provider_id);
        let value =
            serde_json::to_string(provider).map_err(|e| format!("Serialization failed: {}", e))?;

        self.set(&key, &value, std::time::Duration::from_secs(3600))
            .await
            .map_err(|e| format!("Redis error: {}", e))
    }

    async fn get_cached_provider(
        &self,
        provider_id: &str,
    ) -> Result<Option<crate::sso::ProviderResponse>, String> {
        let key = format!("sso:provider:{}", provider_id);

        let value = self
            .get(&key)
            .await
            .map_err(|e| format!("Redis error: {}", e))?;

        if let Some(json_str) = value {
            let provider = serde_json::from_str(&json_str)
                .map_err(|e| format!("Deserialization failed: {}", e))?;
            Ok(Some(provider))
        } else {
            Ok(None)
        }
    }

    async fn delete_cached_provider(&self, provider_id: &str) -> Result<(), String> {
        let key = format!("sso:provider:{}", provider_id);
        self.delete(&key)
            .await
            .map_err(|e| format!("Redis error: {}", e))
    }
}

/// SQL迁移：创建SSO表
pub async fn create_sso_tables(pool: &sqlx::Pool<sqlx::Any>) -> Result<(), sqlx::Error> {
    // SSO提供商表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sso_providers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            provider_type TEXT NOT NULL,
            config TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT true,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // SSO会话表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sso_sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            provider_id TEXT NOT NULL,
            created_at DATETIME NOT NULL,
            expires_at DATETIME NOT NULL,
            last_used_at DATETIME NOT NULL,
            ip_address TEXT,
            user_agent TEXT,
            status TEXT NOT NULL DEFAULT 'Active',
            metadata TEXT NOT NULL DEFAULT '{}',
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (provider_id) REFERENCES sso_providers(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 团队SSO映射表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS team_sso_mappings (
            team_id TEXT NOT NULL,
            provider_id TEXT NOT NULL,
            group_mappings TEXT NOT NULL DEFAULT '[]',
            auto_provision BOOLEAN NOT NULL DEFAULT false,
            default_role TEXT NOT NULL DEFAULT 'Member',
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            PRIMARY KEY (team_id, provider_id),
            FOREIGN KEY (team_id) REFERENCES teams(id),
            FOREIGN KEY (provider_id) REFERENCES sso_providers(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 用户SSO关联表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_sso_links (
            user_id TEXT NOT NULL,
            provider_id TEXT NOT NULL,
            external_user_id TEXT NOT NULL,
            linked_at DATETIME NOT NULL,
            PRIMARY KEY (user_id, provider_id),
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (provider_id) REFERENCES sso_providers(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建索引
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sso_sessions_user_id ON sso_sessions(user_id)")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_sso_sessions_provider_id ON sso_sessions(provider_id)",
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sso_sessions_expires ON sso_sessions(expires_at)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_team_sso_mappings_provider ON team_sso_mappings(provider_id)")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_user_sso_external ON user_sso_links(external_user_id)",
    )
    .execute(pool)
    .await?;

    tracing::info!("SSO tables created successfully");

    Ok(())
}
