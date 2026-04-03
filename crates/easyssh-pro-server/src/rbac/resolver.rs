//! 资源解析器 - 服务器端资源路径解析

use async_trait::async_trait;
use easyssh_core::rbac::{RbacError, Resource, ResourceResolver, ResourceType};
use sqlx::{Pool, Sqlite};

/// 服务器资源解析器trait
#[async_trait]
pub trait ServerResourceResolver: Send + Sync {
    /// 异步解析资源
    async fn resolve_async(&self, path: &str) -> Result<Resource, RbacError>;

    /// 异步获取资源所有者
    async fn get_owner_async(&self, resource: &Resource) -> Option<String>;

    /// 异步检查资源是否存在
    async fn exists_async(&self, resource: &Resource) -> bool;

    /// 获取资源的团队ID
    async fn get_resource_team(&self, resource: &Resource) -> Option<String>;
}

/// 数据库资源解析器
pub struct DatabaseResourceResolver {
    db: Pool<Sqlite>,
}

impl DatabaseResourceResolver {
    /// 创建新的数据库资源解析器
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    /// 解析路径到资源类型
    fn parse_path(&self, path: &str) -> Result<(ResourceType, Option<String>), RbacError> {
        // 移除API前缀
        let clean_path = path
            .trim_start_matches("/api/v1/")
            .trim_start_matches("/api/")
            .trim_start_matches("/");

        let parts: Vec<&str> = clean_path.split('/').collect();

        if parts.is_empty() {
            return Err(RbacError::InvalidResource("Empty path".to_string()));
        }

        // 映射资源类型
        let resource_type = match parts[0] {
            "servers" => ResourceType::Server,
            "teams" => ResourceType::Team,
            "members" => ResourceType::Member,
            "roles" => ResourceType::Role,
            "snippets" => ResourceType::Snippet,
            "keys" => ResourceType::Key,
            "sessions" => ResourceType::Session,
            "audit" => ResourceType::AuditLog,
            "collaboration" => ResourceType::Collaboration,
            "config" => ResourceType::Config,
            "layout" => ResourceType::Layout,
            _ => {
                return Err(RbacError::InvalidResource(format!(
                    "Unknown resource type: {}",
                    parts[0]
                )))
            }
        };

        // 提取资源ID
        let resource_id = parts.get(1).map(|s| s.to_string());

        Ok((resource_type, resource_id))
    }
}

#[async_trait]
impl ServerResourceResolver for DatabaseResourceResolver {
    async fn resolve_async(&self, path: &str) -> Result<Resource, RbacError> {
        let (resource_type, resource_id) = self.parse_path(path)?;

        let resource = if let Some(id) = resource_id {
            Resource::specific(resource_type, id)
        } else {
            Resource::all(resource_type)
        };

        // 尝试获取团队ID
        let team_id = self.get_resource_team(&resource).await;

        if let Some(team_id) = team_id {
            Ok(resource.in_team(team_id))
        } else {
            Ok(resource)
        }
    }

    async fn get_owner_async(&self, resource: &Resource) -> Option<String> {
        let Some(ref resource_id) = resource.resource_id else {
            return None;
        };

        let result: Result<Option<String>, _> = match resource.resource_type {
            ResourceType::Server => {
                sqlx::query_scalar("SELECT created_by FROM servers WHERE id = ?")
                    .bind(resource_id)
                    .fetch_optional(&self.db)
                    .await
            }
            ResourceType::Team => {
                sqlx::query_scalar("SELECT created_by FROM teams WHERE id = ?")
                    .bind(resource_id)
                    .fetch_optional(&self.db)
                    .await
            }
            ResourceType::Snippet => {
                sqlx::query_scalar("SELECT created_by FROM snippets WHERE id = ?")
                    .bind(resource_id)
                    .fetch_optional(&self.db)
                    .await
            }
            ResourceType::Session => {
                sqlx::query_scalar("SELECT host_id FROM collaboration_sessions WHERE id = ?")
                    .bind(resource_id)
                    .fetch_optional(&self.db)
                    .await
            }
            _ => Ok(None),
        };

        result.ok().flatten()
    }

    async fn exists_async(&self, resource: &Resource) -> bool {
        let Some(ref resource_id) = resource.resource_id else {
            return true; // 通配资源总是存在
        };

        let table = match resource.resource_type {
            ResourceType::Server => "servers",
            ResourceType::Team => "teams",
            ResourceType::Member => "team_members",
            ResourceType::Role => "roles",
            ResourceType::Snippet => "snippets",
            ResourceType::Session => "collaboration_sessions",
            _ => return true, // 其他类型不检查
        };

        let query = format!("SELECT EXISTS(SELECT 1 FROM {} WHERE id = ?)", table);

        sqlx::query_scalar::<_, bool>(&query)
            .bind(resource_id)
            .fetch_one(&self.db)
            .await
            .unwrap_or(false)
    }

    async fn get_resource_team(&self, resource: &Resource) -> Option<String> {
        // 如果资源已经包含团队ID，直接返回
        if let Some(ref team_id) = resource.team_id {
            return Some(team_id.clone());
        }

        let Some(ref resource_id) = resource.resource_id else {
            return None;
        };

        let result: Result<Option<String>, _> = match resource.resource_type {
            ResourceType::Server => {
                // 从shared_servers表中获取团队ID
                sqlx::query_scalar("SELECT team_id FROM shared_servers WHERE server_id = ?")
                    .bind(resource_id)
                    .fetch_optional(&self.db)
                    .await
            }
            ResourceType::Snippet => {
                sqlx::query_scalar("SELECT team_id FROM snippets WHERE id = ?")
                    .bind(resource_id)
                    .fetch_optional(&self.db)
                    .await
            }
            ResourceType::Member => {
                sqlx::query_scalar("SELECT team_id FROM team_members WHERE id = ?")
                    .bind(resource_id)
                    .fetch_optional(&self.db)
                    .await
            }
            ResourceType::Role => {
                sqlx::query_scalar("SELECT team_id FROM roles WHERE id = ?")
                    .bind(resource_id)
                    .fetch_optional(&self.db)
                    .await
            }
            ResourceType::Session => {
                sqlx::query_scalar("SELECT team_id FROM collaboration_sessions WHERE id = ?")
                    .bind(resource_id)
                    .fetch_optional(&self.db)
                    .await
            }
            ResourceType::Team => {
                // 团队资源本身就是团队
                Ok(Some(resource_id.clone()))
            }
            _ => Ok(None),
        };

        result.ok().flatten()
    }
}

// 为DatabaseResourceResolver实现同步ResourceResolver trait
// 注意：这只是一个适配器，实际应该使用异步版本
impl ResourceResolver for DatabaseResourceResolver {
    fn resolve(&self, path: &str) -> Result<Resource, RbacError> {
        // 同步实现返回错误，建议使用异步版本
        Err(RbacError::InvalidResource(
            "Use resolve_async for database resolver".to_string(),
        ))
    }

    fn get_owner(&self, resource: &Resource) -> Option<String> {
        // 阻塞式获取，不推荐使用
        let rt = tokio::runtime::Handle::try_current();
        match rt {
            Ok(handle) => handle.block_on(async { self.get_owner_async(resource).await }),
            Err(_) => None,
        }
    }

    fn exists(&self, resource: &Resource) -> bool {
        let rt = tokio::runtime::Handle::try_current();
        match rt {
            Ok(handle) => handle.block_on(async { self.exists_async(resource).await }),
            Err(_) => false,
        }
    }
}

/// 内存资源解析器（用于测试）
pub struct InMemoryResourceResolver {
    resources: std::collections::HashMap<String, (ResourceType, String, Option<String>)>, // path -> (type, id, owner)
}

impl InMemoryResourceResolver {
    /// 创建新的内存资源解析器
    pub fn new() -> Self {
        Self {
            resources: std::collections::HashMap::new(),
        }
    }

    /// 添加资源
    pub fn add_resource(
        &mut self,
        path: impl Into<String>,
        resource_type: ResourceType,
        resource_id: impl Into<String>,
        owner: Option<String>,
    ) {
        self.resources
            .insert(path.into(), (resource_type, resource_id.into(), owner));
    }
}

impl Default for InMemoryResourceResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceResolver for InMemoryResourceResolver {
    fn resolve(&self, path: &str) -> Result<Resource, RbacError> {
        if let Some((resource_type, resource_id, _)) = self.resources.get(path) {
            Ok(Resource::specific(*resource_type, resource_id.clone()))
        } else {
            // 尝试解析路径
            let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

            if parts.len() >= 2 {
                let resource_type = match parts[0] {
                    "servers" => ResourceType::Server,
                    "teams" => ResourceType::Team,
                    "snippets" => ResourceType::Snippet,
                    "keys" => ResourceType::Key,
                    "sessions" => ResourceType::Session,
                    _ => {
                        return Err(RbacError::InvalidResource(format!(
                            "Unknown resource type in path: {}",
                            path
                        )))
                    }
                };

                Ok(Resource::specific(resource_type, parts[1]))
            } else {
                Err(RbacError::InvalidResource(format!(
                    "Cannot parse path: {}",
                    path
                )))
            }
        }
    }

    fn get_owner(&self, resource: &Resource) -> Option<String> {
        for (_, (rt, rid, owner)) in &self.resources {
            if resource.resource_type == *rt {
                if let Some(ref id) = resource.resource_id {
                    if id == rid {
                        return owner.clone();
                    }
                }
            }
        }
        None
    }

    fn exists(&self, resource: &Resource) -> bool {
        if resource.resource_id.is_none() {
            return true; // 通配资源
        }

        self.resources.values().any(|(rt, rid, _)| {
            resource.resource_type == *rt && resource.resource_id.as_ref() == Some(rid)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_resolver() {
        let mut resolver = InMemoryResourceResolver::new();
        resolver.add_resource(
            "/servers/srv-1",
            ResourceType::Server,
            "srv-1",
            Some("user1".to_string()),
        );

        let resource = resolver.resolve("/servers/srv-1").unwrap();
        assert_eq!(resource.resource_type, ResourceType::Server);
        assert_eq!(resource.resource_id, Some("srv-1".to_string()));

        let owner = resolver.get_owner(&resource);
        assert_eq!(owner, Some("user1".to_string()));

        assert!(resolver.exists(&resource));
    }

    #[tokio::test]
    async fn test_database_resolver_resolve() {
        // 这里需要实际的数据库连接才能测试
        // 示例代码展示如何使用
        // let resolver = DatabaseResourceResolver::new(pool);
        // let resource = resolver.resolve_async("/servers/server-123").await.unwrap();
    }

    #[test]
    fn test_parse_path() {
        let resolver = DatabaseResourceResolver::new(
            // 这里需要实际的池
            panic!("Needs database pool"),
        );

        // 无法在没有数据库的情况下测试
    }
}
