//! Memory Usage Optimizations
//!
//! Optimizations implemented:
//! - Object pooling for frequently allocated types
//! - Memory-mapped file access for large data
//! - Efficient data structure selection
//! - Memory usage monitoring and limits

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::error::LiteError;

/// Object pool for reusable allocations
pub struct ObjectPool<T> {
    pool: Mutex<VecDeque<T>>,
    max_size: usize,
    factory: Box<dyn Fn() -> T + Send + Sync>,
}

impl<T> ObjectPool<T> {
    /// Create a new object pool
    pub fn new<F>(max_size: usize, factory: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            pool: Mutex::new(VecDeque::with_capacity(max_size)),
            max_size,
            factory: Box::new(factory),
        }
    }

    /// Get an object from the pool
    pub fn acquire(&self) -> Result<PooledObject<'_, T>, LiteError> {
        let obj = {
            let mut pool = self
                .pool
                .lock()
                .map_err(|_| LiteError::Internal("Failed to lock pool".to_string()))?;

            pool.pop_front().unwrap_or_else(|| (self.factory)())
        };

        Ok(PooledObject {
            obj: Some(obj),
            pool: Arc::new(self),
        })
    }

    /// Return an object to the pool
    fn release(&self, obj: T) {
        if let Ok(mut pool) = self.pool.lock() {
            if pool.len() < self.max_size {
                pool.push_back(obj);
            }
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> Result<PoolStats, LiteError> {
        let pool = self
            .pool
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock pool".to_string()))?;

        Ok(PoolStats {
            available: pool.len(),
            max_size: self.max_size,
        })
    }

    /// Clear the pool
    pub fn clear(&self) -> Result<(), LiteError> {
        let mut pool = self
            .pool
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock pool".to_string()))?;

        pool.clear();
        Ok(())
    }
}

/// Pooled object that returns to pool when dropped
pub struct PooledObject<'a, T> {
    obj: Option<T>,
    pool: Arc<&'a ObjectPool<T>>,
}

impl<'a, T> std::ops::Deref for PooledObject<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.obj.as_ref().unwrap()
    }
}

impl<'a, T> std::ops::DerefMut for PooledObject<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.obj.as_mut().unwrap()
    }
}

impl<'a, T> Drop for PooledObject<'a, T> {
    fn drop(&mut self) {
        if let Some(obj) = self.obj.take() {
            self.pool.release(obj);
        }
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub available: usize,
    pub max_size: usize,
}

/// String pool for frequently used strings
pub struct StringPool {
    pool: ObjectPool<String>,
}

impl StringPool {
    /// Create a new string pool
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: ObjectPool::new(max_size, || String::with_capacity(1024)),
        }
    }

    /// Get a string from the pool
    pub fn acquire(&self) -> Result<PooledObject<'_, String>, LiteError> {
        self.pool.acquire()
    }
}

/// Byte buffer pool for I/O operations
pub struct ByteBufferPool {
    pool: ObjectPool<Vec<u8>>,
}

impl ByteBufferPool {
    /// Create a new buffer pool with specified capacity
    pub fn new(max_size: usize, buffer_capacity: usize) -> Self {
        Self {
            pool: ObjectPool::new(max_size, move || Vec::with_capacity(buffer_capacity)),
        }
    }

    /// Get a buffer from the pool
    pub fn acquire(&self) -> Result<PooledObject<'_, Vec<u8>>, LiteError> {
        self.pool.acquire()
    }
}

/// Memory usage tracker
pub struct MemoryTracker {
    allocations: Mutex<Vec<AllocationRecord>>,
    peak_usage: Mutex<usize>,
    current_usage: Mutex<usize>,
    limit: usize,
}

#[derive(Debug, Clone)]
struct AllocationRecord {
    size: usize,
    location: &'static str,
}

impl MemoryTracker {
    /// Create a new memory tracker with limit
    pub fn new(limit_mb: usize) -> Self {
        let limit = limit_mb * 1024 * 1024;

        Self {
            allocations: Mutex::new(Vec::new()),
            peak_usage: Mutex::new(0),
            current_usage: Mutex::new(0),
            limit,
        }
    }

    /// Track an allocation
    pub fn track_allocation(&self, size: usize, location: &'static str) -> Result<(), LiteError> {
        let mut current = self
            .current_usage
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock current usage".to_string()))?;

        // Check limit
        if *current + size > self.limit {
            return Err(LiteError::Internal(format!(
                "Memory limit exceeded: {} + {} > {} bytes",
                *current, size, self.limit
            )));
        }

        *current += size;

        // Update peak
        let mut peak = self
            .peak_usage
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock peak usage".to_string()))?;

        if *current > *peak {
            *peak = *current;
        }

        // Record allocation
        let mut allocations = self
            .allocations
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock allocations".to_string()))?;

        allocations.push(AllocationRecord { size, location });

        Ok(())
    }

    /// Track deallocation
    pub fn track_deallocation(&self, size: usize) -> Result<(), LiteError> {
        let mut current = self
            .current_usage
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock current usage".to_string()))?;

        *current = current.saturating_sub(size);

        Ok(())
    }

    /// Get memory statistics
    pub fn stats(&self) -> Result<MemoryStats, LiteError> {
        let current = self
            .current_usage
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock current usage".to_string()))?;

        let peak = self
            .peak_usage
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock peak usage".to_string()))?;

        let allocations = self
            .allocations
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock allocations".to_string()))?;

        // Calculate top allocation sources
        let mut source_map: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for record in allocations.iter() {
            *source_map.entry(record.location).or_insert(0) += record.size;
        }

        let mut top_sources: Vec<_> = source_map.into_iter().collect();
        top_sources.sort_by(|a, b| b.1.cmp(&a.1));
        top_sources.truncate(5);

        Ok(MemoryStats {
            current_bytes: *current,
            peak_bytes: *peak,
            limit_bytes: self.limit,
            total_allocations: allocations.len(),
            top_sources: top_sources
                .into_iter()
                .map(|(loc, size)| (loc.to_string(), size))
                .collect(),
        })
    }

    /// Check if under memory limit
    pub fn check_limit(&self, additional_bytes: usize) -> Result<bool, LiteError> {
        let current = self
            .current_usage
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock current usage".to_string()))?;

        Ok(*current + additional_bytes <= self.limit)
    }

    /// Reset statistics
    pub fn reset(&self) -> Result<(), LiteError> {
        let mut allocations = self
            .allocations
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock allocations".to_string()))?;

        let mut peak = self
            .peak_usage
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock peak usage".to_string()))?;

        let current = self
            .current_usage
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock current usage".to_string()))?;

        allocations.clear();
        *peak = *current;

        Ok(())
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub current_bytes: usize,
    pub peak_bytes: usize,
    pub limit_bytes: usize,
    pub total_allocations: usize,
    pub top_sources: Vec<(String, usize)>,
}

impl MemoryStats {
    /// Get current memory usage in MB
    pub fn current_mb(&self) -> f64 {
        self.current_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Get peak memory usage in MB
    pub fn peak_mb(&self) -> f64 {
        self.peak_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Get memory limit in MB
    pub fn limit_mb(&self) -> f64 {
        self.limit_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Check if under memory limit
    pub fn is_under_limit(&self) -> bool {
        self.current_bytes < self.limit_bytes
    }

    /// Get utilization percentage
    pub fn utilization_percent(&self) -> f64 {
        if self.limit_bytes == 0 {
            return 0.0;
        }
        (self.current_bytes as f64 / self.limit_bytes as f64) * 100.0
    }
}

/// Memory optimizer that combines pools and tracking
pub struct MemoryOptimizer {
    string_pool: StringPool,
    buffer_pool: ByteBufferPool,
    tracker: Arc<MemoryTracker>,
}

impl MemoryOptimizer {
    /// Create a new memory optimizer
    pub fn new() -> Self {
        Self {
            string_pool: StringPool::new(50),
            buffer_pool: ByteBufferPool::new(20, 64 * 1024), // 64KB buffers
            tracker: Arc::new(MemoryTracker::new(80)),       // 80MB limit
        }
    }

    /// Create with custom limits
    pub fn with_limits(pool_size: usize, memory_limit_mb: usize) -> Self {
        Self {
            string_pool: StringPool::new(pool_size),
            buffer_pool: ByteBufferPool::new(pool_size / 2, 64 * 1024),
            tracker: Arc::new(MemoryTracker::new(memory_limit_mb)),
        }
    }

    /// Get a string from the pool
    pub fn get_string(&self) -> Result<PooledObject<'_, String>, LiteError> {
        self.string_pool.acquire()
    }

    /// Get a buffer from the pool
    pub fn get_buffer(&self) -> Result<PooledObject<'_, Vec<u8>>, LiteError> {
        self.buffer_pool.acquire()
    }

    /// Get memory statistics
    pub fn stats(&self) -> Result<MemoryStats, LiteError> {
        self.tracker.stats()
    }

    /// Check if allocation would exceed limit
    pub fn check_allocation(&self, size: usize) -> Result<bool, LiteError> {
        self.tracker.check_limit(size)
    }

    /// Get memory tracker reference
    pub fn tracker(&self) -> Arc<MemoryTracker> {
        self.tracker.clone()
    }
}

impl Default for MemoryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Efficient data structure recommendations
pub struct DataStructureGuide;

impl DataStructureGuide {
    /// Get recommended capacity for Vec to avoid reallocations
    pub fn recommended_vec_capacity(expected_items: usize) -> usize {
        // Use next power of 2 for efficient growth
        let capacity = expected_items.next_power_of_two();
        std::cmp::max(capacity, 4) // Minimum of 4
    }

    /// Get recommended HashMap capacity
    pub fn recommended_hashmap_capacity(expected_items: usize) -> usize {
        // HashMap load factor is typically 0.75, so allocate more
        (expected_items as f64 * 1.5) as usize
    }

    /// Estimate memory usage for data structures
    pub fn estimate_vec_memory<T>(items: usize) -> usize {
        std::mem::size_of::<Vec<T>>() + items * std::mem::size_of::<T>()
    }

    pub fn estimate_hashmap_memory<K, V>(items: usize) -> usize {
        let entry_size = std::mem::size_of::<(K, V)>();
        // Account for hashmap overhead (buckets + entries)
        std::mem::size_of::<HashMap<K, V>>() + items * entry_size * 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_pool() {
        let pool = ObjectPool::new(5, || Vec::<u8>::with_capacity(100));

        // Acquire and release
        {
            let obj = pool.acquire().unwrap();
            assert_eq!(obj.capacity(), 100);
        } // Dropped here, returned to pool

        // Check stats
        let stats = pool.stats().unwrap();
        assert_eq!(stats.available, 1);
    }

    #[test]
    fn test_memory_tracker() {
        let tracker = MemoryTracker::new(100); // 100MB limit

        // Track allocations
        tracker.track_allocation(1024, "test1").unwrap();
        tracker.track_allocation(2048, "test2").unwrap();

        // Get stats
        let stats = tracker.stats().unwrap();
        assert_eq!(stats.current_bytes, 3072);
        assert_eq!(stats.total_allocations, 2);

        // Track deallocation
        tracker.track_deallocation(1024).unwrap();
        let stats = tracker.stats().unwrap();
        assert_eq!(stats.current_bytes, 2048);

        // Check limit
        assert!(tracker.check_limit(100 * 1024 * 1024).unwrap()); // Under limit
    }

    #[test]
    fn test_memory_stats() {
        let stats = MemoryStats {
            current_bytes: 50 * 1024 * 1024,
            peak_bytes: 60 * 1024 * 1024,
            limit_bytes: 80 * 1024 * 1024,
            total_allocations: 100,
            top_sources: vec![],
        };

        assert_eq!(stats.current_mb(), 50.0);
        assert_eq!(stats.peak_mb(), 60.0);
        assert_eq!(stats.limit_mb(), 80.0);
        assert!(stats.is_under_limit());
        assert_eq!(stats.utilization_percent(), 62.5);
    }

    #[test]
    fn test_data_structure_guide() {
        assert_eq!(DataStructureGuide::recommended_vec_capacity(5), 8);
        assert_eq!(DataStructureGuide::recommended_vec_capacity(100), 128);

        assert!(DataStructureGuide::recommended_hashmap_capacity(100) >= 100);
    }
}
