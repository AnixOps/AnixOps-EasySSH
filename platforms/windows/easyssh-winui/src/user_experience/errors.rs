//! Error Handling and Recovery for EasySSH
//!
//! Provides user-friendly error messages with actionable recovery suggestions.

use std::collections::VecDeque;
use std::time::{Duration, Instant};
use egui::{Color32, RichText, Ui, Frame, Stroke, Rounding, Margin, Layout, Align};

/// User-friendly error message with recovery actions
#[derive(Clone)]
pub struct UserError {
    pub id: String,
    pub error_type: ErrorType,
    pub title: String,
    pub message: String,
    pub suggestion: String,
    pub actions: Vec<ErrorAction>,
    pub created_at: Instant,
    pub auto_dismiss: bool,
    pub dismiss_after: Duration,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ErrorType {
    Connection,
    Authentication,
    Network,
    FileSystem,
    Configuration,
    Permission,
    Timeout,
    Unknown,
}

impl ErrorType {
    pub fn icon(&self) -> &'static str {
        match self {
            ErrorType::Connection => "🔌",
            ErrorType::Authentication => "🔒",
            ErrorType::Network => "🌐",
            ErrorType::FileSystem => "📁",
            ErrorType::Configuration => "⚙",
            ErrorType::Permission => "🚫",
            ErrorType::Timeout => "⏱",
            ErrorType::Unknown => "⚠",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            ErrorType::Connection => Color32::from_rgb(239, 68, 68),   // Red
            ErrorType::Authentication => Color32::from_rgb(245, 158, 11), // Orange
            ErrorType::Network => Color32::from_rgb(59, 130, 246),      // Blue
            ErrorType::FileSystem => Color32::from_rgb(139, 92, 246),   // Purple
            ErrorType::Configuration => Color32::from_rgb(107, 114, 128), // Gray
            ErrorType::Permission => Color32::from_rgb(236, 72, 153), // Pink
            ErrorType::Timeout => Color32::from_rgb(245, 158, 11),      // Orange
            ErrorType::Unknown => Color32::from_rgb(239, 68, 68),       // Red
        }
    }
}

#[derive(Clone)]
pub struct ErrorAction {
    pub label: String,
    pub action_type: ActionType,
    pub callback: std::sync::Arc<dyn Fn() + Send + Sync>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActionType {
    Retry,
    Cancel,
    Settings,
    Help,
    Report,
    Ignore,
}

impl UserError {
    /// Create error from a raw error string
    pub fn from_error(error: &str) -> Self {
        // Analyze error string to determine type and generate appropriate message
        let (error_type, title, suggestion, actions) = Self::analyze_error(error);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            error_type,
            title,
            message: error.to_string(),
            suggestion,
            actions,
            created_at: Instant::now(),
            auto_dismiss: false,
            dismiss_after: Duration::from_secs(10),
        }
    }

    /// Analyze error string and determine appropriate response
    fn analyze_error(error: &str) -> (ErrorType, String, String, Vec<ErrorAction>) {
        let error_lower = error.to_lowercase();
        let mut actions = vec![];

        // Connection errors
        if error_lower.contains("connection") || error_lower.contains("timeout") || error_lower.contains("refused") {
            let suggestion = if error_lower.contains("timeout") {
                "服务器响应超时。请检查：\n1. 网络连接是否正常\n2. 服务器是否在线\n3. 防火墙设置".to_string()
            } else {
                "无法连接到服务器。请检查主机地址和端口设置。".to_string()
            };

            actions.push(ErrorAction {
                label: "重试连接".to_string(),
                action_type: ActionType::Retry,
                callback: std::sync::Arc::new(|| {}),
            });

            actions.push(ErrorAction {
                label: "检查设置".to_string(),
                action_type: ActionType::Settings,
                callback: std::sync::Arc::new(|| {}),
            });

            return (ErrorType::Connection, "连接失败".to_string(), suggestion, actions);
        }

        // Authentication errors
        if error_lower.contains("auth") || error_lower.contains("password") || error_lower.contains("key") {
            let suggestion = if error_lower.contains("key") {
                "SSH密钥认证失败。请检查：\n1. 密钥文件是否正确\n2. 密钥权限是否正确（应为600）\n3. 服务器是否配置了对应的公钥".to_string()
            } else {
                "密码认证失败。请检查用户名和密码是否正确。".to_string()
            };

            actions.push(ErrorAction {
                label: "重新输入密码".to_string(),
                action_type: ActionType::Retry,
                callback: std::sync::Arc::new(|| {}),
            });

            actions.push(ErrorAction {
                label: "使用密钥认证".to_string(),
                action_type: ActionType::Settings,
                callback: std::sync::Arc::new(|| {}),
            });

            return (ErrorType::Authentication, "认证失败".to_string(), suggestion, actions);
        }

        // Network errors
        if error_lower.contains("network") || error_lower.contains("dns") || error_lower.contains("resolve") {
            actions.push(ErrorAction {
                label: "检查网络".to_string(),
                action_type: ActionType::Settings,
                callback: std::sync::Arc::new(|| {}),
            });

            return (
                ErrorType::Network,
                "网络错误".to_string(),
                "无法解析主机名或网络不可用。请检查DNS设置和网络连接。".to_string(),
                actions,
            );
        }

        // File system errors
        if error_lower.contains("file") || error_lower.contains("directory") || error_lower.contains("path") || error_lower.contains("sftp") {
            let suggestion = if error_lower.contains("permission") {
                "文件权限不足。请检查服务器上的文件权限设置。".to_string()
            } else if error_lower.contains("not found") || error_lower.contains("exist") {
                "文件或目录不存在。请检查路径是否正确。".to_string()
            } else if error_lower.contains("sftp") {
                "SFTP操作失败。请确保服务器支持SFTP并正确配置。".to_string()
            } else {
                "文件操作失败。请检查路径和权限。".to_string()
            };

            actions.push(ErrorAction {
                label: "重试".to_string(),
                action_type: ActionType::Retry,
                callback: std::sync::Arc::new(|| {}),
            });

            return (ErrorType::FileSystem, "文件操作失败".to_string(), suggestion, actions);
        }

        // Permission errors
        if error_lower.contains("permission") || error_lower.contains("denied") || error_lower.contains("unauthorized") {
            actions.push(ErrorAction {
                label: "检查权限".to_string(),
                action_type: ActionType::Settings,
                callback: std::sync::Arc::new(|| {}),
            });

            return (
                ErrorType::Permission,
                "权限不足".to_string(),
                "当前用户没有执行此操作的权限。请联系服务器管理员。".to_string(),
                actions,
            );
        }

        // Configuration errors
        if error_lower.contains("config") || error_lower.contains("setting") || error_lower.contains("invalid") {
            actions.push(ErrorAction {
                label: "查看帮助".to_string(),
                action_type: ActionType::Help,
                callback: std::sync::Arc::new(|| {}),
            });

            return (
                ErrorType::Configuration,
                "配置错误".to_string(),
                "配置参数有误。请检查服务器设置。".to_string(),
                actions,
            );
        }

        // Default unknown error
        actions.push(ErrorAction {
            label: "重试".to_string(),
            action_type: ActionType::Retry,
            callback: std::sync::Arc::new(|| {}),
        });

        actions.push(ErrorAction {
            label: "忽略".to_string(),
            action_type: ActionType::Ignore,
            callback: std::sync::Arc::new(|| {}),
        });

        (
            ErrorType::Unknown,
            "操作失败".to_string(),
            "发生未知错误。如果问题持续，请尝试重新连接或重启应用。".to_string(),
            actions,
        )
    }

    pub fn with_action(mut self, label: impl Into<String>, action_type: ActionType, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.actions.push(ErrorAction {
            label: label.into(),
            action_type,
            callback: std::sync::Arc::new(callback),
        });
        self
    }

    pub fn auto_dismiss(mut self, after: Duration) -> Self {
        self.auto_dismiss = true;
        self.dismiss_after = after;
        self
    }

    pub fn is_expired(&self) -> bool {
        self.auto_dismiss && self.created_at.elapsed() > self.dismiss_after
    }

    /// Render the error message
    pub fn render(&self, ui: &mut Ui, theme: &crate::design::DesignTheme, on_dismiss: impl FnOnce()) -> bool {
        let error_color = self.error_type.color();
        let bg_color = error_color.linear_multiply(0.1);

        let mut dismissed = false;

        Frame::group(ui.style())
            .fill(bg_color)
            .rounding(Rounding::same(8.0))
            .stroke(Stroke::new(1.0, error_color.linear_multiply(0.3)))
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_min_width(400.0);

                // Header with icon and title
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(self.error_type.icon())
                            .size(24.0)
                    );

                    ui.label(
                        RichText::new(&self.title)
                            .size(16.0)
                            .strong()
                            .color(error_color)
                    );

                    // Dismiss button
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            dismissed = true;
                        }
                    });
                });

                ui.add_space(8.0);

                // Technical message (collapsible)
                ui.collapsing("技术详情", |ui| {
                    ui.label(
                        RichText::new(&self.message)
                            .size(12.0)
                            .color(theme.text_secondary)
                            .monospace()
                    );
                });

                ui.add_space(8.0);

                // User-friendly suggestion
                ui.label(
                    RichText::new(&self.suggestion)
                        .size(14.0)
                        .color(theme.text_primary)
                );

                ui.add_space(12.0);

                // Action buttons
                ui.horizontal(|ui| {
                    for action in &self.actions {
                        let button = match action.action_type {
                            ActionType::Retry => {
                                crate::design::AccessibleButton::new(theme, &action.label)
                                    .style(crate::design::AccessibleButtonStyle::Primary)
                                    .build()
                            }
                            ActionType::Settings | ActionType::Help => {
                                crate::design::AccessibleButton::new(theme, &action.label)
                                    .style(crate::design::AccessibleButtonStyle::Secondary)
                                    .build()
                            }
                            _ => {
                                crate::design::AccessibleButton::new(theme, &action.label)
                                    .style(crate::design::AccessibleButtonStyle::Ghost)
                                    .build()
                            }
                        };

                        if ui.add(button).clicked() {
                            (action.callback)();
                        }
                    }
                });
            });

        if dismissed {
            on_dismiss();
        }

        dismissed
    }
}

/// Queue for managing multiple errors
#[derive(Default)]
pub struct ErrorQueue {
    errors: VecDeque<UserError>,
    max_errors: usize,
    deduplicate: bool,
}

impl ErrorQueue {
    pub fn new() -> Self {
        Self {
            errors: VecDeque::new(),
            max_errors: 5,
            deduplicate: true,
        }
    }

    /// Add an error to the queue
    pub fn push(&mut self, error: UserError) {
        // Check for duplicates if deduplication is enabled
        if self.deduplicate {
            let is_duplicate = self.errors.iter().any(|e| {
                e.message == error.message && e.error_type == error.error_type
            });

            if is_duplicate {
                return;
            }
        }

        // Remove oldest if at capacity
        while self.errors.len() >= self.max_errors {
            self.errors.pop_front();
        }

        self.errors.push_back(error);
    }

    /// Create and add error from string
    pub fn push_error(&mut self, error: &str) {
        self.push(UserError::from_error(error));
    }

    /// Remove a specific error
    pub fn remove(&mut self, id: &str) {
        self.errors.retain(|e| e.id != id);
    }

    /// Clear all errors
    pub fn clear(&mut self) {
        self.errors.clear();
    }

    /// Get all errors
    pub fn get_errors(&self) -> &VecDeque<UserError> {
        &self.errors
    }

    /// Get error count
    pub fn count(&self) -> usize {
        self.errors.len()
    }

    /// Check if has errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Clean up expired errors
    pub fn cleanup(&mut self) {
        self.errors.retain(|e| !e.is_expired());
    }

    /// Render all errors
    pub fn render_all(&mut self, ui: &mut Ui, theme: &crate::design::DesignTheme) {
        self.cleanup();

        let errors_to_render: Vec<_> = self.errors.iter().cloned().collect();

        for error in errors_to_render {
            let id = error.id.clone();
            error.render(ui, theme, || {
                self.remove(&id);
            });
            ui.add_space(8.0);
        }
    }
}

/// Inline error display for forms
pub struct InlineError {
    message: String,
    show_icon: bool,
}

impl InlineError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            show_icon: true,
        }
    }

    pub fn render(&self, ui: &mut Ui, _theme: &crate::design::DesignTheme) {
        ui.horizontal(|ui| {
            if self.show_icon {
                ui.label(
                    RichText::new("⚠")
                        .size(16.0)
                        .color(crate::design::SemanticColors::WARNING)
                );
            }

            ui.label(
                RichText::new(&self.message)
                    .size(13.0)
                    .color(crate::design::SemanticColors::DANGER)
            );
        });
    }
}

/// Success message display
pub struct SuccessMessage {
    message: String,
}

impl SuccessMessage {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn render(&self, ui: &mut Ui, theme: &crate::design::DesignTheme) {
        Frame::group(ui.style())
            .fill(crate::design::SemanticColors::SUCCESS.linear_multiply(0.1))
            .rounding(Rounding::same(6.0))
            .stroke(Stroke::new(1.0, crate::design::SemanticColors::SUCCESS.linear_multiply(0.3)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("✓")
                            .size(18.0)
                            .color(crate::design::SemanticColors::SUCCESS)
                    );

                    ui.label(
                        RichText::new(&self.message)
                            .size(14.0)
                            .color(theme.text_primary)
                    );
                });
            });
    }
}

/// Confirmation dialog
pub struct ConfirmDialog {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub cancel_label: String,
    pub is_dangerous: bool,
}

impl ConfirmDialog {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            confirm_label: "确认".to_string(),
            cancel_label: "取消".to_string(),
            is_dangerous: false,
        }
    }

    pub fn dangerous(mut self) -> Self {
        self.is_dangerous = true;
        self
    }

    pub fn with_labels(mut self, confirm: impl Into<String>, cancel: impl Into<String>) -> Self {
        self.confirm_label = confirm.into();
        self.cancel_label = cancel.into();
        self
    }

    pub fn render(&self, ui: &mut Ui, theme: &crate::design::DesignTheme) -> Option<bool> {
        let mut result = None;

        ui.vertical_centered(|ui| {
            ui.add_space(24.0);

            // Icon
            let icon = if self.is_dangerous { "⚠" } else { "?" };
            let icon_color = if self.is_dangerous {
                crate::design::SemanticColors::WARNING
            } else {
                crate::design::BrandColors::C500
            };

            ui.label(
                RichText::new(icon)
                    .size(48.0)
                    .color(icon_color)
            );

            ui.add_space(16.0);

            // Title
            ui.label(
                RichText::new(&self.title)
                    .size(18.0)
                    .strong()
                    .color(theme.text_primary)
            );

            ui.add_space(8.0);

            // Message
            ui.label(
                RichText::new(&self.message)
                    .size(14.0)
                    .color(theme.text_secondary)
            );

            ui.add_space(24.0);

            // Buttons
            ui.horizontal(|ui| {
                let confirm_style = if self.is_dangerous {
                    crate::design::AccessibleButtonStyle::Danger
                } else {
                    crate::design::AccessibleButtonStyle::Primary
                };

                if ui.add(
                    crate::design::AccessibleButton::new(theme, &self.confirm_label)
                        .style(confirm_style)
                        .build()
                ).clicked() {
                    result = Some(true);
                }

                if ui.add(
                    crate::design::AccessibleButton::new(theme, &self.cancel_label)
                        .style(crate::design::AccessibleButtonStyle::Ghost)
                        .build()
                ).clicked() {
                    result = Some(false);
                }
            });
        });

        result
    }
}
