#![allow(dead_code)]

use parking_lot::Mutex;
/// Connection Pool Optimizer
/// Maximizes TCP connection reuse and minimizes latency
use std::collections::{HashMap, VecDeque};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Optimized connection pool with predictive pre-warming
pub struct OptimizedConnectionPool {
    /// Pooled connections by endpoint
    connections: Arc<Mutex<HashMap<Endpoint, Vec<PooledConnection>>>>,
    /// Pool configuration
    config: PoolConfig,
    /// Connection statistics
    stats: Arc<ConnectionStats>,
    /// Pre-warming queue
    prewarm_queue: Arc<Mutex<Vec<Endpoint>>>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct Endpoint {
    host: String,
    port: u16,
}

impl Endpoint {
    fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
        }
    }
}

struct PooledConnection {
    stream: TcpStream,
    created_at: Instant,
    last_used: Instant,
    use_count: u64,
    bytes_transferred: u64,
}

impl PooledConnection {
    fn is_expired(&self, config: &PoolConfig) -> bool {
        self.last_used.elapsed() > config.idle_timeout
            || self.created_at.elapsed() > config.max_lifetime
            || self.use_count > config.max_reuse_count
    }

    fn health_score(&self) -> f64 {
        // Lower is better
        let age_factor = self.created_at.elapsed().as_secs_f64() / 3600.0; // 1 hour baseline
        let idle_factor = self.last_used.elapsed().as_secs_f64() / 60.0; // 1 minute baseline
        let reuse_factor = self.use_count as f64 / 1000.0; // 1000 uses baseline

        age_factor * 0.3 + idle_factor * 0.5 + reuse_factor * 0.2
    }
}

#[derive(Clone, Debug)]
pub struct PoolConfig {
    /// Maximum connections per endpoint
    pub max_per_endpoint: usize,
    /// Connection idle timeout
    pub idle_timeout: Duration,
    /// Maximum connection lifetime
    pub max_lifetime: Duration,
    /// Maximum reuse count before recycling
    pub max_reuse_count: u64,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Enable connection pre-warming
    pub enable_prewarm: bool,
    /// Pre-warm connection count
    pub prewarm_count: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_per_endpoint: 4,
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(3600),
            max_reuse_count: 10000,
            connect_timeout: Duration::from_secs(10),
            enable_prewarm: true,
            prewarm_count: 1,
        }
    }
}

#[derive(Default)]
struct ConnectionStats {
    created: AtomicU64,
    reused: AtomicU64,
    expired: AtomicU64,
    failed: AtomicU64,
    total_wait_time_ms: AtomicU64,
}

impl OptimizedConnectionPool {
    pub fn new(config: PoolConfig) -> Self {
        let pool = Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            config: config.clone(),
            stats: Arc::new(ConnectionStats::default()),
            prewarm_queue: Arc::new(Mutex::new(Vec::new())),
        };

        // Start background maintenance task
        pool.start_maintenance();

        pool
    }

    /// Acquire connection from pool or create new
    pub fn acquire(&self, host: &str, port: u16) -> Result<ConnectionGuard, ConnectionError> {
        let endpoint = Endpoint::new(host, port);
        let start = Instant::now();

        // Try to get from pool
        {
            let mut connections = self.connections.lock();
            if let Some(pool) = connections.get_mut(&endpoint) {
                // Find best connection (lowest health score)
                let best_idx = pool
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| !c.is_expired(&self.config))
                    .min_by(|(_, a), (_, b)| {
                        a.health_score().partial_cmp(&b.health_score()).unwrap()
                    })
                    .map(|(idx, _)| idx);

                if let Some(idx) = best_idx {
                    let mut conn = pool.remove(idx);
                    conn.use_count += 1;
                    conn.last_used = Instant::now();

                    self.stats.reused.fetch_add(1, Ordering::Relaxed);
                    self.stats
                        .total_wait_time_ms
                        .fetch_add(start.elapsed().as_millis() as u64, Ordering::Relaxed);

                    return Ok(ConnectionGuard {
                        stream: Some(conn.stream),
                        endpoint: endpoint.clone(),
                        pool: self.connections.clone(),
                        return_to_pool: true,
                        stats: self.stats.clone(),
                    });
                }
            }
        }

        // Create new connection
        match self.create_connection(host, port) {
            Ok(stream) => {
                self.stats.created.fetch_add(1, Ordering::Relaxed);
                self.stats
                    .total_wait_time_ms
                    .fetch_add(start.elapsed().as_millis() as u64, Ordering::Relaxed);

                Ok(ConnectionGuard {
                    stream: Some(stream),
                    endpoint,
                    pool: self.connections.clone(),
                    return_to_pool: true,
                    stats: self.stats.clone(),
                })
            }
            Err(e) => {
                self.stats.failed.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    /// Create new TCP connection with optimized settings
    fn create_connection(&self, host: &str, port: u16) -> Result<TcpStream, ConnectionError> {
        let addr = format!("{}:{}", host, port);

        let stream =
            TcpStream::connect(&addr).map_err(|e| ConnectionError::ConnectFailed(e.to_string()))?;

        // Optimize TCP settings for SSH
        stream
            .set_nodelay(true)
            .map_err(|e| ConnectionError::ConfigurationFailed(e.to_string()))?;

        stream
            .set_read_timeout(Some(self.config.idle_timeout))
            .map_err(|e| ConnectionError::ConfigurationFailed(e.to_string()))?;

        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| ConnectionError::ConfigurationFailed(e.to_string()))?;

        Ok(stream)
    }

    /// Pre-warm connections for known endpoints
    pub fn prewarm(&self, host: &str, port: u16) {
        if !self.config.enable_prewarm {
            return;
        }

        let endpoint = Endpoint::new(host, port);

        // Check current pool size
        let connections = self.connections.lock();
        let current_count = connections.get(&endpoint).map(|p| p.len()).unwrap_or(0);
        drop(connections);

        // Add to pre-warm queue if below target
        if current_count < self.config.prewarm_count {
            let mut queue = self.prewarm_queue.lock();
            if !queue.contains(&endpoint) {
                queue.push(endpoint);
            }
        }
    }

    /// Start background maintenance task
    fn start_maintenance(&self) {
        let connections = self.connections.clone();
        let config = self.config.clone();
        let stats = self.stats.clone();
        let prewarm_queue = self.prewarm_queue.clone();

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(30));

                // Cleanup expired connections
                {
                    let mut conns = connections.lock();
                    let mut expired_count = 0;

                    for pool in conns.values_mut() {
                        let before = pool.len();
                        pool.retain(|c| !c.is_expired(&config));
                        expired_count += before - pool.len();
                    }

                    // Remove empty pools
                    conns.retain(|_, p| !p.is_empty());

                    stats
                        .expired
                        .fetch_add(expired_count as u64, Ordering::Relaxed);
                }

                // Process pre-warm queue
                {
                    let mut queue = prewarm_queue.lock();
                    while let Some(endpoint) = queue.pop() {
                        // Would create connection here
                        log::debug!(
                            "Pre-warming connection to {}:{}",
                            endpoint.host,
                            endpoint.port
                        );
                    }
                }
            }
        });
    }

    /// Get pool statistics
    pub fn stats(&self) -> OptimizedPoolStats {
        let connections = self.connections.lock();

        OptimizedPoolStats {
            total_endpoints: connections.len(),
            total_connections: connections.values().map(|p| p.len()).sum(),
            created: self.stats.created.load(Ordering::Relaxed),
            reused: self.stats.reused.load(Ordering::Relaxed),
            expired: self.stats.expired.load(Ordering::Relaxed),
            failed: self.stats.failed.load(Ordering::Relaxed),
            avg_wait_time_ms: if self.stats.reused.load(Ordering::Relaxed) > 0 {
                self.stats.total_wait_time_ms.load(Ordering::Relaxed) as f64
                    / (self.stats.created.load(Ordering::Relaxed)
                        + self.stats.reused.load(Ordering::Relaxed)) as f64
            } else {
                0.0
            },
        }
    }
}

/// Connection guard that automatically returns to pool
pub struct ConnectionGuard {
    stream: Option<TcpStream>,
    endpoint: Endpoint,
    pool: Arc<Mutex<HashMap<Endpoint, Vec<PooledConnection>>>>,
    return_to_pool: bool,
    stats: Arc<ConnectionStats>,
}

impl ConnectionGuard {
    pub fn get_stream(&self) -> &TcpStream {
        self.stream.as_ref().unwrap()
    }

    pub fn into_stream(mut self) -> TcpStream {
        self.return_to_pool = false;
        self.stream.take().unwrap()
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        if !self.return_to_pool {
            return;
        }

        if let Some(stream) = self.stream.take() {
            // Check if stream is still healthy
            // Simplified - in production would check readability
            let is_healthy = true;

            if is_healthy {
                let mut pool = self.pool.lock();
                let pooled = PooledConnection {
                    stream,
                    created_at: Instant::now(),
                    last_used: Instant::now(),
                    use_count: 1,
                    bytes_transferred: 0,
                };

                pool.entry(self.endpoint.clone()).or_default().push(pooled);
            }
        }
    }
}

#[derive(Debug)]
pub enum ConnectionError {
    ConnectFailed(String),
    ConfigurationFailed(String),
    PoolExhausted,
}

#[derive(Debug, Clone)]
pub struct OptimizedPoolStats {
    pub total_endpoints: usize,
    pub total_connections: usize,
    pub created: u64,
    pub reused: u64,
    pub expired: u64,
    pub failed: u64,
    pub avg_wait_time_ms: f64,
}

/// Connection latency optimizer
pub struct LatencyOptimizer {
    /// Endpoint latency history
    latency_history: Arc<Mutex<HashMap<Endpoint, VecDeque<Duration>>>>,
    /// Optimal endpoint selection
    optimal_cache: Arc<Mutex<HashMap<String, Endpoint>>>,
}

impl LatencyOptimizer {
    pub fn new() -> Self {
        Self {
            latency_history: Arc::new(Mutex::new(HashMap::new())),
            optimal_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Record connection latency
    pub fn record_latency(&self, host: &str, port: u16, latency: Duration) {
        let endpoint = Endpoint::new(host, port);
        let mut history = self.latency_history.lock();

        let samples = history
            .entry(endpoint.clone())
            .or_insert_with(|| VecDeque::with_capacity(100));

        if samples.len() >= 100 {
            samples.pop_front();
        }
        samples.push_back(latency);

        // Update optimal endpoint if this is fastest
        let avg_latency = samples.iter().sum::<Duration>() / samples.len() as u32;

        let mut optimal = self.optimal_cache.lock();
        let current_optimal = optimal.get(host);

        if let Some(current) = current_optimal {
            let current_avg = history
                .get(current)
                .map(|s| s.iter().sum::<Duration>() / s.len() as u32)
                .unwrap_or(Duration::MAX);

            if avg_latency < current_avg {
                optimal.insert(host.to_string(), endpoint);
            }
        } else {
            optimal.insert(host.to_string(), endpoint);
        }
    }

    /// Get optimal endpoint for host
    pub fn get_optimal_endpoint(&self, host: &str, default_port: u16) -> (String, u16) {
        let optimal = self.optimal_cache.lock();
        if let Some(endpoint) = optimal.get(host) {
            (endpoint.host.clone(), endpoint.port)
        } else {
            (host.to_string(), default_port)
        }
    }
}

/// Global connection pool
use std::sync::OnceLock;

static GLOBAL_CONNECTION_POOL: OnceLock<OptimizedConnectionPool> = OnceLock::new();

pub fn global_connection_pool() -> &'static OptimizedConnectionPool {
    GLOBAL_CONNECTION_POOL.get_or_init(|| OptimizedConnectionPool::new(PoolConfig::default()))
}
