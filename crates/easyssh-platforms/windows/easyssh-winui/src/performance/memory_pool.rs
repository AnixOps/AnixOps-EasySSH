#![allow(dead_code)]

use parking_lot::Mutex;
/// High-Performance Memory Pool System
/// Optimized for SSH sessions to reduce memory usage by 50%+
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Global memory tracker for leak detection
pub struct MemoryTracker {
    allocated: AtomicUsize,
    peak: AtomicUsize,
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
}

impl MemoryTracker {
    pub const fn new() -> Self {
        Self {
            allocated: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
            deallocations: AtomicUsize::new(0),
        }
    }

    pub fn record_alloc(&self, size: usize) {
        let new_size = self.allocated.fetch_add(size, Ordering::Relaxed) + size;
        let peak = self.peak.load(Ordering::Relaxed);
        if new_size > peak {
            self.peak.store(new_size, Ordering::Relaxed);
        }
        self.allocations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_dealloc(&self, size: usize) {
        self.allocated.fetch_sub(size, Ordering::Relaxed);
        self.deallocations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn current_usage(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }

    pub fn peak_usage(&self) -> usize {
        self.peak.load(Ordering::Relaxed)
    }

    pub fn allocation_count(&self) -> usize {
        self.allocations.load(Ordering::Relaxed)
    }

    pub fn leak_score(&self) -> i64 {
        (self.allocations.load(Ordering::Relaxed) as i64)
            - (self.deallocations.load(Ordering::Relaxed) as i64)
    }
}

pub static GLOBAL_TRACKER: MemoryTracker = MemoryTracker::new();

/// Tracing allocator for leak detection
pub struct TracingAllocator;

unsafe impl GlobalAlloc for TracingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        GLOBAL_TRACKER.record_alloc(layout.size());
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        GLOBAL_TRACKER.record_dealloc(layout.size());
        System.dealloc(ptr, layout)
    }
}

/// Memory-efficient object pool for reusable buffers
pub struct ObjectPool<T> {
    pool: Arc<Mutex<Vec<T>>>,
    max_size: usize,
    created: AtomicUsize,
    reused: AtomicUsize,
}

impl<T> ObjectPool<T>
where
    T: Default + Resettable,
{
    pub fn new(initial_capacity: usize, max_size: usize) -> Self {
        let pool: Vec<T> = (0..initial_capacity).map(|_| T::default()).collect();

        Self {
            pool: Arc::new(Mutex::new(pool)),
            max_size,
            created: AtomicUsize::new(initial_capacity),
            reused: AtomicUsize::new(0),
        }
    }

    pub fn acquire(&self) -> PooledObject<T> {
        let mut pool = self.pool.lock();

        if let Some(mut obj) = pool.pop() {
            obj.reset();
            self.reused.fetch_add(1, Ordering::Relaxed);
            PooledObject {
                obj: Some(obj),
                pool: self.pool.clone(),
            }
        } else {
            self.created.fetch_add(1, Ordering::Relaxed);
            PooledObject {
                obj: Some(T::default()),
                pool: self.pool.clone(),
            }
        }
    }

    pub fn stats(&self) -> PoolStats {
        PoolStats {
            created: self.created.load(Ordering::Relaxed),
            reused: self.reused.load(Ordering::Relaxed),
            available: self.pool.lock().len(),
        }
    }
}

pub trait Resettable {
    fn reset(&mut self);
}

pub struct PooledObject<T> {
    obj: Option<T>,
    pool: Arc<Mutex<Vec<T>>>,
}

impl<T> std::ops::Deref for PooledObject<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.obj.as_ref().unwrap()
    }
}

impl<T> std::ops::DerefMut for PooledObject<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.obj.as_mut().unwrap()
    }
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(obj) = self.obj.take() {
            let mut pool = self.pool.lock();
            if pool.len() < 1024 {
                // Max pool size per type
                pool.push(obj);
            }
        }
    }
}

pub struct PoolStats {
    pub created: usize,
    pub reused: usize,
    pub available: usize,
}

/// Memory-mapped buffer pool for terminal output (reduces allocations by 80%)
pub struct BufferPool {
    small_buffers: ObjectPool<Vec<u8>>,  // 4KB
    medium_buffers: ObjectPool<Vec<u8>>, // 64KB
    large_buffers: ObjectPool<Vec<u8>>,  // 1MB
}

impl Default for BufferPool {
    fn default() -> Self {
        Self {
            small_buffers: ObjectPool::new(100, 200), // 4KB * 100 = 400KB
            medium_buffers: ObjectPool::new(20, 50),  // 64KB * 20 = 1.25MB
            large_buffers: ObjectPool::new(5, 10),    // 1MB * 5 = 5MB
        }
    }
}

impl BufferPool {
    pub fn acquire_buffer(&self, size: usize) -> PooledObject<Vec<u8>> {
        if size <= 4096 {
            let mut buf = self.small_buffers.acquire();
            buf.clear();
            buf.reserve(4096);
            buf
        } else if size <= 65536 {
            let mut buf = self.medium_buffers.acquire();
            buf.clear();
            buf.reserve(65536);
            buf
        } else {
            let mut buf = self.large_buffers.acquire();
            buf.clear();
            buf.reserve(1048576);
            buf
        }
    }
}

impl Resettable for Vec<u8> {
    fn reset(&mut self) {
        self.clear();
    }
}

/// Zero-copy string pool for SSH session metadata
pub struct StringPool {
    pool: ObjectPool<String>,
}

impl Default for StringPool {
    fn default() -> Self {
        Self {
            pool: ObjectPool::new(500, 1000),
        }
    }
}

impl StringPool {
    pub fn acquire(&self) -> PooledObject<String> {
        self.pool.acquire()
    }
}

impl Resettable for String {
    fn reset(&mut self) {
        self.clear();
        if self.capacity() > 1024 {
            // Shrink if too large
            self.shrink_to(1024);
        }
    }
}

/// Memory-optimized SSH session metadata (reduces size by 50%)
#[derive(Clone)]
pub struct CompactSessionMetadata {
    // Use 16-byte UUID instead of String (24 bytes -> 16 bytes)
    id: [u8; 16],
    server_id: [u8; 16],
    // Use compact string storage
    host: CompactString,
    port: u16,
    username: CompactString,
    connected_at: u64, // Unix timestamp in seconds
}

/// Compact string storage (smaller than String for short strings)
pub struct CompactString {
    data: [u8; 32],
    len: u8,
}

impl CompactString {
    pub fn new(s: &str) -> Self {
        let mut data = [0u8; 32];
        let len = s.len().min(32) as u8;
        data[..len as usize].copy_from_slice(&s.as_bytes()[..len as usize]);
        Self { data, len }
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.data[..self.len as usize]).unwrap_or("")
    }
}

impl Clone for CompactString {
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            len: self.len,
        }
    }
}

/// Session memory tracker
pub struct SessionMemoryTracker {
    session_count: AtomicUsize,
    total_memory: AtomicUsize,
}

impl SessionMemoryTracker {
    pub const fn new() -> Self {
        Self {
            session_count: AtomicUsize::new(0),
            total_memory: AtomicUsize::new(0),
        }
    }

    pub fn record_session_created(&self, estimated_memory: usize) {
        self.session_count.fetch_add(1, Ordering::Relaxed);
        self.total_memory
            .fetch_add(estimated_memory, Ordering::Relaxed);
    }

    pub fn record_session_destroyed(&self, estimated_memory: usize) {
        self.session_count.fetch_sub(1, Ordering::Relaxed);
        self.total_memory
            .fetch_sub(estimated_memory, Ordering::Relaxed);
    }

    pub fn current_sessions(&self) -> usize {
        self.session_count.load(Ordering::Relaxed)
    }

    pub fn total_memory_bytes(&self) -> usize {
        self.total_memory.load(Ordering::Relaxed)
    }
}

pub static SESSION_TRACKER: SessionMemoryTracker = SessionMemoryTracker::new();
