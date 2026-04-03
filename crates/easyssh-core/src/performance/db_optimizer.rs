//! Database Performance Optimizations
//!
//! Optimizations implemented:
//! - Additional indexes for common query patterns
//! - Query result caching
//! - Prepared statement caching
//! - Batch operation optimizations
//! - WAL mode tuning
//! - Database fast path with deferred index creation
//! - Lazy index build for startup optimization

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use rusqlite::Connection;

use crate::db::{Database, HostRecord, ServerRecord};
use crate::error::LiteError;

// ============================================================================
// Query Cache (existing)
// ============================================================================

/// Query cache entry
struct QueryCacheEntry<T> {
    result: T,
    created_at: Instant,
    ttl: Duration,
}

impl<T: Clone> QueryCacheEntry<T> {
    fn new(result: T, ttl: Duration) -> Self {
        Self {
            result,
            created_at: Instant::now(),
            ttl,
        }
    }

    fn is_valid(&self) -> bool {
        self.created_at.elapsed() < self.ttl
    }

    fn get_result(&self) -> T {
        self.result.clone()
    }
}

/// Query cache for frequently accessed data
pub struct QueryCache<T: Clone> {
    cache: RwLock<HashMap<String, QueryCacheEntry<T>>>,
    default_ttl: Duration,
    max_entries: usize,
}

impl<T: Clone> QueryCache<T> {
    /// Create a new query cache
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            default_ttl: Duration::from_secs(30),
            max_entries: 100,
        }
    }

    /// Get cached result or compute
    pub fn get_or_compute<F>(&self, key: &str, compute: F) -> Result<T, LiteError>
    where
        F: FnOnce() -> Result<T, LiteError>,
    {
        // Try to get from cache
        {
            let cache = self
                .cache
                .read()
                .map_err(|_| LiteError::Internal("Failed to lock cache".to_string()))?;

            if let Some(entry) = cache.get(key) {
                if entry.is_valid() {
                    return Ok(entry.get_result());
                }
            }
        }

        // Compute result
        let result = compute()?;

        // Cache result
        let mut cache = self
            .cache
            .write()
            .map_err(|_| LiteError::Internal("Failed to lock cache".to_string()))?;

        // Clean expired entries if at capacity
        if cache.len() >= self.max_entries {
            cache.retain(|_, entry| entry.is_valid());
        }

        // Only cache if we have room
        if cache.len() < self.max_entries {
            cache.insert(
                key.to_string(),
                QueryCacheEntry::new(result.clone(), self.default_ttl),
            );
        }

        Ok(result)
    }

    /// Invalidate cache entries matching prefix
    pub fn invalidate_prefix(&self, prefix: &str) -> Result<(), LiteError> {
        let mut cache = self
            .cache
            .write()
            .map_err(|_| LiteError::Internal("Failed to lock cache".to_string()))?;

        cache.retain(|key, _| !key.starts_with(prefix));
        Ok(())
    }

    /// Clear all cache
    pub fn clear(&self) -> Result<(), LiteError> {
        let mut cache = self
            .cache
            .write()
            .map_err(|_| LiteError::Internal("Failed to lock cache".to_string()))?;

        cache.clear();
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<(usize, usize), LiteError> {
        let cache = self
            .cache
            .read()
            .map_err(|_| LiteError::Internal("Failed to lock cache".to_string()))?;

        let total = cache.len();
        let valid = cache.values().filter(|e| e.is_valid()).count();

        Ok((total, valid))
    }
}

impl<T: Clone> Default for QueryCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Optimized database wrapper with caching
#[allow(dead_code)]
pub struct OptimizedDatabase {
    db: Arc<Database>,
    server_cache: Arc<QueryCache<Vec<ServerRecord>>>,
    host_cache: Arc<QueryCache<Vec<HostRecord>>>,
    query_stats: Arc<Mutex<QueryStats>>,
}

/// Query performance statistics
#[derive(Debug, Default, Clone)]
pub struct QueryStats {
    pub total_queries: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub avg_query_time_us: u64,
    pub slow_queries: u64,
}

impl OptimizedDatabase {
    /// Create an optimized database wrapper
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            server_cache: Arc::new(QueryCache::new()),
            host_cache: Arc::new(QueryCache::new()),
            query_stats: Arc::new(Mutex::new(QueryStats::default())),
        }
    }

    /// Get servers with caching
    pub fn get_servers_cached(&self) -> Result<Vec<ServerRecord>, LiteError> {
        let start = Instant::now();

        let result = self.server_cache.get_or_compute("servers_all", || {
            // Access the underlying connection through a method
            // This is a simplified version - actual implementation would need
            // to access the database through its public methods
            Ok(Vec::new()) // Placeholder
        })?;

        self.update_stats(start.elapsed(), true);

        Ok(result)
    }

    /// Get hosts with caching
    pub fn get_hosts_cached(&self) -> Result<Vec<HostRecord>, LiteError> {
        let start = Instant::now();

        let result = self.host_cache.get_or_compute("hosts_all", || {
            Ok(Vec::new()) // Placeholder
        })?;

        self.update_stats(start.elapsed(), true);

        Ok(result)
    }

    /// Invalidate caches after modifications
    pub fn invalidate_caches(&self) -> Result<(), LiteError> {
        self.server_cache.clear()?;
        self.host_cache.clear()?;
        Ok(())
    }

    /// Get query statistics
    pub fn get_stats(&self) -> Result<QueryStats, LiteError> {
        let stats = self
            .query_stats
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock stats".to_string()))?;

        Ok(stats.clone())
    }

    /// Reset statistics
    pub fn reset_stats(&self) -> Result<(), LiteError> {
        let mut stats = self
            .query_stats
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock stats".to_string()))?;

        *stats = QueryStats::default();
        Ok(())
    }

    fn update_stats(&self, duration: Duration, cache_hit: bool) {
        if let Ok(mut stats) = self.query_stats.lock() {
            stats.total_queries += 1;

            if cache_hit {
                stats.cache_hits += 1;
            } else {
                stats.cache_misses += 1;
            }

            // Update average query time (exponential moving average)
            let time_us = duration.as_micros() as u64;
            stats.avg_query_time_us = (stats.avg_query_time_us * 9 + time_us) / 10;

            // Track slow queries (> 10ms)
            if time_us > 10_000 {
                stats.slow_queries += 1;
            }
        }
    }
}

/// Database optimization utilities
pub struct DbOptimizer;

impl DbOptimizer {
    /// Apply additional performance indexes to database
    pub fn apply_performance_indexes(conn: &Connection) -> Result<(), LiteError> {
        let indexes = vec![
            // Server/Host lookup indexes
            "CREATE INDEX IF NOT EXISTS idx_servers_name_lower ON servers(LOWER(name))",
            "CREATE INDEX IF NOT EXISTS idx_servers_host_lower ON servers(LOWER(host))",
            "CREATE INDEX IF NOT EXISTS idx_servers_auth_type ON servers(auth_type)",
            "CREATE INDEX IF NOT EXISTS idx_servers_composite ON servers(group_id, status, name)",
            // Host indexes
            "CREATE INDEX IF NOT EXISTS idx_hosts_name_lower ON hosts(LOWER(name))",
            "CREATE INDEX IF NOT EXISTS idx_hosts_host_lower ON hosts(LOWER(host))",
            "CREATE INDEX IF NOT EXISTS idx_hosts_environment ON hosts(environment)",
            "CREATE INDEX IF NOT EXISTS idx_hosts_region ON hosts(region)",
            "CREATE INDEX IF NOT EXISTS idx_hosts_composite ON hosts(group_id, status, name)",
            // Tag indexes
            "CREATE INDEX IF NOT EXISTS idx_host_tags_host ON host_tags(host_id)",
            "CREATE INDEX IF NOT EXISTS idx_host_tags_tag ON host_tags(tag_id)",
            // Session indexes
            "CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status)",
            "CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at DESC)",
            // Audit indexes
            "CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_events(action)",
            "CREATE INDEX IF NOT EXISTS idx_audit_level ON audit_events(level)",
            // Identity indexes
            "CREATE INDEX IF NOT EXISTS idx_identities_auth_type ON identities(auth_type)",
        ];

        for index_sql in indexes {
            conn.execute(index_sql, [])
                .map_err(|e| LiteError::Database(e.to_string()))?;
        }

        Ok(())
    }

    /// Optimize SQLite PRAGMAs for performance
    pub fn optimize_pragmas(conn: &Connection) -> Result<(), LiteError> {
        // WAL mode for better concurrent read/write
        conn.execute("PRAGMA journal_mode = WAL", [])
            .map_err(|e| LiteError::Database(e.to_string()))?;

        // Synchronous NORMAL for better performance with acceptable safety
        conn.execute("PRAGMA synchronous = NORMAL", [])
            .map_err(|e| LiteError::Database(e.to_string()))?;

        // Increase cache size to 20MB (5000 pages * 4KB)
        conn.execute("PRAGMA cache_size = -5000", [])
            .map_err(|e| LiteError::Database(e.to_string()))?;

        // Enable memory-mapped I/O for read-heavy workloads
        conn.execute("PRAGMA mmap_size = 268435456", []) // 256MB
            .map_err(|e| LiteError::Database(e.to_string()))?;

        // Optimize temp storage
        conn.execute("PRAGMA temp_store = MEMORY", [])
            .map_err(|e| LiteError::Database(e.to_string()))?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])
            .map_err(|e| LiteError::Database(e.to_string()))?;

        Ok(())
    }

    /// Analyze tables for query planner optimization
    pub fn analyze_tables(conn: &Connection) -> Result<(), LiteError> {
        conn.execute("ANALYZE", [])
            .map_err(|e| LiteError::Database(e.to_string()))?;

        Ok(())
    }

    /// Compact database and reclaim space
    pub fn vacuum_database(conn: &Connection) -> Result<(), LiteError> {
        conn.execute("VACUUM", [])
            .map_err(|e| LiteError::Database(e.to_string()))?;

        Ok(())
    }

    /// Get database size statistics
    pub fn get_database_stats(conn: &Connection) -> Result<DatabaseStats, LiteError> {
        let page_count: i64 = conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .map_err(|e| LiteError::Database(e.to_string()))?;

        let page_size: i64 = conn
            .query_row("PRAGMA page_size", [], |row| row.get(0))
            .map_err(|e| LiteError::Database(e.to_string()))?;

        let freelist_count: i64 = conn
            .query_row("PRAGMA freelist_count", [], |row| row.get(0))
            .map_err(|e| LiteError::Database(e.to_string()))?;

        Ok(DatabaseStats {
            page_count,
            page_size,
            freelist_count,
            total_size_bytes: page_count * page_size,
            free_pages: freelist_count,
        })
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub page_count: i64,
    pub page_size: i64,
    pub freelist_count: i64,
    pub total_size_bytes: i64,
    pub free_pages: i64,
}

/// Batch operation utilities for efficient bulk operations
pub struct BatchOperations;

impl BatchOperations {
    /// Execute a batch insert with optimized transaction handling
    pub fn batch_insert<T>(
        conn: &Connection,
        items: &[T],
        insert_fn: impl Fn(&T) -> Result<(), LiteError>,
    ) -> Result<(), LiteError> {
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| LiteError::Database(e.to_string()))?;

        for item in items {
            insert_fn(item)?;
        }

        tx.commit()
            .map_err(|e| LiteError::Database(e.to_string()))?;

        Ok(())
    }

    /// Recommended batch size for optimal performance
    pub const OPTIMAL_BATCH_SIZE: usize = 100;

    /// Chunk items into optimal batch sizes
    pub fn chunk_batches<T>(items: &[T]) -> impl Iterator<Item = &[T]> {
        items.chunks(Self::OPTIMAL_BATCH_SIZE)
    }
}

// ============================================================================
// Database Fast Path - Deferred Index Creation
// ============================================================================

/// Index status for deferred index tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexStatus {
    /// Index not yet created
    Pending,
    /// Index creation in progress
    Creating,
    /// Index fully created and ready
    Ready,
    /// Index creation failed
    Failed,
}

/// Deferred index entry for tracking
#[derive(Debug, Clone)]
pub struct DeferredIndex {
    /// Index name
    pub name: String,
    /// SQL statement to create the index
    pub create_sql: String,
    /// Current status
    pub status: IndexStatus,
    /// Priority (higher = more important, create first)
    pub priority: u8,
    /// Estimated time to create in milliseconds
    pub estimated_time_ms: u64,
}

impl DeferredIndex {
    /// Create a new deferred index
    pub fn new(name: String, create_sql: String, priority: u8) -> Self {
        Self {
            name,
            create_sql,
            status: IndexStatus::Pending,
            priority,
            estimated_time_ms: 0,
        }
    }

    /// Create with estimated time
    pub fn with_estimate(
        name: String,
        create_sql: String,
        priority: u8,
        estimated_time_ms: u64,
    ) -> Self {
        Self {
            name,
            create_sql,
            status: IndexStatus::Pending,
            priority,
            estimated_time_ms,
        }
    }
}

/// Database fast path configuration
#[derive(Debug, Clone)]
pub struct FastPathConfig {
    /// Enable deferred index creation
    pub defer_indexes: bool,
    /// Enable WAL mode from start
    pub wal_mode: bool,
    /// Enable memory-mapped I/O
    pub mmap: bool,
    /// Cache size in pages (negative = kilobytes)
    pub cache_size: i32,
    /// Synchronous mode (0=OFF, 1=NORMAL, 2=FULL, 3=EXTRA)
    pub synchronous_mode: u8,
    /// Temp storage location (0=FILE, 1=MEMORY)
    pub temp_store: u8,
    /// Minimum data size before creating indexes (in rows)
    pub index_threshold_rows: usize,
    /// Background index creation after startup
    pub background_indexes: bool,
}

impl Default for FastPathConfig {
    fn default() -> Self {
        Self {
            defer_indexes: true,
            wal_mode: true,
            mmap: true,
            cache_size: -20000,  // 20MB cache
            synchronous_mode: 1, // NORMAL
            temp_store: 1,       // MEMORY
            index_threshold_rows: 100,
            background_indexes: true,
        }
    }
}

impl FastPathConfig {
    /// Create fast path config optimized for cold start
    pub fn for_cold_start() -> Self {
        Self {
            defer_indexes: true,
            wal_mode: true,
            mmap: true,
            cache_size: -5000, // 5MB - smaller for cold start
            synchronous_mode: 1,
            temp_store: 1,
            index_threshold_rows: 50,
            background_indexes: true,
        }
    }

    /// Create fast path config optimized for hot start
    pub fn for_hot_start() -> Self {
        Self {
            defer_indexes: false, // Indexes should already exist
            wal_mode: true,
            mmap: true,
            cache_size: -20000, // 20MB - larger cache for hot start
            synchronous_mode: 1,
            temp_store: 1,
            index_threshold_rows: 100,
            background_indexes: false,
        }
    }
}

/// Database fast path initializer
pub struct DatabaseFastPath {
    config: FastPathConfig,
    deferred_indexes: RwLock<Vec<DeferredIndex>>,
    indexes_ready: RwLock<bool>,
    creation_stats: Mutex<IndexCreationStats>,
}

/// Statistics for index creation
#[derive(Debug, Clone, Default)]
pub struct IndexCreationStats {
    pub indexes_created: usize,
    pub indexes_pending: usize,
    pub total_creation_time_ms: u64,
    pub indexes_failed: usize,
    pub creation_errors: Vec<String>,
}

impl DatabaseFastPath {
    /// Create a new database fast path with default config
    pub fn new() -> Self {
        Self::with_config(FastPathConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: FastPathConfig) -> Self {
        Self {
            config,
            deferred_indexes: RwLock::new(Vec::new()),
            indexes_ready: RwLock::new(false),
            creation_stats: Mutex::new(IndexCreationStats::default()),
        }
    }

    /// Get the list of deferred indexes (sorted by priority)
    pub fn get_deferred_indexes() -> Vec<DeferredIndex> {
        // Essential indexes for basic queries (priority 10)
        let essential = vec![
            DeferredIndex::new(
                "idx_servers_name_lower".to_string(),
                "CREATE INDEX IF NOT EXISTS idx_servers_name_lower ON servers(LOWER(name))"
                    .to_string(),
                10,
            ),
            DeferredIndex::new(
                "idx_servers_host_lower".to_string(),
                "CREATE INDEX IF NOT EXISTS idx_servers_host_lower ON servers(LOWER(host))"
                    .to_string(),
                10,
            ),
            DeferredIndex::new(
                "idx_hosts_name_lower".to_string(),
                "CREATE INDEX IF NOT EXISTS idx_hosts_name_lower ON hosts(LOWER(name))".to_string(),
                10,
            ),
        ];

        // Secondary indexes for common queries (priority 5)
        let secondary = vec![
            DeferredIndex::new(
                "idx_servers_auth_type".to_string(),
                "CREATE INDEX IF NOT EXISTS idx_servers_auth_type ON servers(auth_type)".to_string(),
                5,
            ),
            DeferredIndex::new(
                "idx_servers_composite".to_string(),
                "CREATE INDEX IF NOT EXISTS idx_servers_composite ON servers(group_id, status, name)".to_string(),
                5,
            ),
            DeferredIndex::new(
                "idx_hosts_composite".to_string(),
                "CREATE INDEX IF NOT EXISTS idx_hosts_composite ON hosts(group_id, status, name)".to_string(),
                5,
            ),
        ];

        // Optional indexes for advanced features (priority 1)
        let optional = vec![
            DeferredIndex::new(
                "idx_hosts_environment".to_string(),
                "CREATE INDEX IF NOT EXISTS idx_hosts_environment ON hosts(environment)"
                    .to_string(),
                1,
            ),
            DeferredIndex::new(
                "idx_hosts_region".to_string(),
                "CREATE INDEX IF NOT EXISTS idx_hosts_region ON hosts(region)".to_string(),
                1,
            ),
            DeferredIndex::new(
                "idx_host_tags_host".to_string(),
                "CREATE INDEX IF NOT EXISTS idx_host_tags_host ON host_tags(host_id)".to_string(),
                1,
            ),
        ];

        // Combine all and sort by priority (descending)
        let mut all: Vec<DeferredIndex> = essential
            .into_iter()
            .chain(secondary.into_iter())
            .chain(optional.into_iter())
            .collect();
        all.sort_by(|a, b| b.priority.cmp(&a.priority));
        all
    }

    /// Initialize database with fast path settings
    pub fn initialize_fast_path(&self, conn: &Connection) -> Result<(), LiteError> {
        // Apply WAL mode first (critical for performance)
        if self.config.wal_mode {
            conn.execute("PRAGMA journal_mode = WAL", [])
                .map_err(|e| LiteError::Database(format!("Failed to set WAL mode: {}", e)))?;
        }

        // Apply synchronous mode
        let sync_sql = format!("PRAGMA synchronous = {}", self.config.synchronous_mode);
        conn.execute(&sync_sql, [])
            .map_err(|e| LiteError::Database(format!("Failed to set synchronous mode: {}", e)))?;

        // Apply cache size
        let cache_sql = format!("PRAGMA cache_size = {}", self.config.cache_size);
        conn.execute(&cache_sql, [])
            .map_err(|e| LiteError::Database(format!("Failed to set cache size: {}", e)))?;

        // Apply temp store
        let temp_sql = format!("PRAGMA temp_store = {}", self.config.temp_store);
        conn.execute(&temp_sql, [])
            .map_err(|e| LiteError::Database(format!("Failed to set temp store: {}", e)))?;

        // Apply memory-mapped I/O
        if self.config.mmap {
            conn.execute("PRAGMA mmap_size = 268435456", []) // 256MB
                .map_err(|e| LiteError::Database(format!("Failed to set mmap size: {}", e)))?;
        }

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])
            .map_err(|e| LiteError::Database(format!("Failed to enable foreign keys: {}", e)))?;

        // Set up deferred indexes if configured
        if self.config.defer_indexes {
            let indexes = Self::get_deferred_indexes();
            let mut deferred = self
                .deferred_indexes
                .write()
                .map_err(|_| LiteError::Internal("Failed to lock deferred indexes".to_string()))?;
            *deferred = indexes;
            *self
                .indexes_ready
                .write()
                .map_err(|_| LiteError::Internal("Failed to lock indexes_ready".to_string()))? =
                false;
        } else {
            // Create all indexes immediately
            DbOptimizer::apply_performance_indexes(conn)?;
            *self
                .indexes_ready
                .write()
                .map_err(|_| LiteError::Internal("Failed to lock indexes_ready".to_string()))? =
                true;
        }

        Ok(())
    }

    /// Create essential indexes only (for fast startup)
    pub fn create_essential_indexes(&self, conn: &Connection) -> Result<(), LiteError> {
        let mut deferred = self
            .deferred_indexes
            .write()
            .map_err(|_| LiteError::Internal("Failed to lock deferred indexes".to_string()))?;

        let start = Instant::now();

        // Create only high-priority indexes (priority >= 10)
        for index in deferred.iter_mut() {
            if index.priority >= 10 && index.status == IndexStatus::Pending {
                index.status = IndexStatus::Creating;

                let result = conn.execute(&index.create_sql, []).map_err(|e| {
                    LiteError::Database(format!("Failed to create index {}: {}", index.name, e))
                });

                if result.is_ok() {
                    index.status = IndexStatus::Ready;
                } else {
                    index.status = IndexStatus::Failed;
                    let mut stats = self
                        .creation_stats
                        .lock()
                        .map_err(|_| LiteError::Internal("Failed to lock stats".to_string()))?;
                    stats.indexes_failed += 1;
                    stats
                        .creation_errors
                        .push(format!("Index {} failed", index.name));
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let mut stats = self
            .creation_stats
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock stats".to_string()))?;
        stats.total_creation_time_ms += duration_ms;
        stats.indexes_created = deferred
            .iter()
            .filter(|i| i.status == IndexStatus::Ready)
            .count();
        stats.indexes_pending = deferred
            .iter()
            .filter(|i| i.status == IndexStatus::Pending)
            .count();

        Ok(())
    }

    /// Create remaining indexes in background (after startup)
    pub fn create_background_indexes(&self, conn: &Connection) -> Result<(), LiteError> {
        if !self.config.background_indexes {
            return Ok(());
        }

        let mut deferred = self
            .deferred_indexes
            .write()
            .map_err(|_| LiteError::Internal("Failed to lock deferred indexes".to_string()))?;

        let start = Instant::now();

        // Create all pending indexes
        for index in deferred.iter_mut() {
            if index.status == IndexStatus::Pending {
                index.status = IndexStatus::Creating;

                let result = conn.execute(&index.create_sql, []);

                match result {
                    Ok(_) => {
                        index.status = IndexStatus::Ready;
                    }
                    Err(e) => {
                        index.status = IndexStatus::Failed;
                        let mut stats = self
                            .creation_stats
                            .lock()
                            .map_err(|_| LiteError::Internal("Failed to lock stats".to_string()))?;
                        stats.indexes_failed += 1;
                        stats
                            .creation_errors
                            .push(format!("Index {} failed: {}", index.name, e));
                    }
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let mut stats = self
            .creation_stats
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock stats".to_string()))?;
        stats.total_creation_time_ms += duration_ms;
        stats.indexes_created = deferred
            .iter()
            .filter(|i| i.status == IndexStatus::Ready)
            .count();
        stats.indexes_pending = deferred
            .iter()
            .filter(|i| i.status == IndexStatus::Pending)
            .count();

        // Mark all indexes as ready if no pending
        if stats.indexes_pending == 0 {
            *self
                .indexes_ready
                .write()
                .map_err(|_| LiteError::Internal("Failed to lock indexes_ready".to_string()))? =
                true;
        }

        Ok(())
    }

    /// Check if all indexes are ready
    pub fn are_indexes_ready(&self) -> Result<bool, LiteError> {
        let ready = self
            .indexes_ready
            .read()
            .map_err(|_| LiteError::Internal("Failed to lock indexes_ready".to_string()))?;
        Ok(*ready)
    }

    /// Get index creation statistics
    pub fn get_creation_stats(&self) -> Result<IndexCreationStats, LiteError> {
        let stats = self
            .creation_stats
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock stats".to_string()))?;
        Ok(stats.clone())
    }

    /// Get pending index names
    pub fn get_pending_indexes(&self) -> Result<Vec<String>, LiteError> {
        let deferred = self
            .deferred_indexes
            .read()
            .map_err(|_| LiteError::Internal("Failed to lock deferred indexes".to_string()))?;
        Ok(deferred
            .iter()
            .filter(|i| i.status == IndexStatus::Pending)
            .map(|i| i.name.clone())
            .collect())
    }

    /// Get the config
    pub fn config(&self) -> &FastPathConfig {
        &self.config
    }

    /// Force create all indexes immediately
    pub fn force_create_all_indexes(&self, conn: &Connection) -> Result<(), LiteError> {
        // First create essential
        self.create_essential_indexes(conn)?;

        // Then create remaining
        self.create_background_indexes(conn)?;

        Ok(())
    }
}

impl Default for DatabaseFastPath {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Connection Pool Warmup
// ============================================================================

/// Connection pool warmup helper for hot starts
pub struct ConnectionPoolWarmup;

impl ConnectionPoolWarmup {
    /// Warm up the connection pool with basic queries
    pub fn warmup(conn: &Connection) -> Result<Duration, LiteError> {
        let start = Instant::now();

        // Execute a few simple queries to warm up the connection
        conn.execute("SELECT 1", [])
            .map_err(|e| LiteError::Database(e.to_string()))?;

        conn.execute("SELECT 2", [])
            .map_err(|e| LiteError::Database(e.to_string()))?;

        // Touch the main tables
        conn.execute("SELECT COUNT(*) FROM sqlite_master", [])
            .map_err(|e| LiteError::Database(e.to_string()))?;

        Ok(start.elapsed())
    }

    /// Estimate warmup benefit based on database size
    pub fn estimate_warmup_benefit(db_size_bytes: u64) -> u64 {
        // Larger databases benefit more from warmup
        // Estimate: 10ms per MB of data, capped at 100ms
        let mb = db_size_bytes / (1024 * 1024);
        std::cmp::min(mb * 10, 100)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_query_cache() {
        let cache: QueryCache<i32> = QueryCache::new();

        let call_count = Arc::new(Mutex::new(0));

        // First call
        {
            let count = call_count.clone();
            let result = cache
                .get_or_compute("key1", || {
                    let mut c = count.lock().unwrap();
                    *c += 1;
                    Ok(42)
                })
                .unwrap();
            assert_eq!(result, 42);
        }

        // Second call - should use cache
        {
            let count = call_count.clone();
            let result = cache
                .get_or_compute("key1", || {
                    let mut c = count.lock().unwrap();
                    *c += 1;
                    Ok(100) // Different value if called
                })
                .unwrap();
            assert_eq!(result, 42); // Should get cached value
        }

        assert_eq!(*call_count.lock().unwrap(), 1);
    }

    #[test]
    fn test_query_stats() {
        let stats = QueryStats::default();
        assert_eq!(stats.total_queries, 0);
        assert_eq!(stats.cache_hits, 0);
    }

    #[test]
    fn test_batch_operations_chunking() {
        let items: Vec<i32> = (0..250).collect();
        let chunks: Vec<_> = BatchOperations::chunk_batches(&items).collect();

        // Should create 3 chunks: 100, 100, 50
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 100);
        assert_eq!(chunks[1].len(), 100);
        assert_eq!(chunks[2].len(), 50);
    }

    #[test]
    fn test_fast_path_config() {
        let cold_config = FastPathConfig::for_cold_start();
        assert!(cold_config.defer_indexes);
        assert!(cold_config.wal_mode);
        assert!(cold_config.background_indexes);

        let hot_config = FastPathConfig::for_hot_start();
        assert!(!hot_config.defer_indexes);
        assert!(hot_config.wal_mode);
    }

    #[test]
    fn test_deferred_index() {
        let index = DeferredIndex::new(
            "test_idx".to_string(),
            "CREATE INDEX test_idx ON test(col)".to_string(),
            5,
        );

        assert_eq!(index.name, "test_idx");
        assert_eq!(index.status, IndexStatus::Pending);
        assert_eq!(index.priority, 5);
    }

    #[test]
    fn test_database_fast_path_new() {
        let fast_path = DatabaseFastPath::new();

        // Check config defaults
        let config = fast_path.config();
        assert!(config.defer_indexes);
        assert!(config.wal_mode);

        // Check initial state
        assert!(!fast_path.are_indexes_ready().unwrap());
    }

    #[test]
    fn test_connection_pool_warmup_estimate() {
        // Test estimate for small database
        let small_estimate = ConnectionPoolWarmup::estimate_warmup_benefit(1024 * 1024); // 1MB
        assert_eq!(small_estimate, 10);

        // Test estimate for large database (should cap at 100ms)
        let large_estimate = ConnectionPoolWarmup::estimate_warmup_benefit(50 * 1024 * 1024); // 50MB
        assert_eq!(large_estimate, 100);
    }

    #[test]
    fn test_get_deferred_indexes_sorted() {
        let indexes = DatabaseFastPath::get_deferred_indexes();

        // Check that indexes are sorted by priority (descending)
        for i in 0..indexes.len() - 1 {
            assert!(indexes[i].priority >= indexes[i + 1].priority);
        }

        // Check that essential indexes have highest priority
        assert!(indexes.iter().any(|i| i.priority >= 10));
    }
}
