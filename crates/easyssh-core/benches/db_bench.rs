//! Database Performance Benchmarks
//!
//! Tests SQLite database operations including CRUD, batch operations, and queries.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use easyssh_core::db::{Database, NewGroup, NewHost, NewIdentity, NewServer, NewSnippet, NewTag};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

fn setup_db() -> (TempDir, Database) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");
    let db = Database::new(db_path).unwrap();
    db.init().unwrap();
    (temp_dir, db)
}

fn bench_server_crud(c: &mut Criterion) {
    let mut group = c.benchmark_group("db_server_crud");

    group.bench_function("create", |b| {
        let (_temp_dir, db) = setup_db();
        let mut counter = 0u64;

        b.iter(|| {
            counter += 1;
            let server = NewServer {
                id: format!("server-{}", counter),
                name: format!("Server {}", counter),
                host: format!("192.168.1.{}", counter % 255),
                port: 22,
                username: "admin".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                group_id: None,
                status: "online".to_string(),
            };
            db.add_server(black_box(&server)).unwrap();
        });
    });

    group.bench_function("read", |b| {
        let (_temp_dir, db) = setup_db();

        let server = NewServer {
            id: "read-test".to_string(),
            name: "Read Test".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_type: "password".to_string(),
            identity_file: None,
            group_id: None,
            status: "online".to_string(),
        };
        db.add_server(&server).unwrap();

        b.iter(|| {
            let _ = db.get_server(black_box("read-test")).unwrap();
        });
    });

    group.bench_function("update", |b| {
        let (_temp_dir, db) = setup_db();

        let server = NewServer {
            id: "update-test".to_string(),
            name: "Update Test".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_type: "password".to_string(),
            identity_file: None,
            group_id: None,
            status: "online".to_string(),
        };
        db.add_server(&server).unwrap();

        let mut counter = 0u64;

        b.iter(|| {
            counter += 1;
            let update = easyssh_core::db::UpdateServer {
                id: "update-test".to_string(),
                name: format!("Updated {}", counter),
                host: "192.168.1.2".to_string(),
                port: 22,
                username: "admin".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                group_id: None,
                status: "offline".to_string(),
            };
            db.update_server(black_box(&update)).unwrap();
        });
    });

    group.bench_function("delete", |b| {
        let (_temp_dir, db) = setup_db();

        b.iter(|| {
            let id = format!("delete-{}", std::time::Instant::now().elapsed().as_nanos());
            let server = NewServer {
                id: id.clone(),
                name: "Delete Test".to_string(),
                host: "192.168.1.1".to_string(),
                port: 22,
                username: "admin".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                group_id: None,
                status: "online".to_string(),
            };
            db.add_server(&server).unwrap();
            db.delete_server(black_box(&id)).unwrap();
        });
    });

    group.finish();
}

fn bench_batch_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("db_batch_insert");

    for batch_size in [10, 50, 100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let (_temp_dir, db) = setup_db();

                    for i in 0..batch_size {
                        let server = NewServer {
                            id: format!("batch-server-{}", i),
                            name: format!("Server {}", i),
                            host: format!("192.168.{}.{}", i / 255, i % 255),
                            port: 22,
                            username: "admin".to_string(),
                            auth_type: "password".to_string(),
                            identity_file: None,
                            group_id: None,
                            status: "online".to_string(),
                        };
                        db.add_server(&server).unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_query_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("db_query");

    for record_count in [100, 500, 1000, 5000] {
        let (_temp_dir, db) = setup_db();

        for i in 0..record_count {
            let server = NewServer {
                id: format!("query-server-{}", i),
                name: format!("Server {}", i),
                host: format!("192.168.{}.{}", i / 255, i % 255),
                port: 22,
                username: "admin".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                group_id: None,
                status: if i % 2 == 0 { "online" } else { "offline" }.to_string(),
            };
            db.add_server(&server).unwrap();
        }

        group.bench_with_input(
            BenchmarkId::new("list_all", record_count),
            &record_count,
            |b, _| {
                b.iter(|| {
                    let _ = db.get_servers().unwrap();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    db_benches,
    bench_server_crud,
    bench_batch_insert,
    bench_query_performance
);
criterion_main!(db_benches);
