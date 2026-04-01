//! User Experience Optimization Module for EasySSH
//!
//! This module provides comprehensive UX improvements including:
//! - Loading indicators with progress tracking
//! - User-friendly error messages with recovery actions
//! - First-time user onboarding
//! - Toast notifications
//! - Virtual scrolling for large lists
//! - Debounced search inputs
//!
//! @version 1.0.0

use std::collections::VecDeque;
use std::time::{Duration, Instant};

// Declare submodules
pub mod loading;
pub mod errors;
pub mod onboarding;
pub mod responsiveness;

// Re-export main types for convenience
pub use loading::{
    LoadingOperation, LoadingStateManager,
};


pub use onboarding::{
    OnboardingWizard, OnboardingAction,
    QuickTip,
};


// ============================================================================
// GLOBAL UX STATE MANAGER
// ============================================================================

/// Global UX state manager
pub struct UXManager {
    /// Loading states for various operations
    pub loading_states: LoadingStateManager,
    /// Error message queue
    pub error_queue: errors::ErrorQueue,
    /// Onboarding state
    pub onboarding: onboarding::OnboardingState,
    /// Toast notifications
    pub toasts: VecDeque<ToastNotification>,
    /// Animation frame counter for smooth animations
    pub animation_frame: u64,
    /// Last frame time for performance monitoring
    pub last_frame_time: Instant,
}

impl Default for UXManager {
    fn default() -> Self {
        Self {
            loading_states: LoadingStateManager::default(),
            error_queue: errors::ErrorQueue::default(),
            onboarding: onboarding::OnboardingState::default(),
            toasts: VecDeque::new(),
            animation_frame: 0,
            last_frame_time: Instant::now(),
        }
    }
}

impl UXManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update animation frame
    pub fn update(&mut self) {
        self.animation_frame = self.animation_frame.wrapping_add(1);
        self.loading_states.update(self.animation_frame);
        self.toasts.retain(|t| !t.is_expired());
    }

    /// Show a toast notification
    pub fn show_toast(&mut self, toast: ToastNotification) {
        if self.toasts.len() > 5 {
            self.toasts.pop_front();
        }
        self.toasts.push_back(toast);
    }

    /// Render all active toasts
    pub fn render_toasts(&mut self, ctx: &egui::Context, theme: &crate::design::DesignTheme) {
        let screen_rect = ctx.screen_rect();
        let toast_width = 320.0;
        let toast_height = 64.0;
        let margin = 16.0;
        let gap = 8.0;

        let start_x = screen_rect.max.x - toast_width - margin;
        let mut current_y = screen_rect.max.y - margin - toast_height;

        let toasts_to_render: Vec<_> = self.toasts.iter().cloned().collect();

        for toast in toasts_to_render.iter().rev() {
            let toast_rect = egui::Rect::from_min_size(
                egui::pos2(start_x, current_y),
                egui::vec2(toast_width, toast_height),
            );

            egui::Area::new(egui::Id::new(format!("toast_{}", toast.id)))
                .fixed_pos(toast_rect.min)
                .show(ctx, |ui| {
                    toast.render(ui, theme, toast_rect.size());
                });

            current_y -= toast_height + gap;
        }
    }
}

// ============================================================================
// TOAST NOTIFICATIONS
// ============================================================================

/// Toast notification for transient messages
#[derive(Clone)]
pub struct ToastNotification {
    pub id: String,
    pub title: String,
    pub message: String,
    pub toast_type: ToastType,
    pub created_at: Instant,
    pub duration: Duration,
    pub action: Option<ToastAction>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

#[derive(Clone)]
pub struct ToastAction {
    pub label: String,
    pub callback: std::sync::Arc<dyn Fn() + Send + Sync>,
}

impl ToastNotification {
    pub fn success(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            message: message.into(),
            toast_type: ToastType::Success,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
            action: None,
        }
    }

    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            message: message.into(),
            toast_type: ToastType::Error,
            created_at: Instant::now(),
            duration: Duration::from_secs(5),
            action: None,
        }
    }

    pub fn warning(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            message: message.into(),
            toast_type: ToastType::Warning,
            created_at: Instant::now(),
            duration: Duration::from_secs(4),
            action: None,
        }
    }

    pub fn info(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            message: message.into(),
            toast_type: ToastType::Info,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
            action: None,
        }
    }

    pub fn with_action(mut self, label: impl Into<String>, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.action = Some(ToastAction {
            label: label.into(),
            callback: std::sync::Arc::new(callback),
        });
        self
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.duration
    }

    fn render(&self, ui: &mut egui::Ui, theme: &crate::design::DesignTheme, size: egui::Vec2) {
        let (bg_color, icon, icon_color) = match self.toast_type {
            ToastType::Success => (
                crate::design::SemanticColors::SUCCESS.linear_multiply(0.15),
                "✓",
                crate::design::SemanticColors::SUCCESS,
            ),
            ToastType::Error => (
                crate::design::SemanticColors::DANGER.linear_multiply(0.15),
                "✗",
                crate::design::SemanticColors::DANGER,
            ),
            ToastType::Warning => (
                crate::design::SemanticColors::WARNING.linear_multiply(0.15),
                "⚠",
                crate::design::SemanticColors::WARNING,
            ),
            ToastType::Info => (
                theme.bg_elevated,
                "ℹ",
                crate::design::BrandColors::C500,
            ),
        };

        egui::Frame::group(ui.style())
            .fill(bg_color)
            .rounding(egui::Rounding::same(8.0))
            .stroke(egui::Stroke::new(1.0, icon_color.linear_multiply(0.3)))
            .show(ui, |ui| {
                ui.set_min_size(size);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(icon)
                            .size(20.0)
                            .color(icon_color)
                            .strong()
                    );

                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(&self.title)
                                .size(14.0)
                                .strong()
                                .color(theme.text_primary)
                        );

                        ui.label(
                            egui::RichText::new(&self.message)
                                .size(12.0)
                                .color(theme.text_secondary)
                        );
                    });

                    if let Some(action) = &self.action {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(&action.label).clicked() {
                                (action.callback)();
                            }
                        });
                    }
                });
            });
    }
}

// ============================================================================
// LOADING SPINNER
// ============================================================================

/// Loading spinner animation
pub struct LoadingSpinner {
    size: f32,
    color: egui::Color32,
    speed: f32,
}

impl LoadingSpinner {
    pub fn new(size: f32) -> Self {
        Self {
            size,
            color: crate::design::BrandColors::C500,
            speed: 0.15,
        }
    }

    pub fn with_color(mut self, color: egui::Color32) -> Self {
        self.color = color;
        self
    }

    pub fn render(&self, ui: &mut egui::Ui, frame: u64) {
        let (response, painter) = ui.allocate_painter(
            egui::Vec2::splat(self.size),
            egui::Sense::hover(),
        );

        let center = response.rect.center();
        let radius = self.size / 2.0 - 2.0;
        let stroke_width = self.size / 8.0;

        let num_dots = 8;
        let angle_offset = (frame as f32 * self.speed) % (2.0 * std::f32::consts::PI);

        for i in 0..num_dots {
            let angle = angle_offset + (i as f32 * 2.0 * std::f32::consts::PI / num_dots as f32);
            let dot_pos = center + egui::Vec2::new(angle.cos(), angle.sin()) * radius;
            let alpha = ((i as f32 / num_dots as f32) * 255.0) as u8;
            let dot_color = egui::Color32::from_rgba_premultiplied(
                self.color.r(),
                self.color.g(),
                self.color.b(),
                alpha,
            );
            painter.circle_filled(dot_pos, stroke_width / 2.0, dot_color);
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Helper function to format duration in human-readable format
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Helper function to format bytes in human-readable format
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
