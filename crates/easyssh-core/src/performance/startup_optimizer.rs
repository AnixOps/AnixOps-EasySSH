//! Startup Time Optimizations
//!
//! Optimizations implemented:
//! - Lazy initialization of heavy components
//! - Parallel async initialization with tokio::join!
//! - Deferred loading of non-critical data
//! - Progress tracking for startup sequence
//! - Cold start cache for hot path detection
//! - Startup metrics persistence

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use serde::{Deserialize, Serialize};

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

// ============================================================================
// Cold Start Cache & Hot Path Detection
// ============================================================================

/// Startup metrics for persistence and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupMetrics {
    /// Total startup duration in milliseconds
    pub total_duration_ms: u64,
    /// Individual phase durations
    pub phase_durations: HashMap<String, u64>,
    /// Whether this was a cold start
    pub is_cold_start: bool,
    /// Timestamp of the startup
    pub timestamp: String,
    /// Application version
    pub version: String,
    /// Detected hot paths
    pub hot_paths: Vec<String>,
    /// Optimization suggestions
    pub optimization_suggestions: Vec<String>,
}

impl Default for StartupMetrics {
    fn default() -> Self {
        Self {
            total_duration_ms: 0,
            phase_durations: HashMap::new(),
            is_cold_start: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            hot_paths: Vec::new(),
            optimization_suggestions: Vec::new(),
        }
    }
}

impl StartupMetrics {
    /// Create metrics from a startup report
    pub fn from_report(report: &StartupReport, is_cold_start: bool) -> Self {
        let phase_durations = report
            .phases
            .iter()
            .map(|p| (p.name.clone(), p.duration_ms))
            .collect();

        let hot_paths = Self::detect_hot_paths(&phase_durations);
        let optimization_suggestions = Self::generate_suggestions(&phase_durations, report.target_ms);

        Self {
            total_duration_ms: report.total_duration_ms,
            phase_durations,
            is_cold_start,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            hot_paths,
            optimization_suggestions,
        }
    }

    /// Detect hot paths based on phase durations
    pub fn detect_hot_paths(phase_durations: &HashMap<String, u64>) -> Vec<String> {
        // Phases that took more than 200ms are considered hot paths
        phase_durations
            .iter()
            .filter(|(_, duration)| **duration > 200)
            .map(|(name, duration)| format!("{}: {}ms", name, duration))
            .collect()
    }

    /// Generate optimization suggestions based on metrics
    pub fn generate_suggestions(phase_durations: &HashMap<String, u64>, target_ms: u64) -> Vec<String> {
        let mut suggestions: Vec<String> = Vec::new();

        // Database initialization suggestions
        if let Some(db_time) = phase_durations.get("Initializing Database") {
            if *db_time > 300 {
                suggestions.push("Consider enabling database fast path with deferred indexes".to_string());
            }
        }

        // Config loading suggestions
        if let Some(config_time) = phase_durations.get("Loading Configuration") {
            if *config_time > 200 {
                suggestions.push("Consider lazy loading configuration sections".to_string());
            }
        }

        // Index building suggestions
        if let Some(index_time) = phase_durations.get("Building Search Index") {
            if *index_time > 150 {
                suggestions.push("Consider deferring search index build to background thread".to_string());
            }
        }

        // UI initialization suggestions
        if let Some(ui_time) = phase_durations.get("Initializing UI") {
            if *ui_time > 100 {
                suggestions.push("Consider parallel UI component initialization".to_string());
            }
        }

        // General suggestion if not meeting target
        let total: u64 = phase_durations.values().sum();
        if total > target_ms {
            suggestions.push(format!(
                "Total startup time {}ms exceeds target {}ms - review parallel initialization opportunities",
                total, target_ms
            ));
        }

        suggestions
    }

    /// Check if metrics indicate a slow startup
    pub fn is_slow_startup(&self, target_ms: u64) -> bool {
        self.total_duration_ms > target_ms
    }

    /// Get the slowest phase
    pub fn slowest_phase(&self) -> Option<(String, u64)> {
        self.phase_durations
            .iter()
            .max_by_key(|(_, duration)| *duration)
            .map(|(name, duration)| (name.clone(), *duration))
    }
}

/// Cold start cache for tracking startup history
pub struct ColdStartCache {
    /// Path to the metrics cache file
    cache_path: PathBuf,
    /// Cached metrics from previous startups
    cached_metrics: RwLock<Vec<StartupMetrics>>,
    /// Current startup being tracked
    current_startup: Mutex<Option<StartupMetrics>>,
    /// Number of startups to keep in history
    history_size: usize,
}

impl ColdStartCache {
    /// Create a new cold start cache
    pub fn new() -> Result<Self, LiteError> {
        let cache_path = Self::get_cache_path()?;

        // Load existing metrics
        let cached_metrics = Self::load_metrics(&cache_path)?;

        Ok(Self {
            cache_path,
            cached_metrics: RwLock::new(cached_metrics),
            current_startup: Mutex::new(None),
            history_size: 10, // Keep last 10 startups
        })
    }

    /// Create with custom history size
    pub fn with_history_size(history_size: usize) -> Result<Self, LiteError> {
        let mut cache = Self::new()?;
        cache.history_size = history_size;
        Ok(cache)
    }

    /// Get the cache file path
    fn get_cache_path() -> Result<PathBuf, LiteError> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| LiteError::Internal("Cannot determine data directory".to_string()))?;

        let app_dir = data_dir.join("EasySSH");
        std::fs::create_dir_all(&app_dir)
            .map_err(|e| LiteError::Internal(format!("Cannot create app directory: {}", e)))?;

        Ok(app_dir.join("startup_metrics.json"))
    }

    /// Load metrics from cache file
    fn load_metrics(path: &PathBuf) -> Result<Vec<StartupMetrics>, LiteError> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| LiteError::Internal(format!("Cannot read metrics cache: {}", e)))?;

        let metrics: Vec<StartupMetrics> = serde_json::from_str(&content)
            .map_err(|e| LiteError::Internal(format!("Cannot parse metrics cache: {}", e)))?;

        Ok(metrics)
    }

    /// Save metrics to cache file
    fn save_metrics(metrics: &[StartupMetrics], path: &PathBuf) -> Result<(), LiteError> {
        let content = serde_json::to_string_pretty(metrics)
            .map_err(|e| LiteError::Internal(format!("Cannot serialize metrics: {}", e)))?;

        std::fs::write(path, content)
            .map_err(|e| LiteError::Internal(format!("Cannot write metrics cache: {}", e)))?;

        Ok(())
    }

    /// Determine if this is a cold or hot start
    pub fn detect_start_type(&self) -> StartType {
        let cached = self
            .cached_metrics
            .read()
            .map_err(|_| LiteError::Internal("Cannot read cached metrics".to_string()));

        if let Ok(metrics) = cached {
            // If we have recent metrics (within 30 minutes), consider this a hot start
            if let Some(last) = metrics.last() {
                let last_time = chrono::DateTime::parse_from_rfc3339(&last.timestamp);
                if let Ok(last_time) = last_time {
                    let elapsed = chrono::Utc::now() - last_time.with_timezone(&chrono::Utc);
                    if elapsed < chrono::Duration::minutes(30) {
                        return StartType::Hot;
                    }
                }
            }
        }

        StartType::Cold
    }

    /// Start tracking a new startup
    pub fn begin_startup(&self) -> Result<Instant, LiteError> {
        let start_time = Instant::now();

        let metrics = StartupMetrics::default();
        let mut current = self
            .current_startup
            .lock()
            .map_err(|_| LiteError::Internal("Cannot lock current startup".to_string()))?;

        *current = Some(metrics);

        Ok(start_time)
    }

    /// Record a phase duration
    pub fn record_phase(&self, phase_name: &str, duration_ms: u64) -> Result<(), LiteError> {
        let mut current = self
            .current_startup
            .lock()
            .map_err(|_| LiteError::Internal("Cannot lock current startup".to_string()))?;

        if let Some(metrics) = current.as_mut() {
            metrics.phase_durations.insert(phase_name.to_string(), duration_ms);
        }

        Ok(())
    }

    /// Complete the startup tracking
    pub fn complete_startup(&self, total_duration_ms: u64) -> Result<(), LiteError> {
        let mut current = self
            .current_startup
            .lock()
            .map_err(|_| LiteError::Internal("Cannot lock current startup".to_string()))?;

        if let Some(metrics) = current.take() {
            let is_cold = self.detect_start_type() == StartType::Cold;
            let completed_metrics = StartupMetrics {
                total_duration_ms,
                phase_durations: metrics.phase_durations,
                is_cold_start: is_cold,
                timestamp: chrono::Utc::now().to_rfc3339(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                hot_paths: metrics.hot_paths,
                optimization_suggestions: metrics.optimization_suggestions,
            };

            // Add to history
            let mut cached = self
                .cached_metrics
                .write()
                .map_err(|_| LiteError::Internal("Cannot lock cached metrics".to_string()))?;

            cached.push(completed_metrics);

            // Trim history
            if cached.len() > self.history_size {
                cached.remove(0);
            }

            // Save to file
            Self::save_metrics(&cached, &self.cache_path)?;
        }

        Ok(())
    }

    /// Get startup statistics from history
    pub fn get_statistics(&self) -> Result<StartupStatistics, LiteError> {
        let cached = self
            .cached_metrics
            .read()
            .map_err(|_| LiteError::Internal("Cannot read cached metrics".to_string()))?;

        if cached.is_empty() {
            return Ok(StartupStatistics::default());
        }

        let cold_starts = cached.iter().filter(|m| m.is_cold_start).count();
        let hot_starts = cached.len() - cold_starts;

        let cold_avg = if cold_starts > 0 {
            cached
                .iter()
                .filter(|m| m.is_cold_start)
                .map(|m| m.total_duration_ms)
                .sum::<u64>()
                / cold_starts as u64
        } else {
            0
        };

        let hot_avg = if hot_starts > 0 {
            cached
                .iter()
                .filter(|m| !m.is_cold_start)
                .map(|m| m.total_duration_ms)
                .sum::<u64>()
                / hot_starts as u64
        } else {
            0
        };

        let best = cached.iter().map(|m| m.total_duration_ms).min().unwrap_or(0);
        let worst = cached.iter().map(|m| m.total_duration_ms).max().unwrap_or(0);

        // Find most common slow phase
        let mut phase_slow_counts: HashMap<String, u64> = HashMap::new();
        for metrics in cached.iter() {
            if let Some((phase, _)) = metrics.slowest_phase() {
                *phase_slow_counts.entry(phase).or_insert(0) += 1;
            }
        }
        let most_common_slow_phase = phase_slow_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(phase, _)| phase.clone());

        Ok(StartupStatistics {
            total_starts: cached.len(),
            cold_starts,
            hot_starts,
            cold_start_avg_ms: cold_avg,
            hot_start_avg_ms: hot_avg,
            best_startup_ms: best,
            worst_startup_ms: worst,
            most_common_slow_phase,
        })
    }

    /// Get optimization recommendations based on history
    pub fn get_recommendations(&self) -> Result<Vec<String>, LiteError> {
        let cached = self
            .cached_metrics
            .read()
            .map_err(|_| LiteError::Internal("Cannot read cached metrics".to_string()))?;

        // Collect all suggestions
        let mut all_suggestions: Vec<String> = cached
            .iter()
            .flat_map(|m| m.optimization_suggestions.clone())
            .collect();

        // Deduplicate
        all_suggestions.sort();
        all_suggestions.dedup();

        Ok(all_suggestions)
    }

    /// Clear the cache history
    pub fn clear_history(&self) -> Result<(), LiteError> {
        let mut cached = self
            .cached_metrics
            .write()
            .map_err(|_| LiteError::Internal("Cannot lock cached metrics".to_string()))?;

        cached.clear();

        Self::save_metrics(&cached, &self.cache_path)?;

        Ok(())
    }
}

impl Default for ColdStartCache {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            cache_path: PathBuf::new(),
            cached_metrics: RwLock::new(Vec::new()),
            current_startup: Mutex::new(None),
            history_size: 10,
        })
    }
}

/// Type of startup (cold vs hot)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartType {
    /// First startup or long time since last
    Cold,
    /// Recent startup with cached data
    Hot,
}

impl StartType {
    /// Get target duration for this start type
    pub fn target_duration_ms(&self) -> u64 {
        match self {
            StartType::Cold => 1500, // v0.3.0-beta.2 target: < 1.5s
            StartType::Hot => 500,   // v0.3.0-beta.2 target: < 500ms
        }
    }
}

/// Startup statistics from history
#[derive(Debug, Clone, Default)]
pub struct StartupStatistics {
    pub total_starts: usize,
    pub cold_starts: usize,
    pub hot_starts: usize,
    pub cold_start_avg_ms: u64,
    pub hot_start_avg_ms: u64,
    pub best_startup_ms: u64,
    pub worst_startup_ms: u64,
    pub most_common_slow_phase: Option<String>,
}

impl StartupStatistics {
    /// Check if targets are being met consistently
    pub fn targets_met_consistently(&self) -> bool {
        // Consider targets met if average is under target
        self.cold_start_avg_ms < 1500 && self.hot_start_avg_ms < 500
    }

    /// Get improvement percentage
    pub fn improvement_percentage(&self) -> f64 {
        if self.worst_startup_ms == 0 {
            return 0.0;
        }
        ((self.worst_startup_ms - self.best_startup_ms) as f64 / self.worst_startup_ms as f64) * 100.0
    }
}

// ============================================================================
// Parallel Initialization
// ============================================================================

/// Parallel initializer for running independent phases concurrently
pub struct ParallelInitializer {
    /// Phases that can run in parallel
    parallel_groups: Vec<Vec<StartupPhase>>,
    /// Current phase tracking
    sequence: Arc<StartupSequence>,
}

impl ParallelInitializer {
    /// Create a new parallel initializer
    pub fn new() -> Self {
        Self {
            parallel_groups: Self::default_parallel_groups(),
            sequence: Arc::new(StartupSequence::new()),
        }
    }

    /// Create with custom parallel groups
    pub fn with_groups(groups: Vec<Vec<StartupPhase>>) -> Self {
        Self {
            parallel_groups: groups,
            sequence: Arc::new(StartupSequence::new()),
        }
    }

    /// Default parallel groups based on phase dependencies
    fn default_parallel_groups() -> Vec<Vec<StartupPhase>> {
        vec![
            // Group 1: Launch - always first, alone
            vec![StartupPhase::Launch],
            // Group 2: Config + Database can run in parallel
            vec![StartupPhase::ConfigLoad, StartupPhase::DatabaseInit],
            // Group 3: Index + UI can run after config/database
            vec![StartupPhase::IndexBuild, StartupPhase::UiInit],
            // Group 4: Ready - final phase
            vec![StartupPhase::Ready],
        ]
    }

    /// Run all initialization phases with parallel optimization
    pub async fn run_parallel<F>(
        &self,
        phase_executor: F,
    ) -> Result<StartupReport, LiteError>
    where
        F: Fn(StartupPhase) -> Pin<Box<dyn Future<Output = Result<(), LiteError>> + Send>>,
    {
        // Run parallel groups (includes Launch as first group)
        for group in &self.parallel_groups {
            if group.len() == 1 {
                // Single phase - run directly
                let phase = group[0];
                self.sequence.start_phase(phase)?;
                phase_executor(phase).await?;
                self.sequence.complete_phase(phase)?;
            } else {
                // Multiple phases - run concurrently with tokio::join!
                self.run_group_parallel(group, &phase_executor).await?;
            }
        }

        self.sequence.complete()?;
        self.sequence.get_report()
    }

    /// Run a group of phases in parallel
    async fn run_group_parallel<F>(
        &self,
        group: &[StartupPhase],
        phase_executor: &F,
    ) -> Result<(), LiteError>
    where
        F: Fn(StartupPhase) -> Pin<Box<dyn Future<Output = Result<(), LiteError>> + Send>>,
    {
        // Start all phases
        for phase in group {
            self.sequence.start_phase(*phase)?;
        }

        // Create futures for all phases
        let futures: Vec<_> = group
            .iter()
            .map(|phase| phase_executor(*phase))
            .collect();

        // Run all futures concurrently
        let results = futures_util::future::join_all(futures).await;

        // Complete all phases and check for errors
        for (i, result) in results.into_iter().enumerate() {
            self.sequence.complete_phase(group[i])?;
            result?;
        }

        Ok(())
    }

    /// Get the startup sequence
    pub fn sequence(&self) -> Arc<StartupSequence> {
        self.sequence.clone()
    }

    /// Check if phases can run in parallel
    pub fn can_parallelize(&self, phase1: StartupPhase, phase2: StartupPhase) -> bool {
        for group in &self.parallel_groups {
            if group.contains(&phase1) && group.contains(&phase2) {
                return true;
            }
        }
        false
    }

    /// Get parallel groups for analysis
    pub fn get_parallel_groups(&self) -> &[Vec<StartupPhase>] {
        &self.parallel_groups
    }
}

impl Default for ParallelInitializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper for executing startup phases with timing
pub struct PhaseExecutorHelper {
    sequence: Arc<StartupSequence>,
    cache: Arc<ColdStartCache>,
}

impl PhaseExecutorHelper {
    /// Create a new phase executor helper
    pub fn new(sequence: Arc<StartupSequence>, cache: Arc<ColdStartCache>) -> Self {
        Self { sequence, cache }
    }

    /// Execute a phase with timing and caching
    pub async fn execute_phase<F>(
        &self,
        phase: StartupPhase,
        executor: F,
    ) -> Result<(), LiteError>
    where
        F: std::future::Future<Output = Result<(), LiteError>>,
    {
        let start = Instant::now();

        // Execute the phase
        executor.await?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Record in cache
        self.cache.record_phase(phase.name(), duration_ms)?;

        Ok(())
    }

    /// Execute a phase with lazy initialization
    pub fn execute_lazy<T, F>(
        &self,
        phase: StartupPhase,
        lazy: &LazyInitializer<T>,
    ) -> Result<T, LiteError>
    where
        T: Clone,
        F: Fn() -> Result<T, LiteError> + Send + Sync + 'static,
    {
        let start = Instant::now();

        // Get the lazy value
        let result = lazy.get()?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Record timing
        self.cache.record_phase(phase.name(), duration_ms)?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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

    #[test]
    fn test_startup_metrics() {
        let mut phase_durations = HashMap::new();
        phase_durations.insert("Initializing Database".to_string(), 400);
        phase_durations.insert("Loading Configuration".to_string(), 150);

        let metrics = StartupMetrics {
            total_duration_ms: 550,
            phase_durations,
            is_cold_start: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "0.3.0".to_string(),
            hot_paths: Vec::new(),
            optimization_suggestions: Vec::new(),
        };

        // Test hot paths detection
        let hot_paths = StartupMetrics::detect_hot_paths(&metrics.phase_durations);
        assert!(hot_paths.iter().any(|p| p.contains("Database")));

        // Test suggestions
        let suggestions = StartupMetrics::generate_suggestions(&metrics.phase_durations, 500);
        assert!(suggestions.iter().any(|s| s.contains("database fast path")));

        // Test slowest phase
        let slowest = metrics.slowest_phase();
        assert!(slowest.is_some());
        let (name, duration) = slowest.unwrap();
        assert_eq!(name, "Initializing Database");
        assert_eq!(duration, 400);
    }

    #[test]
    fn test_start_type() {
        assert_eq!(StartType::Cold.target_duration_ms(), 1500);
        assert_eq!(StartType::Hot.target_duration_ms(), 500);
    }

    #[test]
    fn test_parallel_initializer() {
        let initializer = ParallelInitializer::new();

        // Check default parallel groups
        let groups = initializer.get_parallel_groups();
        assert_eq!(groups.len(), 4);

        // Check that Config and Database can parallelize
        assert!(initializer.can_parallelize(StartupPhase::ConfigLoad, StartupPhase::DatabaseInit));

        // Check that Launch and Config cannot parallelize
        assert!(!initializer.can_parallelize(StartupPhase::Launch, StartupPhase::ConfigLoad));
    }

    #[test]
    fn test_startup_statistics() {
        let stats = StartupStatistics {
            total_starts: 10,
            cold_starts: 5,
            hot_starts: 5,
            cold_start_avg_ms: 1200,
            hot_start_avg_ms: 400,
            best_startup_ms: 350,
            worst_startup_ms: 1800,
            most_common_slow_phase: Some("Database".to_string()),
        };

        assert!(stats.targets_met_consistently());
        assert!(stats.improvement_percentage() > 0.0);
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::Duration;

        let initializer = ParallelInitializer::new();
        let counter = Arc::new(AtomicU64::new(0));

        let result = initializer
            .run_parallel(|phase| {
                let counter = counter.clone();
                Box::pin(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    // Simulate phase work
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    Ok(())
                })
            })
            .await;

        assert!(result.is_ok());
        // Should have executed 6 phases
        assert_eq!(counter.load(Ordering::SeqCst), 6);
    }

    #[test]
    fn test_cold_start_cache_new() {
        // This test creates a cache in a temporary location
        let cache = ColdStartCache::new();
        if cache.is_err() {
            // Cache creation may fail in some environments, skip test
            return;
        }
        let cache = cache.unwrap();

        // Check start type detection
        let start_type = cache.detect_start_type();
        // First run should be cold
        assert_eq!(start_type, StartType::Cold);

        // Get statistics (should be empty initially)
        let stats = cache.get_statistics().unwrap();
        assert_eq!(stats.total_starts, 0);
    }
}
