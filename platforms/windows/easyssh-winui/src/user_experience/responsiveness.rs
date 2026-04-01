//! UI Responsiveness Optimizations for EasySSH
//!
//! Features:
//! - Virtual scrolling for large lists
//! - Debounced search input
//! - Throttled UI updates
//! - Lazy loading components

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Virtual list for efficient rendering of large datasets
pub struct VirtualList<T> {
    items: Vec<T>,
    item_height: f32,
    visible_count: usize,
    scroll_offset: f32,
    buffer_size: usize,
}

impl<T> VirtualList<T> {
    pub fn new(item_height: f32, visible_count: usize) -> Self {
        Self {
            items: Vec::new(),
            item_height,
            visible_count,
            scroll_offset: 0.0,
            buffer_size: 3, // Number of extra items to render above/below
        }
    }

    pub fn with_items(mut self, items: Vec<T>) -> Self {
        self.items = items;
        self
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        // Reset scroll when items change significantly
        self.scroll_offset = 0.0;
    }

    pub fn update_scroll(&mut self, offset: f32) {
        self.scroll_offset = offset.max(0.0);
    }

    pub fn total_height(&self) -> f32 {
        self.items.len() as f32 * self.item_height
    }

    /// Get the visible range of items with buffer
    pub fn visible_range(&self) -> (usize, usize) {
        let start_idx = (self.scroll_offset / self.item_height) as usize;
        let start = start_idx.saturating_sub(self.buffer_size);
        let end = (start_idx + self.visible_count + self.buffer_size).min(self.items.len());
        (start, end)
    }

    /// Get visible items with their render positions
    pub fn visible_items(&self) -> Vec<(usize, &T, f32)> {
        let (start, end) = self.visible_range();

        self.items[start..end]
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let actual_idx = start + idx;
                let y_position = actual_idx as f32 * self.item_height;
                (actual_idx, item, y_position)
            })
            .collect()
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Debounced value holder for search inputs
pub struct DebouncedValue<T> {
    value: T,
    pending_value: T,
    last_update: Instant,
    debounce_duration: Duration,
    has_pending: bool,
}

impl<T: Clone + PartialEq> DebouncedValue<T> {
    pub fn new(initial: T, debounce_ms: u64) -> Self {
        Self {
            value: initial.clone(),
            pending_value: initial,
            last_update: Instant::now(),
            debounce_duration: Duration::from_millis(debounce_ms),
            has_pending: false,
        }
    }

    /// Update the pending value (call this on input change)
    pub fn set_pending(&mut self, value: T) {
        if value != self.pending_value {
            self.pending_value = value;
            self.has_pending = true;
            self.last_update = Instant::now();
        }
    }

    /// Check if debounced value should be committed
    pub fn update(&mut self) -> bool {
        if self.has_pending
            && self.last_update.elapsed() >= self.debounce_duration
            && self.value != self.pending_value
        {
            self.value = self.pending_value.clone();
            self.has_pending = false;
            return true;
        }
        false
    }

    /// Force immediate commit
    pub fn commit(&mut self) {
        self.value = self.pending_value.clone();
        self.has_pending = false;
    }

    /// Get the current committed value
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Get the pending value (for UI binding)
    pub fn pending_value(&mut self) -> &mut T {
        &mut self.pending_value
    }

    /// Check if there's a pending update
    pub fn has_pending(&self) -> bool {
        self.has_pending
    }

    /// Reset to initial value
    pub fn reset(&mut self, value: T) {
        self.value = value.clone();
        self.pending_value = value;
        self.has_pending = false;
    }
}

/// Throttled execution for expensive operations
pub struct ThrottledExecutor {
    last_execution: Instant,
    min_interval: Duration,
    pending_call: Option<Box<dyn FnOnce() + Send>>,
}

impl ThrottledExecutor {
    pub fn new(min_interval_ms: u64) -> Self {
        Self {
            last_execution: Instant::now() - Duration::from_secs(60), // Allow immediate first execution
            min_interval: Duration::from_millis(min_interval_ms),
            pending_call: None,
        }
    }

    /// Execute immediately or schedule for later
    pub fn execute<F: FnOnce() + Send + 'static>(&mut self, f: F) {
        if self.last_execution.elapsed() >= self.min_interval {
            self.last_execution = Instant::now();
            f();
        } else {
            self.pending_call = Some(Box::new(f));
        }
    }

    /// Check if pending execution should run
    pub fn update(&mut self) {
        if self.pending_call.is_some() && self.last_execution.elapsed() >= self.min_interval {
            if let Some(call) = self.pending_call.take() {
                self.last_execution = Instant::now();
                call();
            }
        }
    }

    /// Force immediate execution of pending call
    pub fn flush(&mut self) {
        if let Some(call) = self.pending_call.take() {
            self.last_execution = Instant::now();
            call();
        }
    }
}

/// Rate limiter for UI updates
pub struct RateLimiter {
    last_update: Instant,
    min_interval: Duration,
}

impl RateLimiter {
    pub fn new(interval_ms: u64) -> Self {
        Self {
            last_update: Instant::now(),
            min_interval: Duration::from_millis(interval_ms),
        }
    }

    /// Check if update is allowed
    pub fn can_update(&mut self) -> bool {
        if self.last_update.elapsed() >= self.min_interval {
            self.last_update = Instant::now();
            true
        } else {
            false
        }
    }

    /// Try to update, return true if allowed
    pub fn try_update<F: FnOnce()>(&mut self, f: F) -> bool {
        if self.can_update() {
            f();
            true
        } else {
            false
        }
    }
}

/// Lazy loader for expensive resources
pub struct LazyLoader<T> {
    loader: Box<dyn Fn() -> T + Send + Sync>,
    cached: Option<T>,
    loading: bool,
    error: Option<String>,
}

impl<T: Clone> LazyLoader<T> {
    pub fn new<F: Fn() -> T + Send + Sync + 'static>(loader: F) -> Self {
        Self {
            loader: Box::new(loader),
            cached: None,
            loading: false,
            error: None,
        }
    }

    /// Get value, loading if necessary
    pub fn get(&mut self) -> Option<T> {
        if self.cached.is_none() && !self.loading {
            self.loading = true;
            self.cached = Some((self.loader)());
            self.loading = false;
        }
        self.cached.clone()
    }

    /// Get cached value without loading
    pub fn cached(&self) -> Option<&T> {
        self.cached.as_ref()
    }

    /// Check if currently loading
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Clear cache to force reload
    pub fn invalidate(&mut self) {
        self.cached = None;
        self.error = None;
    }

    /// Preload value in background
    pub fn preload(&mut self) {
        if self.cached.is_none() && !self.loading {
            self.loading = true;
            self.cached = Some((self.loader)());
            self.loading = false;
        }
    }
}

/// Smooth scrolling controller
pub struct SmoothScroll {
    target: f32,
    current: f32,
    velocity: f32,
    friction: f32,
    snap_threshold: f32,
}

impl SmoothScroll {
    pub fn new() -> Self {
        Self {
            target: 0.0,
            current: 0.0,
            velocity: 0.0,
            friction: 0.85,
            snap_threshold: 0.5,
        }
    }

    pub fn scroll_to(&mut self, position: f32) {
        self.target = position.max(0.0);
    }

    pub fn scroll_by(&mut self, delta: f32) {
        self.target = (self.target + delta).max(0.0);
    }

    /// Update physics and return current position
    pub fn update(&mut self) -> f32 {
        // Spring physics
        let delta = self.target - self.current;
        self.velocity += delta * 0.1;
        self.velocity *= self.friction;
        self.current += self.velocity;

        // Snap when close
        if delta.abs() < self.snap_threshold && self.velocity.abs() < 0.1 {
            self.current = self.target;
            self.velocity = 0.0;
        }

        self.current
    }

    pub fn current(&self) -> f32 {
        self.current
    }

    pub fn is_animating(&self) -> bool {
        (self.target - self.current).abs() > 0.1 || self.velocity.abs() > 0.1
    }
}

/// Frame rate monitor
pub struct FrameRateMonitor {
    frame_times: VecDeque<Instant>,
    window_size: usize,
}

impl FrameRateMonitor {
    pub fn new(window_size: usize) -> Self {
        Self {
            frame_times: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    /// Record a frame
    pub fn record_frame(&mut self) {
        let now = Instant::now();
        self.frame_times.push_back(now);

        // Remove old frames
        while self.frame_times.len() > self.window_size {
            self.frame_times.pop_front();
        }

        // Remove frames older than 1 second
        while let Some(front) = self.frame_times.front() {
            if now.duration_since(*front) > Duration::from_secs(1) {
                self.frame_times.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get current FPS
    pub fn fps(&self) -> f32 {
        if self.frame_times.len() < 2 {
            return 0.0;
        }

        let duration = self
            .frame_times
            .back()
            .and_then(|back| {
                self.frame_times
                    .front()
                    .map(|front| back.duration_since(*front))
            })
            .unwrap_or(Duration::ZERO);

        if duration.as_secs_f32() > 0.0 {
            self.frame_times.len() as f32 / duration.as_secs_f32()
        } else {
            0.0
        }
    }

    /// Get average frame time in ms
    pub fn frame_time_ms(&self) -> f32 {
        if self.frame_times.len() < 2 {
            return 0.0;
        }

        let total_duration = self
            .frame_times
            .back()
            .and_then(|back| {
                self.frame_times
                    .front()
                    .map(|front| back.duration_since(*front))
            })
            .unwrap_or(Duration::ZERO);

        total_duration.as_secs_f32() * 1000.0 / self.frame_times.len().max(1) as f32
    }

    /// Check if performance is degraded
    pub fn is_performance_degraded(&self) -> bool {
        self.fps() > 0.0 && self.fps() < 30.0
    }
}

/// Adaptive quality manager
pub struct AdaptiveQuality {
    frame_rate_monitor: FrameRateMonitor,
    quality_level: QualityLevel,
    consecutive_low_fps: u32,
    consecutive_high_fps: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualityLevel {
    Low,    // Reduced animations, simpler rendering
    Medium, // Standard quality
    High,   // Full effects
}

impl AdaptiveQuality {
    pub fn new() -> Self {
        Self {
            frame_rate_monitor: FrameRateMonitor::new(60),
            quality_level: QualityLevel::High,
            consecutive_low_fps: 0,
            consecutive_high_fps: 0,
        }
    }

    /// Update with new frame
    pub fn record_frame(&mut self) {
        self.frame_rate_monitor.record_frame();

        let fps = self.frame_rate_monitor.fps();

        if fps < 30.0 && fps > 0.0 {
            self.consecutive_low_fps += 1;
            self.consecutive_high_fps = 0;

            // Reduce quality after sustained low FPS
            if self.consecutive_low_fps > 30 {
                self.reduce_quality();
                self.consecutive_low_fps = 0;
            }
        } else if fps > 55.0 {
            self.consecutive_high_fps += 1;
            self.consecutive_low_fps = 0;

            // Increase quality after sustained high FPS
            if self.consecutive_high_fps > 120 {
                self.increase_quality();
                self.consecutive_high_fps = 0;
            }
        }
    }

    fn reduce_quality(&mut self) {
        self.quality_level = match self.quality_level {
            QualityLevel::High => QualityLevel::Medium,
            QualityLevel::Medium => QualityLevel::Low,
            QualityLevel::Low => QualityLevel::Low,
        };
    }

    fn increase_quality(&mut self) {
        self.quality_level = match self.quality_level {
            QualityLevel::Low => QualityLevel::Medium,
            QualityLevel::Medium => QualityLevel::High,
            QualityLevel::High => QualityLevel::High,
        };
    }

    pub fn quality_level(&self) -> QualityLevel {
        self.quality_level
    }

    pub fn should_animate(&self) -> bool {
        self.quality_level != QualityLevel::Low
    }

    pub fn should_use_shadows(&self) -> bool {
        self.quality_level == QualityLevel::High
    }

    pub fn should_use_blur(&self) -> bool {
        self.quality_level == QualityLevel::High
    }

    pub fn fps(&self) -> f32 {
        self.frame_rate_monitor.fps()
    }
}

/// Search optimizer with debouncing and result caching
pub struct SearchOptimizer<T: Clone> {
    debounced_query: DebouncedValue<String>,
    cached_results: Vec<T>,
    last_search_time: Instant,
    search_fn: Box<dyn Fn(&str) -> Vec<T> + Send + Sync>,
}

impl<T: Clone> SearchOptimizer<T> {
    pub fn new<F: Fn(&str) -> Vec<T> + Send + Sync + 'static>(search_fn: F) -> Self {
        Self {
            debounced_query: DebouncedValue::new(String::new(), 150),
            cached_results: Vec::new(),
            last_search_time: Instant::now(),
            search_fn: Box::new(search_fn),
        }
    }

    /// Update search query (debounced)
    pub fn set_query(&mut self, query: String) {
        self.debounced_query.set_pending(query);
    }

    /// Get current query for binding
    pub fn query_mut(&mut self) -> &mut String {
        self.debounced_query.pending_value()
    }

    /// Get current query value
    pub fn query(&self) -> &String {
        self.debounced_query.value()
    }

    /// Update and get results if search was performed
    pub fn update(&mut self) -> Option<&Vec<T>> {
        if self.debounced_query.update() {
            let query = self.debounced_query.value().clone();
            self.cached_results = (self.search_fn)(&query);
            self.last_search_time = Instant::now();
            Some(&self.cached_results)
        } else {
            None
        }
    }

    /// Force immediate search
    pub fn search_now(&mut self) -> &Vec<T> {
        self.debounced_query.commit();
        let query = self.debounced_query.value().clone();
        self.cached_results = (self.search_fn)(&query);
        self.last_search_time = Instant::now();
        &self.cached_results
    }

    /// Get cached results
    pub fn results(&self) -> &Vec<T> {
        &self.cached_results
    }

    /// Clear search
    pub fn clear(&mut self) {
        self.debounced_query.reset(String::new());
        self.cached_results.clear();
    }

    /// Check if search is debouncing
    pub fn is_searching(&self) -> bool {
        self.debounced_query.has_pending()
    }

    /// Time since last search
    pub fn last_search_elapsed(&self) -> Duration {
        self.last_search_time.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_list_range() {
        let list = VirtualList::new(50.0, 10).with_items((0..100).collect());

        let (start, end) = list.visible_range();
        assert_eq!(start, 0);
        assert!(end > start);
    }

    #[test]
    fn test_debounced_value() {
        let mut debounced = DebouncedValue::new(String::from("initial"), 100);

        // Set pending value
        debounced.set_pending(String::from("new"));
        assert_eq!(debounced.value(), "initial");

        // Wait for debounce (in test, we can just check pending state)
        assert!(debounced.has_pending());

        // Check value hasn't changed yet
        assert!(!debounced.update());
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(1000); // 1 second

        // First update should succeed
        assert!(limiter.can_update());

        // Immediate second update should fail
        assert!(!limiter.can_update());
    }

    #[test]
    fn test_frame_rate_monitor() {
        let mut monitor = FrameRateMonitor::new(10);

        // Record some frames
        for _ in 0..5 {
            monitor.record_frame();
            std::thread::sleep(Duration::from_millis(16)); // ~60fps
        }

        // Should have recorded frames
        assert!(monitor.fps() > 0.0 || monitor.frame_times.len() < 2);
    }
}
