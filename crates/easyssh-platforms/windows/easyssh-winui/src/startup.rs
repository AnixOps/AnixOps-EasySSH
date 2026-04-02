//! Startup Performance Profiler
//!
//! Provides timing measurements for application startup optimization.
//! Tracks cold start and hot start times with detailed breakdown.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Global startup profiler instance
static STARTUP_PROFILER: std::sync::OnceLock<Arc<Mutex<StartupProfiler>>> =
    std::sync::OnceLock::new();

/// Gets the global startup profiler instance
pub fn global_profiler() -> Arc<Mutex<StartupProfiler>> {
    STARTUP_PROFILER
        .get_or_init(|| Arc::new(Mutex::new(StartupProfiler::new())))
        .clone()
}

/// Startup phase timing tracker
#[derive(Debug, Clone)]
pub struct PhaseTiming {
    pub name: String,
    pub start: Instant,
    pub end: Option<Instant>,
    pub duration: Option<Duration>,
}

impl PhaseTiming {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
            end: None,
            duration: None,
        }
    }

    pub fn end(&mut self) -> Duration {
        self.end = Some(Instant::now());
        let duration = self.end.unwrap().duration_since(self.start);
        self.duration = Some(duration);
        duration
    }
}

/// Comprehensive startup profiler
#[derive(Debug)]
pub struct StartupProfiler {
    phases: HashMap<String, PhaseTiming>,
    current_phase: Option<String>,
    app_start: Instant,
    db_initialized: bool,
    ui_ready: bool,
}

impl StartupProfiler {
    pub fn new() -> Self {
        Self {
            phases: HashMap::new(),
            current_phase: None,
            app_start: Instant::now(),
            db_initialized: false,
            ui_ready: false,
        }
    }

    /// Start a new timing phase
    pub fn start_phase(&mut self, name: impl Into<String>) -> String {
        let name = name.into();
        let timing = PhaseTiming::new(name.clone());
        self.phases.insert(name.clone(), timing);
        self.current_phase = Some(name.clone());
        tracing::info!("[Startup] Phase '{}' started", name);
        name
    }

    /// End the current phase
    pub fn end_phase(&mut self, name: &str) -> Duration {
        if let Some(timing) = self.phases.get_mut(name) {
            let duration = timing.end();
            tracing::info!("[Startup] Phase '{}' completed in {:?}", name, duration);
            duration
        } else {
            Duration::ZERO
        }
    }

    /// Mark database as initialized
    pub fn mark_db_initialized(&mut self) {
        self.db_initialized = true;
        let elapsed = self.app_start.elapsed();
        tracing::info!("[Startup] Database initialized after {:?}", elapsed);
    }

    /// Mark UI as ready
    pub fn mark_ui_ready(&mut self) {
        self.ui_ready = true;
        let elapsed = self.app_start.elapsed();
        tracing::info!("[Startup] UI ready after {:?}", elapsed);
    }

    /// Get total elapsed time since app start
    pub fn total_elapsed(&self) -> Duration {
        self.app_start.elapsed()
    }

    /// Get phase timing
    pub fn get_phase(&self, name: &str) -> Option<&PhaseTiming> {
        self.phases.get(name)
    }

    /// Generate startup report
    pub fn generate_report(&self) -> StartupReport {
        let mut phases: Vec<_> = self.phases.values().collect();
        phases.sort_by(|a, b| a.start.cmp(&b.start));

        StartupReport {
            total_time: self.total_elapsed(),
            phases: phases
                .iter()
                .map(|p| PhaseReport {
                    name: p.name.clone(),
                    duration_ms: p.duration.map(|d| d.as_millis() as f64).unwrap_or(0.0),
                })
                .collect(),
            db_initialized: self.db_initialized,
            ui_ready: self.ui_ready,
        }
    }
}

/// Startup phase report entry
#[derive(Debug, Clone, serde::Serialize)]
pub struct PhaseReport {
    pub name: String,
    pub duration_ms: f64,
}

/// Complete startup performance report
#[derive(Debug, Clone, serde::Serialize)]
pub struct StartupReport {
    pub total_time: Duration,
    pub phases: Vec<PhaseReport>,
    pub db_initialized: bool,
    pub ui_ready: bool,
}

impl StartupReport {
    /// Format as human-readable string
    pub fn format(&self) -> String {
        let mut output = String::new();
        output.push_str("\n=== Startup Performance Report ===\n");
        output.push_str(&format!("Total time: {:?}\n", self.total_time));
        output.push_str(&format!("DB initialized: {}\n", self.db_initialized));
        output.push_str(&format!("UI ready: {}\n", self.ui_ready));
        output.push_str("\nPhase breakdown:\n");

        for phase in &self.phases {
            output.push_str(&format!("  - {}: {:.2}ms\n", phase.name, phase.duration_ms));
        }

        output.push_str("==================================\n");
        output
    }
}

/// Macro to time a startup phase
#[macro_export]
macro_rules! startup_phase {
    ($name:expr, $block:block) => {{
        let _profiler = $crate::startup::global_profiler();
        let phase_name = _profiler.lock().unwrap().start_phase($name);
        let result = $block;
        _profiler.lock().unwrap().end_phase(&phase_name);
        result
    }};
}

/// Lazy initialization wrapper for expensive components
pub struct LazyInit<T> {
    init_fn: Box<dyn Fn() -> T + Send + Sync>,
    value: std::sync::OnceLock<T>,
}

impl<T> LazyInit<T> {
    pub fn new<F>(init_fn: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            init_fn: Box::new(init_fn),
            value: std::sync::OnceLock::new(),
        }
    }

    pub fn get(&self) -> &T {
        self.value.get_or_init(|| (self.init_fn)())
    }

    pub fn get_or_none(&self) -> Option<&T> {
        self.value.get()
    }

    pub fn is_initialized(&self) -> bool {
        self.value.get().is_some()
    }
}

/// Deferred initialization queue for non-critical components
pub struct DeferredInitQueue {
    tasks: Vec<Box<dyn FnOnce() + Send>>,
}

impl DeferredInitQueue {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn add<F>(&mut self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.tasks.push(Box::new(task));
    }

    pub fn run_all(self) {
        std::thread::spawn(move || {
            for task in self.tasks {
                task();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_timing() {
        let mut timing = PhaseTiming::new("test");
        std::thread::sleep(Duration::from_millis(10));
        let duration = timing.end();
        assert!(duration.as_millis() >= 10);
    }

    #[test]
    fn test_lazy_init() {
        let lazy = LazyInit::new(|| {
            std::thread::sleep(Duration::from_millis(10));
            42
        });

        assert!(!lazy.is_initialized());
        let value = *lazy.get();
        assert!(lazy.is_initialized());
        assert_eq!(value, 42);
    }
}
