//! SSH Connection Performance Benchmarks
//!
//! This module benchmarks SSH-related operations in EasySSH.
//!
//! # Benchmark Scenarios
//!
//! - Connection pool operations
//! - Session creation and teardown
//! - Command execution latency
//! - Connection multiplexing
//! - Pool cleanup and maintenance
//!
//! # Running Benchmarks
//!
//! ```bash
//! cargo bench --bench ssh_bench
//! ```
//!
//! # Note
//!
//! These benchmarks use mocked SSH operations where possible to avoid
//! requiring actual SSH servers. Some benchmarks may require the `ssh2`
//! feature to be available.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use easyssh_core::connection_pool::{EnhancedConnectionState, EnhancedSshManager};
use easyssh_core::ssh::{ConnectionHealth, SessionMetadata, SshSessionManager};
use std::time::{Duration, Instant};

/// Benchmark SSH session manager creation
fn bench_session_manager_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ssh_session_manager");

    group.bench_function("new", |b| {
        b.iter(|| {
            let manager = SshSessionManager::new();
            black_box(manager);
        });
    });

    group.bench_function("new_with_pool_config", |b| {
        b.iter(|| {
            let manager = SshSessionManager::new().with_pool_config(100, 600, 3600);
            black_box(manager);
        });
    });

    group.finish();
}

/// Benchmark enhanced connection pool operations
fn bench_enhanced_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("enhanced_pool");

    group.bench_function("create_manager", |b| {
        b.iter(|| {
            let manager = EnhancedSshManager::new();
            black_box(manager);
        });
    });

    group.bench_function("create_manager_with_config", |b| {
        b.iter(|| {
            let manager = EnhancedSshManager::new();
            black_box(manager);
        });
    });

    // Benchmark state transitions
    group.bench_function("state_transition", |b| {
        b.iter(|| {
            let states = [
                EnhancedConnectionState::Connected,
                EnhancedConnectionState::Connecting,
                EnhancedConnectionState::Disconnected,
                EnhancedConnectionState::Failed { reason: "test error" },
            ];
            for state in states.iter() {
                black_box(state);
            }
        });
    });

    group.finish();
}

/// Benchmark connection health tracking
fn bench_connection_health(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_health");

    group.bench_function("health_enum_comparison", |b| {
        b.iter(|| {
            let health = ConnectionHealth::Healthy;
            let is_healthy = matches!(black_box(health), ConnectionHealth::Healthy);
            black_box(is_healthy);
        });
    });

    group.bench_function("health_cycle", |b| {
        b.iter(|| {
            let healths = vec![
                ConnectionHealth::Healthy,
                ConnectionHealth::Degraded,
                ConnectionHealth::Unhealthy,
            ];
            for health in healths {
                match black_box(health) {
                    ConnectionHealth::Healthy => {}
                    ConnectionHealth::Degraded => {}
                    ConnectionHealth::Unhealthy => {}
                }
            }
        });
    });

    group.finish();
}

/// Benchmark session metadata operations
fn bench_session_metadata(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_metadata");

    group.bench_function("create_metadata", |b| {
        b.iter(|| {
            let metadata = SessionMetadata {
                id: "session-123".to_string(),
                server_id: "server-456".to_string(),
                host: "192.168.1.100".to_string(),
                port: 22,
                username: "admin".to_string(),
                connected_at: Instant::now(),
            };
            black_box(metadata);
        });
    });

    group.bench_function("metadata_clone", |b| {
        let metadata = SessionMetadata {
            id: "session-123".to_string(),
            server_id: "server-456".to_string(),
            host: "192.168.1.100".to_string(),
            port: 22,
            username: "admin".to_string(),
            connected_at: Instant::now(),
        };

        b.iter(|| {
            let cloned = metadata.clone();
            black_box(cloned);
        });
    });

    group.finish();
}

/// Benchmark pool statistics operations
fn bench_pool_stats(c: &mut Criterion) {
    use easyssh_core::ssh::PoolStats;

    let mut group = c.benchmark_group("pool_stats");

    group.bench_function("create_stats", |b| {
        b.iter(|| {
            let stats = PoolStats {
                total_pools: 10,
                total_sessions: 15,
                pools: vec![],
            };
            black_box(stats);
        });
    });

    group.bench_function("stats_calculation", |b| {
        let stats = PoolStats {
            total_pools: 10,
            total_sessions: 15,
            pools: vec![],
        };

        b.iter(|| {
            let utilization = stats.total_sessions as f64 / stats.total_pools as f64;
            black_box(utilization);
        });
    });

    group.finish();
}

/// Benchmark server key hashing (for connection pooling)
fn bench_server_key(c: &mut Criterion) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut group = c.benchmark_group("server_key");

    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    struct ServerKey {
        host: String,
        port: u16,
        username: String,
    }

    group.bench_function("create_and_hash", |b| {
        b.iter(|| {
            let key = ServerKey {
                host: "192.168.1.100".to_string(),
                port: 22,
                username: "admin".to_string(),
            };
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            black_box(hasher.finish());
        });
    });

    group.bench_function("multiple_keys_hash", |b| {
        let keys: Vec<_> = (0..100)
            .map(|i| ServerKey {
                host: format!("192.168.1.{}", i % 256),
                port: 22 + (i % 10) as u16,
                username: format!("user{}", i),
            })
            .collect();

        b.iter(|| {
            for key in &keys {
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                black_box(hasher.finish());
            }
        });
    });

    group.finish();
}

/// Benchmark connection pool cleanup simulation
fn bench_pool_cleanup(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_cleanup");

    group.bench_function("expired_connection_check", |b| {
        let idle_timeout = Duration::from_secs(600);
        let max_age = Duration::from_secs(3600);

        b.iter(|| {
            let created_at = Instant::now() - Duration::from_secs(700);
            let last_used = Instant::now() - Duration::from_secs(650);

            let is_expired = last_used.elapsed() > idle_timeout || created_at.elapsed() > max_age;
            black_box(is_expired);
        });
    });

    group.bench_function("stale_connection_filter", |b| {
        let connections: Vec<(Instant, Instant)> = (0..100)
            .map(|i| {
                let age = Duration::from_secs(i as u64 * 10);
                let idle = Duration::from_secs((i as u64 * 5) % 700);
                (Instant::now() - age, Instant::now() - idle)
            })
            .collect();

        let idle_timeout = Duration::from_secs(600);
        let max_age = Duration::from_secs(3600);

        b.iter(|| {
            let stale: Vec<_> = connections
                .iter()
                .filter(|(created, last_used)| {
                    last_used.elapsed() > idle_timeout || created.elapsed() > max_age
                })
                .collect();
            black_box(stale);
        });
    });

    group.finish();
}

/// Benchmark SSH command preparation
fn bench_command_preparation(c: &mut Criterion) {
    let mut group = c.benchmark_group("command_preparation");

    group.bench_function("simple_command", |b| {
        b.iter(|| {
            let command = "uname -a";
            black_box(command);
        });
    });

    group.bench_function("complex_command", |b| {
        b.iter(|| {
            let command = r#"cd /var/log && grep -E "ERROR|WARN" application.log | head -n 100 | awk '{print $1, $2, $5}' | sort | uniq -c | sort -rn"#;
            black_box(command);
        });
    });

    group.bench_function("command_with_escaping", |b| {
        b.iter(|| {
            let user_input = "; rm -rf /"; // Potentially dangerous input
            let sanitized = user_input.replace(';', "").replace("rm", "");
            let command = format!("echo '{}'", sanitized);
            black_box(command);
        });
    });

    group.finish();
}

/// Benchmark connection metrics calculation
fn bench_connection_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_metrics");

    group.bench_function("latency_calculation", |b| {
        let start = Instant::now() - Duration::from_millis(150);

        b.iter(|| {
            let latency = start.elapsed().as_millis() as u64;
            black_box(latency);
        });
    });

    group.bench_function("throughput_calculation", |b| {
        let bytes_sent = 1024 * 1024; // 1 MB
        let duration = Duration::from_secs(2);

        b.iter(|| {
            let throughput = bytes_sent as f64 / duration.as_secs_f64();
            black_box(throughput);
        });
    });

    group.bench_function("success_rate_calculation", |b| {
        let successful = 95;
        let total = 100;

        b.iter(|| {
            let rate = (successful as f64 / total as f64) * 100.0;
            black_box(rate);
        });
    });

    group.finish();
}

/// Benchmark reconnection configuration
fn bench_reconnect_config(c: &mut Criterion) {
    use easyssh_core::connection_pool::ReconnectConfig;

    let mut group = c.benchmark_group("reconnect_config");

    group.bench_function("create_config", |b| {
        b.iter(|| {
            let config = ReconnectConfig {
                max_attempts: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 30000,
                backoff_multiplier: 2.0,
            };
            black_box(config);
        });
    });

    group.bench_function("calculate_backoff", |b| {
        let config = ReconnectConfig {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        };

        b.iter(|| {
            for attempt in 1..=3 {
                let delay =
                    config.initial_delay_ms * (config.backoff_multiplier.powi(attempt - 1) as u64);
                let total_delay = std::cmp::min(delay, config.max_delay_ms);
                black_box(total_delay);
            }
        });
    });

    group.finish();
}

/// Benchmark ANSI code stripping (from ssh module)
fn bench_ansi_stripping(c: &mut Criterion) {
    use easyssh_core::ssh::strip_ansi_codes;

    let mut group = c.benchmark_group("ssh_ansi_strip");

    let simple_ansi = "\x1b[31mRed Text\x1b[0m";
    let complex_ansi = "\x1b[1;31mBold Red\x1b[0m \x1b[32mGreen\x1b[0m \x1b[33mYellow\x1b[0m";
    let real_world_ansi =
        "\x1b[0m\x1b[01;34mtest_dir\x1b[0m\x1b[0m  \x1b[01;32mscript.sh\x1b[0m\x1b[0m";
    let no_ansi = "Plain text without any ANSI codes";

    group.bench_with_input("simple", &simple_ansi, |b, input| {
        b.iter(|| {
            let _ = black_box(strip_ansi_codes(input));
        });
    });

    group.bench_with_input("complex", &complex_ansi, |b, input| {
        b.iter(|| {
            let _ = black_box(strip_ansi_codes(input));
        });
    });

    group.bench_with_input("real_world", &real_world_ansi, |b, input| {
        b.iter(|| {
            let _ = black_box(strip_ansi_codes(input));
        });
    });

    group.bench_with_input("no_ansi", &no_ansi, |b, input| {
        b.iter(|| {
            let _ = black_box(strip_ansi_codes(input));
        });
    });

    group.finish();
}

// Criterion group configuration
criterion_group!(
    name = ssh_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(2));
    targets =
        bench_session_manager_creation,
        bench_enhanced_pool,
        bench_connection_health,
        bench_session_metadata,
        bench_pool_stats,
        bench_server_key,
        bench_pool_cleanup,
        bench_command_preparation,
        bench_connection_metrics,
        bench_reconnect_config,
        bench_ansi_stripping
);

criterion_main!(ssh_benches);
