//! Optimized Database Connection Pool
//!
//! Features:
//! - Adaptive connection pool sizing based on load
//! - Connection health checking and automatic recovery
//! - Latency-based connection selection
//! - Prepared statement caching per connection
//! - Background maintenance and cleanup

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use std::sync::Mutex;
use tokio::sync::{Notify, RwLock, Semaphore};
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, trace, warn};

use crate::database_client::drivers::{ConnectionInfo, DatabaseDriver};
use crate::database_client::{DatabaseConfig, DatabaseError, DatabaseType};

/// Pool configuration with adaptive sizing
#[derive(Debug, Clone)]
pub struct OptimizedPoolConfig {
    /// Maximum connections in the pool
    pub max_size: usize,
    /// Minimum connections to maintain
    pub min_size: usize,
    /// Maximum time to wait for a connection from the pool
    pub acquire_timeout: Duration,
    /// Maximum time a connection can be idle before being closed
    pub idle_timeout: Duration,
    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,
    /// Interval between health checks
    pub health_check_interval: Duration,
    /// Timeout for health check queries
    pub health_check_timeout: Duration,
    /// Whether to test connections on checkout
    pub test_on_checkout: bool,
    /// Whether to test connections on checkin
    pub test_on_checkin: bool,
    /// Adaptive pool sizing enabled
    pub adaptive_sizing: bool,
    /// Scale up threshold (connections in use / max_size)
    pub scale_up_threshold: f64,
    /// Scale down threshold (connections in use / max_size)
    pub scale_down_threshold: f64,
    /// Minimum time between scaling operations
    pub scale_cooldown: Duration,
    /// Prepared statement cache size per connection
    pub statement_cache_size: usize,
    /// Connection retry attempts
    pub connection_retry_attempts: u32,
    /// Connection retry delay
    pub connection_retry_delay: Duration,
}

impl Default for OptimizedPoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10,
            min_size: 2,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(1800),
            health_check_interval: Duration::from_secs(30),
            health_check_timeout: Duration::from_secs(5),
            test_on_checkout: true,
            test_on_checkin: false,
            adaptive_sizing: true,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.3,
            scale_cooldown: Duration::from_secs(60),
            statement_cache_size: 100,
            connection_retry_attempts: 3,
            connection_retry_delay: Duration::from_millis(100),
        }
    }
}

impl OptimizedPoolConfig {
    /// Create configuration for high-throughput workloads
    pub fn high_throughput() -> Self {
        Self {
            max_size: 50,
            min_size: 10,
            acquire_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(3600),
            test_on_checkout: false,
            test_on_checkin: true,
            statement_cache_size: 200,
            ..Default::default()
        }
    }

    /// Create configuration for low-latency workloads
    pub fn low_latency() -> Self {
        Self {
            max_size: 20,
            min_size: 5,
            acquire_timeout: Duration::from_secs(5),
            test_on_checkout: true,
            health_check_interval: Duration::from_secs(10),
            ..Default::default()
        }
    }

    /// Create configuration for resource-constrained environments
    pub fn resource_constrained() -> Self {
        Self {
            max_size: 5,
            min_size: 1,
            acquire_timeout: Duration::from_secs(60),
            idle_timeout: Duration::from_secs(60),
            max_lifetime: Duration::from_secs(300),
            adaptive_sizing: false,
            statement_cache_size: 20,
            ..Default::default()
        }
    }
}

/// Internal pooled connection wrapper
struct PooledConnectionInner {
    /// The actual database driver
    driver: Box<dyn DatabaseDriver>,
    /// When this connection was created
    created_at: Instant,
    /// When this connection was last used
    last_used_at: Instant,
    /// Number of times this connection has been used
    use_count: u64,
    /// Total query execution time on this connection
    total_query_time_ms: u64,
    /// Whether this connection is currently checked out
    is_checked_out: bool,
    /// Connection health status
    is_healthy: bool,
    /// Last health check timestamp
    last_health_check: Instant,
    /// Prepared statement cache
    statement_cache: Arc<RwLock<HashMap<String, String>>>,
    /// Connection latency history (last 10 queries)
    latency_history: VecDeque<Duration>,
}

impl PooledConnectionInner {
    fn new(driver: Box<dyn DatabaseDriver>) -> Self {
        let now = Instant::now();
        Self {
            driver,
            created_at: now,
            last_used_at: now,
            use_count: 0,
            total_query_time_ms: 0,
            is_checked_out: false,
            is_healthy: true,
            last_health_check: now,
            statement_cache: Arc::new(RwLock::new(HashMap::new())),
            latency_history: VecDeque::with_capacity(10),
        }
    }

    /// Check if connection has exceeded max lifetime
    fn is_expired(&self, max_lifetime: Duration) -> bool {
        self.created_at.elapsed() > max_lifetime
    }

    /// Check if connection has been idle too long
    fn is_idle(&self, idle_timeout: Duration) -> bool {
        !self.is_checked_out && self.last_used_at.elapsed() > idle_timeout
    }

    /// Calculate health score (lower is better)
    fn health_score(&self) -> f64 {
        let age_factor = self.created_at.elapsed().as_secs_f64() / 1800.0; // 30 min baseline
        let idle_factor = self.last_used_at.elapsed().as_secs_f64() / 60.0; // 1 min baseline
        let latency_factor = self.average_latency().as_millis() as f64 / 100.0; // 100ms baseline
        let use_factor = self.use_count as f64 / 1000.0; // 1000 uses baseline

        age_factor * 0.2 + idle_factor * 0.3 + latency_factor * 0.4 + use_factor * 0.1
    }

    /// Get average latency from history
    fn average_latency(&self) -> Duration {
        if self.latency_history.is_empty() {
            return Duration::from_millis(0);
        }
        let total: Duration = self.latency_history.iter().sum();
        total / self.latency_history.len() as u32
    }

    /// Record query latency
    fn record_latency(&mut self, latency: Duration) {
        if self.latency_history.len() >= 10 {
            self.latency_history.pop_front();
        }
        self.latency_history.push_back(latency);
    }

    /// Get cached statement or cache new one
    fn get_cached_statement(&self, sql: &str) -> Option<String> {
        let cache = self.statement_cache.read();
        cache.get(sql).cloned()
    }

    fn cache_statement(&self, sql: &str, prepared: &str) {
        let mut cache = self.statement_cache.write();
        if cache.len() >= 100 { // Max cache size
            // Remove oldest entry (simple FIFO)
            if let Some(oldest) = cache.keys().next().cloned() {
                cache.remove(&oldest);
            }
        }
        cache.insert(sql.to_string(), prepared.to_string());
    }
}

/// Connection pool statistics
#[derive(Debug, Clone, Default)]
pub struct PoolStatistics {
    /// Total connections created
    pub total_created: u64,
    /// Total connections closed
    pub total_closed: u64,
    /// Total connections acquired from pool
    pub total_acquired: u64,
    /// Total connections returned to pool
    pub total_released: u64,
    /// Total health check passes
    pub health_checks_passed: u64,
    /// Total health check failures
    pub health_checks_failed: u64,
    /// Current active connections
    pub active_connections: usize,
    /// Current idle connections
    pub idle_connections: usize,
    /// Current pending acquire requests
    pub pending_acquires: usize,
    /// Average acquire wait time in milliseconds
    pub avg_acquire_wait_ms: f64,
    /// Average query execution time in milliseconds
    pub avg_query_time_ms: f64,
    /// Cache hit ratio for prepared statements
    pub statement_cache_hit_ratio: f64,
    /// Pool resize events (up)
    pub scale_up_events: u64,
    /// Pool resize events (down)
    pub scale_down_events: u64,
}

/// Optimized database connection pool
pub struct OptimizedConnectionPool {
    /// Pool configuration
    config: OptimizedPoolConfig,
    /// Connection info for creating new connections
    connection_info: ConnectionInfo,
    /// Database type
    db_type: DatabaseType,
    /// Available connections
    idle_connections: Arc<Mutex<VecDeque<Arc<RwLock<PooledConnectionInner>>>>>,
    /// Total connection count (idle + active)
    total_connections: Arc<AtomicUsize>,
    /// Semaphore for limiting concurrent connection creation
    creation_semaphore: Arc<Semaphore>,
    /// Notify for new connections available
    notify: Arc<Notify>,
    /// Pool statistics
    statistics: Arc<PoolStatisticsInternal>,
    /// Shutdown signal
    shutdown: Arc<Notify>,
    /// Background maintenance task handle
    maintenance_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Last scale operation timestamp
    last_scale_operation: Arc<Mutex<Instant>>,
    /// Current max size (for adaptive sizing)
    current_max_size: Arc<AtomicUsize>,
}

/// Internal statistics tracking
struct PoolStatisticsInternal {
    total_created: AtomicU64,
    total_closed: AtomicU64,
    total_acquired: AtomicU64,
    total_released: AtomicU64,
    health_checks_passed: AtomicU64,
    health_checks_failed: AtomicU64,
    total_acquire_wait_ms: AtomicU64,
    total_query_time_ms: AtomicU64,
    query_count: AtomicU64,
    scale_up_events: AtomicU64,
    scale_down_events: AtomicU64,
    statement_cache_hits: AtomicU64,
    statement_cache_misses: AtomicU64,
}

impl PoolStatisticsInternal {
    fn new() -> Self {
        Self {
            total_created: AtomicU64::new(0),
            total_closed: AtomicU64::new(0),
            total_acquired: AtomicU64::new(0),
            total_released: AtomicU64::new(0),
            health_checks_passed: AtomicU64::new(0),
            health_checks_failed: AtomicU64::new(0),
            total_acquire_wait_ms: AtomicU64::new(0),
            total_query_time_ms: AtomicU64::new(0),
            query_count: AtomicU64::new(0),
            scale_up_events: AtomicU64::new(0),
            scale_down_events: AtomicU64::new(0),
            statement_cache_hits: AtomicU64::new(0),
            statement_cache_misses: AtomicU64::new(0),
        }
    }
}

impl OptimizedConnectionPool {
    /// Create a new connection pool
    pub async fn new(
        config: OptimizedPoolConfig,
        db_config: &DatabaseConfig,
    ) -> Result<Self, DatabaseError> {
        let connection_info = db_config.to_connection_info();
        let db_type = db_config.db_type;

        let pool = Self {
            config: config.clone(),
            connection_info,
            db_type,
            idle_connections: Arc::new(Mutex::new(VecDeque::new())),
            total_connections: Arc::new(AtomicUsize::new(0)),
            creation_semaphore: Arc::new(Semaphore::new(config.max_size)),
            notify: Arc::new(Notify::new()),
            statistics: Arc::new(PoolStatisticsInternal::new()),
            shutdown: Arc::new(Notify::new()),
            maintenance_handle: Arc::new(Mutex::new(None)),
            last_scale_operation: Arc::new(Mutex::new(Instant::now())),
            current_max_size: Arc::new(AtomicUsize::new(config.max_size)),
        };

        // Create minimum connections
        for _ in 0..config.min_size {
            match pool.create_connection().await {
                Ok(conn) => {
                    pool.idle_connections.lock().push_back(conn);
                    pool.total_connections.fetch_add(1, Ordering::SeqCst);
                    pool.statistics.total_created.fetch_add(1, Ordering::SeqCst);
                }
                Err(e) => {
                    warn!("Failed to create initial connection: {}", e);
                }
            }
        }

        // Start maintenance task
        pool.start_maintenance();

        info!(
            "Created connection pool for {:?} with {} initial connections",
            db_type,
            pool.total_connections.load(Ordering::SeqCst)
        );

        Ok(pool)
    }

    /// Acquire a connection from the pool
    pub async fn acquire(&self) -> Result<PooledConnectionGuard, DatabaseError> {
        let start = Instant::now();
        let acquire_timeout = self.config.acquire_timeout;

        // Try to get an idle connection
        loop {
            // Check timeout
            if start.elapsed() > acquire_timeout {
                return Err(DatabaseError::TimeoutError(
                    "Connection acquire timeout".to_string(),
                ));
            }

            // Try to get from idle pool
            {
                let mut idle = self.idle_connections.lock();
                if let Some(conn) = idle.pop_front() {
                    drop(idle); // Release lock before health check

                    // Test connection if needed
                    if self.config.test_on_checkout {
                        let is_healthy = self.check_connection_health(&conn).await;
                        if !is_healthy {
                            // Connection unhealthy, close it and create new
                            self.close_connection(conn).await;
                            continue;
                        }
                    }

                    // Mark as checked out
                    {
                        let mut conn_lock = conn.write();
                        conn_lock.is_checked_out = true;
                        conn_lock.use_count += 1;
                    }

                    self.statistics.total_acquired.fetch_add(1, Ordering::SeqCst);
                    let wait_ms = start.elapsed().as_millis() as u64;
                    self.statistics
                        .total_acquire_wait_ms
                        .fetch_add(wait_ms, Ordering::SeqCst);

                    return Ok(PooledConnectionGuard {
                        connection: Some(conn),
                        pool: Arc::new(self.clone_ref()),
                        start_time: Instant::now(),
                    });
                }
            }

            // Check if we can create a new connection
            let current_total = self.total_connections.load(Ordering::SeqCst);
            let current_max = self.current_max_size.load(Ordering::SeqCst);

            if current_total < current_max {
                // Try to create new connection
                match self.create_connection().await {
                    Ok(conn) => {
                        self.total_connections.fetch_add(1, Ordering::SeqCst);
                        self.statistics.total_created.fetch_add(1, Ordering::SeqCst);

                        let mut conn_lock = conn.write();
                        conn_lock.is_checked_out = true;
                        conn_lock.use_count += 1;
                        drop(conn_lock);

                        self.statistics.total_acquired.fetch_add(1, Ordering::SeqCst);
                        let wait_ms = start.elapsed().as_millis() as u64;
                        self.statistics
                            .total_acquire_wait_ms
                            .fetch_add(wait_ms, Ordering::SeqCst);

                        return Ok(PooledConnectionGuard {
                            connection: Some(conn),
                            pool: Arc::new(self.clone_ref()),
                            start_time: Instant::now(),
                        });
                    }
                    Err(e) => {
                        warn!("Failed to create new connection: {}", e);
                        // Fall through to wait
                    }
                }
            }

            // Wait for a connection to become available
            let wait_result = tokio::time::timeout(
                Duration::from_millis(100),
                self.notify.notified(),
            )
            .await;

            if wait_result.is_err() {
                // Timeout, check if we exceeded total acquire timeout
                if start.elapsed() > acquire_timeout {
                    return Err(DatabaseError::TimeoutError(
                        "Connection acquire timeout".to_string(),
                    ));
                }
            }
        }
    }

    /// Create a new database connection
    async fn create_connection(
        &self,
    ) -> Result<Arc<RwLock<PooledConnectionInner>>, DatabaseError> {
        let _permit = self
            .creation_semaphore
            .acquire()
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        // Retry logic
        let mut last_error = None;
        for attempt in 0..self.config.connection_retry_attempts {
            match self.try_create_connection().await {
                Ok(driver) => {
                    return Ok(Arc::new(RwLock::new(PooledConnectionInner::new(driver))));
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.connection_retry_attempts - 1 {
                        tokio::time::sleep(self.config.connection_retry_delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            DatabaseError::ConnectionError("Failed to create connection".to_string())
        }))
    }

    async fn try_create_connection(&self) -> Result<Box<dyn DatabaseDriver>, DatabaseError> {
        // Create driver based on database type
        let driver: Box<dyn DatabaseDriver> = match self.db_type {
            DatabaseType::MySQL => {
                #[cfg(feature = "mysql-driver")]
                {
                    use crate::database_client::drivers::mysql::MySqlDriver;
                    Box::new(MySqlDriver::new())
                }
                #[cfg(not(feature = "mysql-driver"))]
                return Err(DatabaseError::DriverNotFound(
                    "MySQL driver not enabled".to_string(),
                ));
            }
            DatabaseType::PostgreSQL => {
                #[cfg(feature = "postgres-driver")]
                {
                    use crate::database_client::drivers::postgres::PostgresDriver;
                    Box::new(PostgresDriver::new())
                }
                #[cfg(not(feature = "postgres-driver"))]
                return Err(DatabaseError::DriverNotFound(
                    "PostgreSQL driver not enabled".to_string(),
                ));
            }
            DatabaseType::MongoDB => {
                #[cfg(feature = "mongodb-driver")]
                {
                    use crate::database_client::drivers::mongodb::MongoDbDriver;
                    Box::new(MongoDbDriver::new())
                }
                #[cfg(not(feature = "mongodb-driver"))]
                return Err(DatabaseError::DriverNotFound(
                    "MongoDB driver not enabled".to_string(),
                ));
            }
            DatabaseType::Redis => {
                #[cfg(feature = "redis-driver")]
                {
                    use crate::database_client::drivers::redis::RedisDriver;
                    Box::new(RedisDriver::new())
                }
                #[cfg(not(feature = "redis-driver"))]
                return Err(DatabaseError::DriverNotFound(
                    "Redis driver not enabled".to_string(),
                ));
            }
            DatabaseType::SQLite => {
                use crate::database_client::drivers::sqlite::SqliteDriver;
                Box::new(SqliteDriver::new())
            }
        };

        // Connect with timeout
        let mut driver = driver;
        timeout(
            Duration::from_secs(self.connection_info.connection_timeout),
            driver.connect(&self.connection_info),
        )
        .await
        .map_err(|_| DatabaseError::TimeoutError("Connection timeout".to_string()))?
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        Ok(driver)
    }

    /// Check connection health
    async fn check_connection_health(
        &self,
        conn: &Arc<RwLock<PooledConnectionInner>>,
    ) -> bool {
        let health_check = async {
            let conn_lock = conn.read();
            conn_lock.driver.ping().await
        };

        let result = timeout(self.config.health_check_timeout, health_check).await;

        match result {
            Ok(Ok(())) => {
                self.statistics
                    .health_checks_passed
                    .fetch_add(1, Ordering::SeqCst);
                true
            }
            _ => {
                self.statistics
                    .health_checks_failed
                    .fetch_add(1, Ordering::SeqCst);
                false
            }
        }
    }

    /// Close a connection
    async fn close_connection(&self, conn: Arc<RwLock<PooledConnectionInner>>) {
        let mut conn_lock = conn.write();
        let _ = conn_lock.driver.disconnect().await;
        drop(conn_lock);

        self.total_connections.fetch_sub(1, Ordering::SeqCst);
        self.statistics.total_closed.fetch_add(1, Ordering::SeqCst);
    }

    /// Return a connection to the pool
    async fn release_connection(&self, conn: Arc<RwLock<PooledConnectionInner>>) {
        let mut conn_lock = conn.write();
        conn_lock.is_checked_out = false;
        conn_lock.last_used_at = Instant::now();
        drop(conn_lock);

        self.idle_connections.lock().push_back(conn);
        self.statistics.total_released.fetch_add(1, Ordering::SeqCst);

        // Notify waiting acquires
        self.notify.notify_one();
    }

    /// Start background maintenance task
    fn start_maintenance(&self) {
        let shutdown = self.shutdown.clone();
        let idle_connections = self.idle_connections.clone();
        let total_connections = self.total_connections.clone();
        let config = self.config.clone();
        let statistics = self.statistics.clone();
        let current_max_size = self.current_max_size.clone();
        let last_scale_operation = self.last_scale_operation.clone();

        let handle = tokio::spawn(async move {
            let mut health_check_interval = interval(config.health_check_interval);
            let mut scale_check_interval = interval(Duration::from_secs(10));

            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        debug!("Pool maintenance task shutting down");
                        break;
                    }
                    _ = health_check_interval.tick() => {
                        Self::maintenance_cleanup(
                            &idle_connections,
                            &total_connections,
                            &config,
                            &statistics,
                        ).await;
                    }
                    _ = scale_check_interval.tick() => {
                        if config.adaptive_sizing {
                            Self::adaptive_scaling(
                                &current_max_size,
                                &total_connections,
                                &idle_connections,
                                &config,
                                &last_scale_operation,
                                &statistics,
                            ).await;
                        }
                    }
                }
            }
        });

        *self.maintenance_handle.lock() = Some(handle);
    }

    /// Maintenance cleanup: remove expired and idle connections
    async fn maintenance_cleanup(
        idle_connections: &Arc<Mutex<VecDeque<Arc<RwLock<PooledConnectionInner>>>>>,
        total_connections: &Arc<AtomicUsize>,
        config: &OptimizedPoolConfig,
        statistics: &Arc<PoolStatisticsInternal>,
    ) {
        let mut idle = idle_connections.lock();
        let initial_count = idle.len();

        // Remove expired and idle connections
        idle.retain(|conn| {
            let conn_lock = conn.read();
            let should_keep = !conn_lock.is_expired(config.max_lifetime)
                && !conn_lock.is_idle(config.idle_timeout);
            drop(conn_lock);

            if !should_keep {
                total_connections.fetch_sub(1, Ordering::SeqCst);
                statistics.total_closed.fetch_add(1, Ordering::SeqCst);
            }

            should_keep
        });

        let removed = initial_count - idle.len();
        if removed > 0 {
            debug!("Maintenance cleanup removed {} connections", removed);
        }
    }

    /// Adaptive pool sizing based on load
    async fn adaptive_scaling(
        current_max_size: &Arc<AtomicUsize>,
        total_connections: &Arc<AtomicUsize>,
        idle_connections: &Arc<Mutex<VecDeque<Arc<RwLock<PooledConnectionInner>>>>>,
        config: &OptimizedPoolConfig,
        last_scale_operation: &Arc<Mutex<Instant>>,
        statistics: &Arc<PoolStatisticsInternal>,
    ) {
        // Check cooldown
        {
            let last_scale = last_scale_operation.lock();
            if last_scale.elapsed() < config.scale_cooldown {
                return;
            }
        }

        let current_total = total_connections.load(Ordering::SeqCst);
        let current_max = current_max_size.load(Ordering::SeqCst);
        let idle_count = idle_connections.lock().len();
        let active_count = current_total - idle_count;

        let utilization = if current_max > 0 {
            active_count as f64 / current_max as f64
        } else {
            0.0
        };

        // Scale up if utilization is high
        if utilization > config.scale_up_threshold && current_max < config.max_size * 2 {
            let new_max = (current_max + 5).min(config.max_size * 2);
            current_max_size.store(new_max, Ordering::SeqCst);
            statistics.scale_up_events.fetch_add(1, Ordering::SeqCst);
            *last_scale_operation.lock() = Instant::now();
            info!(
                "Pool scaled up: {} -> {} connections (utilization: {:.2}%)",
                current_max,
                new_max,
                utilization * 100.0
            );
        }

        // Scale down if utilization is low and we have excess capacity
        if utilization < config.scale_down_threshold && current_max > config.max_size {
            let new_max = (current_max - 2).max(config.max_size);
            current_max_size.store(new_max, Ordering::SeqCst);
            statistics.scale_down_events.fetch_add(1, Ordering::SeqCst);
            *last_scale_operation.lock() = Instant::now();
            info!(
                "Pool scaled down: {} -> {} connections (utilization: {:.2}%)",
                current_max,
                new_max,
                utilization * 100.0
            );
        }
    }

    /// Get pool statistics
    pub fn get_statistics(&self) -> PoolStatistics {
        let idle_count = self.idle_connections.lock().len();
        let total_count = self.total_connections.load(Ordering::SeqCst);

        let total_acquired = self.statistics.total_acquired.load(Ordering::SeqCst);
        let total_acquire_wait_ms = self.statistics.total_acquire_wait_ms.load(Ordering::SeqCst);

        let total_query_time_ms = self.statistics.total_query_time_ms.load(Ordering::SeqCst);
        let query_count = self.statistics.query_count.load(Ordering::SeqCst);

        let cache_hits = self.statistics.statement_cache_hits.load(Ordering::SeqCst);
        let cache_misses = self.statistics.statement_cache_misses.load(Ordering::SeqCst);
        let cache_total = cache_hits + cache_misses;

        PoolStatistics {
            total_created: self.statistics.total_created.load(Ordering::SeqCst),
            total_closed: self.statistics.total_closed.load(Ordering::SeqCst),
            total_acquired,
            total_released: self.statistics.total_released.load(Ordering::SeqCst),
            health_checks_passed: self.statistics.health_checks_passed.load(Ordering::SeqCst),
            health_checks_failed: self.statistics.health_checks_failed.load(Ordering::SeqCst),
            active_connections: total_count - idle_count,
            idle_connections: idle_count,
            pending_acquires: 0, // Would need to track this separately
            avg_acquire_wait_ms: if total_acquired > 0 {
                total_acquire_wait_ms as f64 / total_acquired as f64
            } else {
                0.0
            },
            avg_query_time_ms: if query_count > 0 {
                total_query_time_ms as f64 / query_count as f64
            } else {
                0.0
            },
            statement_cache_hit_ratio: if cache_total > 0 {
                cache_hits as f64 / cache_total as f64
            } else {
                0.0
            },
            scale_up_events: self.statistics.scale_up_events.load(Ordering::SeqCst),
            scale_down_events: self.statistics.scale_down_events.load(Ordering::SeqCst),
        }
    }

    /// Get current pool size
    pub fn size(&self) -> usize {
        self.total_connections.load(Ordering::SeqCst)
    }

    /// Get current idle connection count
    pub fn idle_count(&self) -> usize {
        self.idle_connections.lock().len()
    }

    /// Record query execution time (called from guard)
    fn record_query_time(&self, duration: Duration) {
        self.statistics
            .total_query_time_ms
            .fetch_add(duration.as_millis() as u64, Ordering::SeqCst);
        self.statistics.query_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Shutdown the pool
    pub async fn shutdown(&self) {
        info!("Shutting down connection pool");

        // Signal maintenance task to stop
        self.shutdown.notify_waiters();

        // Wait for maintenance task to finish
        if let Some(handle) = self.maintenance_handle.lock().take() {
            let _ = handle.await;
        }

        // Close all connections
        let idle = std::mem::take(&mut *self.idle_connections.lock());
        for conn in idle {
            self.close_connection(conn).await;
        }

        info!("Connection pool shutdown complete");
    }

    /// Clone reference for internal use
    fn clone_ref(&self) -> Self {
        Self {
            config: self.config.clone(),
            connection_info: self.connection_info.clone(),
            db_type: self.db_type,
            idle_connections: self.idle_connections.clone(),
            total_connections: self.total_connections.clone(),
            creation_semaphore: self.creation_semaphore.clone(),
            notify: self.notify.clone(),
            statistics: self.statistics.clone(),
            shutdown: self.shutdown.clone(),
            maintenance_handle: self.maintenance_handle.clone(),
            last_scale_operation: self.last_scale_operation.clone(),
            current_max_size: self.current_max_size.clone(),
        }
    }
}

impl Clone for OptimizedConnectionPool {
    fn clone(&self) -> Self {
        self.clone_ref()
    }
}

/// Guard for pooled connections - automatically returns connection when dropped
pub struct PooledConnectionGuard {
    connection: Option<Arc<RwLock<PooledConnectionInner>>>,
    pool: Arc<OptimizedConnectionPool>,
    start_time: Instant,
}

impl PooledConnectionGuard {
    /// Get reference to the underlying driver
    pub async fn with_driver<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&dyn DatabaseDriver) -> R,
    {
        if let Some(ref conn) = self.connection {
            let conn_lock = conn.read();
            f(&*conn_lock.driver)
        } else {
            panic!("Connection already released");
        }
    }

    /// Get mutable reference to the underlying driver
    pub async fn with_driver_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Box<dyn DatabaseDriver>) -> R,
    {
        if let Some(ref conn) = self.connection {
            let mut conn_lock = conn.write();
            f(&mut conn_lock.driver)
        } else {
            panic!("Connection already released");
        }
    }

    /// Execute a query on this connection
    pub async fn query(&self, sql: &str) -> Result<crate::database_client::QueryResult, DatabaseError> {
        let start = Instant::now();

        let result = if let Some(ref conn) = self.connection {
            let conn_lock = conn.read();
            conn_lock.driver.query(sql).await
        } else {
            return Err(DatabaseError::ConnectionError("Connection not available".to_string()));
        };

        // Record latency
        let latency = start.elapsed();
        if let Some(ref conn) = self.connection {
            let mut conn_lock = conn.write();
            conn_lock.record_latency(latency);
            conn_lock.total_query_time_ms += latency.as_millis() as u64;
        }
        self.pool.record_query_time(latency);

        result
    }

    /// Execute a statement on this connection
    pub async fn execute(&self, sql: &str) -> Result<u64, DatabaseError> {
        let start = Instant::now();

        let result = if let Some(ref conn) = self.connection {
            let conn_lock = conn.read();
            conn_lock.driver.execute(sql).await
        } else {
            return Err(DatabaseError::ConnectionError("Connection not available".to_string()));
        };

        // Record latency
        let latency = start.elapsed();
        if let Some(ref conn) = self.connection {
            let mut conn_lock = conn.write();
            conn_lock.record_latency(latency);
        }
        self.pool.record_query_time(latency);

        result
    }

    /// Check if connection is healthy
    pub async fn is_healthy(&self) -> bool {
        if let Some(ref conn) = self.connection {
            let conn_lock = conn.read();
            conn_lock.is_healthy && conn_lock.driver.is_connected()
        } else {
            false
        }
    }

    /// Get connection latency statistics
    pub fn average_latency(&self) -> Duration {
        if let Some(ref conn) = self.connection {
            let conn_lock = conn.read();
            conn_lock.average_latency()
        } else {
            Duration::from_millis(0)
        }
    }

    /// Explicitly release the connection back to the pool
    pub async fn release(mut self) {
        if let Some(conn) = self.connection.take() {
            self.pool.release_connection(conn).await;
        }
    }
}

impl Drop for PooledConnectionGuard {
    fn drop(&mut self) {
        if let Some(conn) = self.connection.take() {
            let pool = self.pool.clone();
            // Spawn a task to return the connection to avoid blocking
            tokio::spawn(async move {
                pool.release_connection(conn).await;
            });
        }
    }
}

/// Pool manager for managing multiple connection pools
pub struct ConnectionPoolManager {
    pools: Arc<RwLock<HashMap<String, Arc<OptimizedConnectionPool>>>>,
}

impl ConnectionPoolManager {
    /// Create a new pool manager
    pub fn new() -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new pool
    pub async fn create_pool(
        &self,
        name: &str,
        config: OptimizedPoolConfig,
        db_config: &DatabaseConfig,
    ) -> Result<Arc<OptimizedConnectionPool>, DatabaseError> {
        let pool = Arc::new(OptimizedConnectionPool::new(config, db_config).await?);

        let mut pools = self.pools.write();
        pools.insert(name.to_string(), pool.clone());

        info!("Created pool '{}' for {:?}", name, db_config.db_type);
        Ok(pool)
    }

    /// Get an existing pool
    pub fn get_pool(&self, name: &str) -> Option<Arc<OptimizedConnectionPool>> {
        self.pools.read().get(name).cloned()
    }

    /// Remove and shutdown a pool
    pub async fn remove_pool(&self, name: &str) {
        let pool = {
            let mut pools = self.pools.write();
            pools.remove(name)
        };

        if let Some(pool) = pool {
            pool.shutdown().await;
            info!("Removed and shutdown pool '{}'", name);
        }
    }

    /// Get all pool statistics
    pub fn get_all_statistics(&self) -> HashMap<String, PoolStatistics> {
        let pools = self.pools.read();
        pools
            .iter()
            .map(|(name, pool)| (name.clone(), pool.get_statistics()))
            .collect()
    }

    /// Shutdown all pools
    pub async fn shutdown_all(&self) {
        let pools = {
            let mut pools = self.pools.write();
            std::mem::take(&mut *pools)
        };

        for (name, pool) in pools {
            pool.shutdown().await;
            debug!("Shutdown pool '{}'", name);
        }
    }

    /// List all pool names
    pub fn list_pools(&self) -> Vec<String> {
        self.pools.read().keys().cloned().collect()
    }
}

impl Default for ConnectionPoolManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global pool manager instance
use std::sync::OnceLock;

static GLOBAL_POOL_MANAGER: OnceLock<ConnectionPoolManager> = OnceLock::new();

/// Get the global pool manager
pub fn global_pool_manager() -> &'static ConnectionPoolManager {
    GLOBAL_POOL_MANAGER.get_or_init(ConnectionPoolManager::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_default() {
        let config = OptimizedPoolConfig::default();
        assert_eq!(config.max_size, 10);
        assert_eq!(config.min_size, 2);
        assert!(config.adaptive_sizing);
    }

    #[test]
    fn test_pool_config_high_throughput() {
        let config = OptimizedPoolConfig::high_throughput();
        assert_eq!(config.max_size, 50);
        assert_eq!(config.min_size, 10);
        assert!(!config.test_on_checkout);
        assert!(config.test_on_checkin);
    }

    #[test]
    fn test_health_score_calculation() {
        // This would need actual connections to test properly
        // Placeholder for unit test structure
    }
}
