//! SSH Connection Performance Benchmarks
//!
//! Tests SSH connection pooling, session management, and command execution performance.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use easyssh_core::ssh::{
    ConnectionHealth, ConnectionInfo, PoolInfo, PoolStats, SessionMetadata, SshSessionManager,
};
use std::time::Instant;

fn bench_session_manager_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ssh_manager_creation");

    group.bench_function("new", |b| {
        b.iter(|| {
            let _ = black_box(SshSessionManager::new());
        });
    });

    group.bench_function("with_pool_config", |b| {
        b.iter(|| {
            let _ = black_box(SshSessionManager::new().with_pool_config(10, 600, 3600));
        });
    });

    group.finish();
}

fn bench_connection_pool_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ssh_pool_simulation");

    group.bench_function("cleanup_expired_empty", |b| {
        let mut manager = SshSessionManager::new();
        b.iter(|| {
            black_box(manager.cleanup_expired());
        });
    });

    group.bench_function("list_sessions_empty", |b| {
        let manager = SshSessionManager::new();
        b.iter(|| {
            let _ = black_box(manager.list_sessions());
        });
    });

    group.bench_function("get_pool_stats_empty", |b| {
        let manager = SshSessionManager::new();
        b.iter(|| {
            let _ = black_box(manager.get_pool_stats());
        });
    });

    group.finish();
}

fn bench_session_metadata_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("ssh_metadata");

    group.bench_function("create", |b| {
        b.iter(|| {
            let metadata = SessionMetadata {
                id: "sess-123".to_string(),
                server_id: "server-1".to_string(),
                host: "192.168.1.1".to_string(),
                port: 22,
                username: "root".to_string(),
                connected_at: Instant::now(),
            };
            black_box(metadata);
        });
    });

    group.bench_function("clone", |b| {
        let metadata = SessionMetadata {
            id: "sess-123".to_string(),
            server_id: "server-1".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "root".to_string(),
            connected_at: Instant::now(),
        };

        b.iter(|| {
            let _ = black_box(metadata.clone());
        });
    });

    group.finish();
}

fn create_pool_stats(pool_count: usize) -> PoolStats {
    let pools: Vec<PoolInfo> = (0..pool_count)
        .map(|i| PoolInfo {
            server: format!("root@192.168.{}.{}", i / 255, i % 255),
            connection_count: 4,
            connections: vec![
                ConnectionInfo {
                    age_secs: 100,
                    idle_secs: 10,
                    health: "Healthy".to_string(),
                    busy: false,
                },
                ConnectionInfo {
                    age_secs: 200,
                    idle_secs: 20,
                    health: "Healthy".to_string(),
                    busy: true,
                },
            ],
        })
        .collect();

    PoolStats {
        total_pools: pool_count,
        total_sessions: pool_count * 4,
        pools,
    }
}

fn bench_pool_stats_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("ssh_pool_stats");

    for pool_count in [1, 5, 10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("serialize", pool_count),
            &pool_count,
            |b, &pool_count| {
                let stats = create_pool_stats(pool_count);
                b.iter(|| {
                    let _ = black_box(serde_json::to_string(&stats).unwrap());
                });
            },
        );
    }

    group.finish();
}

fn bench_connection_health_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("ssh_connection_health");

    group.bench_function("enum_comparison", |b| {
        b.iter(|| {
            let h1 = ConnectionHealth::Healthy;
            let h2 = ConnectionHealth::Healthy;
            let _ = black_box(h1 == h2);
        });
    });

    group.bench_function("enum_clone", |b| {
        let health = ConnectionHealth::Degraded;
        b.iter(|| {
            let _ = black_box(health.clone());
        });
    });

    group.bench_function("debug_format", |b| {
        let health = ConnectionHealth::Healthy;
        b.iter(|| {
            let _ = black_box(format!("{:?}", health));
        });
    });

    group.finish();
}

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

criterion_group!(
    ssh_benches,
    bench_session_manager_creation,
    bench_connection_pool_simulation,
    bench_session_metadata_operations,
    bench_pool_stats_operations,
    bench_connection_health_operations,
    bench_ansi_stripping
);
criterion_main!(ssh_benches);
