//! Performance Optimization Module for EasySSH Lite
//!
//! This module provides performance optimizations for:
//! - Database query optimization with additional indexes
//! - Encryption operation optimization with key caching
//! - Search algorithm optimization with SIMD and parallel processing
//! - Memory usage optimization with object pools
//! - Startup time optimization with lazy loading and parallel initialization

pub mod crypto_optimizer;
pub mod db_optimizer;
pub mod memory_optimizer;
pub mod search_optimizer;
pub mod startup_optimizer;

pub use crypto_optimizer::CryptoOptimizer;
pub use db_optimizer::{DatabaseFastPath, DbOptimizer, FastPathConfig};
pub use memory_optimizer::MemoryOptimizer;
pub use search_optimizer::SearchOptimizer;
pub use startup_optimizer::{
    ColdStartCache, ParallelInitializer, StartType, StartupMetrics, StartupOptimizer,
    StartupSequence, StartupStatistics,
};

/// Performance metrics for monitoring optimization effectiveness
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Database query time in microseconds
    pub db_query_time_us: u64,
    /// Encryption time in microseconds
    pub encryption_time_us: u64,
    /// Search time in microseconds
    pub search_time_us: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Startup time in milliseconds
    pub startup_time_ms: u64,
}

/// Benchmark targets for Lite version
pub struct BenchmarkTargets;

impl BenchmarkTargets {
    /// Cold start time target: < 1.5 seconds
    pub const COLD_START_MS: u64 = 1500;
    /// Search response target: < 100 milliseconds
    pub const SEARCH_RESPONSE_MS: u64 = 100;
    /// Memory usage target: < 80 MB
    pub const MEMORY_USAGE_MB: u64 = 80;
    /// Database query target: < 10 milliseconds
    pub const DB_QUERY_MS: u64 = 10;
    /// Encryption throughput target: > 500 MiB/s
    pub const ENCRYPTION_THROUGHPUT_MIB: u64 = 500;
}

/// Check if current metrics meet benchmark targets
pub fn check_performance_targets(metrics: &PerformanceMetrics) -> Vec<(String, bool, u64, u64)> {
    vec![
        (
            "Startup Time".to_string(),
            metrics.startup_time_ms < BenchmarkTargets::COLD_START_MS,
            metrics.startup_time_ms,
            BenchmarkTargets::COLD_START_MS,
        ),
        (
            "Search Response".to_string(),
            metrics.search_time_us < BenchmarkTargets::SEARCH_RESPONSE_MS * 1000,
            metrics.search_time_us / 1000,
            BenchmarkTargets::SEARCH_RESPONSE_MS,
        ),
        (
            "Memory Usage".to_string(),
            metrics.memory_usage_bytes < BenchmarkTargets::MEMORY_USAGE_MB * 1024 * 1024,
            metrics.memory_usage_bytes / (1024 * 1024),
            BenchmarkTargets::MEMORY_USAGE_MB,
        ),
        (
            "Database Query".to_string(),
            metrics.db_query_time_us < BenchmarkTargets::DB_QUERY_MS * 1000,
            metrics.db_query_time_us / 1000,
            BenchmarkTargets::DB_QUERY_MS,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_targets() {
        assert_eq!(BenchmarkTargets::COLD_START_MS, 1500);
        assert_eq!(BenchmarkTargets::SEARCH_RESPONSE_MS, 100);
        assert_eq!(BenchmarkTargets::MEMORY_USAGE_MB, 80);
        assert_eq!(BenchmarkTargets::DB_QUERY_MS, 10);
    }

    #[test]
    fn test_check_performance_targets() {
        let metrics = PerformanceMetrics {
            startup_time_ms: 1000,
            search_time_us: 50_000,               // 50 ms
            memory_usage_bytes: 70 * 1024 * 1024, // 70 MB
            db_query_time_us: 5_000,              // 5 ms
            ..Default::default()
        };

        let results = check_performance_targets(&metrics);
        assert_eq!(results.len(), 4);

        // All should pass
        for (name, passed, actual, target) in &results {
            assert!(*passed, "{} failed: {} >= {}", name, actual, target);
        }
    }
}
