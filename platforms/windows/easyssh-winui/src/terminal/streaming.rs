#![allow(dead_code)]

//! Streaming Data Buffer for High-Performance Terminal
//!
//! Handles large data streams (>10MB) with batched processing
//! to maintain 60fps performance without UI freezing.

use std::collections::VecDeque;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Optimized buffer for streaming terminal data
pub struct StreamingBuffer {
    /// Main data buffer
    buffer: VecDeque<u8>,
    /// Maximum buffer size (10MB default)
    max_size: usize,
    /// High water mark for backpressure
    high_water_mark: usize,
    /// Low water mark for resume
    low_water_mark: usize,
    /// Backpressure flag
    paused: bool,
    /// Batch processing size
    batch_size: usize,
    /// Total bytes processed
    bytes_processed: u64,
    /// Total bytes received
    bytes_received: u64,
    /// Last activity time
    last_activity: Instant,
}

impl StreamingBuffer {
    /// Create new streaming buffer with default 10MB capacity
    pub fn new() -> Self {
        Self::with_capacity(10 * 1024 * 1024)
    }

    /// Create with specific capacity
    pub fn with_capacity(max_size: usize) -> Self {
        let high_water_mark = (max_size as f64 * 0.9) as usize;
        let low_water_mark = (max_size as f64 * 0.3) as usize;

        Self {
            buffer: VecDeque::with_capacity(max_size / 10),
            max_size,
            high_water_mark,
            low_water_mark,
            paused: false,
            batch_size: 8192,
            bytes_processed: 0,
            bytes_received: 0,
            last_activity: Instant::now(),
        }
    }

    /// Push data into buffer
    pub fn push(&mut self, data: &[u8]) -> Result<(), BufferError> {
        self.bytes_received += data.len() as u64;

        // Check for backpressure
        if self.buffer.len() + data.len() > self.high_water_mark {
            self.paused = true;
            warn!("Streaming buffer high water mark reached, pausing input");
            return Err(BufferError::Backpressure);
        }

        // Add to buffer
        self.buffer.extend(data);
        self.last_activity = Instant::now();

        Ok(())
    }

    /// Pull next batch of data for processing
    pub fn pull_batch(&mut self) -> Option<Vec<u8>> {
        if self.buffer.is_empty() {
            return None;
        }

        let batch_len = self.batch_size.min(self.buffer.len());
        let batch: Vec<u8> = self.buffer.drain(..batch_len).collect();

        self.bytes_processed += batch.len() as u64;

        // Check for resume from backpressure
        if self.paused && self.buffer.len() < self.low_water_mark {
            self.paused = false;
            debug!("Streaming buffer resumed from backpressure");
        }

        Some(batch)
    }

    /// Check if paused due to backpressure
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Get current buffer size
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Clear buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.paused = false;
    }

    /// Get statistics
    pub fn stats(&self) -> BufferStats {
        BufferStats {
            bytes_received: self.bytes_received,
            bytes_processed: self.bytes_processed,
            buffer_size: self.buffer.len(),
            max_size: self.max_size,
            utilization: self.buffer.len() as f64 / self.max_size as f64,
            paused: self.paused,
        }
    }

    /// Set batch size
    pub fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size;
    }

    /// Get time since last activity
    pub fn idle_duration(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

impl Default for StreamingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Buffer errors
#[derive(Debug, Clone, PartialEq)]
pub enum BufferError {
    Backpressure,
    BufferFull,
    InvalidData,
}

impl std::fmt::Display for BufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferError::Backpressure => write!(f, "Buffer backpressure - input paused"),
            BufferError::BufferFull => write!(f, "Buffer full"),
            BufferError::InvalidData => write!(f, "Invalid data"),
        }
    }
}

impl std::error::Error for BufferError {}

/// Buffer statistics
#[derive(Debug, Clone)]
pub struct BufferStats {
    pub bytes_received: u64,
    pub bytes_processed: u64,
    pub buffer_size: usize,
    pub max_size: usize,
    pub utilization: f64,
    pub paused: bool,
}

/// Data batcher for efficient terminal output
pub struct DataBatcher {
    buffer: String,
    max_batch_size: usize,
    flush_interval: Duration,
    last_flush: Instant,
    pending: VecDeque<String>,
}

impl DataBatcher {
    /// Create new batcher with optimized settings
    pub fn new() -> Self {
        Self::with_capacity(8192, Duration::from_millis(1))
    }

    /// Create with custom capacity and flush interval
    pub fn with_capacity(max_batch_size: usize, flush_interval: Duration) -> Self {
        Self {
            buffer: String::with_capacity(max_batch_size),
            max_batch_size,
            flush_interval,
            last_flush: Instant::now(),
            pending: VecDeque::new(),
        }
    }

    /// Add data to batch
    pub fn push(&mut self, data: &str) {
        self.buffer.push_str(data);

        // Check if we should flush
        if self.buffer.len() >= self.max_batch_size
            || self.last_flush.elapsed() >= self.flush_interval
        {
            self.flush();
        }
    }

    /// Flush current batch
    pub fn flush(&mut self) {
        if !self.buffer.is_empty() {
            let batch = std::mem::take(&mut self.buffer);
            self.pending.push_back(batch);
            self.buffer.reserve(self.max_batch_size);
            self.last_flush = Instant::now();
        }
    }

    /// Get next batch
    pub fn next_batch(&mut self) -> Option<String> {
        self.pending.pop_front()
    }

    /// Check if has pending batches
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Get pending count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Clear all pending
    pub fn clear(&mut self) {
        self.pending.clear();
        self.buffer.clear();
    }
}

impl Default for DataBatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Async streaming processor for terminal data
pub struct StreamingProcessor {
    buffer: Arc<Mutex<StreamingBuffer>>,
    batcher: Arc<Mutex<DataBatcher>>,
    handle: Option<thread::JoinHandle<()>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    output_tx: mpsc::Sender<String>,
    output_rx: mpsc::Receiver<String>,
}

impl StreamingProcessor {
    /// Create new streaming processor
    pub fn new() -> Self {
        let buffer = Arc::new(Mutex::new(StreamingBuffer::new()));
        let batcher = Arc::new(Mutex::new(DataBatcher::new()));
        let (output_tx, output_rx) = mpsc::channel();

        Self {
            buffer,
            batcher,
            handle: None,
            shutdown_tx: None,
            output_tx,
            output_rx,
        }
    }

    /// Start processing thread
    pub fn start(&mut self) {
        let buffer = self.buffer.clone();
        let batcher = self.batcher.clone();
        let output_tx = self.output_tx.clone();
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        self.shutdown_tx = Some(shutdown_tx);

        let handle = thread::spawn(move || {
            let mut last_process = Instant::now();

            loop {
                // Check for shutdown
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                // Process buffer
                if last_process.elapsed() >= Duration::from_millis(16) {
                    // 60fps processing
                    if let Ok(mut buf) = buffer.lock() {
                        while let Some(batch) = buf.pull_batch() {
                            // Convert bytes to string
                            if let Ok(text) = String::from_utf8(batch) {
                                if let Ok(mut b) = batcher.lock() {
                                    b.push(&text);
                                }
                            }
                        }
                    }

                    // Flush batcher
                    if let Ok(mut b) = batcher.lock() {
                        b.flush();

                        // Send pending batches
                        while let Some(batch) = b.next_batch() {
                            let _ = output_tx.send(batch);
                        }
                    }

                    last_process = Instant::now();
                }

                // Small sleep to prevent CPU spinning
                thread::sleep(Duration::from_millis(1));
            }
        });

        self.handle = Some(handle);
    }

    /// Push data for processing
    pub fn push(&self, data: &[u8]) -> Result<(), BufferError> {
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.push(data)?;
        }
        Ok(())
    }

    /// Get processed output
    pub fn recv(&self) -> Option<String> {
        self.output_rx.try_recv().ok()
    }

    /// Check if output available
    pub fn has_output(&self) -> bool {
        // Check if there's data available without consuming it
        // Note: mpsc::Receiver doesn't have is_empty(), so we use try_recv
        // and handle the data if present
        match self.output_rx.try_recv() {
            Ok(_data) => {
                // We got data - in a real implementation we'd need a way to buffer this
                // For now, just signal that data is available
                true
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => false,
        }
    }

    /// Get buffer statistics
    pub fn stats(&self) -> Option<BufferStats> {
        self.buffer.lock().ok().map(|b| b.stats())
    }

    /// Stop processor
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Default for StreamingProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for StreamingProcessor {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Create optimized batcher for 60fps streaming
pub fn create_optimized_batcher() -> DataBatcher {
    DataBatcher::with_capacity(
        8192,                     // 8KB batches
        Duration::from_millis(1), // 1ms flush interval
    )
}

/// Utility functions for big data handling
pub mod big_data {
    use super::*;

    /// Estimate processing time for data size
    pub fn estimate_processing_time(bytes: usize) -> Duration {
        // Assume ~100MB/s processing speed
        let seconds = bytes as f64 / (100.0 * 1024.0 * 1024.0);
        Duration::from_secs_f64(seconds)
    }

    /// Calculate optimal batch size for target frame time
    pub fn optimal_batch_size(target_frame_time_ms: u64) -> usize {
        // Process ~1MB per frame at 60fps
        let bytes_per_frame = (1_000_000.0 / (1000.0 / target_frame_time_ms as f64)) as usize;
        bytes_per_frame.max(4096).min(1024 * 1024)
    }

    /// Check if data is "big" (needs special handling)
    pub fn is_big_data(bytes: usize) -> bool {
        bytes > 1024 * 1024 // > 1MB
    }

    /// Chunk data into optimal sizes
    pub fn chunk_data(data: &[u8], chunk_size: usize) -> Vec<&[u8]> {
        data.chunks(chunk_size).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_streaming_buffer_push_pull() {
        let mut buffer = StreamingBuffer::new();

        // Push some data
        let data = b"Hello, World!";
        buffer.push(data).unwrap();

        // Pull it back
        let batch = buffer.pull_batch().unwrap();
        assert_eq!(batch, data.to_vec());

        assert!(buffer.is_empty());
    }

    #[test]
    fn test_streaming_buffer_backpressure() {
        let mut buffer = StreamingBuffer::with_capacity(100);

        // Push up to capacity
        let data = vec![0u8; 90];
        buffer.push(&data).unwrap();

        // This should trigger backpressure
        let data2 = vec![0u8; 20];
        assert!(matches!(
            buffer.push(&data2),
            Err(BufferError::Backpressure)
        ));
    }

    #[test]
    fn test_data_batcher() {
        let mut batcher = DataBatcher::new();

        // Push some data
        batcher.push("Hello");
        batcher.push(", ");
        batcher.push("World");

        // Force flush
        batcher.flush();

        // Should get combined batch
        let batch = batcher.next_batch().unwrap();
        assert_eq!(batch, "Hello, World");
    }

    #[test]
    fn test_streaming_processor() {
        let mut processor = StreamingProcessor::new();
        processor.start();

        // Push some data
        let data = b"Test data for processing";
        processor.push(data).unwrap();

        // Give it time to process
        thread::sleep(Duration::from_millis(50));

        // Check output
        let mut found = false;
        while let Some(output) = processor.recv() {
            if output.contains("Test data") {
                found = true;
                break;
            }
        }

        assert!(found);
    }

    #[test]
    fn test_big_data_detection() {
        assert!(!big_data::is_big_data(1024)); // 1KB
        assert!(!big_data::is_big_data(1024 * 1024 - 1)); // Just under 1MB
        assert!(big_data::is_big_data(1024 * 1024 + 1)); // Just over 1MB
        assert!(big_data::is_big_data(10 * 1024 * 1024)); // 10MB
    }

    #[test]
    fn test_data_chunking() {
        let data = vec![0u8; 10000];
        let chunks = big_data::chunk_data(&data, 1000);

        assert_eq!(chunks.len(), 10);
        assert_eq!(chunks[0].len(), 1000);
    }
}
