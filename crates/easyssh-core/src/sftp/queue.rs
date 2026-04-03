//! 传输队列模块
//!
//! 管理批量文件传输任务

use crate::error::LiteError;
use crate::sftp::types::{
    TransferDirection, TransferOptions, TransferResult, TransferStatus, TransferTask,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

/// 传输队列
///
/// 管理多个传输任务的队列系统
#[derive(Debug)]
pub struct TransferQueue {
    /// 所有任务
    tasks: Arc<RwLock<HashMap<String, TransferTask>>>,
    /// 等待队列
    pending: Arc<RwLock<VecDeque<String>>>,
    /// 活跃任务ID
    active: Arc<RwLock<Vec<String>>>,
    /// 配置
    config: QueueConfig,
    /// 全局取消标志
    cancel_flag: Arc<AtomicBool>,
    /// 暂停标志
    paused: Arc<AtomicBool>,
    /// 最大并发数
    max_concurrent: Arc<AtomicUsize>,
    /// 事件发送器
    event_tx: Option<mpsc::Sender<QueueEvent>>,
}

/// 队列配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// 最大并发传输数
    pub max_concurrent: usize,
    /// 失败重试次数
    pub retry_count: u32,
    /// 重试间隔
    pub retry_delay: Duration,
    /// 自动开始新任务
    pub auto_start: bool,
    /// 队列完成时自动清理
    pub auto_cleanup: bool,
    /// 保留已完成任务的时间
    pub completed_retention: Duration,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            retry_count: 3,
            retry_delay: Duration::from_secs(5),
            auto_start: true,
            auto_cleanup: false,
            completed_retention: Duration::from_secs(3600),
        }
    }
}

impl QueueConfig {
    /// 设置最大并发数
    pub fn with_concurrent(mut self, concurrent: usize) -> Self {
        self.max_concurrent = concurrent.max(1);
        self
    }

    /// 设置重试次数
    pub fn with_retry(mut self, count: u32, delay: Duration) -> Self {
        self.retry_count = count;
        self.retry_delay = delay;
        self
    }

    /// 设置自动开始
    pub fn with_auto_start(mut self, auto_start: bool) -> Self {
        self.auto_start = auto_start;
        self
    }

    /// 设置自动清理
    pub fn with_auto_cleanup(mut self, auto_cleanup: bool) -> Self {
        self.auto_cleanup = auto_cleanup;
        self
    }
}

/// 队列事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueueEvent {
    /// 任务已添加
    TaskAdded { task_id: String },
    /// 任务已开始
    TaskStarted { task_id: String },
    /// 任务进度更新
    TaskProgress {
        task_id: String,
        progress: f64,
        transferred: u64,
        total: u64,
        speed: f64,
    },
    /// 任务已完成
    TaskCompleted {
        task_id: String,
        result: TransferResult,
    },
    /// 任务失败
    TaskFailed { task_id: String, error: String },
    /// 任务已暂停
    TaskPaused { task_id: String },
    /// 任务已恢复
    TaskResumed { task_id: String },
    /// 任务已取消
    TaskCancelled { task_id: String },
    /// 队列已完成
    QueueCompleted,
    /// 队列错误
    QueueError { error: String },
}

/// 队列统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueStats {
    /// 总任务数
    pub total: usize,
    /// 等待中任务数
    pub pending: usize,
    /// 传输中任务数
    pub active: usize,
    /// 已完成任务数
    pub completed: usize,
    /// 失败任务数
    pub failed: usize,
    /// 已取消任务数
    pub cancelled: usize,
    /// 已暂停任务数
    pub paused: usize,
    /// 总传输字节数
    pub total_bytes: u64,
    /// 当前总速度（字节/秒）
    pub current_speed: f64,
    /// 整体进度（0-100）
    pub overall_progress: f64,
}

impl TransferQueue {
    /// 创建新的传输队列
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            pending: Arc::new(RwLock::new(VecDeque::new())),
            active: Arc::new(RwLock::new(Vec::new())),
            config: QueueConfig::default(),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            max_concurrent: Arc::new(AtomicUsize::new(3)),
            event_tx: None,
        }
    }

    /// 使用配置创建队列
    pub fn with_config(mut self, config: QueueConfig) -> Self {
        self.max_concurrent = Arc::new(AtomicUsize::new(config.max_concurrent));
        self.config = config;
        self
    }

    /// 设置事件发送器
    pub fn set_event_sender(&mut self, tx: mpsc::Sender<QueueEvent>) {
        self.event_tx = Some(tx);
    }

    /// 添加传输任务
    pub async fn add(&self, task: TransferTask) -> String {
        let id = task.id.clone();
        let is_pending = task.status == TransferStatus::Pending;

        // 存储任务
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(id.clone(), task);
        }

        // 如果是等待状态，加入待处理队列
        if is_pending {
            let mut pending = self.pending.write().await;
            pending.push_back(id.clone());
        }

        // 发送事件
        self.send_event(QueueEvent::TaskAdded {
            task_id: id.clone(),
        })
        .await;

        // 如果自动开始且未暂停，尝试启动任务
        if self.config.auto_start && !self.paused.load(Ordering::Relaxed) {
            self.process_queue().await.ok();
        }

        id
    }

    /// 添加下载任务
    pub async fn add_download(
        &self,
        source: impl Into<PathBuf>,
        destination: impl Into<PathBuf>,
        client_id: impl Into<String>,
        options: Option<TransferOptions>,
    ) -> String {
        let options = options.unwrap_or_default();

        let task = TransferTask::new(
            source.into(),
            destination.into(),
            TransferDirection::Download,
            client_id.into(),
        )
        .with_options(options);

        self.add(task).await
    }

    /// 添加上传任务
    pub async fn add_upload(
        &self,
        source: impl Into<PathBuf>,
        destination: impl Into<PathBuf>,
        client_id: impl Into<String>,
        options: Option<TransferOptions>,
    ) -> String {
        let options = options.unwrap_or_default();

        let task = TransferTask::new(
            source.into(),
            destination.into(),
            TransferDirection::Upload,
            client_id.into(),
        )
        .with_options(options);

        self.add(task).await
    }

    /// 获取任务
    pub async fn get(&self, task_id: &str) -> Option<TransferTask> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// 获取所有任务
    pub async fn list(&self) -> Vec<TransferTask> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// 获取任务ID列表
    pub async fn list_ids(&self) -> Vec<String> {
        let tasks = self.tasks.read().await;
        tasks.keys().cloned().collect()
    }

    /// 获取指定状态的任务
    pub async fn list_by_status(&self, status: TransferStatus) -> Vec<TransferTask> {
        let tasks = self.tasks.read().await;
        tasks
            .values()
            .filter(|t| t.status == status)
            .cloned()
            .collect()
    }

    /// 启动队列处理
    pub async fn start(&self) -> Result<(), LiteError> {
        self.paused.store(false, Ordering::Relaxed);
        self.process_queue().await
    }

    /// 暂停队列
    pub async fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);

        // 暂停所有活跃任务
        let active_ids = {
            let active = self.active.read().await;
            active.clone()
        };

        for id in active_ids {
            let _ = self.pause_task(&id).await;
        }
    }

    /// 恢复队列
    pub async fn resume(&self) -> Result<(), LiteError> {
        self.paused.store(false, Ordering::Relaxed);

        // 恢复所有暂停的任务
        let paused_tasks = self.list_by_status(TransferStatus::Paused).await;
        for task in paused_tasks {
            let mut tasks = self.tasks.write().await;
            if let Some(t) = tasks.get_mut(&task.id) {
                t.resume();

                // 重新加入待处理队列
                let mut pending = self.pending.write().await;
                if !pending.contains(&t.id) {
                    pending.push_back(t.id.clone());
                }
            }
        }

        self.process_queue().await
    }

    /// 暂停单个任务
    pub async fn pause_task(&self, task_id: &str) -> Result<(), LiteError> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            if task.can_pause() {
                task.pause();
                self.send_event(QueueEvent::TaskPaused {
                    task_id: task_id.to_string(),
                })
                .await;
                Ok(())
            } else {
                Err(LiteError::Io("任务当前不能暂停".to_string()))
            }
        } else {
            Err(LiteError::Io(format!("任务不存在: {}", task_id)))
        }
    }

    /// 恢复单个任务
    pub async fn resume_task(&self, task_id: &str) -> Result<(), LiteError> {
        {
            let mut tasks = self.tasks.write().await;

            if let Some(task) = tasks.get_mut(task_id) {
                if task.can_resume() {
                    task.resume();
                } else {
                    return Err(LiteError::Io("任务当前不能恢复".to_string()));
                }
            } else {
                return Err(LiteError::Io(format!("任务不存在: {}", task_id)));
            }
        }

        // 重新加入待处理队列
        let mut pending = self.pending.write().await;
        if !pending.contains(&task_id.to_string()) {
            pending.push_back(task_id.to_string());
        }

        self.send_event(QueueEvent::TaskResumed {
            task_id: task_id.to_string(),
        })
        .await;

        // 尝试启动
        self.process_queue().await
    }

    /// 取消任务
    pub async fn cancel(&self, task_id: &str) -> Result<(), LiteError> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            if task.can_cancel() {
                task.cancel();

                // 从活跃列表中移除
                let mut active = self.active.write().await;
                active.retain(|id| id != task_id);

                self.send_event(QueueEvent::TaskCancelled {
                    task_id: task_id.to_string(),
                })
                .await;

                Ok(())
            } else {
                Err(LiteError::Io("任务已完成，无法取消".to_string()))
            }
        } else {
            Err(LiteError::Io(format!("任务不存在: {}", task_id)))
        }
    }

    /// 移除任务
    pub async fn remove(&self, task_id: &str) -> Result<(), LiteError> {
        // 先取消（如果是活跃的）
        let _ = self.cancel(task_id).await;

        // 从各个队列中移除
        {
            let mut pending = self.pending.write().await;
            pending.retain(|id| id != task_id);
        }

        {
            let mut active = self.active.write().await;
            active.retain(|id| id != task_id);
        }

        {
            let mut tasks = self.tasks.write().await;
            tasks.remove(task_id);
        }

        Ok(())
    }

    /// 清理已完成的任务
    pub async fn cleanup_completed(&self) -> usize {
        let completed_ids: Vec<String> = {
            let tasks = self.tasks.read().await;
            tasks
                .values()
                .filter(|t| t.status.is_done())
                .map(|t| t.id.clone())
                .collect()
        };

        for id in &completed_ids {
            let _ = self.remove(id).await;
        }

        completed_ids.len()
    }

    /// 清理所有任务
    pub async fn clear(&self) {
        // 取消所有活跃任务
        let active_ids = {
            let active = self.active.read().await;
            active.clone()
        };

        for id in active_ids {
            let _ = self.cancel(&id).await;
        }

        // 清空所有队列
        {
            let mut tasks = self.tasks.write().await;
            tasks.clear();
        }

        {
            let mut pending = self.pending.write().await;
            pending.clear();
        }

        {
            let mut active = self.active.write().await;
            active.clear();
        }
    }

    /// 获取队列统计
    pub async fn stats(&self) -> QueueStats {
        let tasks = self.tasks.read().await;
        let active = self.active.read().await;

        let mut stats = QueueStats::default();
        stats.total = tasks.len();
        stats.active = active.len();

        let mut total_bytes = 0u64;
        let mut current_speed = 0.0;

        for task in tasks.values() {
            match task.status {
                TransferStatus::Pending => stats.pending += 1,
                TransferStatus::Transferring => {
                    current_speed += task.speed_bps();
                }
                TransferStatus::Paused => stats.paused += 1,
                TransferStatus::Completed => {
                    stats.completed += 1;
                }
                TransferStatus::Failed => stats.failed += 1,
                TransferStatus::Cancelled => stats.cancelled += 1,
                _ => {}
            }
            total_bytes += task.total_bytes;
        }

        stats.total_bytes = total_bytes;
        stats.current_speed = current_speed;

        // 计算整体进度
        if total_bytes > 0 {
            let total_progress: f64 = tasks
                .values()
                .map(|t| {
                    if t.status == TransferStatus::Completed {
                        1.0
                    } else {
                        t.progress / 100.0
                    }
                })
                .sum();
            stats.overall_progress = (total_progress / tasks.len() as f64) * 100.0;
        }

        stats
    }

    /// 处理队列（启动等待中的任务）
    async fn process_queue(&self) -> Result<(), LiteError> {
        if self.paused.load(Ordering::Relaxed) {
            return Ok(());
        }

        if self.cancel_flag.load(Ordering::Relaxed) {
            return Ok(());
        }

        let max_concurrent = self.max_concurrent.load(Ordering::Relaxed);

        loop {
            // 检查当前活跃任务数
            let active_count = {
                let active = self.active.read().await;
                active.len()
            };

            if active_count >= max_concurrent {
                break;
            }

            // 获取下一个待处理任务
            let next_task_id = {
                let mut pending = self.pending.write().await;
                pending.pop_front()
            };

            if let Some(task_id) = next_task_id {
                // 检查任务状态
                let can_start = {
                    let tasks = self.tasks.read().await;
                    if let Some(task) = tasks.get(&task_id) {
                        task.status == TransferStatus::Pending
                    } else {
                        false
                    }
                };

                if can_start {
                    // 启动任务
                    self.start_task(&task_id).await.ok();
                }
            } else {
                // 没有更多等待中的任务
                break;
            }
        }

        Ok(())
    }

    /// 启动单个任务
    async fn start_task(&self, task_id: &str) -> Result<(), LiteError> {
        // 更新任务状态
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(task_id) {
                task.start();
            } else {
                return Err(LiteError::Io(format!("任务不存在: {}", task_id)));
            }
        }

        // 添加到活跃列表
        {
            let mut active = self.active.write().await;
            active.push(task_id.to_string());
        }

        // 发送事件
        self.send_event(QueueEvent::TaskStarted {
            task_id: task_id.to_string(),
        })
        .await;

        Ok(())
    }

    /// 更新任务进度
    pub async fn update_progress(&self, task_id: &str, transferred: u64) {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            task.update_progress(transferred);

            // 发送进度事件
            let event = QueueEvent::TaskProgress {
                task_id: task_id.to_string(),
                progress: task.progress,
                transferred: task.transferred_bytes,
                total: task.total_bytes,
                speed: task.speed_bps(),
            };
            drop(tasks);
            self.send_event(event).await;
        }
    }

    /// 标记任务完成
    pub async fn complete_task(&self, task_id: &str, result: TransferResult) {
        // 更新任务状态
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(task_id) {
                task.complete();
            }
        }

        // 从活跃列表移除
        {
            let mut active = self.active.write().await;
            active.retain(|id| id != task_id);
        }

        // 发送事件
        self.send_event(QueueEvent::TaskCompleted {
            task_id: task_id.to_string(),
            result,
        })
        .await;

        // 如果配置了自动清理
        if self.config.auto_cleanup {
            let _ = self.remove(task_id).await;
        }

        // 继续处理队列
        self.process_queue().await.ok();
    }

    /// 标记任务失败
    pub async fn fail_task(&self, task_id: &str, error: impl Into<String>) {
        let error_msg = error.into();

        // 更新任务状态
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(task_id) {
                task.fail(&error_msg);
            }
        }

        // 从活跃列表移除
        {
            let mut active = self.active.write().await;
            active.retain(|id| id != task_id);
        }

        // 发送事件
        self.send_event(QueueEvent::TaskFailed {
            task_id: task_id.to_string(),
            error: error_msg,
        })
        .await;

        // 继续处理队列
        self.process_queue().await.ok();
    }

    /// 发送事件
    async fn send_event(&self, event: QueueEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event).await;
        }
    }

    /// 等待队列完成
    pub async fn wait_complete(&self) {
        loop {
            let stats = self.stats().await;

            // 如果没有等待中或传输中的任务，队列完成
            if stats.pending == 0 && stats.active == 0 {
                self.send_event(QueueEvent::QueueCompleted).await;
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

impl Default for TransferQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// 批量传输操作
pub struct BatchTransfer {
    queue: TransferQueue,
    tasks: Vec<String>,
}

impl BatchTransfer {
    /// 创建新的批量传输
    pub fn new(queue: TransferQueue) -> Self {
        Self {
            queue,
            tasks: Vec::new(),
        }
    }

    /// 添加下载任务
    pub async fn add_download(
        &mut self,
        source: impl Into<PathBuf>,
        destination: impl Into<PathBuf>,
        client_id: impl Into<String>,
    ) -> &mut Self {
        let id = self
            .queue
            .add_download(source, destination, client_id, None)
            .await;
        self.tasks.push(id);
        self
    }

    /// 添加上传任务
    pub async fn add_upload(
        &mut self,
        source: impl Into<PathBuf>,
        destination: impl Into<PathBuf>,
        client_id: impl Into<String>,
    ) -> &mut Self {
        let id = self
            .queue
            .add_upload(source, destination, client_id, None)
            .await;
        self.tasks.push(id);
        self
    }

    /// 执行所有任务
    pub async fn execute(&self) -> Result<Vec<TransferResult>, LiteError> {
        self.queue.start().await?;
        self.queue.wait_complete().await;

        // 收集结果
        let mut results = Vec::new();
        for task_id in &self.tasks {
            if let Some(task) = self.queue.get(task_id).await {
                if task.status == TransferStatus::Completed {
                    // 构造结果
                    let result =
                        TransferResult::new(task_id, task.transferred_bytes, Duration::ZERO);
                    results.push(result);
                }
            }
        }

        Ok(results)
    }

    /// 取消所有任务
    pub async fn cancel(&self) {
        for task_id in &self.tasks {
            let _ = self.queue.cancel(task_id).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transfer_queue_new() {
        let queue = TransferQueue::new();
        let stats = queue.stats().await;
        assert_eq!(stats.total, 0);
        assert_eq!(stats.pending, 0);
    }

    #[tokio::test]
    async fn test_transfer_queue_add() {
        let queue = TransferQueue::new();
        let task = TransferTask::new(
            "/remote/file.txt",
            "/local/file.txt",
            TransferDirection::Download,
            "client-1",
        );

        let id = queue.add(task).await;
        assert!(!id.is_empty());

        let stats = queue.stats().await;
        assert_eq!(stats.total, 1);
        assert_eq!(stats.pending, 1);
    }

    #[tokio::test]
    async fn test_transfer_queue_cancel() {
        let queue = TransferQueue::new();
        let task = TransferTask::new(
            "/remote/file.txt",
            "/local/file.txt",
            TransferDirection::Download,
            "client-1",
        );

        let id = queue.add(task).await;
        queue.cancel(&id).await.unwrap();

        let retrieved = queue.get(&id).await.unwrap();
        assert_eq!(retrieved.status, TransferStatus::Cancelled);
    }

    #[test]
    fn test_queue_config_default() {
        let config = QueueConfig::default();
        assert_eq!(config.max_concurrent, 3);
        assert_eq!(config.retry_count, 3);
        assert!(config.auto_start);
    }

    #[tokio::test]
    async fn test_transfer_queue_clear() {
        let queue = TransferQueue::new();

        for i in 0..5 {
            let task = TransferTask::new(
                format!("/remote/file{}.txt", i),
                format!("/local/file{}.txt", i),
                TransferDirection::Download,
                "client-1",
            );
            queue.add(task).await;
        }

        assert_eq!(queue.stats().await.total, 5);
        queue.clear().await;
        assert_eq!(queue.stats().await.total, 0);
    }
}
