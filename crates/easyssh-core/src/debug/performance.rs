//! 性能监控模块
//!
//! 提供系统性能监控功能

use crate::debug::access::get_access_level;
use crate::debug::types::*;
use crate::debug::DebugAccessLevel;
use std::time::Instant;

/// 获取系统性能指标
pub fn get_performance_metrics() -> Result<PerformanceMetrics, String> {
    // Lite版本只返回基础信息
    let level = get_access_level();

    // 基础指标（所有版本）
    let mut metrics = PerformanceMetrics {
        cpu_usage: 0.0,
        memory_usage_mb: 0.0,
        memory_total_mb: 0.0,
        disk_usage_gb: 0.0,
        disk_total_gb: 0.0,
        network_latency_ms: None,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    // Standard+ 版本提供更详细的信息
    if level.allows_performance_monitoring() {
        // 在实际实现中，这里会调用系统API获取真实数据
        metrics.cpu_usage = get_cpu_usage()?;
        metrics.memory_usage_mb = get_memory_usage()?;
        metrics.memory_total_mb = get_memory_total()?;
    }

    // Pro版本包含网络延迟
    if level == DebugAccessLevel::Admin {
        metrics.network_latency_ms = Some(0.0); // 占位值
    }

    Ok(metrics)
}

/// 获取CPU使用率
fn get_cpu_usage() -> Result<f64, String> {
    // 简化实现，实际应使用系统API
    // Windows: windows-sys crate
    // Linux: /proc/stat
    // macOS: host_statistics
    Ok(0.0)
}

/// 获取内存使用
fn get_memory_usage() -> Result<f64, String> {
    // 简化实现
    Ok(0.0)
}

/// 获取总内存
fn get_memory_total() -> Result<f64, String> {
    // 简化实现
    Ok(0.0)
}

/// 性能采样器
pub struct PerformanceSampler {
    samples: Vec<PerformanceMetrics>,
    max_samples: usize,
    last_sample_time: Instant,
    sample_interval_ms: u64,
}

impl PerformanceSampler {
    /// 创建新的性能采样器
    pub fn new(max_samples: usize, sample_interval_ms: u64) -> Self {
        Self {
            samples: Vec::with_capacity(max_samples),
            max_samples,
            last_sample_time: Instant::now(),
            sample_interval_ms,
        }
    }

    /// 尝试采样（基于时间间隔）
    pub fn try_sample(&mut self) -> Result<Option<PerformanceMetrics>, String> {
        let elapsed = self.last_sample_time.elapsed().as_millis() as u64;

        if elapsed >= self.sample_interval_ms {
            let metrics = get_performance_metrics()?;

            // 添加新样本，移除旧样本
            if self.samples.len() >= self.max_samples {
                self.samples.remove(0);
            }
            self.samples.push(metrics.clone());
            self.last_sample_time = Instant::now();

            Ok(Some(metrics))
        } else {
            Ok(None)
        }
    }

    /// 获取所有样本
    pub fn get_samples(&self) -> &[PerformanceMetrics] {
        &self.samples
    }

    /// 计算平均CPU使用率
    pub fn avg_cpu_usage(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.iter().map(|s| s.cpu_usage).sum::<f64>() / self.samples.len() as f64
    }

    /// 计算平均内存使用
    pub fn avg_memory_usage(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.iter().map(|s| s.memory_usage_mb).sum::<f64>() / self.samples.len() as f64
    }

    /// 清除所有样本
    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

/// 性能计时器
pub struct PerformanceTimer {
    start: Instant,
    name: String,
}

impl PerformanceTimer {
    /// 创建新的计时器
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            name: name.into(),
        }
    }

    /// 获取经过的时间
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    /// 获取经过的毫秒数
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed().as_millis() as u64
    }

    /// 记录并停止计时
    pub fn finish(self) -> PerformanceRecord {
        let duration = self.elapsed_ms();
        PerformanceRecord {
            name: self.name,
            duration_ms: duration,
        }
    }
}

/// 性能记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceRecord {
    pub name: String,
    pub duration_ms: u64,
}

/// 性能分析器
pub struct Profiler {
    records: Vec<PerformanceRecord>,
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Profiler {
    /// 创建新的分析器
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// 添加记录
    pub fn add_record(&mut self, record: PerformanceRecord) {
        self.records.push(record);
    }

    /// 获取所有记录
    pub fn get_records(&self) -> &[PerformanceRecord] {
        &self.records
    }

    /// 获取平均执行时间
    pub fn average_duration(&self) -> u64 {
        if self.records.is_empty() {
            return 0;
        }
        self.records.iter().map(|r| r.duration_ms).sum::<u64>() / self.records.len() as u64
    }

    /// 获取最长执行时间
    pub fn max_duration(&self) -> u64 {
        self.records
            .iter()
            .map(|r| r.duration_ms)
            .max()
            .unwrap_or(0)
    }

    /// 获取最短执行时间
    pub fn min_duration(&self) -> u64 {
        self.records
            .iter()
            .map(|r| r.duration_ms)
            .min()
            .unwrap_or(0)
    }

    /// 生成报告
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("Performance Report\n");
        report.push_str("===================\n\n");

        for record in &self.records {
            report.push_str(&format!("{}: {}ms\n", record.name, record.duration_ms));
        }

        report.push_str("\n");
        report.push_str(&format!("Average: {}ms\n", self.average_duration()));
        report.push_str(&format!("Max: {}ms\n", self.max_duration()));
        report.push_str(&format!("Min: {}ms\n", self.min_duration()));

        report
    }

    /// 清除所有记录
    pub fn clear(&mut self) {
        self.records.clear();
    }
}

/// 线程安全的性能分析器
pub struct ThreadSafeProfiler {
    inner: std::sync::Mutex<Profiler>,
}

impl Default for ThreadSafeProfiler {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadSafeProfiler {
    /// 创建新的线程安全分析器
    pub fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(Profiler::new()),
        }
    }

    /// 添加记录
    pub fn add_record(&self, record: PerformanceRecord) {
        if let Ok(mut profiler) = self.inner.lock() {
            profiler.add_record(record);
        }
    }

    /// 获取报告
    pub fn generate_report(&self) -> String {
        if let Ok(profiler) = self.inner.lock() {
            profiler.generate_report()
        } else {
            "Failed to lock profiler".to_string()
        }
    }
}

lazy_static::lazy_static! {
    /// 全局性能分析器
    static ref GLOBAL_PROFILER: ThreadSafeProfiler = ThreadSafeProfiler::new();
}

/// 获取全局性能分析器
pub fn global_profiler() -> &'static ThreadSafeProfiler {
    &GLOBAL_PROFILER
}

/// 便利宏：测量代码块性能
#[macro_export]
macro_rules! measure_time {
    ($name:expr, $body:expr) => {{
        let timer = $crate::debug::performance::PerformanceTimer::new($name);
        let result = $body;
        let record = timer.finish();
        $crate::debug::performance::global_profiler().add_record(record);
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_timer() {
        let timer = PerformanceTimer::new("test_operation");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let record = timer.finish();
        assert!(record.duration_ms >= 10);
        assert_eq!(record.name, "test_operation");
    }

    #[test]
    fn test_profiler() {
        let mut profiler = Profiler::new();
        profiler.add_record(PerformanceRecord {
            name: "op1".to_string(),
            duration_ms: 100,
        });
        profiler.add_record(PerformanceRecord {
            name: "op2".to_string(),
            duration_ms: 200,
        });

        assert_eq!(profiler.average_duration(), 150);
        assert_eq!(profiler.max_duration(), 200);
        assert_eq!(profiler.min_duration(), 100);
    }

    #[test]
    fn test_performance_sampler() {
        let mut sampler = PerformanceSampler::new(10, 0); // 0间隔确保每次都会采样
        let metrics = sampler.try_sample().unwrap();
        assert!(metrics.is_some());
    }
}
