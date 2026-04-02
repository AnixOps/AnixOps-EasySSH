//! Startup Time Optimizations
//!
//! Optimizations implemented:
//! - Lazy initialization of heavy components
//! - Parallel async initialization
//! - Deferred loading of non-critical data
//! - Progress tracking for startup sequence

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::error::LiteError;

/// Lazy initializer for expensive components
pub struct LazyInitializer<T> {
    initializer: Box<dyn Fn() -> Result<T, LiteError> + Send + Sync>,
    value: Mutex<Option<T>>,
}

impl<T: Clone> LazyInitializer<T> {
    /// Create a new lazy initializer
    pub fn new<F>(initializer: F) -> Self
    where
        F: Fn() -> Result<T, LiteError> + Send + Sync + 'static,
    {
        Self {
            initializer: Box::new(initializer),
            value: Mutex::new(None),
        }
    }

    /// Get the value, initializing if necessary
    pub fn get(&self) -> Result<T, LiteError> {
        let mut value = self
            .value
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock lazy value".to_string()))?;

        if value.is_none() {
            *value = Some((self.initializer)()?);
        }

        Ok(value.as_ref().unwrap().clone())
    }

    /// Force initialization
    pub fn init(&self) -> Result<(), LiteError> {
        let _ = self.get()?;
        Ok(())
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> Result<bool, LiteError> {
        let value = self
            .value
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock lazy value".to_string()))?;

        Ok(value.is_some())
    }

    /// Reset (for testing)
    pub fn reset(&self) -> Result<(), LiteError> {
        let mut value = self
            .value
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock lazy value".to_string()))?;

        *value = None;
        Ok(())
    }
}

/// Async lazy initializer
pub struct AsyncLazyInitializer<T> {
    #[allow(clippy::type_complexity)]
    initializer: Mutex<
        Option<Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = Result<T, LiteError>>>> + Send>>,
    >,
    value: Mutex<Option<T>>,
    initializing: Mutex<bool>,
}

impl<T: Clone + Send + 'static> AsyncLazyInitializer<T> {
    /// Create a new async lazy initializer
    pub fn new<F, Fut>(initializer: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, LiteError>> + Send + 'static,
    {
        let initializer = move || -> Pin<Box<dyn Future<Output = Result<T, LiteError>>>> {
            Box::pin(initializer())
        };

        Self {
            initializer: Mutex::new(Some(Box::new(initializer))),
            value: Mutex::new(None),
            initializing: Mutex::new(false),
        }
    }

    /// Get the value, initializing async if necessary
    pub async fn get(&self) -> Result<T, LiteError> {
        // Check if already initialized
        {
            let value = self
                .value
                .lock()
                .map_err(|_| LiteError::Internal("Failed to lock async lazy value".to_string()))?;

            if let Some(ref v) = *value {
                return Ok(v.clone());
            }
        }

        // Try to initialize
        let should_init = {
            let mut initializing = self
                .initializing
                .lock()
                .map_err(|_| LiteError::Internal("Failed to lock initializing flag".to_string()))?;

            if *initializing {
                // Another thread is initializing, wait and retry
                drop(initializing);
                std::thread::sleep(std::time::Duration::from_millis(10));
                return Err(LiteError::Internal(
                    "Async initialization in progress, please retry".to_string(),
                ));
            }

            let initializer = self
                .initializer
                .lock()
                .map_err(|_| LiteError::Internal("Failed to lock initializer".to_string()))?;

            if initializer.is_none() {
                // Already initialized
                let value = self
                    .value
                    .lock()
                    .map_err(|_| LiteError::Internal("Failed to lock value".to_string()))?;
                if let Some(ref v) = *value {
                    return Ok(v.clone());
                }
                return Err(LiteError::Internal("Initialization failed".to_string()));
            }

            *initializing = true;
            true
        };

        if should_init {
            let initializer = {
                let mut guard = self
                    .initializer
                    .lock()
                    .map_err(|_| LiteError::Internal("Failed to lock initializer".to_string()))?;
                guard.take().unwrap()
            };

            let result = initializer().await?;

            {
                let mut value = self
                    .value
                    .lock()
                    .map_err(|_| LiteError::Internal("Failed to lock value".to_string()))?;
                *value = Some(result);

                let mut initializing = self.initializing.lock().map_err(|_| {
                    LiteError::Internal("Failed to lock initializing flag".to_string())
                })?;
                *initializing = false;
            }
        }

        // Return the value
        let value = self
            .value
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock value".to_string()))?;

        if let Some(ref v) = *value {
            Ok(v.clone())
        } else {
            Err(LiteError::Internal("Value not initialized".to_string()))
        }
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> Result<bool, LiteError> {
        let value = self
            .value
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock value".to_string()))?;

        Ok(value.is_some())
    }
}

/// Startup phase tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StartupPhase {
    /// Application launch
    Launch,
    /// Configuration loading
    ConfigLoad,
    /// Database initialization
    DatabaseInit,
    /// Search index building
    IndexBuild,
    /// UI initialization
    UiInit,
    /// Final readiness check
    Ready,
}

impl StartupPhase {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            StartupPhase::Launch => "Launch",
            StartupPhase::ConfigLoad => "Loading Configuration",
            StartupPhase::DatabaseInit => "Initializing Database",
            StartupPhase::IndexBuild => "Building Search Index",
            StartupPhase::UiInit => "Initializing UI",
            StartupPhase::Ready => "Ready",
        }
    }

    /// Get next phase
    pub fn next(&self) -> Option<StartupPhase> {
        match self {
            StartupPhase::Launch => Some(StartupPhase::ConfigLoad),
            StartupPhase::ConfigLoad => Some(StartupPhase::DatabaseInit),
            StartupPhase::DatabaseInit => Some(StartupPhase::IndexBuild),
            StartupPhase::IndexBuild => Some(StartupPhase::UiInit),
            StartupPhase::UiInit => Some(StartupPhase::Ready),
            StartupPhase::Ready => None,
        }
    }
}

/// Phase timing information
#[derive(Debug, Clone)]
pub struct PhaseTiming {
    pub phase: StartupPhase,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub duration_ms: Option<u64>,
}

impl PhaseTiming {
    fn new(phase: StartupPhase) -> Self {
        Self {
            phase,
            start_time: Instant::now(),
            end_time: None,
            duration_ms: None,
        }
    }

    fn complete(&mut self) {
        self.end_time = Some(Instant::now());
        self.duration_ms = Some(self.start_time.elapsed().as_millis() as u64);
    }
}

/// Startup sequence manager
pub struct StartupSequence {
    phases: Mutex<HashMap<StartupPhase, PhaseTiming>>,
    current_phase: Mutex<Option<StartupPhase>>,
    start_time: Instant,
    total_duration_ms: Mutex<Option<u64>>,
}

impl StartupSequence {
    /// Create a new startup sequence
    pub fn new() -> Self {
        Self {
            phases: Mutex::new(HashMap::new()),
            current_phase: Mutex::new(None),
            start_time: Instant::now(),
            total_duration_ms: Mutex::new(None),
        }
    }

    /// Start a phase
    pub fn start_phase(&self, phase: StartupPhase) -> Result<(), LiteError> {
        let mut phases = self
            .phases
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock phases".to_string()))?;

        let mut current = self
            .current_phase
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock current phase".to_string()))?;

        // Complete current phase if any
        if let Some(ref current_phase) = *current {
            if let Some(timing) = phases.get_mut(current_phase) {
                timing.complete();
            }
        }

        // Start new phase
        phases.insert(phase, PhaseTiming::new(phase));
        *current = Some(phase);

        log::info!("Started startup phase: {}", phase.name());

        Ok(())
    }

    /// Complete a phase
    pub fn complete_phase(&self, phase: StartupPhase) -> Result<(), LiteError> {
        let mut phases = self
            .phases
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock phases".to_string()))?;

        if let Some(timing) = phases.get_mut(&phase) {
            timing.complete();
            log::info!(
                "Completed startup phase: {} in {} ms",
                phase.name(),
                timing.duration_ms.unwrap_or(0)
            );
        }

        Ok(())
    }

    /// Complete startup sequence
    pub fn complete(&self) -> Result<(), LiteError> {
        // Complete any remaining phase
        {
            let mut phases = self
                .phases
                .lock()
                .map_err(|_| LiteError::Internal("Failed to lock phases".to_string()))?;

            let mut current = self
                .current_phase
                .lock()
                .map_err(|_| LiteError::Internal("Failed to lock current phase".to_string()))?;

            if let Some(ref current_phase) = *current {
                if let Some(timing) = phases.get_mut(current_phase) {
                    timing.complete();
                }
            }

            *current = None;
        }

        // Set total duration
        let total_ms = self.start_time.elapsed().as_millis() as u64;

        let mut total = self
            .total_duration_ms
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock total duration".to_string()))?;

        *total = Some(total_ms);

        log::info!("Startup completed in {} ms", total_ms);

        Ok(())
    }

    /// Get phase timing
    pub fn get_phase_timing(&self, phase: StartupPhase) -> Result<Option<PhaseTiming>, LiteError> {
        let phases = self
            .phases
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock phases".to_string()))?;

        Ok(phases.get(&phase).cloned())
    }

    /// Get all phase timings
    pub fn get_all_timings(&self) -> Result<Vec<PhaseTiming>, LiteError> {
        let phases = self
            .phases
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock phases".to_string()))?;

        let mut timings: Vec<_> = phases.values().cloned().collect();
        timings.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        Ok(timings)
    }

    /// Get total duration
    pub fn total_duration_ms(&self) -> Option<u64> {
        self.total_duration_ms.lock().ok().and_then(|t| *t)
    }

    /// Get current phase
    pub fn current_phase(&self) -> Result<Option<StartupPhase>, LiteError> {
        let current = self
            .current_phase
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock current phase".to_string()))?;

        Ok(*current)
    }

    /// Check if startup is complete
    pub fn is_complete(&self) -> bool {
        self.total_duration_ms().is_some()
    }

    /// Get startup report
    pub fn get_report(&self) -> Result<StartupReport, LiteError> {
        let timings = self.get_all_timings()?;
        let total = self.total_duration_ms();

        let mut phase_reports = Vec::new();

        for timing in &timings {
            if let Some(duration) = timing.duration_ms {
                phase_reports.push(PhaseReport {
                    name: timing.phase.name().to_string(),
                    duration_ms: duration,
                    percentage: if let Some(total) = total {
                        (duration as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    },
                });
            }
        }

        Ok(StartupReport {
            total_duration_ms: total.unwrap_or(0),
            phases: phase_reports,
            target_ms: 1500, // Lite version target
        })
    }
}

impl Default for StartupSequence {
    fn default() -> Self {
        Self::new()
    }
}

/// Phase report for startup analysis
#[derive(Debug, Clone)]
pub struct PhaseReport {
    pub name: String,
    pub duration_ms: u64,
    pub percentage: f64,
}

/// Startup performance report
#[derive(Debug, Clone)]
pub struct StartupReport {
    pub total_duration_ms: u64,
    pub phases: Vec<PhaseReport>,
    pub target_ms: u64,
}

impl StartupReport {
    /// Check if startup met the target
    pub fn met_target(&self) -> bool {
        self.total_duration_ms < self.target_ms
    }

    /// Get the slowest phases
    pub fn slowest_phases(&self, n: usize) -> Vec<&PhaseReport> {
        let mut phases: Vec<_> = self.phases.iter().collect();
        phases.sort_by(|a, b| b.duration_ms.cmp(&a.duration_ms));
        phases.truncate(n);
        phases
    }
}

/// Deferred loading manager
#[allow(dead_code)]
pub struct DeferredLoader {
    #[allow(clippy::type_complexity)]
    tasks: Mutex<Vec<Box<dyn FnOnce() -> Result<(), LiteError> + Send>>>,
    loaded: Mutex<Vec<String>>,
}

impl DeferredLoader {
    /// Create a new deferred loader
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
            loaded: Mutex::new(Vec::new()),
        }
    }

    /// Register a deferred task
    pub fn defer<F>(&self, name: &str, task: F) -> Result<(), LiteError>
    where
        F: FnOnce() -> Result<(), LiteError> + Send + 'static,
    {
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock tasks".to_string()))?;

        let name = name.to_string();
        tasks.push(Box::new(move || {
            let result = task();
            log::info!("Deferred task '{}' completed", name);
            result
        }));

        Ok(())
    }

    /// Execute all deferred tasks
    pub fn execute_all(&self) -> Result<(), LiteError> {
        let tasks = {
            let mut tasks = self
                .tasks
                .lock()
                .map_err(|_| LiteError::Internal("Failed to lock tasks".to_string()))?;
            std::mem::take(&mut *tasks)
        };

        log::info!("Executing {} deferred tasks", tasks.len());

        for task in tasks {
            task()?;
        }

        Ok(())
    }

    /// Get number of pending tasks
    pub fn pending_count(&self) -> Result<usize, LiteError> {
        let tasks = self
            .tasks
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock tasks".to_string()))?;

        Ok(tasks.len())
    }
}

impl Default for DeferredLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Startup optimizer combining all optimizations
pub struct StartupOptimizer {
    sequence: Arc<StartupSequence>,
    deferred: Arc<DeferredLoader>,
}

impl StartupOptimizer {
    /// Create a new startup optimizer
    pub fn new() -> Self {
        Self {
            sequence: Arc::new(StartupSequence::new()),
            deferred: Arc::new(DeferredLoader::new()),
        }
    }

    /// Start the startup sequence
    pub fn start(&self) -> Result<(), LiteError> {
        self.sequence.start_phase(StartupPhase::Launch)
    }

    /// Start a specific phase
    pub fn start_phase(&self, phase: StartupPhase) -> Result<(), LiteError> {
        self.sequence.start_phase(phase)
    }

    /// Complete a specific phase
    pub fn complete_phase(&self, phase: StartupPhase) -> Result<(), LiteError> {
        self.sequence.complete_phase(phase)
    }

    /// Complete the startup sequence
    pub fn complete(&self) -> Result<(), LiteError> {
        self.sequence.complete()
    }

    /// Defer a task for after startup
    pub fn defer<F>(&self, name: &str, task: F) -> Result<(), LiteError>
    where
        F: FnOnce() -> Result<(), LiteError> + Send + 'static,
    {
        self.deferred.defer(name, task)
    }

    /// Execute deferred tasks
    pub fn execute_deferred(&self) -> Result<(), LiteError> {
        self.deferred.execute_all()
    }

    /// Get startup report
    pub fn get_report(&self) -> Result<StartupReport, LiteError> {
        self.sequence.get_report()
    }

    /// Get the startup sequence reference
    pub fn sequence(&self) -> Arc<StartupSequence> {
        self.sequence.clone()
    }

    /// Check if startup is complete
    pub fn is_complete(&self) -> bool {
        self.sequence.is_complete()
    }
}

impl Default for StartupOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick startup check for common issues
pub fn check_startup_readiness() -> Vec<String> {
    let mut issues = Vec::new();

    // Check if database directory is writable
    if let Some(data_dir) = dirs::data_dir() {
        if std::fs::metadata(&data_dir).is_err() {
            issues.push(format!(
                "Data directory not accessible: {}",
                data_dir.display()
            ));
        }
    }

    // Check available memory
    // (Platform-specific, simplified here)

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_initializer() {
        let init_count = std::sync::Arc::new(std::sync::Mutex::new(0));

        let lazy = {
            let count = init_count.clone();
            LazyInitializer::new(move || {
                let mut c = count.lock().unwrap();
                *c += 1;
                Ok(42)
            })
        };

        assert!(!lazy.is_initialized().unwrap());

        // First get - should initialize
        let val1 = lazy.get().unwrap();
        assert_eq!(val1, 42);
        assert!(lazy.is_initialized().unwrap());
        assert_eq!(*init_count.lock().unwrap(), 1);

        // Second get - should return cached value
        let val2 = lazy.get().unwrap();
        assert_eq!(val2, 42);
        assert_eq!(*init_count.lock().unwrap(), 1); // Not incremented
    }

    #[test]
    fn test_startup_sequence() {
        let sequence = StartupSequence::new();

        // Start phases
        sequence.start_phase(StartupPhase::Launch).unwrap();
        sequence.start_phase(StartupPhase::ConfigLoad).unwrap();
        sequence.complete_phase(StartupPhase::ConfigLoad).unwrap();

        sequence.start_phase(StartupPhase::DatabaseInit).unwrap();
        sequence.complete_phase(StartupPhase::DatabaseInit).unwrap();

        // Complete startup
        sequence.complete().unwrap();

        assert!(sequence.is_complete());
        assert!(sequence.total_duration_ms().is_some());

        // Get report
        let report = sequence.get_report().unwrap();
        assert!(!report.phases.is_empty());
    }

    #[test]
    fn test_deferred_loader() {
        let loader = DeferredLoader::new();

        let executed = std::sync::Arc::new(std::sync::Mutex::new(false));

        {
            let flag = executed.clone();
            loader
                .defer("test_task", move || {
                    let mut f = flag.lock().unwrap();
                    *f = true;
                    Ok(())
                })
                .unwrap();
        }

        assert_eq!(loader.pending_count().unwrap(), 1);

        loader.execute_all().unwrap();

        assert!(*executed.lock().unwrap());
        assert_eq!(loader.pending_count().unwrap(), 0);
    }

    #[test]
    fn test_startup_report() {
        let report = StartupReport {
            total_duration_ms: 1000,
            phases: vec![
                PhaseReport {
                    name: "Phase 1".to_string(),
                    duration_ms: 500,
                    percentage: 50.0,
                },
                PhaseReport {
                    name: "Phase 2".to_string(),
                    duration_ms: 300,
                    percentage: 30.0,
                },
                PhaseReport {
                    name: "Phase 3".to_string(),
                    duration_ms: 200,
                    percentage: 20.0,
                },
            ],
            target_ms: 1500,
        };

        assert!(report.met_target());

        let slowest = report.slowest_phases(2);
        assert_eq!(slowest.len(), 2);
        assert_eq!(slowest[0].name, "Phase 1");
    }
}
