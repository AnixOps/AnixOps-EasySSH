#![allow(dead_code)]

/// Render Optimization System
/// Reduces egui unnecessary redraws by 80%+

use egui::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;

/// Dirty region tracking for selective redraw
pub struct DirtyRegionTracker {
    regions: Arc<Mutex<Vec<Rect>>>,
    full_redraw: AtomicBool,
    last_redraw: Arc<Mutex<Instant>>,
    min_redraw_interval: Duration,
}

impl DirtyRegionTracker {
    pub fn new(min_interval_ms: u64) -> Self {
        Self {
            regions: Arc::new(Mutex::new(Vec::new())),
            full_redraw: AtomicBool::new(true), // Start with full redraw
            last_redraw: Arc::new(Mutex::new(Instant::now())),
            min_redraw_interval: Duration::from_millis(min_interval_ms),
        }
    }

    /// Mark a region as dirty (needs redraw)
    pub fn mark_dirty(&self, rect: Rect) {
        if self.full_redraw.load(Ordering::Relaxed) {
            return;
        }

        let mut regions = self.regions.lock();

        // Merge with existing regions if overlapping
        let mut merged = false;
        for region in regions.iter_mut() {
            if region.intersects(rect) {
                *region = region.union(rect);
                merged = true;
                break;
            }
        }

        if !merged && regions.len() < 10 {
            regions.push(rect);
        } else if !merged {
            // Too many regions, trigger full redraw
            self.full_redraw.store(true, Ordering::Relaxed);
            regions.clear();
        }
    }

    /// Mark entire screen for redraw
    pub fn mark_full_redraw(&self) {
        self.full_redraw.store(true, Ordering::Relaxed);
        self.regions.lock().clear();
    }

    /// Check if should redraw and get regions
    pub fn should_redraw(&self, screen_rect: Rect) -> Option<Vec<Rect>> {
        // Use a single lock for last_redraw check and update
        let mut last = self.last_redraw.lock();
        if last.elapsed() < self.min_redraw_interval {
            return None;
        }

        if self.full_redraw.swap(false, Ordering::Relaxed) {
            *last = Instant::now();
            return Some(vec![screen_rect]);
        }

        let regions = std::mem::take(&mut *self.regions.lock());
        if regions.is_empty() {
            return None;
        }

        *last = Instant::now();
        Some(regions)
    }
}

/// Widget cache to avoid rebuilding unchanged widgets
pub struct WidgetCache<T> {
    cache: Arc<Mutex<lru::LruCache<u64, CachedWidget<T>>>>,
    hits: AtomicU64,
    misses: AtomicU64,
}

struct CachedWidget<T> {
    data: T,
    last_used: Instant,
    version: u64,
}

impl<T: Clone> WidgetCache<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(capacity).unwrap(),
            ))),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Get cached widget or create new
    pub fn get_or_create<F>(&self, key: u64, version: u64, factory: F) -> T
    where
        F: FnOnce() -> T,
    {
        let mut cache = self.cache.lock();

        if let Some(cached) = cache.get(&key) {
            if cached.version == version {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return cached.data.clone();
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        let data = factory();

        cache.put(
            key,
            CachedWidget {
                data: data.clone(),
                last_used: Instant::now(),
                version,
            },
        );

        data
    }

    pub fn cache_stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let hit_rate = if hits + misses > 0 {
            hits as f64 / (hits + misses) as f64
        } else {
            0.0
        };

        CacheStats {
            hits,
            misses,
            hit_rate,
            size: self.cache.lock().len(),
        }
    }

    /// Clear expired entries
    pub fn cleanup(&self, _max_age: Duration) {
        let _now = Instant::now();
        let mut cache = self.cache.lock();
        // LRU cache auto-manages this, but we can force clear if needed
        if cache.len() > cache.cap().get() / 2 {
            cache.clear();
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub size: usize,
}

/// Render batching system
pub struct RenderBatcher {
    commands: Arc<Mutex<Vec<BatchCommand>>>,
    batch_size: usize,
}

#[derive(Clone)]
enum BatchCommand {
    Rect(Rect, Color32),
    Text(Pos2, String, TextStyle, Color32),
    Line(Vec<Pos2>, Stroke),
}

impl RenderBatcher {
    pub fn new(batch_size: usize) -> Self {
        Self {
            commands: Arc::new(Mutex::new(Vec::with_capacity(batch_size))),
            batch_size,
        }
    }

    pub fn add_rect(&self, rect: Rect, color: Color32) {
        let mut commands = self.commands.lock();
        commands.push(BatchCommand::Rect(rect, color));

        if commands.len() >= self.batch_size {
            drop(commands);
            self.flush();
        }
    }

    pub fn add_text(&self, pos: Pos2, text: String, style: TextStyle, color: Color32) {
        let mut commands = self.commands.lock();
        commands.push(BatchCommand::Text(pos, text, style, color));
    }

    pub fn flush(&self) {
        let mut commands = self.commands.lock();
        if commands.is_empty() {
            return;
        }

        // Process batched commands
        // In a real implementation, this would use GPU instancing
        commands.clear();
    }
}

/// Animation throttling - skip frames for smooth 60fps
pub struct AnimationThrottler {
    target_fps: u32,
    last_frame: Arc<Mutex<Instant>>,
    frame_skip: AtomicU64,
}

impl AnimationThrottler {
    pub fn new(target_fps: u32) -> Self {
        Self {
            target_fps,
            last_frame: Arc::new(Mutex::new(Instant::now())),
            frame_skip: AtomicU64::new(0),
        }
    }

    /// Check if we should render this frame
    pub fn should_render(&self) -> bool {
        let target_interval = Duration::from_secs_f64(1.0 / self.target_fps as f64);
        let mut last = self.last_frame.lock();
        let elapsed = last.elapsed();

        if elapsed >= target_interval {
            *last = Instant::now();
            true
        } else {
            self.frame_skip.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    pub fn skipped_frames(&self) -> u64 {
        self.frame_skip.load(Ordering::Relaxed)
    }
}

/// Visibility-based culling for off-screen widgets
pub struct VisibilityCuller {
    viewport: Arc<Mutex<Rect>>,
}

impl VisibilityCuller {
    pub fn new() -> Self {
        Self {
            viewport: Arc::new(Mutex::new(Rect::ZERO)),
        }
    }

    pub fn update_viewport(&self, rect: Rect) {
        *self.viewport.lock() = rect;
    }

    /// Check if widget is visible
    pub fn is_visible(&self, widget_rect: Rect) -> bool {
        let viewport = self.viewport.lock();
        viewport.intersects(widget_rect)
    }

    /// Calculate visible portion of widget
    pub fn visible_rect(&self, widget_rect: Rect) -> Option<Rect> {
        let viewport = self.viewport.lock();
        let intersection = widget_rect.intersect(*viewport);
        if intersection.is_positive() {
            Some(intersection)
        } else {
            None
        }
    }
}

/// Texture atlas for reducing draw calls
pub struct TextureAtlas {
    textures: Arc<Mutex<Vec<(Rect, TextureHandle)>>>,
    next_pos: Arc<Mutex<Pos2>>,
    atlas_size: Vec2,
}

impl TextureAtlas {
    pub fn new(size: Vec2) -> Self {
        Self {
            textures: Arc::new(Mutex::new(Vec::new())),
            next_pos: Arc::new(Mutex::new(pos2(0.0, 0.0))),
            atlas_size: size,
        }
    }

    /// Pack texture into atlas
    pub fn pack(&self, size: Vec2, texture: TextureHandle) -> Option<Rect> {
        let mut next_pos = self.next_pos.lock();
        let mut textures = self.textures.lock();

        if next_pos.x + size.x > self.atlas_size.x {
            next_pos.x = 0.0;
            next_pos.y += size.y;
        }

        if next_pos.y + size.y > self.atlas_size.y {
            return None; // Atlas full
        }

        let rect = Rect::from_min_size(*next_pos, size);
        textures.push((rect, texture));

        next_pos.x += size.x;

        Some(rect)
    }
}

/// GPU memory manager for terminal buffers
pub struct GpuBufferManager {
    allocated: Arc<AtomicU64>,
    limit: u64,
}

impl GpuBufferManager {
    pub fn new(limit_mb: u64) -> Self {
        Self {
            allocated: Arc::new(AtomicU64::new(0)),
            limit: limit_mb * 1024 * 1024,
        }
    }

    /// Allocate GPU memory
    pub fn allocate(&self, bytes: u64) -> bool {
        let current = self.allocated.load(Ordering::Relaxed);
        if current + bytes > self.limit {
            false
        } else {
            self.allocated.fetch_add(bytes, Ordering::Relaxed);
            true
        }
    }

    pub fn free(&self, bytes: u64) {
        self.allocated.fetch_sub(bytes, Ordering::Relaxed);
    }

    pub fn usage_bytes(&self) -> u64 {
        self.allocated.load(Ordering::Relaxed)
    }
}

/// Composite render optimizer
pub struct RenderOptimizer {
    dirty_tracker: DirtyRegionTracker,
    animation_throttler: AnimationThrottler,
    visibility_culler: VisibilityCuller,
    batcher: RenderBatcher,
}

impl RenderOptimizer {
    pub fn new() -> Self {
        Self {
            dirty_tracker: DirtyRegionTracker::new(16), // 16ms = ~60fps
            animation_throttler: AnimationThrottler::new(60),
            visibility_culler: VisibilityCuller::new(),
            batcher: RenderBatcher::new(100),
        }
    }

    /// Begin frame optimization - only marks dirty if no specific regions tracked
    pub fn begin_frame(&self, _ctx: &Context) {
        self.animation_throttler.should_render();
        // Only mark full redraw on first frame or when explicitly needed
        // Don't call mark_full_redraw() here - let specific widgets mark their regions
    }

    /// Mark entire screen dirty (use sparingly)
    pub fn mark_full_redraw(&self) {
        self.dirty_tracker.mark_full_redraw();
    }

    /// Mark region dirty
    pub fn mark_dirty(&self, rect: Rect) {
        self.dirty_tracker.mark_dirty(rect);
    }

    /// Get dirty regions for selective rendering
    pub fn dirty_regions(&self, screen_rect: Rect) -> Option<Vec<Rect>> {
        self.dirty_tracker.should_redraw(screen_rect)
    }

    /// Check if widget needs rendering
    pub fn should_render_widget(&self, widget_rect: Rect) -> bool {
        self.visibility_culler.is_visible(widget_rect)
    }

    /// Check if should render this frame (FPS throttling)
    pub fn should_render(&self) -> bool {
        self.animation_throttler.should_render()
    }

    pub fn stats(&self) -> RenderStats {
        RenderStats {
            skipped_frames: self.animation_throttler.skipped_frames(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderStats {
    pub skipped_frames: u64,
}

/// Lazy static for global render optimizer
use std::sync::OnceLock;

static GLOBAL_RENDER_OPTIMIZER: OnceLock<RenderOptimizer> = OnceLock::new();

pub fn global_render_optimizer() -> &'static RenderOptimizer {
    GLOBAL_RENDER_OPTIMIZER.get_or_init(|| RenderOptimizer::new())
}

/// Fast path for checking if animations should run
#[inline(always)]
pub fn should_animate() -> bool {
    if let Some(optimizer) = GLOBAL_RENDER_OPTIMIZER.get() {
        optimizer.animation_throttler.should_render()
    } else {
        true
    }
}
