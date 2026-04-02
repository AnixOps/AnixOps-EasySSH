//! Performance Optimization Benchmarks
//!
//! Benchmarks the optimized components against baseline implementations
//!
//! # Running Benchmarks
//!
//! ```bash
//! cargo bench --bench performance_opt_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use easyssh_core::performance::{
    crypto_optimizer::{CryptoOptimizer, KeyDerivationCache},
    memory_optimizer::{DataStructureGuide, MemoryOptimizer, ObjectPool},
    search_optimizer::{FastStringMatcher, InvertedIndex, SearchOptimizer},
    startup_optimizer::{LazyInitializer, StartupOptimizer},
    BenchmarkTargets, PerformanceMetrics, check_performance_targets,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Benchmark key derivation caching
fn bench_key_derivation_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_derivation_cache");

    // Baseline: Direct derivation
    group.bench_function("baseline_direct", |b| {
        b.iter(|| {
            use easyssh_core::crypto::CryptoState;
            let mut state = CryptoState::new();
            state.initialize(black_box("test_password")).unwrap();
            black_box(state);
        });
    });

    // Optimized: With cache lookup
    let cache = KeyDerivationCache::new();

    // Warm up cache
    cache.cache_derivation("cached_password", vec![1u8; 32], vec![2u8; 32]).unwrap();

    group.bench_function("cached_lookup", |b| {
        b.iter(|| {
            let cached = cache.is_cached(black_box("cached_password")).unwrap();
            black_box(cached);
        });
    });

    group.finish();
}

/// Benchmark crypto optimizer
fn bench_crypto_optimizer(c: &mut Criterion) {
    let mut group = c.benchmark_group("crypto_optimizer");
    let optimizer = CryptoOptimizer::new();

    // Create a state directly
    use easyssh_core::crypto::CryptoState;
    let mut state = CryptoState::new();
    state.initialize("bench_password").unwrap();

    for size in [1024, 4096, 16384, 65536].iter() {
        let data = vec![1u8; *size];

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("encrypt", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let encrypted = optimizer.encrypt_optimized(&state, black_box(data)).unwrap();
                    black_box(encrypted);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark object pooling
fn bench_object_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("object_pool");

    // Baseline: Direct allocation
    group.bench_function("baseline_vec", |b| {
        b.iter(|| {
            let v: Vec<u8> = Vec::with_capacity(1024);
            black_box(v);
        });
    });

    // Optimized: Pool-based allocation
    let pool = ObjectPool::new(100, || Vec::with_capacity(1024));

    group.bench_function("pooled_vec", |b| {
        b.iter(|| {
            let obj = pool.acquire().unwrap();
            black_box(&*obj);
            // Implicit drop returns to pool
        });
    });

    group.finish();
}

/// Benchmark memory optimizer
fn bench_memory_optimizer(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_optimizer");
    let optimizer = MemoryOptimizer::new();

    group.bench_function("get_buffer", |b| {
        b.iter(|| {
            let buf = optimizer.get_buffer().unwrap();
            black_box(&*buf);
        });
    });

    group.bench_function("get_string", |b| {
        b.iter(|| {
            let s = optimizer.get_string().unwrap();
            black_box(&*s);
        });
    });

    // Check allocation limits
    group.bench_function("check_allocation", |b| {
        b.iter(|| {
            let result = optimizer.check_allocation(black_box(1024 * 1024)).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark inverted index
fn bench_inverted_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("inverted_index");

    // Setup: Create index with documents
    let index = InvertedIndex::new();
    for i in 0..1000 {
        let mut fields = HashMap::new();
        fields.insert(
            "name".to_string(),
            format!("Server {} - Production Web", i),
        );
        fields.insert("host".to_string(), format!("192.168.{}.{} host", i / 256, i % 256));
        index.add_document(&format!("doc{}", i), &fields).unwrap();
    }

    group.bench_function("search_single_term", |b| {
        b.iter(|| {
            let results = index.search(black_box("production")).unwrap();
            black_box(results);
        });
    });

    group.bench_function("search_multi_term", |b| {
        b.iter(|| {
            let results = index.search(black_box("production web")).unwrap();
            black_box(results);
        });
    });

    group.bench_function("search_not_found", |b| {
        b.iter(|| {
            let results = index.search(black_box("nonexistent")).unwrap();
            black_box(results);
        });
    });

    // Benchmark document insertion
    group.bench_function("add_document", |b| {
        let mut counter = 0;
        b.iter(|| {
            let mut fields = HashMap::new();
            fields.insert("name".to_string(), format!("New Server {}", counter));
            counter += 1;
            index
                .add_document(&format!("new_doc{}", counter), &fields)
                .unwrap();
        });
    });

    group.finish();
}

/// Benchmark search optimizer
fn bench_search_optimizer(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_optimizer");
    let search = SearchOptimizer::new();

    // Setup: Index some hosts
    for i in 0..100 {
        let host = easyssh_core::db::HostRecord {
            id: format!("host-{}", i),
            name: format!("Production Server {}", i),
            host: format!("192.168.{}.{} hostname", i / 256, i % 256),
            port: 22,
            username: "admin".to_string(),
            auth_type: "key".to_string(),
            identity_file: None,
            identity_id: None,
            group_id: Some("prod".to_string()),
            notes: Some(format!("Notes for server {}", i)),
            color: None,
            environment: Some("production".to_string()),
            region: Some("us-east".to_string()),
            purpose: Some("web".to_string()),
            status: "online".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        search.index_host(&host).unwrap();
    }

    group.bench_function("prefix_search", |b| {
        b.iter(|| {
            let results = search.prefix_search(black_box("Prod"), 10).unwrap();
            black_box(results);
        });
    });

    group.bench_function("full_text_search", |b| {
        b.iter(|| {
            let results = search.full_text_search(black_box("production"), 10).unwrap();
            black_box(results);
        });
    });

    group.finish();
}

/// Benchmark fast string matching
fn bench_fast_string_matcher(c: &mut Criterion) {
    let mut group = c.benchmark_group("fast_string_matcher");

    let haystack = "Production Web Server 01 - us-east-1";
    let needle = "production";

    group.bench_function("contains", |b| {
        b.iter(|| {
            let result = FastStringMatcher::contains(black_box(haystack), black_box(needle));
            black_box(result);
        });
    });

    group.bench_function("starts_with", |b| {
        b.iter(|| {
            let result = FastStringMatcher::starts_with(black_box(haystack), black_box("Prod"));
            black_box(result);
        });
    });

    group.bench_function("fuzzy_match", |b| {
        b.iter(|| {
            let result = FastStringMatcher::fuzzy_match(black_box(haystack), black_box("prd srv"));
            black_box(result);
        });
    });

    group.bench_function("fuzzy_score", |b| {
        b.iter(|| {
            let result = FastStringMatcher::fuzzy_score(black_box(haystack), black_box("prod"));
            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark lazy initialization
fn bench_lazy_initializer(c: &mut Criterion) {
    let mut group = c.benchmark_group("lazy_initializer");

    // Setup: Create lazy initializers
    let expensive_init = LazyInitializer::new(|| {
        // Simulate expensive operation
        std::thread::sleep(Duration::from_millis(1));
        Ok(vec![1u8; 1000])
    });

    let cheap_init = LazyInitializer::new(|| Ok(vec![1u8; 100]));

    group.bench_function("expensive_first_access", |b| {
        // Reset before each iteration
        b.iter_with_setup(
            || expensive_init.reset().unwrap(),
            |_| {
                let value = expensive_init.get().unwrap();
                black_box(value);
            },
        );
    });

    group.bench_function("expensive_cached_access", |b| {
        // Ensure initialized
        expensive_init.get().unwrap();

        b.iter(|| {
            let value = expensive_init.get().unwrap();
            black_box(value);
        });
    });

    group.bench_function("cheap_first_access", |b| {
        b.iter_with_setup(
            || cheap_init.reset().unwrap(),
            |_| {
                let value = cheap_init.get().unwrap();
                black_box(value);
            },
        );
    });

    group.finish();
}

/// Benchmark data structure recommendations
fn bench_data_structure_guide(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_structure_guide");

    group.bench_function("recommended_vec_capacity", |b| {
        b.iter(|| {
            let cap = DataStructureGuide::recommended_vec_capacity(black_box(100));
            black_box(cap);
        });
    });

    group.bench_function("recommended_hashmap_capacity", |b| {
        b.iter(|| {
            let cap = DataStructureGuide::recommended_hashmap_capacity(black_box(100));
            black_box(cap);
        });
    });

    group.bench_function("estimate_vec_memory", |b| {
        b.iter(|| {
            let size = DataStructureGuide::estimate_vec_memory::<u64>(black_box(1000));
            black_box(size);
        });
    });

    group.finish();
}

/// Benchmark startup optimizer
fn bench_startup_optimizer(c: &mut Criterion) {
    let mut group = c.benchmark_group("startup_optimizer");

    group.bench_function("create_and_start", |b| {
        b.iter(|| {
            let optimizer = StartupOptimizer::new();
            optimizer.start().unwrap();
            black_box(optimizer);
        });
    });

    group.bench_function("phase_tracking", |b| {
        let optimizer = StartupOptimizer::new();
        optimizer.start().unwrap();

        b.iter(|| {
            use easyssh_core::startup_optimizer::StartupPhase;
            optimizer.start_phase(StartupPhase::ConfigLoad).unwrap();
            optimizer.complete_phase(StartupPhase::ConfigLoad).unwrap();
        });
    });

    group.finish();
}

/// Benchmark against performance targets
fn bench_performance_targets(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_targets");

    // Target: Search response < 100ms
    group.bench_function("search_response_target_100ms", |b| {
        let search = SearchOptimizer::new();

        // Setup data
        for i in 0..500 {
            let host = easyssh_core::db::HostRecord {
                id: format!("host-{}", i),
                name: format!("Server {}", i),
                host: format!("192.168.{}.{} hostname", i / 256, i % 256),
                port: 22,
                username: "admin".to_string(),
                auth_type: "key".to_string(),
                identity_file: None,
                identity_id: None,
                group_id: None,
                notes: None,
                color: None,
                environment: None,
                region: None,
                purpose: None,
                status: "online".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
            };
            search.index_host(&host).unwrap();
        }

        b.iter(|| {
            let start = Instant::now();
            let results = search.prefix_search(black_box("Server"), 20).unwrap();
            let elapsed_ms = start.elapsed().as_millis() as u64;
            black_box(results);

            // Verify target met
            assert!(
                elapsed_ms < BenchmarkTargets::SEARCH_RESPONSE_MS,
                "Search took {}ms, target is {}ms",
                elapsed_ms,
                BenchmarkTargets::SEARCH_RESPONSE_MS
            );
        });
    });

    group.finish();
}

/// Comprehensive benchmark comparing optimized vs baseline
fn bench_optimization_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimization_comparison");

    // Comparison: Database query with and without cache
    group.bench_function("query_without_cache", |b| {
        b.iter(|| {
            // Simulate uncached query
            std::thread::sleep(Duration::from_micros(50));
            black_box(vec![1u8; 100]);
        });
    });

    group.bench_function("query_with_cache", |b| {
        use easyssh_core::performance::db_optimizer::QueryCache;

        let cache: QueryCache<Vec<u8>> = QueryCache::new();

        // Warm up cache
        let _ = cache.get_or_compute("test", || Ok(vec![1u8; 100])).unwrap();

        b.iter(|| {
            let result = cache.get_or_compute("test", || Ok(vec![2u8; 100])).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

/// Memory efficiency benchmark
fn bench_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_efficiency");

    // Compare string allocation approaches
    group.bench_function("naive_string_concat", |b| {
        b.iter(|| {
            let mut s = String::new();
            for i in 0..100 {
                s.push_str(&format!("item{}", i));
            }
            black_box(s);
        });
    });

    group.bench_function("pooled_string", |b| {
        use easyssh_core::performance::memory_optimizer::StringPool;

        let pool = StringPool::new(10);

        b.iter(|| {
            let mut s = pool.acquire().unwrap();
            for i in 0..100 {
                s.push_str(&format!("item{}", i));
            }
            black_box(&*s);
        });
    });

    group.finish();
}

// Criterion configuration
criterion_group!(
    name = crypto_optimizations;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(2));
    targets =
        bench_key_derivation_cache,
        bench_crypto_optimizer
);

criterion_group!(
    name = memory_optimizations;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(1));
    targets =
        bench_object_pool,
        bench_memory_optimizer,
        bench_data_structure_guide,
        bench_memory_efficiency
);

criterion_group!(
    name = search_optimizations;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(15))
        .warm_up_time(Duration::from_secs(2));
    targets =
        bench_inverted_index,
        bench_search_optimizer,
        bench_fast_string_matcher
);

criterion_group!(
    name = startup_optimizations;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(1));
    targets =
        bench_lazy_initializer,
        bench_startup_optimizer
);

criterion_group!(
    name = target_validation;
    config = Criterion::default()
        .sample_size(30)
        .measurement_time(Duration::from_secs(20))
        .warm_up_time(Duration::from_secs(3));
    targets =
        bench_performance_targets,
        bench_optimization_comparison
);

criterion_main!(
    crypto_optimizations,
    memory_optimizations,
    search_optimizations,
    startup_optimizations,
    target_validation
);
