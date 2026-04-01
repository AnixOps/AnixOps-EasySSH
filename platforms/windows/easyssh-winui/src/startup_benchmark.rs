//! Startup Time Benchmark Tests
//!
//! Run with: cargo test -p easyssh-winui startup_benchmark -- --nocapture

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    /// Target cold start time (2 seconds)
    const TARGET_COLD_START_MS: f64 = 2000.0;

    /// Target hot start time (500ms)
    const TARGET_HOT_START_MS: f64 = 500.0;

    /// Benchmark database initialization time
    #[test]
    fn benchmark_db_init() {
        use easyssh_core::{AppState, init_database};

        let start = Instant::now();
        let state = AppState::new();
        let db_init_time = start.elapsed();

        println!("AppState creation: {:?}", db_init_time);
        assert!(db_init_time.as_millis() < 100, "AppState creation should be fast");

        let start = Instant::now();
        init_database(&state).expect("Failed to init database");
        let db_init_time = start.elapsed();

        println!("Database initialization: {:?}", db_init_time);
        // Database init should be fast if using fast path (< 100ms)
        assert!(db_init_time.as_millis() < 500, "Database init should be < 500ms");
    }

    /// Measure simulated startup phases
    #[test]
    fn startup_phases_benchmark() {
        let total_start = Instant::now();

        // Phase 1: Early init (logging, accessibility)
        let phase1_start = Instant::now();
        std::thread::sleep(Duration::from_millis(5)); // Simulated work
        let phase1_time = phase1_start.elapsed();
        println!("Phase 1 (Early init): {:?}", phase1_time);

        // Phase 2: Tokio runtime creation
        let phase2_start = Instant::now();
        let _rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        let phase2_time = phase2_start.elapsed();
        println!("Phase 2 (Runtime creation): {:?}", phase2_time);
        assert!(phase2_time.as_millis() < 100, "Runtime creation should be < 100ms");

        // Phase 3: AppViewModel initialization (includes DB)
        let phase3_start = Instant::now();
        // Simulated AppViewModel creation
        std::thread::sleep(Duration::from_millis(50));
        let phase3_time = phase3_start.elapsed();
        println!("Phase 3 (ViewModel init): {:?}", phase3_time);

        // Phase 4: UI initialization
        let phase4_start = Instant::now();
        // Simulated UI setup
        std::thread::sleep(Duration::from_millis(30));
        let phase4_time = phase4_start.elapsed();
        println!("Phase 4 (UI init): {:?}", phase4_time);

        let total_time = total_start.elapsed();
        println!("\nTotal simulated startup: {:?}", total_time);

        // Report compliance with targets
        let total_ms = total_time.as_millis() as f64;
        if total_ms < TARGET_HOT_START_MS {
            println!("✅ HOT START TARGET MET: {:.0}ms < {:.0}ms", total_ms, TARGET_HOT_START_MS);
        } else if total_ms < TARGET_COLD_START_MS {
            println!("✅ COLD START TARGET MET: {:.0}ms < {:.0}ms", total_ms, TARGET_COLD_START_MS);
        } else {
            println!("❌ STARTUP TOO SLOW: {:.0}ms > {:.0}ms target", total_ms, TARGET_COLD_START_MS);
        }
    }

    /// Test lazy initialization pattern
    #[test]
    fn test_lazy_init() {
        use std::sync::OnceLock;

        static LAZY_VALUE: OnceLock<String> = OnceLock::new();

        let start = Instant::now();
        // First access - initialization happens
        let value1 = LAZY_VALUE.get_or_init(|| {
            std::thread::sleep(Duration::from_millis(50));
            "expensive_value".to_string()
        });
        let first_access_time = start.elapsed();

        let start = Instant::now();
        // Second access - already initialized
        let value2 = LAZY_VALUE.get_or_init(|| {
            std::thread::sleep(Duration::from_millis(50));
            "should_not_happen".to_string()
        });
        let second_access_time = start.elapsed();

        assert_eq!(value1, value2);
        println!("First access (with init): {:?}", first_access_time);
        println!("Second access (cached): {:?}", second_access_time);

        // Second access should be much faster
        assert!(second_access_time.as_micros() < first_access_time.as_micros() / 10,
            "Lazy init should cache the value");
    }
}
