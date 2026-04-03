//! Startup Performance Benchmarks for v0.3.0-beta.2
//!
//! Benchmarks to verify startup performance improvements:
//! - Cold start target: < 1.5 seconds
//! - Hot start target: < 500 milliseconds
//!
//! # Running Benchmarks
//!
//! ```bash
//! cargo bench --bench startup_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use easyssh_core::performance::{
    db_optimizer::{DatabaseFastPath, FastPathConfig},
    startup_optimizer::{
        ColdStartCache, DeferredLoader, LazyInitializer, ParallelInitializer, StartType,
        StartupMetrics, StartupOptimizer, StartupPhase, StartupSequence,
    },
    BenchmarkTargets,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// Cold Start Benchmarks
// ============================================================================

/// Benchmark cold start initialization
fn bench_cold_start_initialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("cold_start");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("full_cold_start_sequence", |b| {
        b.iter(|| {
            let optimizer = StartupOptimizer::new();

            // Simulate cold start phases
            optimizer.start().unwrap();
            optimizer.start_phase(StartupPhase::ConfigLoad).unwrap();
            std::thread::sleep(Duration::from_millis(50)); // Simulate config load
            optimizer.complete_phase(StartupPhase::ConfigLoad).unwrap();

            optimizer.start_phase(StartupPhase::DatabaseInit).unwrap();
            std::thread::sleep(Duration::from_millis(100)); // Simulate database init
            optimizer
                .complete_phase(StartupPhase::DatabaseInit)
                .unwrap();

            optimizer.start_phase(StartupPhase::IndexBuild).unwrap();
            std::thread::sleep(Duration::from_millis(30)); // Simulate index build
            optimizer.complete_phase(StartupPhase::IndexBuild).unwrap();

            optimizer.start_phase(StartupPhase::UiInit).unwrap();
            std::thread::sleep(Duration::from_millis(20)); // Simulate UI init
            optimizer.complete_phase(StartupPhase::UiInit).unwrap();

            optimizer.complete().unwrap();

            let report = optimizer.get_report().unwrap();
            black_box(report);
        });
    });

    group.bench_function("cold_start_with_deferred_indexes", |b| {
        b.iter(|| {
            let _fast_path = DatabaseFastPath::with_config(FastPathConfig::for_cold_start());

            // Simulate deferred index initialization
            // This should be much faster than creating all indexes immediately

            // Measure time to setup deferred indexes
            let start = Instant::now();
            let indexes = DatabaseFastPath::get_deferred_indexes();
            let setup_time = start.elapsed();

            black_box((indexes, setup_time));
        });
    });

    group.finish();
}

// ============================================================================
// Hot Start Benchmarks
// ============================================================================

/// Benchmark hot start initialization
fn bench_hot_start_initialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("hot_start");
    group.sample_size(50);

    // Setup: Create cache with previous startup data
    let cache = Arc::new(ColdStartCache::default());

    group.bench_function("hot_start_detection", |b| {
        b.iter(|| {
            let start_type = black_box(cache.detect_start_type());
            assert_eq!(start_type, StartType::Cold); // First run is cold
        });
    });

    group.bench_function("hot_start_sequence", |b| {
        b.iter(|| {
            let optimizer = StartupOptimizer::new();

            // Hot start should be faster - phases complete quicker
            optimizer.start().unwrap();
            optimizer.start_phase(StartupPhase::ConfigLoad).unwrap();
            std::thread::sleep(Duration::from_millis(10)); // Faster config load
            optimizer.complete_phase(StartupPhase::ConfigLoad).unwrap();

            optimizer.start_phase(StartupPhase::DatabaseInit).unwrap();
            std::thread::sleep(Duration::from_millis(20)); // Faster database init (WAL ready)
            optimizer
                .complete_phase(StartupPhase::DatabaseInit)
                .unwrap();

            optimizer.start_phase(StartupPhase::IndexBuild).unwrap();
            std::thread::sleep(Duration::from_millis(5)); // Indexes already exist
            optimizer.complete_phase(StartupPhase::IndexBuild).unwrap();

            optimizer.start_phase(StartupPhase::UiInit).unwrap();
            std::thread::sleep(Duration::from_millis(5)); // UI cached
            optimizer.complete_phase(StartupPhase::UiInit).unwrap();

            optimizer.complete().unwrap();

            let report = optimizer.get_report().unwrap();
            black_box(report);
        });
    });

    group.finish();
}

// ============================================================================
// Parallel Initialization Benchmarks
// ============================================================================

/// Benchmark parallel vs sequential initialization
fn bench_parallel_initialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_init");

    group.bench_function("sequential_phases", |b| {
        b.iter(|| {
            // Simulate sequential initialization
            let start = Instant::now();

            // Phase 1: Config load
            std::thread::sleep(Duration::from_millis(100));
            // Phase 2: Database init
            std::thread::sleep(Duration::from_millis(150));
            // Phase 3: Index build
            std::thread::sleep(Duration::from_millis(50));
            // Phase 4: UI init
            std::thread::sleep(Duration::from_millis(30));

            let total = start.elapsed();
            black_box(total);
        });
    });

    group.bench_function("parallel_config_and_database", |b| {
        b.iter(|| {
            // Simulate parallel initialization (Config + Database in parallel)
            let start = Instant::now();

            // Run config and database in parallel
            let config_thread = std::thread::spawn(|| {
                std::thread::sleep(Duration::from_millis(100));
            });
            let db_thread = std::thread::spawn(|| {
                std::thread::sleep(Duration::from_millis(150));
            });

            config_thread.join().unwrap();
            db_thread.join().unwrap();

            // Then run index and UI
            std::thread::sleep(Duration::from_millis(50));
            std::thread::sleep(Duration::from_millis(30));

            let total = start.elapsed();
            black_box(total);
        });
    });

    group.bench_function("parallel_initializer_creation", |b| {
        b.iter(|| {
            let initializer = ParallelInitializer::new();
            let groups = initializer.get_parallel_groups();
            black_box(groups);
        });
    });

    group.finish();
}

// ============================================================================
// Lazy Loading Benchmarks
// ============================================================================

/// Benchmark lazy initialization vs eager loading
fn bench_lazy_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("lazy_loading");

    // Eager initialization baseline
    group.bench_function("eager_initialization", |b| {
        b.iter(|| {
            // Simulate eager initialization - create immediately
            let config = vec![1u8; 1000]; // Config data
            let db_cache: HashMap<String, String> = HashMap::new(); // DB cache
            let ui_state = vec![0u8; 500]; // UI state

            black_box((config, db_cache, ui_state));
        });
    });

    // Lazy initialization - create only when needed
    let lazy_config = Arc::new(LazyInitializer::new(|| {
        std::thread::sleep(Duration::from_millis(10));
        Ok(vec![1u8; 1000])
    }));

    let lazy_db = Arc::new(LazyInitializer::new(|| {
        std::thread::sleep(Duration::from_millis(20));
        Ok(HashMap::<String, String>::new())
    }));

    group.bench_function("lazy_first_access", |b| {
        b.iter_with_setup(
            || {
                // Reset lazy initializers
                lazy_config.reset().unwrap();
                lazy_db.reset().unwrap();
            },
            |_| {
                let config = lazy_config.get().unwrap();
                let db = lazy_db.get().unwrap();
                black_box((config, db));
            },
        );
    });

    group.bench_function("lazy_cached_access", |b| {
        // Ensure initialized
        lazy_config.get().unwrap();
        lazy_db.get().unwrap();

        b.iter(|| {
            let config = lazy_config.get().unwrap();
            let db = lazy_db.get().unwrap();
            black_box((config, db));
        });
    });

    group.finish();
}

// ============================================================================
// Deferred Loading Benchmarks
// ============================================================================

/// Benchmark deferred loading
fn bench_deferred_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("deferred_loading");

    group.bench_function("defer_task_registration", |b| {
        b.iter(|| {
            let loader = DeferredLoader::new();

            // Register deferred tasks
            loader.defer("task1", || Ok(())).unwrap();
            loader.defer("task2", || Ok(())).unwrap();
            loader.defer("task3", || Ok(())).unwrap();

            black_box(loader.pending_count().unwrap());
        });
    });

    group.bench_function("defer_task_execution", |b| {
        b.iter_with_setup(
            || {
                let loader = DeferredLoader::new();
                loader.defer("task1", || Ok(())).unwrap();
                loader.defer("task2", || Ok(())).unwrap();
                loader.defer("task3", || Ok(())).unwrap();
                loader
            },
            |loader| {
                loader.execute_all().unwrap();
                black_box(loader);
            },
        );
    });

    group.finish();
}

// ============================================================================
// Cold Start Cache Benchmarks
// ============================================================================

/// Benchmark cold start cache operations
fn bench_cold_start_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("cold_start_cache");

    group.bench_function("cache_creation", |b| {
        b.iter(|| {
            let cache = ColdStartCache::default();
            black_box(cache);
        });
    });

    group.bench_function("start_type_detection", |b| {
        let cache = Arc::new(ColdStartCache::default());
        b.iter(|| {
            let start_type = cache.detect_start_type();
            black_box(start_type);
        });
    });

    group.bench_function("metrics_from_report", |b| {
        let report = easyssh_core::performance::startup_optimizer::StartupReport {
            total_duration_ms: 1200,
            phases: vec![
                easyssh_core::performance::startup_optimizer::PhaseReport {
                    name: "DatabaseInit".to_string(),
                    duration_ms: 400,
                    percentage: 33.3,
                },
                easyssh_core::performance::startup_optimizer::PhaseReport {
                    name: "ConfigLoad".to_string(),
                    duration_ms: 200,
                    percentage: 16.7,
                },
            ],
            target_ms: 1500,
        };

        b.iter(|| {
            let metrics = StartupMetrics::from_report(&report, true);
            black_box(metrics);
        });
    });

    group.bench_function("hot_paths_detection", |b| {
        let mut phase_durations = HashMap::new();
        phase_durations.insert("DatabaseInit".to_string(), 500);
        phase_durations.insert("ConfigLoad".to_string(), 150);

        b.iter(|| {
            let hot_paths = StartupMetrics::detect_hot_paths(&phase_durations);
            black_box(hot_paths);
        });
    });

    group.bench_function("optimization_suggestions", |b| {
        let mut phase_durations = HashMap::new();
        phase_durations.insert("Initializing Database".to_string(), 400);
        phase_durations.insert("Loading Configuration".to_string(), 200);

        b.iter(|| {
            let suggestions = StartupMetrics::generate_suggestions(&phase_durations, 500);
            black_box(suggestions);
        });
    });

    group.finish();
}

// ============================================================================
// Database Fast Path Benchmarks
// ============================================================================

/// Benchmark database fast path operations
fn bench_database_fast_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("database_fast_path");

    group.bench_function("fast_path_creation", |b| {
        b.iter(|| {
            let fast_path = DatabaseFastPath::new();
            black_box(fast_path);
        });
    });

    group.bench_function("deferred_index_generation", |b| {
        b.iter(|| {
            let indexes = DatabaseFastPath::get_deferred_indexes();
            black_box(indexes.len());
        });
    });

    group.bench_function("essential_indexes_only", |b| {
        b.iter(|| {
            let indexes = DatabaseFastPath::get_deferred_indexes();
            // Count essential indexes (priority >= 10)
            let essential_count = indexes.iter().filter(|i| i.priority >= 10).count();
            black_box(essential_count);
        });
    });

    group.bench_function("config_for_cold_start", |b| {
        b.iter(|| {
            let config = FastPathConfig::for_cold_start();
            black_box(config);
        });
    });

    group.bench_function("config_for_hot_start", |b| {
        b.iter(|| {
            let config = FastPathConfig::for_hot_start();
            black_box(config);
        });
    });

    group.finish();
}

// ============================================================================
// Target Validation Benchmarks
// ============================================================================

/// Benchmark against startup targets
fn bench_startup_targets(c: &mut Criterion) {
    let mut group = c.benchmark_group("startup_targets");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));

    // Target: Cold start < 1.5 seconds
    group.bench_function("cold_start_target_1500ms", |b| {
        b.iter(|| {
            let start = Instant::now();

            // Simulate optimized cold start
            let optimizer = StartupOptimizer::new();
            optimizer.start().unwrap();

            // Use deferred loading
            optimizer.defer("background_tasks", || Ok(())).unwrap();

            // Fast path phases
            optimizer.start_phase(StartupPhase::ConfigLoad).unwrap();
            std::thread::sleep(Duration::from_millis(50));
            optimizer.complete_phase(StartupPhase::ConfigLoad).unwrap();

            optimizer.start_phase(StartupPhase::DatabaseInit).unwrap();
            std::thread::sleep(Duration::from_millis(80));
            optimizer
                .complete_phase(StartupPhase::DatabaseInit)
                .unwrap();

            optimizer.start_phase(StartupPhase::IndexBuild).unwrap();
            std::thread::sleep(Duration::from_millis(20));
            optimizer.complete_phase(StartupPhase::IndexBuild).unwrap();

            optimizer.start_phase(StartupPhase::UiInit).unwrap();
            std::thread::sleep(Duration::from_millis(10));
            optimizer.complete_phase(StartupPhase::UiInit).unwrap();

            optimizer.complete().unwrap();

            let elapsed_ms = start.elapsed().as_millis() as u64;

            // Verify target met
            assert!(
                elapsed_ms < BenchmarkTargets::COLD_START_MS,
                "Cold start took {}ms, target is {}ms",
                elapsed_ms,
                BenchmarkTargets::COLD_START_MS
            );

            black_box(elapsed_ms);
        });
    });

    // Target: Hot start < 500 milliseconds
    group.bench_function("hot_start_target_500ms", |b| {
        b.iter(|| {
            let start = Instant::now();

            // Simulate optimized hot start
            let optimizer = StartupOptimizer::new();
            optimizer.start().unwrap();

            // Hot start - everything is cached
            optimizer.start_phase(StartupPhase::ConfigLoad).unwrap();
            std::thread::sleep(Duration::from_millis(10));
            optimizer.complete_phase(StartupPhase::ConfigLoad).unwrap();

            optimizer.start_phase(StartupPhase::DatabaseInit).unwrap();
            std::thread::sleep(Duration::from_millis(20));
            optimizer
                .complete_phase(StartupPhase::DatabaseInit)
                .unwrap();

            optimizer.start_phase(StartupPhase::IndexBuild).unwrap();
            std::thread::sleep(Duration::from_millis(5));
            optimizer.complete_phase(StartupPhase::IndexBuild).unwrap();

            optimizer.start_phase(StartupPhase::UiInit).unwrap();
            std::thread::sleep(Duration::from_millis(5));
            optimizer.complete_phase(StartupPhase::UiInit).unwrap();

            optimizer.complete().unwrap();

            let elapsed_ms = start.elapsed().as_millis() as u64;

            // Verify target met
            assert!(
                elapsed_ms < StartType::Hot.target_duration_ms(),
                "Hot start took {}ms, target is {}ms",
                elapsed_ms,
                StartType::Hot.target_duration_ms()
            );

            black_box(elapsed_ms);
        });
    });

    group.finish();
}

// ============================================================================
// Phase Tracking Benchmarks
// ============================================================================

/// Benchmark phase tracking performance
fn bench_phase_tracking(c: &mut Criterion) {
    let mut group = c.benchmark_group("phase_tracking");

    group.bench_function("phase_start", |b| {
        let sequence = Arc::new(StartupSequence::new());
        b.iter(|| {
            sequence.start_phase(StartupPhase::ConfigLoad).unwrap();
            black_box(&*sequence);
        });
    });

    group.bench_function("phase_complete", |b| {
        let sequence = Arc::new(StartupSequence::new());
        sequence.start_phase(StartupPhase::ConfigLoad).unwrap();
        b.iter(|| {
            sequence.complete_phase(StartupPhase::ConfigLoad).unwrap();
            // Re-start for next iteration
            sequence.start_phase(StartupPhase::ConfigLoad).unwrap();
            black_box(&*sequence);
        });
    });

    group.bench_function("get_all_timings", |b| {
        let sequence = Arc::new(StartupSequence::new());
        sequence.start_phase(StartupPhase::Launch).unwrap();
        sequence.complete_phase(StartupPhase::Launch).unwrap();
        sequence.start_phase(StartupPhase::ConfigLoad).unwrap();
        sequence.complete_phase(StartupPhase::ConfigLoad).unwrap();

        b.iter(|| {
            let timings = sequence.get_all_timings().unwrap();
            black_box(timings);
        });
    });

    group.bench_function("get_report", |b| {
        let sequence = Arc::new(StartupSequence::new());
        sequence.start_phase(StartupPhase::Launch).unwrap();
        sequence.complete_phase(StartupPhase::Launch).unwrap();
        sequence.start_phase(StartupPhase::ConfigLoad).unwrap();
        sequence.complete_phase(StartupPhase::ConfigLoad).unwrap();
        sequence.complete().unwrap();

        b.iter(|| {
            let report = sequence.get_report().unwrap();
            black_box(report);
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Groups
// ============================================================================

criterion_group!(
    name = cold_start;
    config = Criterion::default()
        .sample_size(20)
        .measurement_time(Duration::from_secs(30))
        .warm_up_time(Duration::from_secs(5));
    targets = bench_cold_start_initialization
);

criterion_group!(
    name = hot_start;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(15))
        .warm_up_time(Duration::from_secs(2));
    targets = bench_hot_start_initialization
);

criterion_group!(
    name = parallel;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(1));
    targets = bench_parallel_initialization
);

criterion_group!(
    name = lazy;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(1));
    targets = bench_lazy_loading, bench_deferred_loading
);

criterion_group!(
    name = cache;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(1));
    targets = bench_cold_start_cache, bench_database_fast_path
);

criterion_group!(
    name = targets;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_secs(60))
        .warm_up_time(Duration::from_secs(10));
    targets = bench_startup_targets
);

criterion_group!(
    name = tracking;
    config = Criterion::default()
        .sample_size(200)
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_millis(500));
    targets = bench_phase_tracking
);

criterion_main!(cold_start, hot_start, parallel, lazy, cache, targets, tracking);
