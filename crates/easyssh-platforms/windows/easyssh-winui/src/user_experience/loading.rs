//! Loading State Management for EasySSH
//!
//! Provides comprehensive loading state tracking and visual feedback
//! for all async operations.

use egui::{Frame, ProgressBar as EguiProgressBar, RichText, Rounding, Stroke, Ui, Vec2};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Unique identifier for a loading operation
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LoadingOperation {
    Connect(String),        // server_id
    Disconnect(String),     // session_id
    SFTPRefresh(String),    // session_id
    SFTPUpload(String),     // session_id + file
    SFTPDownload(String),   // session_id + file
    MonitorRefresh(String), // session_id
    ServerListRefresh,
    ConfigImport,
    ConfigExport,
    FileEdit(String), // file path
    Search,
    Custom(String), // any custom operation
}

impl LoadingOperation {
    pub fn display_name(&self) -> String {
        match self {
            LoadingOperation::Connect(_) => "连接服务器".to_string(),
            LoadingOperation::Disconnect(_) => "断开连接".to_string(),
            LoadingOperation::SFTPRefresh(_) => "刷新文件列表".to_string(),
            LoadingOperation::SFTPUpload(_) => "上传文件".to_string(),
            LoadingOperation::SFTPDownload(_) => "下载文件".to_string(),
            LoadingOperation::MonitorRefresh(_) => "刷新监控数据".to_string(),
            LoadingOperation::ServerListRefresh => "刷新服务器列表".to_string(),
            LoadingOperation::ConfigImport => "导入配置".to_string(),
            LoadingOperation::ConfigExport => "导出配置".to_string(),
            LoadingOperation::FileEdit(_) => "编辑文件".to_string(),
            LoadingOperation::Search => "搜索".to_string(),
            LoadingOperation::Custom(name) => name.clone(),
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LoadingOperation::Connect(_) => "→",
            LoadingOperation::Disconnect(_) => "■",
            LoadingOperation::SFTPRefresh(_) | LoadingOperation::MonitorRefresh(_) => "⟳",
            LoadingOperation::SFTPUpload(_) => "↑",
            LoadingOperation::SFTPDownload(_) => "↓",
            LoadingOperation::ServerListRefresh => "☰",
            LoadingOperation::ConfigImport | LoadingOperation::ConfigExport => "💾",
            LoadingOperation::FileEdit(_) => "✎",
            LoadingOperation::Search => "⌕",
            LoadingOperation::Custom(_) => "⏳",
        }
    }
}

/// State of a loading operation
#[derive(Clone, Debug)]
pub struct LoadingState {
    pub operation: LoadingOperation,
    pub started_at: Instant,
    pub progress: Option<f32>, // 0.0 to 1.0, None for indeterminate
    pub message: Option<String>,
    pub cancellable: bool,
    pub stage: LoadingStage,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LoadingStage {
    Starting,
    InProgress,
    Cancelling,
    Completed,
    Failed(String),
}

impl LoadingState {
    pub fn new(operation: LoadingOperation, cancellable: bool) -> Self {
        Self {
            operation,
            started_at: Instant::now(),
            progress: None,
            message: None,
            cancellable,
            stage: LoadingStage::Starting,
        }
    }

    pub fn with_progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress.clamp(0.0, 1.0));
        self.stage = LoadingStage::InProgress;
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn is_long_running(&self) -> bool {
        self.elapsed() > Duration::from_secs(3)
    }

    pub fn is_stalled(&self) -> bool {
        self.elapsed() > Duration::from_secs(30)
    }
}

/// Manager for all loading states
pub struct LoadingStateManager {
    states: HashMap<LoadingOperation, LoadingState>,
    max_concurrent: usize,
}

impl Default for LoadingStateManager {
    fn default() -> Self {
        Self {
            states: HashMap::new(),
            max_concurrent: 5,
        }
    }
}

impl LoadingStateManager {
    /// Start a new loading operation
    pub fn start(&mut self, operation: LoadingOperation, cancellable: bool) -> bool {
        if self.states.len() >= self.max_concurrent {
            return false;
        }

        let state = LoadingState::new(operation.clone(), cancellable);
        self.states.insert(operation, state);
        true
    }

    /// Update progress of an operation
    pub fn update_progress(&mut self, operation: &LoadingOperation, progress: f32) {
        if let Some(state) = self.states.get_mut(operation) {
            state.progress = Some(progress.clamp(0.0, 1.0));
            state.stage = LoadingStage::InProgress;
        }
    }

    /// Update message of an operation
    pub fn update_message(&mut self, operation: &LoadingOperation, message: impl Into<String>) {
        if let Some(state) = self.states.get_mut(operation) {
            state.message = Some(message.into());
        }
    }

    /// Mark operation as completed
    pub fn complete(&mut self, operation: &LoadingOperation) {
        if let Some(state) = self.states.get_mut(operation) {
            state.progress = Some(1.0);
            state.stage = LoadingStage::Completed;
        }
    }

    /// Mark operation as failed
    pub fn fail(&mut self, operation: &LoadingOperation, error: impl Into<String>) {
        if let Some(state) = self.states.get_mut(operation) {
            state.stage = LoadingStage::Failed(error.into());
        }
    }

    /// Cancel an operation
    pub fn cancel(&mut self, operation: &LoadingOperation) {
        if let Some(state) = self.states.get_mut(operation) {
            if state.cancellable {
                state.stage = LoadingStage::Cancelling;
            }
        }
    }

    /// Remove a completed/failed/cancelled operation
    pub fn remove(&mut self, operation: &LoadingOperation) {
        self.states.remove(operation);
    }

    /// Check if an operation is loading
    pub fn is_loading(&self, operation: &LoadingOperation) -> bool {
        self.states.contains_key(operation)
    }

    /// Get state of an operation
    pub fn get_state(&self, operation: &LoadingOperation) -> Option<&LoadingState> {
        self.states.get(operation)
    }

    /// Get all active loading operations
    pub fn get_active(&self) -> Vec<&LoadingState> {
        self.states
            .values()
            .filter(|s| {
                matches!(
                    s.stage,
                    LoadingStage::Starting | LoadingStage::InProgress | LoadingStage::Cancelling
                )
            })
            .collect()
    }

    /// Check if any operation is loading
    pub fn is_any_loading(&self) -> bool {
        !self.get_active().is_empty()
    }

    /// Get number of active operations
    pub fn active_count(&self) -> usize {
        self.get_active().len()
    }

    /// Update animation frame
    pub fn update(&mut self, _frame: u64) {
        // Clean up completed operations after a delay
        let to_remove: Vec<_> = self
            .states
            .iter()
            .filter(|(_, s)| matches!(s.stage, LoadingStage::Completed | LoadingStage::Failed(_)))
            .filter(|(_, s)| s.elapsed() > Duration::from_secs(2))
            .map(|(k, _)| k.clone())
            .collect();

        for op in to_remove {
            self.states.remove(&op);
        }
    }

    /// Render loading overlay for a specific operation
    pub fn render_overlay(
        &self,
        ui: &mut Ui,
        theme: &crate::design::DesignTheme,
        operation: &LoadingOperation,
        frame: u64,
    ) {
        if let Some(state) = self.states.get(operation) {
            self.render_state(ui, theme, state, frame);
        }
    }

    /// Render all active loading states as a panel
    pub fn render_panel(&self, ui: &mut Ui, theme: &crate::design::DesignTheme, frame: u64) {
        let active = self.get_active();
        if active.is_empty() {
            return;
        }

        Frame::group(ui.style())
            .fill(theme.bg_elevated)
            .rounding(Rounding::same(8.0))
            .stroke(Stroke::new(1.0, theme.border_default))
            .show(ui, |ui| {
                ui.set_min_width(300.0);

                // Header
                ui.horizontal(|ui| {
                    let spinner = crate::user_experience::LoadingSpinner::new(16.0);
                    spinner.render(ui, frame);

                    let count = active.len();
                    let header_text = if count == 1 {
                        active[0].operation.display_name()
                    } else {
                        format!("{} 个任务进行中", count)
                    };

                    ui.label(
                        RichText::new(header_text)
                            .size(14.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                });

                ui.separator();

                // Individual operations
                for state in &active {
                    self.render_compact_state(ui, theme, state, frame);
                }
            });
    }

    /// Render a full loading state
    fn render_state(
        &self,
        ui: &mut Ui,
        theme: &crate::design::DesignTheme,
        state: &LoadingState,
        frame: u64,
    ) {
        ui.vertical_centered(|ui| {
            ui.add_space(48.0);

            // Large spinner
            let spinner = crate::user_experience::LoadingSpinner::new(48.0)
                .with_color(theme.interactive_primary);
            spinner.render(ui, frame);

            ui.add_space(24.0);

            // Icon + Operation name
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(state.operation.icon())
                        .size(24.0)
                        .color(theme.interactive_primary),
                );
                ui.label(
                    RichText::new(state.operation.display_name())
                        .size(18.0)
                        .strong()
                        .color(theme.text_primary),
                );
            });

            ui.add_space(8.0);

            // Stage message
            let stage_text = match &state.stage {
                LoadingStage::Starting => "正在启动...",
                LoadingStage::InProgress => state.message.as_deref().unwrap_or("处理中..."),
                LoadingStage::Cancelling => "正在取消...",
                LoadingStage::Completed => "已完成",
                LoadingStage::Failed(e) => e,
            };

            ui.label(
                RichText::new(stage_text)
                    .size(14.0)
                    .color(theme.text_secondary),
            );

            ui.add_space(16.0);

            // Progress bar
            if let Some(progress) = state.progress {
                let progress_bar = ui.add(
                    EguiProgressBar::new(progress)
                        .desired_width(300.0)
                        .text(format!("{:.0}%", progress * 100.0)),
                );

                // Color based on progress
                if progress >= 1.0 {
                    progress_bar.on_hover_text("完成!");
                }
            } else {
                // Indeterminate progress bar
                let progress = (frame as f32 * 0.02) % 1.0;
                ui.add(
                    EguiProgressBar::new(progress)
                        .desired_width(300.0)
                        .text("加载中..."),
                );
            }

            ui.add_space(8.0);

            // Elapsed time
            let elapsed = state.elapsed();
            ui.label(
                RichText::new(format!(
                    "已用时: {}",
                    crate::user_experience::format_duration(elapsed)
                ))
                .size(12.0)
                .color(theme.text_tertiary),
            );

            // Warning for long-running operations
            if state.is_stalled() {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("⚠ 操作时间过长，可能需要重试")
                        .size(12.0)
                        .color(crate::design::SemanticColors::WARNING),
                );
            }

            ui.add_space(24.0);

            // Cancel button
            if state.cancellable
                && matches!(
                    state.stage,
                    LoadingStage::Starting | LoadingStage::InProgress
                )
                && ui.button("取消操作").clicked()
            {
                // Cancel logic handled by caller
            }
        });
    }

    /// Render a compact loading state for the panel
    fn render_compact_state(
        &self,
        ui: &mut Ui,
        theme: &crate::design::DesignTheme,
        state: &LoadingState,
        frame: u64,
    ) {
        ui.horizontal(|ui| {
            // Small spinner or icon
            let spinner = crate::user_experience::LoadingSpinner::new(16.0);
            spinner.render(ui, frame);

            ui.vertical(|ui| {
                // Operation name
                ui.label(
                    RichText::new(state.operation.display_name())
                        .size(13.0)
                        .strong()
                        .color(theme.text_primary),
                );

                // Progress or message
                if let Some(progress) = state.progress {
                    ui.add(
                        EguiProgressBar::new(progress)
                            .desired_width(200.0)
                            .show_percentage(),
                    );
                } else if let Some(message) = &state.message {
                    ui.label(
                        RichText::new(message)
                            .size(11.0)
                            .color(theme.text_secondary),
                    );
                }
            });

            // Time and cancel button
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if state.cancellable
                    && matches!(
                        state.stage,
                        LoadingStage::Starting | LoadingStage::InProgress
                    )
                    && ui.small_button("×").on_hover_text("取消").clicked()
                {
                    // Cancel logic handled by caller
                }

                ui.label(
                    RichText::new(crate::user_experience::format_duration(state.elapsed()))
                        .size(11.0)
                        .color(theme.text_tertiary),
                );
            });
        });

        ui.add_space(8.0);
    }
}

/// Button that shows loading state
pub struct LoadingButton {
    label: String,
    is_loading: bool,
    loading_text: Option<String>,
    enabled: bool,
}

impl LoadingButton {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            is_loading: false,
            loading_text: None,
            enabled: true,
        }
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.is_loading = loading;
        self
    }

    pub fn loading_text(mut self, text: impl Into<String>) -> Self {
        self.loading_text = Some(text.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn render(
        &self,
        ui: &mut Ui,
        theme: &crate::design::DesignTheme,
        frame: u64,
    ) -> egui::Response {
        let button_text = if self.is_loading {
            self.loading_text
                .clone()
                .unwrap_or_else(|| format!("{}...", self.label))
        } else {
            self.label.clone()
        };

        let button = if self.is_loading {
            ui.add_sized(
                Vec2::new(120.0, 36.0),
                egui::Button::new(button_text)
                    .fill(theme.bg_tertiary)
                    .sense(egui::Sense::hover()), // Non-interactive while loading
            )
        } else {
            ui.add_sized(
                Vec2::new(120.0, 36.0),
                crate::design::AccessibleButton::new(theme, button_text)
                    .style(crate::design::AccessibleButtonStyle::Primary)
                    .build(),
            )
        };

        // Show spinner overlay if loading
        if self.is_loading {
            let spinner = crate::user_experience::LoadingSpinner::new(16.0);
            let spinner_rect = button.rect.shrink(8.0);
            let spinner_pos = spinner_rect.left_center() - Vec2::new(8.0, 0.0);

            ui.allocate_ui_at_rect(
                egui::Rect::from_center_size(spinner_pos, Vec2::splat(16.0)),
                |ui| {
                    spinner.render(ui, frame);
                },
            );
        }

        button
    }
}

/// Inline loading indicator for use within forms
pub struct InlineLoading {
    message: String,
    show_spinner: bool,
}

impl InlineLoading {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            show_spinner: true,
        }
    }

    pub fn render(&self, ui: &mut Ui, theme: &crate::design::DesignTheme, frame: u64) {
        ui.horizontal(|ui| {
            if self.show_spinner {
                let spinner = crate::user_experience::LoadingSpinner::new(16.0);
                spinner.render(ui, frame);
                ui.add_space(8.0);
            }

            ui.label(
                RichText::new(&self.message)
                    .size(13.0)
                    .color(theme.text_secondary),
            );
        });
    }
}
