#![allow(dead_code)]

use parking_lot::Mutex;
/// Performance Monitoring and Profiling System
/// Real-time FPS, memory, and operation latency tracking
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Performance metrics snapshot
#[derive(Clone, Debug)]
pub struct PerformanceSnapshot {
    pub timestamp: Instant,
    pub fps: f64,
    pub frame_time_ms: f64,
    pub memory_usage_mb: f64,
    pub memory_peak_mb: f64,
    pub cpu_usage_percent: f64,
    pub draw_calls: u64,
    pub vertices: u64,
}

impl Default for PerformanceSnapshot {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            fps: 0.0,
            frame_time_ms: 0.0,
            memory_usage_mb: 0.0,
            memory_peak_mb: 0.0,
            cpu_usage_percent: 0.0,
            draw_calls: 0,
            vertices: 0,
        }
    }
}

/// Ring buffer for performance history
pub struct PerformanceHistory {
    buffer: VecDeque<PerformanceSnapshot>,
    capacity: usize,
}

impl PerformanceHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, snapshot: PerformanceSnapshot) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(snapshot);
    }

    pub fn iter(&self) -> impl Iterator<Item = &PerformanceSnapshot> {
        self.buffer.iter()
    }

    pub fn avg_fps(&self) -> f64 {
        if self.buffer.is_empty() {
            return 0.0;
        }
        self.buffer.iter().map(|s| s.fps).sum::<f64>() / self.buffer.len() as f64
    }

    pub fn min_fps(&self) -> f64 {
        self.buffer
            .iter()
            .map(|s| s.fps)
            .fold(f64::INFINITY, f64::min)
    }

    pub fn max_frame_time(&self) -> f64 {
        self.buffer
            .iter()
            .map(|s| s.frame_time_ms)
            .fold(0.0, f64::max)
    }
}

/// Real-time performance monitor
pub struct PerformanceMonitor {
    history: Arc<Mutex<PerformanceHistory>>,

    // Current frame tracking
    last_frame_time: Arc<Mutex<Instant>>,
    frame_count: Arc<AtomicU64>,
    last_fps_update: Arc<Mutex<Instant>>,
    current_fps: Arc<AtomicU64>, // Stored as FPS * 1000 (fixed-point)

    // Memory tracking
    memory_samples: Arc<Mutex<VecDeque<f64>>>,

    // Operation latency tracking
    operation_latencies: Arc<Mutex<HashMap<String, VecDeque<f64>>>>,

    // Render stats
    draw_calls: Arc<AtomicU64>,
    vertices: Arc<AtomicU64>,

    // Minimum latency threshold to record (avoids recording trivial operations)
    min_latency_ms: f64,
}

use std::collections::HashMap;

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self::with_threshold(0.01) // 10 microseconds minimum
    }

    /// Create with custom latency threshold (operations faster than this are not recorded)
    pub fn with_threshold(min_latency_ms: f64) -> Self {
        Self {
            history: Arc::new(Mutex::new(PerformanceHistory::new(300))), // 5 seconds at 60fps
            last_frame_time: Arc::new(Mutex::new(Instant::now())),
            frame_count: Arc::new(AtomicU64::new(0)),
            last_fps_update: Arc::new(Mutex::new(Instant::now())),
            current_fps: Arc::new(AtomicU64::new(0)),
            memory_samples: Arc::new(Mutex::new(VecDeque::with_capacity(60))),
            operation_latencies: Arc::new(Mutex::new(HashMap::new())),
            draw_calls: Arc::new(AtomicU64::new(0)),
            vertices: Arc::new(AtomicU64::new(0)),
            min_latency_ms: min_latency_ms.max(0.001), // Minimum 1 microsecond
        }
    }

    /// Call at start of each frame
    pub fn begin_frame(&self) {
        let now = Instant::now();
        let mut last = self.last_frame_time.lock();
        let frame_time = now.duration_since(*last).as_secs_f64() * 1000.0;
        *last = now;
        drop(last); // Explicitly drop to avoid holding lock during FPS update

        // Update FPS counter
        let count = self.frame_count.fetch_add(1, Ordering::Relaxed) + 1;
        let mut last_update = self.last_fps_update.lock();

        if now.duration_since(*last_update) >= Duration::from_secs(1) {
            let fps = count as f64 / now.duration_since(*last_update).as_secs_f64();
            self.current_fps
                .store((fps * 1000.0) as u64, Ordering::Relaxed);
            self.frame_count.store(0, Ordering::Relaxed);
            *last_update = now;
        }
        drop(last_update);

        // Record snapshot every 10 frames
        if count % 10 == 0 {
            self.record_snapshot(frame_time);
        }
    }

    fn record_snapshot(&self, frame_time_ms: f64) {
        let memory_mb = self.get_memory_usage_mb();

        let snapshot = PerformanceSnapshot {
            timestamp: Instant::now(),
            fps: self.current_fps.load(Ordering::Relaxed) as f64 / 1000.0,
            frame_time_ms,
            memory_usage_mb: memory_mb,
            memory_peak_mb: memory_mb, // Simplified
            cpu_usage_percent: 0.0,    // Would need platform-specific code
            draw_calls: self.draw_calls.load(Ordering::Relaxed),
            vertices: self.vertices.load(Ordering::Relaxed),
        };

        self.history.lock().push(snapshot);
    }

    fn get_memory_usage_mb(&self) -> f64 {
        // Simplified - would use platform-specific APIs
        use crate::performance::memory_pool::GLOBAL_TRACKER;
        GLOBAL_TRACKER.current_usage() as f64 / (1024.0 * 1024.0)
    }

    /// Record operation latency (only if above threshold)
    pub fn record_latency(&self, operation: &str, duration_ms: f64) {
        if duration_ms < self.min_latency_ms {
            return; // Skip recording trivial operations
        }

        let mut latencies = self.operation_latencies.lock();
        let queue = latencies
            .entry(operation.to_string())
            .or_insert_with(|| VecDeque::with_capacity(100));

        if queue.len() >= 100 {
            queue.pop_front();
        }
        queue.push_back(duration_ms);
    }

    /// Time an operation
    pub fn time_operation<F, R>(&self, name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        self.record_latency(name, duration);
        result
    }

    /// Get current FPS
    pub fn current_fps(&self) -> f64 {
        let fixed_point = self.current_fps.load(Ordering::Relaxed);
        fixed_point as f64 / 1000.0
    }

    /// Get average frame time
    pub fn avg_frame_time(&self) -> f64 {
        let history = self.history.lock();
        if history.buffer.is_empty() {
            return 0.0;
        }
        history.buffer.iter().map(|s| s.frame_time_ms).sum::<f64>() / history.buffer.len() as f64
    }

    /// Get performance report
    pub fn get_report(&self) -> PerformanceReport {
        let history = self.history.lock();
        let latencies = self.operation_latencies.lock();
        let memory_samples = self.memory_samples.lock();

        // Calculate peak memory from samples
        let memory_peak_mb = memory_samples.iter().fold(0.0f64, |a, b| a.max(*b));

        PerformanceReport {
            current_fps: self.current_fps(),
            avg_fps: history.avg_fps(),
            min_fps: history.min_fps(),
            max_frame_time_ms: history.max_frame_time(),
            memory_usage_mb: self.get_memory_usage_mb(),
            memory_peak_mb,
            operation_latencies: latencies
                .iter()
                .map(|(k, v)| {
                    let avg = if v.is_empty() {
                        0.0
                    } else {
                        v.iter().sum::<f64>() / v.len() as f64
                    };
                    let max = if v.is_empty() {
                        0.0
                    } else {
                        v.iter().fold(0.0f64, |a, b| a.max(*b))
                    };
                    (
                        k.clone(),
                        LatencyStats {
                            avg_ms: avg,
                            max_ms: max,
                            samples: v.len(),
                        },
                    )
                })
                .collect(),
        }
    }

    /// Reset all counters
    pub fn reset(&self) {
        self.frame_count.store(0, Ordering::Relaxed);
        self.draw_calls.store(0, Ordering::Relaxed);
        self.vertices.store(0, Ordering::Relaxed);
        self.history.lock().buffer.clear();
    }

    /// Increment draw call counter
    pub fn record_draw_call(&self, vertex_count: u64) {
        self.draw_calls.fetch_add(1, Ordering::Relaxed);
        self.vertices.fetch_add(vertex_count, Ordering::Relaxed);
    }
}

#[derive(Clone, Debug)]
pub struct PerformanceReport {
    pub current_fps: f64,
    pub avg_fps: f64,
    pub min_fps: f64,
    pub max_frame_time_ms: f64,
    pub memory_usage_mb: f64,
    pub memory_peak_mb: f64,
    pub operation_latencies: HashMap<String, LatencyStats>,
}

#[derive(Clone, Debug)]
pub struct LatencyStats {
    pub avg_ms: f64,
    pub max_ms: f64,
    pub samples: usize,
}

/// Scoped performance timer with lazy recording
pub struct ScopedTimer {
    name: String,
    start: Instant,
    monitor: Option<Arc<PerformanceMonitor>>,
    threshold_ms: f64,
}

impl ScopedTimer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start: Instant::now(),
            monitor: None,
            threshold_ms: 0.01, // 10 microseconds default
        }
    }

    pub fn with_monitor(name: &str, monitor: Arc<PerformanceMonitor>) -> Self {
        // Get threshold from monitor to avoid recording trivial operations
        let threshold_ms = 0.01;
        Self {
            name: name.to_string(),
            start: Instant::now(),
            monitor: Some(monitor),
            threshold_ms,
        }
    }

    /// Create with custom threshold
    pub fn with_threshold(name: &str, threshold_ms: f64) -> Self {
        Self {
            name: name.to_string(),
            start: Instant::now(),
            monitor: None,
            threshold_ms,
        }
    }
}

impl Drop for ScopedTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64() * 1000.0;
        if duration >= self.threshold_ms {
            if let Some(ref monitor) = self.monitor {
                monitor.record_latency(&self.name, duration);
            }
        }
    }
}

/// Frame rate limiter for power saving
pub struct FrameLimiter {
    target_frame_time: Duration,
    last_frame: Instant,
}

impl FrameLimiter {
    pub fn new(target_fps: u32) -> Self {
        Self {
            target_frame_time: Duration::from_secs_f64(1.0 / target_fps as f64),
            last_frame: Instant::now(),
        }
    }

    pub fn wait(&mut self) {
        let elapsed = self.last_frame.elapsed();
        if elapsed < self.target_frame_time {
            std::thread::sleep(self.target_frame_time - elapsed);
        }
        self.last_frame = Instant::now();
    }
}

/// Lazy initialization helper
use std::sync::OnceLock;

static GLOBAL_MONITOR: OnceLock<Arc<PerformanceMonitor>> = OnceLock::new();

pub fn global_monitor() -> Arc<PerformanceMonitor> {
    GLOBAL_MONITOR
        .get_or_init(|| Arc::new(PerformanceMonitor::new()))
        .clone()
}
