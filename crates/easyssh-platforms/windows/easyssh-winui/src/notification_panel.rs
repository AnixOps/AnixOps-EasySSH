#![allow(dead_code)]

//! Notification Panel UI for EasySSH
//!
//! Provides a notification history panel with:
//! - List of all notifications with timestamps
//! - Unread/read indicators
//! - Click to open associated window/action
//! - Clear history / Mark all as read
//! - Filter by type

use chrono::{DateTime, Duration, Local};
use egui::{Color32, Frame, RichText, Rounding, ScrollArea, Stroke, Ui, Vec2};
use std::sync::Arc;

use crate::notifications::{
    NotificationManager, NotificationPriority, NotificationRecord, NotificationType,
};

/// Notification panel state
pub struct NotificationPanel {
    pub visible: bool,
    pub show_unread_only: bool,
    pub selected_types: Vec<NotificationType>,
    pub filter_text: String,
    pub notification_manager: Arc<NotificationManager>,
}

impl NotificationPanel {
    pub fn new(notification_manager: Arc<NotificationManager>) -> Self {
        Self {
            visible: false,
            show_unread_only: false,
            selected_types: vec![],
            filter_text: String::new(),
            notification_manager,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if !self.visible {
            return;
        }

        let unread_count = self.notification_manager.get_unread_count();

        // Panel container
        Frame::none()
            .fill(ui.visuals().panel_fill)
            .stroke(Stroke::new(
                1.0,
                ui.visuals().widgets.inactive.bg_stroke.color,
            ))
            .rounding(Rounding::same(8.0))
            .show(ui, |ui| {
                ui.set_min_width(380.0);
                ui.set_max_width(400.0);

                // Header
                ui.horizontal(|ui| {
                    ui.heading(format!("通知 ({} 未读)", unread_count));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("×").clicked() {
                            self.visible = false;
                        }
                    });
                });
                ui.separator();

                // Toolbar
                ui.horizontal(|ui| {
                    // Filter toggle
                    ui.checkbox(&mut self.show_unread_only, "仅未读");
                    ui.separator();

                    // Mark all as read
                    if ui.button("全部已读").clicked() {
                        let count = self.notification_manager.mark_all_as_read();
                        tracing::info!("Marked {} notifications as read", count);
                    }

                    // Clear all
                    if ui
                        .button("清空")
                        .on_hover_text("清空所有通知历史")
                        .clicked()
                    {
                        self.notification_manager.clear_history();
                        tracing::info!("Notification history cleared");
                    }
                });

                // Filter by type
                ui.collapsing("按类型筛选", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let all_types = vec![
                            NotificationType::ConnectionSuccess,
                            NotificationType::ConnectionFailed,
                            NotificationType::FileTransferComplete,
                            NotificationType::FileTransferFailed,
                            NotificationType::CpuAlert,
                            NotificationType::MemoryAlert,
                            NotificationType::DiskAlert,
                            NotificationType::SessionDisconnected,
                        ];

                        for notif_type in all_types {
                            let selected = self.selected_types.contains(&notif_type);
                            let text =
                                format!("{:?}", notif_type).replace("NotificationType::", "");
                            let mut btn = ui.button(&text);
                            if selected {
                                btn = btn.highlight();
                            }
                            if btn.clicked() {
                                if selected {
                                    self.selected_types.retain(|t| t != &notif_type);
                                } else {
                                    self.selected_types.push(notif_type.clone());
                                }
                            }
                        }
                    });
                });

                ui.separator();

                // Notification list
                let notifications = self.get_filtered_notifications();

                if notifications.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(RichText::new("暂无通知").color(ui.visuals().weak_text_color()));
                    });
                } else {
                    ScrollArea::vertical().max_height(500.0).show(ui, |ui| {
                        for notification in notifications {
                            self.render_notification_item(ui, &notification);
                            ui.separator();
                        }
                    });
                }
            });
    }

    fn get_filtered_notifications(&self) -> Vec<NotificationRecord> {
        let all = self
            .notification_manager
            .get_history(self.show_unread_only, Some(100));

        all.into_iter()
            .filter(|n| {
                // Filter by type if any selected
                if !self.selected_types.is_empty()
                    && !self.selected_types.contains(&n.notification_type)
                {
                    return false;
                }

                // Filter by text search
                if !self.filter_text.is_empty() {
                    let search_lower = self.filter_text.to_lowercase();
                    if !n.title.to_lowercase().contains(&search_lower)
                        && !n.message.to_lowercase().contains(&search_lower)
                    {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    fn render_notification_item(&self, ui: &mut Ui, notification: &NotificationRecord) {
        let unread = !notification.read;

        // Background based on priority and read status
        let bg_color = if unread {
            match notification.priority {
                NotificationPriority::Urgent => Color32::from_rgb(80, 20, 20),
                NotificationPriority::High => Color32::from_rgb(60, 30, 10),
                _ => Color32::from_rgb(40, 40, 50),
            }
        } else {
            ui.visuals().extreme_bg_color
        };

        let frame = Frame::none()
            .fill(bg_color)
            .rounding(Rounding::same(6.0))
            .inner_margin(egui::Margin::same(10.0));

        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                // Unread indicator
                if unread {
                    ui.add_space(2.0);
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(6.0, 6.0), egui::Sense::hover());
                    ui.painter()
                        .circle_filled(rect.center(), 3.0, Color32::from_rgb(0, 150, 255));
                    ui.add_space(6.0);
                } else {
                    ui.add_space(14.0);
                }

                // Icon and content
                ui.vertical(|ui| {
                    // Title row
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&notification.icon).size(16.0));
                        ui.label(RichText::new(&notification.title).strong());

                        // Timestamp
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let time_text = Self::format_timestamp(notification.timestamp);
                            ui.label(
                                RichText::new(time_text)
                                    .small()
                                    .color(ui.visuals().weak_text_color()),
                            );
                        });
                    });

                    // Message
                    ui.label(RichText::new(&notification.message).size(13.0));

                    // Priority badge
                    ui.horizontal(|ui| {
                        match notification.priority {
                            NotificationPriority::Urgent => {
                                ui.colored_label(Color32::from_rgb(255, 100, 100), "紧急");
                            }
                            NotificationPriority::High => {
                                ui.colored_label(Color32::from_rgb(255, 180, 100), "重要");
                            }
                            _ => {}
                        }

                        // Action button if there's action data
                        if notification.action_data.is_some() {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("打开").clicked() {
                                        // Handle action - this will be connected to the main app
                                        tracing::info!(
                                            "Notification action clicked: {}",
                                            notification.id
                                        );
                                    }
                                    if unread && ui.button("标记已读").clicked() {
                                        self.notification_manager.mark_as_read(&notification.id);
                                    }
                                },
                            );
                        }
                    });
                });
            });
        });
    }

    fn format_timestamp(timestamp: DateTime<Local>) -> String {
        let now = Local::now();
        let diff = now - timestamp;

        if diff < Duration::minutes(1) {
            "刚刚".to_string()
        } else if diff < Duration::hours(1) {
            format!("{} 分钟前", diff.num_minutes())
        } else if diff < Duration::days(1) {
            format!("{} 小时前", diff.num_hours())
        } else if diff < Duration::days(7) {
            format!("{} 天前", diff.num_days())
        } else {
            timestamp.format("%Y-%m-%d %H:%M").to_string()
        }
    }
}

/// Notification settings panel
pub struct NotificationSettingsPanel {
    pub visible: bool,
    notification_manager: Arc<NotificationManager>,
}

impl NotificationSettingsPanel {
    pub fn new(notification_manager: Arc<NotificationManager>) -> Self {
        Self {
            visible: false,
            notification_manager,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if !self.visible {
            return;
        }

        let mut settings = self.notification_manager.get_settings();

        Frame::none()
            .fill(ui.visuals().panel_fill)
            .stroke(Stroke::new(
                1.0,
                ui.visuals().widgets.inactive.bg_stroke.color,
            ))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_min_width(450.0);
                ui.set_max_width(500.0);

                // Header
                ui.horizontal(|ui| {
                    ui.heading("通知设置");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("×").clicked() {
                            self.visible = false;
                        }
                    });
                });
                ui.separator();

                // Global settings
                ui.checkbox(&mut settings.global_enabled, "启用通知");
                ui.checkbox(&mut settings.show_tray_notifications, "显示托盘通知");

                // Do not disturb
                ui.separator();
                ui.heading("免打扰模式");

                let dnd_active = self.notification_manager.is_dnd_active();
                if dnd_active {
                    ui.label(
                        RichText::new("✓ 免打扰模式已启用").color(Color32::from_rgb(100, 255, 100)),
                    );
                }

                ui.horizontal(|ui| {
                    if ui.button("启用 1 小时").clicked() {
                        self.notification_manager.enable_dnd(Some(60));
                    }
                    if ui.button("启用 8 小时").clicked() {
                        self.notification_manager.enable_dnd(Some(480));
                    }
                    if ui.button("直到明天").clicked() {
                        // Calculate minutes until midnight
                        let now = Local::now();
                        let tomorrow = (now + Duration::days(1))
                            .date_naive()
                            .and_hms_opt(0, 0, 0)
                            .unwrap();
                        let tomorrow_dt = tomorrow.and_local_timezone(Local).unwrap();
                        let minutes = (tomorrow_dt - now).num_minutes() as u32;
                        self.notification_manager.enable_dnd(Some(minutes));
                    }
                    if dnd_active && ui.button("关闭").clicked() {
                        self.notification_manager.disable_dnd();
                    }
                });

                // Per-type settings
                ui.separator();
                ui.heading("按类型设置");

                ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                    for (notif_type, type_settings) in &mut settings.type_settings {
                        ui.collapsing(
                            format!("{:?}", notif_type).replace("NotificationType::", ""),
                            |ui| {
                                ui.checkbox(&mut type_settings.enabled, "启用");
                                ui.checkbox(&mut type_settings.sound_enabled, "播放声音");
                                ui.checkbox(&mut type_settings.show_in_history, "保存到历史");

                                ui.horizontal(|ui| {
                                    ui.label("优先级:");
                                    egui::ComboBox::from_id_source(format!(
                                        "priority_{:?}",
                                        notif_type
                                    ))
                                    .selected_text(format!("{:?}", type_settings.priority))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut type_settings.priority,
                                            NotificationPriority::Low,
                                            "低",
                                        );
                                        ui.selectable_value(
                                            &mut type_settings.priority,
                                            NotificationPriority::Default,
                                            "默认",
                                        );
                                        ui.selectable_value(
                                            &mut type_settings.priority,
                                            NotificationPriority::High,
                                            "高",
                                        );
                                        ui.selectable_value(
                                            &mut type_settings.priority,
                                            NotificationPriority::Urgent,
                                            "紧急",
                                        );
                                    });
                                });
                            },
                        );
                    }
                });

                // Save button
                ui.separator();
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("保存设置").clicked() {
                            self.notification_manager.update_settings(settings.clone());
                            tracing::info!("Notification settings updated");
                        }
                    });
                });
            });
    }
}

/// Tray notification popup (when minimized)
pub struct TrayNotification {
    pub visible: bool,
    pub message: String,
    pub action: Option<String>,
}

impl TrayNotification {
    pub fn new() -> Self {
        Self {
            visible: false,
            message: String::new(),
            action: None,
        }
    }

    pub fn show(&mut self, message: &str, action: Option<&str>) {
        self.message = message.to_string();
        self.action = action.map(|s| s.to_string());
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }
}
