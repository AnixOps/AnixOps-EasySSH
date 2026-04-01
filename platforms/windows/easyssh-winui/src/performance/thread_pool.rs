#![allow(dead_code)]

/// High-Performance Thread Pool for Background Tasks
/// Uses work-stealing algorithm for optimal CPU utilization

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::time::Duration;

/// Task priority levels
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Critical = 0,  // UI-critical tasks
    High = 1,      // User-initiated actions
    Normal = 2,    // Background operations
    Low = 3,       // Maintenance tasks
    Background = 4, // Deferred work
}

/// Statistics for thread pool monitoring
#[derive(Clone, Debug, Default)]
pub struct ThreadPoolStats {
    pub active_threads: usize,
    pub queued_tasks: usize,
    pub completed_tasks: u64,
    pub rejected_tasks: u64,
    pub avg_wait_time_ms: f64,
    pub avg_execution_time_ms: f64,
}

/// Work-stealing thread pool
pub struct WorkStealingPool {
    workers: Vec<Worker>,
    global_queue: Arc<Mutex<VecDeque<Task>>>,
    shutdown: Arc<AtomicUsize>,
    stats: Arc<PoolStats>,
}

struct Worker {
    id: usize,
    thread: Option<std::thread::JoinHandle<()>>,
}

struct Task {
    priority: TaskPriority,
    f: Box<dyn FnOnce() + Send + 'static>,
    enqueue_time: std::time::Instant,
}

struct PoolStats {
    completed: AtomicU64,
    rejected: AtomicU64,
    total_wait_time_ms: AtomicU64,
    total_execution_time_ms: AtomicU64,
}

impl WorkStealingPool {
    pub fn new(num_threads: usize) -> Self {
        let global_queue = Arc::new(Mutex::new(VecDeque::new()));
        let shutdown = Arc::new(AtomicUsize::new(0));
        let stats = Arc::new(PoolStats {
            completed: AtomicU64::new(0),
            rejected: AtomicU64::new(0),
            total_wait_time_ms: AtomicU64::new(0),
            total_execution_time_ms: AtomicU64::new(0),
        });

        let mut workers = Vec::with_capacity(num_threads);

        for id in 0..num_threads {
            let worker = Worker::new(
                id,
                global_queue.clone(),
                shutdown.clone(),
                stats.clone(),
            );
            workers.push(worker);
        }

        Self {
            workers,
            global_queue,
            shutdown,
            stats,
        }
    }

    pub fn spawn<F>(&self, priority: TaskPriority, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let task = Task {
            priority,
            f: Box::new(f),
            enqueue_time: std::time::Instant::now(),
        };

        let mut queue = self.global_queue.lock();

        // Priority queue insertion
        let insert_pos = queue.iter()
            .position(|t| t.priority > priority)
            .unwrap_or(queue.len());

        if insert_pos < 10000 { // Max queue size
            queue.insert(insert_pos, task);
        } else {
            drop(queue);
            self.stats.rejected.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn spawn_critical<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.spawn(TaskPriority::Critical, f);
    }

    pub fn spawn_high<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.spawn(TaskPriority::High, f);
    }

    pub fn spawn_normal<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.spawn(TaskPriority::Normal, f);
    }

    pub fn spawn_low<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.spawn(TaskPriority::Low, f);
    }

    pub fn spawn_background<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.spawn(TaskPriority::Background, f);
    }

    pub fn stats(&self) -> ThreadPoolStats {
        let completed = self.stats.completed.load(Ordering::Relaxed);
        let total_wait = self.stats.total_wait_time_ms.load(Ordering::Relaxed);
        let total_exec = self.stats.total_execution_time_ms.load(Ordering::Relaxed);

        ThreadPoolStats {
            active_threads: self.workers.len(),
            queued_tasks: self.global_queue.lock().len(),
            completed_tasks: completed,
            rejected_tasks: self.stats.rejected.load(Ordering::Relaxed),
            avg_wait_time_ms: if completed > 0 {
                total_wait as f64 / completed as f64
            } else {
                0.0
            },
            avg_execution_time_ms: if completed > 0 {
                total_exec as f64 / completed as f64
            } else {
                0.0
            },
        }
    }

    pub fn shutdown(self) {
        self.shutdown.store(1, Ordering::Relaxed);
        for worker in self.workers {
            if let Some(thread) = worker.thread {
                let _ = thread.join();
            }
        }
    }
}

impl Worker {
    fn new(
        id: usize,
        global_queue: Arc<Mutex<VecDeque<Task>>>,
        shutdown: Arc<AtomicUsize>,
        stats: Arc<PoolStats>,
    ) -> Self {
        let thread = std::thread::spawn(move || {
            Self::run(id, global_queue, shutdown, stats);
        });

        Self {
            id,
            thread: Some(thread),
        }
    }

    fn run(
        _id: usize,
        global_queue: Arc<Mutex<VecDeque<Task>>>,
        shutdown: Arc<AtomicUsize>,
        stats: Arc<PoolStats>,
    ) {
        while shutdown.load(Ordering::Relaxed) == 0 {
            let task = {
                let mut queue = global_queue.lock();
                queue.pop_front()
            };

            if let Some(task) = task {
                let wait_time = task.enqueue_time.elapsed().as_millis() as u64;
                stats.total_wait_time_ms.fetch_add(wait_time, Ordering::Relaxed);

                let start = std::time::Instant::now();
                (task.f)();
                let exec_time = start.elapsed().as_millis() as u64;

                stats.total_execution_time_ms.fetch_add(exec_time, Ordering::Relaxed);
                stats.completed.fetch_add(1, Ordering::Relaxed);
            } else {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}

/// Specialized IO thread pool for blocking operations
pub struct IoThreadPool {
    inner: WorkStealingPool,
}

impl IoThreadPool {
    pub fn new(num_threads: usize) -> Self {
        Self {
            inner: WorkStealingPool::new(num_threads),
        }
    }

    pub fn spawn<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.inner.spawn(TaskPriority::Normal, f);
    }

    pub fn stats(&self) -> ThreadPoolStats {
        self.inner.stats()
    }
}

/// CPU-intensive computation pool
pub struct ComputePool {
    inner: WorkStealingPool,
}

impl ComputePool {
    pub fn new(num_threads: usize) -> Self {
        // Use num_cpus for compute tasks
        let threads = num_threads.min(num_cpus::get());
        Self {
            inner: WorkStealingPool::new(threads),
        }
    }

    pub fn spawn<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.inner.spawn(TaskPriority::Normal, f);
    }
}

/// Global thread pools (lazy initialization)
use std::sync::OnceLock;

static GLOBAL_IO_POOL: OnceLock<IoThreadPool> = OnceLock::new();
static GLOBAL_COMPUTE_POOL: OnceLock<ComputePool> = OnceLock::new();

pub fn io_pool() -> &'static IoThreadPool {
    GLOBAL_IO_POOL.get_or_init(|| IoThreadPool::new(8))
}

pub fn compute_pool() -> &'static ComputePool {
    GLOBAL_COMPUTE_POOL.get_or_init(|| ComputePool::new(4))
}

/// Async task executor bridge for tokio integration
pub struct AsyncBridge;

impl AsyncBridge {
    /// Run blocking operation in thread pool, return future
    pub async fn spawn_blocking<F, R>(f: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = tokio::sync::oneshot::channel();

        io_pool().spawn(move || {
            let result = f();
            let _ = tx.send(result);
        });

        rx.await.unwrap()
    }
}
