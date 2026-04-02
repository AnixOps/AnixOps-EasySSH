//! Search and Filter Performance Benchmarks
//!
//! This module benchmarks search operations and response times in EasySSH.
//!
//! # Benchmark Scenarios
//!
//! - Server name search (prefix, contains, fuzzy)
//! - Multi-field search (host, username)
//! - Filter operations (by group, status)
//! - Sorting performance
//! - Combined search + filter + sort operations
//! - Search result pagination
//!
//! # Running Benchmarks
//!
//! ```bash
//! cargo bench --bench search_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use easyssh_core::models::{AuthMethod, Server, ServerStatus};

/// Generate test servers with various attributes
fn generate_test_servers(count: usize) -> Vec<Server> {
    let groups = ["production", "staging", "development", "testing"];

    (0..count)
        .map(|i| {
            let group = groups[i % groups.len()];

            Server {
                id: format!("server-{}", i),
                name: format!(
                    "{}-server-{:04}",
                    if i % 3 == 0 {
                        "prod"
                    } else if i % 3 == 1 {
                        "staging"
                    } else {
                        "dev"
                    },
                    i
                ),
                host: format!("192.168.{}.{}", i / 256, i % 256),
                port: 22,
                username: if i % 2 == 0 {
                    "root".to_string()
                } else {
                    "admin".to_string()
                },
                auth_method: AuthMethod::Password {
                    password: String::new(),
                },
                group_id: Some(group.to_string()),
                status: if i % 10 == 0 {
                    ServerStatus::Offline
                } else {
                    ServerStatus::Online
                },
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                schema_version: 1,
            }
        })
        .collect()
}

/// Benchmark server name search operations
fn bench_name_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("name_search");
    let servers = generate_test_servers(1000);

    // Prefix search
    group.bench_function("prefix_match", |b| {
        let query = "prod";
        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| s.name.starts_with(black_box(query)))
                .collect();
            black_box(results);
        });
    });

    // Contains search
    group.bench_function("contains_match", |b| {
        let query = "server";
        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| s.name.contains(black_box(query)))
                .collect();
            black_box(results);
        });
    });

    // Case-insensitive search
    group.bench_function("case_insensitive", |b| {
        let query = "PROD";
        let query_lower = query.to_lowercase();
        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| s.name.to_lowercase().contains(&query_lower))
                .collect();
            black_box(results);
        });
    });

    // Exact match
    group.bench_function("exact_match", |b| {
        let query = "prod-server-0000";
        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| s.name == black_box(query))
                .collect();
            black_box(results);
        });
    });

    group.finish();
}

/// Benchmark multi-field search
fn bench_multi_field_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_field_search");
    let servers = generate_test_servers(1000);

    // Search by host
    group.bench_function("by_host", |b| {
        let query = "192.168.1.";
        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| s.host.contains(black_box(query)))
                .collect();
            black_box(results);
        });
    });

    // Search by username
    group.bench_function("by_username", |b| {
        let query = "root";
        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| s.username == black_box(query))
                .collect();
            black_box(results);
        });
    });

    // Search across all fields
    group.bench_function("all_fields", |b| {
        let query = "prod";
        let query_lower = query.to_lowercase();
        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| {
                    s.name.to_lowercase().contains(&query_lower)
                        || s.host.contains(query)
                        || s.username.to_lowercase().contains(&query_lower)
                })
                .collect();
            black_box(results);
        });
    });

    group.finish();
}

/// Benchmark filter operations
fn bench_filter_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_operations");
    let servers = generate_test_servers(1000);

    // Filter by group
    group.bench_function("by_group", |b| {
        let group = "production";
        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| s.group_id.as_ref() == Some(&black_box(group).to_string()))
                .collect();
            black_box(results);
        });
    });

    // Filter by status
    group.bench_function("by_status", |b| {
        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| s.status == ServerStatus::Online)
                .collect();
            black_box(results);
        });
    });

    // Combined filters
    group.bench_function("combined_filters", |b| {
        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| {
                    s.status == ServerStatus::Online
                        && s.group_id.as_ref() == Some(&"production".to_string())
                })
                .collect();
            black_box(results);
        });
    });

    group.finish();
}

/// Benchmark sorting operations
fn bench_sorting(c: &mut Criterion) {
    let mut group = c.benchmark_group("sorting");

    for size in [100, 500, 1000, 5000].iter() {
        let servers = generate_test_servers(*size);

        group.throughput(Throughput::Elements(*size as u64));

        // Sort by name
        group.bench_with_input(BenchmarkId::new("by_name", size), &servers, |b, servers| {
            b.iter(|| {
                let mut sorted = servers.clone();
                sorted.sort_by(|a, b| a.name.cmp(&b.name));
                black_box(sorted);
            });
        });

        // Sort by host
        group.bench_with_input(BenchmarkId::new("by_host", size), &servers, |b, servers| {
            b.iter(|| {
                let mut sorted = servers.clone();
                sorted.sort_by(|a, b| a.host.cmp(&b.host));
                black_box(sorted);
            });
        });

        // Sort by status then name
        group.bench_with_input(
            BenchmarkId::new("by_status_then_name", size),
            &servers,
            |b, servers| {
                b.iter(|| {
                    let mut sorted = servers.clone();
                    sorted.sort_by(|a, b| {
                        let status_cmp = a.status.as_str().cmp(b.status.as_str());
                        if status_cmp != std::cmp::Ordering::Equal {
                            status_cmp
                        } else {
                            a.name.cmp(&b.name)
                        }
                    });
                    black_box(sorted);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark combined search, filter, and sort
fn bench_combined_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("combined_operations");

    for size in [100, 500, 1000].iter() {
        let servers = generate_test_servers(*size);

        group.throughput(Throughput::Elements(*size as u64));

        // Search + Filter
        group.bench_with_input(
            BenchmarkId::new("search_and_filter", size),
            &servers,
            |b, servers| {
                b.iter(|| {
                    let results: Vec<_> = servers
                        .iter()
                        .filter(|s| {
                            s.name.contains("prod")
                                && s.status == ServerStatus::Online
                                && s.group_id.as_ref() == Some(&"production".to_string())
                        })
                        .collect();
                    black_box(results);
                });
            },
        );

        // Search + Filter + Sort
        group.bench_with_input(
            BenchmarkId::new("search_filter_sort", size),
            &servers,
            |b, servers| {
                b.iter(|| {
                    let mut results: Vec<_> = servers
                        .iter()
                        .filter(|s| s.name.contains("prod") && s.status == ServerStatus::Online)
                        .cloned()
                        .collect();
                    results.sort_by(|a, b| a.name.cmp(&b.name));
                    black_box(results);
                });
            },
        );

        // Complex query
        group.bench_with_input(
            BenchmarkId::new("complex_query", size),
            &servers,
            |b, servers| {
                b.iter(|| {
                    let mut results: Vec<_> = servers
                        .iter()
                        .filter(|s| {
                            // Search in multiple fields
                            let matches_search = s.name.contains("server")
                                || s.host.contains("192.168.1")
                                || s.username.contains("root");

                            // Filter by criteria
                            let matches_filter = s.status == ServerStatus::Online && s.port == 22;

                            matches_search && matches_filter
                        })
                        .cloned()
                        .collect();

                    // Sort by multiple criteria
                    results.sort_by(|a, b| {
                        let group_cmp = a.group_id.cmp(&b.group_id);
                        if group_cmp != std::cmp::Ordering::Equal {
                            group_cmp
                        } else {
                            a.name.cmp(&b.name)
                        }
                    });

                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark pagination operations
fn bench_pagination(c: &mut Criterion) {
    let mut group = c.benchmark_group("pagination");
    let servers = generate_test_servers(1000);

    for page_size in [10, 25, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("slice", page_size),
            page_size,
            |b, &page_size| {
                let page = 3; // Get page 3
                b.iter(|| {
                    let start = page * page_size;
                    let end = std::cmp::min(start + page_size, servers.len());
                    let page_data: Vec<_> = servers[start..end].to_vec();
                    black_box(page_data);
                });
            },
        );
    }

    // Benchmark pagination with filter
    for page_size in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("filter_and_slice", page_size),
            page_size,
            |b, &page_size| {
                b.iter(|| {
                    let filtered: Vec<_> = servers
                        .iter()
                        .filter(|s| s.status == ServerStatus::Online)
                        .take(page_size)
                        .cloned()
                        .collect();
                    black_box(filtered);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark fuzzy search operations
fn bench_fuzzy_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuzzy_search");
    let servers = generate_test_servers(500);

    // Simple fuzzy match (character presence)
    group.bench_function("character_presence", |b| {
        let query = "prd srv"; // Missing vowels
        let query_chars: Vec<char> = query.chars().filter(|c| !c.is_whitespace()).collect();

        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| {
                    let name_lower = s.name.to_lowercase();
                    query_chars.iter().all(|c| name_lower.contains(*c))
                })
                .collect();
            black_box(results);
        });
    });

    // Levenshtein-like distance (simplified)
    group.bench_function("approximate_match", |b| {
        let query = "prod-sever"; // Typo: sever instead of server
        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| {
                    // Simple approximate match: check if 80% of characters match
                    let name_lower = s.name.to_lowercase();
                    let matches = query.chars().filter(|c| name_lower.contains(*c)).count();
                    matches as f64 / query.len() as f64 > 0.7
                })
                .collect();
            black_box(results);
        });
    });

    group.finish();
}

/// Benchmark response time for different search scenarios
fn bench_response_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_time");

    // Simulate fast path (exact match with index)
    group.bench_function("fast_path", |b| {
        let servers = generate_test_servers(100);
        b.iter(|| {
            let id = "server-50";
            let result = servers.iter().find(|s| s.id == id);
            black_box(result);
        });
    });

    // Simulate medium path (filter with early termination)
    group.bench_function("medium_path", |b| {
        let servers = generate_test_servers(500);
        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| s.group_id.as_ref() == Some(&"production".to_string()))
                .take(10)
                .collect();
            black_box(results);
        });
    });

    // Simulate slow path (full scan with complex filter)
    group.bench_function("slow_path", |b| {
        let servers = generate_test_servers(1000);
        b.iter(|| {
            let results: Vec<_> = servers
                .iter()
                .filter(|s| s.username.to_lowercase().contains("admin"))
                .cloned()
                .collect();
            let mut sorted = results;
            sorted.sort_by(|a, b| a.name.cmp(&b.name));
            black_box(sorted);
        });
    });

    group.finish();
}

// Criterion group configuration
criterion_group!(
    name = search_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(2));
    targets =
        bench_name_search,
        bench_multi_field_search,
        bench_filter_operations,
        bench_sorting,
        bench_combined_operations,
        bench_pagination,
        bench_fuzzy_search,
        bench_response_time
);

criterion_main!(search_benches);
