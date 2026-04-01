//! WebGL 终端渲染器
//! 提供高性能GPU加速的终端渲染

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// WebGL渲染器配置
#[derive(Debug, Clone)]
pub struct WebGlConfig {
    /// 是否启用WebGL加速
    pub enabled: bool,
    /// 纹理图集大小
    pub atlas_size: u32,
    /// 最大同时渲染单元格数
    pub max_cells: usize,
    /// 批处理字符阈值
    pub batch_threshold: usize,
    /// 是否启用伽马校正
    pub gamma_correction: bool,
    /// 字体渲染模式
    pub font_render_mode: FontRenderMode,
    /// 抗锯齿级别
    pub antialias_level: AntialiasLevel,
    /// 是否使用半浮点纹理（用于HDR颜色）
    pub use_half_float: bool,
    /// 垂直同步
    pub vsync: bool,
    /// 目标帧率 (0 = 无限制)
    pub target_fps: u32,
    /// 渲染缩放比例
    pub device_pixel_ratio: f64,
}

impl Default for WebGlConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            atlas_size: 4096,
            max_cells: 100_000,
            batch_threshold: 256,
            gamma_correction: true,
            font_render_mode: FontRenderMode::Sdf,
            antialias_level: AntialiasLevel::MSAA4x,
            use_half_float: true,
            vsync: true,
            target_fps: 60,
            device_pixel_ratio: 1.0,
        }
    }
}

impl WebGlConfig {
    /// 创建高性能配置
    pub fn high_performance() -> Self {
        Self {
            enabled: true,
            atlas_size: 8192,
            max_cells: 200_000,
            batch_threshold: 512,
            gamma_correction: true,
            font_render_mode: FontRenderMode::SdfSubpixel,
            antialias_level: AntialiasLevel::MSAA8x,
            use_half_float: true,
            vsync: true,
            target_fps: 144,
            device_pixel_ratio: 2.0,
        }
    }

    /// 创建节能配置
    pub fn power_saving() -> Self {
        Self {
            enabled: false, // 使用Canvas 2D
            atlas_size: 2048,
            max_cells: 50_000,
            batch_threshold: 128,
            gamma_correction: false,
            font_render_mode: FontRenderMode::Bitmap,
            antialias_level: AntialiasLevel::None,
            use_half_float: false,
            vsync: true,
            target_fps: 30,
            device_pixel_ratio: 1.0,
        }
    }
}

/// 字体渲染模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontRenderMode {
    /// 位图渲染（最快，质量最低）
    Bitmap,
    /// 灰度抗锯齿
    Gray,
    /// SDF (Signed Distance Field) 渲染
    Sdf,
    /// SDF + 亚像素渲染
    SdfSubpixel,
    /// LCD子像素渲染
    Lcd,
}

/// 抗锯齿级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntialiasLevel {
    /// 无抗锯齿
    None,
    /// 2x MSAA
    MSAA2x,
    /// 4x MSAA
    MSAA4x,
    /// 8x MSAA
    MSAA8x,
}

/// 渲染统计
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
    /// 当前FPS
    pub fps: f32,
    /// 平均帧时间 (ms)
    pub avg_frame_time_ms: f32,
    /// 最后帧时间 (ms)
    pub last_frame_time_ms: f32,
    /// 总渲染帧数
    pub total_frames: u64,
    /// 丢帧数
    pub dropped_frames: u64,
    /// GPU内存使用 (MB)
    pub gpu_memory_mb: f32,
    /// 批处理次数
    pub batch_count: u32,
    /// 绘制调用次数
    pub draw_calls: u32,
    /// 上传纹理次数
    pub texture_uploads: u64,
    /// 渲染单元格数
    pub cells_rendered: u64,
    /// 缓存命中率
    pub cache_hit_rate: f32,
}

/// 单元格数据（用于GPU批处理）
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CellData {
    /// X坐标（单元格列）
    pub x: u16,
    /// Y坐标（单元格行）
    pub y: u16,
    /// 字符代码
    pub char_code: u32,
    /// 前景色 (RGBA)
    pub fg_color: u32,
    /// 背景色 (RGBA)
    pub bg_color: u32,
    /// 样式标志
    pub style: u16,
}

/// 样式标志
pub mod cell_style {
    pub const BOLD: u16 = 1 << 0;
    pub const ITALIC: u16 = 1 << 1;
    pub const UNDERLINE: u16 = 1 << 2;
    pub const STRIKETHROUGH: u16 = 1 << 3;
    pub const BLINK: u16 = 1 << 4;
    pub const INVERSE: u16 = 1 << 5;
    pub const INVISIBLE: u16 = 1 << 6;
    pub const CURSOR: u16 = 1 << 7;
}

/// 字符纹理图集
pub struct CharAtlas {
    /// 图集纹理ID
    pub texture_id: u32,
    /// 图集大小
    pub size: u32,
    /// 字符位置映射
    pub char_map: std::collections::HashMap<char, AtlasCharInfo>,
    /// 下一个可用位置
    pub next_x: u32,
    pub next_y: u32,
    /// 行高
    pub row_height: u32,
}

/// 图集中的字符信息
#[derive(Debug, Clone, Copy)]
pub struct AtlasCharInfo {
    pub u: f32,
    pub v: f32,
    pub width: f32,
    pub height: f32,
    pub advance: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
}

/// WebGL渲染器
pub struct WebGlRenderer {
    config: WebGlConfig,
    stats: Arc<RwLock<RenderStats>>,
    cell_buffer: Vec<CellData>,
    render_queue: VecDeque<RenderCommand>,
    last_frame_time: Instant,
    frame_times: VecDeque<Duration>,
    atlas: Option<CharAtlas>,
}

/// 渲染命令
#[derive(Debug, Clone)]
pub enum RenderCommand {
    /// 清屏
    Clear { color: u32 },
    /// 渲染单元格
    RenderCells { cells: Vec<CellData> },
    /// 渲染光标
    RenderCursor { x: u16, y: u16, style: CursorStyle },
    /// 滚动区域
    Scroll { y_offset: i16, region: ScrollRegion },
    /// 更新纹理
    UpdateAtlas { chars: Vec<(char, Vec<u8>)> },
    /// 交换缓冲区
    SwapBuffers,
}

/// 光标样式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

/// 滚动区域
#[derive(Debug, Clone, Copy)]
pub struct ScrollRegion {
    pub top: u16,
    pub bottom: u16,
    pub left: u16,
    pub right: u16,
}

impl WebGlRenderer {
    /// 创建新的WebGL渲染器
    pub fn new(config: WebGlConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(RenderStats::default())),
            cell_buffer: Vec::with_capacity(1024),
            render_queue: VecDeque::new(),
            last_frame_time: Instant::now(),
            frame_times: VecDeque::with_capacity(60),
            atlas: None,
        }
    }

    /// 初始化WebGL上下文
    pub fn initialize(&mut self) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        log::info!("Initializing WebGL renderer with config: {:?}", self.config);

        // 这里会初始化实际的WebGL上下文
        // 在实际实现中，这需要与前端JavaScript交互

        // 初始化字符图集
        self.atlas = Some(CharAtlas {
            texture_id: 0,
            size: self.config.atlas_size,
            char_map: std::collections::HashMap::new(),
            next_x: 0,
            next_y: 0,
            row_height: 0,
        });

        Ok(())
    }

    /// 添加单元格到渲染缓冲区
    pub fn add_cell(&mut self, x: u16, y: u16, char_code: char, fg: u32, bg: u32, style: u16) {
        if self.cell_buffer.len() >= self.config.batch_threshold {
            self.flush_cells();
        }

        self.cell_buffer.push(CellData {
            x,
            y,
            char_code: char_code as u32,
            fg_color: fg,
            bg_color: bg,
            style,
        });
    }

    /// 刷新单元格缓冲区到渲染队列
    pub fn flush_cells(&mut self) {
        if self.cell_buffer.is_empty() {
            return;
        }

        let cells = std::mem::take(&mut self.cell_buffer);
        self.cell_buffer.reserve(self.config.batch_threshold);

        self.render_queue
            .push_back(RenderCommand::RenderCells { cells });
    }

    /// 提交清屏命令
    pub fn clear(&mut self, color: u32) {
        self.render_queue.push_back(RenderCommand::Clear { color });
    }

    /// 提交光标渲染命令
    pub fn render_cursor(&mut self, x: u16, y: u16, style: CursorStyle) {
        self.render_queue
            .push_back(RenderCommand::RenderCursor { x, y, style });
    }

    /// 执行渲染帧
    pub async fn render_frame(&mut self) -> Result<(), String> {
        let frame_start = Instant::now();

        // 刷新剩余的单元格
        self.flush_cells();

        // 添加交换缓冲区命令
        self.render_queue.push_back(RenderCommand::SwapBuffers);

        // 处理渲染队列
        while let Some(cmd) = self.render_queue.pop_front() {
            self.execute_command(cmd).await?;
        }

        // 计算帧统计
        let frame_time = frame_start.elapsed();
        let frame_time_ms = frame_time.as_secs_f32() * 1000.0;

        self.frame_times.push_back(frame_time);
        if self.frame_times.len() > 60 {
            self.frame_times.pop_front();
        }

        let avg_frame_time: Duration = self.frame_times.iter().sum();
        let avg_frame_time_ms =
            avg_frame_time.as_secs_f32() * 1000.0 / self.frame_times.len() as f32;

        let fps = 1000.0 / avg_frame_time_ms;

        // 检查是否丢帧
        let target_frame_time = 1000.0 / self.config.target_fps as f32;
        let dropped = frame_time_ms > target_frame_time * 1.5;

        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.fps = fps;
            stats.avg_frame_time_ms = avg_frame_time_ms;
            stats.last_frame_time_ms = frame_time_ms;
            stats.total_frames += 1;
            if dropped {
                stats.dropped_frames += 1;
            }
        }

        self.last_frame_time = frame_start;

        // 帧率限制
        if self.config.target_fps > 0 && self.config.vsync {
            let target_duration = Duration::from_secs_f32(1.0 / self.config.target_fps as f32);
            if frame_time < target_duration {
                tokio::time::sleep(target_duration - frame_time).await;
            }
        }

        Ok(())
    }

    /// 执行单个渲染命令
    async fn execute_command(&mut self, cmd: RenderCommand) -> Result<(), String> {
        match cmd {
            RenderCommand::Clear { color } => {
                // WebGL清屏操作
                log::debug!("WebGL Clear: color={:08x}", color);
            }
            RenderCommand::RenderCells { cells } => {
                // 批量渲染单元格
                let batch_count =
                    (cells.len() + self.config.batch_threshold - 1) / self.config.batch_threshold;

                {
                    let mut stats = self.stats.write().await;
                    stats.batch_count += batch_count as u32;
                    stats.draw_calls += 1;
                    stats.cells_rendered += cells.len() as u64;
                }

                log::debug!(
                    "WebGL Render {} cells in {} batches",
                    cells.len(),
                    batch_count
                );
            }
            RenderCommand::RenderCursor { x, y, style } => {
                log::debug!("WebGL Render cursor at ({}, {}), style={:?}", x, y, style);
            }
            RenderCommand::Scroll { y_offset, region } => {
                log::debug!(
                    "WebGL Scroll: offset={}, region=[{}, {}, {}, {}]",
                    y_offset,
                    region.top,
                    region.bottom,
                    region.left,
                    region.right
                );
            }
            RenderCommand::UpdateAtlas { chars } => {
                {
                    let mut stats = self.stats.write().await;
                    stats.texture_uploads += 1;
                }
                log::debug!("WebGL Update atlas: {} chars", chars.len());
            }
            RenderCommand::SwapBuffers => {
                log::debug!("WebGL Swap buffers");
            }
        }

        Ok(())
    }

    /// 获取当前渲染统计
    pub async fn get_stats(&self) -> RenderStats {
        self.stats.read().await.clone()
    }

    /// 检查WebGL是否可用
    pub fn is_available(&self) -> bool {
        self.config.enabled
    }

    /// 更新配置
    pub fn update_config(&mut self, config: WebGlConfig) {
        log::info!("Updating WebGL config: {:?}", config);
        self.config = config;
    }

    /// 调整图集大小
    pub fn resize_atlas(&mut self, new_size: u32) -> Result<(), String> {
        if let Some(ref mut atlas) = self.atlas {
            if new_size > atlas.size {
                log::info!("Resizing atlas from {} to {}", atlas.size, new_size);
                atlas.size = new_size;
                // 这里会重新创建WebGL纹理
            }
        }
        Ok(())
    }

    /// 预热常用字符
    pub fn warm_up_chars(&mut self, chars: &str) {
        log::debug!("Warming up {} characters", chars.len());
        // 将常用字符添加到图集
    }

    /// 强制同步
    pub async fn sync(&self) {
        // 等待所有GPU操作完成
        tokio::task::yield_now().await;
    }

    /// 清理资源
    pub async fn cleanup(&mut self) {
        log::info!("Cleaning up WebGL renderer");
        self.atlas = None;
        self.cell_buffer.clear();
        self.render_queue.clear();
    }
}

impl Default for WebGlRenderer {
    fn default() -> Self {
        Self::new(WebGlConfig::default())
    }
}

/// GPU性能监控器
pub struct GpuMonitor {
    stats_history: VecDeque<RenderStats>,
    max_history: usize,
    alert_threshold: f32,
}

impl GpuMonitor {
    pub fn new(max_history: usize, alert_threshold: f32) -> Self {
        Self {
            stats_history: VecDeque::with_capacity(max_history),
            max_history,
            alert_threshold,
        }
    }

    /// 记录新的统计
    pub fn record(&mut self, stats: &RenderStats) {
        self.stats_history.push_back(stats.clone());
        if self.stats_history.len() > self.max_history {
            self.stats_history.pop_front();
        }

        // 检查性能告警
        if stats.fps < self.alert_threshold {
            log::warn!("Low FPS detected: {:.1} FPS", stats.fps);
        }

        if stats.avg_frame_time_ms > 16.67 {
            // 60fps threshold
            log::warn!("High frame time: {:.2}ms", stats.avg_frame_time_ms);
        }
    }

    /// 获取平均FPS
    pub fn average_fps(&self) -> f32 {
        if self.stats_history.is_empty() {
            return 0.0;
        }

        let sum: f32 = self.stats_history.iter().map(|s| s.fps).sum();
        sum / self.stats_history.len() as f32
    }

    /// 获取性能趋势
    pub fn performance_trend(&self) -> PerformanceTrend {
        if self.stats_history.len() < 10 {
            return PerformanceTrend::Stable;
        }

        let recent: Vec<_> = self.stats_history.iter().rev().take(10).collect();
        let older: Vec<_> = self.stats_history.iter().rev().skip(10).take(10).collect();

        let recent_avg: f32 = recent.iter().map(|s| s.fps).sum::<f32>() / recent.len() as f32;
        let older_avg: f32 = older.iter().map(|s| s.fps).sum::<f32>() / older.len() as f32;

        let change = (recent_avg - older_avg) / older_avg;

        if change > 0.1 {
            PerformanceTrend::Improving
        } else if change < -0.1 {
            PerformanceTrend::Degrading
        } else {
            PerformanceTrend::Stable
        }
    }
}

/// 性能趋势
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceTrend {
    Improving,
    Stable,
    Degrading,
}

/// 渲染优化建议
pub struct RenderOptimizer;

impl RenderOptimizer {
    /// 根据当前状态优化配置
    pub fn optimize_config(
        current_stats: &RenderStats,
        current_config: &WebGlConfig,
    ) -> WebGlConfig {
        let mut new_config = current_config.clone();

        // 如果FPS持续低于目标，降低质量
        if current_stats.fps < current_config.target_fps as f32 * 0.5 {
            log::warn!("FPS too low, reducing quality settings");

            // 降级抗锯齿
            new_config.antialias_level = match current_config.antialias_level {
                AntialiasLevel::MSAA8x => AntialiasLevel::MSAA4x,
                AntialiasLevel::MSAA4x => AntialiasLevel::MSAA2x,
                AntialiasLevel::MSAA2x => AntialiasLevel::None,
                AntialiasLevel::None => AntialiasLevel::None,
            };

            // 降级字体渲染
            new_config.font_render_mode = match current_config.font_render_mode {
                FontRenderMode::SdfSubpixel => FontRenderMode::Sdf,
                FontRenderMode::Sdf => FontRenderMode::Gray,
                FontRenderMode::Gray => FontRenderMode::Bitmap,
                FontRenderMode::Bitmap => FontRenderMode::Bitmap,
                FontRenderMode::Lcd => FontRenderMode::Gray,
            };

            // 降低批处理阈值
            new_config.batch_threshold = current_config.batch_threshold / 2;
        }

        // 如果GPU内存使用过高，减小图集
        if current_stats.gpu_memory_mb > 512.0 {
            new_config.atlas_size = current_config.atlas_size / 2;
        }

        new_config
    }

    /// 检测性能瓶颈
    pub fn detect_bottleneck(stats: &RenderStats) -> Option<Bottleneck> {
        if stats.draw_calls > 100 {
            return Some(Bottleneck::TooManyDrawCalls);
        }

        if stats.texture_uploads > 10 {
            return Some(Bottleneck::TextureUpload);
        }

        if stats.batch_count > 50 {
            return Some(Bottleneck::InefficientBatching);
        }

        if stats.cache_hit_rate < 0.8 {
            return Some(Bottleneck::LowCacheHit);
        }

        None
    }
}

/// 性能瓶颈类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bottleneck {
    TooManyDrawCalls,
    TextureUpload,
    InefficientBatching,
    LowCacheHit,
}
