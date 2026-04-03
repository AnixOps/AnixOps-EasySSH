//! Database Query Performance Benchmarks
//!
//! This module benchmarks the performance of database operations used in EasySSH.
//!
//! # Benchmark Scenarios
//!
//! - Database initialization and table creation
//! - CRUD operations (Create, Read, Update, Delete)
//! - Batch insertions (servers, groups, sessions)
//! - Query performance with increasing data volumes
//! - Search and filtering operations
//! - Transaction performance
//!
//! # Running Benchmarks
//!
//! ```bash
//! cargo bench --bench db_bench
//! cargo bench --bench db_bench -- --save-baseline baseline1
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use easyssh_core::db::{Database, NewGroup, NewServer, UpdateServer};
use tempfile::TempDir;

/// Create a temporary database for benchmarking
fn create_temp_db() -> (Database, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");
    let db = Database::new(db_path).unwrap();
    db.init().unwrap();
    (db, temp_dir)
}

/// Generate a test server
fn create_test_server(index: usize) -> NewServer {
    NewServer {
        id: format!("server-{}", index),
        name: format!("Test Server {}", index),
        host: format!("192.168.1.{}", index % 256),
        port: 22,
        username: "admin".to_string(),
        auth_type: "password".to_string(),
        identity_file: None,
        group_id: None,
        status: "online".to_string(),
    }
}

/// Generate a test group
fn create_test_group(index: usize) -> NewGroup {
    NewGroup {
        id: format!("group-{}", index),
        name: format!("Group {}", index),
        color: "#FF5733".to_string(),
    }
}

/// Benchmark database initialization
fn bench_database_init(c: &mut Criterion) {
    let mut group = c.benchmark_group("db_init");

    group.bench_function("create_and_init", |b| {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("init_bench.db");

        b.iter(|| {
            let db = Database::new(black_box(db_path.clone())).unwrap();
            db.init().unwrap();
            black_box(db);
        });
    });

    group.finish();
}

/// Benchmark server CRUD operations
fn bench_server_crud(c: &mut Criterion) {
    let mut group = c.benchmark_group("server_crud");

    // Create
    group.bench_function("create", |b| {
        let (db, _temp) = create_temp_db();
        let server = create_test_server(0);

        b.iter(|| {
            db.add_server(black_box(&server)).unwrap();
            // Clean up for next iteration
            db.delete_server(&server.id).ok();
        });
    });

    // Read
    group.bench_function("read", |b| {
        let (db, _temp) = create_temp_db();
        let server = create_test_server(0);
        db.add_server(&server).unwrap();

        b.iter(|| {
            let result = db.get_server(black_box(&server.id)).unwrap();
            black_box(result);
        });
    });

    // Update
    group.bench_function("update", |b| {
        let (db, _temp) = create_temp_db();
        let server = create_test_server(0);
        db.add_server(&server).unwrap();

        let update = UpdateServer {
            id: server.id.clone(),
            name: Some("Updated Name".to_string()),
            host: None,
            port: None,
            username: None,
            auth_type: None,
            identity_file: None,
            group_id: None,
            status: None,
        };

        b.iter(|| {
            db.update_server(black_box(&update)).unwrap();
        });
    });

    // Delete
    group.bench_function("delete", |b| {
        let (db, _temp) = create_temp_db();

        b.iter(|| {
            let server = create_test_server(rand::random::<usize>());
            db.add_server(&server).unwrap();
            db.delete_server(black_box(&server.id)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark group CRUD operations
fn bench_group_crud(c: &mut Criterion) {
    let mut group = c.benchmark_group("group_crud");

    // Create
    group.bench_function("create", |b| {
        let (db, _temp) = create_temp_db();

        b.iter(|| {
            let g = create_test_group(rand::random::<usize>());
            db.add_group(black_box(&g)).unwrap();
        });
    });

    // Read all groups
    group.bench_function("read_all", |b| {
        let (db, _temp) = create_temp_db();
        // Pre-populate with some groups
        for i in 0..10 {
            db.add_group(&create_test_group(i)).unwrap();
        }

        b.iter(|| {
            let groups = db.get_groups().unwrap();
            black_box(groups);
        });
    });

    group.finish();
}

/// Benchmark batch insertions with increasing volumes
fn bench_batch_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_insert");

    for num_items in [10, 100, 1000, 5000].iter() {
        group.throughput(Throughput::Elements(*num_items as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_servers", num_items)),
            num_items,
            |b, &num| {
                b.iter_with_setup(
                    || {
                        let (db, temp) = create_temp_db();
                        (
                            db,
                            temp,
                            (0..num).map(create_test_server).collect::<Vec<_>>(),
                        )
                    },
                    |(db, _temp, servers)| {
                        for server in servers {
                            db.add_server(black_box(&server)).unwrap();
                        }
                    },
                );
            },
        );
    }

    for num_items in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*num_items as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_groups", num_items)),
            num_items,
            |b, &num| {
                b.iter_with_setup(
                    || {
                        let (db, temp) = create_temp_db();
                        (
                            db,
                            temp,
                            (0..num).map(create_test_group).collect::<Vec<_>>(),
                        )
                    },
                    |(db, _temp, groups)| {
                        for g in groups {
                            db.add_group(black_box(&g)).unwrap();
                        }
                    },
                );
            },
        );
    }

    group.finish();
}

/// Benchmark query performance with different data volumes
fn bench_query_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_performance");

    for num_servers in [100, 500, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("get_all_servers_{}", num_servers)),
            num_servers,
            |b, &num| {
                b.iter_with_setup(
                    || {
                        let (db, temp) = create_temp_db();
                        for i in 0..num {
                            db.add_server(&create_test_server(i)).unwrap();
                        }
                        (db, temp)
                    },
                    |(db, _temp)| {
                        let servers = db.get_servers().unwrap();
                        black_box(servers);
                    },
                );
            },
        );
    }

    // Benchmark get_server with different dataset sizes
    for num_servers in [100, 500, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("get_single_server_{}_total", num_servers)),
            num_servers,
            |b, &num| {
                b.iter_with_setup(
                    || {
                        let (db, temp) = create_temp_db();
                        for i in 0..num {
                            db.add_server(&create_test_server(i)).unwrap();
                        }
                        (db, temp, format!("server-{}", num / 2)) // Target middle item
                    },
                    |(db, _temp, target_id)| {
                        let server = db.get_server(black_box(&target_id)).unwrap();
                        black_box(server);
                    },
                );
            },
        );
    }

    group.finish();
}

/// Benchmark search operations
fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");

    // Setup database with searchable content
    let (db, _temp) = create_temp_db();
    for i in 0..1000 {
        let mut server = create_test_server(i);
        server.name = format!(
            "Server {} - {} Test",
            i,
            if i % 2 == 0 {
                "Production"
            } else {
                "Development"
            }
        );
        // Note: NewServer doesn't have notes field in the original structure, so we skip this
        db.add_server(&server).unwrap();
    }

    group.bench_function("search_by_name_prefix", |b| {
        b.iter(|| {
            // Simulate name prefix search
            let results: Vec<_> = db
                .get_servers()
                .unwrap()
                .into_iter()
                .filter(|s| s.name.starts_with("Server 5"))
                .collect();
            black_box(results);
        });
    });

    group.bench_function("search_by_name_contains", |b| {
        b.iter(|| {
            let results: Vec<_> = db
                .get_servers()
                .unwrap()
                .into_iter()
                .filter(|s| s.name.contains("Production"))
                .collect();
            black_box(results);
        });
    });

    group.bench_function("search_all_fields", |b| {
        b.iter(|| {
            let results: Vec<_> = db
                .get_servers()
                .unwrap()
                .into_iter()
                .filter(|s| s.name.contains("500") || s.host.contains("500"))
                .collect();
            black_box(results);
        });
    });

    group.finish();
}

/// Benchmark transaction batch operations
fn bench_transactions(c: &mut Criterion) {
    let mut group = c.benchmark_group("transactions");

    group.bench_function("batch_insert_transaction", |b| {
        b.iter_with_setup(
            || {
                let (db, temp) = create_temp_db();
                let servers: Vec<_> = (0..100).map(create_test_server).collect();
                (db, temp, servers)
            },
            |(db, _temp, servers)| {
                // Simulate transaction-like behavior with bulk insert
                for server in servers {
                    db.add_server(black_box(&server)).unwrap();
                }
            },
        );
    });

    group.bench_function("mixed_operations", |b| {
        b.iter_with_setup(
            || {
                let (db, temp) = create_temp_db();
                // Pre-populate
                for i in 0..50 {
                    db.add_server(&create_test_server(i)).unwrap();
                }
                (db, temp)
            },
            |(db, _temp)| {
                // Mix of operations
                db.add_server(black_box(&create_test_server(999))).unwrap();
                let _ = db.get_server(black_box("server-25")).unwrap();
                db.update_server(black_box(&UpdateServer {
                    id: "server-10".to_string(),
                    name: Some("Updated".to_string()),
                    host: None,
                    port: None,
                    username: None,
                    auth_type: None,
                    identity_file: None,
                    group_id: None,
                    status: None,
                }))
                .unwrap();
                let _ = db.get_servers().unwrap();
            },
        );
    });

    group.finish();
}

/// Benchmark database file operations
fn bench_file_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_operations");

    group.bench_function("database_size_after_insert", |b| {
        b.iter_with_setup(
            || {
                let temp_dir = TempDir::new().unwrap();
                let db_path = temp_dir.path().join("size_test.db");
                let db = Database::new(db_path.clone()).unwrap();
                db.init().unwrap();
                (db, temp_dir, db_path)
            },
            |(db, _temp, db_path)| {
                for i in 0..1000 {
                    db.add_server(black_box(&create_test_server(i))).unwrap();
                }
                let size = std::fs::metadata(&db_path).unwrap().len();
                black_box(size);
            },
        );
    });

    group.finish();
}

// Criterion group configuration
criterion_group!(
    name = db_benches;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(std::time::Duration::from_secs(15))
        .warm_up_time(std::time::Duration::from_secs(2));
    targets =
        bench_database_init,
        bench_server_crud,
        bench_group_crud,
        bench_batch_insert,
        bench_query_performance,
        bench_search,
        bench_transactions,
        bench_file_operations
);

criterion_main!(db_benches);
