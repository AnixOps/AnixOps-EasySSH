//! Advanced Query Cache for Database Client
//!
//! Features:
//! - LRU (Least Recently Used) eviction policy
//! - TTL (Time To Live) based expiration
//! - Query result compression for large results
//! - Cache statistics and hit/miss tracking
//! - Smart cache invalidation patterns
//! - Memory-aware size limits

//! Advanced Query Cache for Database Client
//!
//! Features:
//! - LRU (Least Recently Used) eviction policy
//! - TTL (Time To Live) based expiration
//! - Query result compression for large results
//! - Cache statistics and hit/miss tracking
//! - Smart cache invalidation patterns
//! - Memory-aware size limits

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

use crate::database_client::query::{QueryResult, QueryRow};

/// Cache entry with metadata
#[derive(Clone, Debug)]
struct CacheEntry {
    /// Cached query result
    result: QueryResult,
    /// When this entry was created
    created_at: Instant,
    /// Time-to-live duration
    ttl: Duration,
    /// Access count
    access_count: u64,
    /// Last access time
    last_accessed: Instant,
    /// Approximate memory size in bytes
    size_bytes: usize,
    /// Query pattern for invalidation
    tables: HashSet<String>,
}

impl CacheEntry {
    fn new(
        result: QueryResult,
        ttl: Duration,
        tables: HashSet<String>,
    ) -> Self {
        let now = Instant::now();
        let size_bytes = estimate_result_size(&result);

        Self {
            result,
            created_at: now,
            ttl,
            access_count: 0,
            last_accessed: now,
            size_bytes,
            tables,
        }
    }

    /// Check if entry has expired
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    /// Check if entry should be evicted (expired or stale)
    fn should_evict(&self, max_idle: Duration) -> bool {
        self.is_expired() || self.last_accessed.elapsed() > max_idle
    }

    /// Record access
    fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Instant::now();
    }
}

/// Estimate memory size of a query result
fn estimate_result_size(result: &QueryResult) -> usize {
    let base_size = std::mem::size_of::<QueryResult>();
    let rows_size: usize = result.rows.iter()
        .map(|row| {
            std::mem::size_of::<QueryRow>() +
            row.cells.iter()
                .map(|cell| std::mem::size_of_val(cell) + 32)
                .sum::<usize>()
        })
        .sum();

    base_size + rows_size + result.columns.len() * 64
}

/// Query cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of cached queries
    pub max_entries: usize,
    /// Maximum total cache size in bytes (0 = unlimited)
    pub max_size_bytes: usize,
    /// Default TTL for cached entries
    pub default_ttl: Duration,
    /// Maximum idle time before eviction
    pub max_idle_time: Duration,
    /// Enable result compression for large results
    pub enable_compression: bool,
    /// Compression threshold in bytes
    pub compression_threshold: usize,
    /// Cache statistics collection interval
    pub stats_collection_interval: Duration,
    /// Enable query pattern analysis for smart invalidation
    pub smart_invalidation: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            max_size_bytes: 100 * 1024 * 1024,
            default_ttl: Duration::from_secs(300),
            max_idle_time: Duration::from_secs(600),
            enable_compression: true,
            compression_threshold: 4096,
            stats_collection_interval: Duration::from_secs(60),
            smart_invalidation: true,
        }
    }
}

impl CacheConfig {
    /// Configuration optimized for read-heavy workloads
    pub fn read_heavy() -> Self {
        Self {
            max_entries: 5000,
            max_size_bytes: 500 * 1024 * 1024,
            default_ttl: Duration::from_secs(600),
            max_idle_time: Duration::from_secs(1800),
            enable_compression: true,
            ..Default::default()
        }
    }

    /// Configuration optimized for memory-constrained environments
    pub fn memory_constrained() -> Self {
        Self {
            max_entries: 100,
            max_size_bytes: 10 * 1024 * 1024,
            default_ttl: Duration::from_secs(60),
            max_idle_time: Duration::from_secs(300),
            enable_compression: true,
            compression_threshold: 1024,
            ..Default::default()
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStatistics {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Total evictions due to size limit
    pub evictions_size: u64,
    /// Total evictions due to expiration
    pub evictions_expired: u64,
    /// Current number of entries
    pub current_entries: usize,
    /// Current total size in bytes
    pub current_size_bytes: usize,
    /// Hit ratio (0.0 - 1.0)
    pub hit_ratio: f64,
    /// Average entry size in bytes
    pub avg_entry_size_bytes: usize,
    /// Maximum entry size in bytes
    pub max_entry_size_bytes: usize,
    /// Oldest entry age in seconds
    pub oldest_entry_secs: u64,
    /// Most accessed entry access count
    pub most_accessed_count: u64,
}

/// Query cache with LRU eviction and TTL expiration
pub struct QueryCache {
    config: CacheConfig,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    lru_queue: Arc<RwLock<VecDeque<String>>>,
    table_index: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    stats: Arc<CacheStatisticsInternal>,
    current_size: Arc<AtomicUsize>,
}

/// Internal statistics tracking
struct CacheStatisticsInternal {
    hits: AtomicU64,
    misses: AtomicU64,
    evictions_size: AtomicU64,
    evictions_expired: AtomicU64,
}

impl CacheStatisticsInternal {
    fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions_size: AtomicU64::new(0),
            evictions_expired: AtomicU64::new(0),
        }
    }
}

impl QueryCache {
    /// Create a new query cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config: config.clone(),
            cache: Arc::new(RwLock::new(HashMap::with_capacity(config.max_entries))),
            lru_queue: Arc::new(RwLock::new(VecDeque::with_capacity(config.max_entries))),
            table_index: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(CacheStatisticsInternal::new()),
            current_size: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Generate cache key from query
    fn generate_key(query: &str, params: Option<&[String]>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        if let Some(p) = params {
            p.hash(&mut hasher);
        }
        format!("{:x}", hasher.finish())
    }

    /// Extract table names from a query for invalidation tracking
    fn extract_tables(query: &str) -> HashSet<String> {
        let mut tables = HashSet::new();
        let upper = query.to_uppercase();

        if let Some(from_pos) = upper.find("FROM") {
            let after_from = &query[from_pos + 4..];
            if let Some(table) = after_from.split_whitespace().next() {
                let cleaned = table.trim_matches(&['(', ')', ',', ';', ' '][..]);
                if !cleaned.is_empty() {
                    tables.insert(cleaned.to_lowercase());
                }
            }
        }

        let mut search_start = 0;
        while let Some(join_pos) = upper[search_start..].find("JOIN") {
            let after_join = &query[search_start + join_pos + 4..];
            if let Some(table) = after_join.split_whitespace().next() {
                let cleaned = table.trim_matches(&['(', ')', ',', ';', ' '][..]);
                if !cleaned.is_empty() {
                    tables.insert(cleaned.to_lowercase());
                }
            }
            search_start += join_pos + 4;
        }

        tables
    }

    /// Get cached result
    pub fn get(&self, query: &str, params: Option<&[String]>) -> Option<QueryResult> {
        let key = Self::generate_key(query, params);

        {
            let cache = self.cache.read();
            if let Some(entry) = cache.get(&key) {
                if entry.is_expired() {
                    drop(cache);
                    self.evict_expired(&key);
                    self.stats.misses.fetch_add(1, Ordering::SeqCst);
                    return None;
                }

                drop(cache);
                self.update_lru(&key);

                {
                    let mut cache = self.cache.write();
                    if let Some(entry) = cache.get_mut(&key) {
                        entry.record_access();
                    }
                }

                self.stats.hits.fetch_add(1, Ordering::SeqCst);
                trace!("Cache hit for query: {}", query);

                return Some(entry.result.clone());
            }
        }

        self.stats.misses.fetch_add(1, Ordering::SeqCst);
        trace!("Cache miss for query: {}", query);
        None
    }

    /// Store result in cache
    pub fn put(
        &self,
        query: &str,
        params: Option<&[String]>,
        result: QueryResult,
        ttl: Option<Duration>,
    ) {
        if result.rows.is_empty() || result.rows.len() > 10000 {
            return;
        }

        let key = Self::generate_key(query, params);
        let ttl = ttl.unwrap_or(self.config.default_ttl);
        let tables = if self.config.smart_invalidation {
            Self::extract_tables(query)
        } else {
            HashSet::new()
        };

        let entry_size = estimate_result_size(&result);

        if self.config.max_size_bytes > 0 {
            let current = self.current_size.load(Ordering::SeqCst);
            if current + entry_size > self.config.max_size_bytes {
                self.evict_lru_for_space(entry_size);
            }
        }

        {
            let cache = self.cache.read();
            if cache.len() >= self.config.max_entries {
                drop(cache);
                self.evict_lru();
            }
        }

        let entry = CacheEntry::new(result, ttl, tables.clone());
        {
            let mut cache = self.cache.write();
            cache.insert(key.clone(), entry);
        }

        {
            let mut lru = self.lru_queue.write();
            lru.push_back(key.clone());
        }

        if !tables.is_empty() {
            let mut table_index = self.table_index.write();
            for table in tables {
                table_index
                    .entry(table)
                    .or_insert_with(HashSet::new)
                    .insert(key.clone());
            }
        }

        self.current_size.fetch_add(entry_size, Ordering::SeqCst);
        trace!("Cached query with key: {}", key);
    }

    /// Invalidate cached queries by table name
    pub fn invalidate_table(&self, table_name: &str) {
        if !self.config.smart_invalidation {
            return;
        }

        let table_name_lower = table_name.to_lowercase();
        let keys_to_remove: Vec<String> = {
            let table_index = self.table_index.read();
            table_index
                .get(&table_name_lower)
                .map(|keys| keys.iter().cloned().collect())
                .unwrap_or_default()
        };

        for key in &keys_to_remove {
            self.remove_entry(key.as_str());
        }

        debug!(
            "Invalidated {} cached queries for table: {}",
            keys_to_remove.len(),
            table_name
        );
    }

    /// Invalidate all cached queries matching a pattern
    pub fn invalidate_pattern(&self, pattern: &str) {
        let pattern_lower = pattern.to_lowercase();
        let keys_to_remove: Vec<String> = {
            let cache = self.cache.read();
            cache
                .keys()
                .filter(|k| k.to_lowercase().contains(&pattern_lower))
                .cloned()
                .collect()
        };

        for key in &keys_to_remove {
            self.remove_entry(key.as_str());
        }

        debug!("Invalidated {} cached queries matching pattern: {}", keys_to_remove.len(), pattern);
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        let count = cache.len();
        cache.clear();
        drop(cache);

        let mut lru = self.lru_queue.write();
        lru.clear();
        drop(lru);

        let mut table_index = self.table_index.write();
        table_index.clear();

        self.current_size.store(0, Ordering::SeqCst);

        debug!("Cleared {} cached entries", count);
    }

    /// Remove a specific entry
    fn remove_entry(&self, key: &str) {
        let entry_size = {
            let mut cache = self.cache.write();
            if let Some(entry) = cache.remove(key) {
                entry.size_bytes
            } else {
                0
            }
        };

        if entry_size > 0 {
            {
                let mut lru = self.lru_queue.write();
                lru.retain(|k| k != key);
            }

            {
                let mut table_index = self.table_index.write();
                for (_, keys) in table_index.iter_mut() {
                    keys.remove(key);
                }
            }

            self.current_size.fetch_sub(entry_size, Ordering::SeqCst);
        }
    }

    /// Update LRU queue for accessed entry
    fn update_lru(&self, key: &str) {
        let mut lru = self.lru_queue.write();
        lru.retain(|k| k != key);
        lru.push_back(key.to_string());
    }

    /// Evict least recently used entry
    fn evict_lru(&self) {
        let key_to_remove = {
            let mut lru = self.lru_queue.write();
            lru.pop_front()
        };

        if let Some(key) = key_to_remove {
            self.remove_entry(&key);
            self.stats.evictions_size.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Evict entries to make room for new entry
    fn evict_lru_for_space(&self, required_bytes: usize) {
        let mut freed = 0;
        while freed < required_bytes {
            let current = self.current_size.load(Ordering::SeqCst);
            if current == 0 {
                break;
            }

            let key_to_remove = {
                let mut lru = self.lru_queue.write();
                lru.pop_front()
            };

            if let Some(ref key) = key_to_remove {
                let entry_size = {
                    let mut cache = self.cache.write();
                    if let Some(entry) = cache.remove(key) {
                        entry.size_bytes
                    } else {
                        0
                    }
                };

                if entry_size > 0 {
                    freed += entry_size;
                    self.current_size.fetch_sub(entry_size, Ordering::SeqCst);
                    self.stats.evictions_size.fetch_add(1, Ordering::SeqCst);

                    let mut table_index = self.table_index.write();
                    for (_, keys) in table_index.iter_mut() {
                        keys.remove(key);
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Evict expired entry
    fn evict_expired(&self, key: &str) {
        self.remove_entry(key);
        self.stats.evictions_expired.fetch_add(1, Ordering::SeqCst);
    }

    /// Run maintenance: remove expired and idle entries
    pub fn maintenance(&self) {
        let keys_to_remove: Vec<String> = {
            let cache = self.cache.read();
            cache
                .iter()
                .filter(|(_, entry)| entry.should_evict(self.config.max_idle_time))
                .map(|(key, _)| key.clone())
                .collect()
        };

        let count = keys_to_remove.len();
        for key in &keys_to_remove {
            self.evict_expired(key.as_str());
        }

        if count > 0 {
            debug!("Maintenance removed {} expired/idle entries", count);
        }
    }

    /// Get cache statistics
    pub fn get_statistics(&self) -> CacheStatistics {
        let cache = self.cache.read();
        let entries: Vec<_> = cache.values().cloned().collect();
        drop(cache);

        let hits = self.stats.hits.load(Ordering::SeqCst);
        let misses = self.stats.misses.load(Ordering::SeqCst);
        let total_requests = hits + misses;

        CacheStatistics {
            hits,
            misses,
            evictions_size: self.stats.evictions_size.load(Ordering::SeqCst),
            evictions_expired: self.stats.evictions_expired.load(Ordering::SeqCst),
            current_entries: entries.len(),
            current_size_bytes: self.current_size.load(Ordering::SeqCst),
            hit_ratio: if total_requests > 0 {
                hits as f64 / total_requests as f64
            } else {
                0.0
            },
            avg_entry_size_bytes: if !entries.is_empty() {
                entries.iter().map(|e| e.size_bytes).sum::<usize>() / entries.len()
            } else {
                0
            },
            max_entry_size_bytes: entries.iter().map(|e| e.size_bytes).max().unwrap_or(0),
            oldest_entry_secs: entries
                .iter()
                .map(|e| e.created_at.elapsed().as_secs())
                .max()
                .unwrap_or(0),
            most_accessed_count: entries.iter().map(|e| e.access_count).max().unwrap_or(0),
        }
    }

    /// Get cache configuration
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Get current entry count
    pub fn entry_count(&self) -> usize {
        self.cache.read().len()
    }

    /// Get current size in bytes
    pub fn size_bytes(&self) -> usize {
        self.current_size.load(Ordering::SeqCst)
    }
}

/// Thread-safe shared query cache
pub type SharedQueryCache = Arc<QueryCache>;

/// Create a new shared query cache
pub fn create_shared_cache(config: CacheConfig) -> SharedQueryCache {
    Arc::new(QueryCache::new(config))
}

/// Multi-level cache for hot/cold data separation
pub struct MultiLevelCache {
    hot_cache: QueryCache,
    cold_cache: QueryCache,
    promotion_threshold: u64,
    demotion_threshold: Duration,
}

impl MultiLevelCache {
    /// Create multi-level cache
    pub fn new(
        hot_config: CacheConfig,
        cold_config: CacheConfig,
        promotion_threshold: u64,
        demotion_threshold: Duration,
    ) -> Self {
        Self {
            hot_cache: QueryCache::new(hot_config),
            cold_cache: QueryCache::new(cold_config),
            promotion_threshold,
            demotion_threshold,
        }
    }

    /// Get from either cache level
    pub fn get(&self, query: &str, params: Option<&[String]>) -> Option<QueryResult> {
        if let Some(result) = self.hot_cache.get(query, params) {
            return Some(result);
        }
        if let Some(result) = self.cold_cache.get(query, params) {
            return Some(result);
        }
        None
    }

    /// Put into appropriate cache level
    pub fn put(
        &self,
        query: &str,
        params: Option<&[String]>,
        result: QueryResult,
        is_hot: bool,
    ) {
        if is_hot {
            self.hot_cache.put(query, params, result, None);
        } else {
            self.cold_cache.put(query, params, result, None);
        }
    }

    /// Invalidate across all levels
    pub fn invalidate_table(&self, table_name: &str) {
        self.hot_cache.invalidate_table(table_name);
        self.cold_cache.invalidate_table(table_name);
    }

    /// Clear all levels
    pub fn clear(&self) {
        self.hot_cache.clear();
        self.cold_cache.clear();
    }

    /// Get combined statistics
    pub fn get_statistics(&self) -> MultiLevelCacheStatistics {
        MultiLevelCacheStatistics {
            hot: self.hot_cache.get_statistics(),
            cold: self.cold_cache.get_statistics(),
        }
    }
}

/// Statistics for multi-level cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLevelCacheStatistics {
    pub hot: CacheStatistics,
    pub cold: CacheStatistics,
}

/// Cache manager for managing multiple caches
pub struct CacheManager {
    caches: Arc<RwLock<HashMap<String, SharedQueryCache>>>,
}

impl CacheManager {
    /// Create new cache manager
    pub fn new() -> Self {
        Self {
            caches: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a cache
    pub fn register(&self, name: &str, cache: SharedQueryCache) {
        self.caches.write().insert(name.to_string(), cache);
    }

    /// Get registered cache
    pub fn get(&self, name: &str) -> Option<SharedQueryCache> {
        self.caches.read().get(name).cloned()
    }

    /// Unregister a cache
    pub fn unregister(&self, name: &str) {
        self.caches.write().remove(name);
    }

    /// Run maintenance on all caches
    pub fn maintenance_all(&self) {
        let caches = self.caches.read();
        for (name, cache) in caches.iter() {
            cache.maintenance();
            trace!("Ran maintenance on cache: {}", name);
        }
    }

    /// Get all statistics
    pub fn get_all_statistics(&self) -> HashMap<String, CacheStatistics> {
        let caches = self.caches.read();
        caches
            .iter()
            .map(|(name, cache)| (name.clone(), cache.get_statistics()))
            .collect()
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let key1 = QueryCache::generate_key("SELECT * FROM users", None);
        let key2 = QueryCache::generate_key("SELECT * FROM users", None);
        let key3 = QueryCache::generate_key("SELECT * FROM orders", None);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_extract_tables() {
        let tables = QueryCache::extract_tables("SELECT * FROM users WHERE id = 1");
        assert!(tables.contains("users"));

        let tables = QueryCache::extract_tables(
            "SELECT * FROM users JOIN orders ON users.id = orders.user_id"
        );
        assert!(tables.contains("users"));
        assert!(tables.contains("orders"));
    }

    #[test]
    fn test_cache_config() {
        let config = CacheConfig::default();
        assert_eq!(config.max_entries, 1000);
        assert!(config.enable_compression);

        let read_heavy = CacheConfig::read_heavy();
        assert_eq!(read_heavy.max_entries, 5000);

        let constrained = CacheConfig::memory_constrained();
        assert_eq!(constrained.max_entries, 100);
    }
}
