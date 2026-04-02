//! 权限缓存 - 高性能权限检查结果缓存

use easyssh_core::rbac::{CheckResult, Permission, PermissionContext};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    result: CheckResult,
    timestamp: chrono::DateTime<chrono::Utc>,
    ttl_seconds: u64,
    hit_count: u64,
}

impl CacheEntry {
    /// 检查是否有效
    fn is_valid(&self) -> bool {
        let elapsed = (chrono::Utc::now() - self.timestamp).num_seconds();
        elapsed < self.ttl_seconds as i64
    }

    /// 增加命中计数
    fn hit(&mut self) {
        self.hit_count += 1;
    }
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 默认TTL（秒）
    pub default_ttl: u64,
    /// 最大条目数
    pub max_entries: usize,
    /// 清理间隔（秒）
    pub cleanup_interval: u64,
    /// 是否启用缓存预热
    pub enable_warmup: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: 300, // 5分钟
            max_entries: 10000,
            cleanup_interval: 60, // 1分钟
            enable_warmup: false,
        }
    }
}

/// 权限缓存
pub struct PermissionCache {
    config: CacheConfig,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    stats: Arc<RwLock<CacheStats>>,
}

/// 缓存统计
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evictions: u64,
    pub expired_hits: u64,
}

impl CacheStats {
    /// 计算命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            (self.cache_hits as f64 / total as f64) * 100.0
        }
    }
}

impl PermissionCache {
    /// 创建新的权限缓存
    pub fn new(config: CacheConfig) -> Self {
        let cache = Arc::new(RwLock::new(HashMap::new()));
        let stats = Arc::new(RwLock::new(CacheStats::default()));

        // 启动清理任务
        if config.cleanup_interval > 0 {
            let cache_clone = cache.clone();
            let stats_clone = stats.clone();
            let interval = config.cleanup_interval;

            tokio::spawn(async move {
                let mut interval =
                    tokio::time::interval(tokio::time::Duration::from_secs(interval));

                loop {
                    interval.tick().await;
                    Self::cleanup_expired(&cache_clone, &stats_clone).await;
                }
            });
        }

        Self {
            config,
            cache,
            stats,
        }
    }

    /// 获取缓存键
    fn build_cache_key(ctx: &PermissionContext, permission: &Permission) -> String {
        let mut key = format!(
            "{}:{}:{}",
            ctx.user_id,
            permission.resource.resource_type.as_str(),
            permission.operation.as_str()
        );

        if let Some(ref resource_id) = permission.resource.resource_id {
            key.push(':');
            key.push_str(resource_id);
        }

        if let Some(ref team_id) = ctx.team_id {
            key.push_str(":team:");
            key.push_str(team_id);
        }

        key
    }

    /// 获取缓存结果
    pub async fn get(
        &self,
        ctx: &PermissionContext,
        permission: &Permission,
    ) -> Option<CheckResult> {
        let key = Self::build_cache_key(ctx, permission);
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        stats.total_requests += 1;

        if let Some(entry) = cache.get_mut(&key) {
            if entry.is_valid() {
                entry.hit();
                stats.cache_hits += 1;
                return Some(entry.result.clone());
            } else {
                stats.expired_hits += 1;
                cache.remove(&key);
            }
        }

        stats.cache_misses += 1;
        None
    }

    /// 设置缓存
    pub async fn set(
        &self,
        ctx: &PermissionContext,
        permission: &Permission,
        result: CheckResult,
        ttl_seconds: Option<u64>,
    ) {
        let key = Self::build_cache_key(ctx, permission);
        let ttl = ttl_seconds.unwrap_or(self.config.default_ttl);

        let mut cache = self.cache.write().await;

        // 检查是否需要清理
        if cache.len() >= self.config.max_entries {
            // 简单的LRU清理：移除最早的一个条目
            let oldest_key = cache
                .iter()
                .min_by_key(|(_, entry)| entry.timestamp)
                .map(|(k, _)| k.clone());

            if let Some(key) = oldest_key {
                cache.remove(&key);
                let mut stats = self.stats.write().await;
                stats.evictions += 1;
            }
        }

        let entry = CacheEntry {
            result,
            timestamp: chrono::Utc::now(),
            ttl_seconds: ttl,
            hit_count: 0,
        };

        cache.insert(key, entry);
    }

    /// 清理过期条目
    async fn cleanup_expired(
        cache: &Arc<RwLock<HashMap<String, CacheEntry>>>,
        stats: &Arc<RwLock<CacheStats>>,
    ) {
        let mut cache = cache.write().await;
        let before = cache.len();
        cache.retain(|_, entry| entry.is_valid());
        let after = cache.len();

        if before > after {
            let mut stats = stats.write().await;
            stats.evictions += (before - after) as u64;
        }
    }

    /// 清除特定用户的缓存
    pub async fn clear_user(&self, user_id: &str) {
        let mut cache = self.cache.write().await;
        cache.retain(|key, _| !key.starts_with(&format!("{}:", user_id)));
    }

    /// 清除所有缓存
    pub async fn clear_all(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// 获取当前缓存大小
    pub async fn size(&self) -> usize {
        self.cache.read().await.len()
    }

    /// 预热缓存（预加载常用权限）
    pub async fn warmup(
        &self,
        user_id: &str,
        common_permissions: &[(PermissionContext, Permission)],
        check_fn: impl Fn(&PermissionContext, &Permission) -> CheckResult,
    ) {
        if !self.config.enable_warmup {
            return;
        }

        for (ctx, permission) in common_permissions {
            if ctx.user_id == user_id {
                let result = check_fn(ctx, permission);
                self.set(ctx, permission, result, None).await;
            }
        }
    }

    /// 批量获取
    pub async fn get_batch(
        &self,
        ctx: &PermissionContext,
        permissions: &[Permission],
    ) -> Vec<Option<CheckResult>> {
        let mut results = Vec::new();
        for permission in permissions {
            results.push(self.get(ctx, permission).await);
        }
        results
    }

    /// 批量设置
    pub async fn set_batch(
        &self,
        ctx: &PermissionContext,
        results: &[(Permission, CheckResult)],
        ttl_seconds: Option<u64>,
    ) {
        for (permission, result) in results {
            self.set(ctx, permission, result.clone(), ttl_seconds).await;
        }
    }
}

impl Default for PermissionCache {
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

/// 缓存构建器
pub struct PermissionCacheBuilder {
    config: CacheConfig,
}

impl PermissionCacheBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            config: CacheConfig::default(),
        }
    }

    /// 设置TTL
    pub fn with_ttl(mut self, ttl: u64) -> Self {
        self.config.default_ttl = ttl;
        self
    }

    /// 设置最大条目数
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.config.max_entries = max;
        self
    }

    /// 设置清理间隔
    pub fn with_cleanup_interval(mut self, interval: u64) -> Self {
        self.config.cleanup_interval = interval;
        self
    }

    /// 启用预热
    pub fn enable_warmup(mut self) -> Self {
        self.config.enable_warmup = true;
        self
    }

    /// 构建缓存
    pub fn build(self) -> PermissionCache {
        PermissionCache::new(self.config)
    }
}

impl Default for PermissionCacheBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use easyssh_core::rbac::{Operation, Resource, ResourceType};

    fn create_test_context(user_id: &str) -> PermissionContext {
        PermissionContext::new(user_id)
    }

    fn create_test_permission() -> Permission {
        Permission::new(Resource::all(ResourceType::Server), Operation::Read)
    }

    #[tokio::test]
    async fn test_cache_get_set() {
        let cache = PermissionCache::new(CacheConfig::default());
        let ctx = create_test_context("user1");
        let permission = create_test_permission();

        // 初始为空
        assert!(cache.get(&ctx, &permission).await.is_none());

        // 设置缓存
        let result = CheckResult::allowed();
        cache.set(&ctx, &permission, result.clone(), None).await;

        // 获取缓存
        let cached = cache.get(&ctx, &permission).await;
        assert!(cached.is_some());
        assert!(cached.unwrap().allowed);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let config = CacheConfig {
            default_ttl: 0, // 立即过期
            ..Default::default()
        };
        let cache = PermissionCache::new(config);
        let ctx = create_test_context("user1");
        let permission = create_test_permission();

        // 设置缓存
        cache
            .set(&ctx, &permission, CheckResult::allowed(), None)
            .await;

        // 由于TTL为0，应该立即过期
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert!(cache.get(&ctx, &permission).await.is_none());
    }

    #[tokio::test]
    async fn test_clear_user() {
        let cache = PermissionCache::new(CacheConfig::default());
        let ctx1 = create_test_context("user1");
        let ctx2 = create_test_context("user2");
        let permission = create_test_permission();

        cache
            .set(&ctx1, &permission, CheckResult::allowed(), None)
            .await;
        cache
            .set(&ctx2, &permission, CheckResult::allowed(), None)
            .await;

        assert_eq!(cache.size().await, 2);

        cache.clear_user("user1").await;

        assert_eq!(cache.size().await, 1);
        assert!(cache.get(&ctx1, &permission).await.is_none());
        assert!(cache.get(&ctx2, &permission).await.is_some());
    }

    #[tokio::test]
    async fn test_stats() {
        let cache = PermissionCache::new(CacheConfig::default());
        let ctx = create_test_context("user1");
        let permission = create_test_permission();

        // 初始统计
        let stats = cache.get_stats().await;
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);

        // 未命中
        cache.get(&ctx, &permission).await;
        let stats = cache.get_stats().await;
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.cache_misses, 1);

        // 设置并命中
        cache
            .set(&ctx, &permission, CheckResult::allowed(), None)
            .await;
        cache.get(&ctx, &permission).await;
        let stats = cache.get_stats().await;
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.cache_hits, 1);

        // 命中率
        assert!(stats.hit_rate() > 0.0);
    }

    #[tokio::test]
    async fn test_builder() {
        let cache = PermissionCacheBuilder::new()
            .with_ttl(600)
            .with_max_entries(5000)
            .enable_warmup()
            .build();

        assert_eq!(cache.config.default_ttl, 600);
        assert_eq!(cache.config.max_entries, 5000);
        assert!(cache.config.enable_warmup);
    }
}
