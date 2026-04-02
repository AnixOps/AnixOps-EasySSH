//! 进度追踪模块
//!
//! 实时追踪文件传输进度

use crate::sftp::types::{format_duration, format_size};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};

/// 进度回调类型
pub type ProgressCallback = Arc<dyn Fn(ProgressSnapshot) + Send + Sync>;

/// 进度快照
#[derive(Debug, Clone)]
pub struct ProgressSnapshot {
    /// 任务ID
    pub task_id: String,
    /// 文件路径
    pub path: PathBuf,
    /// 文件名
    pub filename: String,
    /// 总大小
    pub total: u64,
    /// 已传输
    pub transferred: u64,
    /// 百分比
    pub percentage: f64,
    /// 传输速度（字节/秒）
    pub speed: f64,
    /// 已用时间
    pub elapsed: Duration,
    /// 预计剩余时间
    pub eta: Option<Duration>,
    /// 是否已完成
    pub is_complete: bool,
    /// 是否出错
    pub has_error: bool,
    /// 错误信息
    pub error_message: Option<String>,
}

impl ProgressSnapshot {
    /// 创建新的进度快照
    pub fn new(task_id: impl Into<String>, path: impl AsRef<Path>, total: u64) -> Self {
        let path = path.as_ref().to_path_buf();
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self {
            task_id: task_id.into(),
            path,
            filename,
            total,
            transferred: 0,
            percentage: 0.0,
            speed: 0.0,
            elapsed: Duration::ZERO,
            eta: None,
            is_complete: false,
            has_error: false,
            error_message: None,
        }
    }

    /// 标记为完成
    pub fn complete(path: impl AsRef<Path>, total: u64) -> Self {
        let path = path.as_ref().to_path_buf();
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self {
            task_id: String::new(),
            path,
            filename,
            total,
            transferred: total,
            percentage: 100.0,
            speed: 0.0,
            elapsed: Duration::ZERO,
            eta: Some(Duration::ZERO),
            is_complete: true,
            has_error: false,
            error_message: None,
        }
    }

    /// 标记为错误
    pub fn error(path: impl AsRef<Path>, error: impl Into<String>) -> Self {
        let path = path.as_ref().to_path_buf();
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self {
            task_id: String::new(),
            path,
            filename,
            total: 0,
            transferred: 0,
            percentage: 0.0,
            speed: 0.0,
            elapsed: Duration::ZERO,
            eta: None,
            is_complete: false,
            has_error: true,
            error_message: Some(error.into()),
        }
    }

    /// 更新进度
    pub fn update(&mut self, transferred: u64, speed: f64, elapsed: Duration) {
        self.transferred = transferred;
        self.speed = speed;
        self.elapsed = elapsed;

        if self.total > 0 {
            self.percentage = (transferred as f64 / self.total as f64 * 100.0).min(100.0);
        }

        // 计算ETA
        if speed > 0.0 && self.total > transferred {
            let remaining = self.total - transferred;
            let eta_secs = remaining as f64 / speed;
            self.eta = Some(Duration::from_secs_f64(eta_secs));
        }
    }

    /// 获取格式化的大小
    pub fn formatted_total(&self) -> String {
        format_size(self.total)
    }

    /// 获取格式化的已传输
    pub fn formatted_transferred(&self) -> String {
        format_size(self.transferred)
    }

    /// 获取格式化的速度
    pub fn formatted_speed(&self) -> String {
        format_size(self.speed as u64) + "/s"
    }

    /// 获取格式化的ETA
    pub fn formatted_eta(&self) -> String {
        match self.eta {
            Some(eta) if eta.as_secs() > 0 => format_duration(eta),
            _ => "-".to_string(),
        }
    }

    /// 获取进度条字符串
    pub fn progress_bar(&self, width: usize) -> String {
        let filled = (self.percentage / 100.0 * width as f64) as usize;
        let empty = width - filled;

        format!(
            "[{}{}] {:.1}%",
            "=".repeat(filled),
            " ".repeat(empty),
            self.percentage
        )
    }
}

/// 进度追踪器
#[derive(Debug, Clone)]
pub struct ProgressTracker {
    /// 所有追踪的任务
    trackers: Arc<RwLock<HashMap<String, TaskProgressTracker>>>,
    /// 全局事件发送器
    event_tx: Option<mpsc::Sender<ProgressEvent>>,
    /// 更新间隔
    update_interval: Duration,
}

/// 单个任务的进度追踪器
#[derive(Debug, Clone)]
struct TaskProgressTracker {
    /// 开始时间
    start_time: Instant,
    /// 最后更新时间
    last_update: Instant,
    /// 已传输字节历史（用于计算速度）
    history: Vec<(Instant, u64)>,
    /// 当前快照
    current: ProgressSnapshot,
    /// 速度计算器
    speed_calculator: SpeedCalculator,
}

/// 进度事件
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// 进度更新
    Update(ProgressSnapshot),
    /// 任务完成
    Complete(String),
    /// 任务失败
    Error(String, String),
}

impl ProgressTracker {
    /// 创建新的进度追踪器
    pub fn new() -> Self {
        Self {
            trackers: Arc::new(RwLock::new(HashMap::new())),
            event_tx: None,
            update_interval: Duration::from_millis(100),
        }
    }

    /// 设置更新间隔
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.update_interval = interval;
        self
    }

    /// 设置事件发送器
    pub fn set_event_sender(&mut self, tx: mpsc::Sender<ProgressEvent>) {
        self.event_tx = Some(tx);
    }

    /// 开始追踪新任务
    pub async fn start_tracking(
        &self,
        task_id: impl Into<String>,
        path: impl AsRef<Path>,
        total: u64,
    ) -> String {
        let task_id = task_id.into();
        let path = path.as_ref().to_path_buf();

        let tracker = TaskProgressTracker {
            start_time: Instant::now(),
            last_update: Instant::now(),
            history: Vec::with_capacity(60),
            current: ProgressSnapshot::new(&task_id, &path, total),
            speed_calculator: SpeedCalculator::new(10),
        };

        let mut trackers = self.trackers.write().await;
        trackers.insert(task_id.clone(), tracker);

        task_id
    }

    /// 更新进度
    pub async fn update(&self, task_id: &str, transferred: u64) {
        let mut trackers = self.trackers.write().await;

        if let Some(tracker) = trackers.get_mut(task_id) {
            let now = Instant::now();
            let elapsed = now.duration_since(tracker.start_time);

            // 更新速度计算器
            tracker.speed_calculator.add_sample(transferred);
            let speed = tracker.speed_calculator.current_speed();

            // 更新历史
            tracker.history.push((now, transferred));
            // 只保留最近60个样本
            if tracker.history.len() > 60 {
                tracker.history.remove(0);
            }

            // 更新快照
            tracker.current.update(transferred, speed, elapsed);
            tracker.last_update = now;

            let snapshot = tracker.current.clone();
            drop(trackers);

            // 发送事件
            self.send_event(ProgressEvent::Update(snapshot)).await;
        }
    }

    /// 完成任务
    pub async fn complete(&self, task_id: &str) -> Option<ProgressSnapshot> {
        let mut trackers = self.trackers.write().await;

        if let Some(tracker) = trackers.get_mut(task_id) {
            tracker.current.transferred = tracker.current.total;
            tracker.current.percentage = 100.0;
            tracker.current.is_complete = true;
            tracker.current.eta = Some(Duration::ZERO);

            let snapshot = tracker.current.clone();
            drop(trackers);

            self.send_event(ProgressEvent::Complete(task_id.to_string()))
                .await;

            Some(snapshot)
        } else {
            None
        }
    }

    /// 标记任务失败
    pub async fn error(&self, task_id: &str, error: impl Into<String>) {
        let mut trackers = self.trackers.write().await;

        if let Some(tracker) = trackers.get_mut(task_id) {
            tracker.current.has_error = true;
            tracker.current.error_message = Some(error.into());
            let error_msg = tracker.current.error_message.clone().unwrap();
            drop(trackers);

            self.send_event(ProgressEvent::Error(task_id.to_string(), error_msg))
                .await;
        }
    }

    /// 获取当前快照
    pub async fn snapshot(&self, task_id: &str) -> Option<ProgressSnapshot> {
        let trackers = self.trackers.read().await;
        trackers.get(task_id).map(|t| t.current.clone())
    }

    /// 获取所有快照
    pub async fn all_snapshots(&self) -> Vec<ProgressSnapshot> {
        let trackers = self.trackers.read().await;
        trackers.values().map(|t| t.current.clone()).collect()
    }

    /// 停止追踪
    pub async fn stop_tracking(&self, task_id: &str) -> Option<ProgressSnapshot> {
        let mut trackers = self.trackers.write().await;
        trackers.remove(task_id).map(|t| t.current)
    }

    /// 停止所有追踪
    pub async fn stop_all(&self) -> Vec<ProgressSnapshot> {
        let mut trackers = self.trackers.write().await;
        trackers.drain().map(|(_, t)| t.current).collect()
    }

    /// 获取任务速度
    pub async fn speed(&self, task_id: &str) -> Option<f64> {
        let trackers = self.trackers.read().await;
        trackers
            .get(task_id)
            .map(|t| t.speed_calculator.current_speed())
    }

    /// 获取整体统计
    pub async fn overall_stats(&self) -> OverallStats {
        let trackers = self.trackers.read().await;

        let mut total_transferred = 0u64;
        let mut total_size = 0u64;
        let mut total_speed = 0.0;
        let mut completed_count = 0usize;
        let mut error_count = 0usize;

        for tracker in trackers.values() {
            total_transferred += tracker.current.transferred;
            total_size += tracker.current.total;
            total_speed += tracker.speed_calculator.current_speed();

            if tracker.current.is_complete {
                completed_count += 1;
            }
            if tracker.current.has_error {
                error_count += 1;
            }
        }

        let overall_progress = if total_size > 0 {
            (total_transferred as f64 / total_size as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        OverallStats {
            total_tasks: trackers.len(),
            completed_tasks: completed_count,
            error_tasks: error_count,
            total_transferred,
            total_size,
            current_speed: total_speed,
            overall_progress,
        }
    }

    /// 发送事件
    async fn send_event(&self, event: ProgressEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event).await;
        }
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// 速度计算器
///
/// 使用滑动窗口计算传输速度
#[derive(Debug, Clone)]
pub struct SpeedCalculator {
    /// 样本窗口大小
    window_size: usize,
    /// 样本（字节位置，时间戳）
    samples: Vec<(u64, Instant)>,
    /// 当前索引
    current_index: usize,
}

impl SpeedCalculator {
    /// 创建新的速度计算器
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            samples: Vec::with_capacity(window_size),
            current_index: 0,
        }
    }

    /// 添加样本
    pub fn add_sample(&mut self, bytes: u64) {
        let now = Instant::now();

        if self.samples.len() < self.window_size {
            self.samples.push((bytes, now));
        } else {
            self.samples[self.current_index] = (bytes, now);
            self.current_index = (self.current_index + 1) % self.window_size;
        }
    }

    /// 计算当前速度
    pub fn current_speed(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }

        // 使用最早的和最新的样本来计算平均速度
        let (first_bytes, first_time) = self.samples[0];
        let (last_bytes, last_time) = self.samples[self.samples.len() - 1];

        let byte_diff = last_bytes.saturating_sub(first_bytes);
        let time_diff = last_time.duration_since(first_time).as_secs_f64();

        if time_diff > 0.0 {
            byte_diff as f64 / time_diff
        } else {
            0.0
        }
    }

    /// 获取样本数量
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// 清空样本
    pub fn clear(&mut self) {
        self.samples.clear();
        self.current_index = 0;
    }
}

/// 整体统计
#[derive(Debug, Clone)]
pub struct OverallStats {
    /// 总任务数
    pub total_tasks: usize,
    /// 已完成任务数
    pub completed_tasks: usize,
    /// 错误任务数
    pub error_tasks: usize,
    /// 总传输字节数
    pub total_transferred: u64,
    /// 总大小
    pub total_size: u64,
    /// 当前总速度
    pub current_speed: f64,
    /// 整体进度
    pub overall_progress: f64,
}

impl OverallStats {
    /// 获取格式化的总大小
    pub fn formatted_total_size(&self) -> String {
        format_size(self.total_size)
    }

    /// 获取格式化的已传输
    pub fn formatted_transferred(&self) -> String {
        format_size(self.total_transferred)
    }

    /// 获取格式化的速度
    pub fn formatted_speed(&self) -> String {
        format_size(self.current_speed as u64) + "/s"
    }

    /// 获取进度条
    pub fn progress_bar(&self, width: usize) -> String {
        let filled = (self.overall_progress / 100.0 * width as f64) as usize;
        let empty = width - filled;

        format!(
            "[{}{}] {:.1}%",
            "=".repeat(filled),
            " ".repeat(empty),
            self.overall_progress
        )
    }
}

/// 进度显示格式化器
pub struct ProgressFormatter;

impl ProgressFormatter {
    /// 格式化进度为单行字符串
    pub fn format_line(snapshot: &ProgressSnapshot) -> String {
        format!(
            "{} {} {} {} ETA: {}",
            snapshot.filename,
            snapshot.progress_bar(20),
            snapshot.formatted_transferred(),
            snapshot.formatted_speed(),
            snapshot.formatted_eta()
        )
    }

    /// 格式化进度为多行字符串
    pub fn format_multiline(snapshot: &ProgressSnapshot) -> String {
        format!(
            "文件: {}\n\
             进度: {}\n\
             大小: {} / {}\n\
             速度: {}\n\
             已用: {}\n\
             剩余: {}",
            snapshot.filename,
            snapshot.progress_bar(30),
            snapshot.formatted_transferred(),
            snapshot.formatted_total(),
            snapshot.formatted_speed(),
            format_duration(snapshot.elapsed),
            snapshot.formatted_eta()
        )
    }

    /// 格式化整体统计
    pub fn format_overall(stats: &OverallStats) -> String {
        format!(
            "任务: {}/{} 完成 | 进度: {} | 速度: {} | 已传输: {}/{}",
            stats.completed_tasks,
            stats.total_tasks,
            stats.progress_bar(20),
            stats.formatted_speed(),
            stats.formatted_transferred(),
            stats.formatted_total_size()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_progress_tracker_new() {
        let tracker = ProgressTracker::new();
        let snapshots = tracker.all_snapshots().await;
        assert!(snapshots.is_empty());
    }

    #[tokio::test]
    async fn test_progress_tracker_start_tracking() {
        let tracker = ProgressTracker::new();
        let task_id = tracker
            .start_tracking("task-1", "/path/to/file.txt", 1024 * 1024)
            .await;

        assert!(!task_id.is_empty());

        let snapshot = tracker.snapshot(&task_id).await.unwrap();
        assert_eq!(snapshot.total, 1024 * 1024);
        assert_eq!(snapshot.filename, "file.txt");
    }

    #[tokio::test]
    async fn test_progress_tracker_update() {
        let tracker = ProgressTracker::new();
        let task_id = tracker
            .start_tracking("task-1", "/path/to/file.txt", 1000)
            .await;

        tracker.update(&task_id, 500).await;

        let snapshot = tracker.snapshot(&task_id).await.unwrap();
        assert_eq!(snapshot.transferred, 500);
        assert_eq!(snapshot.percentage, 50.0);
    }

    #[test]
    fn test_progress_snapshot_progress_bar() {
        let mut snapshot = ProgressSnapshot::new("task-1", "/path/to/file.txt", 100);
        snapshot.percentage = 50.0;

        let bar = snapshot.progress_bar(10);
        assert!(bar.contains("="));
        assert!(bar.contains("50.0%"));
    }

    #[test]
    fn test_speed_calculator() {
        let mut calc = SpeedCalculator::new(10);

        // 模拟添加样本
        calc.add_sample(0);
        calc.add_sample(1024);
        calc.add_sample(2048);

        let speed = calc.current_speed();
        assert!(speed >= 0.0);
    }

    #[test]
    fn test_overall_stats() {
        let stats = OverallStats {
            total_tasks: 10,
            completed_tasks: 5,
            error_tasks: 1,
            total_transferred: 500,
            total_size: 1000,
            current_speed: 100.0,
            overall_progress: 50.0,
        };

        assert_eq!(stats.total_tasks, 10);
        assert_eq!(stats.overall_progress, 50.0);
    }
}
